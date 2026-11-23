// SPDX-License-Identifier: GPL-3.0-or-later
//! The widget tree: creation, layout, drawing, focus, and pointer and
//! keyboard handling.
//!
//! The engine never calls back into application code. Anything the
//! application should hear about is returned as a list of [`Emitted`] events,
//! which the caller delivers once it has let go of the `Ui`. That keeps a
//! handler free to call straight back into the toolkit.
//!
//! Time is whatever the caller last passed to [`Ui::tick`], in milliseconds.
//! It drives the caret blink, the marching focus ring and the flash a button
//! gives when the keyboard presses it. A caller that never ticks gets a
//! still picture, which is what snapshots and tests want.

use crate::canvas::{Canvas, Rect, BLACK, WHITE};
use crate::cursor::Cursor;
use crate::font::{Face, Font};
use std::collections::HashMap;
use crate::slots::{Handle, Slots};
use crate::style::{state, Look, Painter, Part, PartKind, Style};
use crate::text::Editor;

mod access;
mod controls;
mod divider;
mod dnd;
mod extra;
pub(crate) use extra::plain_dress;
pub use dnd::settle_copies;
mod field;
mod frame;
pub use frame::FrameAct;
mod list;
mod menu;
mod more;
mod picture;
pub use picture::LineStyle;
mod popup;
mod rich;
mod scroll;
mod view;
use scroll::BarPress;
pub use access::{AccessNode, AccessRole, AccessRun, AccessTree};
use menu::MenuOpen;
#[cfg(test)]
use menu::BAR_HEIGHT;
pub use list::{Icon, IconLayout};
pub use menu::{named_shortcut, MenuEntry, PopupInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidHandle,
    WrongType,
    InvalidArgument,
    InvalidState,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Fill,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    I32(i32),
    Str(String),
    /// Content that is not text: a picture, say, as the bytes of its file.
    Bytes(Vec<u8>),
}

impl Value {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Value::Bytes(b) => b,
            Value::Str(s) => s.as_bytes(),
            _ => &[],
        }
    }
    pub fn as_bool(&self) -> bool {
        matches!(self, Value::Bool(true)) || matches!(self, Value::I32(v) if *v != 0) || matches!(self, Value::Str(s) if !s.is_empty())
    }
    pub fn as_i32(&self) -> i32 {
        match self {
            Value::Bool(b) => *b as i32,
            Value::I32(v) => *v,
            Value::Str(_) | Value::Bytes(_) => 0,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Value::Str(s) => s,
            _ => "",
        }
    }
}

/// Something the application may have a handler for, named "Class.event".
#[derive(Debug, Clone, PartialEq)]
pub struct Emitted {
    pub target: Handle,
    pub event: &'static str,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    Move,
    Press,
    Release,
    /// The pointer left the window.
    Leave,
}

/// A key that is not text. Printable characters arrive through [`Ui::text`];
/// `Char` is only for shortcuts, where the letter is not being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Tab,
    Return,
    Escape,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    /// Pull down or put away the menu bar (F10).
    Menu,
    /// Pop up the focused widget's context menu (the Menu key, Shift+F10).
    Context,
    PageUp,
    PageDown,
    Insert,
    /// A function key, F1 to F12. F10 comes as `Menu`.
    F(u8),
    Char(char),
}

/// Modifier keys held during a key press. Control is the command key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// What a display backend needs to know about a window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowInfo {
    pub handle: Handle,
    pub title: String,
    /// A name for the desktop to know the window by again; empty for none.
    pub tag: String,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
}

enum Kind {
    /// The bar between two panes of a stack, dragged to share the room out
    /// (see divider.rs): which way the panes lie, and how long the one
    /// before it has been made.
    Divider {
        orientation: Orientation,
        position: Option<i32>,
    },
    Window {
        title: String,
        /// A name, not for people to read, by which the desktop can know
        /// this window again another day.
        tag: String,
        /// Whether the user may change its size.
        resizable: bool,
        /// The program is busy with something this window is waiting on.
        busy: bool,
        /// The kinds of thing, other than text and files, that may be
        /// dropped on it, the one liked best first.
        accepts: Vec<String>,
        width: i32,
        height: i32,
        content: Option<Handle>,
        menu_bar: Option<Handle>,
        /// The desktop is showing the menu bar (across the top of the
        /// screen, say), so the window does not draw one.
        bar_external: bool,
        /// The window fills its screen, or has asked to: it draws no menu bar.
        full_screen: bool,
        visible: bool,
        /// The widget that receives keys.
        focus: Option<Handle>,
        /// Nothing has chosen a focus yet, so the first text field gets it.
        auto_focus: bool,
        /// Whether the window has the keyboard. Inactive windows hide the
        /// caret, the selection and the focus ring.
        active: bool,
    },
    Stack { orientation: Orientation, spacing: i32, padding: i32, align: Align, children: Vec<Handle> },
    Label { text: String },
    Button { label: String, is_default: bool, is_cancel: bool },
    Checkbox { label: String, checked: bool },
    /// One line of editable text, or with `multiline` a text area of
    /// several. `scroll` is in pixels sideways for the one, in lines down
    /// for the other.
    TextField {
        edit: Editor,
        placeholder: String,
        min_width: i32,
        scroll: i32,
        multiline: bool,
        /// Height of a text area, in lines.
        rows: i32,
        /// Shows bullets in place of the characters.
        password: bool,
        /// A text area drawn with no frame and no scroll bar, as writing
        /// on a page.
        plain: bool,
        /// Where each line sits across the field: at the start, in the
        /// middle, or at the end, as figures do.
        align: Align,
        /// The column a run of up and down keys is keeping to.
        goal_x: Option<i32>,
        /// Scroll to keep the caret in view; off after the wheel has been
        /// turned, until the caret next moves.
        follow: bool,
        /// The selection (anchor, caret) the program last knew of: what it
        /// set, or what it was last told of by a selected event.
        reported: (usize, usize),
    },
    /// A box the program draws on; what it draws is kept as marks.
    /// `raw`: every key is told to the program (for a terminal); `shown`:
    /// the size last told by `resized`.
    Picture { width: i32, height: i32, marks: Vec<picture::Mark>, description: String, keys: bool, raw: bool, right: bool, middle: bool, shown: (i32, i32) },
    /// Shows how far along some work is; `busy` shows moving stripes instead.
    Progress { value: i32, max: i32, busy: bool, length: i32 },
    /// Several pages in the space of one: each a title and a widget.
    Tabs { pages: Vec<(String, Handle)>, selected: usize },
    /// One of a group of choices; its siblings of the same kind are the group.
    Radio { label: String, checked: bool },
    /// A number picked by position. `value` runs from 0 to `max`.
    Slider { orientation: Orientation, value: i32, max: i32, step: i32, length: i32 },
    /// Rows of text, one of which may be selected. `scroll` is the first row showing.
    /// It is also a table (rows are tab-separated cells under `columns`) or
    /// a tree (`meta` gives each item its parent and whether it is unfolded).
    List {
        items: Vec<String>,
        selected: Option<usize>,
        scroll: i32,
        /// How far a table or an icon view is scrolled sideways, in pixels.
        across: i32,
        /// Drawn with no frame, to fill a window to its edges.
        flush: bool,
        rows: i32,
        min_width: i32,
        mode: list::ListMode,
        /// How an icon view lays out its items.
        layout: list::IconLayout,
        columns: Vec<(String, i32)>,
        /// What is kept for each item besides its label.
        meta: Vec<list::ItemMeta>,
        /// Several items may be selected at once.
        multiple: bool,
        /// Return edits the current item's text in place.
        editable: bool,
        /// A click on a column's heading sorts by it.
        sortable: bool,
        /// The column the rows are shown sorted by, and whether smallest first.
        sort: Option<(usize, bool)>,
        /// The column shown as the one the rows are in order of, for a
        /// program that orders them itself.
        marked: Option<usize>,
        /// How each column's cells sit in it, and the face they are in.
        looks: Vec<(Align, crate::font::Face)>,
        /// Where a stretch of selection began.
        anchor: Option<usize>,
        /// The order the rows are shown in when not sorted: the order they
        /// were added in, until the user or the program moves one.
        order: Vec<usize>,
        /// Rows can be dragged to a new place.
        reorderable: bool,
        /// Rows dragged out of another list can be dropped here.
        accepts: bool,
    },
    /// A box showing one of a list of choices. `menu` is the list as last
    /// popped up, kept so it can be thrown away.
    Popup { items: Vec<String>, selected: usize, menu: Option<Handle> },
    /// A window onto a widget larger than itself. `scroll` is how far the
    /// content has been moved left and up, in pixels.
    ScrollView { content: Option<Handle>, width: i32, height: i32, scroll: (i32, i32) },
    /// A number with arrows to step it by.
    Spin { value: i32, min: i32, max: i32, step: i32 },
    /// A scroll bar standing on its own. `value` runs from 0 to `max`.
    ScrollBar { orientation: Orientation, value: i32, max: i32, page: i32, step: i32, length: i32 },
    MenuBar { menus: Vec<Handle> },
    Menu { title: String, items: Vec<Handle> },
    /// A document of words with looks of their own, in paragraphs set as
    /// they say (see rich.rs).
    RichText(Box<rich::RichState>),
    /// `separator` items are dividing lines made by the engine itself.
    MenuItem {
        label: String,
        checked: bool,
        shortcut: Option<char>,
        separator: bool,
        submenu: Option<Handle>,
        /// For the engine's own editing menu: what choosing the item does.
        action: Option<menu::EditAction>,
    },
}

impl Kind {
    fn class(&self) -> &'static str {
        match self {
            Kind::Window { .. } => "Window",
            Kind::Divider { .. } => "Divider",
            Kind::Stack { .. } => "Stack",
            Kind::Label { .. } => "Label",
            Kind::Button { .. } => "Button",
            Kind::Checkbox { .. } => "Checkbox",
            Kind::TextField { multiline: true, .. } => "TextArea",
            Kind::TextField { .. } => "TextField",
            Kind::Progress { .. } => "ProgressBar",
            Kind::Picture { .. } => "Picture",
            Kind::Tabs { .. } => "TabView",
            Kind::Radio { .. } => "RadioButton",
            Kind::Slider { .. } => "Slider",
            Kind::List { mode: list::ListMode::Table, .. } => "Table",
            Kind::List { mode: list::ListMode::Tree, .. } => "TreeView",
            Kind::List { mode: list::ListMode::Icons, .. } => "IconView",
            Kind::List { .. } => "ListBox",
            Kind::Popup { .. } => "PopupButton",
            Kind::ScrollView { .. } => "ScrollView",
            Kind::Spin { .. } => "Stepper",
            Kind::ScrollBar { .. } => "ScrollBar",
            Kind::MenuBar { .. } => "MenuBar",
            Kind::Menu { .. } => "Menu",
            Kind::MenuItem { .. } => "MenuItem",
            Kind::RichText(_) => "RichText",
        }
    }

    /// Something that lives in a menu bar rather than in a window's content.
    fn is_menu_part(&self) -> bool {
        matches!(self, Kind::MenuBar { .. } | Kind::Menu { .. } | Kind::MenuItem { .. })
    }
}

struct Node {
    kind: Kind,
    parent: Option<Handle>,
    rect: Rect,
    enabled: bool,
    /// The menu that pops up when this is clicked with the right button.
    context_menu: Option<Handle>,
    /// Whether it takes up spare room when its window is larger than it
    /// needs to be. Unset, that goes by the kind of widget.
    stretch: Option<bool>,
    /// The least it may be, whatever its kind would make it.
    least: (i32, i32),
    /// The typeface its text is in, where it is not the interface's own.
    face: Option<Face>,
}

// Metrics, in pixels.
const FIELD_MIN_WIDTH: i32 = 140;
const RING_GAP: i32 = 3; // how far the focus ring stands off a button

// Timing, in milliseconds.
const CARET_MS: u64 = 530;
const PULSE_MS: u64 = 50;
const ANT_STEP_MS: u64 = 70;
const ANT_STEPS: u64 = 8; // the focus ring marches this far on arrival, then rests
const FLASH_MS: u64 = 110;
const DOUBLE_CLICK_MS: u64 = 400;

/// The part of a text field that holds the text.
fn field_inner(rect: Rect, m: &crate::style::Metrics) -> Rect {
    Rect::new(
        rect.x + m.field_inset_x,
        rect.y + m.field_inset_y,
        (rect.w - 2 * m.field_inset_x).max(0),
        (rect.h - 2 * m.field_inset_y).max(0),
    )
}

/// What a widget needs to know about its window while it is drawn.
#[derive(Clone, Copy)]
struct Paint {
    focus: Option<Handle>,
    active: bool,
}

#[derive(Default)]
pub struct Ui {
    nodes: Slots<Node>,
    /// How everything looks: the style in use, its sizes and its colours.
    look: Look,
    /// The widget under the pointer, for a style that shows it.
    hover: Option<Handle>,
    /// The widget being drawn.
    painting: Option<Handle>,
    /// The throb of a default button last drawn.
    pulse: u64,
    /// The menu whose title in the bar the pointer is over.
    bar_hot: Option<Handle>,
    /// Windows draw their own frames, where the style has one.
    own_frames: bool,
    /// How many of the screen's pixels lie along one of the interface's,
    /// where the display magnifies; 0 or 1 where it does not.
    fineness: i32,
    /// The button of a title bar the pointer is over, and the one it went
    /// down on: the window, and which button.
    frame_hot: Option<(Handle, u8)>,
    frame_held: Option<(Handle, u8)>,
    /// When and where a title bar was last clicked, to know a double click.
    frame_click: (u64, i32, i32),
    /// The windows that have been made as large as they can be.
    zoomed: Vec<Handle>,
    /// The windows rolled up into their title bars.
    rolled: Vec<Handle>,
    /// Something chosen from a window's own menu, for the display to do.
    frame_asked: Option<(Handle, FrameAct)>,
    /// The menu of a window's frame, made when first wanted.
    frame_menu: Option<Handle>,
    /// The typeface of a window's title, where the style has one for it.
    frame_font: Option<Font>,
    /// Which piece of the widget under the pointer the pointer is over: a
    /// tab, a heading, a part of a scroll bar, a title in the menu bar.
    hover_piece: i32,
    /// What whatever is being drawn lies on: the window, or the pane of a tab view.
    ground: u32,
    /// The stacks that are a box with a title, a bar of tools, a bar of news.
    dress: HashMap<Handle, extra::Dress>,
    /// The buttons that lie flat until the pointer comes.
    flat: Vec<Handle>,
    /// Buttons that stay in, as tools that are on.
    downs: Vec<Handle>,
    /// Buttons drawn as the keys of a calculator.
    keys: Vec<Handle>,
    /// Stacks laid on a pattern of black and white: eight rows of eight.
    backdrops: HashMap<Handle, [u8; 8]>,
    /// The pictures of buttons that have one.
    images: HashMap<Handle, std::rc::Rc<Canvas>>,
    /// The words of help of widgets that have some.
    tips: HashMap<Handle, String>,
    /// When the pointer came to what it is over, and whether help has been sent away since.
    tip_since: u64,
    tip_off: bool,
    /// Whether help was showing when the clock last moved.
    tip_shown: bool,
    /// Where the pointer is, measured from the corner of the content.
    hover_at: (i32, i32),
    /// The arrow of a stepper that is held: 1 down, 2 up.
    spin_held: Option<(Handle, i32, u64)>,
    /// Where the pointer was when the word of help now showing came up.
    tip_at: (i32, i32),
    /// The lists of things combo boxes might say, and the menu one was last shown as.
    choices: HashMap<Handle, Vec<String>>,
    combo_menu: Option<Handle>,
    font: Font,
    /// The typefaces not being worn just now, kept with what they have
    /// already drawn, and the one that is (`font`).
    wardrobe: HashMap<Face, Font>,
    worn: Face,
    /// The widget the pointer went down on, and whether it is still over it.
    pressed: Option<(Handle, bool)>,
    dirty: bool,
    /// Windows the program has given a new size since the display last
    /// asked, for the display to make them that size.
    resized: Vec<Handle>,
    /// Nothing grows into spare room. The tests of single widgets put them
    /// in windows with room to spare and look at them at their natural size.
    natural: bool,
    now: u64,
    /// When the caret last started a blink cycle; typing restarts it.
    caret_epoch: u64,
    /// When the focus last moved.
    focus_epoch: u64,
    /// A button pressed from the keyboard, and when its flash ends.
    flash: Option<(Handle, u64)>,
    /// The caret and ring state last reported as needing a redraw.
    anim: (bool, u64),
    /// Time, place and count of the latest run of clicks.
    clicks: (u64, i32, i32, u32),
    /// Text a copy or cut has produced, waiting for whoever owns the clipboard.
    copy_out: Option<String>,
    /// A text field that has asked for the clipboard's text.
    paste_request: Option<Handle>,
    /// A rich text widget waiting on the clipboard's RTF.
    rich_paste: Option<Handle>,
    /// A scroll bar being held with the pointer.
    bar_press: Option<BarPress>,
    /// The barber's-pole phase last drawn.
    stripes: i32,
    /// A slider whose knob is being dragged, and where on the knob it was taken.
    slider_drag: Option<(Handle, i32)>,
    /// A divider being dragged, and where on it it was taken.
    divider_drag: Option<(Handle, i32)>,
    /// A picture the pointer went down in, which hears where it goes until it is let go.
    picture_drag: Option<Handle>,
    /// A picture that hears the right button, which went down in it.
    picture_right: Option<Handle>,
    /// A picture that hears the middle button, which went down in it.
    picture_middle: Option<Handle>,
    /// A list box the pointer went down in; dragging moves its selection.
    list_drag: Option<Handle>,
    /// A cell of a list being edited in place.
    list_edit: Option<list::ListEdit>,
    /// A table column whose edge is being dragged.
    column_drag: Option<list::ColumnDrag>,
    /// A row being dragged to a new place: the list, the item, and the
    /// place among the rows it would land if let go now.
    row_drag: Option<list::RowDrag>,
    /// Where rows being dragged in from another program would land: the
    /// list, the gap among its rows, and in a tree the item they would hang from.
    foreign_drop: Option<(Handle, usize, Option<usize>)>,
    /// A press on one of several selected rows: if it turns out to be a
    /// click and not the start of a drag, that row alone stays selected.
    list_collapse: Option<(Handle, usize)>,
    /// The modifier keys held, for clicks: Control adds to a selection, Shift stretches it.
    mods: Mods,
    /// A widget to scroll into view at the next drawing.
    reveal: Option<Handle>,
    /// Where dragged text would be inserted if it were dropped now.
    drop_mark: Option<(Handle, usize)>,
    /// A press on selected text that will become a drag if the pointer
    /// moves away: the field and where the press was.
    drag_candidate: Option<(Handle, i32, i32)>,
    /// The text of a drag that has begun, until the backend collects it.
    drag_out: Option<String>,
    /// Things other than text that the program has copied, until the
    /// backend collects them: whether each is to go with what was copied
    /// before it or take its place, its kind, and the thing.
    copy_data_out: Vec<(bool, String, Vec<u8>)>,
    /// Windows that have asked for what is on the clipboard as some kind.
    paste_data_wanted: Vec<(Handle, String)>,
    /// Text an input method is still composing, and the field it is for.
    /// It is shown at the caret, underlined, but is not yet in the field.
    preedit: Option<(Handle, String)>,
    menu_open: Option<MenuOpen>,
    /// The Cut, Copy, Paste menu text fields pop up, made when first wanted.
    edit_menu: Option<Handle>,
    /// A chosen menu item, and when it began to blink.
    menu_blink: Option<(Handle, u64)>,
    /// A menu whose title is lit because a shortcut chose one of its items.
    title_flash: Option<(Handle, u64)>,
}

