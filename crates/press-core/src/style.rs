// SPDX-License-Identifier: GPL-3.0-or-later
//! How things look, apart from what they are.
//!
//! The engine knows what a button is: where it is, what it says, whether it
//! is held. A [`Style`] knows what a button looks like. The engine asks the
//! style for its sizes ([`Metrics`]) and its colours ([`Palette`]), and when
//! it draws, it hands each piece of a window to the style as a [`Part`].
//!
//! The look the engine was born with, black and white in the manner of the
//! first Macintosh, is one style among others: [`EightyFour`]. It is also the
//! one that draws whatever another style declines to, so a style may be
//! written a part at a time.
//!
//! Everything here is plain data with a fixed layout, so that a style can
//! live in a library of its own, in any language (see `press_style.h`).

use crate::canvas::{Canvas, Rect, BLACK, TRANSPARENT, WHITE};
use crate::font::{Face, Font, Type};

/// The sizes a style decides, in pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub button_height: i32,
    pub button_min_width: i32,
    /// Room either side of a button's label.
    pub button_pad: i32,
    /// How much larger all round a default button is than another.
    pub default_ring: i32,
    pub check_size: i32,
    /// Between a checkbox and its label.
    pub check_gap: i32,
    pub radio_size: i32,
    pub radio_gap: i32,
    /// From a text field's edge to its text, sideways and upright.
    pub field_inset_x: i32,
    pub field_inset_y: i32,
    pub progress_height: i32,
    /// A slider's size across its direction of travel.
    pub slider_thick: i32,
    /// How thick a scroll bar is, and how long each of its arrow boxes.
    pub scroll_bar: i32,
    /// The length of a slider's knob along its travel.
    pub slider_knob: i32,
    /// Height of a tab, and of the row of them.
    pub tab_height: i32,
    pub popup_height: i32,
    /// Height of a row in a list, a table or a tree.
    pub row_height: i32,
    /// Height of a window's menu bar, the rule under it included.
    pub menu_bar_height: i32,
    pub menu_item_height: i32,
    /// Room kept to the right of a menu and below it for its shadow.
    pub menu_shadow: i32,
    /// Height of the title bar of a window that draws its own frame, its
    /// top edge included. Nothing here leaves the frame to the desktop.
    pub frame_title: i32,
    /// How thick the frame is along the sides and the bottom of a window.
    pub frame_border: i32,
    /// The buttons in a title bar: their size, the room between them, how
    /// far the first stands from the side of the frame, and how far below
    /// the middle of the title bar they sit.
    pub frame_button_width: i32,
    pub frame_button_height: i32,
    pub frame_button_gap: i32,
    pub frame_button_inset: i32,
    pub frame_button_rise: i32,
    /// Which buttons are at the left of a title bar and which at the
    /// right: four bits for each, the one nearest the side of the frame in
    /// the lowest four. 1 closes, 2 minimizes, 3 zooms.
    pub frame_left: i32,
    pub frame_right: i32,
    /// Not nothing: the menu bar stays in the window, even on a desktop
    /// that offers to show it across the top of the screen.
    pub menu_in_window: i32,
    /// Not nothing: the title bar is a tab at the left of the window's top,
    /// only as wide as its buttons and the title with this much room either
    /// side of it, and the rest of that strip is not the window's.
    pub frame_tab: i32,
    /// How a menu's shortcuts are written: 0 with the sign of the command
    /// key before the letter, 1 as Ctrl+ and the letter, 2 with a caret.
    pub shortcut_form: i32,
    /// How thick the frame is along the bottom of a window, where that is
    /// not as thick as along its sides: nothing, for the same.
    pub frame_foot: i32,
    /// What two clicks on a title bar do: 0 zoom the window, 1 roll it up
    /// into its title bar, 2 put it away.
    pub frame_double: i32,
    /// Where a scroll bar keeps its arrows: 0 one at each end, 1 both at
    /// the far end (the bottom, the right).
    pub scroll_arrows: i32,
    /// Where the tabs of a tab view stand along its top: 0 from the left,
    /// 1 in the middle.
    pub tab_align: i32,
    /// How thick the edge of a menu is, inside which its items lie.
    pub menu_border: i32,
}

