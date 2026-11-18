// SPDX-License-Identifier: GPL-3.0-or-later
//! The picture: a white box of a size the program chooses, which the
//! program draws on with lines, boxes, dots and words. A graph, a chart,
//! a diagram. What is drawn is kept and drawn again whenever the window
//! is, so the program draws once and draws again only when it changes.
//!
//! The pointer pressed in it, dragged from it and let go, and the wheel
//! turned over it, are told to the program with the place in the picture.

use super::*;

/// How a line is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    /// Every other pixel.
    Dotted,
    /// Four pixels on and three off.
    Dashed,
}

/// One thing drawn on a picture, in the picture's own pixels.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Mark {
    Line { from: (i32, i32), to: (i32, i32), style: LineStyle },
    Box { rect: Rect, filled: bool },
    /// A box filled with a pattern, black and white both: an 8 by 8 tile,
    /// a byte a row, laid from the picture's corner.
    Fill { rect: Rect, pattern: [u8; 8] },
    /// A picture of the program's own, black and white both: rows of
    /// `rect.w` pixels, each row padded to whole bytes, the leftmost pixel
    /// of each byte in its top bit.
    Image { rect: Rect, bits: std::rc::Rc<Vec<u8>> },
    /// A picture in colour: the one exception to black and white, for what
    /// is a photograph by nature, such as a camera's picture. Pixels as the
    /// canvas keeps them, opaque, rows of `rect.w`.
    Photo { rect: Rect, pixels: std::rc::Rc<Vec<u32>> },
    /// What follows is drawn in this colour: lines, boxes and words.
    Ink(u32),
    /// A box filled with one colour, covering what was there as a
    /// pattern does.
    Tint { rect: Rect, color: u32 },
    /// Words, in the picture's typeface unless they have one of their own.
    Words { x: i32, y: i32, text: String, face: Option<Face> },
}

/// The most marks a picture keeps; more than this is a program that has
/// forgotten to clear.
const MOST_MARKS: usize = 100_000;

/// A pointer button other than the left, which a picture may ask to hear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OtherButton {
    Right,
    Middle,
}

