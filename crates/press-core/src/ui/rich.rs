// SPDX-License-Identifier: GPL-3.0-or-later
//! The rich text widget: a document of words in fonts and sizes of their
//! own, bold, italic, underlined, struck through and in colour, in
//! paragraphs aligned, indented and bulleted, edited as a word processor
//! edits (see `crate::rich` for the document, and `crate::rtf` for how it
//! is read and written).
//!
//! It is laid out line by line, each word in its own font, drawn smooth
//! from the machine's fonts; lines break at word gaps to fit the widget,
//! or a width the program sets, as a page would. It scrolls up and down,
//! with a bar of its own as a text area has.
//!
//! Copying puts the selection on the clipboard as RTF as well as plain
//! text; pasting takes RTF where the clipboard has it, and plain text
//! where it has not.

use super::*;
use crate::rich::{Document, Editor, Justify, Look};
use std::sync::{Arc, Mutex, OnceLock};

/// The room left between the frame and the words.
const MARGIN: i32 = 4;
/// Tab stops, every half inch.
const TAB: i32 = 48;
/// How far in the words of a bulleted paragraph start from its bullet,
/// when the paragraph does not say.
const BULLET_ROOM: i32 = 24;
/// How far past the end of a line a selection that takes in its break is drawn.
const BREAK_WIDTH: i32 = 5;

/// What the widget keeps.
#[derive(Clone)]
pub(super) struct RichState {
    pub edit: Editor,
    /// How far down it is scrolled, in pixels.
    pub scroll: i32,
    /// Scroll to keep the caret in view; off once the wheel or the bar has
    /// moved it, until the caret moves.
    pub follow: bool,
    /// The place across a run of up and down keys keeps to.
    pub goal_x: Option<i32>,
    /// The selection the program last knew of.
    pub reported: (usize, usize),
    /// Lines break at this many pixels, as on a page; 0, at the widget's width.
    pub wrap: i32,
    /// The document as last laid out, and at what width.
    layout: Option<Arc<Layout>>,
    laid: Option<(i32, u64)>,
    /// Raised at every change to the document, so the layout is made again.
    pub version: u64,
}

impl RichState {
    pub fn new(document: Document) -> RichState {
        RichState { edit: Editor::new(document), scroll: 0, follow: true, goal_x: None, reported: (0, 0), wrap: 0, layout: None, laid: None, version: 0 }
    }
}

/// One letter laid out: where it is in the text, where across, how wide,
/// and which of the layout's faces it is drawn in.
#[derive(Clone, Copy)]
struct Cell {
    at: usize,
    ch: char,
    x: i32,
    w: i32,
    face: usize,
}

/// A line as laid out: the text it holds, where it is down the document,
/// how tall it is and where its baseline is, and its letters.
#[derive(Clone)]
struct Line {
    start: usize,
    end: usize,
    y: i32,
    height: i32,
    ascent: i32,
    /// Where the caret goes at the start of the line.
    x: i32,
    cells: Vec<Cell>,
    /// Where to draw a bullet, for the first line of a bulleted paragraph.
    bullet: Option<i32>,
    /// The line ends in a line break, rather than wrapping into the next.
    breaks: bool,
}

/// A face to draw letters in: the font, its size in pixels, the look.
struct Face {
    font: Arc<fontdue::Font>,
    px: f32,
    look: Look,
    ascent: i32,
    descent: i32,
    gap: i32,
}

pub(super) struct Layout {
    lines: Vec<Line>,
    faces: Vec<Face>,
    height: i32,
}

/// Letters drawn, kept: by face, character and size.
type Glyphs = std::collections::HashMap<(usize, char, u32), (fontdue::Metrics, Vec<u8>)>;

fn glyphs() -> &'static Mutex<Glyphs> {
    static GLYPHS: OnceLock<Mutex<Glyphs>> = OnceLock::new();
    GLYPHS.get_or_init(Default::default)
}

/// The face for a look, falling back to any sans and then to the
/// engine's fallback for a character, so something is always drawn.
fn font_for(look: &Look) -> Option<Arc<fontdue::Font>> {
    crate::font::styled(&look.family, look.bold, look.italic).or_else(|| crate::font::styled("Sans", look.bold, look.italic)).or_else(|| crate::font::fallback('a'))
}

fn glyph(font: &Arc<fontdue::Font>, ch: char, px: f32) -> (fontdue::Metrics, Vec<u8>) {
    let key = (Arc::as_ptr(font) as usize, ch, px.to_bits());
    let mut all = glyphs().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(g) = all.get(&key) {
        return g.clone();
    }
    if all.len() > 20_000 {
        all.clear();
    }
    let g = font.rasterize(ch, px);
    all.insert(key, g.clone());
    g
}

