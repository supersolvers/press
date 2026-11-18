// SPDX-License-Identifier: GPL-3.0-or-later
//! A window's own frame: the title bar, the edges and the buttons in the
//! title bar, drawn by the style round the window's content.
//!
//! Whether a window has one is the style's to say (a style whose title bar
//! has no height leaves the frame to the desktop) and the display's, which
//! turns frames on only where it can show them. Everything else in the
//! engine goes on measuring from the corner of the content, as it always
//! has; only this file knows that there is anything outside it. A display
//! asks how far the frame reaches ([`Ui::frame_insets`]), has the two
//! drawn together ([`Ui::render_framed`]), and hands every pointer event
//! here first ([`Ui::frame_pointer`]), which either says where in the
//! content it fell or what the user is doing to the window.

use super::*;

/// The buttons a title bar may have, as a style numbers them.
pub const CLOSE: u8 = 1;
pub const MINIMIZE: u8 = 2;
pub const ZOOM: u8 = 3;
/// Roll the window up into its title bar, or let it down again.
pub const ROLL: u8 = 4;
/// Show the window's own menu; two clicks on it close the window.
pub const MENU: u8 = 5;

/// What a pointer event on a framed window comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameAct {
    /// It fell in the content, at this place there: it is the content's.
    Inside(i32, i32),
    /// It fell on the frame, and nothing is to be done.
    Nothing,
    /// The window is being dragged by its title bar.
    Move,
    /// An edge or a corner is being dragged: which sides move.
    Resize { left: bool, right: bool, top: bool, bottom: bool },
    Close,
    Minimize,
    /// Make it as large as it can be, or as it was before.
    Zoom,
}

/// The sides of a window a point on its frame would move, if dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Sides {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

impl Ui {
    /// Have windows draw their own frames, where the style has one. A
    /// display that can show such a window says so; otherwise windows are
    /// content alone and their frames are the desktop's.
    pub fn set_own_frames(&mut self, on: bool) {
        self.own_frames = on;
        self.dirty = true;
    }

    /// Whether the style would have the menu bar kept in the window even
    /// on a desktop that offers to show it elsewhere.
    pub fn menu_in_window(&self) -> bool {
        self.look.m.menu_in_window != 0
    }

    /// Tell the engine a window has been made as large as it can be, or put back.
    pub fn window_set_zoomed(&mut self, window: Handle, zoomed: bool) {
        let was = self.zoomed.contains(&window);
        if was != zoomed {
            self.zoomed.retain(|w| *w != window);
            if zoomed {
                self.zoomed.push(window);
            }
            self.dirty = true;
        }
    }

