// SPDX-License-Identifier: GPL-3.0-or-later
//! Text in one bit, in press's own families (their sources are in fonts/): pixel faces drawn for
//! the one size each is used at. Their pixels are bundled as they were
//! drawn (see `press-fontgen`, which takes them out of the font files), so
//! a letter is copied to the screen dot for dot with nothing worked out.
//!
//! A family has Western, Greek, Cyrillic and Hebrew letters of its own. The
//! many thousands of Chinese, Japanese and Korean characters are the same
//! in every family and are kept once, for all of them to fall back on, and
//! so are the families of pictures: signs for the interface, pictographs,
//! and Egyptian hieroglyphs.
//!
//! A letter none of these has is borrowed from whichever of the machine's
//! own fonts fontconfig offers for it. Those outlines were not made for
//! one bit, so they come out plainer, but they can be read.

use crate::canvas::Canvas;
use unicode_normalization::UnicodeNormalization;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

/// A family of letters to draw with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Face {
    /// Heavy and plain: the face of the menus, the titles and the buttons.
    #[default]
    Cicero,
    /// Light and small, for a lot of words in a little room; and its bold.
    Agate,
    AgateBold,
    /// Light and airy, for reading.
    Brevier,
    /// Every letter the same width, for columns and for programs.
    Pica,
    /// The same, heavy and larger.
    Elite,
}

/// The press families (formerly Fonts of Old), Copyright 2026 The Fonts of
/// Old Project Authors, under the SIL Open Font License 1.1 (see assets/fonts).
static CICERO: &[u8] = include_bytes!("../../../assets/fonts/Cicero-Regular.mfont");
static AGATE: &[u8] = include_bytes!("../../../assets/fonts/Agate-Regular.mfont");
static AGATE_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Agate-Bold.mfont");
static BREVIER: &[u8] = include_bytes!("../../../assets/fonts/Brevier-Regular.mfont");
static PICA: &[u8] = include_bytes!("../../../assets/fonts/Pica-Regular.mfont");
static ELITE: &[u8] = include_bytes!("../../../assets/fonts/Elite-Regular.mfont");
/// What every family falls back on, in this order: the characters of
/// China, Japan and Korea; signs for the interface; pictographs; hieroglyphs.
static HAN: &[u8] = include_bytes!("../../../assets/fonts/Han-12.mfont");
static FLEURON: &[u8] = include_bytes!("../../../assets/fonts/Fleuron-Regular.mfont");
static VIGNETTE: &[u8] = include_bytes!("../../../assets/fonts/Vignette-Regular.mfont");
static CARTOUCHE: &[u8] = include_bytes!("../../../assets/fonts/Cartouche-Regular.mfont");

#[derive(Clone)]
struct Glyph {
    width: i32,
    height: i32,
    left: i32,
    /// Distance from the baseline up to the top row.
    top: i32,
    advance: i32,
    bits: Vec<bool>,
    /// For a letter drawn smooth from an outline: how much of each pixel it
    /// covers, 0 to 255, in place of `bits`.
    cover: Option<Vec<u8>>,
}

/// A typeface of the machine's, to be drawn smooth from its outlines at a
/// size in pixels: what a style may ask for in place of a bundled family.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Type {
    /// A family as fontconfig knows it; several, with commas between, are
    /// tried in turn.
    pub family: String,
    pub size: i32,
    pub bold: bool,
}

/// The outline a smooth font draws its letters from.
#[derive(Clone)]
struct Outline {
    face: Arc<fontdue::Font>,
    size: f32,
}

/// One bundled family, read where it lies: nothing is unpacked until a
/// letter is wanted. The layout is set out in `press-fontgen`.
#[derive(Clone, Copy)]
struct Strike {
    data: &'static [u8],
    size: i32,
    ascent: i32,
    descent: i32,
    /// For a character taken from the shared ones: how far the pen moves
    /// (0 for as drawn), and how far its picture is moved right and up.
    wide: i32,
    shift: (i32, i32),
    count: usize,
}

impl Strike {
    const HEAD: usize = 16;

    fn new(data: &'static [u8]) -> Strike {
        assert!(data.len() >= Self::HEAD && &data[..6] == b"MFNT1\0", "a bundled font is not in the form expected");
        let count = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        assert!(data.len() >= Self::HEAD + count * 8, "a bundled font is cut short");
        Strike { data, size: data[6] as i32, ascent: data[7] as i32, descent: data[8] as i32, wide: data[9] as i32, shift: (data[10] as i8 as i32, data[11] as i8 as i32), count }
    }