/// The colours a style decides, as ARGB. The engine draws words and small
/// marks itself, and takes their colours from here.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// What a window is made of, and the words written straight on it.
    pub window: u32,
    pub text: u32,
    /// Where the user's own things are: text fields and lists.
    pub paper: u32,
    pub ink: u32,
    /// A menu bar and the menus pulled down from it.
    pub menu: u32,
    pub menu_text: u32,
    /// Lists, tables and trees, where they are not on the same paper as
    /// what is typed.
    pub list: u32,
    pub list_ink: u32,
    /// What the pages of a tab view lie on, where that is not what a
    /// window is made of.
    pub pane: u32,
}

/// The pieces a window is drawn from.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    /// The whole of a window, before anything is drawn on it.
    Background = 1,
    /// A push button with its label. `DEFAULT`, `HELD`, `FOCUSED`.
    Button = 2,
    /// A checkbox with its label. `CHECKED`, `HELD`, `FOCUSED`.
    Checkbox = 3,
    /// A radio button with its label. `CHECKED`, `HELD`, `FOCUSED`.
    Radio = 4,
    /// The box round a text field or a list, and the paper in it.
    /// `FOCUSED` when it has the keyboard.
    Well = 5,
    /// Show what is already drawn in the rectangle as selected.
    Select = 6,
    /// Show what is already drawn in the rectangle as out of use.
    Dim = 7,
    /// A ring round the rectangle: this has the keyboard.
    Focus = 8,
    /// A slider. `VERTICAL`, `FOCUSED`, `HELD`; `a` is where the knob
    /// starts along it and `b` how long the knob is.
    Slider = 9,
    /// A progress bar. `BUSY` for one that only shows work going on;
    /// otherwise `a` of `b` is done.
    Progress = 10,
    /// A scroll bar. `VERTICAL`; `ENABLED` when there is somewhere to
    /// scroll to; `a` is 1 while the first arrow is held and 2 for the
    /// second; `b` is where the sliding box starts along the bar, or -1
    /// for none, and `c` its length.
    ScrollBar = 11,
    /// The square where two scroll bars meet.
    ScrollCorner = 12,
    /// One tab of a tab view with its title. `FRONT` for the chosen one,
    /// drawn after the pane; `FOCUSED`.
    Tab = 13,
    /// The framed area a tab view's pages show in.
    TabPane = 14,
    /// A pop-up button showing its choice. `FOCUSED`; `HELD` while its
    /// list is up.
    Popup = 15,
    /// A menu bar across the top of a window, before its titles.
    MenuBar = 16,
    /// One title in a menu bar. `HELD` while its menu is down; `ENABLED`.
    MenuTitle = 17,
    /// The outline of a pulled-down menu and its shadow, on a picture of
    /// their own: the items are a pixel inside, and the last column and row
    /// are the shadow's. Pixels left [`TRANSPARENT`] show the desktop.
    MenuFrame = 18,
    /// A dividing line in a menu, in the room of one item.
    MenuSeparator = 19,
    /// The box round a scroll view.
    ViewFrame = 20,
    /// The row of headings over a table, before the headings.
    Header = 21,
    /// One heading of a table, before its words.
    HeaderCell = 22,
    /// The frame of a window that draws its own: the title bar with the
    /// window's title, and the edges. The rectangle is the whole window,
    /// see-through until drawn on; `a` is the height of the title bar,
    /// `b` the thickness of the edges and `c` how wide the title bar is
    /// (all of the window, unless it is a tab), and the content is laid
    /// over the rest afterwards. `ENABLED` for the window in front; `CHECKED` for
    /// one made as large as it can be.
    Frame = 23,
    /// A button in a title bar: `a` is 1 for the one that closes, 2 for
    /// the one that minimizes, 3 for the one that zooms. `HOT`, `HELD`,
    /// `ENABLED` and `CHECKED` as for the frame.
    FrameButton = 24,
    /// The mark beside a menu's item that is switched on, in the room at
    /// the left of the item. Drawn in `ink`.
    MenuCheck = 25,
    /// The mark at the right of a menu's item that opens another menu. The
    /// rectangle is the item's. Drawn in `ink`.
    MenuArrow = 26,
    /// The mark in a table's heading that says the rows are in its order:
    /// its middle is at (`a`, `b`). `CHECKED` for smallest first.
    SortArrow = 27,
    /// The mark before a row of a tree that has rows under it: its middle
    /// is at (`a`, `b`). `CHECKED` when they are showing.
    TreeFold = 28,
    /// Where typing goes, in a text field: the rectangle is one pixel wide
    /// and as tall as the line.
    Caret = 29,
}

