// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing and hit-testing for editable text: the one-line text field, its
//! password form, and the multi-line text area.
//!
//! All three are the same thing seen as a list of visual lines. A one-line
//! field has a single line and scrolls sideways. A text area wraps its text
//! at word gaps into as many lines as it takes and scrolls up and down. A
//! password field shows a bullet for each character.

use super::*;
use unicode_segmentation::UnicodeSegmentation;

/// The two ends of a selection, each a line and a character in it: where
/// it began, and where the caret is.
pub(super) type Ends = ((usize, usize), (usize, usize));

/// A visual line: a byte range of the text. A wrapped line ends where the
/// next begins; the last line of a paragraph ends at its line break.
type Line = (usize, usize);

/// How far past the end of a line a selection that includes the line break
/// is drawn.
const BREAK_WIDTH: i32 = 4;

/// The settings of a field that decide how its text is laid out.
#[derive(Clone, Copy)]
pub(super) struct Shape {
    multiline: bool,
    password: bool,
    scroll: i32,
    follow: bool,
}

impl Ui {
    pub(super) fn shape_of(&self, h: Handle) -> Option<(Shape, Rect)> {
        let node = self.nodes.get(h)?;
        match &node.kind {
            Kind::TextField { multiline, password, scroll, follow, .. } => {
                let mut inner = field_inner(node.rect, &self.look.m);
                if *multiline {
                    // a text area keeps its right-hand side for the scroll bar
                    inner.w = (Self::area_bar_rect(node.rect, self.look.m.scroll_bar).x - 2 - inner.x).max(0);
                }
                Some((Shape { multiline: *multiline, password: *password, scroll: *scroll, follow: *follow }, inner))
            }
            _ => None,
        }
    }

    /// What is drawn for a piece of text: itself, or bullets for a password.
    fn masked(text: &str, password: bool) -> String {
        if password {
            "\u{2022}".repeat(text.chars().count())
        } else {
            text.to_string()
        }
    }

    /// Break text into visual lines no wider than `width`, at word gaps
    /// where there are any. A one-line field is always a single line.
    pub(super) fn wrap(&mut self, text: &str, width: i32, multiline: bool) -> Vec<Line> {
        if !multiline {
            return vec![(0, text.len())];
        }
        let mut out = Vec::new();
        let mut start = 0;
        for paragraph in text.split('\n') {
            let end = start + paragraph.len();
            let (mut line_start, mut x, mut gap) = (start, 0, None::<usize>);
            for (i, ch) in paragraph.char_indices() {
                let (at, advance) = (start + i, self.font.advance(ch));
                if x + advance > width && at > line_start {
                    // break after the last space on the line, or mid-word if it has none
                    let cut = gap.filter(|g| *g > line_start).unwrap_or(at);
                    out.push((line_start, cut));
                    x = self.font.width(&text[cut..at]);
                    line_start = cut;
                    gap = None;
                }
                x += advance;
                if ch == ' ' {
                    gap = Some(at + ch.len_utf8());
                }
            }
            out.push((line_start, end));
            start = end + 1; // past the line break
        }
        out
    }

    /// The visual line a position is on. A position where one line wraps
    /// into the next belongs to the next.
    fn line_of(lines: &[Line], at: usize) -> usize {
        lines.iter().rposition(|(start, _)| *start <= at).unwrap_or(0)
    }

    /// The position in a line nearest to a horizontal offset from its start.
    fn index_in_line(&mut self, text: &str, lines: &[Line], row: usize, x: i32, password: bool) -> usize {
        let (start, end) = lines[row];
        let wraps = lines.get(row + 1).is_some_and(|next| next.0 == end);
        let mut pen = 0;
        for (i, ch) in text[start..end].char_indices() {
            let advance = self.font.advance(if password { '\u{2022}' } else { ch });
            if x < pen + (advance + 1) / 2 {
                return start + i;
            }
            pen += advance;
        }
        // The very end of a wrapped line is the start of the next one; stay
        // on this line by stopping before the space it wrapped at.
        if wraps && text[start..end].ends_with(' ') && end - 1 > start {
            end - 1
        } else {
            end
        }
    }

