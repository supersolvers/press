// SPDX-License-Identifier: GPL-3.0-or-later
//! Rich text: words with a look of their own (a font, a size, bold,
//! italic, underlined, struck through, a colour) in paragraphs with one of
//! their own (aligned, indented, bulleted). What the RichText widget holds
//! and edits, and what RTF is read into and written from (see `rtf`).
//!
//! The text is kept whole, paragraphs parted by line breaks, so places in
//! it are byte offsets into its UTF-8, as they are in a text field. Every
//! byte has the look of the character it belongs to, by a number into a
//! table of looks; every paragraph has its own settings.

/// How a character looks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Look {
    /// A family as fontconfig knows it: "Sans", "DejaVu Serif".
    pub family: String,
    /// In half-points, as RTF has it: 24 is twelve points.
    pub size: u16,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    /// 0xRRGGBB.
    pub colour: u32,
}

impl Default for Look {
    fn default() -> Look {
        Look { family: "Sans".into(), size: 22, bold: false, italic: false, underline: false, strike: false, colour: 0 }
    }
}

impl Look {
    /// The size in pixels at 96 to the inch.
    pub fn pixels(&self) -> f32 {
        self.size as f32 * 96.0 / 144.0
    }
}

/// Where the lines of a paragraph sit across the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Left,
    Centre,
    Right,
    /// Both edges straight, the spaces stretched; its last line as Left.
    Full,
}

/// How a paragraph is set. Indents are in twips, as RTF has them: 1440
/// to the inch, fifteen to a pixel at 96 to the inch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub justify: Justify,
    /// In from the left edge, and from the right.
    pub left: i32,
    pub right: i32,
    /// The first line in further than the rest (or out, if less than 0).
    pub first: i32,
    pub bullet: bool,
}

/// Twips to pixels at 96 to the inch.
pub fn twips_to_pixels(t: i32) -> i32 {
    t / 15
}

pub fn pixels_to_twips(p: i32) -> i32 {
    p * 15
}

/// A document: its text, the look of each byte of it, and each paragraph's settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    text: String,
    /// For each byte of the text, the number of its look.
    marks: Vec<u16>,
    looks: Vec<Look>,
    paragraphs: Vec<Paragraph>,
}

impl Document {
    /// An empty document whose first words will look so.
    pub fn new(look: Look) -> Document {
        Document { text: String::new(), marks: Vec::new(), looks: vec![look], paragraphs: vec![Paragraph::default()] }
    }

