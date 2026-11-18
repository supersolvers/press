// SPDX-License-Identifier: GPL-3.0-or-later
//! Pointer shapes, drawn here in one bit like everything else: black with a
//! white edge so they show on any background.

use crate::canvas::{Canvas, BLACK, TRANSPARENT, WHITE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cursor {
    /// The ordinary pointer.
    #[default]
    Arrow,
    /// The I-beam shown over editable text.
    Text,
    /// Arrows pointing apart, over an edge that can be dragged sideways.
    ResizeColumn,
    /// Arrows pointing up and down, over an edge that can be dragged that way.
    ResizeRow,
    /// Arrows pointing to the top left and the bottom right, over a corner
    /// that can be dragged to make something larger or smaller.
    ResizeCorner,
    /// An hourglass: something is being done, and will be a moment.
    Busy,
}

impl Cursor {
    /// Every shape there is.
    pub const ALL: [Cursor; 6] = [Cursor::Arrow, Cursor::Text, Cursor::ResizeColumn, Cursor::ResizeRow, Cursor::ResizeCorner, Cursor::Busy];
}

/// A pointer picture and the pixel in it that does the pointing.
pub struct CursorImage {
    pub canvas: Canvas,
    pub hot: (i32, i32),
}

// The black part of each shape. The white edge is added around it.
const ARROW: &[&str] = &[
    "X",
    "XX",
    "XXX",
    "XXXX",
    "XXXXX",
    "XXXXXX",
    "XXXXXXX",
    "XXXXXXXX",
    "XXXXXXXXX",
    "XXXXXXXXXX",
    "XXXXXXXXXXX",
    "XXXXXXX",
    "XXX XXXX",
    "XX  XXXX",
    "X    XXXX",
    "     XXXX",
    "      XX",
];

const IBEAM: &[&str] = &[
    "XXX XXX",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "   X",
    "XXX XXX",
];

const SIDEWAYS: &[&str] = &[
    "       X       ",
    "   X   X   X   ",
    "  XX   X   XX  ",
    " XXXXXXXXXXXXX ",
    "XXXXXXXXXXXXXXX",
    " XXXXXXXXXXXXX ",
    "  XX   X   XX  ",
    "   X   X   X   ",
    "       X       ",
];

const UPRIGHT: &[&str] = &[
    "    X    ",
    "   XXX   ",
    "  XXXXX  ",
    " XXXXXXX ",
    "   XXX   ",
    "   XXX   ",
    "   XXX   ",
    "XXXXXXXXX",
    "   XXX   ",
    "   XXX   ",
    "   XXX   ",
    " XXXXXXX ",
    "  XXXXX  ",
    "   XXX   ",
    "    X    ",
];

const CORNER: &[&str] = &[
    "XXXXXXX        ",
    "XXXXXX         ",
    "XXXXX          ",
    "XXXXXX         ",
    "XXX XXX        ",
    "XX   XXX       ",
    "X     XXX      ",
    "       XXX     ",
    "      XXX     X",
    "       XXX   XX",
    "        XXX XXX",
    "         XXXXXX",
    "          XXXXX",
    "         XXXXXX",
    "        XXXXXXX",
];

// Sand run half through: a full glass would look as if nothing had begun,
// and an empty one as if it were over.
const HOURGLASS: &[&str] = &[
    "XXXXXXXXXXX",
    "XXXXXXXXXXX",
    " X       X ",
    " X       X ",
    " XX X X XX ",
    "  XX X XX  ",
    "   XX XX   ",
    "    X X    ",
    "   X X X   ",
    "  X  X  X  ",
    " X   X   X ",
    " X  XXX  X ",
    " X XXXXX X ",
    "XXXXXXXXXXX",
    "XXXXXXXXXXX",
];

impl Cursor {
    pub fn image(self) -> CursorImage {
        // hot spots are given in the shape's own coordinates
        let (shape, hot) = match self {
            Cursor::Arrow => (ARROW, (0, 0)),
            Cursor::Text => (IBEAM, (3, 8)),
            Cursor::ResizeColumn => (SIDEWAYS, (7, 4)),
            Cursor::ResizeRow => (UPRIGHT, (4, 7)),
            Cursor::ResizeCorner => (CORNER, (7, 7)),
            Cursor::Busy => (HOURGLASS, (5, 7)),
        };
        let width = shape.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
        // one pixel of margin all round for the white edge
        let mut canvas = Canvas::new(width + 2, shape.len() as i32 + 2);
        canvas.pixels.fill(TRANSPARENT);
        let black = |x: i32, y: i32| {
            x >= 0 && y >= 0 && shape.get(y as usize).and_then(|row| row.as_bytes().get(x as usize)).is_some_and(|b| *b == b'X')
        };
        for y in -1..=shape.len() as i32 {
            for x in -1..=width {
                if black(x, y) {
                    canvas.set(x + 1, y + 1, BLACK);
                } else if (-1..=1).any(|dy| (-1..=1).any(|dx| black(x + dx, y + dy))) {
                    canvas.set(x + 1, y + 1, WHITE);
                }
            }
        }
        CursorImage { canvas, hot: (hot.0 + 1, hot.1 + 1) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_edged_in_white_and_point_at_black() {
        for cursor in Cursor::ALL {
            let image = cursor.image();
            let c = &image.canvas;
            assert!(c.pixels.iter().all(|p| [BLACK, WHITE, TRANSPARENT].contains(p)));
            // no black pixel touches the transparent surround or the border
            for y in 0..c.height {
                for x in 0..c.width {
                    if c.pixels[(y * c.width + x) as usize] != BLACK {
                        continue;
                    }
                    assert!(x > 0 && y > 0 && x < c.width - 1 && y < c.height - 1);
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        assert_ne!(c.pixels[((y + dy) * c.width + x + dx) as usize], TRANSPARENT);
                    }
                }
            }
            assert!(image.hot.0 < c.width && image.hot.1 < c.height);
            // nothing see-through is shut inside a shape: the glass of the hourglass is white
            let mut outside = vec![false; c.pixels.len()];
            let mut next: Vec<(i32, i32)> = (0..c.width).flat_map(|x| [(x, 0), (x, c.height - 1)]).chain((0..c.height).flat_map(|y| [(0, y), (c.width - 1, y)])).collect();
            while let Some((x, y)) = next.pop() {
                let i = (y * c.width + x) as usize;
                if x < 0 || y < 0 || x >= c.width || y >= c.height || outside[i] || c.pixels[i] != TRANSPARENT {
                    continue;
                }
                outside[i] = true;
                next.extend([(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
            }
            assert!(c.pixels.iter().zip(&outside).all(|(p, out)| *p != TRANSPARENT || *out), "{cursor:?} has a hole");
        }
        assert_eq!(Cursor::Arrow.image().canvas.pixels[{ let i = Cursor::Arrow.image(); (i.hot.1 * i.canvas.width + i.hot.0) as usize }], BLACK);
    }
}
