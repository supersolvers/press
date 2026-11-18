// SPDX-License-Identifier: GPL-3.0-or-later
//! The editing model behind a text field: text (one line, or several for a
//! text area), a caret and a selection anchor. Positions are byte offsets that always sit on a
//! character boundary. Nothing here knows about pixels or widgets.
//!
//! A character is what the reader takes for one: a letter with the accents
//! written after it, the parts of a syllable, a flag. The caret never rests
//! inside one.

use unicode_segmentation::GraphemeCursor;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Editor {
    text: String,
    /// Where typing happens.
    caret: usize,
    /// The other end of the selection; equal to `caret` when nothing is selected.
    anchor: usize,
    /// Whether line breaks are kept. Otherwise they become spaces.
    multiline: bool,
}

/// Longest text a one-line field will hold, in bytes.
pub const MAX_TEXT: usize = 64 * 1024;
/// Longest text a text area will hold, in bytes: room for a long document
/// or a large source file.
pub const MAX_AREA_TEXT: usize = 2 * 1024 * 1024;

// An accent or other mark written after its letter belongs to the letter's word.
fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || crate::font::is_mark(ch)
}

/// Drop what a field cannot show. A text area keeps tabs and line breaks;
/// a one-line field makes them spaces, so pasted or scripted text keeps its
/// word gaps. Line endings from other systems are read as plain line breaks.
fn clean(input: &str, multiline: bool) -> String {
    input
        .replace("\r\n", "\n")
        .chars()
        .filter_map(|ch| match ch {
            '\n' | '\r' if multiline => Some('\n'),
            '\t' if multiline => Some('\t'),
            '\n' | '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect()
}

impl Editor {
    pub fn new(text: &str) -> Editor {
        let mut e = Editor::default();
        e.set_text(text);
        e
    }

    /// An editor that keeps line breaks.
    pub fn new_multiline(text: &str) -> Editor {
        let mut e = Editor { multiline: true, ..Editor::default() };
        e.set_text(text);
        e
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The most text this editor holds, in bytes.
    pub fn limit(&self) -> usize {
        if self.multiline { MAX_AREA_TEXT } else { MAX_TEXT }
    }

    /// Select from `anchor` to `caret`, byte offsets into the text; the same
    /// place twice puts the caret there. A place inside a character moves to
    /// its start, and one past the end to the end.
    pub fn select(&mut self, anchor: usize, caret: usize) {
        self.anchor = self.snap(anchor.min(self.text.len()));
        self.caret = self.snap(caret.min(self.text.len()));
    }

    /// The end of the selection that stays put as it grows: where the
    /// caret was when selecting began.
    pub fn anchor(&self) -> usize {
        self.anchor
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selected range, start first. Empty when nothing is selected.
    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    /// The selected text.
    pub fn selected(&self) -> &str {
        let (from, to) = self.selection();
        &self.text[from..to]
    }

    pub fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }

    /// Replace everything and reset the caret.
    pub fn set_text(&mut self, text: &str) {
        let mut text = clean(text, self.multiline);
        truncate(&mut text, self.limit());
        // One line is usually added to, so the caret goes to the end. Several
        // lines are usually read from the top first.
        self.caret = if self.multiline { 0 } else { text.len() };
        self.anchor = self.caret;
        self.text = text;
    }

    // What the reader takes for one character may be several in the text:
    // a letter and the accents written after it, the parts of a Korean
    // syllable, a flag. The caret steps over such a thing whole and never
    // rests inside it.
    fn prev(&self, pos: usize) -> usize {
        GraphemeCursor::new(pos, self.text.len(), true).prev_boundary(&self.text, 0).ok().flatten().unwrap_or(0)
    }

    fn next(&self, pos: usize) -> usize {
        GraphemeCursor::new(pos, self.text.len(), true).next_boundary(&self.text, 0).ok().flatten().unwrap_or(self.text.len())
    }

    /// Start of the word at or before `pos`.
    fn word_left(&self, pos: usize) -> usize {
        let mut p = pos;
        let before = |p: usize| self.text[..p].chars().next_back();
        while before(p).is_some_and(|c| !is_word(c)) {
            p = self.prev(p);
        }
        while before(p).is_some_and(is_word) {
            p = self.prev(p);
        }
        p
    }

    /// End of the word at or after `pos`.
    fn word_right(&self, pos: usize) -> usize {
        let mut p = pos;
        let at = |p: usize| self.text[p..].chars().next();
        while at(p).is_some_and(|c| !is_word(c)) {
            p = self.next(p);
        }
        while at(p).is_some_and(is_word) {
            p = self.next(p);
        }
        p
    }

    /// Snap an arbitrary offset to the start of the character it is in:
    /// the whole character as the reader sees it, accents and all.
    fn snap(&self, pos: usize) -> usize {
        let mut p = pos.min(self.text.len());
        while !self.text.is_char_boundary(p) {
            p -= 1;
        }
        if GraphemeCursor::new(p, self.text.len(), true).is_boundary(&self.text, 0).unwrap_or(true) {
            p
        } else {
            self.prev(p)
        }
    }

    /// Move the caret; with `extend` the anchor stays and the selection grows.
    pub fn move_to(&mut self, pos: usize, extend: bool) {
        self.caret = self.snap(pos);
        if !extend {
            self.anchor = self.caret;
        }
    }

    pub fn left(&mut self, word: bool, extend: bool) {
        let target = if self.has_selection() && !extend {
            self.selection().0 // collapsing a selection lands on its edge
        } else if word {
            self.word_left(self.caret)
        } else {
            self.prev(self.caret)
        };
        self.move_to(target, extend);
    }

    pub fn right(&mut self, word: bool, extend: bool) {
        let target = if self.has_selection() && !extend {
            self.selection().1
        } else if word {
            self.word_right(self.caret)
        } else {
            self.next(self.caret)
        };
        self.move_to(target, extend);
    }

    pub fn home(&mut self, extend: bool) {
        self.move_to(0, extend);
    }

    pub fn end(&mut self, extend: bool) {
        self.move_to(self.text.len(), extend);
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
    }

    /// Select the word under `pos`, or the run of non-word characters there.
    pub fn select_word_at(&mut self, pos: usize) {
        let pos = self.snap(pos);
        let here = self.text[pos..].chars().next().or_else(|| self.text[..pos].chars().next_back());
        let Some(here) = here else { return };
        let same = |c: char| is_word(c) == is_word(here);
        let (mut start, mut end) = (pos, pos);
        while self.text[..start].chars().next_back().is_some_and(same) {
            start = self.prev(start);
        }
        while self.text[end..].chars().next().is_some_and(same) {
            end = self.next(end);
        }
        self.anchor = start;
        self.caret = end;
    }

    fn remove(&mut self, from: usize, to: usize) -> bool {
        if from == to {
            return false;
        }
        self.text.replace_range(from..to, "");
        self.caret = from;
        self.anchor = from;
        true
    }

    /// Type or paste text over the selection. Returns whether anything changed.
    pub fn insert(&mut self, input: &str) -> bool {
        let mut input = clean(input, self.multiline);
        let (from, to) = self.selection();
        let room = self.limit().saturating_sub(self.text.len() - (to - from));
        truncate(&mut input, room);
        if input.is_empty() && from == to {
            return false;
        }
        self.text.replace_range(from..to, &input);
        self.caret = from + input.len();
        self.anchor = self.caret;
        true
    }

    /// Delete text either side of the selection (or caret), as an input
    /// method asks when it rewrites what it has already entered. Lengths are
    /// in bytes and are trimmed to whole characters.
    pub fn delete_around(&mut self, before: usize, after: usize) -> bool {
        let (from, to) = self.selection();
        let end = self.snap(to.saturating_add(after).min(self.text.len()));
        let start = self.snap(from.saturating_sub(before));
        if start == from && end == to {
            return false;
        }
        self.text.replace_range(to..end, "");
        self.text.replace_range(start..from, "");
        let moved = from - start;
        self.caret -= moved;
        self.anchor -= moved;
        true
    }

    /// Delete the selection, if there is one.
    pub fn backspace_selection(&mut self) -> bool {
        let (from, to) = self.selection();
        self.remove(from, to)
    }

    /// Delete the selection, or the character (or word) before the caret.
    pub fn backspace(&mut self, word: bool) -> bool {
        let (from, to) = self.selection();
        if from != to {
            return self.remove(from, to);
        }
        let mut start = if word { self.word_left(to) } else { self.prev(to) };
        // An accent typed after its letter comes off by itself, leaving the
        // letter, so that a wrong one can be put right. Anything else that
        // reads as one character goes whole.
        if !word {
            if let Some((at, last)) = self.text[start..to].char_indices().next_back().filter(|(at, ch)| *at > 0 && crate::font::is_mark(*ch)) {
                let _ = last;
                start += at;
            }
        }
        self.remove(start, to)
    }

    /// Delete the selection, or the character (or word) after the caret.
    pub fn delete(&mut self, word: bool) -> bool {
        let (from, to) = self.selection();
        if from != to {
            return self.remove(from, to);
        }
        let end = if word { self.word_right(from) } else { self.next(from) };
        self.remove(from, end)
    }
}

/// Shorten to at most `max` bytes without splitting a character, or a
/// letter from its accents.
fn truncate(text: &mut String, max: usize) {
    if text.len() > max {
        let mut cut = max;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        if !GraphemeCursor::new(cut, text.len(), true).is_boundary(text, 0).unwrap_or(true) {
            cut = GraphemeCursor::new(cut, text.len(), true).prev_boundary(text, 0).ok().flatten().unwrap_or(0);
        }
        text.truncate(cut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_replaces_the_selection() {
        let mut e = Editor::new("hello world");
        e.select_all();
        assert!(e.insert("hi"));
        assert_eq!(e.text(), "hi");
        assert_eq!(e.caret(), 2);
        assert!(!e.has_selection());
    }

    #[test]
    fn movement_respects_multibyte_characters() {
        let mut e = Editor::new("naïve café");
        e.left(false, false);
        assert_eq!(e.caret(), "naïve caf".len());
        assert!(e.backspace(false));
        assert_eq!(e.text(), "naïve caé");
        e.move_to(3, false); // inside the two-byte ï: snaps back to its start
        assert_eq!(e.caret(), 2);
        assert!(e.delete(false));
        assert_eq!(e.text(), "nave caé");
    }

    #[test]
    fn a_letter_and_its_accents_are_one_character() {
        // "é" written as e and a combining acute, between two plain letters
        let mut e = Editor::new("ae\u{301}z");
        assert_eq!(e.caret(), 5);
        e.left(false, false);
        assert_eq!(e.caret(), 4); // before z
        e.left(false, false);
        assert_eq!(e.caret(), 1); // before the e, not between it and its accent
        e.right(false, true);
        assert_eq!(e.selected(), "e\u{301}");
        // a click that lands inside it puts the caret before it
        e.move_to(2, false);
        assert_eq!(e.caret(), 1);
        // Delete takes the whole of it
        assert!(e.delete(false));
        assert_eq!(e.text(), "az");
        // Backspace takes an accent off its letter, and then the letter
        let mut e = Editor::new("e\u{301}\u{323}");
        assert!(e.backspace(false));
        assert_eq!(e.text(), "e\u{301}");
        assert!(e.backspace(false));
        assert_eq!(e.text(), "e");
        // an accent typed after a letter joins it
        assert!(e.insert("\u{302}"));
        e.left(false, false);
        assert_eq!(e.caret(), 0);
        // other things that read as one character go whole: a flag, a Korean syllable in its parts
        for whole in ["\u{1F1EB}\u{1F1F7}", "\u{1112}\u{1161}\u{11AB}"] {
            let mut e = Editor::new(&format!("a{whole}"));
            e.left(false, false);
            assert_eq!(e.caret(), 1);
            e.end(false);
            assert!(e.backspace(false));
            assert_eq!(e.text(), "a");
        }
        // a word keeps its accents, and a double click takes them with it
        let mut e = Editor::new("cafe\u{301} noir");
        e.select_word_at(1);
        assert_eq!(e.selected(), "cafe\u{301}");
        e.end(false);
        e.left(true, false);
        e.left(true, false);
        assert_eq!(e.caret(), 0);
    }

    #[test]
    fn word_motion_and_deletion() {
        let mut e = Editor::new("one two  three");
        e.left(true, false);
        assert_eq!(e.caret(), 9);
        e.left(true, false);
        assert_eq!(e.caret(), 4);
        e.right(true, false);
        assert_eq!(e.caret(), 7);
        assert!(e.backspace(true));
        assert_eq!(e.text(), "one   three");
        assert!(e.delete(true));
        assert_eq!(e.text(), "one ");
    }

    #[test]
    fn shift_extends_and_plain_arrows_collapse() {
        let mut e = Editor::new("abcdef");
        e.home(false);
        e.right(false, true);
        e.right(false, true);
        assert_eq!(e.selection(), (0, 2));
        e.right(false, false); // collapses to the right edge without moving on
        assert_eq!((e.caret(), e.has_selection()), (2, false));
        e.end(true);
        e.left(false, false);
        assert_eq!(e.caret(), 2);
    }

    #[test]
    fn double_click_selects_a_word() {
        let mut e = Editor::new("save the file");
        e.select_word_at(6);
        assert_eq!(e.selection(), (5, 8));
        e.select_word_at(4); // on the space
        assert_eq!(e.selection(), (4, 5));
    }

    #[test]
    fn an_input_method_can_take_back_text_around_the_caret() {
        let mut e = Editor::new("abcdéf");
        e.move_to(3, false);
        // "bc" goes from before the caret; the 2 bytes after it are 'd' and
        // half of é, so only 'd' goes
        assert!(e.delete_around(2, 2));
        assert_eq!((e.text(), e.caret()), ("aéf", 1));
        assert!(!e.delete_around(0, 0));
        assert!(e.delete_around(99, 99));
        assert_eq!((e.text(), e.caret()), ("", 0));
    }

    #[test]
    fn control_characters_never_get_in() {
        let mut e = Editor::new("a\nb\u{7}c");
        assert_eq!(e.text(), "a bc");
        assert!(!e.insert("\u{1b}"));
        assert!(e.insert("\tx"));
        assert_eq!(e.text(), "a bc x");
    }
}