impl Ui {
    pub fn new() -> Ui {
        Ui::default()
    }

    /// An interface in which nothing stretches.
    #[cfg(test)]
    pub(crate) fn natural() -> Ui {
        Ui { natural: true, ..Ui::default() }
    }

    // --------------------------------------------------------------- style

    /// Dress the interface in a style. Everything is measured and drawn
    /// its way from now on.
    pub fn set_style(&mut self, style: std::sync::Arc<dyn Style>) {
        self.look = Look::of(style);
        // every typeface is the new style's from now on
        self.wardrobe.clear();
        self.font = self.look.font(self.worn);
        self.frame_font = self.look.frame_font();
        self.dirty = true;
    }

    /// The name of the style in use.
    pub fn style_name(&self) -> String {
        self.look.style.name().to_string()
    }

    /// The colours of the style in use.
    pub fn palette(&self) -> crate::style::Palette {
        self.look.pal
    }

    /// Have the style draw a part, in the typeface being worn. What the
    /// style leaves undrawn is drawn in black and white.
    fn part(&mut self, canvas: &mut Canvas, mut part: Part) {
        if matches!(part.kind, PartKind::Button | PartKind::Checkbox | PartKind::Radio | PartKind::Popup | PartKind::Slider) {
            if self.look.wants & crate::style::wants::HOVER != 0 && self.hover.is_some() && self.hover == self.painting {
                part.state |= state::HOT;
            }
            if part.kind == PartKind::Button && self.look.wants & crate::style::wants::PULSE != 0 {
                part.a = self.pulse_step() as i32;
            }
        }
        let style = self.look.style.clone();
        let mut painter = Painter { canvas, font: &mut self.font };
        if !style.draw(&part, &mut painter) {
            crate::style::EightyFour.draw(&part, &mut painter);
        }
    }

    /// Tell the engine how far the display magnifies: how many of the
    /// screen's pixels lie along one of the interface's. A style that asks
    /// to be drawn fine is then drawn on canvases that fine.
    pub fn set_fineness(&mut self, scale: i32) {
        self.fineness = scale.clamp(1, 8);
    }

    /// How fine the canvases are that the style is drawn on.
    pub fn fine_scale(&self) -> i32 {
        if self.look.wants & crate::style::wants::FINE != 0 { self.fineness.max(1) } else { 1 }
    }

    /// A canvas to draw part of the interface on, as fine as the rest.
    fn sheet(&self, width: i32, height: i32, color: u32) -> Canvas {
        Canvas::fine(width, height, self.fine_scale(), color)
    }

    /// Have the style draw a part if it will, and say whether it did: for
    /// a part the engine makes of others when the style has no word on it.
    fn part_own(&mut self, canvas: &mut Canvas, part: Part) -> bool {
        let style = self.look.style.clone();
        style.draw(&part, &mut Painter { canvas, font: &mut self.font })
    }

    // ------------------------------------------------------------ creation

    fn add(&mut self, kind: Kind) -> Handle {
        self.dirty = true;
        self.nodes.insert(Node { kind, parent: None, rect: Rect::default(), enabled: true, context_menu: None, stretch: None, least: (0, 0), face: None })
    }

    pub fn window_new(&mut self, title: &str, width: i32, height: i32) -> Result<Handle> {
        if width <= 0 || height <= 0 || width > 16384 || height > 16384 {
            return Err(Error::InvalidArgument);
        }
        Ok(self.add(Kind::Window {
            title: title.into(),
            tag: String::new(),
            resizable: true,
            busy: false,
            accepts: Vec::new(),
            width,
            height,
            content: None,
            menu_bar: None,
            bar_external: false,
            full_screen: false,
            visible: false,
            focus: None,
            auto_focus: true,
            active: true,
        }))
    }

    pub fn stack_new(&mut self, orientation: Orientation) -> Handle {
        self.add(Kind::Stack { orientation, spacing: 8, padding: 0, align: Align::Start, children: Vec::new() })
    }

    pub fn label_new(&mut self, text: &str) -> Handle {
        self.add(Kind::Label { text: text.into() })
    }

    pub fn button_new(&mut self, label: &str) -> Handle {
        self.add(Kind::Button { label: label.into(), is_default: false, is_cancel: false })
    }

    pub fn checkbox_new(&mut self, label: &str) -> Handle {
        self.add(Kind::Checkbox { label: label.into(), checked: false })
    }

    pub fn text_field_new(&mut self, text: &str) -> Handle {
        self.field_new(Editor::new(text), false)
    }

    /// A multi-line text field: it wraps its text and keeps line breaks.
    pub fn text_area_new(&mut self, text: &str) -> Handle {
        self.field_new(Editor::new_multiline(text), true)
    }

    fn field_new(&mut self, edit: Editor, multiline: bool) -> Handle {
        let reported = (edit.anchor(), edit.caret());
        self.add(Kind::TextField {
            reported,
            edit,
            placeholder: String::new(),
            min_width: FIELD_MIN_WIDTH,
            scroll: 0,
            multiline,
            rows: 4,
            password: false,
            plain: false,
            align: Align::Start,
            goal_x: None,
            follow: true,
        })
    }

    // ------------------------------------------------------------- access

    fn node(&self, h: Handle) -> Result<&Node> {
        self.nodes.get(h).ok_or(Error::InvalidHandle)
    }

    fn node_mut(&mut self, h: Handle) -> Result<&mut Node> {
        self.dirty = true;
        self.nodes.get_mut(h).ok_or(Error::InvalidHandle)
    }

