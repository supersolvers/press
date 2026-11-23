// SPDX-License-Identifier: GPL-3.0-or-later
//! Take the pixels out of the press font files (built in fonts/) and write them in
//! the compact form press bundles.
//!
//!     cargo run -p press-fontgen -- fonts/build assets/fonts      (make fonts does this)
//!
//! Each of those fonts carries, beside its outlines, the same letters as
//! hand-tuned pixels at the one size they were drawn for. press draws in
//! one bit at that size, so the pixels are all it wants: they are copied
//! out as they are, with nothing rasterised and nothing lost.
//!
//! The text families all carry the same many thousands of Chinese,
//! Japanese and Korean characters. Those are written once, to a file of
//! their own that every family falls back on, and left out of the rest.
//!
//! A `.mfont` file, all numbers little-endian:
//!
//!     "MFNT1\0"                     6 bytes
//!     size                          u8   pixels to the em
//!     ascent, descent               u8   of a line, above and below the baseline
//!     wide                          u8   how far the pen moves for a character taken
//!                                        from the shared file; 0 for its own advance
//!     shift right, shift up         i8   where such a character's picture is put,
//!                                        from where the shared file has it (a
//!                                        fixed-pitch family centres it in its cell)
//!     count                         u32  characters
//!     count x (character u32, offset u32)   sorted by character; the offset is
//!                                        from the end of this table
//!     then for each character:
//!     advance, left, top            i8   the pen's move; where the picture begins from
//!                                        the pen; how far its top row is above the baseline
//!     width, height                 u8
//!     the picture                   width x height bits, row by row, packed,
//!                                        the first pixel in the top bit

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, PartialEq)]
struct Glyph {
    advance: i8,
    left: i8,
    top: i8,
    width: u8,
    height: u8,
    bits: Vec<bool>,
}

struct Strike {
    size: u8,
    ascent: u8,
    descent: u8,
    glyphs: BTreeMap<u32, Glyph>,
}

/// The characters shared between the text families: from the CJK radicals on.
fn shared(ch: u32) -> bool {
    ch >= 0x2E80
}

fn read(path: &Path) -> Result<Strike, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let face = ttf_parser::Face::parse(&data, 0).map_err(|e| format!("{}: {e}", path.display()))?;
    let em = face.units_per_em() as i32;
    let mut chars = Vec::new();
    for table in face.tables().cmap.iter().flat_map(|cmap| cmap.subtables) {
        if table.is_unicode() {
            table.codepoints(|ch| chars.push(ch));
        }
    }
    chars.sort_unstable();
    chars.dedup();
    // whichever size the font was drawn at is the only one it has
    let ids: Vec<(u32, ttf_parser::GlyphId)> = chars.into_iter().filter_map(|ch| Some((ch, face.glyph_index(char::from_u32(ch)?)?))).collect();
    let size = ids.iter().filter(|(_, id)| face.glyph_bounding_box(*id).is_some()).find_map(|(_, id)| face.glyph_raster_image(*id, u16::MAX)).map_or(0, |image| image.pixels_per_em);
    let mut glyphs = BTreeMap::new();
    for (ch, id) in ids {
        let fits = |n: i32| i8::try_from(n).map_err(|_| format!("{}: U+{ch:04X} is too large to write", path.display()));
        let advance = fits((face.glyph_hor_advance(id).unwrap_or(0) as i32 * size as i32 + em / 2) / em)?;
        // A character with no picture is a space of some width: it still
        // moves the pen. The font file gives such a one no pixels at all,
        // which reads back as the next character's, so it is known instead
        // by its outline having nothing in it.
        let Some(image) = face.glyph_raster_image(id, u16::MAX).filter(|_| face.glyph_bounding_box(id).is_some()) else {
            glyphs.insert(ch, Glyph { advance, left: 0, top: 0, width: 0, height: 0, bits: Vec::new() });
            continue;
        };
        if image.format != ttf_parser::RasterImageFormat::BitmapMono {
            return Err(format!("{}: U+{ch:04X} is not a one-bit picture", path.display()));
        }
        let (w, h) = (image.width as usize, image.height as usize);
        let row = w.div_ceil(8);
        let bits = (0..h).flat_map(|y| (0..w).map(move |x| (y, x))).map(|(y, x)| image.data[y * row + x / 8] & (0x80 >> (x % 8)) != 0).collect();
        glyphs.insert(ch, Glyph { advance, left: fits(image.x as i32)?, top: fits(image.y as i32 + h as i32)?, width: image.width as u8, height: image.height as u8, bits });
    }
    if size == 0 {
        return Err(format!("{}: no pixels in it", path.display()));
    }
    let line = |units: i16| ((units.unsigned_abs() as i32 * size as i32 + em / 2) / em) as u8;
    Ok(Strike { size: size as u8, ascent: line(face.ascender()), descent: line(face.descender()), glyphs })
}