/// What state a part is in: these are added together in [`Part::state`].
pub mod state {
    pub const ENABLED: u32 = 1;
    /// The pointer is down on it, or its menu is showing.
    pub const HELD: u32 = 2;
    /// It has the keyboard, in the window being typed into.
    pub const FOCUSED: u32 = 4;
    /// Return presses it.
    pub const DEFAULT: u32 = 8;
    pub const CHECKED: u32 = 16;
    pub const VERTICAL: u32 = 32;
    pub const FRONT: u32 = 64;
    pub const BUSY: u32 = 128;
    /// The pointer is over it. Only told to a style that [`wants`](super::wants) it.
    pub const HOT: u32 = 256;
}

/// What a style would have the engine do for it beyond the ordinary: these
/// are added together in what [`Style::wants`] returns.
pub mod wants {
    /// Buttons and the like are drawn again as the pointer comes over them
    /// and goes, with [`HOT`](super::state::HOT) set.
    pub const HOVER: u32 = 1;
    /// A default button is drawn again some twenty times a second, so that
    /// it can throb; a button's `a` counts the times.
    pub const PULSE: u32 = 2;
}

/// One piece of a window to be drawn.
#[derive(Debug, Clone, Copy)]
pub struct Part<'a> {
    pub kind: PartKind,
    pub state: u32,
    pub rect: Rect,
    /// A count that goes up as time passes, for anything that moves: the
    /// marching of a focus ring, the stripes of a busy progress bar.
    pub phase: i32,
    pub a: i32,
    pub b: i32,
    pub c: i32,
    pub d: i32,
    /// The colour the engine would have drawn it in, for the small marks
    /// it used to draw itself: the ink of the paper or menu they are on.
    pub ink: u32,
    /// The words on it, if it has any.
    pub text: &'a str,
}

impl<'a> Part<'a> {
    pub fn new(kind: PartKind, rect: Rect) -> Part<'a> {
        Part { kind, state: state::ENABLED, rect, phase: 0, a: 0, b: 0, c: 0, d: 0, ink: BLACK, text: "" }
    }

    pub fn text(mut self, text: &'a str) -> Part<'a> {
        self.text = text;
        self
    }

    /// Add a state when a condition holds.
    pub fn when(mut self, flag: u32, on: bool) -> Part<'a> {
        if on {
            self.state |= flag;
        } else {
            self.state &= !flag;
        }
        self
    }

    pub fn phase(mut self, phase: i32) -> Part<'a> {
        self.phase = phase;
        self
    }

    pub fn numbers(mut self, a: i32, b: i32, c: i32) -> Part<'a> {
        (self.a, self.b, self.c) = (a, b, c);
        self
    }

    pub fn ink(mut self, ink: u32) -> Part<'a> {
        self.ink = ink;
        self
    }

    pub fn is(&self, flag: u32) -> bool {
        self.state & flag != 0
    }
}