    /// Plain text, all of one look; line breaks part its paragraphs.
    pub fn plain(text: &str, look: Look) -> Document {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let count = text.matches('\n').count() + 1;
        Document { marks: vec![0; text.len()], text, looks: vec![look], paragraphs: vec![Paragraph::default(); count] }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The nearest character boundary at or before a place.
    pub fn floor(&self, at: usize) -> usize {
        let mut at = at.min(self.text.len());
        while !self.text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }

    /// The look of the character at a place (or, at the end, of the last).
    pub fn look_at(&self, at: usize) -> &Look {
        let at = self.floor(at);
        let i = if at < self.marks.len() { self.marks[at] } else { self.marks.last().copied().unwrap_or(0) };
        &self.looks[i as usize]
    }

    /// The look typing at a place takes: that of the character before it,
    /// unless it starts a paragraph, when it is that of the one after.
    pub fn look_for_typing(&self, at: usize) -> Look {
        let at = self.floor(at);
        let starts = at == 0 || self.text.as_bytes()[at - 1] == b'\n';
        if starts && at < self.text.len() && self.text.as_bytes()[at] != b'\n' {
            return self.look_at(at).clone();
        }
        if at == 0 {
            return self.looks[self.marks.first().copied().unwrap_or(0) as usize].clone();
        }
        let before = self.floor(at - 1);
        self.look_at(before).clone()
    }

    fn mark_of(&mut self, look: &Look) -> u16 {
        if let Some(i) = self.looks.iter().position(|l| l == look) {
            return i as u16;
        }
        if self.looks.len() >= u16::MAX as usize {
            self.compact();
            if let Some(i) = self.looks.iter().position(|l| l == look) {
                return i as u16;
            }
        }
        self.looks.push(look.clone());
        (self.looks.len() - 1) as u16
    }

    /// Forget looks no character has any more.
    fn compact(&mut self) {
        let mut used = vec![false; self.looks.len()];
        for m in &self.marks {
            used[*m as usize] = true;
        }
        used[0] = true;
        let mut map = vec![0u16; self.looks.len()];
        let mut kept = Vec::new();
        for (i, look) in self.looks.iter().enumerate() {
            if used[i] {
                map[i] = kept.len() as u16;
                kept.push(look.clone());
            }
        }
        for m in &mut self.marks {
            *m = map[*m as usize];
        }
        self.looks = kept;
    }

    /// The number of the paragraph a place is in.
    pub fn paragraph_of(&self, at: usize) -> usize {
        self.text.as_bytes()[..at.min(self.text.len())].iter().filter(|b| **b == b'\n').count()
    }

    pub fn paragraph(&self, n: usize) -> &Paragraph {
        &self.paragraphs[n.min(self.paragraphs.len() - 1)]
    }

    /// Each paragraph: where it starts and ends (before its line break), and its settings.
    pub fn paragraphs(&self) -> Vec<(usize, usize, Paragraph)> {
        let mut out = Vec::with_capacity(self.paragraphs.len());
        let mut start = 0;
        for (n, part) in self.text.split('\n').enumerate() {
            out.push((start, start + part.len(), self.paragraphs[n.min(self.paragraphs.len() - 1)]));
            start += part.len() + 1;
        }
        out
    }

    /// The stretches of one look between two places, each with its look.
    pub fn runs(&self, from: usize, to: usize) -> Vec<(usize, usize, &Look)> {
        let mut out: Vec<(usize, usize, &Look)> = Vec::new();
        let to = to.min(self.text.len());
        let mut at = from.min(to);
        while at < to {
            let mark = self.marks[at];
            let mut end = at + 1;
            while end < to && self.marks[end] == mark {
                end += 1;
            }
            out.push((at, end, &self.looks[mark as usize]));
            at = end;
        }
        out
    }

    /// Put text in at a place, looking so. Line breaks in it start
    /// paragraphs set as the one they are typed in.
    pub fn insert(&mut self, at: usize, text: &str, look: &Look) {
        let at = self.floor(at);
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if text.is_empty() {
            return;
        }
        let mark = self.mark_of(look);
        let n = self.paragraph_of(at);
        let breaks = text.matches('\n').count();
        let setting = self.paragraphs[n.min(self.paragraphs.len() - 1)];
        self.paragraphs.splice(n + 1..n + 1, std::iter::repeat_n(setting, breaks));
        self.text.insert_str(at, &text);
        self.marks.splice(at..at, std::iter::repeat_n(mark, text.len()));
    }

    /// Take out what lies between two places. Paragraphs joined keep the
    /// settings of the first.
    pub fn delete(&mut self, from: usize, to: usize) {
        let (from, to) = (self.floor(from.min(to)), self.floor(from.max(to)));
        if from == to {
            return;
        }
        let n = self.paragraph_of(from);
        let breaks = self.text[from..to].matches('\n').count();
        self.paragraphs.drain(n + 1..n + 1 + breaks);
        self.text.replace_range(from..to, "");
        self.marks.drain(from..to);
    }

    /// A part of the document, as a document of its own.
    pub fn slice(&self, from: usize, to: usize) -> Document {
        let (from, to) = (self.floor(from.min(to)), self.floor(from.max(to)));
        let n = self.paragraph_of(from);
        let count = self.text[from..to].matches('\n').count() + 1;
        let mut out = Document { text: self.text[from..to].to_string(), marks: self.marks[from..to].to_vec(), looks: self.looks.clone(), paragraphs: self.paragraphs[n..n + count].to_vec() };
        out.compact();
        out
    }

    /// Put another document in at a place: its words with their looks, its
    /// paragraphs with their settings (all but the first, which joins the
    /// paragraph it lands in).
    pub fn insert_document(&mut self, at: usize, other: &Document) {
        let at = self.floor(at);
        if other.is_empty() {
            return;
        }
        let map: Vec<u16> = other.looks.iter().map(|l| self.mark_of(l)).collect();
        let n = self.paragraph_of(at);
        self.paragraphs.splice(n + 1..n + 1, other.paragraphs.iter().skip(1).copied());
        self.text.insert_str(at, &other.text);
        self.marks.splice(at..at, other.marks.iter().map(|m| map[*m as usize]));
    }

    /// Change the look of what lies between two places.
    pub fn restyle(&mut self, from: usize, to: usize, change: impl Fn(&mut Look)) {
        let (from, to) = (self.floor(from.min(to)), self.floor(from.max(to)));
        let mut done: Vec<(u16, u16)> = Vec::new();
        for at in from..to {
            let was = self.marks[at];
            let now = match done.iter().find(|(w, _)| *w == was) {
                Some((_, n)) => *n,
                None => {
                    let mut look = self.looks[was as usize].clone();
                    change(&mut look);
                    let n = self.mark_of(&look);
                    done.push((was, n));
                    n
                }
            };
            self.marks[at] = now;
        }
    }

    /// Change the settings of every paragraph between two places.
    pub fn reset_paragraphs(&mut self, from: usize, to: usize, change: impl Fn(&mut Paragraph)) {
        let (a, b) = (self.paragraph_of(from.min(to)), self.paragraph_of(from.max(to)));
        let last = b.min(self.paragraphs.len() - 1);
        for p in &mut self.paragraphs[a..=last] {
            change(p);
        }
    }

    /// Set a paragraph's settings outright.
    pub fn set_paragraph(&mut self, n: usize, setting: Paragraph) {
        if let Some(p) = self.paragraphs.get_mut(n) {
            *p = setting;
        }
    }

    /// The families the document uses, each once, in order of first use.
    pub fn families(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (_, _, look) in self.runs(0, self.text.len()) {
            if !out.contains(&look.family) {
                out.push(look.family.clone());
            }
        }
        if out.is_empty() {
            out.push(self.looks[0].family.clone());
        }
        out
    }
}

/// What can be undone: the document and where the selection was, before.
#[derive(Debug, Clone)]
struct Step {
    document: Document,
    anchor: usize,
    caret: usize,
    /// Typing joins the step before it, if that was typing too.
    typing: bool,
}

/// A document being edited: it, the selection, the look the next words
/// typed will have, and what can be undone and done again.
#[derive(Debug, Clone)]
pub struct Editor {
    pub document: Document,
    anchor: usize,
    caret: usize,
    /// A look chosen with nothing selected, for the next words typed.
    pending: Option<Look>,
    undos: Vec<Step>,
    redos: Vec<Step>,
}

/// How many steps are kept to undo.
const UNDOS: usize = 100;

impl Editor {
    pub fn new(document: Document) -> Editor {
        Editor { document, anchor: 0, caret: 0, pending: None, undos: Vec::new(), redos: Vec::new() }
    }

