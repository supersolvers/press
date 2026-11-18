// SPDX-License-Identifier: GPL-3.0-or-later
//! The pop-up button: a box showing one of a list of choices, which pops
//! the whole list up over itself when pressed.
//!
//! As on the original, the list appears with the current choice directly
//! over the box, so that pressing and letting go changes nothing, and a
//! short drag up or down picks a neighbour. A plain click leaves the list
//! up to be clicked, which the original did not do and people now expect.

use super::menu::{EditAction, ITEM_LEFT};
#[cfg(test)]
use super::menu::ITEM_HEIGHT;
use super::*;

const ARROW_ROOM: i32 = 22;

impl Ui {
    pub fn popup_button_new(&mut self) -> Handle {
        self.add(Kind::Popup { items: Vec::new(), selected: 0, menu: None })
    }

    /// Append a choice. The first one added starts as the current choice.
    pub fn popup_button_add(&mut self, h: Handle, label: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Popup { items, .. } => {
                items.push(label.to_string());
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Remove every choice.
    pub fn popup_button_clear(&mut self, h: Handle) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Popup { items, selected, .. } => {
                items.clear();
                *selected = 0;
            }
            _ => return Err(Error::WrongType),
        }
        self.dismiss_popup_of(h);
        Ok(())
    }

    /// The index of the current choice, or -1 if there are none.
    pub fn popup_button_selected(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Popup { items, selected, .. } => Ok(if items.is_empty() { -1 } else { *selected as i32 }),
            _ => Err(Error::WrongType),
        }
    }

    /// Set the current choice from the program. Nothing is emitted.
    pub fn popup_button_set_selected(&mut self, h: Handle, index: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Popup { items, selected, .. } => {
                if index < 0 || index as usize >= items.len() {
                    return Err(Error::InvalidArgument);
                }
                *selected = index as usize;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// The text of the current choice.
    pub(super) fn popup_label(&self, h: Handle) -> Option<String> {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Popup { items, selected, .. }) => items.get(*selected).cloned(),
            _ => None,
        }
    }

    pub(super) fn measure_popup(&mut self, h: Handle) -> (i32, i32) {
        let items = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Popup { items, .. }) => items.clone(),
            _ => Vec::new(),
        };
        let widest = items.iter().map(|i| self.font.width(i)).max().unwrap_or(0).max(40);
        (ITEM_LEFT + widest + ARROW_ROOM + 2, self.look.m.popup_height)
    }

    pub(super) fn paint_popup(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        let label = self.popup_label(h).unwrap_or_default();
        let showing = self.menu_open.as_ref().is_some_and(|m| m.target == Some(h));
        let part = Part::new(PartKind::Popup, rect).text(&label).phase(self.ants_step_public());
        self.part(canvas, part.when(state::FOCUSED, live).when(state::HELD, showing));
    }

    /// Put away the list of a pop-up button if it is showing.
    fn dismiss_popup_of(&mut self, h: Handle) {
        if self.menu_open.as_ref().is_some_and(|m| m.target == Some(h)) {
            self.close_menu();
        }
    }

    /// Pop a button's list up over it. `held` says the pointer button is
    /// down, so that letting go over a choice picks it.
    pub(super) fn popup_open(&mut self, h: Handle, held: bool) {
        let Some(window) = self.window_of(h) else { return };
        let (items, selected, old, rect) = match self.nodes.get(h).map(|n| (&n.kind, n.rect)) {
            Some((Kind::Popup { items, selected, menu }, rect)) => (items.clone(), *selected, *menu, rect),
            _ => return,
        };
        if items.is_empty() {
            return;
        }
        // The list is a menu made for the occasion; the last one is thrown away.
        if let Some(old) = old {
            let _ = self.destroy(old);
        }
        let menu = self.menu_new("");
        let mut current = None;
        for (index, label) in items.iter().enumerate() {
            let item = self.add(Kind::MenuItem {
                label: label.clone(),
                checked: index == selected,
                shortcut: None,
                separator: false,
                submenu: None,
                action: Some(EditAction::Pick(index)),
            });
            let _ = self.menu_add(menu, item);
            if index == selected {
                current = Some(item);
            }
        }
        if let Some(node) = self.nodes.get_mut(menu) {
            node.parent = Some(h); // it is the button's, and goes when the button goes
        }
        if let Some(Kind::Popup { menu: slot, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *slot = Some(menu);
        }
        // placed so the current choice lies exactly over the box
        let y = rect.y - selected as i32 * self.look.m.menu_item_height;
        self.open_popup(window, menu, rect.x, y, Some(h));
        if let Some(open) = &mut self.menu_open {
            open.held = held;
            open.opening = held;
            open.min_inner = rect.w - 3;
            open.levels[0].hot = current;
        }
    }

    /// One of a pop-up button's choices was picked from its list.
    pub(super) fn popup_pick(&mut self, h: Handle, index: usize) -> Vec<Emitted> {
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::Popup { items, selected, .. }) if index < items.len() && *selected != index => {
                *selected = index;
                self.dirty = true;
                vec![Emitted { target: h, event: "PopupButton.changed", args: vec![Value::I32(index as i32)] }]
            }
            _ => Vec::new(), // the same choice again is no change
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };

    /// A window with a pop-up button of three sizes at (10, 60), Medium current.
    fn fixture() -> (Ui, Handle, Handle) {
        let mut ui = Ui::new();
        let win = ui.window_new("Pop", 200, 160).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        ui.stack_set_spacing(col, 30).unwrap();
        let label = ui.label_new("Size:");
        let pop = ui.popup_button_new();
        for size in ["Small", "Medium", "Large"] {
            ui.popup_button_add(pop, size).unwrap();
        }
        ui.popup_button_set_selected(pop, 1).unwrap();
        ui.stack_add(col, label).unwrap();
        ui.stack_add(col, pop).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        (ui, win, pop)
    }

    fn picked(events: Vec<Emitted>) -> Vec<i32> {
        events.into_iter().filter(|e| e.event == "PopupButton.changed").map(|e| e.args[0].as_i32()).collect()
    }

    #[test]
    fn the_list_pops_up_with_the_current_choice_over_the_box() {
        let (mut ui, win, pop) = fixture();
        let rect = ui.nodes.get(pop).unwrap().rect;
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Press).unwrap();
        let popups = ui.popups(win);
        assert_eq!(popups.len(), 1);
        // one row up from the box, so that "Medium" covers "Medium"
        assert_eq!((popups[0].rect.x, popups[0].rect.y), (rect.x, rect.y - ITEM_HEIGHT));
        assert!(popups[0].rect.w >= rect.w); // at least as wide as the box
        // dragging down a row and letting go picks the next one
        ui.tick(1_000);
        ui.pointer(win, rect.x + 30, rect.y + 8 + ITEM_HEIGHT, PointerKind::Move).unwrap();
        ui.pointer(win, rect.x + 30, rect.y + 8 + ITEM_HEIGHT, PointerKind::Release).unwrap();
        assert_eq!(picked(ui.finish_animations()), [2]);
        assert_eq!((ui.popup_button_selected(pop).unwrap(), ui.popups(win).len()), (2, 0));
        // pressing and letting go without moving changes nothing and says nothing
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Press).unwrap();
        ui.tick(2_000);
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Release).unwrap();
        assert!(picked(ui.finish_animations()).is_empty());
        assert_eq!(ui.popup_button_selected(pop).unwrap(), 2);
    }

    #[test]
    fn a_quick_click_leaves_the_list_up_to_be_clicked() {
        let (mut ui, win, pop) = fixture();
        let rect = ui.nodes.get(pop).unwrap().rect;
        let at = (rect.x + 30, rect.y + 8);
        ui.pointer(win, at.0, at.1, PointerKind::Press).unwrap();
        ui.pointer(win, at.0, at.1, PointerKind::Release).unwrap(); // at once: a click, not a hold
        assert_eq!(ui.popups(win).len(), 1);
        // then a click on the first row picks it
        ui.pointer(win, at.0, at.1 - ITEM_HEIGHT, PointerKind::Press).unwrap();
        ui.pointer(win, at.0, at.1 - ITEM_HEIGHT, PointerKind::Release).unwrap();
        assert_eq!(picked(ui.finish_animations()), [0]);
        assert_eq!(ui.popup_button_selected(pop).unwrap(), 0);
        // a click elsewhere puts the list away without choosing
        ui.pointer(win, at.0, at.1, PointerKind::Press).unwrap();
        ui.pointer(win, at.0, at.1, PointerKind::Release).unwrap();
        ui.pointer(win, 180, 150, PointerKind::Press).unwrap();
        ui.pointer(win, 180, 150, PointerKind::Release).unwrap();
        assert_eq!((ui.popups(win).len(), ui.popup_button_selected(pop).unwrap()), (0, 0));
    }

    #[test]
    fn the_keyboard_works_it() {
        let (mut ui, win, pop) = fixture();
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(pop));
        ui.text(win, " ").unwrap(); // Space pops the list up, on the current choice
        assert_eq!(ui.popups(win).len(), 1);
        ui.key(win, Key::Up, NONE).unwrap();
        ui.key(win, Key::Return, NONE).unwrap();
        assert_eq!(picked(ui.finish_animations()), [0]);
        ui.text(win, " ").unwrap();
        ui.key(win, Key::Escape, NONE).unwrap();
        assert_eq!(ui.popup_button_selected(pop).unwrap(), 0);
    }

    #[test]
    fn popup_button_rules() {
        let (mut ui, win, pop) = fixture();
        assert_eq!(ui.popup_button_set_selected(pop, 3), Err(Error::InvalidArgument));
        assert_eq!(ui.popup_button_set_selected(pop, -1), Err(Error::InvalidArgument));
        assert_eq!(ui.popup_button_add(win, "x"), Err(Error::WrongType));
        // it is as wide as its widest choice needs
        let narrow = ui.measure_popup(pop).0;
        ui.popup_button_add(pop, "Extraordinarily Large").unwrap();
        assert!(ui.measure_popup(pop).0 > narrow);
        // disabled, it does not pop up
        ui.set_enabled(pop, false).unwrap();
        let rect = ui.nodes.get(pop).unwrap().rect;
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Press).unwrap();
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Release).unwrap();
        assert_eq!(ui.popups(win).len(), 0);
        ui.set_enabled(pop, true).unwrap();
        // emptied, it has no choice and nothing to pop up
        ui.popup_button_clear(pop).unwrap();
        assert_eq!(ui.popup_button_selected(pop).unwrap(), -1);
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Press).unwrap();
        assert_eq!(ui.popups(win).len(), 0);
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Release).unwrap();
        // the list it made goes when it goes; it draws in black and white
        ui.popup_button_add(pop, "One").unwrap();
        ui.pointer(win, rect.x + 30, rect.y + 8, PointerKind::Press).unwrap();
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
        ui.destroy(pop).unwrap();
        assert_eq!(ui.popups(win).len(), 0);
    }
}
