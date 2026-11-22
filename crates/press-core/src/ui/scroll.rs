// SPDX-License-Identifier: GPL-3.0-or-later
//! Scroll bars: the stand-alone widget, and the one built into a text area.
//!
//! The look is the original's: an arrow at each end, a grey track, and a
//! white box that slides. Two things are of today. The box is sized to show
//! how much of the whole is in view, unless the style would have it square
//! as the original's was, and holding an arrow or the track repeats.
//!
//! Both kinds are worked by the same code. Each is described as a [`Bar`]:
//! where it is and the numbers it stands for.

use super::*;

/// Thickness of a scroll bar, and the length of each arrow box.
#[cfg(test)]
pub(super) const BAR: i32 = 16;
const THUMB_MIN: i32 = 10;

// Holding the button repeats: first after a pause, then steadily.
pub(super) const REPEAT_DELAY_MS: u64 = 300;
pub(super) const REPEAT_MS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    /// The arrow at the top or left.
    Back,
    /// The arrow at the bottom or right.
    Forward,
    /// The track before the box, and after it.
    PageBack,
    PageForward,
    Thumb,
}

/// A scroll bar being held with the pointer.
pub(super) struct BarPress {
    /// The scroll bar widget, or the text area or scroll view whose bar it is.
    pub owner: Handle,
    /// Which of the owner's bars: the one along the bottom, or down the side.
    pub horizontal: bool,
    pub part: Part,
    /// For the box: how far along it the pointer took hold.
    pub grip: i32,
    /// Where the pointer is now.
    pub at: (i32, i32),
    /// When the held arrow or track next repeats.
    pub next: u64,
}

/// A scroll bar: where it is drawn and the range it stands for. `value`
/// runs from 0 to `max`; `page` is how much is in view at once.
#[derive(Debug, Clone, Copy)]
pub(super) struct Bar {
    pub rect: Rect,
    pub vertical: bool,
    pub value: i32,
    pub max: i32,
    pub page: i32,
    pub step: i32,
    pub enabled: bool,
    /// Both arrows are at the far end, one beside the other.
    pub together: bool,
    /// The box is a square, the bar's thickness each way, wherever it is,
    /// as on the original; otherwise it is as long as the part in view.
    pub square: bool,
}

impl Bar {
    /// How thick the bar is, which is also the length of each arrow box.
    fn thick(&self) -> i32 {
        if self.vertical { self.rect.w } else { self.rect.h }
    }

    /// Start and length along the bar's direction of travel.
    fn along(&self) -> (i32, i32) {
        if self.vertical { (self.rect.y, self.rect.h) } else { (self.rect.x, self.rect.w) }
    }

    /// Whether there is anything to scroll.
    fn live(&self) -> bool {
        self.enabled && self.max > 0
    }

    /// The stretch the box slides in. It overlaps the inner edge of each
    /// arrow box by a pixel so that their borders coincide at the ends.
    fn track(&self) -> (i32, i32) {
        let (start, len) = self.along();
        if self.together {
            return (start, (len - 2 * self.thick() + 1).max(0));
        }
        (start + self.thick() - 1, (len - 2 * (self.thick() - 1)).max(0))
    }

    /// Where the box is: start and length along the bar.
    pub(super) fn thumb(&self) -> Option<(i32, i32)> {
        let (start, len) = self.track();
        if !self.live() || len < THUMB_MIN {
            return None;
        }
        let whole = (self.max + self.page.max(1)) as i64;
        let size = if self.square { self.thick().min(len) } else { ((len as i64 * self.page.max(1) as i64 / whole) as i32).clamp(THUMB_MIN, len) };
        let travel = (len - size) as i64;
        Some((start + (travel * self.value.clamp(0, self.max) as i64 / self.max as i64) as i32, size))
    }

    /// The value that puts the start of the box at a position along the bar.
    fn value_at(&self, thumb_start: i32) -> i32 {
        let (start, len) = self.track();
        let Some((_, size)) = self.thumb() else { return self.value };
        let travel = (len - size).max(1) as i64;
        let offset = (thumb_start - start).clamp(0, len - size) as i64;
        // the inverse of `thumb`, which rounds positions down
        ((offset * self.max as i64 + travel - 1) / travel) as i32
    }