    pub fn anchor(&self) -> usize {
        self.anchor
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selection, its start first.
    pub fn selection(&self) -> (usize, usize) {
        (self.anchor.min(self.caret), self.anchor.max(self.caret))
    }

    pub fn has_selection(&self) -> bool {
        self.anchor != self.caret
    }

    pub fn select(&mut self, anchor: usize, caret: usize) {
        self.anchor = self.document.floor(anchor);
        self.caret = self.document.floor(caret);
        self.pending = None;
    }

    /// The whole document taken in, the selection kept where it can be.
    pub fn replace_document(&mut self, document: Document) {
        self.document = document;
        self.anchor = 0;
        self.caret = 0;
        self.pending = None;
        self.undos.clear();
        self.redos.clear();
    }

    /// The look the selection has (that of its first character), or that
    /// the next words typed will have.
    pub fn look(&self) -> Look {
        if let Some(p) = &self.pending {
            return p.clone();
        }
        let (from, to) = self.selection();
        if from < to { self.document.look_at(from).clone() } else { self.document.look_for_typing(self.caret) }
    }

    /// The settings of the paragraph the caret is in.
    pub fn paragraph(&self) -> Paragraph {
        *self.document.paragraph(self.document.paragraph_of(self.caret))
    }

    fn remember(&mut self, typing: bool) {
        if typing && self.undos.last().is_some_and(|s| s.typing) && self.redos.is_empty() {
            return;
        }
        self.undos.push(Step { document: self.document.clone(), anchor: self.anchor, caret: self.caret, typing });
        if self.undos.len() > UNDOS {
            self.undos.remove(0);
        }
        self.redos.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undos.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redos.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(step) = self.undos.pop() else { return false };
        self.redos.push(Step { document: std::mem::replace(&mut self.document, step.document), anchor: self.anchor, caret: self.caret, typing: false });
        (self.anchor, self.caret) = (step.anchor, step.caret);
        self.pending = None;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(step) = self.redos.pop() else { return false };
        self.undos.push(Step { document: std::mem::replace(&mut self.document, step.document), anchor: self.anchor, caret: self.caret, typing: false });
        (self.anchor, self.caret) = (step.anchor, step.caret);
        self.pending = None;
        true
    }

    /// Type: the selection replaced by text in the look for typing. Returns
    /// whether anything changed.
    pub fn type_text(&mut self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let look = self.look();
        // a word's end is a good place for undo to stop
        self.remember(self.anchor == self.caret && !text.contains([' ', '\n']));
        let (from, to) = self.selection();
        self.document.delete(from, to);
        self.document.insert(from, text, &look);
        self.caret = from + text.replace("\r\n", "\n").replace('\r', "\n").len();
        self.anchor = self.caret;
        self.pending = None;
        true
    }

    /// The selection replaced by another document (a paste).
    pub fn put_document(&mut self, other: &Document) -> bool {
        self.remember(false);
        let (from, to) = self.selection();
        self.document.delete(from, to);
        self.document.insert_document(from, other);
        self.caret = from + other.len();
        self.anchor = self.caret;
        self.pending = None;
        true
    }

    /// Take out the selection. Returns whether anything changed.
    pub fn delete_selection(&mut self) -> bool {
        let (from, to) = self.selection();
        if from == to {
            return false;
        }
        self.remember(false);
        self.document.delete(from, to);
        self.anchor = from;
        self.caret = from;
        true
    }

    /// Take out the character (or with `word`, the word) before the caret,
    /// or the selection.
    pub fn backspace(&mut self, word: bool) -> bool {
        if self.has_selection() {
            return self.delete_selection();
        }
        if self.caret == 0 {
            return false;
        }
        let from = if word { self.word_start(self.caret) } else { self.document.floor(self.caret - 1) };
        self.anchor = from;
        self.delete_selection()
    }

    /// Take out the character (or word) after the caret, or the selection.
    pub fn delete(&mut self, word: bool) -> bool {
        if self.has_selection() {
            return self.delete_selection();
        }
        if self.caret >= self.document.len() {
            return false;
        }
        let to = if word { self.word_end(self.caret) } else { self.next(self.caret) };
        self.anchor = to;
        self.delete_selection()
    }

    /// The place after the character at a place.
    pub fn next(&self, at: usize) -> usize {
        let text = self.document.text();
        text[at.min(text.len())..].chars().next().map_or(text.len(), |c| at + c.len_utf8())
    }

    fn word_start(&self, at: usize) -> usize {
        let text = self.document.text();
        let before = &text[..self.document.floor(at)];
        let trimmed = before.trim_end_matches(|c: char| c.is_whitespace() && c != '\n');
        let start = trimmed.rfind(|c: char| !c.is_alphanumeric()).map_or(0, |i| i + trimmed[i..].chars().next().map_or(1, |c| c.len_utf8()));
        if start == trimmed.len() && start > 0 { self.document.floor(start - 1) } else { start }
    }

    fn word_end(&self, at: usize) -> usize {
        let text = self.document.text();
        let rest = &text[at..];
        let skipped = rest.len() - rest.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '\n').len();
        let after = &rest[skipped..];
        let word = after.find(|c: char| !c.is_alphanumeric()).unwrap_or(after.len());
        let end = at + skipped + word;
        if end == at { self.next(at) } else { end }
    }

