// SPDX-License-Identifier: GPL-3.0-or-later
//! Radio buttons and sliders, and the pointer and key handling they share
//! with lists.
//!
//! Radio buttons that share a parent are one group: choosing one unchooses
//! the rest. A slider is a number picked by position.

use super::*;

#[cfg(test)]
const KNOB: i32 = 9; // the slider's knob, along its direction of travel
#[cfg(test)]
use super::list::ROW_HEIGHT;

impl Ui {
    // ---------------------------------------------------------- radio buttons

    pub fn radio_button_new(&mut self, label: &str) -> Handle {
        self.add(Kind::Radio { label: label.into(), checked: false })
    }

    pub fn radio_button_set_label(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Radio { label, .. } => {
                *label = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn radio_button_checked(&self, h: Handle) -> Result<bool> {
        match &self.node(h)?.kind {
            Kind::Radio { checked, .. } => Ok(*checked),
            _ => Err(Error::WrongType),
        }
    }

    /// Check or uncheck from the program. Checking one unchecks the others
    /// in its group. Nothing is emitted.
    pub fn radio_button_set_checked(&mut self, h: Handle, value: bool) -> Result<()> {
        if !matches!(self.node(h)?.kind, Kind::Radio { .. }) {
            return Err(Error::WrongType);
        }
        if value {
            self.radio_choose(h);
        } else if let Kind::Radio { checked, .. } = &mut self.node_mut(h)?.kind {
            *checked = false;
        }
        Ok(())
    }

    /// The radio buttons that share a parent with this one, itself included.
    fn radio_group(&self, h: Handle) -> Vec<Handle> {
        let siblings = self.nodes.get(h).and_then(|n| n.parent).map(|p| self.children(p)).unwrap_or_else(|| vec![h]);
        siblings.into_iter().filter(|s| matches!(self.nodes.get(*s).map(|n| &n.kind), Some(Kind::Radio { .. }))).collect()
    }

    /// Whether Tab stops here: always, unless this is a radio button that is
    /// not the one to stop at in its group (the chosen one, or the first).
    pub(super) fn radio_stop(&self, h: Handle) -> bool {
        if !matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Radio { .. })) {
            return true;
        }
        let group: Vec<Handle> = self.radio_group(h).into_iter().filter(|r| self.focusable(*r)).collect();
        let chosen = group.iter().copied().find(|r| matches!(self.nodes.get(*r).map(|n| &n.kind), Some(Kind::Radio { checked: true, .. })));
        chosen.or(group.first().copied()) == Some(h)
    }

    /// Make this the chosen one of its group. Returns whether that is a change.
    fn radio_choose(&mut self, h: Handle) -> bool {
        let mut changed = false;
        for other in self.radio_group(h) {
            if let Some(Kind::Radio { checked, .. }) = self.nodes.get_mut(other).map(|n| &mut n.kind) {
                changed |= *checked != (other == h);
                *checked = other == h;
            }
        }
        self.dirty = true;
        changed
    }

    /// The user chose a radio button.
    pub(super) fn radio_pick(&mut self, h: Handle) -> Vec<Emitted> {
        if self.radio_choose(h) {
            vec![Emitted { target: h, event: "RadioButton.chosen", args: vec![] }]
        } else {
            Vec::new() // it was already the one
        }
    }