    /// The character boundary in a text field nearest to a point in the window.
    /// The character boundary in a text field nearest to a point in the window.
    pub(super) fn index_at(&mut self, h: Handle, x: i32, y: i32) -> usize {
        self.in_face(h, |ui| ui.index_at_in(h, x, y))
    }

    fn index_at_in(&mut self, h: Handle, x: i32, y: i32) -> usize {
        let Some((shape, inner)) = self.shape_of(h) else { return 0 };
        let Some(Kind::TextField { edit, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return 0 };
        let text = edit.text().to_string();
        let lines = self.wrap(&text, inner.w - 2, shape.multiline);
        let line = self.font.line_height();
        let (row, x) = if shape.multiline {
            ((shape.scroll + (y - inner.y).div_euclid(line)).clamp(0, lines.len() as i32 - 1) as usize, x - inner.x - 1)
        } else {
            (0, x - inner.x - 1 + shape.scroll)
        };
        self.index_in_line(&text, &lines, row, x, shape.password)
    }

    /// Move the caret of a text area up or down a visual line, keeping to
    /// the column it started from as far as the lines allow.
    pub(super) fn field_vertical(&mut self, h: Handle, down: bool, extend: bool) -> Result<()> {
        self.in_face(h, |ui| ui.field_vertical_in(h, down, extend))
    }

    fn field_vertical_in(&mut self, h: Handle, down: bool, extend: bool) -> Result<()> {
        let Some((shape, inner)) = self.shape_of(h) else { return Err(Error::WrongType) };
        let (text, caret, goal) = match &self.node(h)?.kind {
            Kind::TextField { edit, goal_x, .. } => (edit.text().to_string(), edit.caret(), *goal_x),
            _ => return Err(Error::WrongType),
        };
        let lines = self.wrap(&text, inner.w - 2, shape.multiline);
        let row = Self::line_of(&lines, caret);
        let goal = goal.unwrap_or_else(|| self.font.width(&text[lines[row].0..caret]));
        let target = match (down, row) {
            (false, 0) => 0, // up from the top line goes to the very start
            (true, r) if r + 1 >= lines.len() => text.len(),
            (false, r) => self.index_in_line(&text, &lines, r - 1, goal, false),
            (true, r) => self.index_in_line(&text, &lines, r + 1, goal, false),
        };
        self.editor_mut(h)?.move_to(target, extend);
        if let Kind::TextField { goal_x, .. } = &mut self.node_mut(h)?.kind {
            *goal_x = Some(goal); // remembered across a run of ups and downs
        }
        Ok(())
    }

    /// Move the caret to the start or end of its visual line.
    pub(super) fn field_line_edge(&mut self, h: Handle, end: bool, extend: bool) -> Result<()> {
        self.in_face(h, |ui| ui.field_line_edge_in(h, end, extend))
    }

    fn field_line_edge_in(&mut self, h: Handle, end: bool, extend: bool) -> Result<()> {
        let Some((shape, inner)) = self.shape_of(h) else { return Err(Error::WrongType) };
        let (text, caret) = match &self.node(h)?.kind {
            Kind::TextField { edit, .. } => (edit.text().to_string(), edit.caret()),
            _ => return Err(Error::WrongType),
        };
        let lines = self.wrap(&text, inner.w - 2, shape.multiline);
        let row = Self::line_of(&lines, caret);
        let target = if end { self.index_in_line(&text, &lines, row, i32::MAX, false) } else { lines[row].0 };
        self.editor_mut(h)?.move_to(target, extend);
        Ok(())
    }

    /// Turn the mouse wheel over a text area or a scroll bar: `lines` down,
    /// or up if negative. A scroll bar reports the change.
    pub fn wheel(&mut self, window: Handle, x: i32, y: i32, lines: i32) -> Result<Vec<Emitted>> {
        self.check_window(window)?;
        // whatever is under the pointer has first claim: a text area, a
        // scroll bar. Otherwise the scroll view the pointer is in.
        let target = self.hit(window, x, y).filter(|h| !matches!(self.nodes.get(*h).map(|n| &n.kind), Some(Kind::ScrollView { .. })));
        let scrollable = target.filter(|h| [false, true].into_iter().any(|horizontal| self.bar_of(*h, horizontal).is_some()));
        if let Some(out) = target.and_then(|h| self.picture_wheel(h, x, y, lines)) {
            return Ok(out);
        }
        if let Some(out) = target.and_then(|h| self.slider_wheel(h, lines)) {
            return Ok(out);
        }
        Ok(match scrollable.or_else(|| self.view_at(window, x, y)) {
            Some(h) => self.bar_wheel(h, lines),
            None => Vec::new(),
        })
    }

    /// The part of a text field that holds its text.
    pub(super) fn text_rect(&self, h: Handle) -> Option<Rect> {
        self.shape_of(h).map(|(_, inner)| inner)
    }

    /// A field's text and the visual lines it is drawn as.
    fn field_lines(&mut self, h: Handle) -> Option<(String, Vec<Line>, Shape, Rect)> {
        let (shape, inner) = self.shape_of(h)?;
        let Some(Kind::TextField { edit, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        let text = edit.text().to_string();
        let lines = self.wrap(&text, inner.w - 2, shape.multiline);
        Some((text, lines, shape, inner))
    }

    /// A field's text as assistive technology is given it: line by line as
    /// drawn, each character with its place, and the two ends of the
    /// selection as a line and a character in it.
    pub(super) fn field_runs(&mut self, h: Handle) -> (Vec<AccessRun>, Option<Ends>) {
        self.in_face(h, |ui| ui.field_runs_in(h))
    }

    fn field_runs_in(&mut self, h: Handle) -> (Vec<AccessRun>, Option<Ends>) {
        let Some((text, lines, shape, inner)) = self.field_lines(h) else { return (Vec::new(), None) };
        let Some(Kind::TextField { edit, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return (Vec::new(), None) };
        let (anchor, caret) = (edit.anchor(), edit.caret());
        let line_height = self.font.line_height();
        let mut runs = Vec::new();
        for (row, &(start, end)) in lines.iter().enumerate() {
            let mut piece = text[start..end].to_string();
            let (mut chars, mut word_starts, mut x, mut spaced) = (Vec::new(), Vec::new(), 0, true);
            for (i, whole) in piece.graphemes(true).enumerate() {
                let width: i32 = whole.chars().map(|ch| self.font.advance(ch)).sum();
                chars.push((whole.len().min(255) as u8, x, width));
                x += width;
                // a word begins at the first thing after a space, and takes the spaces after it
                let space = whole.chars().all(char::is_whitespace);
                if spaced && !space && i < 256 {
                    word_starts.push(i as u8);
                }
                spaced = space;
            }
            // a line that ends where the user pressed Return ends in that break
            if text[end..].starts_with('\n') {
                piece.push('\n');
                chars.push((1, x, 0));
            }
            let rect = if shape.multiline {
                Rect::new(inner.x + 1, inner.y + (row as i32 - shape.scroll) * line_height, x, line_height)
            } else {
                Rect::new(inner.x + 1 - shape.scroll, inner.y + (inner.h - line_height) / 2, x, line_height)
            };
            runs.push(AccessRun { rect, text: piece, chars, word_starts });
        }
        let place = |at: usize| {
            let row = Self::line_of(&lines, at);
            let (start, end) = lines[row];
            (row, text[start..at.clamp(start, end)].graphemes(true).count())
        };
        (runs, Some((place(anchor), place(caret))))
    }

    /// The place in a field's text of a line and a character in it.
    pub(super) fn field_offset(&mut self, h: Handle, at: (usize, usize)) -> usize {
        self.in_face(h, |ui| ui.field_offset_in(h, at))
    }

    fn field_offset_in(&mut self, h: Handle, (row, index): (usize, usize)) -> usize {
        let Some((text, lines, ..)) = self.field_lines(h) else { return 0 };
        let Some(&(start, end)) = lines.get(row.min(lines.len().saturating_sub(1))) else { return 0 };
        start + text[start..end].graphemes(true).take(index).map(str::len).sum::<usize>()
    }

    /// Draw a text field of any kind.
    pub(super) fn paint_field(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        let Some((shape, inner)) = self.shape_of(h) else { return };
        let Some(Kind::TextField { edit, placeholder, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (edit, placeholder) = (edit.clone(), placeholder.clone());
        let line = self.font.line_height();
        let (paper, ink) = (self.look.pal.paper, self.look.pal.ink);
        self.part(canvas, Part::new(PartKind::Well, rect).numbers(if shape.multiline { 2 } else { 0 }, 0, 0).when(state::FOCUSED, live));
        if inner.w <= 0 || inner.h <= 0 {
            return;
        }
        // Text being composed by an input method is shown where it will go:
        // at the caret, or in place of the selection.
        let pre = self.preedit.as_ref().filter(|(f, _)| *f == h).map(|(_, t)| t.clone()).unwrap_or_default();
        let (from, to) = if pre.is_empty() { (edit.caret(), edit.caret()) } else { edit.selection() };
        let shown = format!("{}{}{}", &edit.text()[..from], pre, &edit.text()[to..]);
        let caret = from + pre.len();
        let lines = self.wrap(&shown, inner.w - 2, shape.multiline);
        let caret_row = Self::line_of(&lines, caret);
        // the offset of a position from the start of the line it is drawn on
        let offset = |ui: &mut Ui, row: usize, at: usize| ui.font.width(&Self::masked(&shown[lines[row].0..at.clamp(lines[row].0, lines[row].1)], shape.password));
        let caret_x = offset(self, caret_row, caret);

        let mut scroll = shape.scroll;
        let (first, rows, slide) = if shape.multiline {
            // scroll by whole lines: no further than the text goes, and far
            // enough to show the caret when it has just moved
            let rows = (inner.h / line).max(1);
            scroll = scroll.clamp(0, (lines.len() as i32 - rows).max(0));
            if shape.follow {
                scroll = scroll.min(caret_row as i32).max(caret_row as i32 - rows + 1);
            }
            (scroll as usize, rows as usize, 0)
        } else {
            // scroll sideways. The text sits one pixel in, leaving a column
            // for the caret so it never lands on top of a glyph.
            let total = offset(self, 0, shown.len());
            scroll = scroll.min((total + 2 - inner.w).max(0));
            scroll = scroll.min(caret_x).max(caret_x - (inner.w - 1));
            (0, 1, scroll)
        };

        // Draw on a canvas of the inner size so that what overflows is clipped.
        let mut clip = Canvas::filled(inner.w, inner.h, paper);
        let top = |row: usize| if shape.multiline { (row - first) as i32 * line } else { (inner.h - line) / 2 };
        if shown.is_empty() {
            self.font.draw(&mut clip, &placeholder, 1, top(first), ink);
            self.part(&mut clip, Part::new(PartKind::Dim, Rect::new(0, 0, inner.w, inner.h)));
        }
        let selection = (live && pre.is_empty() && edit.has_selection()).then(|| edit.selection());
        let mark = self.drop_mark.filter(|(f, _)| *f == h).map(|(_, at)| at.min(shown.len()));
        for row in first..lines.len().min(first + rows) {
            let (start, end) = lines[row];
            let y = top(row);
            // the band this line occupies: the whole height of a one-line field
            let band = if shape.multiline { Rect::new(0, y, inner.w, line) } else { Rect::new(0, 0, inner.w, inner.h) };
            let drawn = Self::masked(&shown[start..end], shape.password);
            self.font.draw(&mut clip, &drawn, 1 - slide, y, ink);
            if !pre.is_empty() && from <= end && caret >= start {
                // underlined, the usual sign of text not yet committed
                let (x0, x1) = (offset(self, row, from.max(start)), offset(self, row, caret.min(end)));
                clip.fill(Rect::new(x0 + 1 - slide, band.y + band.h - 1, x1 - x0, 1), ink);
            }
            if mark.is_some_and(|at| Self::line_of(&lines, at) == row) {
                // where dragged text would land
                let x = offset(self, row, mark.unwrap_or(0));
                clip.fill(Rect::new(x - slide, band.y, 2, band.h), ink);
            }
            if let Some((sel_from, sel_to)) = selection.filter(|(f, t)| *f <= end && *t >= start) {
                let (x0, mut x1) = (offset(self, row, sel_from.max(start)), offset(self, row, sel_to.min(end)));
                if sel_to > end {
                    x1 += BREAK_WIDTH; // the selection runs on past this line
                }
                // the text sits one pixel in: the band starts in that margin and runs
                // to the last column of the last letter, which is one further
                self.part(&mut clip, Part::new(PartKind::Select, Rect::new(x0 - slide, band.y, x1 - x0 + 1, band.h)).numbers(1, 0, 0));
            }
            if live && selection.is_none() && row == caret_row && self.caret_on() {
                self.part(&mut clip, Part::new(PartKind::Caret, Rect::new(caret_x - slide, band.y, 1, band.h)).ink(ink));
            }
        }
        canvas.blit(&clip, inner.x, inner.y);
        if let Some(Kind::TextField { scroll: stored, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *stored = scroll;
        }
        if shape.multiline {
            self.bar_draw(h, false, canvas);
        }
    }

    /// Where the caret is drawn, in window coordinates.
    pub(super) fn caret_rect(&mut self, h: Handle) -> Option<Rect> {
        self.in_face(h, |ui| ui.caret_rect_in(h))
    }

    fn caret_rect_in(&mut self, h: Handle) -> Option<Rect> {
        let (shape, inner) = self.shape_of(h)?;
        let Some(Kind::TextField { edit, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        let (text, from) = (edit.text().to_string(), edit.selection().0);
        let pre = self.preedit.as_ref().filter(|(f, _)| *f == h).map(|(_, t)| t.clone()).unwrap_or_default();
        let lines = self.wrap(&text, inner.w - 2, shape.multiline);
        let row = Self::line_of(&lines, from);
        let x = self.font.width(&Self::masked(&text[lines[row].0..from], shape.password)) + self.font.width(&pre);
        let line = self.font.line_height();
        Some(if shape.multiline {
            Rect::new(inner.x + x, inner.y + (row as i32 - shape.scroll) * line, 1, line)
        } else {
            Rect::new(inner.x + x - shape.scroll, inner.y, 1, inner.h)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };
    const SHIFT: Mods = Mods { shift: true, ctrl: false, alt: false };
    const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false };

    /// A window holding a text area 200 pixels wide and `rows` lines high,
    /// over a default button.
    fn area(text: &str, rows: i32) -> (Ui, Handle, Handle, Handle) {
        let mut ui = Ui::natural();
        let win = ui.window_new("Notes", 240, 200).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let area = ui.text_area_new(text);
        ui.text_field_set_min_width(area, 200).unwrap();
        ui.text_area_set_rows(area, rows).unwrap();
        let ok = ui.button_new("OK");
        ui.button_set_default(ok, true).unwrap();
        ui.stack_add(col, area).unwrap();
        ui.stack_add(col, ok).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        (ui, win, area, ok)
    }

    fn caret(ui: &Ui, h: Handle) -> usize {
        match &ui.nodes.get(h).unwrap().kind {
            Kind::TextField { edit, .. } => edit.caret(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn text_wraps_at_word_gaps_and_keeps_line_breaks() {
        let mut ui = Ui::new();
        let text = "the quick brown fox jumps\nover\n\nthe lazy dog";
        let width = ui.font.width("the quick brown ") + 2;
        let lines = ui.wrap(text, width, true);
        let shown: Vec<&str> = lines.iter().map(|(a, b)| &text[*a..*b]).collect();
        assert_eq!(shown, ["the quick brown ", "fox jumps", "over", "", "the lazy dog"]);
        // every byte is on exactly one line, bar the line breaks themselves
        assert_eq!(lines.iter().map(|(a, b)| b - a).sum::<usize>(), text.len() - 3);
        // a word too long for the line is broken where it must be
        let narrow = ui.font.width("abcdefgh");
        let long = ui.wrap("abcdefghijklmnopqrstuvwxyz", narrow, true);
        assert!(long.len() > 2 && long.iter().all(|(a, b)| b > a));
        // a one-line field never wraps
        assert_eq!(ui.wrap(text, 10, false), [(0, text.len())]);
        // text ending in a line break has an empty last line for the caret
        assert_eq!(ui.wrap("a\n", 100, true), [(0, 1), (2, 2)]);
    }

    #[test]
    fn return_starts_a_new_line_in_a_text_area() {
        let (mut ui, win, area, _ok) = area("", 4);
        ui.text(win, "one").unwrap();
        let ev = ui.key(win, Key::Return, NONE).unwrap();
        assert_eq!(ev, vec![Emitted { target: area, event: "TextField.changed", args: vec![Value::Str("one\n".into())] }]);
        assert!(ui.finish_animations().is_empty()); // the default button was not pressed
        ui.text(win, "two").unwrap();
        assert_eq!(ui.text_field_text(area).unwrap(), "one\ntwo");
        // pasted text keeps its lines, whatever line endings it came with, and its tabs
        ui.text_field_select_all(area).unwrap();
        ui.paste_into(area, "a\r\nb\rc\td").unwrap();
        assert_eq!(ui.text_field_text(area).unwrap(), "a\nb\nc\td");
        // it is a text field as far as type checks go, with a class of its own
        assert_eq!(ui.class_of(area).unwrap(), "TextArea");
        assert!(ui.is_a(area, "TextField").unwrap() && ui.is_a(area, "TextArea").unwrap());
        let field = ui.text_field_new("");
        assert!(!ui.is_a(field, "TextArea").unwrap());
        assert_eq!(ui.text_area_set_rows(field, 3), Err(Error::WrongType));
        assert_eq!(ui.text_field_set_password(area, true), Err(Error::InvalidState));
    }

    #[test]
    fn up_and_down_move_by_drawn_lines_and_keep_their_column() {
        let (mut ui, win, area, _) = area("a long first line here\nab\nthe third line is long", 4);
        ui.key(win, Key::Home, CTRL).unwrap();
        for _ in 0..12 {
            ui.key(win, Key::Right, NONE).unwrap();
        }
        assert_eq!(caret(&ui, area), 12);
        ui.key(win, Key::Down, NONE).unwrap();
        assert_eq!(caret(&ui, area), "a long first line here\nab".len()); // the short line: as far as it goes
        ui.key(win, Key::Down, NONE).unwrap();
        let third = "a long first line here\nab\n".len();
        let x12 = ui.font.width("a long first");
        let landed = caret(&ui, area) - third;
        // back out at the column it started in, not the short line's
        assert!((ui.font.width(&"the third line is long"[..landed]) - x12).abs() <= 6, "{landed}");
        ui.key(win, Key::Up, NONE).unwrap();
        ui.key(win, Key::Up, NONE).unwrap();
        assert_eq!(caret(&ui, area), 12);
        // Home and End work on the line; with Control, on the whole text
        ui.key(win, Key::End, NONE).unwrap();
        assert_eq!(caret(&ui, area), "a long first line here".len());
        ui.key(win, Key::Home, NONE).unwrap();
        assert_eq!(caret(&ui, area), 0);
        ui.key(win, Key::End, CTRL).unwrap();
        assert_eq!(caret(&ui, area), ui.text_field_text(area).unwrap().len());
        // up from the top line goes to the start, down from the bottom to the end
        ui.key(win, Key::Down, NONE).unwrap();
        assert_eq!(caret(&ui, area), ui.text_field_text(area).unwrap().len());
        ui.key(win, Key::Home, CTRL).unwrap();
        ui.key(win, Key::Right, NONE).unwrap();
        ui.key(win, Key::Up, SHIFT).unwrap();
        ui.text(win, "A").unwrap(); // the selection made by Shift+Up is replaced
        assert!(ui.text_field_text(area).unwrap().starts_with("A long"));
    }

    #[test]
    fn a_click_lands_on_the_line_under_the_pointer() {
        let (mut ui, win, area, _) = area("first\nsecond\nthird", 4);
        let line = ui.font.line_height();
        // the area's text starts at (16, 13)
        let x = 16 + ui.font.width("sec");
        ui.pointer(win, x, 13 + line + 3, PointerKind::Press).unwrap();
        ui.pointer(win, x, 13 + line + 3, PointerKind::Release).unwrap();
        assert_eq!(caret(&ui, area), "first\nsec".len());
        // dragging down a line selects across the line break
        ui.tick(5_000);
        ui.pointer(win, x, 13 + 3, PointerKind::Press).unwrap();
        ui.pointer(win, x, 13 + 2 * line + 3, PointerKind::Move).unwrap();
        ui.pointer(win, x, 13 + 2 * line + 3, PointerKind::Release).unwrap();
        ui.key(win, Key::Char('c'), CTRL).unwrap();
        let copied = ui.take_copy().unwrap();
        // from part-way along the first line to part-way along the third
        assert!(copied.contains("\nsecond\n") && copied.len() < "first\nsecond\nthird".len(), "{copied:?}");
        assert!("first".ends_with(copied.split('\n').next().unwrap()) && "third".starts_with(copied.rsplit('\n').next().unwrap()));
        // the I-beam shows over it, as over any text
        assert_eq!(ui.cursor_at(win, x, 13 + 3), crate::cursor::Cursor::Text);
    }

    #[test]
    fn long_text_scrolls_with_the_caret_and_the_wheel() {
        let text: Vec<String> = (1..=10).map(|n| format!("line {n}")).collect();
        let (mut ui, win, area, _) = area(&text.join("\n"), 3);
        let line = ui.font.line_height();
        let scroll = |ui: &Ui| match &ui.nodes.get(area).unwrap().kind {
            Kind::TextField { scroll, .. } => *scroll,
            _ => unreachable!(),
        };
        // the caret starts at the end (the text is selected), so the end shows
        ui.key(win, Key::End, CTRL).unwrap();
        ui.render(win).unwrap();
        assert_eq!(scroll(&ui), 7);
        // the wheel moves the view without moving the caret...
        ui.wheel(win, 30, 20, -5).unwrap();
        ui.render(win).unwrap();
        assert_eq!((scroll(&ui), caret(&ui, area)), (2, ui.text_field_text(area).unwrap().len()));
        ui.wheel(win, 30, 20, -50).unwrap();
        ui.render(win).unwrap();
        assert_eq!(scroll(&ui), 0); // ...and stops at the top
        // a click then lands on the line that is showing there
        ui.pointer(win, 16, 13 + line + 2, PointerKind::Press).unwrap();
        ui.pointer(win, 16, 13 + line + 2, PointerKind::Release).unwrap();
        assert_eq!(caret(&ui, area), "line 1\n".len());
        // typing brings the view back to the caret
        ui.wheel(win, 30, 20, 50).unwrap();
        ui.render(win).unwrap();
        assert_eq!(scroll(&ui), 7);
        ui.text(win, "x").unwrap();
        ui.render(win).unwrap();
        assert_eq!(scroll(&ui), 1); // just far enough to show the caret's line, the second
        // the wheel does nothing to a one-line field or a button
        ui.wheel(win, 30, 90, 3).unwrap();
        // nothing drawn spills below the area into the window
        let canvas = ui.render(win).unwrap();
        assert!(canvas.pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }

    #[test]
    fn a_password_shows_bullets_and_stays_put() {
        let mut ui = Ui::new();
        let win = ui.window_new("Login", 200, 60).unwrap();
        let field = ui.text_field_new("");
        ui.window_set_content(win, field).unwrap();
        ui.text(win, "hunter2").unwrap();
        let plain = ui.render(win).unwrap().pixels;
        ui.text_field_set_password(field, true).unwrap();
        let hidden = ui.render(win).unwrap().pixels;
        assert_ne!(plain, hidden);
        // what is drawn depends only on how many characters there are
        ui.text_field_set_text(field, "abcdefg").unwrap();
        assert_eq!(ui.render(win).unwrap().pixels, hidden);
        assert_eq!(ui.text_field_text(field).unwrap(), "abcdefg"); // the program can still read it
        // it cannot be copied or cut
        ui.text_field_select_all(field).unwrap();
        ui.key(win, Key::Char('c'), CTRL).unwrap();
        assert!(ui.key(win, Key::Char('x'), CTRL).unwrap().is_empty());
        assert_eq!((ui.take_copy(), ui.text_field_text(field).unwrap().as_str()), (None, "abcdefg"));
        // nor dragged out, and a double click does not reveal its words
        ui.pointer(win, 20, 10, PointerKind::Press).unwrap();
        ui.pointer(win, 60, 40, PointerKind::Move).unwrap();
        assert_eq!(ui.take_drag(), None);
        ui.pointer(win, 60, 40, PointerKind::Release).unwrap();
        // input methods and assistive technology are kept out of it
        assert_eq!(ui.ime_target(win), None);
        let tree = ui.accessible(win).unwrap();
        let node = tree.nodes.iter().find(|n| n.handle == field).unwrap();
        assert!(node.password && node.value.is_none());
        // a click still lands between the right bullets
        ui.tick(5_000);
        let x = 5 + 1 + ui.font.width("\u{2022}\u{2022}\u{2022}");
        ui.pointer(win, x, 10, PointerKind::Press).unwrap();
        ui.pointer(win, x, 10, PointerKind::Release).unwrap();
        ui.text(win, "X").unwrap();
        assert_eq!(ui.text_field_text(field).unwrap(), "abcXdefg");
    }

    #[test]
    fn a_tab_in_a_text_area_is_kept_and_is_four_spaces_wide() {
        let (mut ui, _win, area, _) = area("\tindented\nnot", 4);
        assert_eq!(ui.text_field_text(area).unwrap(), "\tindented\nnot");
        assert_eq!(ui.font.width("\t"), 4 * ui.font.width(" "));
        // a one-line field still makes it a space
        let field = ui.text_field_new("a\tb");
        assert_eq!(ui.text_field_text(field).unwrap(), "a b");
    }

    #[test]
    fn the_program_can_select_and_is_told_when_the_user_moves_the_caret() {
        let (mut ui, win, area, _) = area("hello world", 4);
        // what the program does itself is not told back to it
        ui.text_field_select(area, 6, 11).unwrap();
        assert_eq!(ui.text_field_selection(area).unwrap(), (6, 11));
        assert!(ui.selection_events().is_empty());
        // the selection runs backwards when the caret is put first
        ui.text_field_select(area, 5, 0).unwrap();
        assert_eq!(ui.text_field_selection(area).unwrap(), (0, 5));
        // places past the end go to the end; inside a character, to its start
        ui.text_field_set_text(area, "caf\u{e9}").unwrap();
        ui.text_field_select(area, 99, 4).unwrap();
        assert_eq!(ui.text_field_selection(area).unwrap(), (3, 5));
        assert!(ui.selection_events().is_empty());
        // typing moves the caret, and that is told once
        ui.text_field_select(area, 5, 5).unwrap();
        ui.text(win, "!").unwrap();
        assert_eq!(ui.selection_events(), vec![Emitted { target: area, event: "TextField.selected", args: vec![Value::I32(6), Value::I32(6)] }]);
        assert!(ui.selection_events().is_empty());
        // and so does selecting with the keys
        ui.key(win, Key::Left, SHIFT).unwrap();
        assert_eq!(ui.selection_events(), vec![Emitted { target: area, event: "TextField.selected", args: vec![Value::I32(5), Value::I32(6)] }]);
        assert_eq!(ui.text_field_select(area, -1, 0), Err(Error::InvalidArgument));
    }
}
