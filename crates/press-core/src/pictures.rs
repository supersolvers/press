// SPDX-License-Identifier: GPL-3.0-or-later
//! The large pictures that stand for things: on the desktop, and in a list
//! shown as icons. Each is 32 pixels square, black lines round a white
//! inside and clear outside, so that whatever is behind shows round it.

use crate::canvas::{Canvas, Rect, BLACK, TRANSPARENT, WHITE};
use crate::font::Font;

/// The side of a picture.
pub const SIDE: i32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picture {
    Folder,
    Document,
    Application,
    /// A disk: what holds everything else.
    Disk,
    Trash,
    /// A drive plugged in: a box with a light on its front.
    Drive,
    /// A drive across the network: the box, wired to the network below.
    Server,
}

pub fn picture(kind: Picture) -> Canvas {
    let mut c = Canvas::new(SIDE, SIDE);
    c.pixels.fill(TRANSPARENT);
    let boxed = |c: &mut Canvas, r: Rect| {
        c.fill(r, WHITE);
        c.frame(r, BLACK);
    };
    match kind {
        Picture::Folder => {
            boxed(&mut c, Rect::new(2, 5, 12, 6));
            boxed(&mut c, Rect::new(1, 9, 30, 19));
            // the tab and the body are one shape
            c.fill(Rect::new(3, 9, 10, 1), WHITE);
        }
        Picture::Document => {
            boxed(&mut c, Rect::new(6, 1, 20, 30));
            // the corner turned down
            for i in 0..6 {
                c.fill(Rect::new(20 + i, 1, 6 - i, 1 + i), TRANSPARENT);
                c.set(20 + i, 1 + i, BLACK);
            }
            c.fill(Rect::new(20, 1, 1, 6), BLACK);
            c.fill(Rect::new(20, 6, 6, 1), BLACK);
        }
        Picture::Application => {
            // a diamond, with a sheet of work in the middle of it
            for d in 0..16 {
                c.fill(Rect::new(15 - d, d, 2 * d + 2, 1), WHITE);
                c.fill(Rect::new(15 - d, 31 - d, 2 * d + 2, 1), WHITE);
                for y in [d, 31 - d] {
                    c.set(15 - d, y, BLACK);
                    c.set(16 + d, y, BLACK);
                }
            }
            c.frame(Rect::new(11, 10, 10, 12), BLACK);
            for y in [13, 16, 19] {
                c.fill(Rect::new(13, y, 6, 1), BLACK);
            }
        }
        Picture::Disk => {
            // a shutter at the top, a label below
            boxed(&mut c, Rect::new(3, 2, 26, 28));
            c.fill(Rect::new(27, 2, 2, 2), TRANSPARENT);
            c.set(27, 3, BLACK);
            c.set(28, 4, BLACK);
            c.frame(Rect::new(9, 2, 13, 10), BLACK);
            c.fill(Rect::new(17, 4, 3, 6), BLACK);
            c.frame(Rect::new(7, 16, 18, 14), BLACK);
            for y in [20, 23, 26] {
                c.fill(Rect::new(10, y, 12, 1), BLACK);
            }
        }
        Picture::Drive | Picture::Server => {
            let top = if kind == Picture::Drive { 9 } else { 3 };
            boxed(&mut c, Rect::new(2, top, 28, 15));
            // the front: a lip along the bottom, and the light
            c.fill(Rect::new(3, top + 10, 26, 1), BLACK);
            c.fill(Rect::new(23, top + 6, 3, 2), BLACK);
            for y in [top + 4, top + 6] {
                c.fill(Rect::new(6, y, 10, 1), BLACK);
            }
            if kind == Picture::Drive {
                // its feet
                c.fill(Rect::new(5, top + 15, 3, 2), BLACK);
                c.fill(Rect::new(24, top + 15, 3, 2), BLACK);
            } else {
                // a cable down to the network, and two more machines on it
                c.fill(Rect::new(15, top + 15, 2, 9), BLACK);
                c.fill(Rect::new(1, top + 24, 30, 2), BLACK);
                for x in [3, 23] {
                    boxed(&mut c, Rect::new(x, top + 19, 6, 5));
                }
            }
        }
        Picture::Trash => {
            boxed(&mut c, Rect::new(13, 2, 6, 4));
            boxed(&mut c, Rect::new(7, 5, 18, 4));
            boxed(&mut c, Rect::new(9, 8, 14, 23));
            for x in [12, 15, 18] {
                c.fill(Rect::new(x, 11, 1, 17), BLACK);
            }
        }
    }
    c
}

/// A name cut short, with an ellipsis, to fit a width.
pub fn fitted(font: &mut Font, name: &str, width: i32) -> String {
    if font.width(name) <= width {
        return name.to_string();
    }
    let mut shown = String::new();
    for ch in name.chars() {
        if font.width(&shown) + font.advance(ch) + font.width("\u{2026}") > width {
            break;
        }
        shown.push(ch);
    }
    shown + "\u{2026}"
}

/// How tall a picture with its name under it is.
pub fn labelled_height(font: &Font) -> i32 {
    SIDE + 3 + font.line_height() + 2
}

/// A picture with its name underneath on a white strip, `width` wide and
/// clear elsewhere. Selected, both are turned over.
pub fn labelled(font: &mut Font, name: &str, art: &Canvas, width: i32, selected: bool) -> Canvas {
    let line = font.line_height();
    let mut c = Canvas::new(width, labelled_height(font));
    c.pixels.fill(TRANSPARENT);
    let left = (width - art.width) / 2;
    c.blit(art, left, (SIDE - art.height).max(0) / 2);
    let shown = fitted(font, name, width - 8);
    let w = font.width(&shown);
    let strip = Rect::new((width - w) / 2 - 3, SIDE + 2, w + 6, line + 2);
    c.fill(strip, WHITE);
    font.draw(&mut c, &shown, strip.x + 3, strip.y + 1, BLACK);
    if selected {
        c.invert(strip);
        // only the picture itself turns over, not what is round it
        let top = (SIDE - art.height).max(0) / 2;
        for y in 0..art.height {
            for x in 0..art.width {
                match art.get(x, y) {
                    BLACK => c.set(left + x, top + y, WHITE),
                    WHITE => c.set(left + x, top + y, BLACK),
                    _ => {}
                }
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_are_one_bit_and_clear_at_the_corners() {
        let mut font = Font::new();
        for kind in [Picture::Folder, Picture::Document, Picture::Application, Picture::Disk, Picture::Trash] {
            let art = picture(kind);
            assert!(art.pixels.iter().all(|p| [BLACK, WHITE, TRANSPARENT].contains(p)));
            assert_eq!(art.get(0, 0), TRANSPARENT, "{kind:?}");
            assert!(art.pixels.iter().filter(|p| **p == WHITE).count() > 100, "{kind:?}");
            // selecting turns the picture over and leaves the outside clear
            let (plain, chosen) = (labelled(&mut font, "Letters", &art, 84, false), labelled(&mut font, "Letters", &art, 84, true));
            assert_eq!((plain.width, plain.height, chosen.get(0, 0)), (84, labelled_height(&font), TRANSPARENT));
            let whites = |c: &Canvas| (0..SIDE).flat_map(|y| (0..84).map(move |x| (x, y))).filter(|(x, y)| c.get(*x, *y) == WHITE).count();
            assert!(whites(&chosen) < whites(&plain), "{kind:?}");
        }
        // a long name is cut to fit
        let long = fitted(&mut font, "A very long name for a folder indeed", 76);
        assert!(long.ends_with('\u{2026}') && font.width(&long) <= 76);
    }
}

