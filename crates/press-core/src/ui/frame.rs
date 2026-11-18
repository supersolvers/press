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
        if own { (m.frame_border, m.frame_title, m.frame_border, m.frame_border) } else { (0, 0, 0, 0) }
    }

    /// The size of a window with its frame.
    fn frame_outer(&self, window: Handle) -> Option<Rect> {
        let (l, t, r, b) = self.frame_insets(window);
        match self.nodes.get(window).map(|n| &n.kind) {
            Some(Kind::Window { width, height, .. }) => Some(Rect::new(0, 0, width + l + r, height + t + b)),
            _ => None,
        }
    }

    /// Where the buttons of a window's title bar are, measured from the
    /// corner of its frame: those the style puts at the left, from the
    /// edge inwards, and then those at the right.
    pub fn frame_buttons(&self, window: Handle) -> Vec<(u8, Rect)> {
        let m = self.look.m;
        let Some(outer) = self.frame_outer(window).filter(|_| self.frame_insets(window).1 > 0) else { return Vec::new() };
        let resizable = matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. }));
        let top = (m.frame_title - m.frame_button_height) / 2 + m.frame_button_rise;
        let mut out = Vec::new();
        for (list, from_left) in [(m.frame_left, true), (m.frame_right, false)] {
            let mut at = m.frame_button_inset;
            for place in 0..4 {
                let which = ((list >> (4 * place)) & 0xf) as u8;
                // a window that cannot change size has nothing to zoom to
                if !(CLOSE..=ZOOM).contains(&which) || (which == ZOOM && !resizable) {
                    continue;
                }
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
        let (active, zoomed) = (self.is_active(window), self.zoomed.contains(&window));
        let mut canvas = Canvas::filled(outer.w, outer.h, crate::canvas::TRANSPARENT);
        self.painting = None;
        let frame = Part::new(PartKind::Frame, outer).text(&title).numbers(t, l, 0);
        self.part(&mut canvas, frame.when(state::ENABLED, active).when(state::CHECKED, zoomed));
        for (which, rect) in self.frame_buttons(window) {
            let over = |held: Option<(Handle, u8)>| held == Some((window, which));
            let part = Part::new(PartKind::FrameButton, rect).numbers(which as i32, 0, 0);
            let part = part.when(state::ENABLED, active).when(state::CHECKED, zoomed);
            // held shows only while the pointer is still on the button
            self.part(&mut canvas, part.when(state::HOT, over(self.frame_hot)).when(state::HELD, over(self.frame_held) && over(self.frame_hot)));
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
                        _ => FrameAct::Zoom,
                    };
                }
            }
            return FrameAct::Nothing;
        }
        let content = Rect::new(l, t, outer.w - l - r, outer.h - t - b);
        // what began in the content stays the content's, wherever it goes
        if content.contains(x, y) || self.pressed.is_some() || kind == PointerKind::Leave || (kind != PointerKind::Press && !outer.contains(x, y)) {
            return FrameAct::Inside(x - l, y - t);
        }
        if kind != PointerKind::Press {
            return FrameAct::Nothing;
        }
        if over.is_some() {
            self.frame_held = over;
            self.dirty = true;
            return FrameAct::Nothing;
        }
        let resizable = matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. })) && !self.zoomed.contains(&window);
        if resizable {
            // an edge is taken hold of within a few pixels of it, and a
            // corner from a little way along either side
            let (grab, corner) = (l.max(4), 14);
            let (left, right) = (x < grab, x >= outer.w - grab);
            let (top, bottom) = (y < 3, y >= outer.h - grab);
            if left || right || top || bottom {
                let near = |at: i32, len: i32| (at < corner, at >= len - corner);
                let ((near_left, near_right), (near_top, near_bottom)) = (near(x, outer.w), near(y, outer.h));
                return FrameAct::Resize {
                    left: left || ((top || bottom) && near_left),
                    right: right || ((top || bottom) && near_right),
                    top: top || ((left || right) && near_top),
                    bottom: bottom || ((left || right) && near_bottom),
                };
            }
        }
        if y < t {
            // twice on the title bar zooms; once takes hold of the window
            let twice = self.now.saturating_sub(self.frame_click.0) <= DOUBLE_CLICK_MS && (x - self.frame_click.1).abs() <= 4 && (y - self.frame_click.2).abs() <= 4;
            self.frame_click = if twice { (0, 0, 0) } else { (self.now, x, y) };
            return if twice && resizable_at_all(self, window) { FrameAct::Zoom } else { FrameAct::Move };
        }
        FrameAct::Nothing
    }
}

fn resizable_at_all(ui: &Ui, window: Handle) -> bool {
    matches!(ui.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { resizable: true, .. }))
}