    fn word(&self, at: usize) -> u32 {
        u32::from_le_bytes([self.data[at], self.data[at + 1], self.data[at + 2], self.data[at + 3]])
    }

    /// A character's picture, if this family has it.
    fn glyph(&self, ch: char) -> Option<Glyph> {
        let (mut low, mut high) = (0, self.count);
        while low < high {
            let middle = (low + high) / 2;
            match self.word(Self::HEAD + middle * 8).cmp(&(ch as u32)) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => {
                    let at = Self::HEAD + self.count * 8 + self.word(Self::HEAD + middle * 8 + 4) as usize;
                    let head = self.data.get(at..at + 5)?;
                    let (width, height) = (head[3] as i32, head[4] as i32);
                    let packed = self.data.get(at + 5..at + 5 + ((width * height + 7) / 8) as usize)?;
                    let bits = (0..(width * height) as usize).map(|i| packed[i / 8] & (0x80 >> (i % 8)) != 0).collect();
                    return Some(Glyph { width, height, left: head[1] as i8 as i32, top: head[2] as i8 as i32, advance: head[0] as i8 as i32, bits, cover: None });
                }
            }
        }
        None
    }

    fn has(&self, ch: char) -> bool {
        self.glyph(ch).is_some()
    }
}

pub struct Font {
    /// The family's own letters.
    own: Strike,
    /// Set for a font drawn smooth; the bundled family is then only what
    /// it falls back on for signs and pictures.
    outline: Option<Outline>,
    glyphs: HashMap<char, Glyph>,
    pub ascent: i32,
    pub descent: i32,
}

/// The machine's fonts that have been borrowed from so far, shared by every
/// `Font` in the program: a large one takes a moment to read, and should be
/// read once.
#[derive(Default)]
struct Borrowed {
    faces: Vec<(PathBuf, u32, Arc<fontdue::Font>)>,
    /// What fontconfig offered for each page of 256 characters asked about.
    pages: HashMap<u32, Option<Arc<fontdue::Font>>>,
}

fn borrowed() -> &'static Mutex<Borrowed> {
    static BORROWED: OnceLock<Mutex<Borrowed>> = OnceLock::new();
    BORROWED.get_or_init(Default::default)
}

/// Ideographs and the like, whose strokes are too many to be drawn heavy at
/// this size. Everything else is asked for in bold, to sit beside the
/// family's own letters.
fn dense(ch: char) -> bool {
    ch as u32 >= 0x2E80
}

/// Ask fontconfig which font file to use for a character.
fn offered(ch: char) -> Option<(PathBuf, u32)> {
    let weight = if dense(ch) { "" } else { ":bold" };
    let pattern = format!("sans{weight}:scalable=true:charset={:x}", ch as u32);
    let out = Command::new("fc-match").args(["-f", "%{file}\n%{index}\n", &pattern]).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let mut lines = text.lines();
    let file = PathBuf::from(lines.next().filter(|l| !l.is_empty())?);
    Some((file, lines.next().and_then(|l| l.parse().ok()).unwrap_or(0)))
}

/// A font of the machine's that has this character, if there is one.
fn borrow(ch: char) -> Option<Arc<fontdue::Font>> {
    if std::env::var_os("PRESS_FONT_FALLBACK").is_some_and(|v| v == "0") {
        return None;
    }
    let mut b = borrowed().lock().unwrap_or_else(|e| e.into_inner());
    let page = ch as u32 >> 8;
    if !b.pages.contains_key(&page) {
        let face = offered(ch).and_then(|(file, index)| face_at(&mut b, file, index));
        b.pages.insert(page, face);
    }
    let has = |f: &Arc<fontdue::Font>| f.lookup_glyph_index(ch) != 0;
    match b.pages.get(&page) {
        Some(Some(face)) if has(face) => Some(face.clone()),
        _ => b.faces.iter().map(|(_, _, f)| f).find(|f| has(f)).cloned(),
    }
}

/// Read a font file, or find it among those already read.
fn face_at(b: &mut Borrowed, file: PathBuf, index: u32) -> Option<Arc<fontdue::Font>> {
    if let Some((_, _, face)) = b.faces.iter().find(|(f, i, _)| *f == file && *i == index) {
        return Some(face.clone());
    }
    let bytes = std::fs::read(&file).ok()?;
    let settings = fontdue::FontSettings { collection_index: index, ..Default::default() };
    let face = Arc::new(fontdue::Font::from_bytes(bytes, settings).ok()?);
    b.faces.push((file, index, face.clone()));
    Some(face)
}

