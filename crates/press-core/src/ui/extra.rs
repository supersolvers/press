// SPDX-License-Identifier: GPL-3.0-or-later
//! What a whole program wants round its dialogs: a box with a title round
//! a group of things, a bar of tools across the top of a window and a bar
//! of news along its foot, buttons that lie flat until the pointer comes,
//! pictures in colour on buttons and in lists, a word of help beside the
//! pointer, a number with arrows to step it, and a text field with a list
//! of things it might say.
//!
//! The three bars and boxes are stacks in dress: they lay their children
//! out as a stack does, with room kept for their own edges, and a style
//! draws those. The rest hang what they add on widgets there already.

use super::*;
use std::rc::Rc;

/// What a stack is dressed as.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Dress {
    /// A box with a title, round things that belong together.
    Group(String),
    /// A bar of tools.
    Toolbar,
    /// A bar of news along the foot of a window; with each thing it
    /// says in a cell of its own, or all on one strip.
    Status(bool),
}

/// How long the pointer rests on a thing before its word of help shows.
const TIP_MS: u64 = 700;
/// How wide the number of a stepper is.
const STEPPER_FIELD: i32 = 54;

/// A colour part of the way from one to another, `at` out of 255.
fn mixed(from: u32, to: u32, at: u32) -> u32 {
    let mix = |shift: u32| ((((from >> shift) & 0xff) * (255 - at) + ((to >> shift) & 0xff) * at + 127) / 255) << shift;
    0xff00_0000 | mix(16) | mix(8) | mix(0)
}

/// The edges of a box or a bar as black and white draws them, for a style
/// that has not said how it would: a black line on white; on anything
/// else a line cut into it, dark with light beside it.
pub(crate) fn plain_dress(part: &Part, canvas: &mut Canvas, font: &mut Font) {
    let (rect, ground) = (part.rect, part.c as u32);
    let white = ground == crate::canvas::WHITE;
    let (dark, light) = if white { (BLACK, WHITE) } else { (mixed(ground, BLACK, 110), mixed(ground, WHITE, 190)) };
    let cut = |canvas: &mut Canvas, r: Rect| {
        if !white {
            canvas.frame(Rect::new(r.x + 1, r.y + 1, r.w, r.h), light);
        }
        canvas.frame(r, dark);
    };
    let line = font.line_height();
    match part.kind {
        PartKind::GroupBox => {
            let top = if part.text.is_empty() { 0 } else { line / 2 };
            cut(canvas, Rect::new(rect.x, rect.y + top, rect.w - 1, rect.h - top - 1));
            if !part.text.is_empty() {
                let wide = font.width(part.text);
                canvas.fill(Rect::new(rect.x + 7, rect.y, wide + 6, line), ground);
                font.draw(canvas, part.text, rect.x + 10, rect.y, part.ink);
            }
        }
        PartKind::Toolbar => {
            canvas.fill(Rect::new(rect.x, rect.y + rect.h - 2, rect.w, 1), dark);
            canvas.fill(Rect::new(rect.x, rect.y + rect.h - 1, rect.w, 1), light);
        }
        PartKind::StatusBar => {
            canvas.fill(Rect::new(rect.x, rect.y, rect.w, 1), dark);
            canvas.fill(Rect::new(rect.x, rect.y + 1, rect.w, 1), light);
        }
        PartKind::StatusPanel => {
            if white {
                // a line between one cell and the next
                if part.a > 0 {
                    canvas.fill(Rect::new(rect.x - 2, rect.y, 1, rect.h), dark);
                }
            } else {
                // sunk into the bar: dark above and to the left, light below and to the right
                canvas.fill(Rect::new(rect.x, rect.y, rect.w, 1), dark);
                canvas.fill(Rect::new(rect.x, rect.y, 1, rect.h), dark);
                canvas.fill(Rect::new(rect.x, rect.y + rect.h - 1, rect.w, 1), light);
                canvas.fill(Rect::new(rect.x + rect.w - 1, rect.y, 1, rect.h), light);
            }
        }
        PartKind::Tooltip => {
            canvas.fill(rect, ground);
            canvas.frame(rect, part.ink);
            font.draw(canvas, part.text, rect.x + 4, rect.y + (rect.h - line) / 2, part.ink);
        }
        _ => {}
    }
}

