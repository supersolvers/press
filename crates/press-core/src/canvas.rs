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

/// A picture to draw on. Places and sizes are in pixels of the interface;
/// a canvas may be finer than that, keeping `scale` pixels of the screen
/// along each side of one of the interface's, for smooth letters and
/// edges on a screen that magnifies. Everything here is asked for in the
/// interface's pixels all the same: a pixel set is a square of the
/// screen's, and only what is drawn smooth uses the finer ones.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: i32,
    pub height: i32,
    /// Rows of `width * scale` pixels, `height * scale` of them.
    pub pixels: Vec<u32>,
    /// How many of the screen's pixels lie along one of the interface's: 1
    /// unless the canvas was made finer.
    pub scale: i32,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Canvas {
        let (width, height) = (width.max(1), height.max(1));
        Canvas { width, height, pixels: vec![WHITE; (width * height) as usize], scale: 1 }
    }

    /// A canvas of one colour all over.
    pub fn filled(width: i32, height: i32, color: u32) -> Canvas {
        Canvas::fine(width, height, 1, color)
    }

    /// A canvas of one colour all over, `scale` times as fine as the
    /// interface's pixels.
    pub fn fine(width: i32, height: i32, scale: i32, color: u32) -> Canvas {
        let (width, height, scale) = (width.max(1), height.max(1), scale.clamp(1, 8));
        Canvas { width, height, pixels: vec![color; (width * scale * height * scale) as usize], scale }
    }

    /// A canvas of one colour as fine as another.
    pub fn like(other: &Canvas, width: i32, height: i32, color: u32) -> Canvas {
        Canvas::fine(width, height, other.scale, color)
    }

    /// Copy a box of another canvas as fine as this one to a place on this
    /// one, pixel for pixel of those they keep, see-through ones and all.
    pub fn copy(&mut self, src: &Canvas, from: Rect, to: (i32, i32)) {
        let s = self.scale;
        if src.scale != s {
            for y in 0..from.h {
                for x in 0..from.w {
                    self.set(to.0 + x, to.1 + y, src.get(from.x + x, from.y + y));
                }
            }
            return;
        }
        let ((wide, tall), (from_wide, from_tall)) = (self.span(), src.span());
        for y in 0..from.h * s {
            for x in 0..from.w * s {
                let (sx, sy, dx, dy) = (from.x * s + x, from.y * s + y, to.0 * s + x, to.1 * s + y);
                if sx >= 0 && sy >= 0 && sx < from_wide && sy < from_tall && dx >= 0 && dy >= 0 && dx < wide && dy < tall {
                    self.pixels[(dy * wide + dx) as usize] = src.pixels[(sy * from_wide + sx) as usize];
                }
            }
        }
    }

    /// How many pixels the canvas keeps across and down.
    pub fn span(&self) -> (i32, i32) {
        (self.width * self.scale, self.height * self.scale)
    }

    /// Every pixel the canvas keeps that lies in a box of the interface's.
    #[inline]
    fn each(&mut self, r: Rect, mut f: impl FnMut(&mut u32)) {
        let (s, wide) = (self.scale, self.width * self.scale);
        for y in r.y.max(0) * s..(r.y + r.h).min(self.height) * s {
            for x in r.x.max(0) * s..(r.x + r.w).min(self.width) * s {
                f(&mut self.pixels[(y * wide + x) as usize]);
            }
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            if self.scale == 1 {
                self.pixels[(y * self.width + x) as usize] = color;
            } else {
                self.each(Rect::new(x, y, 1, 1), |p| *p = color);
            }
        }
    }

    /// One colour laid over another, as far as `a` out of 255.
    #[inline]
    fn over(under: u32, color: u32, a: u32) -> u32 {
        let under_a = under >> 24;
        if under_a == 0 {
            return (a << 24) | (color & 0x00ff_ffff);
        }
        let mix = |shift: u32| {
            let (top, bottom) = ((color >> shift) & 0xff, (under >> shift) & 0xff);
            ((top * a + bottom * (255 - a) + 127) / 255) << shift
        };
        let out_a = (a + under_a * (255 - a) / 255).min(255);
        (out_a << 24) | mix(16) | mix(8) | mix(0)
    }

    /// Lay a colour over a pixel, as far as `cover` out of 255: how smooth
    /// edges and letters are drawn. A see-through pixel takes the colour
    /// with that much of it showing.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, color: u32, cover: u8) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let a = cover as u32 * (color >> 24) / 255;
        if a == 0 {
            return;
        }
        if self.scale == 1 {
            let at = &mut self.pixels[(y * self.width + x) as usize];
            *at = Self::over(*at, color, a);
        } else {
            self.each(Rect::new(x, y, 1, 1), |p| *p = Self::over(*p, color, a));
        }
    }

    /// Lay a colour over one of the pixels the canvas keeps, counted in
    /// those: for what is drawn finer than the interface's pixels.
    #[inline]
    pub fn blend_fine(&mut self, x: i32, y: i32, color: u32, cover: u8) {
        let (wide, tall) = self.span();
        let a = cover as u32 * (color >> 24) / 255;
        if x < 0 || y < 0 || x >= wide || y >= tall || a == 0 {
            return;
        }
        let at = &mut self.pixels[(y * wide + x) as usize];
        *at = Self::over(*at, color, a);
    }

    pub fn get(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.pixels[(y * self.scale * self.width * self.scale + x * self.scale) as usize]
        } else {
            WHITE
        }
    }

    pub fn fill(&mut self, r: Rect, color: u32) {
        self.each(r, |p| *p = color);
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
        self.each(r, |p| *p = if *p == BLACK { WHITE } else { BLACK });
    }

    /// Knock out every other pixel in a checkerboard: the classic way to grey
    /// something out on a display that has no grey.
    pub fn dim(&mut self, r: Rect) {
        for y in r.y.max(0)..(r.y + r.h).min(self.height) {
            for x in r.x.max(0)..(r.x + r.w).min(self.width) {
                if (x + y) % 2 == 0 {
                    self.set(x, y, WHITE);
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
        if src.scale == self.scale && self.scale > 1 {
            // as fine as each other: pixel for pixel of those they keep
            let (s, (wide, tall), (from_wide, from_tall)) = (self.scale, self.span(), src.span());
            for sy in 0..from_tall {
                for sx in 0..from_wide {
                    let pixel = src.pixels[(sy * from_wide + sx) as usize];
                    let (dx, dy) = (x * s + sx, y * s + sy);
                    if pixel >> 24 == 0 || dx < 0 || dy < 0 || dx >= wide || dy >= tall {
                        continue;
                    }
                    let at = &mut self.pixels[(dy * wide + dx) as usize];
                    *at = if pixel >> 24 == 255 { pixel } else { Self::over(*at, pixel, pixel >> 24) };
                }
            }
            return;
        }
        for sy in 0..src.height {
            for sx in 0..src.width {
                let pixel = src.get(sx, sy);
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
        if src.scale != self.scale {
            for y in part.y.max(0)..(part.y + part.h).min(self.height).min(src.height) {
                for x in part.x.max(0)..(part.x + part.w).min(self.width).min(src.width) {
                    self.set(x, y, src.get(x, y));
                }
            }
            return;
        }
        let (s, wide, from_wide) = (self.scale, self.width * self.scale, src.width * src.scale);
        for y in part.y.max(0) * s..(part.y + part.h).min(self.height).min(src.height) * s {
            for x in part.x.max(0) * s..(part.x + part.w).min(self.width).min(src.width) * s {
                self.pixels[(y * wide + x) as usize] = src.pixels[(y * from_wide + x) as usize];
            }
        }
    }

    /// Magnify by an integer factor with no smoothing: every pixel the
    /// canvas keeps becomes a square of them, on a canvas no finer than
    /// the interface.
    pub fn scaled(&self, scale: i32) -> Canvas {
        let (scale, (wide, tall)) = (scale.max(1), self.span());
        let mut out = Canvas::new(wide * scale, tall * scale);
        for y in 0..out.height {
            for x in 0..out.width {
                out.pixels[(y * out.width + x) as usize] = self.pixels[((y / scale) * wide + x / scale) as usize];
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
