// SPDX-License-Identifier: GPL-3.0-or-later
//! The scroll view: a framed window onto a widget that may be larger than
//! the space given to it.
//!
//! It always carries a scroll bar down its right-hand side, as scrolling
//! panes on the original did, and grows one along the bottom only when the
//! content is too wide. The content is laid out at its full size and simply
//! placed further up and to the left as the view scrolls, so everything
//! else (hit-testing, focus, the caret's position for input methods) goes on
//! working in window coordinates; only drawing and hit-testing need to know
//! the content is seen through an opening.

use super::scroll::Bar;
#[cfg(test)]
use super::scroll::BAR;
use super::*;

/// How far an arrow, or a line of the wheel, moves the content, in pixels.
const STEP: i32 = 16;

/// Where everything in a scroll view goes.
struct Layout {
    /// The opening the content shows through.
    port: Rect,
    /// The bar down the side, and the one along the bottom if there is one.
    vbar: Rect,
    hbar: Option<Rect>,
    /// The size the content is laid out at: never smaller than the opening.
    content: (i32, i32),
    /// The furthest it can be scrolled each way.
    max: (i32, i32),
}

impl Ui {
    pub fn scroll_view_new(&mut self) -> Handle {
        self.add(Kind::ScrollView { content: None, width: 200, height: 120, scroll: (0, 0) })
    }

    /// Put a widget in the view. It is laid out at the size it asks for.
    pub fn scroll_view_set_content(&mut self, view: Handle, child: Handle) -> Result<()> {
        if !matches!(self.node(view)?.kind, Kind::ScrollView { .. }) {
            return Err(Error::WrongType);
        }
        self.adoptable(view, child)?;
        let old = match &mut self.node_mut(view)?.kind {
            Kind::ScrollView { content, scroll, .. } => {
                *scroll = (0, 0);
                content.replace(child)
            }
            _ => unreachable!(),
        };
        if let Some(old) = old.and_then(|o| self.nodes.get_mut(o)) {
            old.parent = None;
        }
        self.node_mut(child)?.parent = Some(view);
        Ok(())
    }

    /// Set the size the view asks for, frame and bars included.
    pub fn scroll_view_set_size(&mut self, h: Handle, w: Option<i32>, hgt: Option<i32>) -> Result<()> {
        if w.into_iter().chain(hgt).any(|v| !(3 * self.look.m.scroll_bar..=16384).contains(&v)) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::ScrollView { width, height, .. } => {
                *width = w.unwrap_or(*width);
                *height = hgt.unwrap_or(*height);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// How far the content has been scrolled: (left, up), in pixels.
    pub fn scroll_view_offset(&mut self, h: Handle) -> Result<(i32, i32)> {
        if !matches!(self.node(h)?.kind, Kind::ScrollView { .. }) {
            return Err(Error::WrongType);
        }
        Ok(self.clamp_view(h))
    }

    /// Scroll from the program. Positions past the end are brought back.
    pub fn scroll_view_set_offset(&mut self, h: Handle, x: Option<i32>, y: Option<i32>) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::ScrollView { scroll, .. } => {
                scroll.0 = x.unwrap_or(scroll.0);
                scroll.1 = y.unwrap_or(scroll.1);
            }
            _ => return Err(Error::WrongType),
        }
        self.clamp_view(h);
        Ok(())
    }