impl Ui {
    // ------------------------------------------------------- boxes and bars

    fn dressed(&mut self, orientation: Orientation, dress: Dress, spacing: i32) -> Handle {
        let h = self.stack_new(orientation);
        let _ = self.stack_set_spacing(h, spacing);
        self.dress.insert(h, dress);
        h
    }

    /// A box with a title, round things that belong together. It is a stack.
    pub fn group_box_new(&mut self, title: &str, orientation: Orientation) -> Handle {
        self.dressed(orientation, Dress::Group(title.into()), 8)
    }

    pub fn group_box_set_title(&mut self, h: Handle, title: &str) -> Result<()> {
        match self.dress.get_mut(&h) {
            Some(Dress::Group(was)) => {
                *was = title.into();
                self.dirty = true;
                Ok(())
            }
            _ => Err(self.node(h).err().unwrap_or(Error::WrongType)),
        }
    }

    /// A bar of tools: a stack along the top of a window, by custom.
    pub fn toolbar_new(&mut self) -> Handle {
        self.dressed(Orientation::Horizontal, Dress::Toolbar, 2)
    }

    /// A bar of news: a stack along the foot of a window, by custom.
    pub fn status_bar_new(&mut self) -> Handle {
        self.dressed(Orientation::Horizontal, Dress::Status(false), 12)
    }

    /// Have a bar of news keep each thing it says in a cell of its own,
    /// or all on one strip as it does at first.
    pub fn status_bar_set_panels(&mut self, h: Handle, panels: bool) -> Result<()> {
        match self.dress.get_mut(&h) {
            Some(Dress::Status(was)) => {
                *was = panels;
                self.dirty = true;
                Ok(())
            }
            _ => Err(self.node(h).err().unwrap_or(Error::WrongType)),
        }
    }

