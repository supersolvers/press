// SPDX-License-Identifier: GPL-3.0-or-later
//! RTF, the Rich Text Format word processors have long traded in: read
//! into a `rich::Document`, and written from one.
//!
//! What is read is what a document of words has: the fonts and colours
//! tables, a character's font, size, bold, italic, underline, strike and
//! colour, a paragraph's alignment, indents and bullet (the old way, with
//! `\pn`, and the newer, with a list), tabs, line breaks, the named
//! punctuation, characters in the code page and in Unicode. What a word
//! processor keeps besides (style sheets, headers and footers, pictures,
//! fields, its notes about the document) is passed over. What is written
//! is RTF any of them will read: the tables, then each paragraph with its
//! settings and each stretch of words with its look.

use crate::rich::{Document, Justify, Look, Paragraph};

/// The characters of Windows code page 1252 from 0x80 to 0x9F, which is
/// where it differs from Latin-1.
const CP1252: [char; 32] = [
    '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}', '\u{017D}', '\u{FFFD}',
    '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
];

fn from_cp1252(b: u8) -> char {
    match b {
        0x80..=0x9F => CP1252[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// The state a group carries: the look of characters, the paragraph's
/// settings, and how many characters stand in for each `\u`.
#[derive(Clone)]
struct State {
    look: Look,
    font: i32,
    paragraph: Paragraph,
    skip: usize,
    /// Inside something that is not words of the document.
    hidden: bool,
}

/// A word processor's own families, as fontconfig would know them.
fn family_for(name: &str, kind: &str) -> String {
    let name = name.trim().trim_end_matches(';').trim();
    if !name.is_empty() {
        return name.to_string();
    }
    match kind {
        "froman" => "Serif",
        "fmodern" => "Monospace",
        _ => "Sans",
    }
    .into()
}

/// Read RTF. What cannot be read as RTF is taken as plain text.
pub fn read(input: &str) -> Document {
    let bytes = input.as_bytes();
    if !input.trim_start().starts_with("{\\rtf") {
        return Document::plain(input, Look::default());
    }
    let mut fonts: Vec<(i32, String)> = Vec::new();
    let mut colours: Vec<u32> = Vec::new();
    let mut document = Document::new(Look::default());
    let mut paragraph_set = false;
    let mut stack: Vec<State> = Vec::new();
    let mut state = State { look: Look::default(), font: -1, paragraph: Paragraph::default(), skip: 1, hidden: false };
    // what the current group is, if it is one of the tables
    #[derive(Clone, Copy, PartialEq)]
    enum Table {
        None,
        Fonts,
        Font,
        Colours,
    }
    let mut table = Table::None;
    let mut tables: Vec<Table> = Vec::new();
    let mut font_entry: (i32, String, String) = (0, String::new(), String::new());
    let mut colour = (0u32, 0u32, 0u32, false);
    let mut pending_skip = 0usize;
    // a list's bullets: which overrides (\lsN) are bulleted
    let mut words = String::new();
    let mut first_group = true;
    let mut i = 0;

    // put down what has been read of the words, in the look it has
    let flush = |document: &mut Document, words: &mut String, state: &State| {
        if !words.is_empty() {
            let at = document.len();
            document.insert(at, words, &state.look);
            words.clear();
        }
    };

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'{' => {
                flush(&mut document, &mut words, &state);
                stack.push(state.clone());
                tables.push(table);
                if table == Table::Fonts {
                    table = Table::Font;
                    font_entry = (0, String::new(), String::new());
                }
                i += 1;
                // a group that starts with \* is something this reader may pass over
                if bytes.get(i..i + 2) == Some(b"\\*") {
                    state.hidden = true;
                    i += 2;
                }
                if first_group {
                    first_group = false;
                }
            }
            b'}' => {
                flush(&mut document, &mut words, &state);
                if table == Table::Font {
                    let (n, name, kind) = std::mem::take(&mut font_entry);
                    fonts.push((n, family_for(&name, &kind)));
                }
                table = tables.pop().unwrap_or(Table::None);
                if let Some(s) = stack.pop() {
                    // a paragraph's settings last until \pard, whatever the groups
                    let paragraph = state.paragraph;
                    state = s;
                    state.paragraph = paragraph;
                }
                i += 1;
            }
            b'\\' => {
                i += 1;
                let Some(&c) = bytes.get(i) else { break };
                if !c.is_ascii_alphabetic() {
                    i += 1;
                    let ch = match c {
                        b'\\' | b'{' | b'}' => Some(c as char),
                        b'~' => Some('\u{00A0}'),
                        b'-' => None,
                        b'_' => Some('\u{2011}'),
                        b'\'' => {
                            let hex = input.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
                            i += 2;
                            if pending_skip > 0 {
                                pending_skip -= 1;
                                None
                            } else {
                                hex.map(from_cp1252)
                            }
                        }
                        b'\n' | b'\r' => Some('\n'),
                        _ => None,
                    };
                    if let Some(ch) = ch {
                        if table == Table::Font {
                            font_entry.1.push(ch);
                        } else if !state.hidden && table == Table::None {
                            words.push(ch);
                        }
                    }
                    continue;
                }
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = &input[start..i];
                let num_start = i;
                if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let param: Option<i32> = input[num_start..i].parse().ok();
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
                let on = param.is_none_or(|p| p != 0);
                // words of the document, not of a table or something passed over
                let visible = !state.hidden && table == Table::None;
                // what changes the look puts down the words before it first
                let look_word = matches!(word, "b" | "i" | "ul" | "ulnone" | "uld" | "uldb" | "ulw" | "strike" | "fs" | "f" | "cf" | "plain");
                if look_word || matches!(word, "par" | "pard") {
                    flush(&mut document, &mut words, &state);
                }
                match word {
                    // groups that hold no words of the document
                    "fonttbl" => {
                        table = Table::Fonts;
                        state.hidden = true;
                    }
                    "colortbl" => {
                        table = Table::Colours;
                        state.hidden = true;
                        colour = (0, 0, 0, false);
                    }
                    "stylesheet" | "info" | "pict" | "header" | "footer" | "headerl" | "headerr" | "headerf" | "footerl" | "footerr" | "footerf" | "footnote" | "listtable" | "listoverridetable" | "pntext" | "pntxtb" | "pntxta" | "fldinst" | "object" | "themedata" | "colorschememapping" | "latentstyles" | "datastore" | "xmlnstbl" | "rsidtbl" | "generator" | "mmathPr" | "filetbl" | "revtbl" => {
                        state.hidden = true;
                    }
                    "f" if table == Table::Font => font_entry.0 = param.unwrap_or(0),
                    "fnil" | "froman" | "fswiss" | "fmodern" | "fscript" | "fdecor" | "ftech" | "fbidi" if table == Table::Font => font_entry.2 = word.into(),
                    "red" if table == Table::Colours => (colour.0, colour.3) = (param.unwrap_or(0).clamp(0, 255) as u32, true),
                    "green" if table == Table::Colours => (colour.1, colour.3) = (param.unwrap_or(0).clamp(0, 255) as u32, true),
                    "blue" if table == Table::Colours => (colour.2, colour.3) = (param.unwrap_or(0).clamp(0, 255) as u32, true),
                    "plain" => {
                        let family = state.look.family.clone();
                        state.look = Look { family, ..Look::default() };
                    }
                    "b" => state.look.bold = on,
                    "i" => state.look.italic = on,
                    "ul" | "uld" | "uldb" | "ulw" | "ulth" | "ulwave" => state.look.underline = on,
                    "ulnone" => state.look.underline = false,
                    "strike" | "striked" => state.look.strike = on,
                    "fs" => state.look.size = param.unwrap_or(24).clamp(2, 3200) as u16,
                    "f" => {
                        state.font = param.unwrap_or(0);
                        if let Some((_, name)) = fonts.iter().find(|(n, _)| *n == state.font) {
                            state.look.family = name.clone();
                        }
                    }
                    "cf" => state.look.colour = colours.get(param.unwrap_or(0) as usize).copied().unwrap_or(0),
                    "pard" => state.paragraph = Paragraph::default(),
                    "ql" => state.paragraph.justify = Justify::Left,
                    "qc" => state.paragraph.justify = Justify::Centre,
                    "qr" => state.paragraph.justify = Justify::Right,
                    "qj" => state.paragraph.justify = Justify::Full,
                    "li" => state.paragraph.left = param.unwrap_or(0),
                    "ri" => state.paragraph.right = param.unwrap_or(0),
                    "fi" => state.paragraph.first = param.unwrap_or(0),
                    // a bullet, the old way and the newer
                    "pnlvlblt" | "ls" => state.paragraph.bullet = true,
                    "par" => {
                        if !state.hidden {
                            let n = document.paragraph_of(document.len());
                            document.set_paragraph(n, state.paragraph);
                            paragraph_set = true;
                            let at = document.len();
                            document.insert(at, "\n", &state.look);
                        }
                    }
                    "line" if visible => words.push('\n'),
                    "tab" if visible => words.push('\t'),
                    "emdash" if visible => words.push('\u{2014}'),
                    "endash" if visible => words.push('\u{2013}'),
                    "bullet" if visible => words.push('\u{2022}'),
                    "lquote" if visible => words.push('\u{2018}'),
                    "rquote" if visible => words.push('\u{2019}'),
                    "ldblquote" if visible => words.push('\u{201C}'),
                    "rdblquote" if visible => words.push('\u{201D}'),
                    "emspace" | "enspace" | "qmspace" if visible => words.push(' '),
                    "uc" => state.skip = param.unwrap_or(1).max(0) as usize,
                    "u" => {
                        let n = param.unwrap_or(0);
                        let code = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                        if !state.hidden && table == Table::None {
                            if let Some(ch) = char::from_u32(code) {
                                words.push(ch);
                            }
                        } else if table == Table::Font {
                            if let Some(ch) = char::from_u32(code) {
                                font_entry.1.push(ch);
                            }
                        }
                        pending_skip = state.skip;
                    }
                    _ => {}
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                // plain characters: words, a font's name, or the end of a colour
                let ch = input[i..].chars().next().unwrap_or(' ');
                i += ch.len_utf8();
                if table == Table::Colours && ch == ';' {
                    colours.push(if colour.3 { colour.0 << 16 | colour.1 << 8 | colour.2 } else { 0 });
                    colour = (0, 0, 0, false);
                    continue;
                }
                if pending_skip > 0 {
                    pending_skip -= 1;
                    continue;
                }
                if table == Table::Font {
                    font_entry.1.push(ch);
                } else if !state.hidden && table == Table::None {
                    words.push(ch);
                }
            }
        }
    }
    flush(&mut document, &mut words, &state);
    // the last paragraph, ended or not, has the settings in force
    let n = document.paragraph_of(document.len());
    if !paragraph_set || !document.text().ends_with('\n') {
        document.set_paragraph(n, state.paragraph);
    }
    // a document ends with \par; that last break makes no paragraph of its own
    if document.text().ends_with('\n') {
        let len = document.len();
        document.delete(len - 1, len);
    }
    // the look of an empty document's first words: the first font's
    if document.is_empty() {
        if let Some((_, name)) = fonts.first() {
            document = Document::new(Look { family: name.clone(), ..Look::default() });
        }
    }
    document
}

/// Words written as RTF: what is not plain ASCII as `\u`, with a `?` for
/// readers that do not know it.
fn escape(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\t' => out.push_str("\\tab "),
            '\u{2022}' => out.push_str("\\bullet "),
            c if (c as u32) < 0x80 => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{}?", *unit as i16));
                }
            }
        }
    }
}