    fn view_layout(&mut self, h: Handle) -> Option<Layout> {
        let node = self.nodes.get(h)?;
        let Kind::ScrollView { content, .. } = &node.kind else { return None };
        let (rect, content) = (node.rect, *content);
        let wanted = content.map_or((0, 0), |c| self.measure(c));
        // the frame, less the bar down the right, which shares its edge
        let across = (rect.w - 1 - self.look.m.scroll_bar).max(0);
        let hbar = wanted.0 > across;
        let down = (rect.h - if hbar { 1 + self.look.m.scroll_bar } else { 2 }).max(0);
        let port = Rect::new(rect.x + 1, rect.y + 1, across, down);
        let size = (wanted.0.max(port.w), wanted.1.max(port.h));
        Some(Layout {
            port,
            // with a bar along the bottom, the side bar stops short at the corner box
            vbar: Rect::new(rect.x + rect.w - self.look.m.scroll_bar, rect.y, self.look.m.scroll_bar, rect.h - if hbar { self.look.m.scroll_bar - 1 } else { 0 }),
            hbar: hbar.then(|| Rect::new(rect.x, rect.y + rect.h - self.look.m.scroll_bar, rect.w - self.look.m.scroll_bar + 1, self.look.m.scroll_bar)),
            content: size,
            max: (size.0 - port.w, size.1 - port.h),
        })
    }