    /// How far a window's frame reaches past its content: left, top, right
    /// and bottom. All nothing for a window whose frame is the desktop's.
    pub fn frame_insets(&self, window: Handle) -> (i32, i32, i32, i32) {
        let m = &self.look.m;
        let own = self.own_frames && m.frame_title > 0 && matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { full_screen: false, .. }));
        let foot = if m.frame_foot > 0 { m.frame_foot } else { m.frame_border };
        if own { (m.frame_border, m.frame_title, m.frame_border, foot) } else { (0, 0, 0, 0) }
    }

    /// Whether a window is rolled up into its title bar.
    pub fn window_rolled(&self, window: Handle) -> bool {
        self.rolled.contains(&window)
    }

    fn roll(&mut self, window: Handle) {
        if self.rolled.contains(&window) {
            self.rolled.retain(|w| *w != window);
        } else {
            self.rolled.push(window);
        }
        self.dirty = true;
    }

    /// What was last chosen from a window's own menu, for the display to do.
    pub fn take_frame_ask(&mut self) -> Option<(Handle, FrameAct)> {
        self.frame_asked.take()
    }

    /// Something was chosen from a window's own menu, or asked by its
    /// button: what the engine can do itself it does, and the rest waits
    /// for the display.
    pub(super) fn frame_ask(&mut self, window: Handle, which: u8) {
        match which {
            ROLL => self.roll(window),
            CLOSE => self.frame_asked = Some((window, FrameAct::Close)),
            MINIMIZE => self.frame_asked = Some((window, FrameAct::Minimize)),
            ZOOM => self.frame_asked = Some((window, FrameAct::Zoom)),
            _ => {}
        }
    }

    /// Do something with the typeface of a window's title worn, where the
    /// style has one for it.
    fn in_frame_face<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let Some(font) = self.frame_font.take() else { return f(self) };
        let was = std::mem::replace(&mut self.font, font);
        let out = f(self);
        self.frame_font = Some(std::mem::replace(&mut self.font, was));
        out
    }

    /// Pop up the menu of a window's frame under its button.
    fn frame_menu_open(&mut self, window: Handle, x: i32, y: i32) {
        if let Some(old) = self.frame_menu.take() {
            let _ = self.destroy(old);
        }
        let menu = self.menu_new("");
        let resizable = matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. }));
        let zoomed = self.zoomed.contains(&window);
        let mut entries = vec![(if zoomed { "Restore" } else { "Maximize" }, ZOOM, resizable), ("Minimize", MINIMIZE, true), ("", 0, true), ("Close", CLOSE, true)];
        entries.retain(|e| e.2);
        for (label, which, _) in entries {
            let item = self.add(Kind::MenuItem { label: label.into(), checked: false, shortcut: None, separator: which == 0, submenu: None, action: (which != 0).then_some(menu::EditAction::Frame(which)) });
            let _ = self.menu_add(menu, item);
        }
        self.frame_menu = Some(menu);
        self.open_popup(window, menu, x, y, None);
    }

    /// The size of a window with its frame.
    fn frame_outer(&self, window: Handle) -> Option<Rect> {
        let (l, t, r, b) = self.frame_insets(window);
        match self.nodes.get(window).map(|n| &n.kind) {
            // rolled up, it is its title bar and the foot of its frame and no more
            Some(Kind::Window { width, .. }) if self.rolled.contains(&window) && t > 0 => Some(Rect::new(0, 0, width + l + r, t + b)),
            Some(Kind::Window { width, height, .. }) => Some(Rect::new(0, 0, width + l + r, height + t + b)),
            _ => None,
        }
    }

    /// The buttons a list of them names, the one nearest the side first:
    /// those a window that cannot change size has no use for left out.
    fn frame_listed(&self, window: Handle, list: i32) -> Vec<u8> {
        let resizable = matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. }));
        (0..4).map(|place| ((list >> (4 * place)) & 0xf) as u8).filter(|which| (CLOSE..=MENU).contains(which) && (*which != ZOOM || resizable)).collect()
    }

    /// How wide a window's title bar is: all of the window, or for a style
    /// whose title bar is a tab, as wide as its buttons and the title need.
    pub fn frame_bar_width(&mut self, window: Handle) -> i32 {
        let m = self.look.m;
        let Some(outer) = self.frame_outer(window) else { return 0 };
        if m.frame_tab <= 0 {
            return outer.w;
        }
        let title = match self.nodes.get(window).map(|n| &n.kind) {
            Some(Kind::Window { title, .. }) => title.clone(),
            _ => String::new(),
        };
        let room = |count: usize| if count == 0 { 0 } else { m.frame_button_inset + count as i32 * (m.frame_button_width + m.frame_button_gap) - m.frame_button_gap };
        let (left, right) = (room(self.frame_listed(window, m.frame_left).len()), room(self.frame_listed(window, m.frame_right).len()));
        let words = self.in_frame_face(|ui| ui.font.width(&title));
        let wide = left + m.frame_tab + words + m.frame_tab + right + if right > 0 { m.frame_button_inset } else { 0 };
        wide.clamp((left + right + 2 * m.frame_tab).min(outer.w), outer.w)
    }

    /// Where the buttons of a window's title bar are, measured from the
    /// corner of its frame: those the style puts at the left, from the
    /// edge inwards, and then those at the right.
    pub fn frame_buttons(&mut self, window: Handle) -> Vec<(u8, Rect)> {
        let m = self.look.m;
        let Some(outer) = self.frame_outer(window).filter(|_| self.frame_insets(window).1 > 0) else { return Vec::new() };
        // the buttons at the right stand at the right of the title bar, which may be a tab
        let outer = Rect::new(outer.x, outer.y, self.frame_bar_width(window), outer.h);
        let top = (m.frame_title - m.frame_button_height) / 2 + m.frame_button_rise;
        let mut out = Vec::new();
        for (list, from_left) in [(m.frame_left, true), (m.frame_right, false)] {
            let mut at = m.frame_button_inset;
            for which in self.frame_listed(window, list) {
                let x = if from_left { at } else { outer.w - at - m.frame_button_width };
                out.push((which, Rect::new(x, top, m.frame_button_width, m.frame_button_height)));
                at += m.frame_button_width + m.frame_button_gap;
            }
        }
        out
    }

    /// Draw a window with its frame round it. Outside the frame's shape
    /// the picture is see-through. A window with no frame of its own is
    /// drawn as [`Ui::render`] draws it.
    pub fn render_framed(&mut self, window: Handle) -> Result<Canvas> {
        let content = self.render(window)?;
        let (l, t, ..) = self.frame_insets(window);
        let Some(outer) = self.frame_outer(window).filter(|_| t > 0) else { return Ok(content) };
        let title = match self.nodes.get(window).map(|n| &n.kind) {
            Some(Kind::Window { title, .. }) => title.clone(),
            _ => String::new(),
        };
        let (active, zoomed, rolled) = (self.is_active(window), self.zoomed.contains(&window), self.rolled.contains(&window));
        let mut canvas = Canvas::filled(outer.w, outer.h, crate::canvas::TRANSPARENT);
        self.painting = None;
        let bar = self.frame_bar_width(window);
        let frame = Part::new(PartKind::Frame, outer).text(&title).numbers(t, l, bar);
        let frame = frame.when(state::ENABLED, active).when(state::CHECKED, zoomed).when(state::HELD, rolled);
        self.in_frame_face(|ui| ui.part(&mut canvas, frame));
        for (which, rect) in self.frame_buttons(window) {
            let over = |held: Option<(Handle, u8)>| held == Some((window, which));
            let part = Part::new(PartKind::FrameButton, rect).numbers(which as i32, 0, 0);
            let part = part.when(state::ENABLED, active).when(state::CHECKED, zoomed);
            // held shows only while the pointer is still on the button
            self.part(&mut canvas, part.when(state::HOT, over(self.frame_hot)).when(state::HELD, over(self.frame_held) && over(self.frame_hot)));
        }
        if rolled {
            return Ok(canvas);
        }
        // the content has no see-through pixels of its own
        for y in 0..content.height {
            for x in 0..content.width {
                canvas.set(l + x, t + y, content.pixels[(y * content.width + x) as usize]);
            }
        }
        Ok(canvas)
    }

    /// Take a pointer event on a window, measured from the corner of its
    /// frame, and say what it comes to.
    pub fn frame_pointer(&mut self, window: Handle, x: i32, y: i32, kind: PointerKind) -> FrameAct {
        let (l, t, r, b) = self.frame_insets(window);
        if t == 0 {
            return FrameAct::Inside(x, y);
        }
        let Some(outer) = self.frame_outer(window) else { return FrameAct::Nothing };
        let over = self.frame_buttons(window).into_iter().find(|(_, rect)| rect.contains(x, y)).map(|(which, _)| (window, which)).filter(|_| kind != PointerKind::Leave);
        if over != self.frame_hot {
            self.frame_hot = over;
            self.dirty = true;
        }
        // a button that is held has the pointer until it is let go
        if let Some(held) = self.frame_held.filter(|h| h.0 == window) {
            if kind == PointerKind::Release {
                self.frame_held = None;
                self.dirty = true;
                if over == Some(held) {
                    return match held.1 {
                        CLOSE => FrameAct::Close,
                        MINIMIZE => FrameAct::Minimize,
                        ROLL => {
                            self.roll(window);
                            FrameAct::Nothing
                        }
                        _ => FrameAct::Zoom,
                    };
                }
            }
            return FrameAct::Nothing;
        }
        let rolled = self.rolled.contains(&window);
        let content = Rect::new(l, t, outer.w - l - r, if rolled { 0 } else { outer.h - t - b });
        // beside a title bar that is a tab there is nothing of the window's
        let beside_tab = y < t - b.max(1) && x >= self.frame_bar_width(window);
        // what began in the content stays the content's, wherever it goes
        if content.contains(x, y) || self.pressed.is_some() || kind == PointerKind::Leave || (kind != PointerKind::Press && !outer.contains(x, y)) {
            return FrameAct::Inside(x - l, y - t);
        }
        if kind != PointerKind::Press || beside_tab {
            return FrameAct::Nothing;
        }
        let twice = self.now.saturating_sub(self.frame_click.0) <= DOUBLE_CLICK_MS && (x - self.frame_click.1).abs() <= 4 && (y - self.frame_click.2).abs() <= 4;
        if let Some((_, which)) = over {
            if which == MENU {
                // the window's own menu drops from its button; twice on it closes
                self.frame_click = if twice { (0, 0, 0) } else { (self.now, x, y) };
                if twice {
                    self.close_menu();
                    return FrameAct::Close;
                }
                self.frame_menu_open(window, -l, 0);
                return FrameAct::Nothing;
            }
            self.frame_held = over;
            self.dirty = true;
            return FrameAct::Nothing;
        }
        let sides = self.frame_sides(window, x, y);
        if sides != Sides::default() {
            return FrameAct::Resize { left: sides.left, right: sides.right, top: sides.top, bottom: sides.bottom };
        }
        if y < t {
            // twice on the title bar does what the style says; once takes hold of the window
            self.frame_click = if twice { (0, 0, 0) } else { (self.now, x, y) };
            if !twice {
                return FrameAct::Move;
            }
            return match self.look.m.frame_double {
                1 => {
                    self.roll(window);
                    FrameAct::Nothing
                }
                2 => FrameAct::Minimize,
                _ if resizable_at_all(self, window) => FrameAct::Zoom,
                _ => FrameAct::Move,
            };
        }
        FrameAct::Nothing
    }

    /// The sides of a window that a point on its own frame takes hold of:
    /// none, for a point that is not on an edge that can be dragged.
    fn frame_sides(&self, window: Handle, x: i32, y: i32) -> Sides {
        let (l, t, _, b) = self.frame_insets(window);
        let none = Sides::default();
        let Some(outer) = self.frame_outer(window).filter(|o| t > 0 && o.contains(x, y)) else { return none };
        let resizable = matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. }));
        if !resizable || self.zoomed.contains(&window) || self.rolled.contains(&window) {
            return none;
        }
        // an edge is taken hold of within a few pixels of it, and a
        // corner from a little way along either side
        let (grab, foot, corner) = (l.max(4), b.max(4), 14);
        let (left, right) = (x < grab, x >= outer.w - grab);
        let (top, bottom) = (y < 3, y >= outer.h - foot);
        if !(left || right || top || bottom) {
            return none;
        }
        let near = |at: i32, len: i32| (at < corner, at >= len - corner);
        let ((near_left, near_right), (near_top, near_bottom)) = (near(x, outer.w), near(y, outer.h));
        Sides {
            left: left || ((top || bottom) && near_left),
            right: right || ((top || bottom) && near_right),
            top: top || ((left || right) && near_top),
            bottom: bottom || ((left || right) && near_bottom),
        }
    }

    /// The pointer to show over a window's own frame, for a point measured
    /// from the corner of its content: arrows over an edge that can be dragged.
    pub(super) fn frame_cursor(&self, window: Handle, x: i32, y: i32) -> Option<crate::cursor::Cursor> {
        use crate::cursor::Cursor;
        let (l, t, ..) = self.frame_insets(window);
        if t == 0 || self.frame_held.is_some() {
            return None;
        }
        let sides = self.frame_sides(window, x + l, y + t);
        match (sides.left || sides.right, sides.top || sides.bottom) {
            (true, true) => Some(Cursor::ResizeCorner),
            (true, false) => Some(Cursor::ResizeColumn),
            (false, true) => Some(Cursor::ResizeRow),
            _ => None,
        }
    }
}

fn resizable_at_all(ui: &Ui, window: Handle) -> bool {
    matches!(ui.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. }))
}
