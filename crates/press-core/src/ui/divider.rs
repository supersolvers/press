// SPDX-License-Identifier: GPL-3.0-or-later
//! A divider: the bar between two panes of a stack, dragged to give one
//! more room and the other less, as between a file manager's folders and
//! what is in the one open.
//!
//! It stands in a stack between two of its children. Until it is dragged,
//! the stack lays them out as it always does; once it has a position, the
//! child before it is exactly that long along the stack, and the children
//! after it share what is left.

use super::*;

/// How thick a divider is, along the stack it divides.
pub(super) const THICK: i32 = 5;
/// The least either pane is left by dragging.
const LEAST: i32 = 24;

impl Ui {
    /// A divider for a stack laid out this way: horizontal for panes side
    /// by side, vertical for panes one over the other.
    pub fn divider_new(&mut self, orientation: Orientation) -> Handle {
        self.add(Kind::Divider { orientation, position: None })
    }

    /// How long the pane before the divider is, in pixels along the stack:
    /// as last set or dragged, or as laid out if it never has been.
    pub fn divider_position(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Divider { position: Some(p), .. } => Ok(*p),
            Kind::Divider { orientation, .. } => {
                let before = self.pane_before(h).and_then(|p| self.nodes.get(p)).map(|n| n.rect);
                Ok(before.map_or(0, |r| if *orientation == Orientation::Vertical { r.h } else { r.w }))
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Make the pane before the divider this long; a negative length gives
    /// it back to the stack to lay out as it would.
    pub fn divider_set_position(&mut self, h: Handle, value: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Divider { position, .. } => {
                *position = (value >= 0).then_some(value);
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// The child of the divider's stack just before it.
    fn pane_before(&self, h: Handle) -> Option<Handle> {
        let parent = self.nodes.get(h)?.parent?;
        let Kind::Stack { children, .. } = &self.nodes.get(parent)?.kind else { return None };
        let at = children.iter().position(|c| *c == h)?;
        at.checked_sub(1).map(|i| children[i])
    }

    /// For each child of a stack, the length a divider after it has fixed.
    pub(super) fn fixed_by_dividers(&self, children: &[Handle]) -> Vec<Option<i32>> {
        (0..children.len())
            .map(|i| match children.get(i + 1).and_then(|d| self.nodes.get(*d)).map(|n| &n.kind) {
                Some(Kind::Divider { position, .. }) => *position,
                _ => None,
            })
            .collect()
    }

    /// Whether a widget is a divider, and which way the panes it divides lie.
    pub(super) fn divider_way(&self, h: Handle) -> Option<Orientation> {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Divider { orientation, .. }) => Some(*orientation),
            _ => None,
        }
    }

    /// The pointer over a divider, or dragging one: a press takes hold, a
    /// move drags, and letting go says where it was left.
    pub(super) fn divider_pointer(&mut self, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        if let Some((h, grip)) = self.divider_drag {
            let way = self.divider_way(h)?;
            let vertical = way == Orientation::Vertical;
            let (Some(before), Some(parent)) = (self.pane_before(h), self.nodes.get(h).and_then(|n| n.parent)) else {
                self.divider_drag = None;
                return Some(Vec::new());
            };
            let (start, room) = {
                let (b, p) = (self.nodes.get(before).map(|n| n.rect).unwrap_or_default(), self.nodes.get(parent).map(|n| n.rect).unwrap_or_default());
                if vertical { (b.y, p.y + p.h - b.y) } else { (b.x, p.x + p.w - b.x) }
            };
            let at = if vertical { y } else { x };
            let length = (at - grip - start).clamp(LEAST.min(room), (room - THICK - LEAST).max(LEAST.min(room)));
            if let Some(Kind::Divider { position, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
                if *position != Some(length) {
                    *position = Some(length);
                    self.dirty = true;
                }
            }
            if kind == PointerKind::Release {
                self.divider_drag = None;
                return Some(vec![Emitted { target: h, event: "Divider.moved", args: vec![Value::I32(length)] }]);
            }
            return Some(Vec::new());
        }
        let h = over.filter(|h| self.divider_way(*h).is_some())?;
        if kind != PointerKind::Press {
            return None;
        }
        let r = self.nodes.get(h)?.rect;
        let grip = if self.divider_way(h)? == Orientation::Vertical { y - r.y } else { x - r.x };
        self.divider_drag = Some((h, grip));
        Some(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_divider_is_dragged_to_share_out_a_row() {
        let mut ui = Ui::new();
        let win = ui.window_new("Panes", 400, 200).unwrap();
        let row = ui.stack_new(Orientation::Horizontal);
        ui.stack_set_spacing(row, 0).unwrap();
        let (left, right) = (ui.list_box_new(), ui.list_box_new());
        let divider = ui.divider_new(Orientation::Horizontal);
        for c in [left, divider, right] {
            ui.stack_add(row, c).unwrap();
        }
        ui.window_set_content(win, row).unwrap();
        ui.render(win).unwrap();
        assert_eq!(ui.class_of(divider).unwrap(), "Divider");
        // set: the left pane is that wide, the divider next, the right the rest
        ui.divider_set_position(divider, 120).unwrap();
        ui.render(win).unwrap();
        let rect = |ui: &Ui, h: Handle| ui.nodes.get(h).unwrap().rect;
        assert_eq!(rect(&ui, left).w, 120);
        assert_eq!(rect(&ui, divider).x, 120);
        assert_eq!(rect(&ui, right).x, 120 + THICK);
        assert_eq!(rect(&ui, right).w, 400 - 120 - THICK);
        assert_eq!(ui.cursor_at(win, 122, 100), Cursor::ResizeColumn);
        // dragged: taken a pixel in, and let go 60 to the right
        ui.pointer(win, 121, 100, PointerKind::Press).unwrap();
        ui.pointer(win, 150, 100, PointerKind::Move).unwrap();
        let ev = ui.pointer(win, 181, 100, PointerKind::Release).unwrap();
        assert_eq!(ev, vec![Emitted { target: divider, event: "Divider.moved", args: vec![Value::I32(180)] }]);
        ui.render(win).unwrap();
        assert_eq!(rect(&ui, left).w, 180);
        assert_eq!(ui.divider_position(divider).unwrap(), 180);
        // never past where the other pane would vanish
        ui.pointer(win, 181, 100, PointerKind::Press).unwrap();
        ui.pointer(win, 399, 100, PointerKind::Release).unwrap();
        assert_eq!(ui.divider_position(divider).unwrap(), 400 - THICK - LEAST);
        // and given back, the stack shares the row out as it would
        ui.divider_set_position(divider, -1).unwrap();
        ui.render(win).unwrap();
        assert_eq!(rect(&ui, left).w + rect(&ui, right).w + THICK, 400);
    }
}