/// What a style draws with: the picture, and the typeface in use.
pub struct Painter<'a> {
    pub canvas: &'a mut Canvas,
    pub font: &'a mut Font,
}

/// A look for the interface.
pub trait Style: Send + Sync {
    /// The name it is chosen by.
    fn name(&self) -> &str;
    fn metrics(&self) -> Metrics;
    fn palette(&self) -> Palette;
    /// The typeface to write in where the interface would use one of the
    /// bundled families: one of the machine's, drawn smooth. None keeps
    /// the family, in its own pixels.
    fn typeface(&self, _face: Face) -> Option<Type> {
        None
    }
    /// The typeface for the title of a window that draws its own frame,
    /// where that is not the interface's: bold, as often as not.
    fn frame_typeface(&self) -> Option<Type> {
        None
    }
    /// What it would have the engine do for it: see [`wants`].
    fn wants(&self) -> u32 {
        0
    }
    /// Draw a part. A style that returns false has drawn nothing, and the
    /// part is drawn as [`EightyFour`] draws it.
    fn draw(&self, part: &Part, painter: &mut Painter) -> bool;
}

/// Room either side of a tab's title.
pub const TAB_PAD: i32 = 10;
/// Room either side of a title in a menu bar.
pub const MENU_TITLE_PAD: i32 = 9;
/// Where an item's words start in a menu, leaving room for a tick.
pub const MENU_ITEM_LEFT: i32 = 17;

/// Black and white, one bit to a pixel, in the manner of the first
/// Macintosh, of 1984: named for its year, as the other styles are for
/// theirs. It was the engine's only look once.
#[derive(Debug, Clone, Copy, Default)]
pub struct EightyFour;

impl EightyFour {
    pub const METRICS: Metrics = Metrics {
        button_height: 20,
        button_min_width: 58,
        button_pad: 12,
        default_ring: 4, // 3 px ring plus a 1 px gap around a default button
        check_size: 12,
        check_gap: 6,
        radio_size: 12,
        radio_gap: 6,
        field_inset_x: 5, // 1 px of room for the focus frame, the frame, 3 px of padding
        field_inset_y: 3,
        progress_height: 12,
        slider_thick: 16,
        scroll_bar: 16,
        slider_knob: 9,
        tab_height: 20,
        popup_height: 19, // the box, and a pixel of shadow below it
        row_height: 16,
        menu_bar_height: 20,
        menu_item_height: 16,
        menu_shadow: 1,
        // its frame is the desktop's, and its menu bar goes where the desktop likes
        frame_title: 0,
        frame_border: 0,
        frame_button_width: 0,
        frame_button_height: 0,
        frame_button_gap: 0,
        frame_button_inset: 0,
        frame_button_rise: 0,
        frame_left: 0,
        frame_right: 0,
        menu_in_window: 0,
        frame_tab: 0,
        shortcut_form: 0,
        frame_foot: 0,
        frame_double: 0,
        scroll_arrows: 0,
        tab_align: 0,
        menu_border: 1,
    };

    pub const PALETTE: Palette = Palette { window: WHITE, text: BLACK, paper: WHITE, ink: BLACK, menu: WHITE, menu_text: BLACK, list: WHITE, list_ink: BLACK, pane: WHITE };

    /// How far the focus ring stands off a control.
    const RING_GAP: i32 = 3;