fn write(path: &Path, strike: &Strike, (wide, right, up): (u8, i8, i8), keep: &dyn Fn(u32, &Glyph) -> bool) -> Result<usize, String> {
    let kept: Vec<(&u32, &Glyph)> = strike.glyphs.iter().filter(|(ch, g)| keep(**ch, g)).collect();
    let mut out = b"MFNT1\0".to_vec();
    out.extend([strike.size, strike.ascent, strike.descent, wide, right as u8, up as u8]);
    out.extend((kept.len() as u32).to_le_bytes());
    let mut pictures = Vec::new();
    for (ch, g) in &kept {
        out.extend(ch.to_le_bytes());
        out.extend((pictures.len() as u32).to_le_bytes());
        pictures.extend([g.advance as u8, g.left as u8, g.top as u8, g.width, g.height]);
        for chunk in g.bits.chunks(8) {
            pictures.push(chunk.iter().enumerate().fold(0u8, |byte, (i, on)| byte | ((*on as u8) << (7 - i))));
        }
    }
    out.extend(pictures);
    std::fs::write(path, &out).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(kept.len())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [from, to] = args.as_slice() else {
        return Err("Usage: press-fontgen fonts/build OUT-FOLDER (extract bitmap strikes from built fonts)".into());
    };
    let (from, to) = (Path::new(from), Path::new(to));
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    // the family the shared characters are taken from, first
    let text = ["Cicero-Regular", "Agate-Regular", "Agate-Bold", "Brevier-Regular", "Pica-Regular", "Elite-Regular"];
    let pictures = ["Fleuron-Regular", "Vignette-Regular", "Cartouche-Regular"];
    let first = read(&from.join(format!("{}.ttf", text[0])))?;
    let common: BTreeMap<u32, Glyph> = first.glyphs.iter().filter(|(ch, _)| shared(**ch)).map(|(ch, g)| (*ch, g.clone())).collect();
    let all = Strike { size: first.size, ascent: first.ascent, descent: first.descent, glyphs: common.clone() };
    let count = write(&to.join("Han-12.mfont"), &all, (0, 0, 0), &|_, _| true)?;
    println!("Han-12: {count} characters shared by the text families");
    for name in text {
        let strike = read(&from.join(format!("{name}.ttf")))?;
        // A shared character is left out where it is the same picture. A
        // fixed-pitch family moves the pen further for one and centres it in
        // its cell: how far, and where, is found from one such character
        // and holds for nearly all of them.
        let model = '\u{4E2D}' as u32;
        let (wide, right, up) = match (common.get(&model), strike.glyphs.get(&model)) {
            (Some(c), Some(g)) if (c.advance, c.left, c.top) != (g.advance, g.left, g.top) => (g.advance as u8, g.left - c.left, g.top - c.top),
            _ => (0, 0, 0),
        };
        let fits = |ch: u32, g: &Glyph| {
            common.get(&ch).is_some_and(|c| {
                let pen = if wide == 0 { c.advance } else { wide as i8 };
                (c.width, c.height, &c.bits) == (g.width, g.height, &g.bits) && (c.left + right, c.top + up, pen) == (g.left, g.top, g.advance)
            })
        };
        let count = write(&to.join(format!("{name}.mfont")), &strike, (wide, right, up), &|ch, g| !fits(ch, g))?;
        let shared = if wide == 0 { "as drawn".to_string() } else { format!("advance {wide}, moved {right} right and {up} up") };
        println!("{name}: {count} characters of its own, {} px, line {}+{}, shared ones {shared}", strike.size, strike.ascent, strike.descent);
    }
    for name in pictures {
        let strike = read(&from.join(format!("{name}.ttf")))?;
        let count = write(&to.join(format!("{name}.mfont")), &strike, (0, 0, 0), &|_, _| true)?;
        println!("{name}: {count} characters, {} px, line {}+{}", strike.size, strike.ascent, strike.descent);
    }
    Ok(())
}