/// Lay a document out at a width: each paragraph's lines, broken at word
/// gaps, set across as the paragraph says.
fn lay_out(document: &Document, width: i32) -> Layout {
    let mut faces: Vec<Face> = Vec::new();
    let face_of = |look: &Look, faces: &mut Vec<Face>| -> usize {
        if let Some(i) = faces.iter().position(|f| f.look == *look) {
            return i;
        }
        let font = font_for(look).expect("a font of some kind");
        let px = look.pixels().max(1.0);
        let m = font.horizontal_line_metrics(px);
        let (ascent, descent, gap) = m.map_or((px as i32, (px / 4.0) as i32, 0), |m| (m.ascent.ceil() as i32, (-m.descent).ceil() as i32, m.line_gap.round() as i32));
        faces.push(Face { font, px, look: look.clone(), ascent, descent, gap });
        faces.len() - 1
    };
    let mut lines = Vec::new();
    let mut y = MARGIN;
    let text = document.text();
    for (start, end, p) in document.paragraphs() {
        let left = MARGIN + crate::rich::twips_to_pixels(p.left);
        let right = (width - MARGIN - crate::rich::twips_to_pixels(p.right)).max(left + 8);
        let mut first = left + crate::rich::twips_to_pixels(p.first);
        let mut rest = left;
        let mut bullet = None;
        if p.bullet {
            // the bullet hangs where the first line would start, the words after it
            if first >= left {
                bullet = Some(left);
                first = left + BULLET_ROOM;
                rest = first;
            } else {
                bullet = Some(first.max(MARGIN));
                first = left;
            }
        }
        // the paragraph's letters, each in its face, with its advance
        let mut letters: Vec<Cell> = Vec::new();
        for (from, to, look) in document.runs(start, end) {
            let face = face_of(look, &mut faces);
            for (i, ch) in text[from..to].char_indices() {
                let w = if ch == '\t' { 0 } else { faces[face].font.metrics(ch, faces[face].px).advance_width.round() as i32 };
                letters.push(Cell { at: from + i, ch, x: 0, w, face });
            }
        }
        let empty_face = face_of(&document.look_for_typing(start), &mut faces);
        // break into lines
        let mut index = 0;
        let mut is_first = true;
        loop {
            let x0 = if is_first { first } else { rest };
            let mut x = x0;
            let mut cut = None::<usize>; // after the last space that fits
            let mut j = index;
            while j < letters.len() {
                let c = letters[j];
                let w = if c.ch == '\t' { ((x - x0) / TAB + 1) * TAB + x0 - x } else { c.w };
                if x + w > right && j > index && c.ch != ' ' {
                    break;
                }
                letters[j].x = x;
                letters[j].w = w;
                x += w;
                if c.ch == ' ' || c.ch == '\t' {
                    cut = Some(j + 1);
                }
                j += 1;
            }
            let end_at = if j >= letters.len() { j } else { cut.filter(|c| *c > index).unwrap_or(j) };
            let cells: Vec<Cell> = letters[index..end_at].to_vec();
            let line_start = cells.first().map_or(if index < letters.len() { letters[index].at } else { end }, |c| c.at);
            let line_end = if end_at >= letters.len() { end } else { letters[end_at].at };
            let breaks = end_at >= letters.len();
            // how tall: the tallest face on it
            let used: Vec<usize> = if cells.is_empty() { vec![empty_face] } else { cells.iter().map(|c| c.face).collect() };
            let ascent = used.iter().map(|f| faces[*f].ascent).max().unwrap_or(10);
            let below = used.iter().map(|f| faces[*f].descent + faces[*f].gap.max(0)).max().unwrap_or(3);
            let mut line = Line { start: line_start, end: line_end, y, height: ascent + below, ascent, x: x0, cells, bullet: if is_first { bullet } else { None }, breaks };
            // across, as the paragraph says; its spaces at the end hang off it
            let used_width = line.cells.iter().rev().skip_while(|c| c.ch == ' ').map(|c| c.x + c.w).next().unwrap_or(x0) - x0;
            let spare = (right - x0 - used_width).max(0);
            let shift = match p.justify {
                Justify::Left => 0,
                Justify::Centre => spare / 2,
                Justify::Right => spare,
                Justify::Full => 0,
            };
            if p.justify == Justify::Full && !breaks {
                let gaps: Vec<usize> = line.cells.iter().enumerate().filter(|(_, c)| c.ch == ' ').map(|(i, _)| i).filter(|i| *i + 1 < line.cells.len()).collect();
                if !gaps.is_empty() {
                    let mut extra_done = 0;
                    for (n, cell) in line.cells.iter_mut().enumerate() {
                        let before = gaps.iter().filter(|g| **g < n).count() as i32;
                        let extra = spare * before / gaps.len() as i32;
                        cell.x += extra;
                        extra_done = extra;
                    }
                    let _ = extra_done;
                }
            }
            if shift > 0 {
                line.x += shift;
                for c in &mut line.cells {
                    c.x += shift;
                }
            }
            y += line.height;
            lines.push(line);
            index = end_at;
            is_first = false;
            if index >= letters.len() {
                break;
            }
        }
    }
    Layout { lines, faces, height: y + MARGIN }
}