    /// Move the caret, taking the selection with it if `extend`.
    pub fn move_to(&mut self, at: usize, extend: bool) {
        self.caret = self.document.floor(at);
        if !extend {
            self.anchor = self.caret;
        }
        self.pending = None;
    }

    pub fn left(&mut self, word: bool, extend: bool) {
        if !extend && self.has_selection() {
            let (from, _) = self.selection();
            return self.move_to(from, false);
        }
        let at = if word { self.word_start(self.caret) } else if self.caret == 0 { 0 } else { self.document.floor(self.caret - 1) };
        self.move_to(at, extend);
    }

    pub fn right(&mut self, word: bool, extend: bool) {
        if !extend && self.has_selection() {
            let (_, to) = self.selection();
            return self.move_to(to, false);
        }
        let at = if word { self.word_end(self.caret) } else { self.next(self.caret) };
        self.move_to(at, extend);
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.document.len();
        self.pending = None;
    }

    /// Select the word a place is in.
    pub fn select_word_at(&mut self, at: usize) {
        let text = self.document.text();
        let at = self.document.floor(at);
        let is_word = |c: char| c.is_alphanumeric() || c == '\'';
        let start = text[..at].rfind(|c: char| !is_word(c)).map_or(0, |i| i + text[i..].chars().next().map_or(1, |c| c.len_utf8()));
        let end = text[at..].find(|c: char| !is_word(c)).map_or(text.len(), |i| at + i);
        self.anchor = start;
        self.caret = if end == start { self.next(at) } else { end };
        self.pending = None;
    }

