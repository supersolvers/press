// SPDX-License-Identifier: GPL-3.0-or-later
//! A one-bit drawing surface. Pixels are stored as ARGB so a frame can be
//! handed straight to a display, but only black and white are ever written.

pub const BLACK: u32 = 0xff00_0000;
pub const WHITE: u32 = 0xffff_ffff;
/// Used only around the outline of a pop-up, where the desktop shows through.
pub const TRANSPARENT: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2 * d).max(0), (self.h - 2 * d).max(0))
    }
}

pub struct Canvas {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u32>,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Canvas {
        let (width, height) = (width.max(1), height.max(1));
        Canvas { width, height, pixels: vec![WHITE; (width * height) as usize] }
    }

    /// A canvas of one colour all over.
    pub fn filled(width: i32, height: i32, color: u32) -> Canvas {
        let mut canvas = Canvas::new(width, height);
        canvas.pixels.fill(color);
        canvas
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Lay a colour over a pixel, as far as `cover` out of 255: how smooth
    /// edges and letters are drawn. A see-through pixel takes the colour
    /// with that much of it showing.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, color: u32, cover: u8) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let at = &mut self.pixels[(y * self.width + x) as usize];
        let a = cover as u32 * (color >> 24) / 255;
        if a == 0 {
            return;
        }
        let under = *at;
        let under_a = under >> 24;
        if under_a == 0 {
            *at = (a << 24) | (color & 0x00ff_ffff);
            return;
        }
        let mix = |shift: u32| {
            let (top, bottom) = ((color >> shift) & 0xff, (under >> shift) & 0xff);
            ((top * a + bottom * (255 - a) + 127) / 255) << shift
        };
        let out_a = (a + under_a * (255 - a) / 255).min(255);
        *at = (out_a << 24) | mix(16) | mix(8) | mix(0);
    }

    pub fn get(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize]
        } else {
            WHITE
        }
    }

    pub fn fill(&mut self, r: Rect, color: u32) {
        for y in r.y.max(0)..(r.y + r.h).min(self.height) {
            for x in r.x.max(0)..(r.x + r.w).min(self.width) {
                self.pixels[(y * self.width + x) as usize] = color;
            }
        }
    }

    /// One-pixel outline just inside `r`.
    pub fn frame(&mut self, r: Rect, color: u32) {
        if r.w <= 0 || r.h <= 0 {
            return;
        }
        self.fill(Rect::new(r.x, r.y, r.w, 1), color);
        self.fill(Rect::new(r.x, r.y + r.h - 1, r.w, 1), color);
        self.fill(Rect::new(r.x, r.y, 1, r.h), color);
        self.fill(Rect::new(r.x + r.w - 1, r.y, 1, r.h), color);
    }

    /// Swap black and white inside `r`.
    pub fn invert(&mut self, r: Rect) {
        for y in r.y.max(0)..(r.y + r.h).min(self.height) {
            for x in r.x.max(0)..(r.x + r.w).min(self.width) {
                let p = &mut self.pixels[(y * self.width + x) as usize];
                *p = if *p == BLACK { WHITE } else { BLACK };
            }
        }
    }

    /// Knock out every other pixel in a checkerboard: the classic way to grey
    /// something out on a display that has no grey.
    pub fn dim(&mut self, r: Rect) {
        for y in r.y.max(0)..(r.y + r.h).min(self.height) {
            for x in r.x.max(0)..(r.x + r.w).min(self.width) {
                if (x + y) % 2 == 0 {
                    self.pixels[(y * self.width + x) as usize] = WHITE;
                }
            }
        }
    }

    /// Filled rounded rectangle with stepped pixel corners. `radius` is the
    /// corner size in pixels (4 gives the familiar push-button shape).
    pub fn round_fill(&mut self, r: Rect, radius: i32, color: u32) {
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        for row in 0..r.h {
            let from_edge = row.min(r.h - 1 - row);
            let indent = corner_indent(radius, from_edge);
            self.fill(Rect::new(r.x + indent, r.y + row, r.w - 2 * indent, 1), color);
        }
    }

    /// Rounded outline `thickness` pixels wide, drawn inside `r`.
    pub fn round_frame(&mut self, r: Rect, radius: i32, thickness: i32, color: u32, inside: u32) {
        self.round_fill(r, radius, color);
        self.round_fill(r.inset(thickness), (radius - thickness).max(0), inside);
    }

    /// A dashed outline just inside `r`, with the corner pixels left out. The
    /// dashes are two pixels on, two off, and shift clockwise as `phase` grows:
    /// step the phase to make them march.
    pub fn ants(&mut self, r: Rect, phase: i32) {
        if r.w < 3 || r.h < 3 {
            return;
        }
        let (right, bottom) = (r.x + r.w - 1, r.y + r.h - 1);
        let top = (r.x + 1..right).map(|x| (x, r.y));
        let down = (r.y + 1..bottom).map(|y| (right, y));
        let back = (r.x + 1..right).rev().map(|x| (x, bottom));
        let up = (r.y + 1..bottom).rev().map(|y| (r.x, y));
        for (i, (x, y)) in top.chain(down).chain(back).chain(up).enumerate() {
            if (i as i32 - phase).rem_euclid(4) < 2 {
                self.set(x, y, BLACK);
            }
        }
    }

    /// Copy another canvas onto this one with its top-left corner at (x, y).
    /// Its transparent pixels leave what is underneath.
    pub fn blit(&mut self, src: &Canvas, x: i32, y: i32) {
        for sy in 0..src.height {
            for sx in 0..src.width {
                let pixel = src.pixels[(sy * src.width + sx) as usize];
                match pixel >> 24 {
                    0 => {}
                    255 => self.set(x + sx, y + sy, pixel),
                    // partly see-through, as a soft shadow is
                    _ => self.blend(x + sx, y + sy, pixel, 255),
                }
            }
        }
    }

    /// Copy the part of another canvas of the same size that falls inside
    /// `part`, in place. Drawing through an opening is done this way.
    pub fn blit_part(&mut self, src: &Canvas, part: Rect) {
        for y in part.y.max(0)..(part.y + part.h).min(self.height).min(src.height) {
            for x in part.x.max(0)..(part.x + part.w).min(self.width).min(src.width) {
                self.pixels[(y * self.width + x) as usize] = src.pixels[(y * src.width + x) as usize];
            }
        }
    }

    /// Magnify by an integer factor with no smoothing.
    pub fn scaled(&self, scale: i32) -> Canvas {
        let scale = scale.max(1);
        let mut out = Canvas::new(self.width * scale, self.height * scale);
        for y in 0..out.height {
            for x in 0..out.width {
                out.pixels[(y * out.width + x) as usize] = self.pixels[((y / scale) * self.width + x / scale) as usize];
            }
        }
        out
    }
}

/// How far a row `from_edge` pixels from the top or bottom is pulled in to
/// make a quarter circle of the given radius.
fn corner_indent(radius: i32, from_edge: i32) -> i32 {
    if from_edge >= radius {
        return 0;
    }
    let r = radius as f32;
    let dy = r - from_edge as f32 - 0.5;
    let dx = (r * r - dy * dy).max(0.0).sqrt();
    (r - dx).round() as i32
}