    /// The class name of a live object.
    pub fn class_of(&self, h: Handle) -> Result<&'static str> {
        Ok(self.node(h)?.kind.class())
    }

    /// Whether `h` is an instance of `class` ("Widget" matches everything).
    pub fn is_a(&self, h: Handle, class: &str) -> Result<bool> {
        // a text area is a text field with more lines
        let actual = self.class_of(h)?;
        // likewise a table, a tree view and an icon view are list boxes
        let is_list = class == "ListBox" && matches!(actual, "Table" | "TreeView" | "IconView");
        // a group box, a toolbar and a status bar are stacks in dress, and a combo box a text field with choices
        let dressed = self.dressed_as(h) == Some(class);
        Ok(class == "Widget" || actual == class || (class == "TextField" && actual == "TextArea") || is_list || dressed)
    }

    /// Whether anything has changed since the last `take_dirty`.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn windows(&self) -> Vec<WindowInfo> {
        self.nodes
            .handles()
            .into_iter()
            .filter_map(|h| match &self.nodes.get(h)?.kind {
                Kind::Window { title, tag, width, height, visible, .. } => {
                    Some(WindowInfo { handle: h, title: title.clone(), tag: tag.clone(), width: *width, height: *height, visible: *visible })
                }
                _ => None,
            })
            .collect()
    }

    // ---------------------------------------------------------- tree edits

    /// Destroy a widget and everything inside it. Returns every handle that
    /// became invalid, so the caller can drop what it keyed on them.
    pub fn destroy(&mut self, h: Handle) -> Result<Vec<Handle>> {
        let parent = self.node(h)?.parent;
        if let Some(p) = parent {
            if let Some(node) = self.nodes.get_mut(p) {
                if node.context_menu == Some(h) {
                    node.context_menu = None;
                }
                match &mut node.kind {
                    Kind::Window { content, menu_bar, .. } => {
                        if *content == Some(h) {
                            *content = None;
                        }
                        if *menu_bar == Some(h) {
                            *menu_bar = None;
                        }
                    }
                    Kind::Stack { children, .. } => children.retain(|c| *c != h),
                    Kind::ScrollView { content, .. } if *content == Some(h) => *content = None,
                    Kind::Popup { menu, .. } if *menu == Some(h) => *menu = None,
                    Kind::Tabs { pages, selected } => {
                        pages.retain(|p| p.1 != h);
                        *selected = (*selected).min(pages.len().saturating_sub(1));
                    }
                    Kind::MenuBar { menus } => menus.retain(|c| *c != h),
                    Kind::Menu { items, .. } => items.retain(|c| *c != h),
                    Kind::MenuItem { submenu, .. } if *submenu == Some(h) => *submenu = None,
                    _ => {}
                }
            }
        }
        let mut gone = Vec::new();
        let mut pending = vec![h];
        while let Some(next) = pending.pop() {
            self.forget(next);
            if let Some(node) = self.nodes.remove(next) {
                pending.extend(node.context_menu);
                match node.kind {
                    Kind::Window { content, menu_bar, .. } => pending.extend(content.into_iter().chain(menu_bar)),
                    Kind::Stack { children, .. } => pending.extend(children),
                    Kind::ScrollView { content, .. } => pending.extend(content),
                    Kind::Popup { menu, .. } => pending.extend(menu),
                    Kind::Tabs { pages, .. } => pending.extend(pages.into_iter().map(|p| p.1)),
                    Kind::MenuBar { menus } => pending.extend(menus),
                    Kind::Menu { items, .. } => pending.extend(items),
                    Kind::MenuItem { submenu, .. } => pending.extend(submenu),
                    _ => {}
                }
                gone.push(next);
            }
        }
        if self.pressed.is_some_and(|(p, _)| gone.contains(&p)) {
            self.pressed = None;
        }
        if self.flash.is_some_and(|(f, _)| gone.contains(&f)) {
            self.flash = None;
        }
        self.preedit = self.preedit.take().filter(|(f, _)| !gone.contains(f));
        if self.bar_press.as_ref().is_some_and(|p| gone.contains(&p.owner)) {
            self.bar_press = None;
        }
        self.slider_drag = self.slider_drag.filter(|(s, _)| !gone.contains(s));
        self.divider_drag = self.divider_drag.filter(|(d, _)| !gone.contains(d));
        self.picture_drag = self.picture_drag.filter(|p| !gone.contains(p));
        self.picture_right = self.picture_right.filter(|p| !gone.contains(p));
        self.picture_middle = self.picture_middle.filter(|p| !gone.contains(p));
        self.list_edit = self.list_edit.take().filter(|e| !gone.contains(&e.list));
        self.column_drag = self.column_drag.filter(|d| !gone.contains(&d.0));
        self.row_drag = self.row_drag.filter(|d| !gone.contains(&d.from) && !gone.contains(&d.to));
        self.list_collapse = self.list_collapse.filter(|c| !gone.contains(&c.0));
        self.foreign_drop = self.foreign_drop.filter(|f| !gone.contains(&f.0));
        self.list_drag = self.list_drag.filter(|l| !gone.contains(l));
        self.drop_mark = self.drop_mark.filter(|(f, _)| !gone.contains(f));
        self.drag_candidate = self.drag_candidate.filter(|(f, ..)| !gone.contains(f));
        // An open menu that lost any of its levels, or an item it was
        // pointing at, is simply put away.
        let broken = |m: &MenuOpen| gone.contains(&m.window) || m.levels.iter().any(|l| gone.contains(&l.menu) || l.hot.is_some_and(|h| gone.contains(&h)));
        if self.menu_open.as_ref().is_some_and(broken) {
            self.close_menu();
        }
        self.title_flash = self.title_flash.filter(|(m, _)| !gone.contains(m));
        // A window whose focused widget went away simply has no focus;
        // focus is checked for liveness wherever it is read.
        self.dirty = true;
        Ok(gone)
    }

    pub fn destroy_all(&mut self) -> Vec<Handle> {
        let all = self.nodes.handles();
        for h in &all {
            self.forget(*h);
            self.nodes.remove(*h);
        }
        self.pressed = None;
        self.flash = None;
        self.menu_open = None;
        self.menu_blink = None;
        self.title_flash = None;
        self.edit_menu = None;
        self.preedit = None;
        self.bar_press = None;
        self.dirty = true;
        all
    }

    /// A child must be a free-standing, non-window widget.
    fn adoptable(&self, parent: Handle, child: Handle) -> Result<()> {
        let node = self.node(child)?;
        if matches!(node.kind, Kind::Window { .. }) || node.kind.is_menu_part() {
            return Err(Error::WrongType);
        }
        if node.parent.is_some() {
            return Err(Error::InvalidState);
        }
        // refuse to make a widget its own ancestor
        let mut up = Some(parent);
        while let Some(h) = up {
            if h == child {
                return Err(Error::InvalidArgument);
            }
            up = self.node(h)?.parent;
        }
        Ok(())
    }

    pub fn window_set_content(&mut self, window: Handle, child: Handle) -> Result<()> {
        if !matches!(self.node(window)?.kind, Kind::Window { .. }) {
            return Err(Error::WrongType);
        }
        self.adoptable(window, child)?;
        let old = match &mut self.node_mut(window)?.kind {
            Kind::Window { content, .. } => content.replace(child),
            _ => unreachable!(),
        };
        if let Some(old) = old {
            if let Some(n) = self.nodes.get_mut(old) {
                n.parent = None;
            }
        }
        self.node_mut(child)?.parent = Some(window);
        Ok(())
    }

    pub fn stack_add(&mut self, stack: Handle, child: Handle) -> Result<()> {
        if !matches!(self.node(stack)?.kind, Kind::Stack { .. }) {
            return Err(Error::WrongType);
        }
        self.adoptable(stack, child)?;
        if let Kind::Stack { children, .. } = &mut self.node_mut(stack)?.kind {
            children.push(child);
        }
        self.node_mut(child)?.parent = Some(stack);
        Ok(())
    }

    /// Put a child at a place among a stack's children, counting from 0,
    /// those after it moving along. A place past the end is the end. A
    /// child already in this stack is moved to the place.
    pub fn stack_insert(&mut self, stack: Handle, child: Handle, index: i32) -> Result<()> {
        if !matches!(self.node(stack)?.kind, Kind::Stack { .. }) {
            return Err(Error::WrongType);
        }
        if index < 0 {
            return Err(Error::InvalidArgument);
        }
        if self.node(child)?.parent != Some(stack) {
            self.adoptable(stack, child)?;
        }
        if let Kind::Stack { children, .. } = &mut self.node_mut(stack)?.kind {
            children.retain(|c| *c != child);
            children.insert((index as usize).min(children.len()), child);
        }
        self.node_mut(child)?.parent = Some(stack);
        self.dirty = true;
        Ok(())
    }

    /// Make a window that has not been shown the size its content asks
    /// for, with room for its menu bar if it draws one, and no smaller than
    /// a window may be.
    pub fn window_fit(&mut self, window: Handle) -> Result<()> {
        // a window already showing is made the size, as set_size makes it
        if let Kind::Window { visible: true, content, .. } = &self.node(window)?.kind {
            let (w, h) = content.map_or((0, 0), |c| self.measure(c));
            return self.window_set_size(window, w.clamp(64, 16384), h.clamp(32, 16384));
        }
        let (content, bar) = match &self.node(window)?.kind {
            Kind::Window { content, menu_bar, bar_external, .. } => (*content, if menu_bar.is_some() && !bar_external { self.look.m.menu_bar_height } else { 0 }),
            _ => return Err(Error::WrongType),
        };
        let (w, h) = content.map_or((0, 0), |c| self.measure(c));
        if let Kind::Window { width, height, .. } = &mut self.node_mut(window)?.kind {
            *width = w.clamp(64, 16384);
            *height = (h + bar).clamp(32, 16384);
        }
        self.dirty = true;
        Ok(())
    }

    /// The widget in a window that has the keyboard, if any has. A window
    /// that has not chosen yet answers with the text field it would start
    /// with, and is left still to choose.
    pub fn window_focused(&self, window: Handle) -> Result<Option<Handle>> {
        match self.node(window)?.kind {
            Kind::Window { auto_focus: true, content: Some(_), .. } => Ok(self
                .focus_order(window)
                .into_iter()
                .find(|h| matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::TextField { .. })))),
            Kind::Window { .. } => Ok(self.live_focus(window)),
            _ => Err(Error::WrongType),
        }
    }

    // ---------------------------------------------------------- properties

    pub fn set_enabled(&mut self, h: Handle, enabled: bool) -> Result<()> {
        self.node_mut(h)?.enabled = enabled;
        Ok(())
    }

    /// Say how small a widget may be, at the least. It is still as large
    /// as its kind makes it if that is larger.
    pub fn set_min_size(&mut self, h: Handle, width: i32, height: i32) -> Result<()> {
        if !(0..=16384).contains(&width) || !(0..=16384).contains(&height) {
            return Err(Error::InvalidArgument);
        }
        self.node_mut(h)?.least = (width, height);
        self.dirty = true;
        Ok(())
    }

    /// Set the text of a label, a button, a checkbox, a text field or a text
    /// area in another of the bundled typefaces.
    pub fn set_typeface(&mut self, h: Handle, face: Face) -> Result<()> {
        match self.node(h)?.kind {
            Kind::Label { .. } | Kind::Button { .. } | Kind::Checkbox { .. } | Kind::TextField { .. } | Kind::Picture { .. } => {}
            _ => return Err(Error::WrongType),
        }
        self.node_mut(h)?.face = (face != Face::default()).then_some(face);
        self.dirty = true;
        Ok(())
    }

    /// Do something with a widget's own typeface as the one in use, and put
    /// the interface's back afterwards. Everything that measures, draws or
    /// finds a place in a widget's text goes through here.
    fn in_face<R>(&mut self, h: Handle, f: impl FnOnce(&mut Self) -> R) -> R {
        let want = self.nodes.get(h).and_then(|n| n.face).unwrap_or_default();
        self.wearing(want, f)
    }

    /// Do something with a given typeface worn.
    fn wearing<R>(&mut self, want: Face, f: impl FnOnce(&mut Self) -> R) -> R {
        if want == self.worn {
            return f(self);
        }
        let was = self.worn;
        let font = self.wardrobe.remove(&want).unwrap_or_else(|| self.look.font(want));
        let old = std::mem::replace(&mut self.font, font);
        self.wardrobe.insert(was, old);
        self.worn = want;
        let out = f(self);
        let back = self.wardrobe.remove(&was).unwrap_or_else(|| self.look.font(was));
        let worn = std::mem::replace(&mut self.font, back);
        self.wardrobe.insert(want, worn);
        self.worn = was;
        out
    }

    /// Say whether a widget takes up spare room, overriding what its kind
    /// would do.
    pub fn set_stretch(&mut self, h: Handle, stretch: bool) -> Result<()> {
        self.node_mut(h)?.stretch = Some(stretch);
        self.dirty = true;
        Ok(())
    }

    /// Whether a widget grows, down the window (`vertical`) or across it,
    /// when there is room to spare. Things that show more when larger do:
    /// lists, text areas, scroll views and tab views both ways; one-line
    /// fields and progress bars across; sliders and scroll bars along their
    /// length. A stack grows if anything in it does.
    fn grows(&self, h: Handle, vertical: bool) -> bool {
        let Some(node) = self.nodes.get(h).filter(|_| !self.natural) else { return false };
        match node.stretch {
            Some(false) => return false,
            // told to grow, it grows along the stack it is in; across that
            // stack it does as its kind does, so that a spacer in a row of
            // buttons does not make the row taller
            Some(true) => {
                let along = match node.parent.and_then(|p| self.nodes.get(p)).map(|p| &p.kind) {
                    Some(Kind::Stack { orientation, .. }) => (*orientation == Orientation::Vertical) == vertical,
                    _ => true,
                };
                // a picture told to grow fills its room both ways, as a
                // terminal's screen does
                if along || matches!(node.kind, Kind::Picture { .. }) {
                    return true;
                }
            }
            None => {}
        }
        match &node.kind {
            Kind::List { .. } | Kind::ScrollView { .. } | Kind::Tabs { .. } => true,
            Kind::TextField { multiline, .. } => *multiline || !vertical,
            Kind::Progress { .. } => !vertical,
            Kind::Slider { orientation, .. } | Kind::ScrollBar { orientation, .. } => (*orientation == Orientation::Vertical) == vertical,
            // a divider reaches right across the stack it divides
            Kind::Divider { orientation, .. } => (*orientation == Orientation::Vertical) != vertical,
            Kind::Stack { children, .. } => children.iter().any(|c| self.grows(*c, vertical)),
            _ => false,
        }
    }

    pub fn enabled(&self, h: Handle) -> Result<bool> {
        Ok(self.node(h)?.enabled)
    }

    pub fn window_set_title(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Window { title, .. } => {
                *title = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Give a window a name the desktop can know it by again.
    pub fn window_set_tag(&mut self, h: Handle, value: &str) -> Result<()> {
        if value.len() > 4096 || value.chars().any(char::is_control) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Window { tag, .. } => {
                *tag = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn window_show(&mut self, h: Handle) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Window { visible, .. } => {
                *visible = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Say whether a window is waiting on something the program is doing:
    /// while it is, the pointer over it is an hourglass.
    pub fn window_set_busy(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Window { busy, .. } => {
                *busy = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Say whether the user may change a window's size.
    pub fn window_set_resizable(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Window { resizable, .. } => {
                *resizable = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Ask for a window to fill its screen, or to stop. The backend passes
    /// the wish on to the desktop; until the desktop answers, this is what
    /// the window is taken to be. While it is, the window draws no menu bar.
    pub fn window_set_full_screen(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Window { full_screen, .. } => *full_screen = value,
            _ => return Err(Error::WrongType),
        }
        self.dismiss_menu(h);
        self.dirty = true;
        Ok(())
    }

    /// Whether a window fills its screen, or has asked to.
    pub fn window_full_screen(&self, h: Handle) -> Result<bool> {
        match &self.node(h)?.kind {
            Kind::Window { full_screen, .. } => Ok(*full_screen),
            _ => Err(Error::WrongType),
        }
    }

    /// The desktop says whether a window now fills its screen: if that is
    /// news, note it and tell the program with `full_screen_changed`.
    pub fn window_note_full_screen(&mut self, h: Handle, value: bool) -> Result<Vec<Emitted>> {
        if self.window_full_screen(h)? == value {
            return Ok(Vec::new());
        }
        self.window_set_full_screen(h, value)?;
        Ok(vec![Emitted { target: h, event: "Window.full_screen_changed", args: vec![Value::Bool(value)] }])
    }

    /// The one size a window that cannot be resized may be: the size it
    /// was made (less a menu bar the desktop has since taken to show
    /// elsewhere). Nothing for a window that can be resized.
    pub fn window_fixed_size(&self, h: Handle) -> Result<Option<(i32, i32)>> {
        match &self.node(h)?.kind {
            Kind::Window { resizable: false, width, height, .. } => Ok(Some((*width, *height))),
            Kind::Window { .. } => Ok(None),
            _ => Err(Error::WrongType),
        }
    }

    /// The smallest a window can be made without cutting into its content,
    /// or its present size if the program has already made it smaller.
    pub fn window_min_size(&mut self, h: Handle) -> Result<(i32, i32)> {
        let (content, bar, now) = match &self.node(h)?.kind {
            Kind::Window { content, menu_bar, bar_external, full_screen, width, height, .. } => {
                // the size kept for the window is the whole of it, with a bar only if drawn in it
                (*content, if menu_bar.is_some() && !bar_external && !full_screen { self.look.m.menu_bar_height } else { 0 }, (*width, *height))
            }
            _ => return Err(Error::WrongType),
        };
        let (w, h) = content.map_or((0, 0), |c| self.measure(c));
        Ok((w.min(now.0).max(64), (h + bar).min(now.1).max(32)))
    }

    /// Record the size the display gave the window.
    /// The program gives a window a new size, whether or not it is showing:
    /// the size of its inside, as `window_new` takes it. A window of one
    /// size stays of one size, the new one.
    pub fn window_set_size(&mut self, h: Handle, w: i32, h_px: i32) -> Result<()> {
        if !(1..=16384).contains(&w) || !(1..=16384).contains(&h_px) {
            return Err(Error::InvalidArgument);
        }
        // with room for its menu bar, where that is in the window, as fit leaves it
        let bar = match &self.node(h)?.kind {
            Kind::Window { menu_bar, bar_external, .. } if menu_bar.is_some() && !bar_external => self.look.m.menu_bar_height,
            Kind::Window { .. } => 0,
            _ => return Err(Error::WrongType),
        };
        self.window_resize(h, w, h_px + bar)?;
        if !self.resized.contains(&h) {
            self.resized.push(h);
        }
        self.dirty = true;
        Ok(())
    }

    /// The size of a window's inside.
    pub fn window_size(&self, h: Handle) -> Result<(i32, i32)> {
        match &self.node(h)?.kind {
            Kind::Window { width, height, .. } => Ok((*width, *height)),
            _ => Err(Error::WrongType),
        }
    }

    /// Whether the program has given a window a new size that the display
    /// has still to make it; asking clears it.
    pub fn take_resized(&mut self, h: Handle) -> bool {
        let was = self.resized.contains(&h);
        self.resized.retain(|r| *r != h);
        was
    }

    pub fn window_resize(&mut self, h: Handle, w: i32, h_px: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Window { width, height, .. } => {
                *width = w.max(1);
                *height = h_px.max(1);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn stack_set_spacing(&mut self, h: Handle, value: i32) -> Result<()> {
        if !(0..=4096).contains(&value) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Stack { spacing, .. } => {
                *spacing = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn stack_set_padding(&mut self, h: Handle, value: i32) -> Result<()> {
        if !(0..=4096).contains(&value) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Stack { padding, .. } => {
                *padding = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn stack_set_align(&mut self, h: Handle, value: Align) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Stack { align, .. } => {
                *align = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn label_set_text(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Label { text } => {
                *text = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn button_set_label(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Button { label, .. } => {
                *label = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn checkbox_set_label(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Checkbox { label, .. } => {
                *label = value.into();
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn button_set_default(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Button { is_default, .. } => {
                *is_default = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn button_set_cancel(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Button { is_cancel, .. } => {
                *is_cancel = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// An event from a text field. It carries the field's text as it now
    /// stands, so a handler need not ask for it.
    fn field_event(&self, h: Handle, event: &'static str) -> Emitted {
        let text = self.text_field_text(h).unwrap_or_default();
        Emitted { target: h, event, args: vec![Value::Str(text)] }
    }

    fn editor_mut(&mut self, h: Handle) -> Result<&mut Editor> {
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { edit, goal_x, follow, .. } => {
                // whatever is about to happen to the text, the view follows
                // the caret again and a run of ups and downs is over
                *goal_x = None;
                *follow = true;
                Ok(edit)
            }
            _ => Err(Error::WrongType),
        }
    }

    fn is_multiline(&self, h: Handle) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::TextField { multiline: true, .. }))
    }

    fn is_password(&self, h: Handle) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::TextField { password: true, .. }))
    }

    /// Show bullets in place of the characters. A password cannot be copied,
    /// cut or dragged out, and input methods are kept away from it.
    pub fn text_field_set_password(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { multiline: true, .. } => Err(Error::InvalidState),
            Kind::TextField { password, .. } => {
                *password = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Set the height of a text area, in lines of text.
    /// Where a field's text sits across it; Fill is taken as Start.
    pub fn text_field_set_align(&mut self, h: Handle, value: Align) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { align, .. } => *align = if value == Align::Fill { Align::Start } else { value },
            _ => return Err(Error::WrongType),
        }
        self.dirty = true;
        Ok(())
    }

    /// Draw a text area with no frame and no scroll bar, as writing on a page.
    pub fn text_area_set_plain(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { multiline: true, plain, .. } => *plain = value,
            _ => return Err(Error::WrongType),
        }
        self.dirty = true;
        Ok(())
    }

    pub fn text_area_set_rows(&mut self, h: Handle, value: i32) -> Result<()> {
        if !(1..=1000).contains(&value) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { multiline: true, rows, .. } => {
                *rows = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Replace a field's text from the program. This is not an edit by the
    /// user, so nothing is emitted.
    pub fn text_field_set_text(&mut self, h: Handle, value: &str) -> Result<()> {
        self.editor_mut(h)?.set_text(value);
        self.known(h);
        Ok(())
    }

    /// Select from `from` to `to`, byte offsets into the text: `to` is where
    /// the caret goes, scrolled into view. From the program, so it is not
    /// reported back as a selected event.
    pub fn text_field_select(&mut self, h: Handle, from: i32, to: i32) -> Result<()> {
        if from < 0 || to < 0 {
            return Err(Error::InvalidArgument);
        }
        self.editor_mut(h)?.select(from as usize, to as usize);
        self.caret_epoch = self.now;
        self.known(h);
        Ok(())
    }

    /// The selection, start first, as byte offsets into the text.
    pub fn text_field_selection(&self, h: Handle) -> Result<(i32, i32)> {
        match &self.node(h)?.kind {
            Kind::TextField { edit, .. } => {
                let (from, to) = edit.selection();
                Ok((from as i32, to as i32))
            }
            _ => Err(Error::WrongType),
        }
    }

    /// The program knows the field's selection as it now is.
    fn known(&mut self, h: Handle) {
        if let Some(Kind::TextField { edit, reported, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *reported = (edit.anchor(), edit.caret());
        }
    }

    /// The text fields whose caret or selection has moved since the program
    /// was last told: by typing, the keys, the pointer, a paste. Each says
    /// where its selection now begins and ends, start first; equal ends are
    /// the caret alone. Password fields keep their caret to themselves.
    pub fn selection_events(&mut self) -> Vec<Emitted> {
        let mut out = Vec::new();
        for h in self.nodes.handles() {
            if let Some(Kind::TextField { edit, reported, password: false, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
                let now = (edit.anchor(), edit.caret());
                if *reported != now {
                    *reported = now;
                    let (from, to) = edit.selection();
                    out.push(Emitted { target: h, event: "TextField.selected", args: vec![Value::I32(from as i32), Value::I32(to as i32)] });
                }
            }
        }
        out
    }

    pub fn text_field_text(&self, h: Handle) -> Result<String> {
        match &self.node(h)?.kind {
            Kind::TextField { edit, .. } => Ok(edit.text().to_string()),
            _ => Err(Error::WrongType),
        }
    }

    pub fn text_field_set_placeholder(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { placeholder, .. } => {
                *placeholder = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn text_field_set_min_width(&mut self, h: Handle, value: i32) -> Result<()> {
        if !(16..=16384).contains(&value) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::TextField { min_width, .. } => {
                *min_width = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn text_field_select_all(&mut self, h: Handle) -> Result<()> {
        self.editor_mut(h)?.select_all();
        self.known(h);
        Ok(())
    }

    // ----------------------------------------------------------- clipboard
    //
    // The engine does not own a clipboard. A copy leaves its text for the
    // caller to collect with `take_copy`; a paste leaves a request, which the
    // caller answers with `paste_into` once it has the text. On a display
    // that can take a moment, so the two halves are separate.

    /// Copy the selection. With nothing selected the clipboard is left alone.
    pub fn text_field_copy(&mut self, h: Handle) -> Result<()> {
        let secret = self.is_password(h);
        let edit = self.editor_mut(h)?;
        if edit.has_selection() && !secret {
            self.copy_out = Some(edit.selected().to_string());
        }
        Ok(())
    }

    /// Copy the selection, then delete it.
    pub fn text_field_cut(&mut self, h: Handle) -> Result<Vec<Emitted>> {
        self.text_field_copy(h)?;
        let mut out = Vec::new();
        // a password is not put on the clipboard, so it is not cut either
        if !self.is_password(h) && self.editor_mut(h)?.backspace_selection() {
            out.push(self.field_event(h, "TextField.changed"));
        }
        Ok(out)
    }

    /// Ask for the clipboard's text to be pasted over the selection.
    pub fn text_field_request_paste(&mut self, h: Handle) -> Result<()> {
        self.editor_mut(h)?;
        self.paste_request = Some(h);
        Ok(())
    }

    /// Answer a paste request. Line breaks arrive as spaces.
    pub fn paste_into(&mut self, h: Handle, text: &str) -> Result<Vec<Emitted>> {
        let mut out = Vec::new();
        // a cell being edited in a list takes a paste as a text field would;
        // the list reports the change when the edit is kept
        if self.list_editing(h) {
            self.list_edit_text(text);
            return Ok(out);
        }
        if let Some(out) = self.rich_paste_text(h, text) {
            return Ok(out);
        }
        if self.editor_mut(h)?.insert(text) {
            out.push(self.field_event(h, "TextField.changed"));
        }
        self.caret_epoch = self.now;
        Ok(out)
    }

    pub fn take_copy(&mut self) -> Option<String> {
        self.copy_out.take()
    }

    pub fn take_paste_request(&mut self) -> Option<Handle> {
        self.paste_request.take()
    }

    pub fn checkbox_set_checked(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Checkbox { checked, .. } => {
                *checked = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn checkbox_checked(&self, h: Handle) -> Result<bool> {
        match &self.node(h)?.kind {
            Kind::Checkbox { checked, .. } => Ok(*checked),
            _ => Err(Error::WrongType),
        }
    }

    // -------------------------------------------------------------- layout

    fn children(&self, h: Handle) -> Vec<Handle> {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Window { content, .. }) => content.iter().copied().collect(),
            Some(Kind::Stack { children, .. }) if self.runs_backwards(h) => children.iter().rev().copied().collect(),
            Some(Kind::Stack { children, .. }) => children.clone(),
            Some(Kind::ScrollView { content, .. }) => content.iter().copied().collect(),
            // only the page that is showing takes part in layout, drawing and focus
            Some(Kind::Tabs { .. }) => self.tab_page(h).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    /// The size a widget would like to be.
    fn measure(&mut self, h: Handle) -> (i32, i32) {
        let least = self.nodes.get(h).map_or((0, 0), |n| n.least);
        let (w, tall) = self.natural_size(h);
        (w.max(least.0), tall.max(least.1))
    }

    /// The size a widget's kind would make it.
    /// The size a widget's kind would make it, in its own typeface.
    fn natural_size(&mut self, h: Handle) -> (i32, i32) {
        self.in_face(h, |ui| ui.natural_size_in(h))
    }

    fn natural_size_in(&mut self, h: Handle) -> (i32, i32) {
        let line = self.font.line_height();
        let m = self.look.m;
        let Some(node) = self.nodes.get(h) else { return (0, 0) };
        match &node.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            Kind::MenuBar { .. } | Kind::Menu { .. } | Kind::MenuItem { .. } => (0, 0),
            Kind::Divider { orientation: Orientation::Vertical, .. } => (0, divider::THICK),
            Kind::Divider { .. } => (divider::THICK, 0),
            Kind::ScrollView { width, height, .. } => (*width, *height),
            Kind::Popup { .. } => self.measure_popup(h),
            Kind::Progress { .. } => self.measure_progress(h),
            Kind::Picture { .. } => self.measure_picture(h),
            Kind::Tabs { .. } => self.measure_tabs(h),
            Kind::Radio { .. } => self.measure_radio(h),
            Kind::Slider { .. } => self.measure_slider(h),
            Kind::List { .. } => self.measure_list(h),
            Kind::ScrollBar { orientation, length, .. } => {
                if *orientation == Orientation::Vertical { (m.scroll_bar, *length) } else { (*length, m.scroll_bar) }
            }
            Kind::Spin { .. } => self.measure_stepper(),
            Kind::RichText(_) => self.measure_rich(h),
            Kind::Label { text } => {
                let text = text.clone();
                (self.font.width(&text), line)
            }
            Kind::TextField { min_width, multiline, rows, .. } => (*min_width, line * if *multiline { *rows } else { 1 } + 2 * m.field_inset_y),
            Kind::Button { label, is_default, .. } => {
                let (label, ring) = (label.clone(), if *is_default { 2 * m.default_ring } else { 0 });
                // with room for its picture; a tool in a bar is no wider than it needs
                let label = self.button_text(h, &label);
                let least = if self.is_flat(h) { m.button_height } else { m.button_min_width };
                let pad = if self.is_flat(h) { m.button_pad / 2 } else { m.button_pad };
                ((self.font.width(&label) + 2 * pad).max(least) + ring, m.button_height.max(self.button_image_height(h)) + ring)
            }
            Kind::Checkbox { label, .. } => {
                let label = label.clone();
                (m.check_size + m.check_gap + self.font.width(&label), line.max(m.check_size))
            }
            Kind::Stack { orientation, spacing, padding, children, .. } => {
                let (orientation, spacing, padding, children) = (*orientation, *spacing, *padding, children.clone());
                let (dl, dt, dr, db) = self.dress_insets(h);
                let (mut along, mut across) = (0, 0);
                for (i, c) in children.iter().enumerate() {
                    let (w, h) = self.measure(*c);
                    let (a, x) = if orientation == Orientation::Vertical { (h, w) } else { (w, h) };
                    along += a + if i > 0 { spacing } else { 0 };
                    across = across.max(x);
                }
                let (along, across) = (along + 2 * padding, across + 2 * padding);
                let (w, tall) = if orientation == Orientation::Vertical { (across, along) } else { (along, across) };
                (w + dl + dr, tall + dt + db)
            }
        }
    }

    fn arrange(&mut self, h: Handle, rect: Rect) {
        let Some(node) = self.nodes.get_mut(h) else { return };
        node.rect = rect;
        match &node.kind {
            Kind::Window { content: Some(c), menu_bar, bar_external, full_screen, .. } => {
                // the menu bar takes its height from the top of the window,
                // unless the desktop is showing it somewhere else or the
                // window fills the screen
                let (c, bar) = (*c, if menu_bar.is_some() && !bar_external && !full_screen { self.look.m.menu_bar_height } else { 0 });
                self.arrange(c, Rect::new(rect.x, rect.y + bar, rect.w, (rect.h - bar).max(0)));
            }
            Kind::ScrollView { .. } => self.arrange_view(h),
            Kind::Tabs { .. } => self.arrange_tabs(h, rect),
            Kind::Stack { orientation, spacing, padding, align, children } => {
                let (orientation, spacing, align, mut children, padding) = (*orientation, *spacing, *align, children.clone(), *padding);
                if self.runs_backwards(h) {
                    children.reverse();
                }
                // inside its dress, if it wears one
                let (dl, dt, dr, db) = self.dress_insets(h);
                let inner = Rect::new(rect.x + dl, rect.y + dt, (rect.w - dl - dr).max(0), (rect.h - dt - db).max(0)).inset(padding);
                let vertical = orientation == Orientation::Vertical;
                let mut pos = if vertical { inner.y } else { inner.x };
                let room = if vertical { inner.w } else { inner.h };
                // room left over along the stack is shared out among the
                // children that grow, any odd pixels going to the first of them
                // a pane before a divider that has been dragged is as long as it was left
                let fixed = self.fixed_by_dividers(&children);
                let sizes: Vec<(i32, i32)> = children.iter().zip(&fixed).map(|(c, f)| {
                    let (w, hh) = self.measure(*c);
                    match f {
                        Some(f) if vertical => (w, *f),
                        Some(f) => (*f, hh),
                        None => (w, hh),
                    }
                }).collect();
                let needed: i32 = sizes.iter().map(|(w, hh)| if vertical { *hh } else { *w }).sum::<i32>() + spacing * (children.len() as i32 - 1).max(0);
                let growers = children.iter().zip(&fixed).filter(|(c, f)| f.is_none() && self.grows(**c, vertical)).count() as i32;
                let spare = ((if vertical { inner.h } else { inner.w }) - needed).max(0);
                // a row of a dialog's buttons stands at the far end of its room
                if growers == 0 && self.is_button_row(h) {
                    pos += spare;
                }
                let mut shared = 0;
                for ((c, (w, hh)), fixed) in children.into_iter().zip(sizes).zip(fixed) {
                    let (mut along, across) = if vertical { (hh, w) } else { (w, hh) };
                    if growers > 0 && fixed.is_none() && self.grows(c, vertical) {
                        along += spare / growers + i32::from(shared < spare % growers);
                        shared += 1;
                    }
                    // across the stack, a child that grows takes the full width
                    let align = if self.grows(c, !vertical) { Align::Fill } else { align };
                    let (offset, size) = match align {
                        Align::Start => (0, across),
                        Align::Center => ((room - across) / 2, across),
                        Align::End => (room - across, across),
                        Align::Fill => (0, room),
                    };
                    let r = if vertical {
                        Rect::new(inner.x + offset, pos, size, along)
                    } else {
                        Rect::new(pos, inner.y + offset, along, size)
                    };
                    self.arrange(c, r);
                    pos += along + spacing;
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------- drawing

    /// Lay out and draw a window's content at its current size.
    pub fn render(&mut self, window: Handle) -> Result<Canvas> {
        let (w, h) = match &self.node(window)?.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            _ => return Err(Error::WrongType),
        };
        self.arrange(window, Rect::new(0, 0, w, h));
        let ctx = Paint { focus: self.resolve_focus(window), active: self.is_active(window) };
        // a widget that has just taken the keyboard is scrolled into view
        if let Some(target) = self.reveal.take() {
            self.reveal_now(window, target);
        }
        let mut canvas = self.sheet(w, h, WHITE);
        self.ground = self.look.pal.window;
        self.part(&mut canvas, Part::new(PartKind::Background, Rect::new(0, 0, w, h)));
        for c in self.children(window) {
            self.paint(c, &mut canvas, true, ctx);
        }
        self.paint_bar(window, &mut canvas);
        Ok(canvas)
    }

    fn paint(&mut self, h: Handle, canvas: &mut Canvas, parent_enabled: bool, ctx: Paint) {
        self.in_face(h, |ui| ui.paint_in(h, canvas, parent_enabled, ctx))
    }

    fn paint_in(&mut self, h: Handle, canvas: &mut Canvas, parent_enabled: bool, ctx: Paint) {
        let Some(node) = self.nodes.get(h) else { return };
        self.painting = Some(h);
        let rect = node.rect;
        let enabled = parent_enabled && node.enabled;
        let held = self.pressed == Some((h, true)) || self.flash.is_some_and(|(f, _)| f == h) || self.downs.contains(&h);
        // has the keyboard, and its window is the one being typed into
        let live = ctx.focus == Some(h) && ctx.active && enabled;
        let ants = self.ants_step() as i32;
        let line = self.font.line_height();
        match &node.kind {
            // a divider is the window's own ground showing between the panes
            Kind::Window { .. } | Kind::MenuBar { .. } | Kind::Menu { .. } | Kind::MenuItem { .. } | Kind::Divider { .. } => {}
            Kind::Stack { .. } => {
                if let Some(pattern) = self.backdrops.get(&h).copied() {
                    Self::lay_pattern(canvas, rect, &pattern, self.look.pal.window, self.look.pal.text);
                }
                self.paint_dress(h, rect, canvas);
                for c in self.children(h) {
                    self.paint(c, canvas, enabled, ctx);
                }
                return; // children grey themselves out
            }
            Kind::Spin { .. } => self.paint_stepper(h, rect, live, canvas),
            Kind::Label { text } => {
                let (text, color) = (text.clone(), self.look.pal.text);
                self.font.draw(canvas, &text, rect.x, rect.y + (rect.h - line) / 2, color);
            }
            Kind::Button { label, is_default, .. } => {
                let (label, is_default) = (label.clone(), *is_default);
                let label = self.button_text(h, &label);
                if self.is_flat(h) && !held && !live && self.hover != Some(h) {
                    // a tool in a bar, with nobody near it: its label and no edge
                    self.paint_flat(rect, &label, canvas);
                } else {
                    let part = Part::new(PartKind::Button, rect).text(&label).phase(ants).when(state::ENABLED, enabled).when(state::KEY, self.keys.contains(&h));
                    self.part(canvas, part.when(state::DEFAULT, is_default).when(state::HELD, held).when(state::FOCUSED, live));
                }
                self.paint_button_image(h, rect, &label, canvas);
            }
            Kind::Checkbox { label, checked } => {
                let (label, checked) = (label.clone(), *checked);
                let part = Part::new(PartKind::Checkbox, rect).text(&label).phase(ants);
                self.part(canvas, part.when(state::CHECKED, checked).when(state::HELD, held).when(state::FOCUSED, live));
            }
            Kind::TextField { .. } => self.paint_field(h, rect, live, canvas),
            Kind::RichText(_) => self.paint_rich(h, rect, live, canvas),
            Kind::ScrollBar { orientation, .. } => {
                let horizontal = *orientation == Orientation::Horizontal;
                self.bar_draw(h, horizontal, canvas);
                return; // a dead bar greys its own arrows
            }
            Kind::Popup { .. } => self.paint_popup(h, rect, live, canvas),
            Kind::Progress { .. } => self.paint_progress(h, rect, canvas),
            Kind::Picture { .. } => self.paint_picture(h, rect, live.then_some(ants), canvas),
            Kind::Tabs { .. } => {
                self.paint_tabs(h, rect, live, enabled, ctx, canvas);
                return; // its page greys itself out
            }
            Kind::Radio { .. } => self.paint_radio(h, rect, live, held, canvas),
            Kind::Slider { .. } => self.paint_slider(h, rect, live, canvas),
            Kind::List { .. } => self.paint_list(h, rect, live, canvas),
            Kind::ScrollView { .. } => {
                self.paint_view(h, rect, enabled, ctx, canvas);
                return; // its content greys itself out
            }
        }
        // a style that greys its own buttons has done so
        let greyed = matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Button { .. })) && self.look.wants & crate::style::wants::GREYS_BUTTONS != 0;
        if !enabled && !greyed {
            self.part(canvas, Part::new(PartKind::Dim, rect));
        }
    }

    // --------------------------------------------------------------- input

    /// The enabled, clickable widget under a point, if any.
    fn hit(&self, h: Handle, x: i32, y: i32) -> Option<Handle> {
        let node = self.nodes.get(h)?;
        if !node.enabled {
            return None;
        }
        match &node.kind {
            Kind::Window { .. } | Kind::Stack { .. } => self.children(h).into_iter().rev().find_map(|c| self.hit(c, x, y)),
            Kind::Tabs { .. } if node.rect.contains(x, y) => self.hit_tabs(h, x, y),
            Kind::ScrollView { .. } if node.rect.contains(x, y) => match self.view_port(h) {
                // inside the view, whatever of the content shows there; on
                // the frame and the bars, the view itself
                Some(port) if port.contains(x, y) => self.children(h).into_iter().find_map(|c| self.hit(c, x, y)),
                _ => Some(h),
            },
            Kind::Button { .. }
            | Kind::Checkbox { .. }
            | Kind::TextField { .. }
            | Kind::ScrollBar { .. }
            | Kind::Spin { .. }
            | Kind::Popup { .. }
            | Kind::Radio { .. }
            | Kind::Slider { .. }
            | Kind::Divider { .. }
            | Kind::List { .. }
            | Kind::Picture { .. }
            | Kind::RichText(_)
                if node.rect.contains(x, y) =>
            {
                Some(h)
            }
            _ => None,
        }
    }

    /// Say which modifier keys are held. They change what a click does in a
    /// list: Control adds to the selection and Shift stretches it.
    pub fn set_modifiers(&mut self, mods: Mods) {
        self.mods = mods;
    }

    /// Feed a pointer event, in content coordinates, to a window.
    pub fn pointer(&mut self, window: Handle, x: i32, y: i32, kind: PointerKind) -> Result<Vec<Emitted>> {
        // A press while a cell is being edited either lands in the cell, and
        // is all used up there, or ends the edit and then does what it would.
        let mut out = Vec::new();
        if kind == PointerKind::Press && self.list_edit.is_some() {
            let (used, events) = self.list_edit_press(x, y);
            out = events;
            if used {
                return Ok(out);
            }
        }
        out.extend(self.pointer_event(window, x, y, kind)?);
        Ok(out)
    }

    fn pointer_event(&mut self, window: Handle, x: i32, y: i32, kind: PointerKind) -> Result<Vec<Emitted>> {
        let (w, h) = match &self.node(window)?.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            _ => return Err(Error::WrongType),
        };
        // Hit-testing needs current geometry even if nothing has been drawn yet.
        self.arrange(window, Rect::new(0, 0, w, h));
        // the title in the menu bar the pointer is over, for a style that lights it
        if self.look.wants & crate::style::wants::HOVER != 0 {
            let over_title = if kind == PointerKind::Leave { None } else { self.bar_title_at(window, x, y) };
            if over_title != self.bar_hot {
                self.bar_hot = over_title;
                self.dirty = true;
            }
        }
        if let Some(out) = self.menu_pointer(window, x, y, kind) {
            return Ok(out);
        }
        let over = self.hit(window, x, y);
        let hover = over.filter(|_| kind != PointerKind::Leave);
        // which piece of it, where it has pieces a style may light one of
        let piece = if self.look.wants & crate::style::wants::HOVER != 0 { hover.map_or(0, |h| self.piece_at(h, x, y)) } else { 0 };
        self.hover_at = (x, y);
        if kind == PointerKind::Press {
            self.tip_dismiss();
        }
        if hover != self.hover || piece != self.hover_piece {
            // a tool in a bar shows its edge under the pointer in every style
            let tool = [hover, self.hover].into_iter().flatten().any(|h| self.is_flat(h) || matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Spin { .. })));
            if hover != self.hover {
                self.hover = hover;
                self.tip_reset();
            }
            self.hover_piece = piece;
            self.dirty |= tool || self.look.wants & crate::style::wants::HOVER != 0;
        }
        if let Some(out) = self.divider_pointer(over, x, y, kind) {
            return Ok(out);
        }
        if let Some(out) = self.stepper_pointer(over, x, y, kind) {
            return Ok(out);
        }
        if let Some(out) = self.combo_pointer(over, x, y, kind) {
            return Ok(out);
        }
        if let Some(out) = self.bar_pointer(over, x, y, kind) {
            return Ok(out);
        }
        if let Some(out) = self.control_pointer(over, x, y, kind) {
            return Ok(out);
        }
        if let Some(out) = self.picture_pointer(over, x, y, kind) {
            return Ok(out);
        }
        if let Some(out) = self.rich_pointer(window, over, x, y, kind) {
            return Ok(out);
        }
        // pressing a pop-up button pops its list up, which is a menu from then on
        if let (PointerKind::Press, Some(pop)) = (kind, over.filter(|h| matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::Popup { .. })))) {
            self.popup_open(pop, true);
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let is_field = |ui: &Ui, h: Handle| matches!(ui.nodes.get(h).map(|n| &n.kind), Some(Kind::TextField { .. }));
        match kind {
            PointerKind::Press => {
                self.clicks_at(x, y);
                self.pressed = over.map(|h| (h, true));
                self.dirty |= over.is_some();
                // A click in a text field moves the keyboard there. Buttons and
                // checkboxes act without taking it, so typing is not interrupted.
                if let Some(field) = over.filter(|h| is_field(self, *h)) {
                    if self.resolve_focus(window) != Some(field) {
                        self.set_focus(window, Some(field), false);
                    }
                    let at = self.index_at(field, x, y);
                    let count = self.clicks.3;
                    let secret = self.is_password(field);
                    let edit = self.editor_mut(field)?;
                    let (from, to) = edit.selection();
                    match count {
                        // A press on selected text may be the start of a drag;
                        // the caret is only moved if it turns out to be a click.
                        1 if at >= from && at < to && !secret => self.drag_candidate = Some((field, x, y)),
                        1 => edit.move_to(at, false),
                        // a password gives nothing away about where its words are
                        2 if !secret => edit.select_word_at(at),
                        _ => edit.select_all(),
                    }
                    self.caret_epoch = self.now;
                }
            }
            PointerKind::Move | PointerKind::Leave => {
                if let Some((field, px, py)) = self.drag_candidate {
                    if kind == PointerKind::Move && ((x - px).abs() > 3 || (y - py).abs() > 3) {
                        // far enough to be meant: the selection is on its way out
                        self.drag_candidate = None;
                        self.pressed = None;
                        self.drag_out = Some(self.editor_mut(field)?.selected().to_string());
                    }
                } else if let Some((h, _)) = self.pressed.filter(|(h, _)| is_field(self, *h)) {
                    // dragging selects; a double click keeps its word
                    if kind == PointerKind::Move && self.clicks.3 == 1 {
                        let at = self.index_at(h, x, y);
                        self.editor_mut(h)?.move_to(at, true);
                        self.caret_epoch = self.now;
                    }
                } else if let Some((h, was_over)) = self.pressed {
                    let is_over = kind == PointerKind::Move && over == Some(h);
                    if is_over != was_over {
                        self.pressed = Some((h, is_over));
                        self.dirty = true;
                    }
                }
            }
            PointerKind::Release => {
                if let Some((field, ..)) = self.drag_candidate.take() {
                    // it was only a click: put the caret where it landed
                    let at = self.index_at(field, x, y);
                    self.editor_mut(field)?.move_to(at, false);
                    self.caret_epoch = self.now;
                }
                if let Some((h, _)) = self.pressed.take() {
                    self.dirty = true;
                    if over == Some(h) {
                        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
                            Some(Kind::Button { .. }) => out.push(Emitted { target: h, event: "Button.clicked", args: vec![] }),
                            Some(Kind::Checkbox { checked, .. }) => {
                                *checked = !*checked;
                                out.push(Emitted { target: h, event: "Checkbox.toggled", args: vec![Value::Bool(*checked)] });
                            }
                            Some(Kind::Radio { .. }) => out.extend(self.radio_pick(h)),
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// Count a press towards a double or triple click: one more if it
    /// follows the last soon enough and near enough. Returns the count.
    fn clicks_at(&mut self, x: i32, y: i32) -> i32 {
        let (t, px, py, n) = self.clicks;
        let again = n > 0 && self.now - t <= DOUBLE_CLICK_MS && (x - px).abs() <= 3 && (y - py).abs() <= 3;
        self.clicks = (self.now, x, y, if again { n + 1 } else { 1 });
        self.clicks.3 as i32
    }

    // --------------------------------------------------------------- focus

    fn window_of(&self, h: Handle) -> Option<Handle> {
        let mut at = h;
        loop {
            let node = self.nodes.get(at)?;
            if matches!(node.kind, Kind::Window { .. }) {
                return Some(at);
            }
            at = node.parent?;
        }
    }

    /// A widget that can take the keyboard: the right kind, and enabled all
    /// the way up the tree.
    fn focusable(&self, h: Handle) -> bool {
        let Some(node) = self.nodes.get(h) else { return false };
        if !matches!(node.kind, Kind::TextField { .. } | Kind::Button { .. } | Kind::Checkbox { .. } | Kind::Popup { .. } | Kind::Radio { .. } | Kind::Slider { .. } | Kind::Spin { .. } | Kind::List { .. } | Kind::Tabs { .. } | Kind::Picture { keys: true, .. } | Kind::RichText(_)) {
            return false;
        }
        let mut up = Some(h);
        while let Some(at) = up {
            match self.nodes.get(at) {
                Some(n) if n.enabled => up = n.parent,
                _ => return false,
            }
        }
        // what is on a page that is not showing cannot have the keyboard
        !self.on_hidden_page(h)
    }

    /// The widgets Tab visits, in tree order.
    fn focus_order(&self, window: Handle) -> Vec<Handle> {
        let mut out = Vec::new();
        let mut pending = self.children(window);
        pending.reverse();
        while let Some(h) = pending.pop() {
            // a radio group is one stop: its chosen button, and the arrows do the rest
            if self.focusable(h) && self.radio_stop(h) {
                out.push(h);
            }
            pending.extend(self.children(h).into_iter().rev());
        }
        out
    }

    pub(super) fn is_active(&self, window: Handle) -> bool {
        matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { active: true, .. }))
    }

    /// The window's focus, if it is still alive and able to take keys.
    fn live_focus(&self, window: Handle) -> Option<Handle> {
        match self.nodes.get(window).map(|n| &n.kind) {
            Some(Kind::Window { focus, .. }) => focus.filter(|f| self.focusable(*f) && self.window_of(*f) == Some(window)),
            _ => None,
        }
    }

    /// Like `live_focus`, but first gives a window that has never had a
    /// focus its starting one: the first text field, with its text selected,
    /// as in a classic dialog. Buttons are reached with Tab.
    fn resolve_focus(&mut self, window: Handle) -> Option<Handle> {
        let pending = matches!(
            self.nodes.get(window).map(|n| &n.kind),
            Some(Kind::Window { auto_focus: true, content: Some(_), .. })
        );
        if pending {
            let first = self
                .focus_order(window)
                .into_iter()
                .find(|h| matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::TextField { .. })));
            self.set_focus(window, first, true);
        }
        self.live_focus(window)
    }

    fn set_focus(&mut self, window: Handle, target: Option<Handle>, select: bool) {
        if let Some(Kind::Window { focus, auto_focus, .. }) = self.nodes.get_mut(window).map(|n| &mut n.kind) {
            *focus = target;
            *auto_focus = false;
        }
        // Arriving in a one-line field selects its text, ready to be typed
        // over. A text area is for writing in, so its caret is left alone.
        if let (true, Some(Kind::TextField { edit, multiline: false, .. })) = (select, target.and_then(|t| self.nodes.get_mut(t)).map(|n| &mut n.kind)) {
            edit.select_all();
        }
        self.focus_epoch = self.now;
        self.caret_epoch = self.now;
        self.reveal = target; // scrolled into view when next drawn
        self.list_edit = None; // an edit left behind is abandoned; Tab and clicks keep it first
        self.preedit = None; // a composition does not follow the focus
        self.dirty = true;
    }

    // -------------------------------------------------------- input methods
    //
    // An input method composes text the keyboard cannot type directly. While
    // it works, it shows a draft (the "preedit") in the field; when the user
    // settles on something, it commits it.

    /// Whether the window's focus is a text field an input method can work
    /// in, and if so where its caret is, so the input method can put its
    /// candidate list beside it.
    pub fn ime_target(&mut self, window: Handle) -> Option<Rect> {
        // an input method would show, and might remember, what is typed
        let field = self.live_focus(window).filter(|f| !self.is_password(*f))?;
        // a cell being edited in a list is as good a place as a text field
        self.list_edit_caret(field).or_else(|| self.caret_rect(field))
    }

    /// Show the input method's draft in the focused field; empty clears it.
    pub fn ime_preedit(&mut self, window: Handle, text: &str) {
        let field = self.live_focus(window).filter(|f| matches!(self.nodes.get(*f).map(|n| &n.kind), Some(Kind::TextField { .. })) || self.list_editing(*f));
        let next = field.filter(|_| !text.is_empty()).map(|f| (f, text.to_string()));
        if self.preedit != next {
            self.preedit = next;
            self.caret_epoch = self.now;
            self.dirty = true;
        }
    }

    /// The input method settled on some text: first take back `before` and
    /// `after` bytes either side of the caret if it asks, then enter `text`.
    pub fn ime_commit(&mut self, window: Handle, text: &str, before: usize, after: usize) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        self.preedit = None;
        self.dirty = true;
        let Some(field) = self.live_focus(window) else { return Ok(Vec::new()) };
        if self.list_editing(field) {
            self.list_edit_compose(text, before, after);
            return Ok(Vec::new());
        }
        let Ok(edit) = self.editor_mut(field) else { return Ok(Vec::new()) };
        let removed = edit.delete_around(before, after);
        let entered = !text.is_empty() && edit.insert(text);
        self.caret_epoch = self.now;
        Ok(if removed || entered { vec![self.field_event(field, "TextField.changed")] } else { Vec::new() })
    }

    /// Give a widget the keyboard. A text field has its text selected, as if
    /// it had been reached with Tab.
    pub fn focus(&mut self, h: Handle) -> Result<()> {
        if !matches!(self.node(h)?.kind, Kind::TextField { .. } | Kind::Button { .. } | Kind::Checkbox { .. } | Kind::Popup { .. } | Kind::Radio { .. } | Kind::Slider { .. } | Kind::Spin { .. } | Kind::List { .. } | Kind::Tabs { .. } | Kind::Picture { keys: true, .. } | Kind::RichText(_)) {
            return Err(Error::WrongType);
        }
        let window = self.window_of(h).ok_or(Error::InvalidState)?;
        if !self.focusable(h) {
            return Err(Error::InvalidState);
        }
        self.set_focus(window, Some(h), true);
        Ok(())
    }

    /// The display gave or took the keyboard.
    pub fn window_set_active(&mut self, window: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(window)?.kind {
            Kind::Window { active, .. } => *active = value,
            _ => return Err(Error::WrongType),
        }
        self.caret_epoch = self.now; // come back with the caret showing
        if !value && self.menu_open.as_ref().is_some_and(|m| m.window == window) {
            self.close_menu(); // the user went elsewhere
        }
        Ok(())
    }

    // ---------------------------------------------------------------- time

    fn caret_on(&self) -> bool {
        ((self.now - self.caret_epoch) / CARET_MS).is_multiple_of(2)
    }

    /// The focus ring's phase, for controls drawn in other modules.
    fn ants_step_public(&self) -> i32 {
        self.ants_step() as i32
    }

    fn ants_step(&self) -> u64 {
        ((self.now - self.focus_epoch) / ANT_STEP_MS).min(ANT_STEPS)
    }

    /// Whether any window is showing a blinking caret, and whether any is
    /// showing a focus ring.
    fn animating(&self) -> (bool, bool) {
        let (mut caret, mut ring) = (false, false);
        for w in self.nodes.handles() {
            if !self.is_active(w) {
                continue;
            }
            match self.live_focus(w).and_then(|f| self.nodes.get(f)).map(|n| &n.kind) {
                Some(Kind::TextField { edit, .. }) => caret |= !edit.has_selection(),
                Some(Kind::RichText(state)) => caret |= !state.edit.has_selection(),
                Some(Kind::List { .. }) => caret |= self.list_edit.is_some(),
                Some(_) => ring = true,
                None => {}
            }
        }
        (caret, ring)
    }

    fn anim_state(&self) -> (bool, u64) {
        let (caret, ring) = self.animating();
        (caret && self.caret_on(), if ring { self.ants_step() } else { ANT_STEPS })
    }

    /// Advance the clock. Returns the click of a button whose keyboard flash
    /// has just ended.
    /// Let time pass, as far as the engine's clock is concerned: for a
    /// caller with no clock of its own to tick by.
    pub fn advance(&mut self, milliseconds: u64) -> Vec<Emitted> {
        self.tick(self.now + milliseconds)
    }

    pub fn tick(&mut self, now: u64) -> Vec<Emitted> {
        self.now = self.now.max(now);
        let mut out = Vec::new();
        if self.flash.is_some_and(|(_, until)| until <= self.now) {
            out.extend(self.finish_flash());
        }
        out.extend(self.menu_tick());
        out.extend(self.bar_tick());
        out.extend(self.stepper_tick());
        self.progress_tick();
        // help that has waited long enough shows
        if self.tip_showing() != self.tip_shown {
            self.tip_shown = self.tip_showing();
            self.tip_at = self.hover_at; // it stays where it came up
            self.dirty = true;
        }
        if self.pulsing() && self.pulse_step() != self.pulse {
            self.pulse = self.pulse_step();
            self.dirty = true;
        }
        let anim = self.anim_state();
        if anim != self.anim {
            self.anim = anim;
            self.dirty = true;
        }
        out
    }

    /// The time at which `tick` next has something to do, if any.
    /// Which piece of a widget a point is over, as a number a style is not
    /// told but which changes when the piece does: a tab, a table's
    /// heading, a part of a scroll bar.
    fn piece_at(&mut self, h: Handle, x: i32, y: i32) -> i32 {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Tabs { .. }) => self.tab_at(h, x, y).map_or(0, |i| 1 + i as i32),
            Some(Kind::Spin { .. }) => self.stepper_arrow_at(h, x, y),
            _ => {
                if let Some((column, _)) = self.heading_over(h, x, y) {
                    return 1000 + column as i32;
                }
                for horizontal in [false, true] {
                    if let Some(part) = self.bar_part_at(h, horizontal, x, y) {
                        return 2000 + i32::from(horizontal) * 10 + part;
                    }
                }
                0
            }
        }
    }

    /// How many times a throbbing button has been drawn again by now.
    fn pulse_step(&self) -> u64 {
        self.now / PULSE_MS
    }

    /// Whether the style has a default button throb, and there is one to.
    fn pulsing(&self) -> bool {
        self.look.wants & crate::style::wants::PULSE != 0
            && self.nodes.handles().into_iter().any(|h| matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Button { is_default: true, .. })))
    }

    pub fn next_deadline(&self) -> Option<u64> {
        let (caret, ring) = self.animating();
        let mut deadlines = Vec::new();
        deadlines.extend(self.flash.map(|(_, until)| until));
        deadlines.extend(self.menu_deadline());
        deadlines.extend(self.bar_deadline());
        deadlines.extend(self.progress_deadline());
        deadlines.extend(self.tip_deadline());
        deadlines.extend(self.spin_held.map(|s| s.2));
        if self.pulsing() {
            deadlines.push((self.pulse_step() + 1) * PULSE_MS);
        }
        if caret {
            deadlines.push(self.caret_epoch + ((self.now - self.caret_epoch) / CARET_MS + 1) * CARET_MS);
        }
        if ring && self.ants_step() < ANT_STEPS {
            deadlines.push(self.focus_epoch + (self.ants_step() + 1) * ANT_STEP_MS);
        }
        deadlines.into_iter().min()
    }

    /// End every animation that is holding an event back, and return those
    /// events: the click of a flashing button, the choice of a blinking
    /// menu item.
    pub fn finish_animations(&mut self) -> Vec<Emitted> {
        let mut out = self.finish_flash();
        out.extend(self.finish_menu_blink());
        out
    }

    /// End a keyboard flash now and return the click it was holding back.
    fn finish_flash(&mut self) -> Vec<Emitted> {
        let Some((h, _)) = self.flash.take() else { return Vec::new() };
        self.dirty = true;
        if self.focusable(h) {
            vec![Emitted { target: h, event: "Button.clicked", args: vec![] }]
        } else {
            Vec::new() // destroyed or disabled while it flashed
        }
    }

    // ------------------------------------------------------------ keyboard

    /// Press a widget from the keyboard. A button flashes first and reports
    /// its click when the flash ends, so the user sees which one answered.
    fn activate(&mut self, h: Handle, out: &mut Vec<Emitted>) {
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::Button { .. }) => {
                out.extend(self.finish_flash());
                self.flash = Some((h, self.now + FLASH_MS));
            }
            Some(Kind::Checkbox { checked, .. }) => {
                *checked = !*checked;
                out.push(Emitted { target: h, event: "Checkbox.toggled", args: vec![Value::Bool(*checked)] });
            }
            Some(Kind::Popup { .. }) => self.popup_open(h, false),
            Some(Kind::Radio { .. }) => out.extend(self.radio_pick(h)),
            _ => {}
        }
        self.dirty = true;
    }

    /// The window's default (or cancel) button, if it has an enabled one.
    fn find_button(&self, window: Handle, default: bool) -> Option<Handle> {
        self.focus_order(window).into_iter().find(|h| match self.nodes.get(*h).map(|n| &n.kind) {
            Some(Kind::Button { is_default, is_cancel, .. }) => if default { *is_default } else { *is_cancel },
            _ => false,
        })
    }

    fn check_window(&self, window: Handle) -> Result<()> {
        match self.node(window)?.kind {
            Kind::Window { .. } => Ok(()),
            _ => Err(Error::WrongType),
        }
    }

    /// Feed a key press to a window.
    ///
    /// Tab and Shift+Tab move the focus. Return presses the default button
    /// and Escape the cancel button, wherever the focus is. Everything else
    /// goes to the focused text field.
    pub fn key(&mut self, window: Handle, key: Key, mods: Mods) -> Result<Vec<Emitted>> {
        let (w, h) = match &self.node(window)?.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            _ => return Err(Error::WrongType),
        };
        // moving by line needs to know how the text is wrapped
        self.arrange(window, Rect::new(0, 0, w, h));
        let focus = self.resolve_focus(window);
        let in_field = focus.is_some_and(|f| matches!(self.nodes.get(f).map(|n| &n.kind), Some(Kind::TextField { .. })));
        // The context-menu key pops up the focused widget's menu, or puts an
        // open menu away. With nothing to pop up it works the menu bar.
        let key = match key {
            Key::Context if self.menu_open.as_ref().is_some_and(|m| m.window == window) => Key::Escape,
            Key::Context => match focus.and_then(|f| self.context_for(f).map(|menu| (f, menu))) {
                Some((widget, menu)) => {
                    // just below the caret of a text field, or the corner of anything else
                    let at = self.caret_rect(widget).or(self.nodes.get(widget).map(|n| n.rect)).unwrap_or_default();
                    self.open_popup(window, menu, at.x, at.y + at.h, Some(widget));
                    return Ok(Vec::new());
                }
                None => Key::Menu,
            },
            other => other,
        };
        // A picture that hears every key (a terminal) has them before the
        // window does: Control with a character is the program's, unless
        // Shift is held too, when it may choose a menu entry; F10 is F10.
        let raw = focus.filter(|f| self.picture_hears_all(*f));
        let menu_down = self.menu_open.as_ref().is_some_and(|m| m.window == window);
        if let (Some(picture), false) = (raw, menu_down) {
            if matches!(key, Key::Char(_)) && mods.ctrl && mods.shift {
                if let Some(out) = self.menu_key(window, key, mods, false) {
                    return Ok(out);
                }
            }
            let key = if key == Key::Menu { Key::F(10) } else { key };
            return Ok(vec![self.picture_key_down(picture, key, mods)]);
        }
        // a rich text widget keeps its words' shortcuts, after the menus'
        let in_rich = focus.is_some_and(|f| self.is_rich(f));
        if let Some(out) = self.menu_key(window, key, mods, in_field || in_rich) {
            return Ok(out);
        }
        if let Some(out) = focus.filter(|_| in_rich).and_then(|f| self.rich_key(f, key, mods)) {
            return Ok(out);
        }
        let mut out = Vec::new();
        // a cell being edited in place has the keys; Tab keeps the edit and moves on
        if focus.is_some_and(|f| self.list_editing(f)) {
            if key != Key::Tab {
                return Ok(self.list_edit_key(key, mods));
            }
            out = self.list_end_edit(true);
        }
        // the arrows and their kin work a focused radio group, slider or list
        if let Some(control) = focus.filter(|_| !in_field && !matches!(key, Key::Tab | Key::Return | Key::Escape)) {
            if let Some(handled) = self.control_key(control, key, mods) {
                return Ok(handled);
            }
        }
        match key {
            Key::Tab if !mods.ctrl && !mods.alt => {
                let order = self.focus_order(window);
                if !order.is_empty() {
                    let n = order.len();
                    let next = match (focus.and_then(|f| order.iter().position(|h| *h == f)), mods.shift) {
                        (Some(i), false) => (i + 1) % n,
                        (Some(i), true) => (i + n - 1) % n,
                        (None, false) => 0,
                        (None, true) => n - 1,
                    };
                    self.set_focus(window, Some(order[next]), true);
                }
            }
            Key::Escape => {
                if let Some(button) = self.find_button(window, false) {
                    self.activate(button, &mut out);
                }
            }
            // in a text area Return starts a new line and nothing else
            Key::Return if focus.is_some_and(|f| self.is_multiline(f)) => {
                let field = focus.expect("checked above");
                if self.editor_mut(field)?.insert("\n") {
                    out.push(self.field_event(field, "TextField.changed"));
                }
                self.caret_epoch = self.now;
            }
            Key::Return => {
                // Return in a list box opens the selected row, and then the dialog answers
                if let Some(events) = focus.and_then(|f| self.list_return(f)) {
                    out.extend(events);
                    if focus.is_some_and(|f| self.list_editing(f)) {
                        return Ok(out); // Return began an edit; it is not for the dialog
                    }
                }
                if let (true, Some(field)) = (in_field, focus) {
                    out.push(self.field_event(field, "TextField.submitted"));
                }
                if let Some(button) = self.find_button(window, true) {
                    self.activate(button, &mut out);
                }
            }
            _ if in_field => {
                let field = focus.expect("in_field implies a focus");
                let (word, extend) = (mods.ctrl, mods.shift);
                if mods.ctrl {
                    match key {
                        Key::Char('c') => self.text_field_copy(field)?,
                        Key::Char('x') => out.extend(self.text_field_cut(field)?),
                        Key::Char('v') => self.text_field_request_paste(field)?,
                        _ => {}
                    }
                    if matches!(key, Key::Char('c' | 'x' | 'v')) {
                        self.caret_epoch = self.now;
                        return Ok(out);
                    }
                }
                if self.is_multiline(field) && !mods.ctrl && matches!(key, Key::Up | Key::Down | Key::Home | Key::End) {
                    // in a text area these move by the lines as they are drawn
                    match key {
                        Key::Up | Key::Down => self.field_vertical(field, key == Key::Down, extend)?,
                        _ => self.field_line_edge(field, key == Key::End, extend)?,
                    }
                    self.caret_epoch = self.now;
                    return Ok(out);
                }
                let edit = self.editor_mut(field)?;
                let changed = match key {
                    Key::Left => { edit.left(word, extend); false }
                    Key::Right => { edit.right(word, extend); false }
                    // a one-line field treats up and down as the ends of the line;
                    // in a text area, with Control, these reach the ends of the text
                    Key::Home | Key::Up => { edit.home(extend); false }
                    Key::End | Key::Down => { edit.end(extend); false }
                    Key::Backspace => edit.backspace(word),
                    Key::Delete => edit.delete(word),
                    Key::Char('a') if mods.ctrl => { edit.select_all(); false }
                    _ => return Ok(out),
                };
                self.caret_epoch = self.now;
                if changed {
                    out.push(self.field_event(field, "TextField.changed"));
                }
            }
            _ => {}
        }
        Ok(out)
    }

    /// Feed typed text to a window. It goes into the focused text field; a
    /// space presses a focused button or checkbox.
    pub fn text(&mut self, window: Handle, text: &str) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        if let Some(out) = self.menu_text(window, text) {
            return Ok(out);
        }
        let mut out = Vec::new();
        let Some(focus) = self.resolve_focus(window) else { return Ok(out) };
        if self.list_editing(focus) {
            self.list_edit_text(text);
            return Ok(out);
        }
        // a picture with the keyboard is told what is typed
        if self.picture_takes_keys(focus) {
            return Ok(vec![Emitted { target: focus, event: "Picture.typed", args: vec![Value::Str(text.to_string())] }]);
        }
        // typing in a list box jumps to a row
        if let Some(found) = self.list_type(focus, text) {
            return Ok(found);
        }
        if let Some(out) = self.rich_input(focus, text) {
            return Ok(out);
        }
        match self.editor_mut(focus) {
            Ok(edit) => {
                if edit.insert(text) {
                    out.push(self.field_event(focus, "TextField.changed"));
                    self.caret_epoch = self.now;
                }
            }
            Err(_) if text == " " => self.activate(focus, &mut out),
            Err(_) => {}
        }
        Ok(out)
    }

    /// The picture of a pointer shape, as the style being worn has it.
    pub fn cursor_image(&self, cursor: Cursor) -> crate::cursor::CursorImage {
        cursor.image_in(self.look.pal.pointer, self.look.pal.pointer_edge)
    }

    /// The pointer shape for a position in a window: an hourglass while
    /// the window is busy, an I-beam over text that can be edited and while
    /// a selection is being dragged, otherwise the arrow.
    pub fn cursor_at(&mut self, window: Handle, x: i32, y: i32) -> Cursor {
        // over an edge of its own frame that can be dragged, the arrows that say so
        if let Some(cursor) = self.frame_cursor(window, x, y) {
            return cursor;
        }
        let is_field = |ui: &Ui, h: Handle| matches!(ui.nodes.get(h).map(|n| &n.kind), Some(Kind::TextField { .. }));
        if self.menu_open.as_ref().is_some_and(|m| m.window == window) {
            return Cursor::Arrow;
        }
        if matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { busy: true, .. })) {
            return Cursor::Busy;
        }
        if let Some((held, _)) = self.pressed {
            return if is_field(self, held) { Cursor::Text } else { Cursor::Arrow };
        }
        // over a divider, or dragging one, arrows the way it goes
        let dividing = self.divider_drag.map(|(d, _)| d).or_else(|| self.hit(window, x, y).filter(|h| self.divider_way(*h).is_some()));
        if let Some(way) = dividing.and_then(|d| self.divider_way(d)) {
            return if way == Orientation::Vertical { Cursor::ResizeRow } else { Cursor::ResizeColumn };
        }
        match self.hit(window, x, y) {
            Some(h) if self.over_column_edge(h, x, y) => Cursor::ResizeColumn,
            Some(h) if is_field(self, h) && self.text_rect(h).is_some_and(|r| r.contains(x, y)) => Cursor::Text,
            Some(h) if self.is_rich(h) && self.rich_inner(h).is_some_and(|r| r.contains(x, y)) => Cursor::Text,
            _ => Cursor::Arrow,
        }
    }

    /// The display asked to close a window.
    /// The pointer's back or forward button was pressed over the window.
    pub fn navigate(&self, window: Handle, forward: bool) -> Result<Vec<Emitted>> {
        match self.node(window)?.kind {
            Kind::Window { .. } => Ok(vec![Emitted { target: window, event: "Window.navigate", args: vec![Value::Bool(forward)] }]),
            _ => Err(Error::WrongType),
        }
    }

    pub fn close_requested(&self, window: Handle) -> Result<Vec<Emitted>> {
        match self.node(window)?.kind {
            Kind::Window { .. } => Ok(vec![Emitted { target: window, event: "Window.close_requested", args: vec![] }]),
            _ => Err(Error::WrongType),
        }
    }
}

#[cfg(test)]
mod typeface_tests {
    use super::*;

    #[test]
    fn a_widget_can_be_set_in_another_typeface() {
        let mut ui = Ui::new();
        let win = ui.window_new("Faces", 400, 200).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        let (plain, mono, field) = (ui.label_new("iiii"), ui.label_new("iiii"), ui.text_field_new("iiii"));
        ui.set_typeface(mono, Face::Pica).unwrap();
        ui.set_typeface(field, Face::Pica).unwrap();
        for w in [plain, mono, field] {
            ui.stack_add(col, w).unwrap();
        }
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        // the label is as wide as its text in its own face: four narrow i in Cicero, four cells in Pica
        assert_eq!(ui.natural_size(plain).0, Font::new().width("iiii"));
        assert_eq!(ui.natural_size(mono).0, 4 * 7);
        // the interface's own face is back in use afterwards
        assert_eq!(ui.font.width("iiii"), Font::new().width("iiii"));
        // a click in the field lands by Pica's widths: in the middle of the third cell is after the second i
        let inner = ui.text_rect(field).unwrap();
        assert_eq!(ui.index_at(field, inner.x + 1 + 2 * 7 + 2, inner.y + 2), 2);
        // the field's text is given to assistive technology in Pica's widths too
        let (runs, _) = ui.field_runs(field);
        assert!(runs[0].chars.iter().all(|c| c.2 == 7));
        // a widget with no text of its own refuses one
        let slider = ui.slider_new(Orientation::Horizontal);
        assert_eq!(ui.set_typeface(slider, Face::Pica), Err(Error::WrongType));
        // and back to the interface's face
        ui.set_typeface(mono, Face::Cicero).unwrap();
        assert_eq!(ui.natural_size(mono).0, Font::new().width("iiii"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialog(ui: &mut Ui) -> (Handle, Handle, Handle) {
        let win = ui.window_new("Test", 200, 100).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let ok = ui.button_new("OK");
        let check = ui.checkbox_new("Option");
        ui.stack_add(col, ok).unwrap();
        ui.stack_add(col, check).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        (win, ok, check)
    }

    #[test]
    fn click_emits_only_when_released_over_the_button() {
        let mut ui = Ui::new();
        let (win, ok, _) = dialog(&mut ui);
        // button occupies x 10..68, y 10..30
        ui.pointer(win, 20, 20, PointerKind::Press).unwrap();
        let ev = ui.pointer(win, 20, 20, PointerKind::Release).unwrap();
        assert_eq!(ev, vec![Emitted { target: ok, event: "Button.clicked", args: vec![] }]);

        ui.pointer(win, 20, 20, PointerKind::Press).unwrap();
        ui.pointer(win, 150, 90, PointerKind::Move).unwrap();
        assert!(ui.pointer(win, 150, 90, PointerKind::Release).unwrap().is_empty());
    }

    #[test]
    fn checkbox_toggles_and_reports_state() {
        let mut ui = Ui::new();
        let (win, _, check) = dialog(&mut ui);
        ui.pointer(win, 14, 44, PointerKind::Press).unwrap();
        let ev = ui.pointer(win, 14, 44, PointerKind::Release).unwrap();
        assert_eq!(ev[0].args, vec![Value::Bool(true)]);
        assert!(ui.checkbox_checked(check).unwrap());
    }

    #[test]
    fn pointer_works_before_the_first_render() {
        let mut ui = Ui::new();
        let win = ui.window_new("Test", 200, 100).unwrap();
        let ok = ui.button_new("OK");
        ui.window_set_content(win, ok).unwrap();
        ui.pointer(win, 5, 5, PointerKind::Press).unwrap();
        assert_eq!(ui.pointer(win, 5, 5, PointerKind::Release).unwrap().len(), 1);
    }

    #[test]
    fn disabled_widgets_ignore_the_pointer() {
        let mut ui = Ui::new();
        let (win, ok, _) = dialog(&mut ui);
        ui.set_enabled(ok, false).unwrap();
        ui.pointer(win, 20, 20, PointerKind::Press).unwrap();
        assert!(ui.pointer(win, 20, 20, PointerKind::Release).unwrap().is_empty());
    }

    #[test]
    fn destroy_invalidates_the_whole_subtree() {
        let mut ui = Ui::new();
        let (win, ok, check) = dialog(&mut ui);
        let gone = ui.destroy(win).unwrap();
        assert_eq!(gone.len(), 4);
        assert_eq!(ui.class_of(ok), Err(Error::InvalidHandle));
        assert_eq!(ui.set_enabled(check, true), Err(Error::InvalidHandle));
    }

    #[test]
    fn tree_rules() {
        let mut ui = Ui::new();
        let (win, ok, _) = dialog(&mut ui);
        let other = ui.stack_new(Orientation::Horizontal);
        assert_eq!(ui.stack_add(other, ok), Err(Error::InvalidState)); // already has a parent
        assert_eq!(ui.stack_add(other, win), Err(Error::WrongType)); // windows are not children
        assert_eq!(ui.stack_add(other, other), Err(Error::InvalidArgument)); // no cycles
        assert_eq!(ui.label_set_text(ok, "x"), Err(Error::WrongType));
    }

    #[test]
    fn stack_insert_places_and_moves() {
        let mut ui = Ui::new();
        let stack = ui.stack_new(Orientation::Vertical);
        let (a, b, c) = (ui.label_new("a"), ui.label_new("b"), ui.label_new("c"));
        ui.stack_add(stack, a).unwrap();
        ui.stack_insert(stack, b, 0).unwrap();
        ui.stack_insert(stack, c, 99).unwrap(); // past the end is the end
        assert_eq!(ui.children(stack), vec![b, a, c]);
        ui.stack_insert(stack, c, 0).unwrap(); // one already here is moved
        assert_eq!(ui.children(stack), vec![c, b, a]);
        ui.stack_insert(stack, c, 2).unwrap();
        assert_eq!(ui.children(stack), vec![b, a, c]);
        let other = ui.stack_new(Orientation::Vertical);
        assert_eq!(ui.stack_insert(other, a, 0), Err(Error::InvalidState)); // it has a parent
        assert_eq!(ui.stack_insert(stack, a, -1), Err(Error::InvalidArgument));
        assert_eq!(ui.stack_insert(stack, stack, 0), Err(Error::InvalidArgument)); // no cycles
    }

    #[test]
    fn window_fits_its_content() {
        let mut ui = Ui::new();
        let win = ui.window_new("t", 500, 400).unwrap();
        let column = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(column, 16).unwrap();
        let label = ui.label_new("Some words to measure");
        ui.stack_add(column, label).unwrap();
        ui.window_set_content(win, column).unwrap();
        let (w, h) = ui.measure(column);
        ui.window_fit(win).unwrap();
        assert_eq!(ui.window_min_size(win).unwrap(), (w.max(64), h.max(32)));
        assert_eq!(ui.window_focused(win), Ok(None)); // nothing here takes keys
        let field = ui.text_field_new("");
        ui.stack_add(column, field).unwrap();
        assert_eq!(ui.window_focused(win), Ok(Some(field)));
        ui.window_show(win).unwrap();
        // once showing, fitting it again tells the display of its new size
        ui.take_resized(win);
        ui.window_fit(win).unwrap();
        assert!(ui.take_resized(win));
    }

    #[test]
    fn output_is_strictly_one_bit() {
        let mut ui = Ui::new();
        let (win, ok, _) = dialog(&mut ui);
        ui.set_enabled(ok, false).unwrap();
        let canvas = ui.render(win).unwrap();
        assert!(canvas.pixels.iter().all(|p| *p == BLACK || *p == WHITE));
        assert!(canvas.pixels.contains(&BLACK));
    }

    /// A window holding a text field, a checkbox, and Cancel and OK buttons.
    fn form(ui: &mut Ui) -> (Handle, Handle, Handle, Handle, Handle) {
        let win = ui.window_new("Form", 240, 140).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let field = ui.text_field_new("");
        let check = ui.checkbox_new("Option");
        let cancel = ui.button_new("Cancel");
        let ok = ui.button_new("OK");
        ui.button_set_cancel(cancel, true).unwrap();
        ui.button_set_default(ok, true).unwrap();
        for w in [field, check, cancel, ok] {
            ui.stack_add(col, w).unwrap();
        }
        ui.window_set_content(win, col).unwrap();
        (win, field, check, cancel, ok)
    }

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };
    const SHIFT: Mods = Mods { shift: true, ctrl: false, alt: false };
    const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false };

    fn names(events: &[Emitted]) -> Vec<&'static str> {
        events.iter().map(|e| e.event).collect()
    }

    #[test]
    fn typing_goes_to_the_first_text_field() {
        let mut ui = Ui::new();
        let (win, field, ..) = form(&mut ui);
        let ev = ui.text(win, "Clarus").unwrap();
        assert_eq!(ev, vec![Emitted { target: field, event: "TextField.changed", args: vec![Value::Str("Clarus".into())] }]);
        assert_eq!(ui.text_field_text(field).unwrap(), "Clarus");
        // setting the text from the program is not a user edit
        ui.text_field_set_text(field, "Moof").unwrap();
        assert!(ui.key(win, Key::Left, NONE).unwrap().is_empty());
        assert_eq!(names(&ui.key(win, Key::Backspace, NONE).unwrap()), ["TextField.changed"]);
        assert_eq!(ui.text_field_text(field).unwrap(), "Mof");
        ui.key(win, Key::Char('a'), CTRL).unwrap();
        ui.text(win, "!").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "!");
    }

    #[test]
    fn tab_walks_the_enabled_widgets_and_wraps() {
        let mut ui = Ui::new();
        let (win, field, check, cancel, ok) = form(&mut ui);
        ui.set_enabled(cancel, false).unwrap();
        ui.text(win, "x").unwrap(); // focus starts in the field
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(check));
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(ok)); // Cancel is disabled
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(field));
        // arriving by Tab selects the text, so typing replaces it
        ui.text(win, "y").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "y");
        ui.key(win, Key::Tab, SHIFT).unwrap();
        assert_eq!(ui.live_focus(win), Some(ok));
        // a focused widget that becomes disabled stops receiving keys
        ui.set_enabled(ok, false).unwrap();
        assert_eq!(ui.live_focus(win), None);
        assert!(ui.text(win, " ").unwrap().is_empty());
    }

    #[test]
    fn space_presses_the_focused_widget() {
        let mut ui = Ui::new();
        let (win, _, check, cancel, _) = form(&mut ui);
        ui.focus(check).unwrap();
        let ev = ui.text(win, " ").unwrap();
        assert_eq!(ev, vec![Emitted { target: check, event: "Checkbox.toggled", args: vec![Value::Bool(true)] }]);
        ui.focus(cancel).unwrap();
        assert!(ui.text(win, " ").unwrap().is_empty()); // the button flashes first
        assert_eq!(ui.tick(FLASH_MS), vec![Emitted { target: cancel, event: "Button.clicked", args: vec![] }]);
        assert!(ui.tick(FLASH_MS * 2).is_empty());
    }

    #[test]
    fn return_and_escape_reach_the_dialog_buttons() {
        let mut ui = Ui::new();
        let (win, field, _, cancel, ok) = form(&mut ui);
        assert_eq!(names(&ui.key(win, Key::Return, NONE).unwrap()), ["TextField.submitted"]);
        assert_eq!(ui.next_deadline(), Some(FLASH_MS));
        assert_eq!(ui.finish_animations(), vec![Emitted { target: ok, event: "Button.clicked", args: vec![] }]);
        ui.key(win, Key::Escape, NONE).unwrap();
        assert_eq!(ui.finish_animations()[0].target, cancel);
        // a disabled default button does not answer, the field still submits
        ui.set_enabled(ok, false).unwrap();
        assert_eq!(names(&ui.key(win, Key::Return, NONE).unwrap()), ["TextField.submitted"]);
        assert!(ui.finish_animations().is_empty());
        // a button destroyed mid-flash never clicks
        ui.key(win, Key::Escape, NONE).unwrap();
        ui.destroy(cancel).unwrap();
        assert!(ui.tick(10_000).is_empty());
        let _ = field;
    }

    #[test]
    fn the_pointer_places_the_caret_and_selects() {
        let mut ui = Ui::new();
        let (win, field, check, ..) = form(&mut ui);
        ui.text_field_set_text(field, "save the file").unwrap();
        ui.focus(check).unwrap();
        ui.render(win).unwrap();
        // the field's text starts at x = 16; click far to the left of it, then drag right
        ui.pointer(win, 12, 20, PointerKind::Press).unwrap();
        assert_eq!(ui.live_focus(win), Some(field));
        ui.pointer(win, 400, 90, PointerKind::Move).unwrap();
        ui.pointer(win, 400, 90, PointerKind::Release).unwrap();
        ui.text(win, "x").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "x"); // everything was selected
        // a double click selects a word
        ui.text_field_set_text(field, "save the file").unwrap();
        ui.tick(5_000);
        let x = 16 + ui.font.width("save t");
        for _ in 0..2 {
            ui.pointer(win, x, 20, PointerKind::Press).unwrap();
            ui.pointer(win, x, 20, PointerKind::Release).unwrap();
        }
        ui.text(win, "a").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "save a file");
        // clicking a button does not take the keyboard away from the field
        ui.pointer(win, 20, 100, PointerKind::Press).unwrap();
        ui.pointer(win, 20, 100, PointerKind::Release).unwrap();
        assert_eq!(ui.live_focus(win), Some(field));
    }

    #[test]
    fn the_pointer_is_an_i_beam_over_editable_text() {
        let mut ui = Ui::new();
        let (win, field, ..) = form(&mut ui);
        ui.render(win).unwrap();
        assert_eq!(ui.cursor_at(win, 60, 20), Cursor::Text);
        assert_eq!(ui.cursor_at(win, 60, 50), Cursor::Arrow); // the checkbox
        assert_eq!(ui.cursor_at(win, 230, 130), Cursor::Arrow); // empty space
        // it stays an I-beam while a selection is dragged out of the field
        ui.pointer(win, 60, 20, PointerKind::Press).unwrap();
        assert_eq!(ui.cursor_at(win, 230, 130), Cursor::Text);
        ui.pointer(win, 230, 130, PointerKind::Release).unwrap();
        ui.set_enabled(field, false).unwrap();
        assert_eq!(ui.cursor_at(win, 60, 20), Cursor::Arrow);
        // a busy window shows the hourglass wherever the pointer is on it
        ui.window_set_busy(win, true).unwrap();
        assert_eq!(ui.cursor_at(win, 60, 20), Cursor::Busy);
        assert_eq!(ui.cursor_at(win, 230, 130), Cursor::Busy);
        ui.window_set_busy(win, false).unwrap();
        assert_eq!(ui.cursor_at(win, 230, 130), Cursor::Arrow);
    }

    #[test]
    fn spare_room_goes_to_the_widgets_that_can_use_it() {
        let mut ui = Ui::new();
        let win = ui.window_new("Notes", 300, 200).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        ui.stack_set_spacing(col, 8).unwrap();
        let (label, area, field) = (ui.label_new("Note:"), ui.text_area_new(""), ui.text_field_new(""));
        let row = ui.stack_new(Orientation::Horizontal);
        let (save, gap, close) = (ui.button_new("Save"), ui.label_new(""), ui.button_new("Close"));
        for w in [save, gap, close] {
            ui.stack_add(row, w).unwrap();
        }
        for w in [label, area, field, row] {
            ui.stack_add(col, w).unwrap();
        }
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        let rect = |ui: &Ui, h: Handle| ui.nodes.get(h).unwrap().rect;
        let natural = ui.measure(col);
        assert!(natural.1 < 200);
        // the text area takes all the spare height, and it and the field the full width
        let (a, f, l, r) = (rect(&ui, area), rect(&ui, field), rect(&ui, label), rect(&ui, row));
        assert_eq!((a.x, a.w, f.x, f.w), (10, 280, 10, 280));
        assert_eq!(a.h, ui.measure(area).1 + 200 - natural.1);
        assert_eq!((l.w, l.h), ui.measure(label));
        // what was below the text area has moved down with it, to the bottom
        assert_eq!(r.y + r.h, 190);
        // nothing in the row of buttons grows, so it stays as it was...
        assert_eq!(rect(&ui, close).x + rect(&ui, close).w, 10 + ui.measure(row).0);
        // ...until something in it is told to: then the last button is pushed to the edge
        ui.set_stretch(gap, true).unwrap();
        ui.render(win).unwrap();
        assert_eq!(rect(&ui, close).x + rect(&ui, close).w, 290);
        assert_eq!(rect(&ui, save).w, ui.measure(save).0);
        // and a widget told not to grow keeps its natural size
        ui.set_stretch(area, false).unwrap();
        ui.render(win).unwrap();
        assert_eq!((rect(&ui, area).w, rect(&ui, area).h), ui.measure(area));
        // a window resized larger shares out the new room the same way
        ui.set_stretch(area, true).unwrap();
        ui.window_resize(win, 400, 300).unwrap();
        ui.render(win).unwrap();
        assert_eq!((rect(&ui, area).w, rect(&ui, area).h), (380, ui.measure(area).1 + 300 - natural.1));
        // two that grow share odd pixels without losing any
        let other = ui.text_area_new("");
        ui.stack_add(col, other).unwrap();
        ui.window_resize(win, 400, 401).unwrap();
        ui.render(win).unwrap();
        let (a, o) = (rect(&ui, area), rect(&ui, other));
        assert_eq!(o.y + o.h, 391);
        assert!((a.h - o.h).abs() <= 1);
    }

    #[test]
    fn a_widget_can_be_given_a_least_size() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Tiles", 300, 200).unwrap();
        let row = ui.stack_new(Orientation::Horizontal);
        let (a, b) = (ui.button_new("7"), ui.button_new("A much longer label"));
        ui.stack_add(row, a).unwrap();
        ui.stack_add(row, b).unwrap();
        ui.window_set_content(win, row).unwrap();
        let natural = (ui.measure(a), ui.measure(b));
        ui.set_min_size(a, 10, 44).unwrap();
        ui.set_min_size(b, 10, 10).unwrap();
        // taller where that was asked; never smaller than it would have been
        assert_eq!((ui.measure(a), ui.measure(b)), ((natural.0 .0, 44), natural.1));
        ui.render(win).unwrap();
        assert_eq!(ui.nodes.get(a).unwrap().rect.h, 44);
        assert_eq!(ui.set_min_size(a, -1, 0), Err(Error::InvalidArgument));
    }

    #[test]
    fn long_text_scrolls_to_keep_the_caret_in_view() {
        let mut ui = Ui::natural();
        let (win, field, ..) = form(&mut ui);
        ui.text(win, &"wide ".repeat(40)).unwrap();
        let canvas = ui.render(win).unwrap();
        // nothing spills past the field's frame (x 10..150) into the window
        assert!((0..canvas.height).all(|y| (151..canvas.width).all(|x| canvas.get(x, y) == WHITE || y > 34)));
        // the left edge of the field now shows text from well inside the string
        ui.tick(5_000);
        ui.pointer(win, 16, 20, PointerKind::Press).unwrap();
        ui.pointer(win, 16, 20, PointerKind::Release).unwrap();
        ui.key(win, Key::Home, SHIFT).unwrap();
        ui.key(win, Key::Backspace, NONE).unwrap(); // delete everything scrolled off to the left
        let left = ui.text_field_text(field).unwrap().len();
        assert!(left > 0 && left < 100, "{left}");
    }

    #[test]
    fn cut_copy_and_paste_pass_through_the_caller() {
        let mut ui = Ui::new();
        let (win, field, ..) = form(&mut ui);
        ui.text(win, "one two").unwrap();
        ui.key(win, Key::Char('c'), CTRL).unwrap();
        assert_eq!(ui.take_copy(), None); // nothing selected: the clipboard is left alone
        ui.key(win, Key::Left, Mods { shift: true, ctrl: true, alt: false }).unwrap();
        assert_eq!(names(&ui.key(win, Key::Char('x'), CTRL).unwrap()), ["TextField.changed"]);
        assert_eq!(ui.take_copy().as_deref(), Some("two"));
        assert_eq!(ui.text_field_text(field).unwrap(), "one ");
        // a paste is a request the caller answers, possibly later
        ui.key(win, Key::Char('v'), CTRL).unwrap();
        assert_eq!(ui.take_paste_request(), Some(field));
        assert_eq!(ui.take_paste_request(), None);
        let pasted = ui.paste_into(field, "2\nand 3").unwrap();
        assert_eq!(pasted[0].args, vec![Value::Str("one 2 and 3".into())]); // the event carries the new text
        assert_eq!(ui.text_field_text(field).unwrap(), "one 2 and 3");
        // the answer may arrive after the field has gone
        ui.destroy(field).unwrap();
        assert_eq!(ui.paste_into(field, "x"), Err(Error::InvalidHandle));
    }

    #[test]
    fn an_input_method_composes_in_the_focused_field() {
        let mut ui = Ui::new();
        let (win, field, check, ..) = form(&mut ui);
        ui.text(win, "ab").unwrap();
        ui.key(win, Key::Left, NONE).unwrap();
        let plain = ui.render(win).unwrap().pixels;
        let caret = ui.ime_target(win).unwrap();
        // a draft is drawn but is not yet the field's text
        ui.ime_preedit(win, "ni");
        assert_eq!(ui.text_field_text(field).unwrap(), "ab");
        assert_ne!(ui.render(win).unwrap().pixels, plain);
        assert!(ui.ime_target(win).unwrap().x > caret.x); // the caret follows the draft
        // committing replaces the draft with the chosen text
        let ev = ui.ime_commit(win, "你", 0, 0).unwrap();
        assert_eq!(ev[0].args, vec![Value::Str("a你b".into())]);
        // the method may take back what it entered and try again
        ui.ime_commit(win, "好", "你".len(), 0).unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "a好b");
        // a draft is dropped when the focus moves on, and none starts on a checkbox
        ui.ime_preedit(win, "x");
        ui.focus(check).unwrap();
        assert_eq!(ui.ime_target(win), None);
        ui.ime_preedit(win, "y");
        assert!(ui.ime_commit(win, "z", 0, 0).unwrap().is_empty());
        ui.focus(field).unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "a好b");
    }

    #[test]
    fn the_clock_only_runs_for_what_is_on_show() {
        let mut ui = Ui::new();
        let (win, field, check, ..) = form(&mut ui);
        ui.render(win).unwrap();
        assert_eq!(ui.next_deadline(), Some(CARET_MS)); // caret in the auto-focused field
        ui.tick(0);
        ui.take_dirty();
        ui.tick(CARET_MS - 1);
        assert!(!ui.take_dirty());
        ui.tick(CARET_MS);
        assert!(ui.take_dirty()); // the caret went out
        ui.window_set_active(win, false).unwrap();
        assert_eq!(ui.next_deadline(), None);
        ui.window_set_active(win, true).unwrap();
        // a focus ring marches for a moment, then rests
        ui.focus(check).unwrap();
        assert_eq!(ui.next_deadline(), Some(CARET_MS + ANT_STEP_MS));
        ui.tick(CARET_MS + ANT_STEP_MS * ANT_STEPS);
        assert_eq!(ui.next_deadline(), None);
        // a selection shows no caret, so nothing needs to blink
        ui.focus(field).unwrap();
        ui.text(win, "abc").unwrap();
        ui.key(win, Key::Home, SHIFT).unwrap();
        assert_eq!(ui.next_deadline(), None);
        assert_eq!(ui.focus(win), Err(Error::WrongType));
    }
}

#[cfg(test)]
mod style_tests {
    use super::*;
    use crate::style::{Metrics, EightyFour, Palette};

    /// A style that changes sizes and colours and draws nothing itself.
    struct Plain;

    impl Style for Plain {
        fn name(&self) -> &str {
            "plain"
        }
        fn metrics(&self) -> Metrics {
            Metrics { button_height: 30, ..EightyFour::METRICS }
        }
        fn palette(&self) -> Palette {
            Palette { window: 0xffc0c0c0, text: 0xff000080, ..EightyFour::PALETTE }
        }
        fn draw(&self, part: &Part, painter: &mut Painter) -> bool {
            if part.kind == PartKind::Background {
                painter.canvas.fill(part.rect, 0xffc0c0c0);
                return true;
            }
            false
        }
    }

    fn dialog(ui: &mut Ui) -> (Handle, Handle) {
        let window = ui.window_new("Style", 200, 100).unwrap();
        let column = ui.stack_new(Orientation::Vertical);
        let (label, button) = (ui.label_new("Words"), ui.button_new("Press"));
        ui.stack_add(column, label).unwrap();
        ui.stack_add(column, button).unwrap();
        ui.window_set_content(window, column).unwrap();
        (window, button)
    }

    #[test]
    fn a_style_decides_sizes_and_colours() {
        let mut ui = Ui::natural();
        let (window, button) = dialog(&mut ui);
        ui.render(window).unwrap();
        assert_eq!(ui.nodes.get(button).unwrap().rect.h, EightyFour::METRICS.button_height);
        assert_eq!(ui.style_name(), "eightyfour");

        ui.set_style(std::sync::Arc::new(Plain));
        let canvas = ui.render(window).unwrap();
        assert_eq!(ui.style_name(), "plain");
        assert_eq!(ui.nodes.get(button).unwrap().rect.h, 30);
        // the window is the style's grey, and the label's words its blue
        assert_eq!(canvas.get(199, 99), 0xffc0c0c0);
        assert!(canvas.pixels.contains(&0xff000080));
    }

    #[test]
    fn what_a_style_leaves_is_drawn_in_black_and_white() {
        let mut ui = Ui::natural();
        let (window, button) = dialog(&mut ui);
        ui.set_style(std::sync::Arc::new(Plain));
        let canvas = ui.render(window).unwrap();
        // the button, which the style did not draw, has its black outline
        let r = ui.nodes.get(button).unwrap().rect;
        assert_eq!(canvas.get(r.x + r.w / 2, r.y), BLACK);
        assert_eq!(canvas.get(r.x + r.w / 2, r.y + 1), WHITE);
    }

    /// A style of taller rows and smooth letters that wants to hear of the pointer.
    struct Tall;

    impl Style for Tall {
        fn name(&self) -> &str {
            "tall"
        }
        fn metrics(&self) -> Metrics {
            Metrics { row_height: 24, menu_bar_height: 30, scroll_bar: 20, ..EightyFour::METRICS }
        }
        fn palette(&self) -> Palette {
            EightyFour::PALETTE
        }
        fn typeface(&self, face: Face) -> Option<crate::font::Type> {
            (face == Face::Cicero).then(|| crate::font::Type { family: "sans".into(), size: 14, bold: false, crisp: false })
        }
        fn wants(&self) -> u32 {
            crate::style::wants::HOVER
        }
        fn draw(&self, _: &Part, _: &mut Painter) -> bool {
            false
        }
    }

    #[test]
    fn a_style_decides_the_height_of_rows_and_bars() {
        let mut ui = Ui::natural();
        let list = ui.list_box_new();
        ui.list_box_set_rows(list, 5).unwrap();
        let plain = ui.measure(list).1;
        ui.set_style(std::sync::Arc::new(Tall));
        assert_eq!(ui.measure(list).1 - plain, 5 * (24 - EightyFour::METRICS.row_height));
        let bar = ui.scroll_bar_new(Orientation::Vertical);
        assert_eq!(ui.measure(bar).0, 20);
    }

    #[test]
    fn a_style_may_write_in_a_typeface_of_the_machines() {
        let mut ui = Ui::natural();
        let (window, _) = dialog(&mut ui);
        ui.set_style(std::sync::Arc::new(Tall));
        if !ui.font.is_smooth() {
            return; // a machine with no fonts of its own: the bundled family serves
        }
        let canvas = ui.render(window).unwrap();
        // smooth letters have edges that are neither black nor white
        assert!(canvas.pixels.iter().any(|p| *p != BLACK && *p != WHITE));
        // and the line is as tall as the typeface asked for, not the bundled one
        assert!(ui.font.line_height() >= 14);
    }

    #[test]
    fn the_pointer_coming_over_a_button_is_news_only_to_a_style_that_wants_it() {
        for (wants, style) in [(false, std::sync::Arc::new(EightyFour) as std::sync::Arc<dyn Style>), (true, std::sync::Arc::new(Tall))] {
            let mut ui = Ui::natural();
            let (window, button) = dialog(&mut ui);
            ui.set_style(style);
            ui.render(window).unwrap();
            ui.dirty = false;
            let r = ui.nodes.get(button).unwrap().rect;
            ui.pointer(window, r.x + 3, r.y + 3, PointerKind::Move).unwrap();
            assert_eq!(ui.dirty, wants);
        }
    }

    #[test]
    fn a_colour_laid_over_another_is_mixed_with_it() {
        let mut canvas = Canvas::new(2, 1);
        canvas.blend(0, 0, BLACK, 128);
        let grey = canvas.get(0, 0) & 0xff;
        assert!((126..=129).contains(&grey) && canvas.get(0, 0) >> 24 == 255);
        // on a see-through pixel it is simply that solid
        canvas.set(1, 0, crate::canvas::TRANSPARENT);
        canvas.blend(1, 0, BLACK, 64);
        assert_eq!(canvas.get(1, 0), 64 << 24);
    }

    /// A style with a frame of its own: a title bar with a button at each end.
    struct Framed;

    impl Style for Framed {
        fn name(&self) -> &str {
            "framed"
        }
        fn metrics(&self) -> Metrics {
            Metrics { frame_title: 20, frame_border: 3, frame_button_width: 12, frame_button_height: 12, frame_button_inset: 5, frame_left: 0x1, frame_right: 0x3, menu_in_window: 1, ..EightyFour::METRICS }
        }
        fn palette(&self) -> Palette {
            EightyFour::PALETTE
        }
        fn draw(&self, _: &Part, _: &mut Painter) -> bool {
            false
        }
    }

    #[test]
    fn a_window_draws_its_own_frame_where_the_style_has_one_and_the_display_can_show_it() {
        let mut ui = Ui::natural();
        let (window, _) = dialog(&mut ui);
        ui.set_style(std::sync::Arc::new(Framed));
        // until a display says it can show such a window, the frame is the desktop's
        assert_eq!(ui.frame_insets(window), (0, 0, 0, 0));
        assert_eq!(ui.render_framed(window).unwrap().width, 200);
        ui.set_own_frames(true);
        assert_eq!(ui.frame_insets(window), (3, 20, 3, 3));
        assert!(ui.menu_in_window());
        let canvas = ui.render_framed(window).unwrap();
        assert_eq!((canvas.width, canvas.height), (206, 123));
        // the content is where it was, further in by the frame
        let content = ui.render(window).unwrap();
        assert_eq!(canvas.get(3 + 50, 20 + 50), content.get(50, 50));
        // black and white has no frame of its own
        ui.set_style(std::sync::Arc::new(EightyFour));
        assert_eq!(ui.frame_insets(window), (0, 0, 0, 0));
    }

    /// A style with a frame that casts a shadow, and OK before Cancel.
    struct Shadowed;

    impl Style for Shadowed {
        fn name(&self) -> &str {
            "shadowed"
        }
        fn metrics(&self) -> Metrics {
            Metrics { frame_shadow: 12, ok_leads: 1, ..Framed.metrics() }
        }
        fn palette(&self) -> Palette {
            EightyFour::PALETTE
        }
        fn draw(&self, _: &Part, _: &mut Painter) -> bool {
            false
        }
    }

    /// A style that asks to be drawn as fine as the screen, and draws nothing itself.
    struct Fine;

    impl Style for Fine {
        fn name(&self) -> &str {
            "fine"
        }
        fn metrics(&self) -> Metrics {
            Metrics { frame_shadow: 6, ..Framed.metrics() }
        }
        fn palette(&self) -> Palette {
            EightyFour::PALETTE
        }
        fn wants(&self) -> u32 {
            crate::style::wants::FINE
        }
        fn draw(&self, _: &Part, _: &mut Painter) -> bool {
            false
        }
    }

    #[test]
    fn a_style_that_asks_is_drawn_as_fine_as_the_screen_and_what_is_made_of_pixels_keeps_them() {
        let mut ui = Ui::natural();
        let (window, _) = dialog(&mut ui);
        let list = ui.list_box_new();
        ui.list_box_add(list, "Row").unwrap();
        let column = ui.children(window)[0];
        ui.stack_add(column, list).unwrap();
        let field = ui.text_field_new("Typed");
        ui.stack_add(column, field).unwrap();
        // black and white does not ask: it is a pixel to a pixel however far the screen magnifies
        ui.set_fineness(3);
        assert_eq!((ui.fine_scale(), ui.render(window).unwrap().scale), (1, 1));
        ui.set_style(std::sync::Arc::new(Fine));
        ui.set_own_frames(true);
        ui.set_fineness(1);
        let (coarse, _) = ui.render_shadowed(window).unwrap();
        ui.set_fineness(2);
        let (fine, reach) = ui.render_shadowed(window).unwrap();
        // the same picture in the interface's pixels, kept twice as fine
        assert_eq!((fine.scale, fine.width, fine.height, reach), (2, coarse.width, coarse.height, 6));
        assert_eq!(fine.pixels.len(), 4 * coarse.pixels.len());
        // everything here is made of whole pixels (the letters are the bundled ones),
        // so inside the window each is a square of four alike, the colour it was
        let (wide, _) = fine.span();
        for y in reach..coarse.height - reach {
            for x in reach..coarse.width - reach {
                let was = coarse.get(x, y);
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    assert_eq!(fine.pixels[((2 * y + dy) * wide + 2 * x + dx) as usize], was, "at {x},{y}");
                }
            }
        }
        // a menu and a word of help are as fine as the window
        let bar = ui.menu_bar_new();
        let menu = ui.menu_new("File");
        let item = ui.menu_item_new("Open");
        ui.menu_add(menu, item).unwrap();
        ui.menu_bar_add(bar, menu).unwrap();
        ui.window_set_menu_bar(window, bar).unwrap();
        ui.render(window).unwrap();
        ui.pointer(window, 12, 8, PointerKind::Press).unwrap();
        assert_eq!(ui.popups(window).len(), 1);
        assert_eq!(ui.render_popup(window, 0).unwrap().scale, 2);
    }

    #[test]
    fn a_window_casts_a_shadow_where_the_style_has_one() {
        let mut ui = Ui::natural();
        let (window, _) = dialog(&mut ui);
        ui.set_style(std::sync::Arc::new(Framed));
        ui.set_own_frames(true);
        let framed = ui.render_framed(window).unwrap();
        // no shadow in a style that casts none: the picture is the frame's
        let (plain, reach) = ui.render_shadowed(window).unwrap();
        assert_eq!((reach, plain.width), (0, framed.width));
        ui.set_style(std::sync::Arc::new(Shadowed));
        let (canvas, reach) = ui.render_shadowed(window).unwrap();
        assert_eq!((reach, canvas.width, canvas.height), (12, framed.width + 24, framed.height + 24));
        // the window is where it was, further in by the shadow's reach
        assert_eq!(canvas.get(12 + 50, 12 + 60), framed.get(50, 60));
        // below it the shadow is black and part see-through, fading to nothing; above, there is less
        let under = |y: i32| canvas.get(12 + framed.width / 2, y) >> 24;
        let foot = 12 + framed.height;
        assert!(under(foot + 1) > 0 && under(foot + 1) < 255 && under(foot + 1) > under(foot + 9));
        assert_eq!(canvas.get(12 + framed.width / 2, foot + 1) & 0xff_ffff, 0);
        assert!(under(2) < under(foot + 2));
        assert_eq!(canvas.get(0, 0) >> 24, 0);
        // it may be taken hold of by an edge a few pixels out into its shadow, and no further
        assert_eq!(ui.frame_grip(window), 5);
        assert_eq!(ui.frame_pointer(window, -3, 60, PointerKind::Press), FrameAct::Resize { left: true, right: false, top: false, bottom: false });
        assert_eq!(ui.frame_pointer(window, framed.width + 2, framed.height + 2, PointerKind::Press), FrameAct::Resize { left: false, right: true, top: false, bottom: true });
        assert_eq!(ui.frame_pointer(window, -9, 60, PointerKind::Press), FrameAct::Nothing);
        // made as large as it can be, it casts none
        ui.window_set_zoomed(window, true);
        assert_eq!((ui.frame_shadow(window), ui.frame_grip(window)), (0, 0));
    }

    #[test]
    fn a_row_of_buttons_is_in_the_order_the_style_has_them_at_the_far_end() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Ask", 300, 60).unwrap();
        let row = ui.button_row_new();
        let (ok, cancel) = (ui.button_new("OK"), ui.button_new("Cancel"));
        ui.stack_add(row, ok).unwrap();
        ui.stack_add(row, cancel).unwrap();
        ui.window_set_content(window, row).unwrap();
        assert!(ui.is_a(row, "ButtonRow").unwrap() && ui.is_a(row, "Stack").unwrap());
        let places = |ui: &mut Ui| {
            ui.render(window).unwrap();
            (ui.nodes.get(ok).unwrap().rect, ui.nodes.get(cancel).unwrap().rect)
        };
        // black and white has the one that agrees last, at the right of the window
        let (o, c) = places(&mut ui);
        assert!(c.x < o.x && o.x + o.w == 300, "{c:?} {o:?}");
        // and the keyboard goes along them as they are shown
        assert_eq!(ui.children(row), vec![cancel, ok]);
        // a style that has OK first shows them as they were added, still at the far end
        ui.set_style(std::sync::Arc::new(Shadowed));
        let (o, c) = places(&mut ui);
        assert!(o.x < c.x && c.x + c.w == 300, "{o:?} {c:?}");
    }

    #[test]
    fn a_frame_is_moved_by_its_title_closed_by_its_box_and_stretched_by_its_edges() {
        let mut ui = Ui::natural();
        let (window, button) = dialog(&mut ui);
        ui.set_style(std::sync::Arc::new(Framed));
        ui.set_own_frames(true);
        ui.render_framed(window).unwrap();
        // in the content, a point is the content's, measured from its corner
        assert_eq!(ui.frame_pointer(window, 3 + 10, 20 + 10, PointerKind::Move), FrameAct::Inside(10, 10));
        // the title bar takes hold of the window; twice on it zooms
        assert_eq!(ui.frame_pointer(window, 100, 10, PointerKind::Press), FrameAct::Move);
        assert_eq!(ui.frame_pointer(window, 100, 10, PointerKind::Press), FrameAct::Zoom);
        // the close box is at the left, and acts when let go on it
        let (which, close) = ui.frame_buttons(window)[0];
        assert_eq!((which, close.x), (frame::CLOSE, 5));
        assert_eq!(ui.frame_pointer(window, close.x + 2, close.y + 2, PointerKind::Press), FrameAct::Nothing);
        assert_eq!(ui.frame_pointer(window, close.x + 2, close.y + 2, PointerKind::Release), FrameAct::Close);
        // let go elsewhere, it does nothing
        ui.frame_pointer(window, close.x + 2, close.y + 2, PointerKind::Press);
        assert_eq!(ui.frame_pointer(window, 100, 60, PointerKind::Release), FrameAct::Nothing);
        // the bottom right corner stretches both ways, the right edge one
        assert_eq!(ui.frame_pointer(window, 205, 122, PointerKind::Press), FrameAct::Resize { left: false, right: true, top: false, bottom: true });
        assert_eq!(ui.frame_pointer(window, 205, 60, PointerKind::Press), FrameAct::Resize { left: false, right: true, top: false, bottom: false });
        // a press that began on a button in the content stays the content's when it strays over the frame
        let r = ui.nodes.get(button).unwrap().rect;
        ui.pointer(window, r.x + 2, r.y + 2, PointerKind::Press).unwrap();
        assert!(matches!(ui.frame_pointer(window, 100, 10, PointerKind::Move), FrameAct::Inside(..)));
    }

    /// A style whose title bar is a tab.
    struct Tabbed;

    impl Style for Tabbed {
        fn name(&self) -> &str {
            "tabbed"
        }
        fn metrics(&self) -> Metrics {
            Metrics { frame_tab: 6, ..Framed.metrics() }
        }
        fn palette(&self) -> Palette {
            EightyFour::PALETTE
        }
        fn draw(&self, _: &Part, _: &mut Painter) -> bool {
            false
        }
    }

    #[test]
    fn a_title_bar_may_be_a_tab_as_wide_as_its_title() {
        let mut ui = Ui::natural();
        let (window, _) = dialog(&mut ui);
        ui.set_own_frames(true);
        ui.set_style(std::sync::Arc::new(Framed));
        assert_eq!(ui.frame_bar_width(window), 206);
        ui.set_style(std::sync::Arc::new(Tabbed));
        let bar = ui.frame_bar_width(window);
        assert!(bar < 206 && bar > ui.font.width("Style"), "the tab is {bar} wide");
        // the button at its right is at the right of the tab, not of the window
        let (which, zoom) = ui.frame_buttons(window)[1];
        assert_eq!(which, frame::ZOOM);
        assert!(zoom.x + zoom.w <= bar);
        // the tab takes hold of the window; the strip beside it is nothing of the window's
        assert_eq!(ui.frame_pointer(window, bar / 2, 8, PointerKind::Press), FrameAct::Move);
        assert_eq!(ui.frame_pointer(window, bar + 20, 8, PointerKind::Press), FrameAct::Nothing);
    }

    /// A style with the manners of another kind of desktop, which also
    /// notes down every part it is asked to draw.
    struct Mannered(std::sync::Mutex<Vec<(PartKind, u32, i32, i32)>>);

    impl Style for Mannered {
        fn name(&self) -> &str {
            "mannered"
        }
        fn metrics(&self) -> Metrics {
            Metrics { shortcut_form: 1, frame_foot: 9, frame_double: 1, scroll_arrows: 1, tab_align: 1, menu_border: 3, frame_left: 0x5, frame_right: 0x64, ..Framed.metrics() }
        }
        fn palette(&self) -> Palette {
            Palette { list: 0xffaaaaaa, pane: 0xffeeeeee, ..EightyFour::PALETTE }
        }
        fn wants(&self) -> u32 {
            crate::style::wants::HOVER
        }
        fn draw(&self, part: &Part, _: &mut Painter) -> bool {
            self.0.lock().unwrap().push((part.kind, part.state, part.a, part.d));
            false
        }
    }

    fn mannered() -> (Ui, std::sync::Arc<Mannered>) {
        let style = std::sync::Arc::new(Mannered(Default::default()));
        let mut ui = Ui::natural();
        ui.set_own_frames(true);
        ui.set_style(style.clone());
        (ui, style)
    }

    #[test]
    fn a_style_says_how_shortcuts_are_written_and_how_thick_a_menu_is() {
        let (mut ui, style) = mannered();
        let (window, _) = dialog(&mut ui);
        let (bar, menu) = (ui.menu_bar_new(), ui.menu_new("File"));
        let item = ui.menu_item_new("Loud");
        ui.menu_item_set_shortcut(item, "l").unwrap();
        ui.menu_item_set_checked(item, true).unwrap();
        ui.menu_add(menu, item).unwrap();
        ui.menu_bar_add(bar, menu).unwrap();
        ui.window_set_menu_bar(window, bar).unwrap();
        ui.render(window).unwrap();
        ui.pointer(window, 20, 8, PointerKind::Press).unwrap();
        let plain_width = {
            let mut plain = Ui::natural();
            plain.font.width("\u{2318}L")
        };
        assert!(ui.font.width("Ctrl+L") > plain_width);
        let popup = ui.popups(window)[0].rect;
        // an edge of three pixels all round the one item
        assert_eq!(popup.h, EightyFour::METRICS.menu_item_height + 2 * 3 + EightyFour::METRICS.menu_shadow);
        style.0.lock().unwrap().clear();
        ui.render_popup(window, 0).unwrap();
        let drawn: Vec<PartKind> = style.0.lock().unwrap().iter().map(|p| p.0).collect();
        // the tick beside the item is the style's to draw now
        assert!(drawn.contains(&PartKind::MenuCheck) && drawn.contains(&PartKind::MenuFrame));
    }

    #[test]
    fn a_title_bar_may_have_a_button_that_puts_the_window_behind() {
        let (mut ui, _) = mannered();
        let (window, _) = dialog(&mut ui);
        let (_, back) = *ui.frame_buttons(window).iter().find(|b| b.0 == frame::BACK).expect("the style asked for one");
        assert_eq!(ui.frame_pointer(window, back.x + 2, back.y + 2, PointerKind::Press), FrameAct::Nothing);
        // it is for the display to do: the engine has one window's word only
        assert_eq!(ui.frame_pointer(window, back.x + 2, back.y + 2, PointerKind::Release), FrameAct::Back);
    }

    #[test]
    fn a_window_rolls_up_into_its_title_and_has_a_menu_of_its_own() {
        let (mut ui, _) = mannered();
        let (window, _) = dialog(&mut ui);
        // the frame is thicker along its foot than along its sides
        assert_eq!(ui.frame_insets(window), (3, 20, 3, 9));
        let whole = ui.render_framed(window).unwrap().height;
        // the button at the right rolls it up; so do two clicks on the title
        let (which, roll) = *ui.frame_buttons(window).iter().find(|b| b.0 == frame::ROLL).unwrap();
        assert_eq!(which, frame::ROLL);
        ui.frame_pointer(window, roll.x + 2, roll.y + 2, PointerKind::Press);
        assert_eq!(ui.frame_pointer(window, roll.x + 2, roll.y + 2, PointerKind::Release), FrameAct::Nothing);
        assert!(ui.window_rolled(window));
        assert_eq!(ui.render_framed(window).unwrap().height, 20 + 9);
        ui.frame_pointer(window, 100, 10, PointerKind::Press);
        ui.frame_pointer(window, 100, 10, PointerKind::Press);
        assert!(!ui.window_rolled(window));
        assert_eq!(ui.render_framed(window).unwrap().height, whole);
        // the button at the left drops the window's own menu, and what is chosen there waits for the display
        let menu = ui.frame_buttons(window)[0];
        assert_eq!(menu.0, frame::MENU);
        ui.tick(10_000);
        assert_eq!(ui.frame_pointer(window, menu.1.x + 2, menu.1.y + 2, PointerKind::Press), FrameAct::Nothing);
        assert_eq!(ui.popups(window).len(), 1);
        ui.frame_ask(window, frame::CLOSE);
        assert_eq!(ui.take_frame_ask(), Some((window, FrameAct::Close)));
        assert_eq!(ui.take_frame_ask(), None);
    }

    #[test]
    fn the_pointer_changes_over_an_edge_of_a_frame_that_can_be_dragged() {
        let (mut ui, _) = mannered();
        let (window, _) = dialog(&mut ui);
        ui.render_framed(window).unwrap();
        // measured from the corner of the content: the right edge, the foot, and the corner between
        assert_eq!(ui.cursor_at(window, 201, 50), Cursor::ResizeColumn);
        assert_eq!(ui.cursor_at(window, 100, 105), Cursor::ResizeRow);
        assert_eq!(ui.cursor_at(window, 201, 105), Cursor::ResizeCorner);
        assert_eq!(ui.cursor_at(window, 100, -10), Cursor::Arrow);
    }

    #[test]
    fn tabs_stand_in_the_middle_and_scroll_arrows_together_where_the_style_says() {
        let (mut ui, style) = mannered();
        let window = ui.window_new("Tabs", 300, 200).unwrap();
        let tabs = ui.tab_view_new();
        let (one, two) = (ui.label_new("One"), ui.scroll_bar_new(Orientation::Vertical));
        ui.tab_view_add(tabs, "First", one).unwrap();
        ui.tab_view_add(tabs, "Second", two).unwrap();
        ui.window_set_content(window, tabs).unwrap();
        ui.render(window).unwrap();
        let (rects, whole) = (ui.tab_rects(tabs), ui.nodes.get(tabs).unwrap().rect);
        let (left, right) = (rects[0].x - whole.x, whole.x + whole.w - (rects[1].x + rects[1].w));
        assert!((left - right).abs() <= 1, "tabs are {left} from the left and {right} from the right");
        // the pointer over a tab is told to a style that wants to hear of it
        style.0.lock().unwrap().clear();
        ui.pointer(window, rects[1].x + 3, rects[1].y + 3, PointerKind::Move).unwrap();
        ui.render(window).unwrap();
        assert!(style.0.lock().unwrap().iter().any(|p| p.0 == PartKind::Tab && p.1 & state::HOT != 0));
        // both arrows of a scroll bar are at its far end
        ui.tab_view_set_selected(tabs, 1).unwrap();
        ui.render(window).unwrap();
        let r = ui.nodes.get(two).unwrap().rect;
        let thick = EightyFour::METRICS.scroll_bar;
        assert_ne!(ui.bar_part_at(two, false, r.x + 2, r.y + 2), Some(1), "no arrow at the near end");
        assert_eq!(ui.bar_part_at(two, false, r.x + 2, r.y + r.h - thick - 2), Some(1));
        assert_eq!(ui.bar_part_at(two, false, r.x + 2, r.y + r.h - 2), Some(2));
    }

    #[test]
    fn a_box_and_a_bar_are_stacks_with_room_kept_for_their_edges() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Extras", 300, 200).unwrap();
        let (plain, boxed, bar) = (ui.stack_new(Orientation::Vertical), ui.group_box_new("Paper", Orientation::Vertical), ui.toolbar_new());
        for stack in [plain, boxed] {
            let label = ui.label_new("Words");
            ui.stack_add(stack, label).unwrap();
        }
        let (bare, dressed) = (ui.measure(plain), ui.measure(boxed));
        assert!(dressed.0 > bare.0 && dressed.1 > bare.1 + 10, "{bare:?} and {dressed:?}");
        // each is what it is, and a stack besides; a plain stack is not a box
        assert!(ui.is_a(boxed, "GroupBox").unwrap() && ui.is_a(boxed, "Stack").unwrap());
        assert!(ui.is_a(bar, "Toolbar").unwrap() && !ui.is_a(bar, "GroupBox").unwrap());
        assert!(!ui.is_a(plain, "GroupBox").unwrap());
        assert_eq!(ui.group_box_set_title(plain, "No"), Err(Error::WrongType));
        // the box is drawn before what is in it: its line is on the window
        ui.window_set_content(window, boxed).unwrap();
        let canvas = ui.render(window).unwrap();
        let r = ui.nodes.get(boxed).unwrap().rect;
        assert_eq!(canvas.get(r.x, r.y + r.h / 2), BLACK);
    }

    #[test]
    fn a_tool_lies_flat_until_the_pointer_comes_and_help_shows_when_it_rests() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Extras", 300, 200).unwrap();
        let bar = ui.toolbar_new();
        let tool = ui.button_new("Open");
        ui.button_set_flat(tool, true).unwrap();
        ui.set_tooltip(tool, "Open a sheet").unwrap();
        ui.stack_add(bar, tool).unwrap();
        ui.window_set_content(window, bar).unwrap();
        let before = ui.render(window).unwrap();
        let r = ui.nodes.get(tool).unwrap().rect;
        // no narrower than its words, and no wider than a tool needs to be
        assert!(r.w < EightyFour::METRICS.button_min_width);
        // flat: no line along its top
        assert_eq!(before.get(r.x + r.w / 2, r.y), WHITE);
        ui.dirty = false;
        ui.pointer(window, r.x + 4, r.y + 4, PointerKind::Move).unwrap();
        assert!(ui.dirty, "the pointer coming over a tool is drawn in every style");
        assert_eq!(ui.render(window).unwrap().get(r.x + r.w / 2, r.y), BLACK);
        // help waits for the pointer to rest, shows, and is sent away by a press
        assert!(!ui.tip_showing());
        assert!(ui.next_deadline().is_some());
        ui.advance(800);
        assert!(ui.tip_showing());
        // it hangs over the window as a menu does, below the pointer, and may reach past the window's edge
        let tips = ui.popups(window);
        assert!(tips.len() == 1 && tips[0].tip && tips[0].rect.y > r.y + 4);
        let picture = ui.render_popup(window, 0).unwrap();
        assert_eq!((picture.width, picture.height), (tips[0].rect.w, tips[0].rect.h));
        assert_eq!(picture.get(0, 0), BLACK);
        // it stays where it came up while the pointer wanders over the same thing
        ui.pointer(window, r.x + 6, r.y + 5, PointerKind::Move).unwrap();
        assert_eq!(ui.popups(window), tips);
        ui.pointer(window, r.x + 4, r.y + 4, PointerKind::Press).unwrap();
        assert!(!ui.tip_showing() && ui.popups(window).is_empty());
    }

    #[test]
    fn a_stepper_steps_between_its_bounds() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Extras", 300, 200).unwrap();
        let stepper = ui.stepper_new();
        ui.stepper_set(stepper, 1, 1).unwrap();
        ui.stepper_set(stepper, 2, 3).unwrap();
        ui.stepper_set(stepper, 0, 2).unwrap();
        ui.window_set_content(window, stepper).unwrap();
        ui.render(window).unwrap();
        let r = ui.nodes.get(stepper).unwrap().rect;
        let (down, up) = ((r.x + r.w - 4, r.y + r.h - 3), (r.x + r.w - 4, r.y + 3));
        let click = |ui: &mut Ui, at: (i32, i32)| {
            let mut out = ui.pointer(window, at.0, at.1, PointerKind::Press).unwrap();
            out.extend(ui.pointer(window, at.0, at.1, PointerKind::Release).unwrap());
            out
        };
        assert_eq!(click(&mut ui, up), vec![Emitted { target: stepper, event: "Stepper.changed", args: vec![Value::I32(3)] }]);
        assert_eq!(click(&mut ui, up), vec![], "no further than its most");
        click(&mut ui, down);
        click(&mut ui, down);
        assert_eq!(click(&mut ui, down), vec![]);
        assert_eq!(ui.stepper_value(stepper), Ok(1));
        // the arrow keys step it when it has the keyboard
        ui.focus(stepper).unwrap();
        ui.key(window, Key::Up, Mods::default()).unwrap();
        assert_eq!(ui.stepper_value(stepper), Ok(2));
        // an arrow held down steps again after a pause, and again, until it is let go
        ui.stepper_set(stepper, 2, 50).unwrap();
        ui.pointer(window, up.0, up.1, PointerKind::Press).unwrap();
        assert_eq!(ui.stepper_value(stepper), Ok(3));
        ui.advance(100);
        assert_eq!(ui.stepper_value(stepper), Ok(3));
        assert_eq!(ui.advance(250).len(), 1);
        ui.advance(70);
        assert_eq!(ui.stepper_value(stepper), Ok(5));
        // not while the pointer has slid off it
        ui.pointer(window, r.x + 4, r.y + 4, PointerKind::Move).unwrap();
        ui.advance(70);
        assert_eq!(ui.stepper_value(stepper), Ok(5));
        ui.pointer(window, r.x + 4, r.y + 4, PointerKind::Release).unwrap();
        ui.advance(500);
        assert_eq!((ui.stepper_value(stepper), ui.next_deadline()), (Ok(5), None));
    }

    #[test]
    fn a_bar_of_news_can_keep_each_thing_in_a_cell() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Extras", 300, 200).unwrap();
        let bar = ui.status_bar_new();
        for words in ["Ready", "Page 1"] {
            let label = ui.label_new(words);
            ui.stack_add(bar, label).unwrap();
        }
        ui.window_set_content(window, bar).unwrap();
        let strip = ui.render(window).unwrap();
        ui.status_bar_set_panels(bar, true).unwrap();
        assert!(ui.render(window).unwrap().pixels != strip.pixels);
        let plain = ui.stack_new(Orientation::Horizontal);
        assert_eq!(ui.status_bar_set_panels(plain, true), Err(Error::WrongType));
    }

    #[test]
    fn a_combo_box_is_a_text_field_that_drops_a_list_of_what_it_might_say() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Extras", 300, 200).unwrap();
        let combo = ui.combo_box_new("A4");
        ui.combo_box_add(combo, "A3").unwrap();
        ui.combo_box_add(combo, "Letter").unwrap();
        assert!(ui.is_a(combo, "ComboBox").unwrap() && ui.is_a(combo, "TextField").unwrap());
        let plain = ui.text_field_new("A4");
        assert_eq!(ui.combo_box_add(plain, "No"), Err(Error::WrongType));
        ui.window_set_content(window, combo).unwrap();
        ui.render(window).unwrap();
        let r = ui.nodes.get(combo).unwrap().rect;
        // a press on the button at its end drops the list
        ui.pointer(window, r.x + r.w - 8, r.y + r.h / 2, PointerKind::Press).unwrap();
        ui.pointer(window, r.x + r.w - 8, r.y + r.h / 2, PointerKind::Release).unwrap();
        let list = ui.popups(window);
        assert_eq!(list.len(), 1);
        // and one chosen from it is what the field says
        let at = (list[0].rect.x + 10, list[0].rect.y + EightyFour::METRICS.menu_item_height + 6);
        ui.pointer(window, at.0, at.1, PointerKind::Press).unwrap();
        let mut out = ui.pointer(window, at.0, at.1, PointerKind::Release).unwrap();
        out.extend(ui.finish_animations());
        assert_eq!(ui.text_field_text(combo).unwrap(), "Letter");
        assert!(out.iter().any(|e| e.event == "TextField.changed"));
    }

    #[test]
    fn a_picture_in_colour_goes_before_a_buttons_label() {
        let mut ui = Ui::natural();
        let window = ui.window_new("Extras", 300, 200).unwrap();
        let (plain, pictured) = (ui.button_new("Open"), ui.button_new("Open"));
        ui.button_set_image(pictured, Some(Canvas::filled(16, 16, 0xff2060c0))).unwrap();
        assert!(ui.measure(pictured).0 >= ui.measure(plain).0);
        ui.window_set_content(window, pictured).unwrap();
        let canvas = ui.render(window).unwrap();
        assert!(canvas.pixels.contains(&0xff2060c0));
        // too large a picture is not taken, and none takes one away
        ui.button_set_image(pictured, Some(Canvas::new(400, 400))).unwrap();
        assert!(!ui.render(window).unwrap().pixels.contains(&0xff2060c0));
    }

    #[test]
    fn a_button_that_cannot_be_pressed_keeps_its_edge_in_black_and_white() {
        let mut ui = Ui::natural();
        // its words are greyed, and not its edge, as on the first system
        let win = ui.window_new("Buttons", 200, 60).unwrap();
        let button = ui.button_new("Save");
        ui.set_enabled(button, false).unwrap();
        let column = ui.stack_new(Orientation::Vertical);
        ui.stack_add(column, button).unwrap();
        ui.window_set_content(win, column).unwrap();
        let canvas = ui.render(win).unwrap();
        let r = ui.nodes.get(button).unwrap().rect;
        assert_eq!(r.h, 18);
        assert!((r.x + 3..r.x + r.w - 3).all(|x| canvas.get(x, r.y) == BLACK && canvas.get(x, r.y + r.h - 1) == BLACK));
        assert!((r.y + 3..r.y + r.h - 3).all(|y| canvas.get(r.x, y) == BLACK));
    }


    #[test]
    fn a_window_can_be_given_a_new_size_while_it_shows() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Calculator", 104, 146).unwrap();
        ui.window_set_resizable(win, false).unwrap();
        assert!(!ui.take_resized(win));
        ui.window_set_size(win, 516, 314).unwrap();
        // the display is told once, and a window of one size is of the new one
        assert!(ui.take_resized(win) && !ui.take_resized(win));
        assert_eq!(ui.window_fixed_size(win).unwrap(), Some((516, 314)));
        assert_eq!(ui.window_set_size(win, 0, 10), Err(Error::InvalidArgument));
        let label = ui.label_new("x");
        assert_eq!(ui.window_set_size(label, 10, 10), Err(Error::WrongType));
    }

}