    fn part_at(&self, x: i32, y: i32) -> Option<Part> {
        if !self.rect.contains(x, y) {
            return None;
        }
        let (start, len) = self.along();
        let pos = if self.vertical { y } else { x };
        if pos >= start + len - self.thick() {
            return Some(Part::Forward);
        }
        if (self.together && pos >= start + len - 2 * self.thick()) || (!self.together && pos < start + self.thick()) {
            return Some(Part::Back);
        }
        let (thumb, size) = self.thumb()?;
        Some(if pos < thumb {
            Part::PageBack
        } else if pos >= thumb + size {
            Part::PageForward
        } else {
            Part::Thumb
        })
    }

    /// The value after one press of a part.
    fn stepped(&self, part: Part) -> i32 {
        let (step, page) = (self.step.max(1), (self.page - self.step.max(1)).max(self.step.max(1)));
        let to = match part {
            Part::Back => self.value - step,
            Part::Forward => self.value + step,
            Part::PageBack => self.value - page,
            Part::PageForward => self.value + page,
            Part::Thumb => self.value,
        };
        to.clamp(0, self.max.max(0))
    }

    /// The bar as a part for the style to draw. `held` is the arrow being
    /// pressed, if one is.
    /// A part of the bar as a style numbers it for the pointer being over
    /// it: 1 the first arrow, 2 the second, 3 the sliding box.
    fn piece(part: Part) -> i32 {
        match part {
            Part::Back => 1,
            Part::Forward => 2,
            Part::Thumb => 3,
            _ => 0,
        }
    }

    pub(super) fn as_part(&self, held: Option<Part>) -> crate::style::Part<'static> {
        let held = match held {
            Some(Part::Back) => 1,
            Some(Part::Forward) => 2,
            _ => 0,
        };
        let (thumb, size) = self.thumb().unwrap_or((-1, 0));
        crate::style::Part::new(PartKind::ScrollBar, self.rect).numbers(held, thumb, size).when(state::VERTICAL, self.vertical).when(state::ENABLED, self.live())
    }
}

impl Ui {
    // ------------------------------------------------------------ the widget

    pub fn scroll_bar_new(&mut self, orientation: Orientation) -> Handle {
        self.add(Kind::ScrollBar { orientation, value: 0, max: 100, page: 10, step: 1, length: 120 })
    }