    /// Select the paragraph a place is in.
    pub fn select_paragraph_at(&mut self, at: usize) {
        let text = self.document.text();
        let at = self.document.floor(at);
        self.anchor = text[..at].rfind('\n').map_or(0, |i| i + 1);
        self.caret = text[at..].find('\n').map_or(text.len(), |i| at + i);
        self.pending = None;
    }

    /// Change the look of the selection; with nothing selected, of the
    /// words about to be typed.
    pub fn restyle(&mut self, change: impl Fn(&mut Look)) -> bool {
        let (from, to) = self.selection();
        if from == to {
            let mut look = self.look();
            change(&mut look);
            self.pending = Some(look);
            return false;
        }
        self.remember(false);
        self.document.restyle(from, to, change);
        true
    }

    /// Change the settings of the paragraphs the selection touches.
    pub fn reset_paragraphs(&mut self, change: impl Fn(&mut Paragraph)) -> bool {
        self.remember(false);
        let (from, to) = self.selection();
        self.document.reset_paragraphs(from, to, change);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold() -> Look {
        Look { bold: true, ..Look::default() }
    }

    #[test]
    fn looks_follow_their_words_through_edits() {
        let mut d = Document::plain("Hello world", Look::default());
        d.restyle(6, 11, |l| l.bold = true);
        assert!(!d.look_at(0).bold && d.look_at(6).bold);
        d.insert(6, "big ", &Look { italic: true, ..Look::default() });
        assert_eq!(d.text(), "Hello big world");
        assert!(d.look_at(6).italic && d.look_at(10).bold);
        d.delete(0, 6);
        assert_eq!(d.text(), "big world");
        assert_eq!(d.runs(0, d.len()).len(), 2);
        // typing after bold words is bold
        assert!(d.look_for_typing(d.len()).bold);
    }

    #[test]
    fn paragraphs_keep_their_settings_as_they_are_split_and_joined() {
        let mut d = Document::plain("one\ntwo", Look::default());
        d.reset_paragraphs(0, 0, |p| p.justify = Justify::Centre);
        assert_eq!(d.paragraphs().len(), 2);
        // a line break typed in a centred paragraph starts another centred one
        d.insert(2, "\n", &Look::default());
        let ps = d.paragraphs();
        assert_eq!(ps.len(), 3);
        assert_eq!((ps[0].2.justify, ps[1].2.justify, ps[2].2.justify), (Justify::Centre, Justify::Centre, Justify::Left));
        // joined, the first's settings stay
        d.delete(2, 3);
        assert_eq!(d.paragraphs().len(), 2);
        assert_eq!(d.paragraph(0).justify, Justify::Centre);
        assert_eq!(d.paragraph(1).justify, Justify::Left);
    }

    #[test]
    fn a_slice_pasted_back_brings_its_looks_and_paragraphs() {
        let mut d = Document::plain("ab\ncd", Look::default());
        d.restyle(0, 1, |l| *l = bold());
        d.reset_paragraphs(3, 3, |p| p.bullet = true);
        let piece = d.slice(0, 5);
        let mut e = Document::plain("X", Look::default());
        e.insert_document(1, &piece);
        assert_eq!(e.text(), "Xab\ncd");
        assert!(e.look_at(1).bold && !e.look_at(2).bold);
        assert!(e.paragraph(1).bullet && !e.paragraph(0).bullet);
    }

    #[test]
    fn the_editor_types_restyles_and_undoes() {
        let mut ed = Editor::new(Document::new(Look::default()));
        ed.type_text("Hi");
        ed.type_text(" there");
        assert_eq!(ed.document.text(), "Hi there");
        // bold chosen with nothing selected is for what is typed next
        ed.restyle(|l| l.bold = true);
        ed.type_text("!");
        assert!(ed.document.look_at(8).bold && !ed.document.look_at(0).bold);
        ed.undo();
        assert_eq!(ed.document.text(), "Hi there");
        ed.undo();
        // the words typed together go together
        assert!(ed.document.text().len() < 8);
        ed.redo();
        ed.select(0, 2);
        assert!(ed.restyle(|l| l.italic = true));
        assert!(ed.document.look_at(0).italic && ed.look().italic);
        ed.move_to(ed.document.len(), false);
        assert!(ed.backspace(true));
        assert_eq!(ed.document.text(), "Hi ");
    }
}