    /// The class a dressed stack or a text field with choices goes by,
    /// beside the one its kind gives it.
    pub(super) fn dressed_as(&self, h: Handle) -> Option<&'static str> {
        match self.dress.get(&h) {
            Some(Dress::Group(_)) => Some("GroupBox"),
            Some(Dress::Toolbar) => Some("Toolbar"),
            Some(Dress::Status(_)) => Some("StatusBar"),
            None => self.choices.contains_key(&h).then_some("ComboBox"),
        }
    }

    /// The room a stack's dress takes of it: left, top, right, bottom.
    pub(super) fn dress_insets(&mut self, h: Handle) -> (i32, i32, i32, i32) {
        match self.dress.get(&h) {
            Some(Dress::Group(title)) => {
                let top = if title.is_empty() { 8 } else { self.font.line_height() + 4 };
                (9, top, 10, 10)
            }
            Some(Dress::Toolbar) => (4, 3, 4, 5),
            Some(Dress::Status(false)) => (6, 5, 6, 3),
            Some(Dress::Status(true)) => (7, 5, 7, 4),
            None => (0, 0, 0, 0),
        }
    }

    /// Draw a stack's dress, before what is in it.
    pub(super) fn paint_dress(&mut self, h: Handle, rect: Rect, canvas: &mut Canvas) {
        let Some(dress) = self.dress.get(&h).cloned() else { return };
        let (kind, title) = match &dress {
            Dress::Group(title) => (PartKind::GroupBox, title.as_str()),
            Dress::Toolbar => (PartKind::Toolbar, ""),
            Dress::Status(_) => (PartKind::StatusBar, ""),
        };
        let (ground, ink) = (self.ground, self.look.pal.text);
        self.part(canvas, Part::new(kind, rect).text(title).numbers(0, 0, ground as i32).ink(ink));
        if dress == Dress::Status(true) {
            // a cell round each thing it says, the last reaching the end of the bar
            let cells: Vec<Rect> = self.children(h).into_iter().filter_map(|c| self.nodes.get(c).map(|n| n.rect)).filter(|r| r.w > 0).collect();
            for (i, r) in cells.iter().enumerate() {
                let right = if i + 1 == cells.len() { rect.x + rect.w - 2 } else { r.x + r.w + 5 };
                let cell = Rect::new(r.x - 5, rect.y + 3, right - (r.x - 5), rect.h - 4);
                self.part(canvas, Part::new(PartKind::StatusPanel, cell).numbers(i as i32, 0, ground as i32).ink(ink));
            }
        }
    }

    // ------------------------------------------------- buttons and pictures

    /// Have a button lie flat, with no edge of its own until the pointer
    /// is over it or down on it: a tool in a bar.
    pub fn button_set_flat(&mut self, h: Handle, flat: bool) -> Result<()> {
        if !matches!(self.node(h)?.kind, Kind::Button { .. }) {
            return Err(Error::WrongType);
        }
        self.flat.retain(|f| *f != h);
        if flat {
            self.flat.push(h);
        }
        self.dirty = true;
        Ok(())
    }

    /// Give a button a picture, in colour, before its label; none takes it away.
    pub fn button_set_image(&mut self, h: Handle, image: Option<Canvas>) -> Result<()> {
        if !matches!(self.node(h)?.kind, Kind::Button { .. }) {
            return Err(Error::WrongType);
        }
        match image.filter(|i| i.width <= 256 && i.height <= 256) {
            Some(image) => self.images.insert(h, Rc::new(image)),
            None => self.images.remove(&h),
        };
        self.dirty = true;
        Ok(())
    }

    pub(super) fn is_flat(&self, h: Handle) -> bool {
        self.flat.contains(&h)
    }

    /// What a style is given as a button's label: the label itself, with
    /// room before it for the button's picture, which is laid on after.
    pub(super) fn button_text(&mut self, h: Handle, label: &str) -> String {
        let Some(image) = self.images.get(&h).cloned() else { return label.to_string() };
        let need = image.width + if label.is_empty() { 0 } else { 5 };
        let space = self.font.width(" ").max(1);
        format!("{}{label}", " ".repeat(((need + space - 1) / space) as usize))
    }

    /// Lay a button's picture where room was left for it: before its
    /// label, the two together in the middle of the button.
    pub(super) fn paint_button_image(&mut self, h: Handle, rect: Rect, text: &str, canvas: &mut Canvas) {
        let Some(image) = self.images.get(&h).cloned() else { return };
        let wide = self.font.width(text);
        canvas.blit(&image, rect.x + (rect.w - wide) / 2, rect.y + (rect.h - image.height) / 2);
    }

    pub(super) fn button_image_height(&self, h: Handle) -> i32 {
        self.images.get(&h).map_or(0, |i| i.height + 6)
    }

    /// A button lying flat, with nobody near it: its label and no more.
    pub(super) fn paint_flat(&mut self, rect: Rect, text: &str, canvas: &mut Canvas) {
        let (wide, line, color) = (self.font.width(text), self.font.line_height(), self.look.pal.text);
        self.font.draw(canvas, text, rect.x + (rect.w - wide) / 2, rect.y + (rect.h - line) / 2, color);
    }

    // ---------------------------------------------------------------- help

    /// A word of help, shown beside the pointer when it rests on the widget.
    pub fn set_tooltip(&mut self, h: Handle, text: &str) -> Result<()> {
        self.node(h)?;
        if text.is_empty() {
            self.tips.remove(&h);
        } else {
            self.tips.insert(h, text.to_string());
        }
        Ok(())
    }

    /// The pointer has come to something else, or gone, or been pressed:
    /// any help showing is put away, and the wait begins again.
    pub(super) fn tip_reset(&mut self) {
        if self.tip_showing() {
            self.dirty = true;
        }
        self.tip_since = self.now;
        self.tip_off = false;
    }

    /// A press or a key: no help until the pointer has been somewhere else.
    pub(super) fn tip_dismiss(&mut self) {
        if self.tip_showing() {
            self.dirty = true;
        }
        self.tip_off = true;
    }

    fn tip_waiting(&self) -> bool {
        !self.tip_off && self.menu_open.is_none() && self.hover.is_some_and(|h| self.tips.contains_key(&h))
    }

    pub(super) fn tip_showing(&self) -> bool {
        self.tip_waiting() && self.now >= self.tip_since + TIP_MS
    }

    /// When help that is waiting should show.
    pub(super) fn tip_deadline(&self) -> Option<u64> {
        (self.tip_waiting() && self.now < self.tip_since + TIP_MS).then_some(self.tip_since + TIP_MS)
    }

    /// Where the word of help showing over a window goes, and what it
    /// says: below the pointer and to its right, wherever that is. It may
    /// reach past the window, as a menu may.
    pub(super) fn tip_place(&mut self, window: Handle) -> Option<(Rect, String)> {
        let text = self.hover.filter(|h| self.tip_showing() && self.window_of(*h) == Some(window)).and_then(|h| self.tips.get(&h).cloned())?;
        let (wide, tall) = (self.font.width(&text) + 8, self.font.line_height() + 4);
        let at = if self.tip_shown { self.tip_at } else { self.hover_at };
        Some((Rect::new(at.0 + 10, at.1 + 20, wide, tall), text))
    }

    /// The picture of the word of help showing over a window.
    pub(super) fn render_tip(&mut self, window: Handle) -> Option<Canvas> {
        let (rect, text) = self.tip_place(window)?;
        let (ground, ink) = (self.look.pal.tip, self.look.pal.tip_text);
        let mut canvas = Canvas::filled(rect.w, rect.h, crate::canvas::TRANSPARENT);
        self.part(&mut canvas, Part::new(PartKind::Tooltip, Rect::new(0, 0, rect.w, rect.h)).text(&text).numbers(0, 0, ground as i32).ink(ink));
        Some(canvas)
    }

    // ------------------------------------------------------------ steppers

    /// A number with an arrow either side of... beside it, to step it down and up.
    pub fn stepper_new(&mut self) -> Handle {
        self.add(Kind::Spin { value: 0, min: 0, max: 100, step: 1 })
    }

    pub fn stepper_value(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Spin { value, .. } => Ok(*value),
            _ => Err(Error::WrongType),
        }
    }

    /// Set what a stepper is at, or one of its bounds or its step: 0 the
    /// value, 1 the least, 2 the most, 3 the step. Nothing is emitted.
    pub fn stepper_set(&mut self, h: Handle, which: u8, to: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Spin { value, min, max, step } => {
                match which {
                    1 => *min = to.min(*max),
                    2 => *max = to.max(*min),
                    3 if to >= 1 => *step = to,
                    3 => return Err(Error::InvalidArgument),
                    _ => *value = to,
                }
                *value = (*value).clamp(*min, *max);
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub(super) fn measure_stepper(&mut self) -> (i32, i32) {
        let m = self.look.m;
        (STEPPER_FIELD + 1 + m.scroll_bar, (self.font.line_height() + 2 * m.field_inset_y).max(18))
    }

    /// The two arrows of a stepper, one over the other at its right.
    fn stepper_arrows(&self, rect: Rect) -> Rect {
        let thick = self.look.m.scroll_bar;
        Rect::new(rect.x + rect.w - thick, rect.y, thick, rect.h)
    }

    pub(super) fn paint_stepper(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        let Some(Kind::Spin { value, min, max, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (value, min, max) = (*value, *min, *max);
        let arrows = self.stepper_arrows(rect);
        let field = Rect::new(rect.x, rect.y, arrows.x - 1 - rect.x, rect.h);
        self.part(canvas, Part::new(PartKind::Well, field).when(state::FOCUSED, live));
        let (text, ink, line) = (value.to_string(), self.look.pal.ink, self.font.line_height());
        let wide = self.font.width(&text);
        self.font.draw(canvas, &text, field.x + field.w - self.look.m.field_inset_x - wide, field.y + (field.h - line) / 2, ink);
        let held = self.spin_held.filter(|s| s.0 == h).map_or(0, |s| s.1);
        let hot = if self.hover == Some(h) { self.stepper_arrow_at(h, self.hover_at.0, self.hover_at.1) } else { 0 };
        let (ground, words) = (self.ground, self.look.pal.text);
        let mut part = Part::new(PartKind::Stepper, arrows).numbers(held, 0, ground as i32).ink(words).when(state::ENABLED, min < max);
        part.d = hot;
        if self.part_own(canvas, part) {
            return;
        }
        // two of the style's buttons, with an arrow on each
        let half = arrows.h / 2;
        let was = self.painting.take(); // each lights for itself, not for the whole stepper
        let lights = self.look.wants & crate::style::wants::HOVER != 0;
        for (which, r) in [(2, Rect::new(arrows.x, arrows.y, arrows.w, half)), (1, Rect::new(arrows.x, arrows.y + half, arrows.w, arrows.h - half))] {
            self.part(canvas, Part::new(PartKind::Button, r).when(state::HELD, held == which).when(state::HOT, lights && hot == which));
            let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2 - 1);
            let ink = self.ink_on(canvas, cx, r.y + 2, words);
            for d in 0..3 {
                let wide = if which == 2 { 1 + 2 * d } else { 5 - 2 * d };
                canvas.fill(Rect::new(cx - wide / 2, cy + d, wide, 1), ink);
            }
        }
        self.painting = was;
    }

    /// The colour to draw a mark in on a button the style has just drawn:
    /// the colour of words, unless the button there is too near it to show.
    fn ink_on(&self, canvas: &Canvas, x: i32, y: i32, words: u32) -> u32 {
        let light = |c: u32| (((c >> 16) & 0xff) * 3 + ((c >> 8) & 0xff) * 6 + (c & 0xff)) as i32;
        let under = light(canvas.get(x.clamp(0, canvas.width - 1), y.clamp(0, canvas.height - 1)));
        if (under - light(words)).abs() >= 900 { words } else if under > 1275 { BLACK } else { WHITE }
    }

    /// Which arrow of a stepper a point is on: 1 down, 2 up, 0 neither.
    pub(super) fn stepper_arrow_at(&self, h: Handle, x: i32, y: i32) -> i32 {
        let Some(rect) = self.nodes.get(h).map(|n| n.rect) else { return 0 };
        let arrows = self.stepper_arrows(rect);
        if !arrows.contains(x, y) { 0 } else if y < arrows.y + arrows.h / 2 { 2 } else { 1 }
    }

    /// An arrow held down steps again and again: after a pause, then steadily.
    pub(super) fn stepper_tick(&mut self) -> Vec<Emitted> {
        let Some((h, arrow, next)) = self.spin_held else { return Vec::new() };
        if self.now < next {
            return Vec::new();
        }
        self.spin_held = Some((h, arrow, self.now + scroll::REPEAT_MS));
        // only while the pointer is still on the arrow
        if self.stepper_arrow_at(h, self.hover_at.0, self.hover_at.1) != arrow {
            return Vec::new();
        }
        self.stepper_step(h, if arrow == 1 { -1 } else { 1 })
    }

    fn stepper_step(&mut self, h: Handle, by: i32) -> Vec<Emitted> {
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::Spin { value, min, max, step }) => {
                let to = value.saturating_add(by.saturating_mul(*step)).clamp(*min, *max);
                if to == *value {
                    return Vec::new();
                }
                *value = to;
                self.dirty = true;
                vec![Emitted { target: h, event: "Stepper.changed", args: vec![Value::I32(to)] }]
            }
            _ => Vec::new(),
        }
    }

    /// Give a stepper first refusal of a pointer event.
    pub(super) fn stepper_pointer(&mut self, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        if kind == PointerKind::Release && self.spin_held.take().is_some() {
            self.dirty = true;
            return Some(Vec::new());
        }
        let h = over.filter(|h| kind == PointerKind::Press && matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::Spin { .. })))?;
        let arrow = self.stepper_arrow_at(h, x, y);
        if arrow == 0 {
            return Some(Vec::new());
        }
        self.spin_held = Some((h, arrow, self.now + scroll::REPEAT_DELAY_MS));
        self.dirty = true;
        Some(self.stepper_step(h, if arrow == 1 { -1 } else { 1 }))
    }

    /// The arrow keys step a stepper that has the keyboard.
    pub(super) fn stepper_key(&mut self, h: Handle, key: Key) -> Option<Vec<Emitted>> {
        if !matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Spin { .. })) {
            return None;
        }
        match key {
            Key::Up | Key::Right => Some(self.stepper_step(h, 1)),
            Key::Down | Key::Left => Some(self.stepper_step(h, -1)),
            _ => None,
        }
    }

    // --------------------------------------------------------- combo boxes

    /// A text field with a list of things it might say, dropped from a
    /// button at its end. It is a text field.
    pub fn combo_box_new(&mut self, text: &str) -> Handle {
        let h = self.text_field_new(text);
        self.choices.insert(h, Vec::new());
        h
    }

    pub fn combo_box_add(&mut self, h: Handle, choice: &str) -> Result<()> {
        self.node(h)?;
        self.choices.get_mut(&h).ok_or(Error::WrongType)?.push(choice.to_string());
        Ok(())
    }

    pub fn combo_box_clear(&mut self, h: Handle) -> Result<()> {
        self.node(h)?;
        self.choices.get_mut(&h).ok_or(Error::WrongType)?.clear();
        Ok(())
    }

    /// The room a combo box's button takes of its field, at the right.
    pub(super) fn combo_room(&self, h: Handle) -> i32 {
        if self.choices.contains_key(&h) { self.look.m.scroll_bar + 2 } else { 0 }
    }

    fn combo_button(&self, h: Handle) -> Option<Rect> {
        let rect = self.nodes.get(h).filter(|_| self.choices.contains_key(&h))?.rect;
        let side = self.look.m.scroll_bar.min(rect.h - 4);
        Some(Rect::new(rect.x + rect.w - 2 - side, rect.y + (rect.h - side) / 2, side, side))
    }

    /// Draw a combo box's button, over the end of its field.
    pub(super) fn paint_combo(&mut self, h: Handle, canvas: &mut Canvas) {
        let Some(b) = self.combo_button(h) else { return };
        let showing = self.menu_open.as_ref().is_some_and(|m| m.target == Some(h));
        let hot = self.look.wants & crate::style::wants::HOVER != 0 && self.hover == Some(h) && b.contains(self.hover_at.0, self.hover_at.1);
        let (ground, words) = (self.ground, self.look.pal.text);
        if self.part_own(canvas, Part::new(PartKind::ComboButton, b).numbers(0, 0, ground as i32).ink(words).when(state::HELD, showing).when(state::HOT, hot)) {
            return;
        }
        let was = self.painting.take(); // the button is not the field: the pointer over the field does not light it
        self.part(canvas, Part::new(PartKind::Button, b).when(state::HELD, showing));
        self.painting = was;
        // an arrow down, in the colour of words on a button
        let (cx, cy) = (b.x + b.w / 2 + i32::from(showing), b.y + b.h / 2 - 1 + i32::from(showing));
        let ink = self.ink_on(canvas, cx, b.y + 2, self.look.pal.text);
        for d in 0..4 {
            canvas.fill(Rect::new(cx - 3 + d, cy + d, 7 - 2 * d, 1), ink);
        }
    }

    /// A press on a combo box's button drops its list.
    pub(super) fn combo_pointer(&mut self, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        let h = over.filter(|_| kind == PointerKind::Press)?;
        let b = self.combo_button(h).filter(|b| b.contains(x, y))?;
        let (window, rect) = (self.window_of(h)?, self.nodes.get(h)?.rect);
        let choices = self.choices.get(&h).cloned().unwrap_or_default();
        if choices.is_empty() {
            return Some(Vec::new());
        }
        if let Some(old) = self.combo_menu.take() {
            let _ = self.destroy(old);
        }
        let menu = self.menu_new("");
        for (index, label) in choices.iter().enumerate() {
            let item = self.add(Kind::MenuItem { label: label.clone(), checked: false, shortcut: None, separator: false, submenu: None, action: Some(menu::EditAction::Fill(index)) });
            let _ = self.menu_add(menu, item);
        }
        self.combo_menu = Some(menu);
        let _ = b;
        self.open_popup(window, menu, rect.x, rect.y + rect.h - 1, Some(h));
        if let Some(open) = &mut self.menu_open {
            open.min_inner = rect.w - 1 - 2 * self.look.m.menu_border;
        }
        Some(Vec::new())
    }

    /// One of a combo box's choices was picked: the field says that now.
    pub(super) fn combo_fill(&mut self, h: Handle, index: usize) -> Vec<Emitted> {
        let Some(choice) = self.choices.get(&h).and_then(|c| c.get(index)).cloned() else { return Vec::new() };
        if self.text_field_set_text(h, &choice).is_err() {
            return Vec::new();
        }
        let _ = self.text_field_select_all(h);
        vec![self.field_event(h, "TextField.changed")]
    }

    /// Forget what was hung on a widget that is gone.
    pub(super) fn forget(&mut self, h: Handle) {
        self.dress.remove(&h);
        self.images.remove(&h);
        self.tips.remove(&h);
        self.choices.remove(&h);
        self.flat.retain(|f| *f != h);
    }
}