/// The machine's font for a family by name, as fontconfig chooses it: the
/// family itself where it is installed, and the nearest thing where not.
fn named(family: &str, bold: bool) -> Option<Arc<fontdue::Font>> {
    static NAMED: OnceLock<Mutex<HashMap<(String, bool), Option<Arc<fontdue::Font>>>>> = OnceLock::new();
    let key = (family.to_string(), bold);
    let known = NAMED.get_or_init(Default::default);
    if let Some(found) = known.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return found.clone();
    }
    let pattern = format!("{family}{}:scalable=true", if bold { ":bold" } else { "" });
    let found = Command::new("fc-match").args(["-f", "%{file}\n%{index}\n", &pattern]).output().ok().and_then(|out| {
        let text = String::from_utf8(out.stdout).ok()?;
        let mut lines = text.lines();
        let file = PathBuf::from(lines.next().filter(|l| !l.is_empty())?);
        let index = lines.next().and_then(|l| l.parse().ok()).unwrap_or(0);
        face_at(&mut borrowed().lock().unwrap_or_else(|e| e.into_inner()), file, index)
    });
    known.lock().unwrap_or_else(|e| e.into_inner()).insert(key, found.clone());
    found
}

/// Whether a character is a mark that is written after a letter and sits
/// on it: an accent, in the widest sense.
pub fn is_mark(ch: char) -> bool {
    matches!(ch as u32, 0x0300..=0x036F | 0x0483..=0x0489 | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

impl Default for Font {
    fn default() -> Self {
        Font::new()
    }
}

impl Font {
    /// The face of the interface.
    pub fn new() -> Font {
        Font::of(Face::default())
    }

    /// One of the bundled families.
    pub fn of(face: Face) -> Font {
        let own = Strike::new(match face {
            Face::Cicero => CICERO,
            Face::Agate => AGATE,
            Face::AgateBold => AGATE_BOLD,
            Face::Brevier => BREVIER,
            Face::Pica => PICA,
            Face::Elite => ELITE,
        });
        Font { own, outline: None, glyphs: HashMap::new(), ascent: own.ascent, descent: own.descent }
    }

    /// A typeface of the machine's, drawn smooth. `face` is the bundled
    /// family it stands in for, which it falls back on for signs and
    /// pictures, and is all there is if the machine has no such font.
    pub fn smooth(face: Face, wanted: &Type) -> Font {
        let mut font = Font::of(face);
        let size = wanted.size.clamp(6, 96) as f32;
        if let Some(outline) = named(&wanted.family, wanted.bold) {
            if let Some(line) = outline.horizontal_line_metrics(size) {
                font.ascent = line.ascent.round() as i32;
                font.descent = (-line.descent).round().max(1.0) as i32;
                font.outline = Some(Outline { face: outline, size });
            }
        }
        font
    }

    /// Whether letters are drawn smooth, from an outline.
    pub fn is_smooth(&self) -> bool {
        self.outline.is_some()
    }

    /// Height of a line of text: ascent plus descent.
    pub fn line_height(&self) -> i32 {
        self.ascent + self.descent
    }

    /// A character's picture from what is bundled: the family's own, or
    /// failing that one of those every family falls back on.
    fn bundled(&self, ch: char) -> Option<Glyph> {
        if let Some(glyph) = self.own.glyph(ch) {
            return Some(glyph);
        }
        if let Some(mut glyph) = Strike::new(HAN).glyph(ch) {
            // a fixed-pitch family gives these a cell of its own width, and centres them in it
            if self.own.wide != 0 {
                glyph.advance = self.own.wide;
            }
            glyph.left += self.own.shift.0;
            glyph.top += self.own.shift.1;
            return Some(glyph);
        }
        [FLEURON, VIGNETTE, CARTOUCHE].into_iter().find_map(|pictures| Strike::new(pictures).glyph(ch))
    }

    /// Whether what is bundled has a character, so that nothing need be
    /// borrowed from the machine to draw it.
    pub fn has(&self, ch: char) -> bool {
        self.own.has(ch) || [HAN, FLEURON, VIGNETTE, CARTOUCHE].into_iter().any(|more| Strike::new(more).has(ch))
    }

    fn glyph(&mut self, ch: char) -> &Glyph {
        if ch == '\t' && !self.glyphs.contains_key(&ch) {
            // a tab, kept in a text area, is the width of four spaces: an
            // indent lines up as it would at tab stops
            let space = self.glyph(' ').advance;
            self.glyphs.insert(ch, Glyph { width: 0, height: 0, left: 0, top: 0, advance: 4 * space, bits: Vec::new(), cover: None });
        }
        if !self.glyphs.contains_key(&ch) {
            if let Some(outline) = self.outline.clone() {
                let glyph = self.smooth_glyph(ch, &outline);
                self.glyphs.insert(ch, glyph);
                return &self.glyphs[&ch];
            }
            let size = self.own.size;
            let glyph = self.bundled(ch).or_else(|| {
                // not one of ours: borrowed from the machine, and drawn from its outline
                let other = borrow(ch).filter(|_| !ch.is_control() && !ch.is_whitespace())?;
                // Borrowed outlines are thinner than a pixel in places; taking
                // less than half coverage as ink keeps their strokes whole.
                let ink = if dense(ch) { 96 } else { 112 };
                let (m, coverage) = other.rasterize(ch, size as f32);
                Some(Glyph { width: m.width as i32, height: m.height as i32, left: m.xmin, top: m.ymin + m.height as i32, advance: m.advance_width.round() as i32, bits: coverage.iter().map(|&c| c >= ink).collect(), cover: None })
            });
            // a character nobody has takes the room of a space and shows nothing
            let glyph = glyph.unwrap_or_else(|| Glyph { width: 0, height: 0, left: 0, top: 0, advance: self.own.glyph(' ').map_or(size / 3, |space| space.advance), bits: Vec::new(), cover: None });
            self.glyphs.insert(ch, glyph);
        }
        &self.glyphs[&ch]
    }

    /// A character for a smooth font: from its own outline, or one borrowed
    /// from another of the machine's fonts, or failing both a bundled sign.
    fn smooth_glyph(&self, ch: char, outline: &Outline) -> Glyph {
        let drawn = |face: &fontdue::Font| {
            let (m, coverage) = face.rasterize(ch, outline.size);
            Glyph { width: m.width as i32, height: m.height as i32, left: m.xmin, top: m.ymin + m.height as i32, advance: m.advance_width.round() as i32, bits: Vec::new(), cover: Some(coverage) }
        };
        if outline.face.lookup_glyph_index(ch) != 0 {
            return drawn(&outline.face);
        }
        if !ch.is_control() && !ch.is_whitespace() {
            if let Some(mut glyph) = self.bundled(ch).filter(|_| !dense(ch)) {
                // a sign of the engine's own (a tick, an arrow), set on this line
                glyph.top = glyph.top.min(self.ascent);
                return glyph;
            }
            if let Some(other) = borrow(ch) {
                return drawn(&other);
            }
        }
        let space = outline.face.metrics(' ', outline.size).advance_width.round() as i32;
        Glyph { width: 0, height: 0, left: 0, top: 0, advance: space.max(1), bits: Vec::new(), cover: None }
    }

    /// How far the pen moves for one character. An accent written after
    /// its letter sits on the letter and moves the pen nowhere.
    pub fn advance(&mut self, ch: char) -> i32 {
        if is_mark(ch) {
            0
        } else {
            self.glyph(ch).advance
        }
    }

    pub fn width(&mut self, text: &str) -> i32 {
        text.chars().map(|ch| self.advance(ch)).sum()
    }

    /// Put a glyph's ink on a canvas, its origin at `x` on the baseline.
    fn ink(canvas: &mut Canvas, g: &Glyph, x: i32, baseline: i32, color: u32) {
        if let Some(cover) = &g.cover {
            for row in 0..g.height {
                for col in 0..g.width {
                    let c = cover[(row * g.width + col) as usize];
                    if c > 0 {
                        canvas.blend(x + g.left + col, baseline - g.top + row, color, c);
                    }
                }
            }
            return;
        }
        for row in 0..g.height {
            for col in 0..g.width {
                if g.bits[(row * g.width + col) as usize] {
                    canvas.set(x + g.left + col, baseline - g.top + row, color);
                }
            }
        }
    }

    /// Draw with the top of the line at `y`.
    pub fn draw(&mut self, canvas: &mut Canvas, text: &str, x: i32, y: i32, color: u32) {
        let baseline = y + self.ascent;
        let mut pen = x;
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            // the accents written after this letter, if any
            let mut marks = Vec::new();
            while let Some(mark) = chars.peek().copied().filter(|c| is_mark(*c)) {
                marks.push(mark);
                chars.next();
            }
            if marks.is_empty() || is_mark(ch) {
                let g = self.glyph(ch);
                Self::ink(canvas, g, pen, baseline, color);
                pen += if is_mark(ch) { 0 } else { g.advance };
                continue;
            }
            // the family draws many accented letters itself, and best: use its
            // own where the letter and its accents make one of those
            let together: String = std::iter::once(ch).chain(marks.iter().copied()).nfc().collect();
            let mut made = together.chars();
            if let (Some(one), None) = (made.next(), made.next()) {
                if self.own.has(one) {
                    Self::ink(canvas, self.glyph(one), pen, baseline, color);
                    pen += self.glyph(ch).advance;
                    continue;
                }
            }
            // otherwise the letter, and each accent set over or under the
            // middle of it, clear of the letter and of the accents before
            let (advance, mut top, mut bottom, middle) = {
                let g = self.glyph(ch);
                Self::ink(canvas, g, pen, baseline, color);
                (g.advance, g.top, g.top - g.height, 2 * g.left + g.width)
            };
            for mark in marks {
                let g = self.glyph(mark);
                let at = pen + (middle - g.width) / 2 - g.left;
                let foot = g.top - g.height; // how far its lowest ink is above the baseline
                let lift = if foot > 0 {
                    // one that goes above: raised if the letter is tall
                    let lift = (top + 1 - foot).max(0);
                    top = g.top + lift;
                    lift
                } else if g.top <= 0 {
                    // one that goes below: lowered if the letter hangs down
                    // (out of the line if need be: run into the tail it would make another letter of it)
                    let drop = (g.top + 1 - bottom).max(0);
                    bottom = foot - drop;
                    -drop
                } else {
                    0 // one that goes through the letter stays where it is
                };
                Self::ink(canvas, g, at, baseline - lift, color);
            }
            pen += advance;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACES: [Face; 6] = [Face::Cicero, Face::Agate, Face::AgateBold, Face::Brevier, Face::Pica, Face::Elite];

    #[test]
    fn every_family_is_bundled_whole() {
        for face in FACES {
            let mut font = Font::of(face);
            assert!(font.ascent > 8 && font.descent > 0, "{face:?}");
            // its own letters, and what every family falls back on, with nothing borrowed
            for ch in ['A', 'g', '\u{e9}', '\u{3bb}', '\u{416}', '\u{5d0}', '\u{4f60}', '\u{3042}', '\u{d55c}', '\u{25b6}', '\u{13000}'] {
                assert!(font.has(ch), "{face:?} lacks {ch}");
                let g = font.glyph(ch);
                assert!(g.advance > 0 && g.bits.iter().any(|&b| b), "{face:?} draws nothing for {ch}");
            }
            // a space moves the pen and leaves no mark
            assert!(font.has(' ') && font.advance(' ') > 0 && !font.glyph(' ').bits.iter().any(|&b| b), "{face:?}");
            // letters stay within the line
            let (top, height) = (font.glyph('g').top, font.glyph('g').height);
            assert!(top <= font.ascent && top - height >= -font.descent, "{face:?}");
        }
    }

    #[test]
    fn families_differ_as_they_should() {
        let width = |face: Face, text: &str| Font::of(face).width(text);
        // the two with every letter the same width, and the characters of the East two letters wide
        for (face, cell) in [(Face::Pica, 7), (Face::Elite, 8)] {
            assert_eq!((width(face, "i"), width(face, "M"), width(face, " ")), (cell, cell, cell), "{face:?}");
            assert_eq!(width(face, "\u{4f60}"), 2 * cell, "{face:?}");
        }
        // elsewhere letters take the room they need, and those characters the square they were drawn in
        assert!(width(Face::Cicero, "i") < width(Face::Cicero, "M"));
        assert_eq!(width(Face::Cicero, "\u{4f60}"), 12);
        // the bold is wider than the light it was made from, and the small face narrower than the large
        assert!(width(Face::AgateBold, "Hamburg") > width(Face::Agate, "Hamburg"));
        assert!(width(Face::Agate, "Hamburg") < width(Face::Cicero, "Hamburg"));
        // the face of the interface is the default
        assert_eq!(Font::new().width("File"), width(Face::Cicero, "File"));
    }

    #[test]
    fn a_letter_nobody_has_still_takes_its_room() {
        let mut font = Font::new();
        // a private-use character: in no bundled family
        let nobody = '\u{f8ff}';
        assert!(!font.has(nobody));
        assert!(font.advance(nobody) >= 0);
    }
}