    fn scroll_bar(part: &Part, canvas: &mut Canvas) {
        let (rect, vertical, live) = (part.rect, part.is(state::VERTICAL), part.is(state::ENABLED));
        let (start, len) = if vertical { (rect.y, rect.h) } else { (rect.x, rect.w) };
        let span = |start: i32, len: i32| if vertical { Rect::new(rect.x, start, rect.w, len) } else { Rect::new(start, rect.y, len, rect.h) };
        let thick = if vertical { rect.w } else { rect.h };
        if len < 2 * thick {
            return; // no room for the arrows: draw nothing rather than a muddle
        }
        canvas.fill(rect, WHITE);
        canvas.frame(rect, BLACK);
        // the stretch the box slides in overlaps each arrow box by a pixel
        let (track, track_len) = (start + thick - 1, (len - 2 * (thick - 1)).max(0));
        // the track is grey where there is somewhere to go
        if live {
            let inside = span(track + 1, track_len - 2);
            let r = if vertical { Rect::new(inside.x + 1, inside.y, inside.w - 2, inside.h) } else { Rect::new(inside.x, inside.y + 1, inside.w, inside.h - 2) };
            canvas.fill(r, BLACK);
            canvas.dim(r);
        }
        for (back, box_start) in [(true, start), (false, start + len - thick)] {
            let r = span(box_start, thick);
            canvas.frame(r, BLACK);
            // a solid arrowhead, pointing the way the content will move into view
            let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
            for d in 0..5 {
                let (ax, aw) = (-d, 2 * d + 1);
                let (dx, dy, w, h) = match (vertical, back) {
                    (true, true) => (ax, d - 2, aw, 1),
                    (true, false) => (ax, 2 - d, aw, 1),
                    (false, true) => (d - 2, ax, 1, aw),
                    (false, false) => (2 - d, ax, 1, aw),
                };
                canvas.fill(Rect::new(cx + dx - if vertical { 0 } else { 1 }, cy + dy - if vertical { 1 } else { 0 }, w, h), BLACK);
            }
            if !live {
                canvas.dim(r.inset(1));
            } else if part.a == if back { 1 } else { 2 } {
                canvas.invert(r.inset(1));
            }
        }
        if part.b >= 0 {
            let r = span(part.b, part.c);
            canvas.fill(r, WHITE);
            canvas.frame(r, BLACK);
        }
    }

    fn slider(part: &Part, canvas: &mut Canvas) {
        let (rect, vertical, knob) = (part.rect, part.is(state::VERTICAL), part.a);
        let size = part.b;
        if part.is(state::FOCUSED) {
            canvas.ants(rect.inset(-Self::RING_GAP), part.phase);
        }
        // a thin groove, filled from the low end up to the knob
        let (groove, filled, knob_rect) = if vertical {
            let g = Rect::new(rect.x + rect.w / 2 - 2, rect.y + size / 2, 4, rect.h - size + 1);
            (g, Rect::new(g.x, knob, g.w, g.y + g.h - knob), Rect::new(rect.x + 1, knob, rect.w - 2, size))
        } else {
            let g = Rect::new(rect.x + size / 2, rect.y + rect.h / 2 - 2, rect.w - size + 1, 4);
            (g, Rect::new(g.x, g.y, knob - g.x, g.h), Rect::new(knob, rect.y + 1, size, rect.h - 2))
        };
        canvas.fill(groove, WHITE);
        canvas.frame(groove, BLACK);
        canvas.fill(filled, BLACK);
        // the knob: a white tab with a ridge to take hold of
        canvas.round_frame(knob_rect, 2, 1, BLACK, WHITE);
        let ridge = if vertical {
            Rect::new(knob_rect.x + 3, knob_rect.y + size / 2, knob_rect.w - 6, 1)
        } else {
            Rect::new(knob_rect.x + size / 2, knob_rect.y + 3, 1, knob_rect.h - 6)
        };
        canvas.fill(ridge, BLACK);
        if part.is(state::HELD) {
            canvas.invert(knob_rect.inset(1));
        }
    }
}

impl Style for EightyFour {
    fn name(&self) -> &str {
        "eightyfour"
    }

    fn metrics(&self) -> Metrics {
        Self::METRICS
    }

    fn palette(&self) -> Palette {
        Self::PALETTE
    }