impl Layout {
    /// The line a place is drawn on: a place where one line wraps into the
    /// next belongs to the next.
    fn line_of(&self, at: usize) -> usize {
        let mut found = 0;
        for (i, l) in self.lines.iter().enumerate() {
            if l.start <= at {
                found = i;
            } else {
                break;
            }
        }
        // the end of a line that breaks belongs to it, not to the empty line after
        found
    }

    /// Where across a place is drawn, on its line.
    fn x_of(&self, row: usize, at: usize) -> i32 {
        let line = &self.lines[row];
        for c in &line.cells {
            if c.at >= at {
                return c.x;
            }
        }
        line.cells.last().map_or(line.x, |c| c.x + c.w)
    }

    /// The place on a line nearest to a point across.
    fn index_in(&self, row: usize, x: i32) -> usize {
        let line = &self.lines[row];
        for c in &line.cells {
            if x < c.x + c.w / 2 {
                return c.at;
            }
        }
        // past the last letter: before the wrap's last space, or at the end
        match line.cells.last() {
            Some(c) if !line.breaks && c.ch == ' ' => c.at,
            Some(c) if !line.breaks => c.at + c.ch.len_utf8(),
            _ => line.end,
        }
    }

    /// The line at a point down the document.
    fn row_at(&self, y: i32) -> usize {
        self.lines.iter().position(|l| y < l.y + l.height).unwrap_or(self.lines.len().saturating_sub(1))
    }
}

impl Ui {
    pub fn rich_text_new(&mut self) -> Handle {
        self.add(Kind::RichText(Box::new(RichState::new(Document::new(Look::default())))))
    }

    fn rich(&self, h: Handle) -> Result<&RichState> {
        match &self.node(h)?.kind {
            Kind::RichText(state) => Ok(state),
            _ => Err(Error::WrongType),
        }
    }

    fn rich_mut(&mut self, h: Handle) -> Result<&mut RichState> {
        self.dirty = true;
        match &mut self.node_mut(h)?.kind {
            Kind::RichText(state) => Ok(state),
            _ => Err(Error::WrongType),
        }
    }