    pub fn scroll_bar_value(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::ScrollBar { value, .. } => Ok(*value),
            _ => Err(Error::WrongType),
        }
    }

    /// Move the box from the program. Out-of-range values are brought into
    /// range. Nothing is emitted: this is not the user's doing.
    pub fn scroll_bar_set_value(&mut self, h: Handle, to: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::ScrollBar { value, max, .. } => {
                *value = to.clamp(0, *max);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn scroll_bar_max(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::ScrollBar { max, .. } => Ok(*max),
            _ => Err(Error::WrongType),
        }
    }

    /// Set the largest value. The value is brought down to it if need be.
    pub fn scroll_bar_set_max(&mut self, h: Handle, to: i32) -> Result<()> {
        if to < 0 {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::ScrollBar { value, max, .. } => {
                *max = to;
                *value = (*value).min(to);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Set one of the sizes that must be at least 1: how much is in view
    /// (`page`), how far an arrow moves (`step`), or the bar's length in pixels.
    fn scroll_bar_set(&mut self, h: Handle, to: i32, least: i32, pick: impl FnOnce(&mut Kind) -> Option<&mut i32>) -> Result<()> {
        if to < least || to > 1 << 24 {
            return Err(Error::InvalidArgument);
        }
        *pick(&mut self.node_mut(h)?.kind).ok_or(Error::WrongType)? = to;
        Ok(())
    }

    pub fn scroll_bar_set_page(&mut self, h: Handle, to: i32) -> Result<()> {
        self.scroll_bar_set(h, to, 1, |k| match k {
            Kind::ScrollBar { page, .. } => Some(page),
            _ => None,
        })
    }

    pub fn scroll_bar_set_step(&mut self, h: Handle, to: i32) -> Result<()> {
        self.scroll_bar_set(h, to, 1, |k| match k {
            Kind::ScrollBar { step, .. } => Some(step),
            _ => None,
        })
    }

    pub fn scroll_bar_set_length(&mut self, h: Handle, to: i32) -> Result<()> {
        self.scroll_bar_set(h, to, 2 * self.look.m.scroll_bar, |k| match k {
            Kind::ScrollBar { length, .. } => Some(length),
            _ => None,
        })
    }

    // ------------------------------------------------- bars of either kind

    /// Where a text area's scroll bar goes: down its right-hand side,
    /// sharing the area's frame.
    pub(super) fn area_bar_rect(rect: Rect, thick: i32) -> Rect {
        let frame = rect.inset(1);
        Rect::new(frame.x + frame.w - thick, frame.y, thick, frame.h)
    }

    /// The scroll bar a widget is or has, down its side or (`horizontal`)
    /// along its bottom. A scroll view may have both; the others have one.
    pub(super) fn bar_of(&mut self, h: Handle, horizontal: bool) -> Option<Bar> {
        let square = self.look.wants & crate::style::wants::SQUARE_SCROLL_BOX != 0;
        self.in_face(h, |ui| ui.bar_of_in(h, horizontal)).map(|bar| Bar { square, ..bar })
    }

    fn bar_of_in(&mut self, h: Handle, horizontal: bool) -> Option<Bar> {
        let node = self.nodes.get(h)?;
        let enabled = self.focusable_chain_enabled(h);
        match &node.kind {
            Kind::ScrollView { .. } => self.view_bar(h, horizontal, enabled),
            Kind::List { .. } if !horizontal => self.list_bar(h, enabled),
            Kind::List { .. } => self.list_bar_across(h, enabled),
            Kind::ScrollBar { orientation, .. } if (*orientation == Orientation::Horizontal) != horizontal => None,
            Kind::TextField { .. } if horizontal => None,
            Kind::ScrollBar { orientation, value, max, page, step, .. } => Some(Bar {
                // no thicker than a scroll bar is, whatever room it was given across
                rect: if *orientation == Orientation::Vertical {
                    Rect::new(node.rect.x, node.rect.y, node.rect.w.min(self.look.m.scroll_bar), node.rect.h)
                } else {
                    Rect::new(node.rect.x, node.rect.y, node.rect.w, node.rect.h.min(self.look.m.scroll_bar))
                },
                vertical: *orientation == Orientation::Vertical,
                value: *value,
                max: *max,
                page: *page,
                step: *step,
                enabled,
                square: false, together: self.look.m.scroll_arrows == 1,
            }),
            Kind::TextField { multiline: true, plain: true, .. } => None,
            Kind::RichText(_) if horizontal => None,
            Kind::RichText(_) => self.rich_bar(h, enabled),
            Kind::TextField { multiline: true, edit, scroll, .. } => {
                let (rect, text, scroll) = (node.rect, edit.text().to_string(), *scroll);
                let (_, inner) = self.shape_of(h)?;
                let rows = (inner.h / self.font.line_height()).max(1);
                let lines = self.wrap(&text, inner.w - 2, true).len() as i32;
                let max = (lines - rows).max(0);
                Some(Bar { rect: Self::area_bar_rect(rect, self.look.m.scroll_bar), vertical: true, value: scroll.clamp(0, max), max, page: rows, step: 1, enabled, square: false, together: self.look.m.scroll_arrows == 1 })
            }
            _ => None,
        }
    }

    /// Enabled all the way up the tree.
    fn focusable_chain_enabled(&self, h: Handle) -> bool {
        let mut up = Some(h);
        while let Some(at) = up {
            match self.nodes.get(at) {
                Some(n) if n.enabled => up = n.parent,
                _ => return false,
            }
        }
        true
    }

    /// Move a bar because the user worked it, and say so if it is a widget.
    pub(super) fn bar_move(&mut self, h: Handle, horizontal: bool, to: i32) -> Vec<Emitted> {
        let Some(bar) = self.bar_of(h, horizontal) else { return Vec::new() };
        let to = to.clamp(0, bar.max.max(0));
        if to == bar.value {
            return Vec::new();
        }
        self.dirty = true;
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::ScrollBar { value, .. }) => {
                *value = to;
                vec![Emitted { target: h, event: "ScrollBar.changed", args: vec![Value::I32(to)] }]
            }
            Some(Kind::TextField { scroll, follow, .. }) => {
                *scroll = to;
                *follow = false; // the view stays put until the caret next moves
                Vec::new()
            }
            Some(Kind::RichText(_)) => {
                self.rich_bar_moved(h, to);
                Vec::new()
            }
            Some(Kind::List { scroll, across, .. }) => {
                *if horizontal { across } else { scroll } = to;
                Vec::new()
            }
            Some(Kind::ScrollView { scroll, .. }) => {
                if horizontal {
                    scroll.0 = to;
                } else {
                    scroll.1 = to;
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// The part being held, for drawing it pressed: only while the pointer
    /// is still on it.
    pub(super) fn bar_held(&mut self, h: Handle, horizontal: bool) -> Option<Part> {
        let (part, at) = self.bar_press.as_ref().filter(|p| p.owner == h && p.horizontal == horizontal).map(|p| (p.part, p.at))?;
        let bar = self.bar_of(h, horizontal)?;
        (bar.part_at(at.0, at.1) == Some(part)).then_some(part)
    }

    /// Give scroll bars first refusal of a pointer event. `over` is the
    /// widget under the pointer.
    pub(super) fn bar_pointer(&mut self, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        if let Some((owner, horizontal)) = self.bar_press.as_ref().map(|p| (p.owner, p.horizontal)) {
            // a bar is being held: every event is its business until release
            let mut out = Vec::new();
            match kind {
                PointerKind::Release => self.bar_press = None,
                PointerKind::Move | PointerKind::Leave | PointerKind::Press => {
                    let Some(bar) = self.bar_of(owner, horizontal) else {
                        self.bar_press = None;
                        return Some(out);
                    };
                    if let Some(press) = &mut self.bar_press {
                        press.at = (x, y);
                        if press.part == Part::Thumb {
                            let to = bar.value_at(if bar.vertical { y } else { x } - press.grip);
                            out = self.bar_move(owner, horizontal, to);
                        }
                    }
                }
            }
            self.dirty = true;
            return Some(out);
        }
        if kind != PointerKind::Press {
            return None;
        }
        let owner = over?;
        // whichever of its bars the pointer is on
        let (horizontal, bar, part) = [false, true].into_iter().find_map(|horizontal| {
            let bar = self.bar_of(owner, horizontal)?;
            Some((horizontal, bar, bar.part_at(x, y)?))
        })?;
        if !bar.live() {
            return Some(Vec::new()); // nothing to scroll, but the click is the bar's
        }
        let grip = bar.thumb().map_or(0, |(start, _)| if bar.vertical { y } else { x } - start);
        self.bar_press = Some(BarPress { owner, horizontal, part, grip, at: (x, y), next: self.now + REPEAT_DELAY_MS });
        self.dirty = true;
        Some(self.bar_move(owner, horizontal, bar.stepped(part)))
    }

    /// Repeat a held arrow or track.
    pub(super) fn bar_tick(&mut self) -> Vec<Emitted> {
        let Some((owner, horizontal, part, at, next)) = self.bar_press.as_ref().map(|p| (p.owner, p.horizontal, p.part, p.at, p.next)) else {
            return Vec::new();
        };
        if part == Part::Thumb || self.now < next {
            return Vec::new();
        }
        let Some(bar) = self.bar_of(owner, horizontal) else {
            self.bar_press = None;
            return Vec::new();
        };
        if let Some(press) = &mut self.bar_press {
            press.next = self.now + REPEAT_MS;
        }
        // only while the pointer is still on the part: paging stops when the
        // box arrives under it, and sliding off an arrow pauses it
        if bar.part_at(at.0, at.1) != Some(part) {
            return Vec::new();
        }
        self.bar_move(owner, horizontal, bar.stepped(part))
    }

    pub(super) fn bar_deadline(&self) -> Option<u64> {
        self.bar_press.as_ref().filter(|p| p.part != Part::Thumb).map(|p| p.next)
    }

    /// Turn the wheel over something with a scroll bar. It works the bar
    /// down the side if there is one, otherwise the one along the bottom.
    pub(super) fn bar_wheel(&mut self, h: Handle, lines: i32) -> Vec<Emitted> {
        for horizontal in [false, true] {
            if let Some(bar) = self.bar_of(h, horizontal) {
                return self.bar_move(h, horizontal, bar.value + lines * bar.step.max(1));
            }
        }
        Vec::new()
    }

    /// Draw one of a widget's scroll bars.
    pub(super) fn bar_draw(&mut self, h: Handle, horizontal: bool, canvas: &mut Canvas) {
        let held = self.bar_held(h, horizontal);
        // whether its window is the one in front, for a style that empties the bars of the others
        let front = self.window_of(h).is_none_or(|w| self.is_active(w));
        if let Some(bar) = self.bar_of(h, horizontal) {
            let mut part = bar.as_part(held).when(state::FRONT, front);
            // the piece of it the pointer is over, for a style that lights it
            let piece = self.hover_piece - 2000 - i32::from(horizontal) * 10;
            if self.hover == Some(h) && (1..=3).contains(&piece) {
                part.d = piece;
            }
            self.part(canvas, part);
        }
    }

    /// The part of one of a widget's scroll bars a point is over, as a
    /// style numbers it; none if it has no such bar or the point is off it.
    pub(super) fn bar_part_at(&mut self, h: Handle, horizontal: bool, x: i32, y: i32) -> Option<i32> {
        let part = self.bar_of(h, horizontal)?.part_at(x, y)?;
        Some(Bar::piece(part)).filter(|p| *p > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(value: i32, max: i32, page: i32) -> Bar {
        Bar { rect: Rect::new(10, 20, BAR, 200), vertical: true, value, max, page, step: 1, enabled: true, square: false, together: false }
    }

    #[test]
    fn the_box_shows_position_and_proportion() {
        // a tenth in view: the box is about a tenth of the track
        let (start, size) = bar(0, 90, 10).thumb().unwrap();
        let (track, len) = bar(0, 90, 10).track();
        assert_eq!((start, size), (track, len / 10));
        // at the far end it sits against the far arrow
        let (start, size) = bar(90, 90, 10).thumb().unwrap();
        assert_eq!(start + size, track + len);
        // it never gets too small to grab
        assert_eq!(bar(0, 100_000, 1).thumb().unwrap().1, THUMB_MIN);
        // nothing to scroll: no box, and the bar is dead
        assert_eq!(bar(0, 0, 10).thumb(), None);
        assert_eq!(bar(0, 0, 10).part_at(15, 100), None);
        // dragging the box to a place and asking what value that is round-trips
        for value in [0, 1, 37, 89, 90] {
            let b = bar(value, 90, 10);
            assert_eq!(b.value_at(b.thumb().unwrap().0), value);
        }
        // a style may have it square, as the original's was, wherever it is and however much is in view
        let square = |value| Bar { square: true, ..bar(value, 90, 10) };
        let (track, len) = square(0).track();
        assert_eq!(square(0).thumb(), Some((track, BAR)));
        assert_eq!(square(90).thumb(), Some((track + len - BAR, BAR)));
        for value in [0, 1, 37, 89, 90] {
            assert_eq!(square(value).value_at(square(value).thumb().unwrap().0), value);
        }
    }

    #[test]
    fn each_part_is_where_it_looks() {
        let b = bar(45, 90, 10);
        let (thumb, size) = b.thumb().unwrap();
        assert_eq!(b.part_at(15, 21), Some(Part::Back));
        assert_eq!(b.part_at(15, 218), Some(Part::Forward));
        assert_eq!(b.part_at(15, thumb - 3), Some(Part::PageBack));
        assert_eq!(b.part_at(15, thumb + size + 3), Some(Part::PageForward));
        assert_eq!(b.part_at(15, thumb + 1), Some(Part::Thumb));
        assert_eq!(b.part_at(40, 100), None);
        // an arrow moves a step; the track moves a page less a step, for context
        assert_eq!((b.stepped(Part::Back), b.stepped(Part::Forward)), (44, 46));
        assert_eq!((b.stepped(Part::PageBack), b.stepped(Part::PageForward)), (36, 54));
        assert_eq!(bar(89, 90, 10).stepped(Part::PageForward), 90);
    }

    /// A window with a vertical scroll bar at (10, 10), 16 by 160.
    fn widget() -> (Ui, Handle, Handle) {
        let mut ui = Ui::natural();
        let win = ui.window_new("Scroll", 120, 200).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let bar = ui.scroll_bar_new(Orientation::Vertical);
        ui.scroll_bar_set_length(bar, 160).unwrap();
        ui.stack_add(col, bar).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        (ui, win, bar)
    }

    fn values(events: &[Emitted]) -> Vec<i32> {
        events.iter().filter(|e| e.event == "ScrollBar.changed").map(|e| e.args[0].as_i32()).collect()
    }

    #[test]
    fn arrows_step_and_repeat_while_held() {
        let (mut ui, win, bar) = widget();
        let down = (18, 10 + 160 - 5);
        assert_eq!(values(&ui.pointer(win, down.0, down.1, PointerKind::Press).unwrap()), [1]);
        // nothing more until the pause is over, then steadily
        assert!(ui.tick(REPEAT_DELAY_MS - 1).is_empty());
        assert_eq!(ui.next_deadline(), Some(REPEAT_DELAY_MS));
        assert_eq!(values(&ui.tick(REPEAT_DELAY_MS)), [2]);
        assert_eq!(values(&ui.tick(REPEAT_DELAY_MS + REPEAT_MS)), [3]);
        // sliding off the arrow pauses it; back on, it carries on
        ui.pointer(win, 80, 80, PointerKind::Move).unwrap();
        assert!(ui.tick(REPEAT_DELAY_MS + 2 * REPEAT_MS).is_empty());
        ui.pointer(win, down.0, down.1, PointerKind::Move).unwrap();
        assert_eq!(values(&ui.tick(REPEAT_DELAY_MS + 3 * REPEAT_MS)), [4]);
        ui.pointer(win, down.0, down.1, PointerKind::Release).unwrap();
        assert!(ui.tick(10_000).is_empty());
        assert_eq!((ui.scroll_bar_value(bar).unwrap(), ui.next_deadline()), (4, None));
        // at the top, the up arrow has nowhere to go and says nothing
        ui.scroll_bar_set_value(bar, 0).unwrap();
        assert!(ui.pointer(win, 18, 14, PointerKind::Press).unwrap().is_empty());
        ui.pointer(win, 18, 14, PointerKind::Release).unwrap();
    }

    #[test]
    fn the_box_can_be_dragged_and_the_track_pages() {
        let (mut ui, win, bar) = widget();
        // take hold of the box (it starts just below the up arrow) and pull it down
        ui.pointer(win, 18, 30, PointerKind::Press).unwrap();
        let ev = ui.pointer(win, 18, 30 + 59, PointerKind::Move).unwrap();
        let half = values(&ev)[0];
        assert!((45..=55).contains(&half), "{half}"); // about half way along
        // far past the end, even outside the window, it stops at the end
        assert_eq!(values(&ui.pointer(win, 300, 900, PointerKind::Move).unwrap()), [100]);
        ui.pointer(win, 300, 900, PointerKind::Release).unwrap();
        // a click in the track above the box pages back by a page less a step
        assert_eq!(values(&ui.pointer(win, 18, 40, PointerKind::Press).unwrap()), [91]);
        ui.pointer(win, 18, 40, PointerKind::Release).unwrap();
        // the wheel moves it too
        ui.wheel(win, 18, 60, -3).unwrap();
        assert_eq!(ui.scroll_bar_value(bar).unwrap(), 88);
    }

    #[test]
    fn settings_are_checked_and_the_value_kept_in_range() {
        let (mut ui, win, bar) = widget();
        ui.scroll_bar_set_value(bar, 500).unwrap();
        assert_eq!(ui.scroll_bar_value(bar).unwrap(), 100);
        ui.scroll_bar_set_max(bar, 20).unwrap();
        assert_eq!(ui.scroll_bar_value(bar).unwrap(), 20);
        assert_eq!(ui.scroll_bar_set_max(bar, -1), Err(Error::InvalidArgument));
        assert_eq!(ui.scroll_bar_set_page(bar, 0), Err(Error::InvalidArgument));
        assert_eq!(ui.scroll_bar_set_length(bar, 10), Err(Error::InvalidArgument));
        assert_eq!(ui.scroll_bar_set_step(win, 1), Err(Error::WrongType));
        // with nothing to scroll the bar swallows clicks and does nothing
        ui.scroll_bar_set_max(bar, 0).unwrap();
        assert!(ui.pointer(win, 18, 165, PointerKind::Press).unwrap().is_empty());
        assert_eq!(ui.next_deadline(), None);
        // a disabled one too, and it is drawn in black and white whatever its state
        ui.scroll_bar_set_max(bar, 50).unwrap();
        ui.set_enabled(bar, false).unwrap();
        assert!(ui.pointer(win, 18, 165, PointerKind::Press).unwrap().is_empty());
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }

    #[test]
    fn a_text_area_has_a_scroll_bar_of_its_own() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Notes", 240, 120).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let text: Vec<String> = (1..=12).map(|n| format!("line {n}")).collect();
        let area = ui.text_area_new(&text.join("\n"));
        ui.text_field_set_min_width(area, 200).unwrap();
        ui.text_area_set_rows(area, 4).unwrap();
        ui.stack_add(col, area).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.key(win, Key::Home, Mods { shift: false, ctrl: true, alt: false }).unwrap();
        ui.render(win).unwrap();
        let scroll = |ui: &Ui| match &ui.nodes.get(area).unwrap().kind {
            Kind::TextField { scroll, .. } => *scroll,
            _ => unreachable!(),
        };
        let bar = ui.bar_of(area, false).unwrap();
        assert_eq!((bar.value, bar.max, bar.page), (0, 8, 4));
        // it runs down the right-hand side, inside the area
        let area_rect = ui.nodes.get(area).unwrap().rect;
        assert_eq!((bar.rect.x + bar.rect.w, bar.rect.h), (area_rect.x + area_rect.w - 1, area_rect.h - 2));
        // the down arrow scrolls a line, and nothing is reported: it is the area's own business
        let down = (bar.rect.x + 8, bar.rect.y + bar.rect.h - 5);
        assert!(ui.pointer(win, down.0, down.1, PointerKind::Press).unwrap().is_empty());
        ui.pointer(win, down.0, down.1, PointerKind::Release).unwrap();
        ui.render(win).unwrap();
        assert_eq!(scroll(&ui), 1);
        // working the bar does not move the caret or take the text's selection
        ui.text(win, "X").unwrap();
        assert!(ui.text_field_text(area).unwrap().starts_with("Xline 1"));
        // the pointer is an arrow over the bar and an I-beam over the text
        assert_eq!(ui.cursor_at(win, down.0, down.1), crate::cursor::Cursor::Arrow);
        assert_eq!(ui.cursor_at(win, 30, 20), crate::cursor::Cursor::Text);
        // dragging the box to the bottom shows the last lines
        let (thumb, _) = ui.bar_of(area, false).unwrap().thumb().unwrap();
        ui.pointer(win, down.0, thumb + 2, PointerKind::Press).unwrap();
        ui.pointer(win, down.0, 500, PointerKind::Move).unwrap();
        ui.pointer(win, down.0, 500, PointerKind::Release).unwrap();
        ui.render(win).unwrap();
        assert_eq!(scroll(&ui), 8);
        // text that fits leaves the bar blank and dead
        ui.text_field_set_text(area, "short").unwrap();
        ui.render(win).unwrap();
        assert_eq!(ui.bar_of(area, false).unwrap().thumb(), None);
        // the text wraps short of the bar
        ui.text_field_set_text(area, &"word ".repeat(30)).unwrap();
        let canvas = ui.render(win).unwrap();
        let bar = ui.bar_of(area, false).unwrap();
        assert!(bar.max > 0);
        assert!((bar.rect.y + 1..bar.rect.y + bar.rect.h - 1).all(|y| canvas.get(bar.rect.x - 1, y) == WHITE));
    }
}