    /// Bring the scroll position within what the content allows.
    fn clamp_view(&mut self, h: Handle) -> (i32, i32) {
        let Some(max) = self.view_layout(h).map(|l| l.max) else { return (0, 0) };
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::ScrollView { scroll, .. }) => {
                *scroll = (scroll.0.clamp(0, max.0), scroll.1.clamp(0, max.1));
                *scroll
            }
            _ => (0, 0),
        }
    }

    /// The opening of a scroll view, in window coordinates.
    pub(super) fn view_port(&self, h: Handle) -> Option<Rect> {
        // The same sums as `view_layout`, for callers that cannot measure.
        // Whether there is a bar along the bottom was settled at the last
        // arrangement: the content's rectangle is wider than the opening.
        let node = self.nodes.get(h)?;
        let Kind::ScrollView { content, .. } = &node.kind else { return None };
        let rect = node.rect;
        let across = (rect.w - 1 - self.look.m.scroll_bar).max(0);
        let hbar = content.and_then(|c| self.nodes.get(c)).is_some_and(|c| c.rect.w > across);
        Some(Rect::new(rect.x + 1, rect.y + 1, across, (rect.h - if hbar { 1 + self.look.m.scroll_bar } else { 2 }).max(0)))
    }

    /// One of a scroll view's bars, for the shared scroll bar code.
    pub(super) fn view_bar(&mut self, h: Handle, horizontal: bool, enabled: bool) -> Option<Bar> {
        let layout = self.view_layout(h)?;
        let scroll = self.clamp_view(h);
        Some(if horizontal {
            Bar { rect: layout.hbar?, vertical: false, value: scroll.0, max: layout.max.0, page: layout.port.w, step: STEP, enabled, together: self.look.m.scroll_arrows == 1 }
        } else {
            Bar { rect: layout.vbar, vertical: true, value: scroll.1, max: layout.max.1, page: layout.port.h, step: STEP, enabled, together: self.look.m.scroll_arrows == 1 }
        })
    }

    /// Lay out a scroll view's content: at its full size, moved up and left
    /// by however far the view has been scrolled.
    pub(super) fn arrange_view(&mut self, h: Handle) {
        let Some(layout) = self.view_layout(h) else { return };
        let scroll = self.clamp_view(h);
        if let Some(Kind::ScrollView { content: Some(c), .. }) = self.nodes.get(h).map(|n| &n.kind) {
            let c = *c;
            self.arrange(c, Rect::new(layout.port.x - scroll.0, layout.port.y - scroll.1, layout.content.0, layout.content.1));
        }
    }

    pub(super) fn paint_view(&mut self, h: Handle, rect: Rect, enabled: bool, ctx: Paint, canvas: &mut Canvas) {
        let Some(layout) = self.view_layout(h) else { return };
        self.part(canvas, Part::new(PartKind::ViewFrame, rect));
        // The content is drawn on a sheet of its own and only the part that
        // falls in the opening is copied across: that is the clipping.
        let mut sheet = Canvas::filled(canvas.width, canvas.height, self.ground);
        for c in self.children(h) {
            self.paint(c, &mut sheet, enabled, ctx);
        }
        canvas.blit_part(&sheet, layout.port);
        self.bar_draw(h, false, canvas);
        if let Some(hbar) = layout.hbar {
            self.bar_draw(h, true, canvas);
            // the box in the corner where the two bars meet
            self.part(canvas, Part::new(PartKind::ScrollCorner, Rect::new(hbar.x + hbar.w - 1, hbar.y, self.look.m.scroll_bar, self.look.m.scroll_bar)));
        }
    }

    /// The innermost scroll view whose opening is under a point.
    pub(super) fn view_at(&self, window: Handle, x: i32, y: i32) -> Option<Handle> {
        let mut found = None;
        let mut pending = self.children(window);
        while let Some(h) = pending.pop() {
            let Some(node) = self.nodes.get(h).filter(|n| n.enabled) else { continue };
            match &node.kind {
                Kind::ScrollView { .. } => {
                    if node.rect.contains(x, y) {
                        found = Some(h);
                        // only what shows through the opening can be under the pointer
                        if self.view_port(h).is_some_and(|p| p.contains(x, y)) {
                            pending = self.children(h);
                        } else {
                            pending.clear();
                        }
                    }
                }
                _ => pending.extend(self.children(h)),
            }
        }
        found
    }

    /// Scroll every view a widget is inside so that the widget shows,
    /// moving each as little as will do.
    pub(super) fn reveal_now(&mut self, window: Handle, target: Handle) {
        let Some(size) = self.nodes.get(window).map(|n| n.rect) else { return };
        // innermost first; each move shifts everything inside, so the
        // window is laid out again before the next view is looked at
        let mut views = Vec::new();
        let mut up = self.nodes.get(target).and_then(|n| n.parent);
        while let Some(at) = up {
            let Some(node) = self.nodes.get(at) else { break };
            if matches!(node.kind, Kind::ScrollView { .. }) {
                views.push(at);
            }
            up = node.parent;
        }
        for view in views {
            let (Some(want), Some(port)) = (self.nodes.get(target).map(|n| n.rect.inset(-4)), self.view_port(view)) else { continue };
            // how far the widget sticks out of the opening on each side
            let shift = |start: i32, len: i32, port_start: i32, port_len: i32| {
                if start < port_start || len > port_len {
                    start - port_start
                } else {
                    (start + len - (port_start + port_len)).max(0)
                }
            };
            let (dx, dy) = (shift(want.x, want.w, port.x, port.w), shift(want.y, want.h, port.y, port.h));
            if (dx, dy) != (0, 0) {
                if let Some(Kind::ScrollView { scroll, .. }) = self.nodes.get_mut(view).map(|n| &mut n.kind) {
                    *scroll = (scroll.0 + dx, scroll.1 + dy);
                }
                self.arrange(window, size);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };

    /// A window holding a scroll view 160 by 100 at (10, 10) over a column
    /// of eight checkboxes, with a button below the view.
    fn fixture() -> (Ui, Handle, Handle, Vec<Handle>, Handle) {
        let mut ui = Ui::natural();
        let win = ui.window_new("View", 200, 180).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let view = ui.scroll_view_new();
        ui.scroll_view_set_size(view, Some(160), Some(100)).unwrap();
        let list = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(list, 6).unwrap();
        let boxes: Vec<Handle> = (1..=8).map(|n| ui.checkbox_new(&format!("Option {n}"))).collect();
        for b in &boxes {
            ui.stack_add(list, *b).unwrap();
        }
        ui.scroll_view_set_content(view, list).unwrap();
        let ok = ui.button_new("OK");
        ui.stack_add(col, view).unwrap();
        ui.stack_add(col, ok).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        (ui, win, view, boxes, ok)
    }

    fn rect(ui: &Ui, h: Handle) -> Rect {
        ui.nodes.get(h).unwrap().rect
    }

    fn toggled(events: Vec<Emitted>) -> Vec<Handle> {
        events.into_iter().filter(|e| e.event == "Checkbox.toggled").map(|e| e.target).collect()
    }

    fn click(ui: &mut Ui, win: Handle, x: i32, y: i32) -> Vec<Emitted> {
        let mut ev = ui.pointer(win, x, y, PointerKind::Press).unwrap();
        ev.extend(ui.pointer(win, x, y, PointerKind::Release).unwrap());
        ev
    }

    #[test]
    fn content_is_clipped_to_the_opening() {
        let (mut ui, win, view, boxes, ok) = fixture();
        // eight rows of content do not fit in a view 100 high
        assert_eq!(ui.scroll_view_offset(view).unwrap(), (0, 0));
        assert!(rect(&ui, boxes[7]).y > rect(&ui, view).y + 100);
        // yet nothing is drawn below the view but the button
        let canvas = ui.render(win).unwrap();
        let (below, button) = (rect(&ui, view).y + 100, rect(&ui, ok));
        assert!((below..button.y).all(|y| (0..canvas.width).all(|x| canvas.get(x, y) == WHITE)));
        // and a checkbox that is out of sight cannot be clicked where it would have been
        let hidden = rect(&ui, boxes[7]);
        assert!(toggled(click(&mut ui, win, hidden.x + 4, hidden.y + 4)).is_empty());
        // one that shows can
        let shown = rect(&ui, boxes[1]);
        assert_eq!(toggled(click(&mut ui, win, shown.x + 4, shown.y + 4)), [boxes[1]]);
        // content narrower than the view needs no bar along the bottom
        assert!(ui.bar_of(view, true).is_none() && ui.bar_of(view, false).is_some());
    }

    #[test]
    fn the_bar_and_the_wheel_scroll_the_content() {
        let (mut ui, win, view, boxes, _) = fixture();
        let before = rect(&ui, boxes[0]).y;
        let bar = ui.bar_of(view, false).unwrap();
        // the bar runs down the right-hand side of the frame
        assert_eq!((bar.rect.x + bar.rect.w, bar.rect.h), (rect(&ui, view).x + 160, 100));
        click(&mut ui, win, bar.rect.x + 8, bar.rect.y + bar.rect.h - 5);
        ui.render(win).unwrap();
        assert_eq!(ui.scroll_view_offset(view).unwrap(), (0, STEP));
        assert_eq!(rect(&ui, boxes[0]).y, before - STEP);
        // the wheel works anywhere in the view, over a checkbox or between them
        ui.wheel(win, 40, 60, 2).unwrap();
        assert_eq!(ui.scroll_view_offset(view).unwrap().1, 3 * STEP);
        ui.wheel(win, 40, 60, 100).unwrap();
        let end = ui.scroll_view_offset(view).unwrap().1;
        assert_eq!(end, ui.bar_of(view, false).unwrap().max); // it stops at the end
        ui.render(win).unwrap();
        // now the last checkbox shows and can be clicked
        let last = rect(&ui, boxes[7]);
        assert_eq!(toggled(click(&mut ui, win, last.x + 4, last.y + 4)), [boxes[7]]);
        // the program can scroll it too, and cannot overshoot
        ui.scroll_view_set_offset(view, None, Some(-50)).unwrap();
        assert_eq!(ui.scroll_view_offset(view).unwrap(), (0, 0));
        ui.scroll_view_set_offset(view, Some(999), Some(999)).unwrap();
        assert_eq!(ui.scroll_view_offset(view).unwrap(), (0, end));
    }

    #[test]
    fn the_keyboard_brings_what_it_reaches_into_view() {
        let (mut ui, win, view, boxes, ok) = fixture();
        ui.focus(boxes[0]).unwrap();
        ui.render(win).unwrap();
        for _ in 0..7 {
            ui.key(win, Key::Tab, NONE).unwrap();
        }
        assert_eq!(ui.live_focus(win), Some(boxes[7]));
        ui.render(win).unwrap();
        // the view scrolled just far enough for the last checkbox to show
        let (port, last) = (ui.view_port(view).unwrap(), rect(&ui, boxes[7]));
        assert!(last.y >= port.y && last.y + last.h <= port.y + port.h, "{last:?} in {port:?}");
        assert!(ui.scroll_view_offset(view).unwrap().1 > 0);
        // Space reaches it, as it would anywhere
        assert_eq!(toggled(ui.text(win, " ").unwrap()), [boxes[7]]);
        // Tab goes on out of the view to the button, and round again to the top
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(ok));
        ui.key(win, Key::Tab, NONE).unwrap();
        ui.render(win).unwrap();
        // back near the top: as far as shows the first checkbox, and no further
        let (port, first) = (ui.view_port(view).unwrap(), rect(&ui, boxes[0]));
        assert!(first.y >= port.y && ui.scroll_view_offset(view).unwrap().1 < 6);
    }

    #[test]
    fn wide_content_gets_a_bar_along_the_bottom() {
        let (mut ui, win, view, _, _) = fixture();
        let wide = ui.label_new("A line of text a good deal wider than the view it is shown in.");
        let old = ui.children(view)[0];
        ui.scroll_view_set_content(view, wide).unwrap();
        ui.destroy(old).unwrap();
        ui.render(win).unwrap();
        let (h, v) = (ui.bar_of(view, true).unwrap(), ui.bar_of(view, false).unwrap());
        // the two bars meet at a corner box and do not overlap
        assert_eq!(h.rect.y + BAR, rect(&ui, view).y + 100);
        assert!(v.rect.y + v.rect.h <= h.rect.y + 1 && h.rect.x + h.rect.w <= v.rect.x + 1);
        // short content: the side bar is there but dead
        assert_eq!((v.max, v.thumb()), (0, None));
        // the bottom bar's right arrow moves the text left
        let before = rect(&ui, wide).x;
        click(&mut ui, win, h.rect.x + h.rect.w - 5, h.rect.y + 8);
        ui.render(win).unwrap();
        assert_eq!((rect(&ui, wide).x, ui.scroll_view_offset(view).unwrap()), (before - STEP, (STEP, 0)));
        // with only a bar along the bottom to work, the wheel works that
        let end = ui.bar_of(view, true).unwrap().max;
        ui.wheel(win, 40, 40, 100).unwrap();
        assert_eq!(ui.scroll_view_offset(view).unwrap(), (STEP, 0)); // the side bar had first claim, and is dead
        let _ = end;
    }

    #[test]
    fn scroll_view_rules() {
        let (mut ui, win, view, boxes, ok) = fixture();
        // a widget has one parent, and a view is not its own content
        assert_eq!(ui.scroll_view_set_content(view, ok), Err(Error::InvalidState));
        let loose = ui.scroll_view_new();
        assert_eq!(ui.scroll_view_set_content(loose, loose), Err(Error::InvalidArgument));
        assert_eq!(ui.scroll_view_set_content(ok, view), Err(Error::WrongType));
        assert_eq!(ui.scroll_view_set_size(view, Some(10), None), Err(Error::InvalidArgument));
        // disabling the view disables what is in it
        ui.set_enabled(view, false).unwrap();
        let shown = rect(&ui, boxes[0]);
        assert!(toggled(click(&mut ui, win, shown.x + 4, shown.y + 4)).is_empty());
        ui.set_enabled(view, true).unwrap();
        // assistive technology sees through to the content
        let tree = ui.accessible(win).unwrap();
        assert!(tree.nodes.iter().any(|n| n.handle == boxes[7]));
        // destroying the view takes its content with it
        ui.destroy(view).unwrap();
        assert_eq!(ui.class_of(boxes[0]), Err(Error::InvalidHandle));
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }
}