    pub(super) fn is_rich(&self, h: Handle) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::RichText(_)))
    }

    /// The part inside the frame, less the bar, where the words are.
    pub(super) fn rich_inner(&self, h: Handle) -> Option<Rect> {
        let node = self.nodes.get(h)?;
        let inner = field_inner(node.rect, &self.look.m);
        let bar = Self::area_bar_rect(node.rect, self.look.m.scroll_bar);
        Some(Rect::new(inner.x, inner.y, (bar.x - 1 - inner.x).max(0), inner.h))
    }

    /// The document laid out at the width it is shown at.
    fn rich_layout(&mut self, h: Handle) -> Option<Arc<Layout>> {
        let inner = self.rich_inner(h)?;
        let Some(Kind::RichText(state)) = self.nodes.get_mut(h).map(|n| &mut n.kind) else { return None };
        let width = if state.wrap > 0 { state.wrap + 2 * MARGIN } else { inner.w };
        if state.laid != Some((width, state.version)) || state.layout.is_none() {
            state.layout = Some(Arc::new(lay_out(&state.edit.document, width)));
            state.laid = Some((width, state.version));
        }
        state.layout.clone()
    }

    pub(super) fn measure_rich(&self, h: Handle) -> (i32, i32) {
        let _ = h;
        (240, 160)
    }

    /// The scroll bar down its side.
    pub(super) fn rich_bar(&mut self, h: Handle, enabled: bool) -> Option<scroll::Bar> {
        let rect = self.nodes.get(h)?.rect;
        let inner = self.rich_inner(h)?;
        let layout = self.rich_layout(h)?;
        let scroll = self.rich(h).ok()?.scroll;
        let max = (layout.height - inner.h).max(0);
        let line = self.font.line_height();
        Some(scroll::Bar { rect: Self::area_bar_rect(rect, self.look.m.scroll_bar), vertical: true, value: scroll.clamp(0, max), max, page: inner.h.max(1), step: line, enabled, square: false, together: self.look.m.scroll_arrows == 1 })
    }

    pub(super) fn rich_bar_moved(&mut self, h: Handle, to: i32) {
        if let Ok(state) = self.rich_mut(h) {
            state.scroll = to;
            state.follow = false;
        }
    }

    // ------------------------------------------------------------ drawing

    pub(super) fn paint_rich(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        self.part(canvas, Part::new(PartKind::Well, rect).numbers(2, 0, 0).when(state::FOCUSED, live));
        let Some(inner) = self.rich_inner(h) else { return };
        if inner.w <= 0 || inner.h <= 0 {
            return;
        }
        let Some(layout) = self.rich_layout(h) else { return };
        let Ok(state) = self.rich(h) else { return };
        let (anchor, caret) = (state.edit.anchor(), state.edit.caret());
        let (from, to) = state.edit.selection();
        let mut scroll = state.scroll;
        let follow = state.follow;
        // keep the caret in view when it has just moved
        let caret_row = layout.line_of(caret);
        if follow {
            let line = &layout.lines[caret_row];
            scroll = scroll.min(line.y).max(line.y + line.height - inner.h);
        }
        scroll = scroll.clamp(0, (layout.height - inner.h).max(0));
        let _ = anchor;
        let paper = self.look.pal.paper;
        let mut clip = Canvas::like(canvas, inner.w, inner.h, paper);
        for (row, line) in layout.lines.iter().enumerate() {
            let top = line.y - scroll;
            if top + line.height < 0 || top > inner.h {
                continue;
            }
            let base = top + line.ascent;
            if let Some(bx) = line.bullet {
                let face = line.cells.first().map_or(0, |c| c.face).min(layout.faces.len().saturating_sub(1));
                if let Some(f) = layout.faces.get(face) {
                    Self::draw_letter(&mut clip, f, '\u{2022}', bx, base);
                }
            }
            for c in &line.cells {
                let f = &layout.faces[c.face];
                if !matches!(c.ch, ' ' | '\t' | '\n') {
                    Self::draw_letter(&mut clip, f, c.ch, c.x, base);
                }
                let ink = 0xff00_0000 | f.look.colour;
                if f.look.underline {
                    clip.fill(Rect::new(c.x, base + (f.px / 9.0).ceil() as i32, c.w, (f.px / 14.0).ceil().max(1.0) as i32), ink);
                }
                if f.look.strike {
                    clip.fill(Rect::new(c.x, base - (f.px / 3.2) as i32, c.w, (f.px / 14.0).ceil().max(1.0) as i32), ink);
                }
            }
            // the selection over the words, and past the end of a line whose break it takes in
            if live && from < to && from <= line.end && to >= line.start && !(to == line.start && row > 0) {
                let x0 = layout.x_of(row, from.max(line.start));
                let mut x1 = layout.x_of(row, to.min(line.end));
                if to > line.end && line.breaks {
                    x1 += BREAK_WIDTH;
                }
                if x1 > x0 {
                    self.part(&mut clip, Part::new(PartKind::Select, Rect::new(x0, top, x1 - x0, line.height)).numbers(1, 0, 0));
                }
            }
            if live && from == to && row == caret_row && self.caret_on() {
                let x = layout.x_of(row, caret);
                self.part(&mut clip, Part::new(PartKind::Caret, Rect::new(x, top, 1, line.height)).ink(self.look.pal.ink));
            }
        }
        canvas.blit(&clip, inner.x, inner.y);
        if let Ok(state) = self.rich_mut(h) {
            state.scroll = scroll;
        }
        self.bar_draw(h, false, canvas);
    }

    fn draw_letter(canvas: &mut Canvas, face: &Face, ch: char, x: i32, base: i32) {
        let font = if face.font.lookup_glyph_index(ch) == 0 { crate::font::fallback(ch).unwrap_or(face.font.clone()) } else { face.font.clone() };
        let (m, cover) = glyph(&font, ch, face.px);
        let ink = 0xff00_0000 | face.look.colour;
        let (left, top) = (x + m.xmin, base - m.height as i32 - m.ymin);
        for row in 0..m.height {
            for col in 0..m.width {
                let c = cover[row * m.width + col];
                if c > 0 {
                    canvas.blend(left + col as i32, top + row as i32, ink, c);
                }
            }
        }
    }

    // ------------------------------------------------------------ finding places

    /// The place in the document nearest a point in the window.
    fn rich_index_at(&mut self, h: Handle, x: i32, y: i32) -> usize {
        let (Some(inner), Some(layout)) = (self.rich_inner(h), self.rich_layout(h)) else { return 0 };
        let scroll = self.rich(h).map_or(0, |s| s.scroll);
        let row = layout.row_at(y - inner.y + scroll);
        layout.index_in(row, x - inner.x)
    }

    /// Where the caret is drawn, in the window.
    pub(super) fn rich_caret_rect(&mut self, h: Handle) -> Option<Rect> {
        let inner = self.rich_inner(h)?;
        let layout = self.rich_layout(h)?;
        let state = self.rich(h).ok()?;
        let row = layout.line_of(state.edit.caret());
        let line = &layout.lines[row];
        Some(Rect::new(inner.x + layout.x_of(row, state.edit.caret()), inner.y + line.y - state.scroll, 1, line.height))
    }

    /// Up or down a line (or a page), keeping to the place across.
    fn rich_vertical(&mut self, h: Handle, lines: i32, extend: bool) {
        let (Some(inner), Some(layout)) = (self.rich_inner(h), self.rich_layout(h)) else { return };
        let Ok(state) = self.rich(h) else { return };
        let caret = state.edit.caret();
        let row = layout.line_of(caret);
        let goal = state.goal_x.unwrap_or_else(|| layout.x_of(row, caret));
        let target = if lines.abs() > 1 {
            // a page: as many lines as fill the view
            let y = layout.lines[row].y + lines.signum() * inner.h;
            layout.row_at(y.max(0))
        } else {
            (row as i32 + lines).clamp(0, layout.lines.len() as i32 - 1) as usize
        };
        let at = if target == row && lines < 0 { 0 } else if target == row && lines > 0 { usize::MAX } else { layout.index_in(target, goal) };
        if let Ok(state) = self.rich_mut(h) {
            let at = at.min(state.edit.document.len());
            state.edit.move_to(at, extend);
            state.goal_x = Some(goal);
            state.follow = true;
        }
    }

    /// To the start or the end of the line the caret is drawn on.
    fn rich_line_edge(&mut self, h: Handle, end: bool, extend: bool) {
        let Some(layout) = self.rich_layout(h) else { return };
        let Ok(state) = self.rich(h) else { return };
        let row = layout.line_of(state.edit.caret());
        let line = &layout.lines[row];
        let at = if end { layout.index_in(row, i32::MAX) } else { line.start };
        if let Ok(state) = self.rich_mut(h) {
            state.edit.move_to(at, extend);
            state.follow = true;
        }
    }

    // ------------------------------------------------------------ reporting

    /// Say what has happened: the document changed, and where the
    /// selection now is, if the program has not heard.
    fn rich_report(&mut self, h: Handle, changed: bool) -> Vec<Emitted> {
        let mut out = Vec::new();
        self.caret_epoch = self.now;
        let Ok(state) = self.rich_mut(h) else { return out };
        if changed {
            state.version += 1;
            out.push(Emitted { target: h, event: "RichText.changed", args: vec![] });
        }
        state.follow = true;
        let now = state.edit.selection();
        if now != state.reported {
            state.reported = now;
            out.push(Emitted { target: h, event: "RichText.selected", args: vec![Value::I32(now.0 as i32), Value::I32(now.1 as i32)] });
        }
        out
    }

    // ------------------------------------------------------------ the pointer

    /// The pointer on a rich text widget: a click puts the caret, a drag
    /// selects, a double click takes a word and a third its paragraph;
    /// Shift with a click stretches the selection to it.
    pub(super) fn rich_pointer(&mut self, window: Handle, over: Option<Handle>, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        let held = self.pressed.map(|(h, _)| h).filter(|h| self.is_rich(*h));
        match kind {
            PointerKind::Press => {
                let h = over.filter(|h| self.is_rich(*h))?;
                if !self.rich_inner(h).is_some_and(|r| r.contains(x, y)) {
                    return None;
                }
                let count = self.clicks_at(x, y);
                self.pressed = Some((h, true));
                if self.resolve_focus(window) != Some(h) {
                    self.set_focus(window, Some(h), false);
                }
                let at = self.rich_index_at(h, x, y);
                let extend = self.mods.shift;
                if let Ok(state) = self.rich_mut(h) {
                    match count {
                        1 => state.edit.move_to(at, extend),
                        2 => state.edit.select_word_at(at),
                        _ => state.edit.select_paragraph_at(at),
                    }
                    state.goal_x = None;
                }
                Some(self.rich_report(h, false))
            }
            PointerKind::Move => {
                let h = held?;
                if self.clicks.3 == 1 {
                    let at = self.rich_index_at(h, x, y);
                    if let Ok(state) = self.rich_mut(h) {
                        state.edit.move_to(at, true);
                        state.goal_x = None;
                    }
                    return Some(self.rich_report(h, false));
                }
                Some(Vec::new())
            }
            PointerKind::Release => {
                let h = held?;
                self.pressed = None;
                Some(self.rich_report(h, false))
            }
            PointerKind::Leave => held.map(|_| Vec::new()),
        }
    }

    // ------------------------------------------------------------ keys

    /// A key for a focused rich text widget. None for a key it does not
    /// take (Escape, say), which the window then has.
    pub(super) fn rich_key(&mut self, h: Handle, key: Key, mods: Mods) -> Option<Vec<Emitted>> {
        let (word, extend) = (mods.ctrl, mods.shift);
        let mut changed = false;
        match key {
            Key::Up | Key::Down if !mods.ctrl => self.rich_vertical(h, if key == Key::Up { -1 } else { 1 }, extend),
            Key::PageUp | Key::PageDown => self.rich_vertical(h, if key == Key::PageUp { -2 } else { 2 }, extend),
            Key::Home | Key::End if !mods.ctrl => self.rich_line_edge(h, key == Key::End, extend),
            Key::Return if !mods.alt => changed = self.rich_mut(h).ok()?.edit.type_text("\n"),
            Key::Tab if !mods.ctrl && !mods.alt && !mods.shift => changed = self.rich_mut(h).ok()?.edit.type_text("\t"),
            Key::Char('c') if mods.ctrl => {
                self.rich_text_copy(h).ok()?;
                return Some(Vec::new());
            }
            Key::Char('x') if mods.ctrl => return self.rich_text_cut(h).ok(),
            Key::Char('v') if mods.ctrl => {
                self.rich_text_paste(h).ok()?;
                return Some(Vec::new());
            }
            Key::Char('z') if mods.ctrl => changed = self.rich_mut(h).ok()?.edit.undo(),
            Key::Char('y') if mods.ctrl => changed = self.rich_mut(h).ok()?.edit.redo(),
            Key::Char('b') if mods.ctrl => changed = self.rich_toggle(h, |l| &mut l.bold),
            Key::Char('i') if mods.ctrl => changed = self.rich_toggle(h, |l| &mut l.italic),
            Key::Char('u') if mods.ctrl => changed = self.rich_toggle(h, |l| &mut l.underline),
            _ => {
                let state = self.rich_mut(h).ok()?;
                let edit = &mut state.edit;
                match key {
                    Key::Left => edit.left(word, extend),
                    Key::Right => edit.right(word, extend),
                    Key::Home | Key::Up => edit.move_to(0, extend),
                    Key::End | Key::Down => edit.move_to(edit.document.len(), extend),
                    Key::Backspace => changed = edit.backspace(word),
                    Key::Delete => changed = edit.delete(word),
                    Key::Char('a') if mods.ctrl => edit.select_all(),
                    _ => return None,
                }
                state.goal_x = None;
            }
        }
        if !matches!(key, Key::Up | Key::Down | Key::PageUp | Key::PageDown) {
            if let Ok(state) = self.rich_mut(h) {
                state.goal_x = None;
            }
        }
        Some(self.rich_report(h, changed))
    }

    fn rich_toggle(&mut self, h: Handle, flag: fn(&mut Look) -> &mut bool) -> bool {
        let Ok(state) = self.rich_mut(h) else { return false };
        let on = !*flag(&mut state.edit.look());
        state.edit.restyle(|l| *flag(l) = on)
    }

    /// Text typed into a focused rich text widget.
    pub(super) fn rich_input(&mut self, h: Handle, text: &str) -> Option<Vec<Emitted>> {
        if !self.is_rich(h) {
            return None;
        }
        let changed = self.rich_mut(h).ok()?.edit.type_text(text);
        Some(self.rich_report(h, changed))
    }

    // ------------------------------------------------------------ the program's calls

    pub fn rich_text_set_rtf(&mut self, h: Handle, rtf: &str) -> Result<()> {
        let state = self.rich_mut(h)?;
        state.edit.replace_document(crate::rtf::read(rtf));
        state.version += 1;
        state.scroll = 0;
        state.reported = (0, 0);
        Ok(())
    }

    pub fn rich_text_rtf(&self, h: Handle) -> Result<String> {
        Ok(crate::rtf::write(&self.rich(h)?.edit.document))
    }

    pub fn rich_text_set_text(&mut self, h: Handle, text: &str) -> Result<()> {
        let state = self.rich_mut(h)?;
        let look = state.edit.document.look_at(0).clone();
        state.edit.replace_document(Document::plain(text, look));
        state.version += 1;
        state.scroll = 0;
        state.reported = (0, 0);
        Ok(())
    }

    pub fn rich_text_text(&self, h: Handle) -> Result<String> {
        Ok(self.rich(h)?.edit.document.text().to_string())
    }

    pub fn rich_text_set_wrap(&mut self, h: Handle, width: i32) -> Result<()> {
        self.rich_mut(h)?.wrap = width.max(0);
        Ok(())
    }

    pub fn rich_text_select(&mut self, h: Handle, from: i32, to: i32) -> Result<()> {
        let state = self.rich_mut(h)?;
        state.edit.select(from.max(0) as usize, to.max(0) as usize);
        state.reported = state.edit.selection();
        state.follow = true;
        state.goal_x = None;
        Ok(())
    }

    pub fn rich_text_selection(&self, h: Handle) -> Result<(i32, i32)> {
        let (a, b) = self.rich(h)?.edit.selection();
        Ok((a as i32, b as i32))
    }

    pub fn rich_text_select_all(&mut self, h: Handle) -> Result<()> {
        let state = self.rich_mut(h)?;
        state.edit.select_all();
        state.reported = state.edit.selection();
        Ok(())
    }

    /// Put the selection on the clipboard: as plain text, and as RTF beside it.
    pub fn rich_text_copy(&mut self, h: Handle) -> Result<()> {
        let state = self.rich(h)?;
        let (from, to) = state.edit.selection();
        if from == to {
            return Ok(());
        }
        let piece = state.edit.document.slice(from, to);
        self.copy_out = Some(piece.text().to_string());
        self.copy_data_out.push((true, "text/rtf".into(), crate::rtf::write(&piece).into_bytes()));
        Ok(())
    }

    pub fn rich_text_cut(&mut self, h: Handle) -> Result<Vec<Emitted>> {
        self.rich_text_copy(h)?;
        let changed = self.rich_mut(h)?.edit.delete_selection();
        Ok(self.rich_report(h, changed))
    }

    /// Ask for the clipboard: as RTF first; if it has none, as plain text.
    pub fn rich_text_paste(&mut self, h: Handle) -> Result<()> {
        self.rich(h)?;
        let window = self.window_of(h).ok_or(Error::InvalidState)?;
        self.rich_paste = Some(h);
        self.paste_data_wanted.push((window, "text/rtf".into()));
        Ok(())
    }

    /// What the clipboard held as RTF for a paste into a rich text widget
    /// has arrived: put it in, or, if there was none, ask for plain text.
    pub(super) fn rich_pasted(&mut self, window: Handle, kind: &str, data: &[u8]) -> Option<Vec<Emitted>> {
        let h = self.rich_paste.filter(|h| kind == "text/rtf" && self.window_of(*h) == Some(window))?;
        self.rich_paste = None;
        let text = String::from_utf8_lossy(data);
        if text.trim_start().starts_with("{\\rtf") {
            let piece = crate::rtf::read(&text);
            let changed = self.rich_mut(h).ok()?.edit.put_document(&piece);
            return Some(self.rich_report(h, changed));
        }
        self.paste_request = Some(h);
        Some(Vec::new())
    }

    /// Plain text pasted into a rich text widget: typed, line breaks and all.
    pub(super) fn rich_paste_text(&mut self, h: Handle, text: &str) -> Option<Vec<Emitted>> {
        if !self.is_rich(h) {
            return None;
        }
        let changed = self.rich_mut(h).ok()?.edit.type_text(text);
        Some(self.rich_report(h, changed))
    }

    pub fn rich_text_insert(&mut self, h: Handle, text: &str) -> Result<()> {
        let state = self.rich_mut(h)?;
        state.edit.type_text(text);
        state.version += 1;
        state.reported = state.edit.selection();
        state.follow = true;
        Ok(())
    }

    pub fn rich_text_undo(&mut self, h: Handle) -> Result<()> {
        let state = self.rich_mut(h)?;
        if state.edit.undo() {
            state.version += 1;
            state.reported = state.edit.selection();
        }
        Ok(())
    }

    pub fn rich_text_redo(&mut self, h: Handle) -> Result<()> {
        let state = self.rich_mut(h)?;
        if state.edit.redo() {
            state.version += 1;
            state.reported = state.edit.selection();
        }
        Ok(())
    }

    pub fn rich_text_can_undo(&self, h: Handle) -> Result<bool> {
        Ok(self.rich(h)?.edit.can_undo())
    }

    pub fn rich_text_can_redo(&self, h: Handle) -> Result<bool> {
        Ok(self.rich(h)?.edit.can_redo())
    }

    /// Change the look of the selection, or of what is typed next.
    pub fn rich_text_restyle(&mut self, h: Handle, change: impl Fn(&mut Look)) -> Result<()> {
        let state = self.rich_mut(h)?;
        if state.edit.restyle(change) {
            state.version += 1;
        }
        Ok(())
    }

    pub fn rich_text_reset_paragraphs(&mut self, h: Handle, change: impl Fn(&mut crate::rich::Paragraph)) -> Result<()> {
        let state = self.rich_mut(h)?;
        state.edit.reset_paragraphs(change);
        state.version += 1;
        Ok(())
    }

    /// The look of the selection, or of what is typed next.
    pub fn rich_text_look(&self, h: Handle) -> Result<Look> {
        Ok(self.rich(h)?.edit.look())
    }

    /// The settings of the paragraph the caret is in.
    pub fn rich_text_paragraph(&self, h: Handle) -> Result<crate::rich::Paragraph> {
        Ok(self.rich(h)?.edit.paragraph())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };
    const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false };

    fn window() -> (Ui, Handle, Handle) {
        let mut ui = Ui::natural();
        let win = ui.window_new("Letter", 320, 240).unwrap();
        let rich = ui.rich_text_new();
        ui.set_stretch(rich, true).unwrap();
        ui.window_set_content(win, rich).unwrap();
        ui.render(win).unwrap();
        ui.focus(rich).unwrap();
        (ui, win, rich)
    }

    #[test]
    fn words_typed_take_the_look_chosen_and_are_written_as_rtf() {
        let (mut ui, win, rich) = window();
        let out = ui.text(win, "Plain ").unwrap();
        assert!(out.iter().any(|e| e.event == "RichText.changed"));
        assert!(out.iter().any(|e| e.event == "RichText.selected"));
        // Control and B with nothing selected: what is typed next is bold
        ui.key(win, Key::Char('b'), CTRL).unwrap();
        ui.text(win, "bold").unwrap();
        assert_eq!(ui.rich_text_text(rich).unwrap(), "Plain bold");
        let rtf = ui.rich_text_rtf(rich).unwrap();
        assert!(rtf.contains("\\b bold}") && rtf.contains(" Plain }"), "{rtf}");
        // and back in, the same
        ui.rich_text_set_rtf(rich, &rtf).unwrap();
        ui.rich_text_select(rich, 6, 10).unwrap();
        assert!(ui.rich_text_look(rich).unwrap().bold);
        ui.rich_text_select(rich, 0, 5).unwrap();
        assert!(!ui.rich_text_look(rich).unwrap().bold);
        // it draws
        ui.render(win).unwrap();
    }

    #[test]
    fn long_words_wrap_and_the_keys_move_by_the_lines_drawn() {
        let (mut ui, win, rich) = window();
        ui.text(win, &"word ".repeat(60)).unwrap();
        let layout = ui.rich_layout(rich).unwrap();
        assert!(layout.lines.len() > 3);
        // every line within the width
        let inner = ui.rich_inner(rich).unwrap();
        assert!(layout.lines.iter().all(|l| l.cells.iter().filter(|c| c.ch != ' ').all(|c| c.x + c.w <= inner.w)));
        ui.key(win, Key::Home, CTRL).unwrap();
        ui.key(win, Key::Down, NONE).unwrap();
        let caret = ui.rich_text_selection(rich).unwrap().0 as usize;
        assert_eq!(layout.line_of(caret), 1);
        ui.key(win, Key::End, NONE).unwrap();
        let end = ui.rich_text_selection(rich).unwrap().0 as usize;
        assert_eq!(layout.line_of(end), 1);
        // a click lands under the pointer
        let first = &layout.lines[0];
        let c = first.cells[2];
        ui.pointer(win, inner.x + c.x + 1, inner.y + first.y + 2, PointerKind::Press).unwrap();
        ui.pointer(win, inner.x + c.x + 1, inner.y + first.y + 2, PointerKind::Release).unwrap();
        assert_eq!(ui.rich_text_selection(rich).unwrap(), (c.at as i32, c.at as i32));
        ui.render(win).unwrap();
    }

    #[test]
    fn copying_offers_rtf_and_pasting_takes_it() {
        let (mut ui, win, rich) = window();
        ui.text(win, "one two").unwrap();
        ui.rich_text_select(rich, 4, 7).unwrap();
        ui.rich_text_restyle(rich, |l| l.italic = true).unwrap();
        ui.key(win, Key::Char('c'), CTRL).unwrap();
        assert_eq!(ui.take_copy().as_deref(), Some("two"));
        let copies = ui.take_copy_data();
        let (also, kind, rtf) = &copies[0];
        assert!(*also && kind == "text/rtf");
        // pasted at the start: asked for as RTF, and put in with its look
        ui.key(win, Key::Home, CTRL).unwrap();
        ui.key(win, Key::Char('v'), CTRL).unwrap();
        assert_eq!(ui.take_paste_data_requests(), vec![(win, "text/rtf".to_string())]);
        let out = ui.paste_data_into(win, "text/rtf", rtf.clone()).unwrap();
        assert!(out.iter().any(|e| e.event == "RichText.changed"));
        assert_eq!(ui.rich_text_text(rich).unwrap(), "twoone two");
        ui.rich_text_select(rich, 0, 3).unwrap();
        assert!(ui.rich_text_look(rich).unwrap().italic);
        // with no RTF on the clipboard, plain text is asked for instead
        ui.key(win, Key::Char('v'), CTRL).unwrap();
        ui.take_paste_data_requests();
        ui.paste_data_into(win, "text/rtf", Vec::new()).unwrap();
        assert_eq!(ui.take_paste_request(), Some(rich));
        ui.paste_into(rich, "A\nB").unwrap();
        assert!(ui.rich_text_text(rich).unwrap().starts_with("A\nB"));
    }

    #[test]
    fn paragraphs_are_set_as_they_say() {
        let (mut ui, win, rich) = window();
        ui.text(win, "middle").unwrap();
        ui.rich_text_reset_paragraphs(rich, |p| p.justify = Justify::Centre).unwrap();
        let layout = ui.rich_layout(rich).unwrap();
        let inner = ui.rich_inner(rich).unwrap();
        let line = &layout.lines[0];
        let used = line.cells.last().map(|c| c.x + c.w).unwrap() - line.cells[0].x;
        // as far in from the left as the right, near enough
        assert!((line.cells[0].x - (inner.w - line.cells[0].x - used)).abs() <= 2 * MARGIN + 1);
        ui.rich_text_reset_paragraphs(rich, |p| p.bullet = true).unwrap();
        let layout = ui.rich_layout(rich).unwrap();
        assert!(layout.lines[0].bullet.is_some());
    }
}
