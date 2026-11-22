// SPDX-License-Identifier: GPL-3.0-or-later
//! Drag and drop.
//!
//! Coming in: text dropped on a text field is inserted where it lands, and
//! files dropped anywhere on a window are reported to the application.
//! Going out: selected text can be dragged from a text field to anywhere
//! that takes text. The engine decides what a drag is and where a drop
//! goes; the display backend does the talking to other programs.

use super::*;

impl Ui {
    /// The enabled text field at a point, if there is one.
    fn field_at(&self, window: Handle, x: i32, y: i32) -> Option<Handle> {
        self.hit(window, x, y).filter(|h| matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::TextField { .. })))
    }

    /// Something is being dragged over the window at `at`, or has left it
    /// (`None`). Returns whether a text field there would take dropped text,
    /// and shows an insertion mark in it while it would.
    pub fn drag_over(&mut self, window: Handle, at: Option<(i32, i32)>) -> bool {
        let mark = at.and_then(|(x, y)| {
            let field = self.field_at(window, x, y)?;
            Some((field, self.index_at(field, x, y)))
        });
        if self.drop_mark != mark {
            self.drop_mark = mark;
            self.dirty = true;
        }
        mark.is_some()
    }

    /// Text was dropped at a point. It goes into the text field there, at
    /// the place it was dropped, and that field takes the keyboard.
    pub fn drop_text(&mut self, window: Handle, x: i32, y: i32, text: &str) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        self.drag_over(window, None);
        let Some(field) = self.field_at(window, x, y) else { return Ok(Vec::new()) };
        let at = self.index_at(field, x, y);
        self.set_focus(window, Some(field), false);
        let edit = self.editor_mut(field)?;
        edit.move_to(at, false);
        Ok(if edit.insert(text) { vec![self.field_event(field, "TextField.changed")] } else { Vec::new() })
    }

    /// Files were dropped on the window. `paths` holds one path per line;
    /// `copy` says the user asked for copies rather than for the files to move.
    pub fn drop_files(&mut self, window: Handle, paths: &str, copy: bool) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        self.drag_over(window, None);
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![Emitted { target: window, event: "Window.files_dropped", args: vec![Value::Str(paths.into()), Value::Bool(copy)] }])
    }

    /// Put something other than text on the clipboard, by its kind.
    pub fn copy_data(&mut self, kind: &str, data: &[u8]) -> Result<()> {
        if !is_kind(kind) {
            return Err(Error::InvalidArgument);
        }
        self.copy_data_out.push((false, kind.to_string(), data.to_vec()));
        Ok(())
    }

    /// Add another kind to what was last copied.
    pub fn copy_also(&mut self, kind: &str, data: &[u8]) -> Result<()> {
        if !is_kind(kind) {
            return Err(Error::InvalidArgument);
        }
        self.copy_data_out.push((true, kind.to_string(), data.to_vec()));
        Ok(())
    }

    /// What the program has copied that is not text, for the backend to
    /// offer, in the order copied: for each, whether it goes with what was
    /// copied before it (or else takes its place), its kind, and the thing.
    pub fn take_copy_data(&mut self) -> Vec<(bool, String, Vec<u8>)> {
        std::mem::take(&mut self.copy_data_out)
    }

    /// A window asks for what is on the clipboard as a kind.
    pub fn window_paste_data(&mut self, window: Handle, kind: &str) -> Result<()> {
        self.check_window(window)?;
        if !is_kind(kind) {
            return Err(Error::InvalidArgument);
        }
        self.paste_data_wanted.push((window, kind.to_string()));
        Ok(())
    }

    /// The windows waiting on the clipboard, and the kinds they asked for.
    pub fn take_paste_data_requests(&mut self) -> Vec<(Handle, String)> {
        std::mem::take(&mut self.paste_data_wanted)
    }

    /// What the clipboard held of the kind a window asked for has arrived.
    pub fn paste_data_into(&mut self, window: Handle, kind: &str, data: Vec<u8>) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        // RTF asked for by a rich text widget is its, not the program's
        if let Some(out) = self.rich_pasted(window, kind, &data) {
            return Ok(out);
        }
        Ok(vec![Emitted { target: window, event: "Window.data_pasted", args: vec![Value::Str(kind.into()), Value::Bytes(data)] }])
    }

    /// A window will take drops of a kind.
    pub fn window_accept_drops(&mut self, window: Handle, kind: &str) -> Result<()> {
        if !is_kind(kind) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(window)?.kind {
            Kind::Window { accepts, .. } => {
                if !accepts.iter().any(|k| k == kind) {
                    accepts.push(kind.to_string());
                }
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// The kinds of thing a window takes drops of, the one liked best first.
    pub fn window_accepts(&self, window: Handle) -> Vec<String> {
        match self.nodes.get(window).map(|n| &n.kind) {
            Some(Kind::Window { accepts, .. }) => accepts.clone(),
            _ => Vec::new(),
        }
    }

    /// Something of a kind the window accepts was dropped on it.
    pub fn drop_data(&mut self, window: Handle, kind: &str, data: Vec<u8>, x: i32, y: i32) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        self.drag_over(window, None);
        Ok(vec![Emitted { target: window, event: "Window.data_dropped", args: vec![Value::Str(kind.into()), Value::Bytes(data), Value::I32(x), Value::I32(y)] }])
    }

    /// The text of a drag the user has just begun, for the display backend
    /// to offer to wherever it is dropped.
    pub fn take_drag(&mut self) -> Option<String> {
        self.drag_out.take()
    }

    /// A picture of text being dragged, to go under the pointer: the text
    /// itself on white inside a dotted outline, as it would look where it
    /// lands. A long piece is cut short, with an ellipsis to say so: at
    /// most three lines, none wider than a small window.
    pub fn text_drag_picture(&mut self, text: &str) -> Option<Canvas> {
        const WIDEST: i32 = 240;
        const LINES: usize = 3;
        let mut lines: Vec<String> = text.lines().map(|l| l.trim_end().replace('\t', "    ")).collect();
        while lines.first().is_some_and(|l| l.trim().is_empty()) {
            lines.remove(0);
        }
        let cut = lines.len() > LINES;
        lines.truncate(LINES);
        if lines.is_empty() {
            return None;
        }
        let last = lines.len() - 1;
        for (n, line) in lines.iter_mut().enumerate() {
            let more = cut && n == last;
            if more || self.font.width(line) > WIDEST {
                // room is left for the ellipsis, which says there was more
                let room = WIDEST - self.font.width("\u{2026}");
                while !line.is_empty() && self.font.width(line) > room {
                    line.pop();
                }
                line.push('\u{2026}');
            }
        }
        let widest = lines.iter().map(|l| self.font.width(l)).max().unwrap_or(0);
        let line_height = self.font.line_height();
        let (w, h) = (widest + 10, lines.len() as i32 * line_height + 6);
        let mut canvas = Canvas::new(w, h);
        canvas.pixels.fill(crate::canvas::TRANSPARENT);
        canvas.fill(Rect::new(1, 1, w - 2, h - 2), WHITE);
        for (n, line) in lines.iter().enumerate() {
            self.font.draw(&mut canvas, line, 5, 3 + n as i32 * line_height, BLACK);
        }
        canvas.ants(Rect::new(0, 0, w, h), 0);
        Some(canvas)
    }
}

/// Put what has been copied since last time onto a clipboard's contents:
/// each either takes the place of everything there or joins it, a kind
/// given twice keeping the later thing.
pub fn settle_copies(held: &mut Vec<(String, Vec<u8>)>, copies: Vec<(bool, String, Vec<u8>)>) {
    for (also, kind, data) in copies {
        if !also {
            held.clear();
        }
        held.retain(|(have, _)| *have != kind);
        held.push((kind, data));
    }
}

/// Whether a name will do for a kind of content: a media type, or one of
/// the older plain names, with nothing in it that could not go on the wire.
fn is_kind(kind: &str) -> bool {
    !kind.is_empty() && kind.len() <= 255 && kind.chars().all(|c| c.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_copied_together_stay_together() {
        let kinds = |held: &[(String, Vec<u8>)]| held.iter().map(|(k, d)| format!("{k}={}", String::from_utf8_lossy(d))).collect::<Vec<_>>();
        let copy = |also: bool, kind: &str, data: &str| (also, kind.to_string(), data.as_bytes().to_vec());
        let mut held = Vec::new();
        settle_copies(&mut held, vec![copy(false, "image/png", "P"), copy(true, "text/plain", "a pear")]);
        assert_eq!(kinds(&held), ["image/png=P", "text/plain=a pear"]);
        // a kind given again keeps the later thing, and goes to the end
        settle_copies(&mut held, vec![copy(true, "image/png", "Q")]);
        assert_eq!(kinds(&held), ["text/plain=a pear", "image/png=Q"]);
        // a fresh copy takes the place of all of it
        settle_copies(&mut held, vec![copy(false, "image/bmp", "B")]);
        assert_eq!(kinds(&held), ["image/bmp=B"]);
    }

    #[test]
    fn other_kinds_of_content_are_copied_pasted_and_dropped() {
        let (mut ui, win, _field) = field_window("");
        let png = vec![0x89, b'P', b'N', b'G', 0, 255];
        // copying: held for the backend, by its kind
        ui.copy_data("image/png", &png).unwrap();
        ui.copy_also("text/plain", b"a pear").unwrap();
        assert_eq!(ui.take_copy_data(), vec![(false, "image/png".to_string(), png.clone()), (true, "text/plain".to_string(), b"a pear".to_vec())]);
        assert!(ui.take_copy_data().is_empty());
        assert_eq!(ui.copy_also("", &png), Err(Error::InvalidArgument));
        assert_eq!(ui.copy_data("", &png), Err(Error::InvalidArgument));
        assert_eq!(ui.copy_data("image png", &png), Err(Error::InvalidArgument));
        // pasting: asked for, and answered with an event carrying the bytes untouched
        ui.window_paste_data(win, "image/png").unwrap();
        assert_eq!(ui.take_paste_data_requests(), vec![(win, "image/png".to_string())]);
        let ev = ui.paste_data_into(win, "image/png", png.clone()).unwrap();
        assert_eq!(ev, vec![Emitted { target: win, event: "Window.data_pasted", args: vec![Value::Str("image/png".into()), Value::Bytes(png.clone())] }]);
        // dropping: only kinds the window has named, in the order it named them
        assert!(ui.window_accepts(win).is_empty());
        ui.window_accept_drops(win, "image/png").unwrap();
        ui.window_accept_drops(win, "image/bmp").unwrap();
        ui.window_accept_drops(win, "image/png").unwrap();
        assert_eq!(ui.window_accepts(win), ["image/png", "image/bmp"]);
        let ev = ui.drop_data(win, "image/png", png.clone(), 12, 34).unwrap();
        assert_eq!(ev[0].event, "Window.data_dropped");
        assert_eq!((ev[0].args[1].as_bytes(), ev[0].args[2].as_i32(), ev[0].args[3].as_i32()), (&png[..], 12, 34));
    }

    fn field_window(text: &str) -> (Ui, Handle, Handle) {
        let mut ui = Ui::new();
        let win = ui.window_new("Drop", 240, 80).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let field = ui.text_field_new(text);
        let label = ui.label_new("Drop a file here");
        ui.stack_add(col, field).unwrap();
        ui.stack_add(col, label).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        (ui, win, field)
    }

    #[test]
    fn dropped_text_lands_where_it_is_dropped() {
        let (mut ui, win, field) = field_window("save file");
        let x = 16 + ui.font.width("save ");
        // hovering shows where it would go, only over the field
        let plain = ui.render(win).unwrap().pixels;
        assert!(ui.drag_over(win, Some((x, 20))));
        assert_ne!(ui.render(win).unwrap().pixels, plain);
        assert!(!ui.drag_over(win, Some((x, 60))));
        assert_eq!(ui.render(win).unwrap().pixels, plain);
        // dropping inserts at that point and reports the new text
        let ev = ui.drop_text(win, x, 20, "the ").unwrap();
        assert_eq!(ev[0].args, vec![Value::Str("save the file".into())]);
        assert_eq!(ui.live_focus(win), Some(field));
        // a drop that misses every field does nothing
        assert!(ui.drop_text(win, x, 60, "lost").unwrap().is_empty());
        // a disabled field takes nothing
        ui.set_enabled(field, false).unwrap();
        assert!(!ui.drag_over(win, Some((x, 20))));
        assert!(ui.drop_text(win, x, 20, "no").unwrap().is_empty());
    }

    #[test]
    fn dropped_files_are_reported_to_the_window() {
        let (mut ui, win, _) = field_window("");
        let ev = ui.drop_files(win, "/tmp/a.txt\n/tmp/b c.txt", false).unwrap();
        assert_eq!(ev, vec![Emitted { target: win, event: "Window.files_dropped", args: vec![Value::Str("/tmp/a.txt\n/tmp/b c.txt".into()), Value::Bool(false)] }]);
        assert!(ui.drop_files(win, "", false).unwrap().is_empty());
    }

    #[test]
    fn dragged_text_has_a_picture_of_itself() {
        use crate::canvas::TRANSPARENT;
        let mut ui = Ui::new();
        let count = |c: &Canvas, colour: u32| c.pixels.iter().filter(|p| **p == colour).count();
        // a word: itself, in black on white, a little larger than the word
        let word = ui.text_drag_picture("pear").unwrap();
        assert_eq!((word.width, word.height), (ui.font.width("pear") + 10, ui.font.line_height() + 6));
        assert!(count(&word, BLACK) > 20 && count(&word, WHITE) > count(&word, BLACK));
        // a long line is cut short, and many lines are cut to three
        let long = ui.text_drag_picture(&"pears and plums ".repeat(40)).unwrap();
        assert!(long.width <= 250 && long.height == word.height);
        let many = ui.text_drag_picture("one\ntwo\nthree\nfour\nfive").unwrap();
        assert_eq!(many.height, 3 * ui.font.line_height() + 6);
        // the last line shown ends in an ellipsis, so it is wider than its word alone
        assert!(many.width > ui.font.width("three") + 10);
        // nothing to show for nothing but space
        assert!(ui.text_drag_picture(" \n ").is_none());
        let _ = TRANSPARENT;
    }

    #[test]
    fn selected_text_can_be_dragged_out() {
        let (mut ui, win, field) = field_window("save the file");
        ui.focus(field).unwrap(); // selects everything
        let x = 16 + ui.font.width("save t");
        // pressing on the selection and moving off starts a drag
        ui.pointer(win, x, 20, PointerKind::Press).unwrap();
        assert_eq!(ui.take_drag(), None);
        ui.pointer(win, x + 2, 21, PointerKind::Move).unwrap();
        assert_eq!(ui.take_drag(), None); // not far enough to be meant
        ui.pointer(win, x + 12, 40, PointerKind::Move).unwrap();
        assert_eq!(ui.take_drag().as_deref(), Some("save the file"));
        assert_eq!(ui.take_drag(), None);
        ui.text(win, "x").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "x"); // the selection survived the drag
        // a plain click on the selection just places the caret
        ui.text_field_set_text(field, "save the file").unwrap();
        ui.text_field_select_all(field).unwrap();
        ui.tick(5_000);
        ui.pointer(win, x, 20, PointerKind::Press).unwrap();
        ui.pointer(win, x, 20, PointerKind::Release).unwrap();
        assert_eq!(ui.take_drag(), None);
        ui.text(win, "!").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "save t!he file");
    }
}