    pub(super) fn measure_radio(&mut self, h: Handle) -> (i32, i32) {
        let label = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Radio { label, .. }) => label.clone(),
            _ => String::new(),
        };
        (self.look.m.radio_size + self.look.m.radio_gap + self.font.width(&label), self.font.line_height().max(self.look.m.radio_size))
    }

    pub(super) fn paint_radio(&mut self, h: Handle, rect: Rect, live: bool, held: bool, canvas: &mut Canvas) {
        let Some(Kind::Radio { label, checked }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (label, checked) = (label.clone(), *checked);
        let part = Part::new(PartKind::Radio, rect).text(&label).phase(self.ants_step_public());
        self.part(canvas, part.when(state::CHECKED, checked).when(state::HELD, held).when(state::FOCUSED, live));
    }

    // ----------------------------------------------------------------- sliders

    pub fn slider_new(&mut self, orientation: Orientation) -> Handle {
        self.add(Kind::Slider { orientation, value: 0, max: 100, step: 1, length: 120 })
    }

    pub fn slider_value(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Slider { value, .. } => Ok(*value),
            _ => Err(Error::WrongType),
        }
    }

    pub fn slider_max(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Slider { max, .. } => Ok(*max),
            _ => Err(Error::WrongType),
        }
    }

    /// Set the value from the program; it is brought into range. Nothing is emitted.
    pub fn slider_set_value(&mut self, h: Handle, to: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Slider { value, max, .. } => {
                *value = to.clamp(0, *max);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn slider_set_max(&mut self, h: Handle, to: i32) -> Result<()> {
        if to < 1 {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Slider { value, max, .. } => {
                *max = to;
                *value = (*value).min(to);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn slider_set_step(&mut self, h: Handle, to: i32) -> Result<()> {
        if to < 1 {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Slider { step, .. } => {
                *step = to;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn slider_set_length(&mut self, h: Handle, to: i32) -> Result<()> {
        if !(3 * self.look.m.slider_knob..=16384).contains(&to) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Slider { length, .. } => {
                *length = to;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// A slider's shape: (vertical, start and length of the knob's travel, value, max).
    fn slider_shape(&self, h: Handle) -> Option<(bool, i32, i32, i32, i32)> {
        let node = self.nodes.get(h)?;
        let Kind::Slider { orientation, value, max, .. } = &node.kind else { return None };
        let vertical = *orientation == Orientation::Vertical;
        let (start, len) = if vertical { (node.rect.y, node.rect.h) } else { (node.rect.x, node.rect.w) };
        Some((vertical, start, (len - self.look.m.slider_knob).max(1), *value, (*max).max(1)))
    }

    /// Where the knob starts, along the slider. A vertical slider has its
    /// largest value at the top.
    fn knob_at(&self, h: Handle) -> Option<i32> {
        let (vertical, start, travel, value, max) = self.slider_shape(h)?;
        let along = (travel as i64 * value as i64 / max as i64) as i32;
        Some(start + if vertical { travel - along } else { along })
    }

    /// The value that puts the knob's start at a position along the slider.
    fn slider_value_at(&self, h: Handle, knob_start: i32) -> Option<i32> {
        let (vertical, start, travel, _, max) = self.slider_shape(h)?;
        let along = (knob_start - start).clamp(0, travel);
        let along = if vertical { travel - along } else { along };
        Some(((along as i64 * max as i64 + travel as i64 / 2) / travel as i64) as i32)
    }

    /// Move a slider because the user worked it.
    fn slider_move(&mut self, h: Handle, to: i32) -> Vec<Emitted> {
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::Slider { value, max, .. }) => {
                let to = to.clamp(0, *max);
                if to == *value {
                    return Vec::new();
                }
                *value = to;
                self.dirty = true;
                vec![Emitted { target: h, event: "Slider.changed", args: vec![Value::I32(to)] }]
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn measure_slider(&self, h: Handle) -> (i32, i32) {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Slider { orientation: Orientation::Vertical, length, .. }) => (self.look.m.slider_thick, *length),
            Some(Kind::Slider { length, .. }) => (*length, self.look.m.slider_thick),
            _ => (0, 0),
        }
    }

    pub(super) fn paint_slider(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        let (Some((vertical, ..)), Some(knob)) = (self.slider_shape(h), self.knob_at(h)) else { return };
        let held = self.slider_drag.is_some_and(|(held, _)| held == h);
        let part = Part::new(PartKind::Slider, rect).numbers(knob, self.look.m.slider_knob, 0).phase(self.ants_step_public());
        self.part(canvas, part.when(state::VERTICAL, vertical).when(state::FOCUSED, live).when(state::HELD, held));
    }

    // -------------------------------------------------------------- input

    /// Give sliders and list boxes first refusal of a pointer event.
    pub(super) fn control_pointer(&mut self, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        // a knob or a row being dragged keeps the pointer until release
        if let Some((slider, grip)) = self.slider_drag {
            let Some((vertical, ..)) = self.slider_shape(slider) else {
                self.slider_drag = None;
                return Some(Vec::new());
            };
            let pos = if vertical { y } else { x } - grip;
            let out = self.slider_value_at(slider, pos).map(|v| self.slider_move(slider, v)).unwrap_or_default();
            if kind == PointerKind::Release {
                self.slider_drag = None;
                self.dirty = true;
            }
            return Some(out);
        }
        if self.column_drag.is_some() {
            self.column_drag_to(x, kind == PointerKind::Release);
            return Some(Vec::new());
        }
        if let Some(list) = self.list_drag {
            let out = if kind == PointerKind::Move { self.list_drag_to(list, x, y) } else { Vec::new() };
            let mut out = out;
            if kind == PointerKind::Release {
                self.list_drag = None;
                out.extend(self.row_drop());
            }
            return Some(out);
        }
        if kind != PointerKind::Press {
            return None;
        }
        let h = over?;
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Slider { .. }) => {
                let (vertical, ..) = self.slider_shape(h)?;
                let (pos, knob) = (if vertical { y } else { x }, self.knob_at(h)?);
                // on the knob: hold it where it was taken. Elsewhere on the
                // groove: the knob comes to the pointer, then follows it.
                let grip = if (knob..knob + self.look.m.slider_knob).contains(&pos) { pos - knob } else { self.look.m.slider_knob / 2 };
                self.slider_drag = Some((h, grip));
                self.dirty = true;
                Some(self.slider_value_at(h, pos - grip).map(|v| self.slider_move(h, v)).unwrap_or_default())
            }
            Some(Kind::List { .. }) => Some(self.list_press(h, x, y)),
            Some(Kind::Tabs { .. }) => Some(self.tabs_press(h, x, y)),
            _ => None,
        }
    }

    /// Give the focused control first refusal of a key.
    pub(super) fn control_key(&mut self, h: Handle, key: Key, mods: Mods) -> Option<Vec<Emitted>> {
        if let Some(told) = self.picture_key(h, key) {
            return Some(told);
        }
        if let Some(told) = self.stepper_key(h, key) {
            return Some(told);
        }
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Radio { .. }) => {
                // the arrows walk the group, choosing as they go
                let forward = match key {
                    Key::Down | Key::Right => true,
                    Key::Up | Key::Left => false,
                    _ => return None,
                };
                let group: Vec<Handle> = self.radio_group(h).into_iter().filter(|r| self.focusable(*r)).collect();
                let at = group.iter().position(|r| *r == h)?;
                let next = group[if forward { (at + 1) % group.len() } else { (at + group.len() - 1) % group.len() }];
                let window = self.window_of(h)?;
                self.set_focus(window, Some(next), false);
                Some(self.radio_pick(next))
            }
            Some(Kind::Slider { value, max, step, .. }) => {
                let (value, max, step) = (*value, *max, *step);
                let to = match key {
                    Key::Right | Key::Up => value + step,
                    Key::Left | Key::Down => value - step,
                    Key::Home => 0,
                    Key::End => max,
                    _ => return None,
                };
                Some(self.slider_move(h, to))
            }
            Some(Kind::List { .. }) => self.list_key(h, key, mods),
            Some(Kind::Tabs { .. }) => self.tabs_key(h, key),
            _ => None,
        }
    }

    /// Turn the wheel over a slider.
    pub(super) fn slider_wheel(&mut self, h: Handle, lines: i32) -> Option<Vec<Emitted>> {
        match self.nodes.get(h).map(|n| &n.kind) {
            // wheel up is more, as pushing the knob up or right would be
            Some(Kind::Slider { value, step, .. }) => {
                let to = *value - lines * *step;
                Some(self.slider_move(h, to))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };

    fn window(ui: &mut Ui, content: Handle) -> Handle {
        let win = ui.window_new("Controls", 240, 220).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        ui.stack_add(col, content).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        win
    }

    fn rect(ui: &Ui, h: Handle) -> Rect {
        ui.nodes.get(h).unwrap().rect
    }

    fn click(ui: &mut Ui, win: Handle, x: i32, y: i32) -> Vec<Emitted> {
        let mut ev = ui.pointer(win, x, y, PointerKind::Press).unwrap();
        ev.extend(ui.pointer(win, x, y, PointerKind::Release).unwrap());
        ev
    }

    fn numbers(events: &[Emitted], name: &str) -> Vec<i32> {
        events.iter().filter(|e| e.event == name).map(|e| e.args[0].as_i32()).collect()
    }

    #[test]
    fn radio_buttons_in_one_parent_are_one_group() {
        let mut ui = Ui::new();
        let group = ui.stack_new(Orientation::Vertical);
        let radios: Vec<Handle> = ["Small", "Medium", "Large"].iter().map(|l| ui.radio_button_new(l)).collect();
        for r in &radios {
            ui.stack_add(group, *r).unwrap();
        }
        let other = ui.stack_new(Orientation::Vertical);
        let lone = ui.radio_button_new("Elsewhere");
        ui.stack_add(other, lone).unwrap();
        let both = ui.stack_new(Orientation::Vertical);
        ui.stack_add(both, group).unwrap();
        ui.stack_add(both, other).unwrap();
        let win = window(&mut ui, both);
        ui.radio_button_set_checked(lone, true).unwrap();
        let checked = |ui: &Ui| radios.iter().map(|r| ui.radio_button_checked(*r).unwrap()).collect::<Vec<_>>();

        // clicking one chooses it and reports it once
        let r1 = rect(&ui, radios[1]);
        let ev = click(&mut ui, win, r1.x + 4, r1.y + 4);
        assert_eq!(ev, vec![Emitted { target: radios[1], event: "RadioButton.chosen", args: vec![] }]);
        assert!(click(&mut ui, win, r1.x + 4, r1.y + 4).is_empty()); // already the one
        // choosing another unchooses it; the group next door is left alone
        let r2 = rect(&ui, radios[2]);
        click(&mut ui, win, r2.x + 4, r2.y + 4);
        assert_eq!(checked(&ui), [false, false, true]);
        assert!(ui.radio_button_checked(lone).unwrap());
        // the arrows walk the group, choosing as they go, and wrap
        ui.focus(radios[2]).unwrap();
        let ev = ui.key(win, Key::Down, NONE).unwrap();
        assert_eq!((ev[0].target, checked(&ui)), (radios[0], vec![true, false, false]));
        assert_eq!(ui.live_focus(win), Some(radios[0]));
        ui.key(win, Key::Up, NONE).unwrap();
        assert_eq!(checked(&ui), [false, false, true]);
        // Tab treats the group as one stop: from its chosen button straight on to the next group
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(lone));
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(radios[2]));
        // the program can set one without a report
        ui.radio_button_set_checked(radios[1], true).unwrap();
        assert_eq!(checked(&ui), [false, true, false]);
        ui.radio_button_set_checked(radios[1], false).unwrap();
        assert_eq!(checked(&ui), [false, false, false]); // none chosen is allowed
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }

    #[test]
    fn a_slider_follows_the_pointer_and_the_keys() {
        let mut ui = Ui::natural();
        let slider = ui.slider_new(Orientation::Horizontal);
        ui.slider_set_length(slider, 109).unwrap(); // 100 pixels of travel for a max of 100
        let win = window(&mut ui, slider);
        let r = rect(&ui, slider);
        // a press on the groove brings the knob there
        let ev = ui.pointer(win, r.x + KNOB / 2 + 30, r.y + 8, PointerKind::Press).unwrap();
        assert_eq!(numbers(&ev, "Slider.changed"), [30]);
        // and it then follows the pointer, even off the end and outside the window
        assert_eq!(numbers(&ui.pointer(win, r.x + KNOB / 2 + 75, 500, PointerKind::Move).unwrap(), "Slider.changed"), [75]);
        assert_eq!(numbers(&ui.pointer(win, 2000, 500, PointerKind::Move).unwrap(), "Slider.changed"), [100]);
        assert!(ui.pointer(win, 2000, 500, PointerKind::Move).unwrap().is_empty()); // no change, no report
        ui.pointer(win, 2000, 500, PointerKind::Release).unwrap();
        // taking the knob by its edge does not make it jump
        let knob = ui.knob_at(slider).unwrap();
        assert!(ui.pointer(win, knob + 1, r.y + 8, PointerKind::Press).unwrap().is_empty());
        ui.pointer(win, knob + 1, r.y + 8, PointerKind::Release).unwrap();
        // keys step it; Home and End go to the ends
        ui.focus(slider).unwrap();
        ui.slider_set_step(slider, 5).unwrap();
        assert_eq!(numbers(&ui.key(win, Key::Left, NONE).unwrap(), "Slider.changed"), [95]);
        assert_eq!(numbers(&ui.key(win, Key::Home, NONE).unwrap(), "Slider.changed"), [0]);
        assert!(ui.key(win, Key::Left, NONE).unwrap().is_empty());
        assert_eq!(numbers(&ui.key(win, Key::End, NONE).unwrap(), "Slider.changed"), [100]);
        // the wheel works over it; up is more
        assert_eq!(numbers(&ui.wheel(win, r.x + 20, r.y + 8, 2).unwrap(), "Slider.changed"), [90]);
        // settings are checked and the value kept in range
        ui.slider_set_max(slider, 10).unwrap();
        assert_eq!(ui.slider_value(slider).unwrap(), 10);
        assert_eq!(ui.slider_set_max(slider, 0), Err(Error::InvalidArgument));
        ui.slider_set_value(slider, -4).unwrap();
        assert_eq!(ui.slider_value(slider).unwrap(), 0);
        // a vertical one has its largest value at the top
        let tall = ui.slider_new(Orientation::Vertical);
        ui.slider_set_length(tall, 109).unwrap();
        let win2 = window(&mut ui, tall);
        let r = rect(&ui, tall);
        assert_eq!(numbers(&ui.pointer(win2, r.x + 8, r.y + KNOB / 2 + 20, PointerKind::Press).unwrap(), "Slider.changed"), [80]);
    }

    #[test]
    fn a_list_box_selects_scrolls_and_opens() {
        let mut ui = Ui::natural();
        let list = ui.list_box_new();
        for name in ["Apple", "Banana", "Cherry", "Date", "Elderberry", "Fig", "Grape", "Honeydew"] {
            ui.list_box_add(list, name).unwrap();
        }
        ui.list_box_set_rows(list, 4).unwrap();
        let win = window(&mut ui, list);
        let r = rect(&ui, list);
        assert_eq!(ui.list_box_selected(list).unwrap(), -1);
        // a click selects the row and gives the list the keyboard
        let ev = click(&mut ui, win, r.x + 20, r.y + 2 + ROW_HEIGHT + 4);
        assert_eq!(numbers(&ev, "ListBox.changed"), [1]);
        assert_eq!(ui.live_focus(win), Some(list));
        // a second click on the same row, straight after, opens it
        let ev = click(&mut ui, win, r.x + 20, r.y + 2 + ROW_HEIGHT + 4);
        assert_eq!((numbers(&ev, "ListBox.changed"), numbers(&ev, "ListBox.activated")), (vec![], vec![1]));
        // dragging down the rows moves the selection
        ui.tick(5_000);
        ui.pointer(win, r.x + 20, r.y + 6, PointerKind::Press).unwrap();
        let ev = ui.pointer(win, r.x + 20, r.y + 6 + 2 * ROW_HEIGHT, PointerKind::Move).unwrap();
        assert_eq!(numbers(&ev, "ListBox.changed"), [2]);
        ui.pointer(win, r.x + 20, r.y + 6 + 2 * ROW_HEIGHT, PointerKind::Release).unwrap();
        // the arrows move it and the list scrolls to follow
        for _ in 0..4 {
            ui.key(win, Key::Down, NONE).unwrap();
        }
        assert_eq!(ui.list_box_selected(list).unwrap(), 6);
        let bar = ui.bar_of(list, false).unwrap();
        assert_eq!((bar.value, bar.max, bar.page), (3, 4, 4));
        assert_eq!(numbers(&ui.key(win, Key::End, NONE).unwrap(), "ListBox.changed"), [7]);
        assert!(ui.key(win, Key::Down, NONE).unwrap().is_empty()); // already at the end
        // typing a letter jumps to the next row that starts with it
        assert_eq!(numbers(&ui.text(win, "c").unwrap(), "ListBox.changed"), [2]);
        assert_eq!(ui.bar_of(list, false).unwrap().value, 2);
        // Return opens the selected row
        assert_eq!(numbers(&ui.key(win, Key::Return, NONE).unwrap(), "ListBox.activated"), [2]);
        // its scroll bar and the wheel move the view without moving the selection
        let bar = ui.bar_of(list, false).unwrap();
        click(&mut ui, win, bar.rect.x + 8, bar.rect.y + bar.rect.h - 5);
        ui.wheel(win, r.x + 20, r.y + 20, 1).unwrap();
        assert_eq!((ui.bar_of(list, false).unwrap().value, ui.list_box_selected(list).unwrap()), (4, 2));
        // a click then lands on the row showing there
        ui.tick(10_000);
        assert_eq!(numbers(&click(&mut ui, win, r.x + 20, r.y + 6), "ListBox.changed"), [4]);
        // the program can select and clear
        ui.list_box_set_selected(list, 0).unwrap();
        assert_eq!(ui.bar_of(list, false).unwrap().value, 0);
        ui.list_box_set_selected(list, -1).unwrap();
        assert_eq!(ui.list_box_set_selected(list, 8), Err(Error::InvalidArgument));
        ui.list_box_clear(list).unwrap();
        assert_eq!(ui.list_box_selected(list).unwrap(), -1);
        assert!(click(&mut ui, win, r.x + 20, r.y + 6).is_empty());
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }
}