impl Ui {
    pub fn picture_new(&mut self, width: i32, height: i32) -> Result<Handle> {
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
            return Err(Error::InvalidArgument);
        }
        Ok(self.add(Kind::Picture { width, height, marks: Vec::new(), description: String::new(), keys: false, raw: false, right: false, middle: false, shown: (0, 0) }))
    }

    fn marks_mut(&mut self, h: Handle) -> Result<&mut Vec<Mark>> {
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { marks, .. } => Ok(marks),
            _ => Err(Error::WrongType),
        }
    }

    fn mark(&mut self, h: Handle, mark: Mark) -> Result<()> {
        let marks = self.marks_mut(h)?;
        if marks.len() >= MOST_MARKS {
            return Err(Error::InvalidState);
        }
        marks.push(mark);
        self.dirty = true;
        Ok(())
    }

    /// Rub everything out.
    pub fn picture_clear(&mut self, h: Handle) -> Result<()> {
        self.marks_mut(h)?.clear();
        self.dirty = true;
        Ok(())
    }

    pub fn picture_line(&mut self, h: Handle, x0: i32, y0: i32, x1: i32, y1: i32, style: LineStyle) -> Result<()> {
        self.mark(h, Mark::Line { from: (x0, y0), to: (x1, y1), style })
    }

    pub fn picture_box(&mut self, h: Handle, x: i32, y: i32, w: i32, h_: i32, filled: bool) -> Result<()> {
        if w < 0 || h_ < 0 {
            return Err(Error::InvalidArgument);
        }
        self.mark(h, Mark::Box { rect: Rect::new(x, y, w, h_), filled })
    }

    /// Draw what follows in a colour, 0xRRGGBB: lines, boxes outlined and
    /// filled, and words. A picture draws in black until told otherwise,
    /// and again once it is cleared.
    pub fn picture_set_color(&mut self, h: Handle, rgb: u32) -> Result<()> {
        self.mark(h, Mark::Ink(0xff00_0000 | rgb))
    }

    /// Fill a box with one colour, 0xRRGGBB. It covers what was there, as
    /// a pattern does, and what is drawn after lies on it.
    pub fn picture_fill_color(&mut self, h: Handle, x: i32, y: i32, w: i32, h_: i32, rgb: u32) -> Result<()> {
        if w < 0 || h_ < 0 {
            return Err(Error::InvalidArgument);
        }
        self.mark(h, Mark::Tint { rect: Rect::new(x, y, w, h_), color: 0xff00_0000 | rgb })
    }

    /// Fill a box with a pattern: eight bytes, a row of the tile each, the
    /// leftmost pixel in the top bit. A set bit is black and a clear one
    /// white, so that this covers what was drawn there before; all noughts
    /// rubs the box out. The tile is laid from the picture's corner, so
    /// boxes side by side in one pattern join without a seam.
    pub fn picture_fill(&mut self, h: Handle, x: i32, y: i32, w: i32, h_: i32, pattern: &[u8]) -> Result<()> {
        let Ok(pattern) = <[u8; 8]>::try_from(pattern) else { return Err(Error::InvalidArgument) };
        if w < 0 || h_ < 0 {
            return Err(Error::InvalidArgument);
        }
        self.mark(h, Mark::Fill { rect: Rect::new(x, y, w, h_), pattern })
    }

    /// Lay down a picture of the program's own making, a pixel a bit: rows
    /// of `w` pixels, each padded to whole bytes, the leftmost pixel in the
    /// top bit of a byte, set for black. It covers what was there, as a
    /// fill does. For a scene drawn pixel by pixel (shading, a photograph
    /// made one-bit, a game's world) this is one call where dots would be
    /// thousands.
    pub fn picture_image(&mut self, h: Handle, x: i32, y: i32, w: i32, h_: i32, bits: &[u8]) -> Result<()> {
        if w <= 0 || h_ <= 0 || w > 4096 || h_ > 4096 || bits.len() != ((w as usize).div_ceil(8)) * h_ as usize {
            return Err(Error::InvalidArgument);
        }
        self.mark(h, Mark::Image { rect: Rect::new(x, y, w, h_), bits: std::rc::Rc::new(bits.to_vec()) })
    }

    /// Lay down a picture in colour: rows of `w` pixels, three bytes each,
    /// red, green and blue. press is black and white; this is for what is a
    /// photograph by nature, such as a camera's picture, and covers what
    /// was there, as an image does.
    pub fn picture_photo(&mut self, h: Handle, x: i32, y: i32, w: i32, h_: i32, rgb: &[u8]) -> Result<()> {
        if w <= 0 || h_ <= 0 || w > 4096 || h_ > 4096 || rgb.len() != w as usize * h_ as usize * 3 {
            return Err(Error::InvalidArgument);
        }
        let pixels = rgb.chunks_exact(3).map(|p| 0xff00_0000 | u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])).collect();
        self.mark(h, Mark::Photo { rect: Rect::new(x, y, w, h_), pixels: std::rc::Rc::new(pixels) })
    }

    /// Let the picture have the keyboard: Tab stops at it, a press in it
    /// takes the keyboard there, and the arrow keys and what is typed are
    /// told to the program.
    pub fn picture_set_takes_keys(&mut self, h: Handle, on: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { keys, .. } => {
                *keys = on;
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Let the picture hear every key, for a terminal or an emulator: it
    /// takes the keyboard, and each key comes as `key_down` with the
    /// modifiers held, Tab, Return, Escape, the function keys and Control
    /// or Alt with a character among them. What is typed without them
    /// still comes as `typed`.
    pub fn picture_set_raw_keys(&mut self, h: Handle, on: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { keys, raw, .. } => {
                *raw = on;
                *keys = *keys || on;
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub(super) fn picture_hears_all(&self, h: Handle) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Picture { raw: true, .. }))
    }

    /// The number the interface gives a key, or 0 for a character.
    pub(super) fn key_code(key: Key) -> i32 {
        match key {
            Key::Tab => 1,
            Key::Return => 2,
            Key::Escape => 3,
            Key::Backspace => 4,
            Key::Delete => 5,
            Key::Left => 6,
            Key::Right => 7,
            Key::Up => 8,
            Key::Down => 9,
            Key::Home => 10,
            Key::End => 11,
            Key::Menu => 12,
            Key::Context => 13,
            Key::PageUp => 14,
            Key::PageDown => 15,
            Key::Insert => 16,
            Key::F(n) => 20 + n as i32,
            Key::Char(_) => 0,
        }
    }

    /// A key for a picture that hears every key: its number (0 for a
    /// character), the character, and the modifier bits.
    pub(super) fn picture_key_down(&self, h: Handle, key: Key, mods: Mods) -> Emitted {
        let ch = match key {
            Key::Char(c) => c as i32,
            _ => 0,
        };
        let bits = mods.shift as i32 | (mods.ctrl as i32) << 1 | (mods.alt as i32) << 2;
        Emitted { target: h, event: "Picture.key_down", args: vec![Value::I32(Self::key_code(key)), Value::I32(ch), Value::I32(bits)] }
    }

    /// Change the picture's size. What is drawn is kept, and what now falls
    /// outside is not shown.
    pub fn picture_set_size(&mut self, h: Handle, width: i32, height: i32) -> Result<()> {
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { width: w, height: h_, .. } => {
                (*w, *h_) = (width, height);
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Lay out the windows showing and tell each picture told to stretch
    /// whose room has changed size since it was last told: `resized`.
    pub fn layout_events(&mut self) -> Vec<Emitted> {
        let pictures: Vec<Handle> = self.nodes.handles().into_iter().filter(|h| self.nodes.get(*h).is_some_and(|n| n.stretch == Some(true) && matches!(n.kind, Kind::Picture { .. }))).collect();
        let watched = !pictures.is_empty();
        if !watched {
            return Vec::new();
        }
        for info in self.windows().into_iter().filter(|w| w.visible) {
            self.arrange(info.handle, Rect::new(0, 0, info.width, info.height));
        }
        let mut out = Vec::new();
        for h in pictures {
            let Some(node) = self.nodes.get_mut(h) else { continue };
            let (w, h_) = (node.rect.w, node.rect.h);
            if node.stretch != Some(true) || w <= 0 || h_ <= 0 {
                continue;
            }
            if let Kind::Picture { shown, .. } = &mut node.kind {
                if *shown != (w, h_) {
                    *shown = (w, h_);
                    out.push(Emitted { target: h, event: "Picture.resized", args: vec![Value::I32(w), Value::I32(h_)] });
                }
            }
        }
        out
    }

    pub(super) fn picture_takes_keys(&self, h: Handle) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Picture { keys: true, .. }))
    }

    /// A key pressed while a picture has the keyboard.
    pub(super) fn picture_key(&self, h: Handle, key: Key) -> Option<Vec<Emitted>> {
        if !self.picture_takes_keys(h) {
            return None;
        }
        // the numbers the interface gives the keys
        let code = match key {
            Key::Backspace => 4,
            Key::Delete => 5,
            Key::Left => 6,
            Key::Right => 7,
            Key::Up => 8,
            Key::Down => 9,
            Key::Home => 10,
            Key::End => 11,
            _ => return None,
        };
        Some(vec![Emitted { target: h, event: "Picture.key_pressed", args: vec![Value::I32(code)] }])
    }

    /// Words with the top of their line at `y`, in the picture's typeface.
    pub fn picture_text(&mut self, h: Handle, x: i32, y: i32, text: &str) -> Result<()> {
        self.mark(h, Mark::Words { x, y, text: text.to_string(), face: None })
    }

    /// Words in a typeface of their own, whatever the picture's.
    pub fn picture_text_in(&mut self, h: Handle, x: i32, y: i32, text: &str, face: Face) -> Result<()> {
        self.mark(h, Mark::Words { x, y, text: text.to_string(), face: Some(face) })
    }

    /// How wide words would be drawn in a given typeface.
    pub fn picture_text_width_in(&mut self, h: Handle, text: &str, face: Face) -> Result<i32> {
        if !matches!(self.node(h)?.kind, Kind::Picture { .. }) {
            return Err(Error::WrongType);
        }
        Ok(self.wearing(face, |ui| ui.font.width(text)))
    }

    /// How tall a line of words is in a given typeface.
    pub fn picture_line_height_in(&mut self, h: Handle, face: Face) -> Result<i32> {
        if !matches!(self.node(h)?.kind, Kind::Picture { .. }) {
            return Err(Error::WrongType);
        }
        Ok(self.wearing(face, |ui| ui.font.line_height()))
    }

    /// How wide words would be drawn in the picture, so that they can be
    /// placed: centred under a mark, or kept clear of an edge.
    pub fn picture_text_width(&mut self, h: Handle, text: &str) -> Result<i32> {
        if !matches!(self.node(h)?.kind, Kind::Picture { .. }) {
            return Err(Error::WrongType);
        }
        Ok(self.in_face(h, |ui| ui.font.width(text)))
    }

    /// How tall a line of words is in the picture's typeface.
    pub fn picture_line_height(&mut self, h: Handle) -> Result<i32> {
        if !matches!(self.node(h)?.kind, Kind::Picture { .. }) {
            return Err(Error::WrongType);
        }
        Ok(self.in_face(h, |ui| ui.font.line_height()))
    }

    /// What the picture shows, in words, for those who cannot see it.
    pub fn picture_set_description(&mut self, h: Handle, text: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { description, .. } => {
                *description = text.to_string();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub(super) fn measure_picture(&self, h: Handle) -> (i32, i32) {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Picture { width, height, .. }) => (*width, *height),
            _ => (0, 0),
        }
    }

    pub(super) fn paint_picture(&mut self, h: Handle, rect: Rect, ring: Option<i32>, canvas: &mut Canvas) {
        let Some(Kind::Picture { marks, raw, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        // a picture that hears every key shows where they go itself, as a
        // terminal does with its cursor, and is not ringed
        let ring = ring.filter(|_| !*raw);
        let marks = marks.clone();
        canvas.fill(rect, WHITE);
        // with the keyboard, it is ringed as a button is
        if let Some(phase) = ring {
            self.part(canvas, Part::new(PartKind::Focus, rect.inset(-RING_GAP)).phase(phase));
        }
        // patterns and images first lay down both black and white, in the order asked for
        for mark in &marks {
            match mark {
                Mark::Fill { rect: r, pattern } => {
                    for y in r.y.max(0)..(r.y + r.h).min(rect.h) {
                        let row = pattern[(y & 7) as usize];
                        for x in r.x.max(0)..(r.x + r.w).min(rect.w) {
                            canvas.set(rect.x + x, rect.y + y, if row & (0x80 >> (x & 7)) != 0 { BLACK } else { WHITE });
                        }
                    }
                }
                Mark::Image { rect: r, bits } => {
                    let stride = (r.w as usize).div_ceil(8);
                    for y in r.y.max(0)..(r.y + r.h).min(rect.h) {
                        let row = &bits[(y - r.y) as usize * stride..][..stride];
                        for x in r.x.max(0)..(r.x + r.w).min(rect.w) {
                            let i = (x - r.x) as usize;
                            canvas.set(rect.x + x, rect.y + y, if row[i / 8] & (0x80 >> (i % 8)) != 0 { BLACK } else { WHITE });
                        }
                    }
                }
                Mark::Tint { rect: r, color } => {
                    for y in r.y.max(0)..(r.y + r.h).min(rect.h) {
                        for x in r.x.max(0)..(r.x + r.w).min(rect.w) {
                            canvas.set(rect.x + x, rect.y + y, *color);
                        }
                    }
                }
                Mark::Photo { rect: r, pixels } => {
                    for y in r.y.max(0)..(r.y + r.h).min(rect.h) {
                        let row = &pixels[(y - r.y) as usize * r.w as usize..][..r.w as usize];
                        for x in r.x.max(0)..(r.x + r.w).min(rect.w) {
                            canvas.set(rect.x + x, rect.y + y, row[(x - r.x) as usize]);
                        }
                    }
                }
                _ => {}
            }
        }
        // everything is kept inside the picture's own box
        let color = std::cell::Cell::new(BLACK);
        let mut ink = |x: i32, y: i32| {
            if (0..rect.w).contains(&x) && (0..rect.h).contains(&y) {
                canvas.set(rect.x + x, rect.y + y, color.get());
            }
        };
        let mut words = Vec::new();
        for mark in &marks {
            match mark {
                Mark::Ink(to) => color.set(*to),
                Mark::Line { from, to, style } => line(*from, *to, *style, &mut ink),
                Mark::Box { rect: r, filled: true } => {
                    for y in r.y..r.y + r.h {
                        for x in r.x..r.x + r.w {
                            ink(x, y);
                        }
                    }
                }
                Mark::Box { rect: r, filled: false } if r.w > 0 && r.h > 0 => {
                    let (x1, y1) = (r.x + r.w - 1, r.y + r.h - 1);
                    line((r.x, r.y), (x1, r.y), LineStyle::Solid, &mut ink);
                    line((r.x, y1), (x1, y1), LineStyle::Solid, &mut ink);
                    line((r.x, r.y), (r.x, y1), LineStyle::Solid, &mut ink);
                    line((x1, r.y), (x1, y1), LineStyle::Solid, &mut ink);
                }
                Mark::Box { .. } | Mark::Fill { .. } | Mark::Image { .. } | Mark::Photo { .. } | Mark::Tint { .. } => {}
                Mark::Words { x, y, text, face } => words.push((*x, *y, text.clone(), *face, color.get())),
            }
        }
        if !words.is_empty() {
            // words are drawn on a sheet the picture's size, then laid on, so
            // that they too stop at its edges; white is left as it was
            let mut sheet = Canvas::new(rect.w, rect.h);
            for y in 0..rect.h {
                for x in 0..rect.w {
                    sheet.set(x, y, canvas.get(rect.x + x, rect.y + y));
                }
            }
            let own = self.nodes.get(h).and_then(|n| n.face).unwrap_or_default();
            for (x, y, text, face, color) in &words {
                self.wearing(face.unwrap_or(own), |ui| ui.font.draw(&mut sheet, text, *x, *y, *color));
            }
            for y in 0..rect.h {
                for x in 0..rect.w {
                    canvas.set(rect.x + x, rect.y + y, sheet.get(x, y));
                }
            }
        }
    }

    /// The pointer pressed in a picture, or a picture being dragged in.
    /// It keeps the pointer from the press until it is let go, wherever
    /// the pointer goes, and is told where it is in its own pixels.
    pub(super) fn picture_pointer(&mut self, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        let place = |ui: &Ui, h: Handle| {
            let r = ui.nodes.get(h).map(|n| n.rect).unwrap_or_default();
            vec![Value::I32(x - r.x), Value::I32(y - r.y)]
        };
        if let Some(h) = self.picture_drag {
            return Some(match kind {
                PointerKind::Move => vec![Emitted { target: h, event: "Picture.dragged", args: place(self, h) }],
                PointerKind::Release => {
                    self.picture_drag = None;
                    vec![Emitted { target: h, event: "Picture.released", args: place(self, h) }]
                }
                _ => Vec::new(),
            });
        }
        let h = over.filter(|h| matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::Picture { .. })))?;
        if kind != PointerKind::Press {
            return None;
        }
        self.picture_drag = Some(h);
        if self.picture_takes_keys(h) {
            if let Some(window) = self.window_of(h) {
                if self.resolve_focus(window) != Some(h) {
                    self.set_focus(window, Some(h), false);
                }
            }
        }
        let mut args = place(self, h);
        args.push(Value::I32(self.clicks_at(x, y)));
        Some(vec![Emitted { target: h, event: "Picture.pressed", args }])
    }

    /// Let the picture hear the right button, as a remote desktop or a game
    /// must: pressed in it, it is told so with right_pressed and not
    /// popped up a context menu; where the pointer goes while it is held
    /// comes by dragged, and letting go by right_released.
    pub fn picture_set_right_button(&mut self, h: Handle, on: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { right, .. } => {
                *right = on;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Let the picture hear the middle button, as right_button does the
    /// right: middle_pressed, dragged while it is held, and middle_released.
    pub fn picture_set_middle_button(&mut self, h: Handle, on: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Picture { middle, .. } => {
                *middle = on;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// The right or middle button went down over a widget: the events, if
    /// it is a picture that hears that button.
    pub(super) fn picture_button_press(&mut self, h: Handle, x: i32, y: i32, button: OtherButton) -> Option<Vec<Emitted>> {
        let r = self
            .nodes
            .get(h)
            .filter(|n| match (&n.kind, button) {
                (Kind::Picture { right, .. }, OtherButton::Right) => *right,
                (Kind::Picture { middle, .. }, OtherButton::Middle) => *middle,
                _ => false,
            })?
            .rect;
        *self.button_hold(button) = Some(h);
        // the pointer is heard wherever it goes until the button comes up
        self.picture_drag = Some(h);
        if self.picture_takes_keys(h) {
            if let Some(window) = self.window_of(h) {
                if self.resolve_focus(window) != Some(h) {
                    self.set_focus(window, Some(h), false);
                }
            }
        }
        let event = match button {
            OtherButton::Right => "Picture.right_pressed",
            OtherButton::Middle => "Picture.middle_pressed",
        };
        Some(vec![Emitted { target: h, event, args: vec![Value::I32(x - r.x), Value::I32(y - r.y)] }])
    }

    fn button_hold(&mut self, button: OtherButton) -> &mut Option<Handle> {
        match button {
            OtherButton::Right => &mut self.picture_right,
            OtherButton::Middle => &mut self.picture_middle,
        }
    }

    /// The right or middle button came up at a point in a window: told to
    /// the picture it went down in, if that picture hears it.
    fn button_release(&mut self, window: Handle, x: i32, y: i32, button: OtherButton) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        let Some(h) = self.button_hold(button).take() else { return Ok(Vec::new()) };
        if self.picture_drag == Some(h) {
            self.picture_drag = None;
        }
        let Some(r) = self.nodes.get(h).map(|n| n.rect) else { return Ok(Vec::new()) };
        let event = match button {
            OtherButton::Right => "Picture.right_released",
            OtherButton::Middle => "Picture.middle_released",
        };
        Ok(vec![Emitted { target: h, event, args: vec![Value::I32(x - r.x), Value::I32(y - r.y)] }])
    }

    pub fn right_release(&mut self, window: Handle, x: i32, y: i32) -> Result<Vec<Emitted>> {
        self.button_release(window, x, y, OtherButton::Right)
    }

    pub fn middle_release(&mut self, window: Handle, x: i32, y: i32) -> Result<Vec<Emitted>> {
        self.button_release(window, x, y, OtherButton::Middle)
    }

    /// The wheel turned over a picture: `lines` down, or up if negative.
    pub(super) fn picture_wheel(&self, h: Handle, x: i32, y: i32, lines: i32) -> Option<Vec<Emitted>> {
        let node = self.nodes.get(h).filter(|n| matches!(n.kind, Kind::Picture { .. }))?;
        let r = node.rect;
        Some(vec![Emitted { target: h, event: "Picture.scrolled", args: vec![Value::I32(lines), Value::I32(x - r.x), Value::I32(y - r.y)] }])
    }
}

/// The pixels of a line from one end to the other, both ends included.
fn line(from: (i32, i32), to: (i32, i32), style: LineStyle, ink: &mut impl FnMut(i32, i32)) {
    let (mut x, mut y) = from;
    let (dx, dy) = ((to.0 - x).abs(), -(to.1 - y).abs());
    let (sx, sy) = (if x < to.0 { 1 } else { -1 }, if y < to.1 { 1 } else { -1 });
    let mut err = dx + dy;
    // a line wholly outside any sensible picture is not walked pixel by pixel
    if dx > 1 << 16 || -dy > 1 << 16 {
        return;
    }
    let mut step = 0;
    loop {
        let on = match style {
            LineStyle::Solid => true,
            LineStyle::Dotted => step % 2 == 0,
            LineStyle::Dashed => step % 7 < 4,
        };
        if on {
            ink(x, y);
        }
        if (x, y) == to {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        step += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn black(canvas: &Canvas, r: Rect) -> i32 {
        let mut n = 0;
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                n += (canvas.get(x, y) == BLACK) as i32;
            }
        }
        n
    }

    fn setup() -> (Ui, Handle, Handle) {
        let mut ui = Ui::natural();
        let win = ui.window_new("w", 120, 80).unwrap();
        let pic = ui.picture_new(100, 60).unwrap();
        let column = ui.stack_new(Orientation::Vertical);
        ui.stack_add(column, pic).unwrap();
        ui.window_set_content(win, column).unwrap();
        (ui, win, pic)
    }

    #[test]
    fn a_picture_that_hears_every_key_gets_them_all() {
        let (mut ui, win, pic) = setup();
        let bar = ui.menu_bar_new();
        let edit = ui.menu_new("Edit");
        let copy = ui.menu_item_new("Copy");
        ui.menu_item_set_shortcut(copy, "c").unwrap();
        ui.menu_add(edit, copy).unwrap();
        ui.menu_bar_add(bar, edit).unwrap();
        ui.window_set_menu_bar(win, bar).unwrap();
        ui.picture_set_raw_keys(pic, true).unwrap();
        ui.focus(pic).unwrap();
        let ctrl = Mods { ctrl: true, ..Mods::default() };
        let told = |ev: Vec<Emitted>| -> (Handle, &'static str, Vec<i32>) { (ev[0].target, ev[0].event, ev[0].args.iter().map(Value::as_i32).collect()) };
        // Control-C is the program's, not the menu's
        assert_eq!(told(ui.key(win, Key::Char('c'), ctrl).unwrap()), (pic, "Picture.key_down", vec![0, 'c' as i32, 2]));
        // with Shift as well it chooses the entry
        assert_eq!(ui.key(win, Key::Char('c'), Mods { shift: true, ..ctrl }).unwrap()[0].target, copy);
        assert_eq!(told(ui.key(win, Key::Tab, Mods::default()).unwrap()).2, vec![1, 0, 0]);
        assert_eq!(told(ui.key(win, Key::Menu, Mods::default()).unwrap()).2, vec![30, 0, 0]);
        assert_eq!(told(ui.key(win, Key::F(5), Mods { alt: true, ..Mods::default() }).unwrap()).2, vec![25, 0, 4]);
        assert_eq!(told(ui.key(win, Key::PageUp, Mods::default()).unwrap()).2, vec![14, 0, 0]);
        // and typing still comes as typed
        assert_eq!(ui.text(win, "x").unwrap()[0].event, "Picture.typed");
    }

    #[test]
    fn a_stretching_picture_is_told_its_room() {
        let mut ui = Ui::new();
        let win = ui.window_new("w", 120, 80).unwrap();
        let pic = ui.picture_new(100, 60).unwrap();
        let column = ui.stack_new(Orientation::Vertical);
        ui.stack_add(column, pic).unwrap();
        ui.window_set_content(win, column).unwrap();
        ui.set_stretch(pic, true).unwrap();
        ui.picture_set_size(pic, 20, 10).unwrap();
        // not until it is showing
        assert!(ui.layout_events().is_empty());
        ui.window_show(win).unwrap();
        let ev = ui.layout_events();
        assert_eq!(ev.len(), 1);
        assert_eq!((ev[0].target, ev[0].event), (pic, "Picture.resized"));
        let first: Vec<i32> = ev[0].args.iter().map(Value::as_i32).collect();
        assert_eq!(first, vec![120, 80]);
        // told once, and again when the window changes size
        assert!(ui.layout_events().is_empty());
        ui.window_resize(win, 300, 200).unwrap();
        let ev = ui.layout_events();
        assert_eq!(ev[0].args.iter().map(Value::as_i32).collect::<Vec<_>>(), vec![300, 200]);
        assert_eq!(ui.picture_set_size(pic, 0, 5), Err(Error::InvalidArgument));
    }

    #[test]
    fn a_picture_draws_in_colour_when_it_is_told_to() {
        let (mut ui, win, pic) = setup();
        let r = { ui.render(win).unwrap(); ui.nodes.get(pic).unwrap().rect };
        // black until told otherwise; a filled box in one colour lies under the lines
        ui.picture_line(pic, 0, 0, 9, 0, LineStyle::Solid).unwrap();
        ui.picture_set_color(pic, 0xcc2200).unwrap();
        ui.picture_line(pic, 0, 2, 9, 2, LineStyle::Solid).unwrap();
        ui.picture_box(pic, 20, 20, 4, 4, true).unwrap();
        ui.picture_fill_color(pic, 0, 2, 5, 5, 0x2060c0).unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(c.get(r.x + 3, r.y), BLACK);
        assert_eq!(c.get(r.x + 3, r.y + 2), 0xffcc2200);
        assert_eq!(c.get(r.x + 21, r.y + 21), 0xffcc2200);
        assert_eq!(c.get(r.x + 3, r.y + 4), 0xff2060c0);
        // cleared, it draws in black again
        ui.picture_clear(pic).unwrap();
        ui.picture_line(pic, 0, 0, 9, 0, LineStyle::Solid).unwrap();
        assert_eq!(ui.render(win).unwrap().get(r.x + 3, r.y), BLACK);
    }

    #[test]
    fn what_is_drawn_stays_inside_and_clears() {
        let (mut ui, win, pic) = setup();
        let r = { ui.render(win).unwrap(); ui.nodes.get(pic).unwrap().rect };
        assert_eq!((r.w, r.h), (100, 60));
        ui.picture_line(pic, 0, 0, 9, 0, LineStyle::Solid).unwrap();
        ui.picture_line(pic, 0, 2, 9, 2, LineStyle::Dotted).unwrap();
        ui.picture_line(pic, 0, 4, 13, 4, LineStyle::Dashed).unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(black(&c, Rect::new(r.x, r.y, 10, 1)), 10);
        assert_eq!(black(&c, Rect::new(r.x, r.y + 2, 10, 1)), 5);
        assert_eq!(black(&c, Rect::new(r.x, r.y + 4, 14, 1)), 8);
        // a line running far out of the picture is cut at its edge
        ui.picture_clear(pic).unwrap();
        ui.picture_line(pic, -500, 30, 500, 30, LineStyle::Solid).unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(black(&c, Rect::new(0, 0, 120, 80)), 100);
        assert_eq!(black(&c, Rect::new(r.x, r.y + 30, 100, 1)), 100);
        // boxes, outlined and filled
        ui.picture_clear(pic).unwrap();
        ui.picture_box(pic, 10, 10, 5, 4, false).unwrap();
        ui.picture_box(pic, 30, 10, 5, 4, true).unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(black(&c, Rect::new(r.x + 10, r.y + 10, 5, 4)), 14);
        assert_eq!(black(&c, Rect::new(r.x + 30, r.y + 10, 5, 4)), 20);
        ui.picture_clear(pic).unwrap();
        assert_eq!(black(&ui.render(win).unwrap(), Rect::new(0, 0, 120, 80)), 0);
    }

    #[test]
    fn words_are_drawn_and_measured() {
        let (mut ui, win, pic) = setup();
        let wide = ui.picture_text_width(pic, "1.5").unwrap();
        assert!(wide > 0 && ui.picture_line_height(pic).unwrap() > 0);
        ui.picture_text(pic, 2, 2, "1.5").unwrap();
        let c = ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        assert!(black(&c, Rect::new(r.x, r.y, wide + 4, 20)) > 5);
        // words that run past the edge stop there
        ui.picture_clear(pic).unwrap();
        ui.picture_text(pic, 95, 2, "WWWW").unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(black(&c, Rect::new(r.x + r.w, 0, 120 - r.x - r.w, 80)), 0);
        assert_eq!(ui.picture_text_width(win, "x"), Err(Error::WrongType));
    }

    #[test]
    fn a_picture_that_hears_the_right_button_is_told_of_it() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        // by default the right button is the context menu's, and a picture hears nothing
        assert!(ui.context(win, r.x + 5, r.y + 7).unwrap().is_empty());
        assert!(ui.right_release(win, r.x + 5, r.y + 7).unwrap().is_empty());
        ui.picture_set_right_button(pic, true).unwrap();
        let ev = ui.context(win, r.x + 5, r.y + 7).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.right_pressed", args: vec![Value::I32(5), Value::I32(7)] }]);
        // held, the pointer is heard wherever it goes
        let ev = ui.pointer(win, r.x + 200, r.y - 3, PointerKind::Move).unwrap();
        assert_eq!(ev[0].event, "Picture.dragged");
        assert_eq!(ev[0].args, vec![Value::I32(200), Value::I32(-3)]);
        let ev = ui.right_release(win, r.x + 9, r.y + 2).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.right_released", args: vec![Value::I32(9), Value::I32(2)] }]);
        assert!(ui.pointer(win, r.x + 2, r.y + 2, PointerKind::Move).unwrap().is_empty());
        assert!(ui.right_release(win, r.x + 9, r.y + 2).unwrap().is_empty());
    }

    #[test]
    fn a_picture_that_hears_the_middle_button_is_told_of_it() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        assert!(ui.middle_press(win, r.x + 5, r.y + 7).unwrap().is_empty());
        ui.picture_set_middle_button(pic, true).unwrap();
        let ev = ui.middle_press(win, r.x + 5, r.y + 7).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.middle_pressed", args: vec![Value::I32(5), Value::I32(7)] }]);
        assert_eq!(ui.pointer(win, r.x + 20, r.y + 3, PointerKind::Move).unwrap()[0].event, "Picture.dragged");
        // the right button's release is not the middle's
        assert!(ui.right_release(win, r.x + 20, r.y + 3).unwrap().is_empty());
        let ev = ui.middle_release(win, r.x + 20, r.y + 3).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.middle_released", args: vec![Value::I32(20), Value::I32(3)] }]);
        // the right button stays the context menu's on a picture that hears only the middle
        assert!(ui.context(win, r.x + 5, r.y + 7).unwrap().is_empty());
    }

    #[test]
    fn the_pointer_is_told_in_the_pictures_own_pixels() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        let ev = ui.pointer(win, r.x + 5, r.y + 7, PointerKind::Press).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.pressed", args: vec![Value::I32(5), Value::I32(7), Value::I32(1)] }]);
        // dragged out of the picture, it still hears where the pointer is
        let ev = ui.pointer(win, r.x + 200, r.y - 3, PointerKind::Move).unwrap();
        assert_eq!(ev[0].event, "Picture.dragged");
        assert_eq!(ev[0].args, vec![Value::I32(200), Value::I32(-3)]);
        let ev = ui.pointer(win, r.x + 1, r.y + 1, PointerKind::Release).unwrap();
        assert_eq!(ev[0].event, "Picture.released");
        assert!(ui.pointer(win, r.x + 2, r.y + 2, PointerKind::Move).unwrap().is_empty());
        let ev = ui.wheel(win, r.x + 3, r.y + 4, -2).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.scrolled", args: vec![Value::I32(-2), Value::I32(3), Value::I32(4)] }]);
        // a second press soon after, in the same place, is a double click
        ui.now += 10_000;
        ui.pointer(win, r.x + 5, r.y + 7, PointerKind::Press).unwrap();
        ui.pointer(win, r.x + 5, r.y + 7, PointerKind::Release).unwrap();
        let ev = ui.pointer(win, r.x + 5, r.y + 7, PointerKind::Press).unwrap();
        assert_eq!(ev[0].args[2], Value::I32(2));
    }

    #[test]
    fn patterns_fill_and_cover() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        // a checkerboard is half black
        ui.picture_fill(pic, 0, 0, 16, 16, &[0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55]).unwrap();
        assert_eq!(black(&ui.render(win).unwrap(), Rect::new(r.x, r.y, 16, 16)), 128);
        // all noughts covers what was there with white
        ui.picture_fill(pic, 0, 0, 8, 16, &[0; 8]).unwrap();
        assert_eq!(black(&ui.render(win).unwrap(), Rect::new(r.x, r.y, 16, 16)), 64);
        // lines drawn are on top of patterns, whichever was asked for first
        ui.picture_line(pic, 0, 0, 7, 0, LineStyle::Solid).unwrap();
        assert_eq!(black(&ui.render(win).unwrap(), Rect::new(r.x, r.y, 8, 1)), 8);
        assert_eq!(ui.picture_fill(pic, 0, 0, 4, 4, &[1, 2, 3]), Err(Error::InvalidArgument));
    }

    #[test]
    fn images_are_laid_down_bit_for_bit() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        // ten pixels across is two bytes a row; a diagonal of black
        let bits: Vec<u8> = (0..10).flat_map(|y: usize| {
            let mut row = [0u8; 2];
            row[y / 8] = 0x80 >> (y % 8);
            row
        }).collect();
        ui.picture_fill(pic, 0, 0, 10, 10, &[0xFF; 8]).unwrap();
        ui.picture_image(pic, 0, 0, 10, 10, &bits).unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(black(&c, Rect::new(r.x, r.y, 10, 10)), 10);
        assert!((0..10).all(|i| c.get(r.x + i, r.y + i) == BLACK));
        // one cut off at the picture's edge is drawn as far as it goes
        ui.picture_clear(pic).unwrap();
        ui.picture_image(pic, 95, 55, 10, 10, &[0xFF; 20]).unwrap();
        assert_eq!(black(&ui.render(win).unwrap(), Rect::new(0, 0, 120, 80)), 25);
        assert_eq!(ui.picture_image(pic, 0, 0, 10, 10, &[0; 19]), Err(Error::InvalidArgument));
    }

    #[test]
    fn photos_are_laid_down_in_colour() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        // two by two: red, green; blue, a grey
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        ui.picture_photo(pic, 3, 4, 2, 2, &rgb).unwrap();
        // a line drawn first is still on top
        ui.picture_line(pic, 0, 5, 10, 5, LineStyle::Solid).unwrap();
        let c = ui.render(win).unwrap();
        assert_eq!(c.get(r.x + 3, r.y + 4), 0xffff_0000);
        assert_eq!(c.get(r.x + 4, r.y + 4), 0xff00_ff00);
        assert_eq!(c.get(r.x + 3, r.y + 5), BLACK, "lines are drawn over photos");
        // cut off at the picture's edge, and refused when the size is wrong
        ui.picture_photo(pic, 99, 59, 2, 2, &rgb).unwrap();
        assert_eq!(ui.render(win).unwrap().get(r.x + 99, r.y + 59), 0xffff_0000);
        assert_eq!(ui.picture_photo(pic, 0, 0, 2, 2, &rgb[..9]), Err(Error::InvalidArgument));
        assert_eq!(ui.picture_photo(pic, 0, 0, 0, 2, &[]), Err(Error::InvalidArgument));
    }

    #[test]
    fn a_picture_can_have_the_keyboard() {
        let (mut ui, win, pic) = setup();
        ui.render(win).unwrap();
        ui.window_set_active(win, true).unwrap();
        // not unless it asks
        assert_eq!(ui.focus(pic), Err(Error::WrongType));
        assert!(ui.key(win, Key::Left, Mods::default()).unwrap().is_empty());
        ui.picture_set_takes_keys(pic, true).unwrap();
        ui.focus(pic).unwrap();
        let ev = ui.key(win, Key::Left, Mods::default()).unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.key_pressed", args: vec![Value::I32(6)] }]);
        let ev = ui.text(win, "u").unwrap();
        assert_eq!(ev, vec![Emitted { target: pic, event: "Picture.typed", args: vec![Value::Str("u".into())] }]);
    }

    #[test]
    fn a_picture_is_described_and_sized_sensibly() {
        let mut ui = Ui::natural();
        assert_eq!(ui.picture_new(0, 10), Err(Error::InvalidArgument));
        assert_eq!(ui.picture_new(10, 5000), Err(Error::InvalidArgument));
        let pic = ui.picture_new(10, 10).unwrap();
        ui.picture_set_description(pic, "a graph of sin x").unwrap();
        assert_eq!(ui.picture_box(pic, 0, 0, -1, 2, true), Err(Error::InvalidArgument));
        ui.set_typeface(pic, Face::Agate).unwrap();
    }

    #[test]
    fn words_can_be_drawn_in_a_typeface_of_their_own() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Faces", 300, 120).unwrap();
        let pic = ui.picture_new(280, 100).unwrap();
        ui.window_set_content(win, pic).unwrap();
        let text = "Wide words";
        let plain = ui.picture_text_width(pic, text).unwrap();
        let elite = ui.picture_text_width_in(pic, text, Face::Elite).unwrap();
        assert!(elite > plain);
        assert!(ui.picture_line_height_in(pic, Face::Elite).unwrap() > ui.picture_line_height_in(pic, Face::Agate).unwrap());
        // drawn in its own face, it is as wide as that face says
        ui.picture_text_in(pic, 0, 0, text, Face::Elite).unwrap();
        let canvas = ui.render(win).unwrap();
        let r = ui.nodes.get(pic).unwrap().rect;
        let inked: Vec<i32> = (0..280).filter(|x| (0..30).any(|y| canvas.get(r.x + x, r.y + y) == BLACK)).collect();
        let drawn = inked.last().unwrap() - inked.first().unwrap() + 1;
        assert!(drawn > plain && drawn <= elite, "{drawn} {plain} {elite}");
        assert_eq!(ui.picture_text_width_in(win, text, Face::Elite), Err(Error::WrongType));
    }
}