/// A document written as RTF.
pub fn write(document: &Document) -> String {
    let families = document.families();
    let mut colours: Vec<u32> = vec![0];
    for (_, _, look) in document.runs(0, document.len()) {
        if !colours.contains(&look.colour) {
            colours.push(look.colour);
        }
    }
    let mut out = String::from("{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1\n{\\fonttbl");
    for (n, family) in families.iter().enumerate() {
        let kind = match family.to_lowercase().as_str() {
            f if f.contains("serif") && !f.contains("sans") => "froman",
            f if f.contains("mono") || f.contains("courier") => "fmodern",
            _ => "fswiss",
        };
        out.push_str(&format!("{{\\f{n}\\{kind} "));
        escape(family, &mut out);
        out.push_str(";}");
    }
    // the bullet's font, for readers that draw it with one
    let symbol = families.len();
    out.push_str(&format!("{{\\f{symbol}\\fnil\\fcharset2 Symbol;}}}}\n{{\\colortbl ;"));
    for c in &colours[1..] {
        out.push_str(&format!("\\red{}\\green{}\\blue{};", c >> 16 & 255, c >> 8 & 255, c & 255));
    }
    out.push_str("}\n{\\*\\generator PRESS;}\\viewkind4\n");
    let paragraphs = document.paragraphs();
    for (start, end, p) in paragraphs.iter() {
        out.push_str("\\pard");
        if p.bullet {
            out.push_str(&format!("{{\\pntext\\f{symbol}\\'B7\\tab}}{{\\*\\pn\\pnlvlblt\\pnf{symbol}\\pnindent0{{\\pntxtb\\'B7}}}}"));
        }
        match p.justify {
            Justify::Left => {}
            Justify::Centre => out.push_str("\\qc"),
            Justify::Right => out.push_str("\\qr"),
            Justify::Full => out.push_str("\\qj"),
        }
        if p.left != 0 {
            out.push_str(&format!("\\li{}", p.left));
        }
        if p.right != 0 {
            out.push_str(&format!("\\ri{}", p.right));
        }
        if p.first != 0 {
            out.push_str(&format!("\\fi{}", p.first));
        }
        out.push(' ');
        let runs = document.runs(*start, *end);
        if runs.is_empty() {
            // an empty paragraph keeps the size of what was typed in it
            let look = document.look_at(*start);
            out.push_str(&format!("\\fs{} ", look.size));
        }
        for (from, to, look) in runs {
            let f = families.iter().position(|x| *x == look.family).unwrap_or(0);
            let cf = colours.iter().position(|c| *c == look.colour).unwrap_or(0);
            out.push_str(&format!("{{\\f{f}\\fs{}", look.size));
            if look.bold {
                out.push_str("\\b");
            }
            if look.italic {
                out.push_str("\\i");
            }
            if look.underline {
                out.push_str("\\ul");
            }
            if look.strike {
                out.push_str("\\strike");
            }
            if cf != 0 {
                out.push_str(&format!("\\cf{cf}"));
            }
            out.push(' ');
            escape(&document.text()[from..to], &mut out);
            out.push('}');
        }
        // every paragraph ends \par, the last too: readers take it as the end of the words
        out.push_str("\\par\n");
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_written_reads_back_the_same() {
        let mut d = Document::plain("Plain and bold\nCentred, in red\nA bullet: caf\u{e9} \u{2014} \u{65e5}\u{672c}", Look::default());
        d.restyle(10, 14, |l| l.bold = true);
        d.restyle(15, 30, |l| {
            l.colour = 0xc00000;
            l.family = "DejaVu Serif".into();
            l.size = 32;
            l.italic = true;
        });
        d.reset_paragraphs(15, 15, |p| p.justify = Justify::Centre);
        d.reset_paragraphs(31, 31, |p| {
            p.bullet = true;
            p.left = 360;
            p.first = -360;
        });
        let back = read(&write(&d));
        assert_eq!(back.text(), d.text());
        assert!(back.look_at(10).bold && !back.look_at(9).bold);
        let red = back.look_at(16);
        assert_eq!((red.colour, red.family.as_str(), red.size, red.italic), (0xc00000, "DejaVu Serif", 32, true));
        assert_eq!(back.paragraph(1).justify, Justify::Centre);
        let bullet = back.paragraph(2);
        assert!(bullet.bullet);
        assert_eq!((bullet.left, bullet.first), (360, -360));
        assert_eq!(back.paragraph(0), &Paragraph::default());
    }

    #[test]
    fn rtf_as_word_processors_write_it_is_read() {
        // in the manner of the period's word pad: tables, \pard, \plain, code-page characters
        let wordpad = r#"{\rtf1\ansi\ansicpg1252\deff0\deflang2057{\fonttbl{\f0\fswiss\fcharset0 Arial;}{\f1\froman\fprq2\fcharset0 Times New Roman;}}
{\colortbl ;\red0\green0\blue255;}
{\*\generator Riched20 10.0;}\viewkind4\uc1
\pard\qc\f0\fs20 Title \b bold\b0\par
\pard\cf1\f1\fs24 Caf\'e9 \u8364?5\cf0\par
{\pntext\f0\'B7\tab}{\*\pn\pnlvlblt\pnf0\pnindent0{\pntxtb\'B7}}\fi-360\li720 item\par
}"#;
        let d = read(wordpad);
        assert_eq!(d.text(), "Title bold\nCaf\u{e9} \u{20ac}5\nitem");
        assert_eq!(d.paragraph(0).justify, Justify::Centre);
        assert!(d.look_at(6).bold && !d.look_at(0).bold);
        assert_eq!(d.look_at(0).family, "Arial");
        assert_eq!(d.look_at(0).size, 20);
        let cafe = d.look_at(11);
        assert_eq!((cafe.family.as_str(), cafe.colour, cafe.size), ("Times New Roman", 0x0000ff, 24));
        assert!(d.paragraph(2).bullet);
        assert_eq!(d.paragraph(2).left, 720);
        // what is not RTF is plain text
        assert_eq!(read("just words").text(), "just words");
    }
}