    fn draw(&self, part: &Part, p: &mut Painter) -> bool {
        let m = Self::METRICS;
        let (rect, canvas) = (part.rect, &mut *p.canvas);
        let line = p.font.line_height();
        let (focused, held) = (part.is(state::FOCUSED), part.is(state::HELD));
        match part.kind {
            PartKind::Background => canvas.fill(rect, WHITE),
            PartKind::Button => {
                if focused {
                    canvas.ants(rect.inset(-Self::RING_GAP), part.phase);
                }
                let mut face = rect;
                if part.is(state::DEFAULT) {
                    canvas.round_frame(rect, 8, 3, BLACK, WHITE);
                    face = rect.inset(m.default_ring);
                }
                canvas.round_frame(face, 4, 1, BLACK, WHITE);
                let tw = p.font.width(part.text);
                p.font.draw(canvas, part.text, face.x + (face.w - tw) / 2, face.y + (face.h - line) / 2, BLACK);
                if held {
                    // invert the face, keeping the rounded corners white
                    let mut mask = Canvas::new(canvas.width, canvas.height);
                    mask.round_fill(face, 4, BLACK);
                    for y in face.y..face.y + face.h {
                        for x in face.x..face.x + face.w {
                            if mask.get(x, y) == BLACK {
                                canvas.invert(Rect::new(x, y, 1, 1));
                            }
                        }
                    }
                }
            }
            PartKind::Checkbox => {
                if focused {
                    canvas.ants(rect.inset(-Self::RING_GAP), part.phase);
                }
                let b = Rect::new(rect.x, rect.y + (rect.h - m.check_size) / 2, m.check_size, m.check_size);
                canvas.fill(b, WHITE);
                canvas.frame(b, BLACK);
                if held {
                    canvas.frame(b.inset(1), BLACK); // heavier box while the pointer is down
                }
                if part.is(state::CHECKED) {
                    for i in 1..m.check_size - 1 {
                        canvas.set(b.x + i, b.y + i, BLACK);
                        canvas.set(b.x + m.check_size - 1 - i, b.y + i, BLACK);
                    }
                }
                p.font.draw(canvas, part.text, rect.x + m.check_size + m.check_gap, rect.y + (rect.h - line) / 2, BLACK);
            }
            PartKind::Radio => {
                if focused {
                    canvas.ants(rect.inset(-Self::RING_GAP), part.phase);
                }
                let ring = Rect::new(rect.x, rect.y + (rect.h - m.radio_size) / 2, m.radio_size, m.radio_size);
                // a ring, heavier while the pointer is down on it, with a dot when chosen
                canvas.round_frame(ring, m.radio_size / 2, if held { 2 } else { 1 }, BLACK, WHITE);
                if part.is(state::CHECKED) {
                    canvas.round_fill(ring.inset(3), m.radio_size / 2 - 3, BLACK);
                }
                p.font.draw(canvas, part.text, rect.x + m.radio_size + m.radio_gap, rect.y + (rect.h - line) / 2, BLACK);
            }
            PartKind::Well => {
                let frame = rect.inset(1);
                canvas.fill(frame, WHITE);
                canvas.frame(frame, BLACK);
                if focused {
                    canvas.frame(rect, BLACK); // the frame doubles in weight while typing goes here
                }
            }
            PartKind::Select => canvas.invert(rect),
            PartKind::Dim => canvas.dim(rect),
            PartKind::Focus => canvas.ants(rect, part.phase),
            PartKind::Slider => Self::slider(part, canvas),
            PartKind::Progress => {
                canvas.fill(rect, WHITE);
                canvas.frame(rect, BLACK);
                let inner = rect.inset(1);
                if part.is(state::BUSY) {
                    // slanted stripes, four pixels on and four off, sliding to the right
                    for y in inner.y..inner.y + inner.h {
                        for x in inner.x..inner.x + inner.w {
                            if (x + y - part.phase).rem_euclid(8) < 4 {
                                canvas.set(x, y, BLACK);
                            }
                        }
                    }
                } else {
                    let done = (inner.w as i64 * part.a as i64 / part.b.max(1) as i64) as i32;
                    canvas.fill(Rect::new(inner.x, inner.y, done, inner.h), BLACK);
                }
            }
            PartKind::ScrollBar => Self::scroll_bar(part, canvas),
            PartKind::ScrollCorner => canvas.frame(rect, BLACK),
            PartKind::Tab => {
                let front = part.is(state::FRONT);
                canvas.round_frame(Rect::new(rect.x, rect.y + if front { 0 } else { 2 }, rect.w, rect.h + 6), 4, 1, BLACK, WHITE);
                p.font.draw(canvas, part.text, rect.x + TAB_PAD, rect.y + (rect.h - line) / 2 + if front { 0 } else { 1 }, BLACK);
                if front {
                    // open at the bottom, so that the tab and the pane are one shape
                    let pane = rect.y + rect.h - 1;
                    canvas.fill(Rect::new(rect.x + 1, pane, rect.w - 2, 7), WHITE);
                    canvas.fill(Rect::new(rect.x, pane + 1, 1, 6), WHITE);
                    canvas.fill(Rect::new(rect.x + rect.w - 1, pane + 1, 1, 6), WHITE);
                    if focused {
                        canvas.ants(Rect::new(rect.x + 4, rect.y + 3, rect.w - 8, rect.h - 6), part.phase);
                    }
                }
            }
            PartKind::TabPane => {
                canvas.fill(rect, WHITE);
                canvas.frame(rect, BLACK);
            }
            PartKind::Popup => {
                if focused {
                    canvas.ants(rect.inset(-Self::RING_GAP), part.phase);
                }
                // a white box with a hard shadow, like the menu it will become
                let face = Rect::new(rect.x, rect.y, rect.w - 1, rect.h - 1);
                canvas.fill(Rect::new(rect.x + 2, rect.y + rect.h - 1, rect.w - 2, 1), BLACK);
                canvas.fill(Rect::new(rect.x + rect.w - 1, rect.y + 2, 1, rect.h - 2), BLACK);
                canvas.fill(face, WHITE);
                canvas.frame(face, BLACK);
                // the text sits where its menu item will be when the list is up
                p.font.draw(canvas, part.text, face.x + 1 + MENU_ITEM_LEFT, face.y + 1 + (m.menu_item_height - line) / 2, BLACK);
                // a downward arrowhead: there is more where this came from
                let (cx, cy) = (face.x + face.w - 12, face.y + face.h / 2 - 2);
                for d in 0..5 {
                    canvas.fill(Rect::new(cx - 4 + d, cy + d, 9 - 2 * d, 1), BLACK);
                }
                if held {
                    canvas.invert(face.inset(1));
                }
            }
            PartKind::MenuBar => {
                canvas.fill(Rect::new(rect.x, rect.y, rect.w, rect.h - 1), WHITE);
                canvas.fill(Rect::new(rect.x, rect.y + rect.h - 1, rect.w, 1), BLACK);
            }
            PartKind::MenuTitle => {
                p.font.draw(canvas, part.text, rect.x + MENU_TITLE_PAD, rect.y + (rect.h - line) / 2, BLACK);
                if !part.is(state::ENABLED) {
                    canvas.dim(rect);
                } else if held {
                    canvas.invert(rect);
                }
            }
            PartKind::MenuFrame => {
                let (w, h) = (rect.w - 1, rect.h - 1);
                canvas.frame(Rect::new(rect.x, rect.y, w, h), BLACK);
                // a hard one-pixel shadow down the right and along the bottom
                canvas.fill(Rect::new(rect.x + w, rect.y, 1, h + 1), BLACK);
                canvas.fill(Rect::new(rect.x, rect.y + h, w + 1, 1), BLACK);
                canvas.fill(Rect::new(rect.x + w, rect.y, 1, 2), TRANSPARENT);
                canvas.fill(Rect::new(rect.x, rect.y + h, 2, 1), TRANSPARENT);
            }
            PartKind::MenuSeparator => {
                for x in (rect.x..rect.x + rect.w).filter(|x| x % 2 == 0) {
                    canvas.set(x, rect.y + rect.h / 2, BLACK);
                }
            }
            PartKind::ViewFrame => {
                canvas.fill(rect, WHITE);
                canvas.frame(rect, BLACK);
            }
            PartKind::Header => canvas.fill(Rect::new(rect.x, rect.y + rect.h - 1, rect.w, 1), BLACK),
            PartKind::HeaderCell => {}
            PartKind::MenuCheck => p.font.draw(canvas, "\u{2713}", rect.x + 3, rect.y + (rect.h - line) / 2, part.ink),
            PartKind::MenuArrow => {
                // a solid arrow pointing at where the submenu opens
                let (tip, mid) = (rect.x + rect.w - 8, rect.y + rect.h / 2);
                for d in 0..4 {
                    canvas.fill(Rect::new(tip - d, mid - d, 1, 2 * d + 1), part.ink);
                }
            }
            PartKind::SortArrow => {
                // a small arrowhead: up for smallest first
                for d in 0..4 {
                    let dy = if part.is(state::CHECKED) { d - 2 } else { 1 - d };
                    canvas.fill(Rect::new(part.a - d, part.b + dy, 2 * d + 1, 1), part.ink);
                }
            }
            PartKind::TreeFold => {
                // a solid triangle: pointing right when folded, down when open
                let (cx, cy) = (part.a, part.b);
                for d in 0..5 {
                    let r = if part.is(state::CHECKED) { Rect::new(cx - 4 + d, cy - 2 + d, 9 - 2 * d, 1) } else { Rect::new(cx - 2 + d, cy - 4 + d, 1, 9 - 2 * d) };
                    canvas.fill(r, part.ink);
                }
            }
            PartKind::Caret => canvas.fill(rect, part.ink),
            PartKind::Frame => {
                // a plain frame for a style that asks for one and draws none
                canvas.fill(rect, WHITE);
                canvas.frame(rect, BLACK);
                canvas.fill(Rect::new(rect.x, rect.y + part.a - 1, rect.w, 1), BLACK);
                let tw = p.font.width(part.text);
                p.font.draw(canvas, part.text, rect.x + (rect.w - tw) / 2, rect.y + (part.a - line) / 2, BLACK);
            }
            PartKind::FrameButton => {
                canvas.fill(rect, WHITE);
                canvas.frame(rect, BLACK);
                if held {
                    canvas.invert(rect.inset(1));
                }
            }
        }
        true
    }
}

/// The style in use, with what it was last asked for its sizes and colours.
#[derive(Clone)]
pub(crate) struct Look {
    pub style: std::sync::Arc<dyn Style>,
    pub m: Metrics,
    pub pal: Palette,
    pub wants: u32,
}

impl Look {
    /// The font for the title of a window's own frame, if the style has
    /// one for it apart from the interface's.
    pub fn frame_font(&self) -> Option<Font> {
        self.style.frame_typeface().map(|wanted| Font::smooth(Face::Cicero, &wanted))
    }

    /// A font for one of the families, as the style would have it.
    pub fn font(&self, face: Face) -> Font {
        match self.style.typeface(face) {
            Some(wanted) => Font::smooth(face, &wanted),
            None => Font::of(face),
        }
    }
}

impl Look {
    pub fn of(style: std::sync::Arc<dyn Style>) -> Look {
        let (m, pal, wants) = (style.metrics(), style.palette(), style.wants());
        Look { style, m, pal, wants }
    }
}

impl Default for Look {
    fn default() -> Look {
        Look::of(std::sync::Arc::new(EightyFour))
    }
}
