/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Drawing for styles whose edges are smooth: blending, gradients, rounded
 * shapes and soft shadows, on a press_surface. A style includes this file;
 * it is not part of the style interface, and a style may as well draw with
 * anything else.
 */
#ifndef PRESS_STYLES_SMOOTH_H
#define PRESS_STYLES_SMOOTH_H

#include "press_style.h"

#include <math.h>
#include <stdlib.h>
#include <stddef.h>

typedef struct {
    int x, y, w, h;
} box;

static const press_surface *on;

/* Where drawing is kept to, when it is: nothing outside this box is touched. */
static box clip_box;
static int clipped;

static inline void clip_to(box r) {
    clip_box = r;
    clipped = 1;
}

static inline void clip_off(void) {
    clipped = 0;
}

static inline int allowed(int x, int y) {
    return !clipped || (x >= clip_box.x && y >= clip_box.y && x < clip_box.x + clip_box.w && y < clip_box.y + clip_box.h);
}

static inline uint32_t rgb(uint32_t hex) {
    return 0xff000000u | hex;
}

static inline int chan(uint32_t c, int shift) {
    return (int)((c >> shift) & 0xff);
}

/* A colour part of the way from one to another, t from 0 to 1. */
static inline uint32_t mix(uint32_t from, uint32_t to, float t) {
    if (t < 0) t = 0;
    if (t > 1) t = 1;
    uint32_t out = 0;
    for (int shift = 0; shift <= 24; shift += 8)
        out |= (uint32_t)(chan(from, shift) + (chan(to, shift) - chan(from, shift)) * t + 0.5f) << shift;
    return out;
}

static inline uint32_t with_alpha(uint32_t c, float a) {
    return (c & 0x00ffffffu) | ((uint32_t)(a * 255.0f + 0.5f) << 24);
}

/* How many of the surface's pixels lie along one of the interface's: 1
 * unless the style asked to be drawn fine and the screen magnifies. What
 * follows is asked for in the interface's pixels throughout; a pixel set
 * is a square of the surface's, and the smooth shapes use the finer ones. */
static inline int fineness(void) {
    return on->scale > 1 ? on->scale : 1;
}

static inline int stride(void) {
    return on->scale > 1 ? on->stride : on->width;
}

/* One of the surface's own pixels, counted in those. */
static inline uint32_t *fine_at(int x, int y) {
    int s = fineness();
    if (x < 0 || y < 0 || x >= on->width * s || y >= on->height * s || !allowed(x / s, y / s)) return NULL;
    return &on->pixels[y * stride() + x];
}

static inline uint32_t at(int x, int y) {
    int s = fineness();
    return (x >= 0 && y >= 0 && x < on->width && y < on->height) ? on->pixels[y * s * stride() + x * s] : 0;
}

static inline void px(int x, int y, uint32_t c) {
    int s = fineness();
    if (x < 0 || y < 0 || x >= on->width || y >= on->height || !allowed(x, y)) return;
    for (int j = 0; j < s; j++)
        for (int i = 0; i < s; i++)
            on->pixels[(y * s + j) * stride() + x * s + i] = c;
}

static inline uint32_t laid_over(uint32_t under, uint32_t c, float a) {
    float ua = chan(under, 24) / 255.0f;
    if (ua <= 0.002f) return with_alpha(c, a);
    float oa = a + ua * (1 - a);
    uint32_t out = (uint32_t)(oa * 255.0f + 0.5f) << 24;
    for (int shift = 0; shift <= 16; shift += 8) {
        float v = (chan(c, shift) * a + chan(under, shift) * ua * (1 - a)) / oa;
        out |= (uint32_t)(v + 0.5f) << shift;
    }
    return out;
}

/* Lay a colour over one of the surface's own pixels, as far as cover (0
 * to 1) and the colour's own alpha allow. */
static inline void blend_fine(int x, int y, uint32_t c, float cover) {
    uint32_t *p = fine_at(x, y);
    float a = cover * chan(c, 24) / 255.0f;
    if (!p || a <= 0.002f) return;
    *p = laid_over(*p, c, a > 1 ? 1 : a);
}

/* Lay a colour over a pixel, as far as cover (0 to 1) and the colour's own
 * alpha allow. A see-through pixel takes the colour that solid. */
static inline void blend(int x, int y, uint32_t c, float cover) {
    int s = fineness();
    for (int j = 0; j < s; j++)
        for (int i = 0; i < s; i++)
            blend_fine(x * s + i, y * s + j, c, cover);
}

static inline void fill(box r, uint32_t c) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, c);
}

static inline void fill_over(box r, uint32_t c) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            blend(x, y, c, 1);
}

static inline box inset(box r, int d) {
    box out = {r.x + d, r.y + d, r.w - 2 * d, r.h - 2 * d};
    if (out.w < 0) out.w = 0;
    if (out.h < 0) out.h = 0;
    return out;
}

/* One colour at the top running to another at the bottom. */
static inline void gradient(box r, uint32_t top, uint32_t bottom) {
    int s = fineness(), rows = r.h * s;
    for (int y = 0; y < rows; y++) {
        uint32_t c = mix(top, bottom, rows > 1 ? (float)y / (rows - 1) : 0);
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            uint32_t *p = fine_at(x, r.y * s + y);
            if (p) *p = c;
        }
    }
}

/* A one-pixel outline just inside the box. */
static inline void outline(box r, uint32_t c) {
    if (r.w <= 0 || r.h <= 0) return;
    fill((box){r.x, r.y, r.w, 1}, c);
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, c);
    fill((box){r.x, r.y, 1, r.h}, c);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, c);
}

/* How far a point is outside a box with rounded corners: below zero
 * inside it. */
static inline float outside_at(box r, float radius, float x, float y) {
    float half_w = r.w / 2.0f, half_h = r.h / 2.0f;
    if (radius > half_w) radius = half_w;
    if (radius > half_h) radius = half_h;
    float dx = fabsf(x - (r.x + half_w)) - (half_w - radius);
    float dy = fabsf(y - (r.y + half_h)) - (half_h - radius);
    float ox = dx > 0 ? dx : 0, oy = dy > 0 ? dy : 0;
    float in = dx > dy ? dx : dy;
    return sqrtf(ox * ox + oy * oy) + (in < 0 ? in : 0) - radius;
}

/* How far the middle of a pixel is outside such a box. */
static inline float outside(box r, float radius, int x, int y) {
    return outside_at(r, radius, x + 0.5f, y + 0.5f);
}

static inline float covered(float d) {
    float c = 0.5f - d;
    return c < 0 ? 0 : c > 1 ? 1 : c;
}

/* A box with rounded corners, smooth at the edge: one colour at its top
 * running to another at its bottom. As sharp as the surface is fine. */
static inline void round_fill(box r, float radius, uint32_t top, uint32_t bottom) {
    int s = fineness(), rows = r.h * s;
    for (int y = 0; y < rows; y++) {
        uint32_t c = mix(top, bottom, rows > 1 ? (float)y / (rows - 1) : 0);
        for (int x = 0; x < r.w * s; x++) {
            float d = outside_at(r, radius, r.x + (x + 0.5f) / s, r.y + (y + 0.5f) / s);
            float cover = covered(d * s);
            if (cover > 0) blend_fine(r.x * s + x, r.y * s + y, c, cover);
        }
    }
}

/* The outline of such a box, `thick` pixels wide, inside it. */
static inline void round_line(box r, float radius, float thick, uint32_t c) {
    int s = fineness();
    for (int y = 0; y < r.h * s; y++) {
        for (int x = 0; x < r.w * s; x++) {
            float d = outside_at(r, radius, r.x + (x + 0.5f) / s, r.y + (y + 0.5f) / s);
            float cover = covered(d * s) - covered((d + thick) * s);
            if (cover > 0) blend_fine(r.x * s + x, r.y * s + y, c, cover);
        }
    }
}

/* A rounded box, filled and outlined in one. */
static inline void round_box(box r, float radius, uint32_t top, uint32_t bottom, uint32_t edge) {
    round_fill(r, radius, top, bottom);
    round_line(r, radius, 1, edge);
}

/* A soft shadow round a rounded box: darkest at its edge and gone `spread`
 * pixels out. Nothing is drawn under the box itself. */
static inline void shadow(box r, float radius, float spread, uint32_t c, float strength) {
    int s = fineness(), reach = (int)spread + 1;
    for (int y = (r.y - reach) * s; y < (r.y + r.h + reach) * s; y++) {
        for (int x = (r.x - reach) * s; x < (r.x + r.w + reach) * s; x++) {
            float d = outside_at(r, radius, (x + 0.5f) / s, (y + 0.5f) / s);
            if (d * s <= -0.5f || d >= spread) continue;
            float t = 1 - (d < 0 ? 0 : d) / spread;
            blend_fine(x, y, c, strength * t * t * (1 - covered(d * s)));
        }
    }
}

/* How much of the pixel from a to a + 1 lies between two edges. */
static inline float between(int a, float from, float to) {
    float lo = from > a ? from : a, hi = to < a + 1 ? to : a + 1;
    return hi > lo ? hi - lo : 0;
}

/* A shape between two rows whose left and right edges each run straight
 * from where they are at the top (l0, r0) to where they are at the bottom
 * (l1, r1): a slanted stroke, or an arrowhead where the edges meet. Smooth
 * along its edges and as sharp as the surface is fine. For a surface that
 * magnifies: the marks below draw their own squares where it does not. */
static inline void slanted(float y0, float y1, float l0, float r0, float l1, float r1, uint32_t c) {
    int s = fineness();
    for (int y = (int)floorf(y0 * s); y < (int)ceilf(y1 * s); y++) {
        float t = ((y + 0.5f) / s - y0) / (y1 - y0);
        t = t < 0 ? 0 : t > 1 ? 1 : t;
        float l = (l0 + (l1 - l0) * t) * s, r = (r0 + (r1 - r0) * t) * s, rows = between(y, y0 * s, y1 * s);
        for (int x = (int)floorf(l); x < (int)ceilf(r); x++)
            blend_fine(x, y, c, rows * between(x, l, r));
    }
}

/* The same on its side: between two columns, its top and bottom edges
 * running from (t0, b0) at the left to (t1, b1) at the right. */
static inline void slanted_across(float x0, float x1, float t0, float b0, float t1, float b1, uint32_t c) {
    int s = fineness();
    for (int x = (int)floorf(x0 * s); x < (int)ceilf(x1 * s); x++) {
        float f = ((x + 0.5f) / s - x0) / (x1 - x0);
        f = f < 0 ? 0 : f > 1 ? 1 : f;
        float t = (t0 + (t1 - t0) * f) * s, b = (b0 + (b1 - b0) * f) * s, columns = between(x, x0 * s, x1 * s);
        for (int y = (int)floorf(t); y < (int)ceilf(b); y++)
            blend_fine(x, y, c, columns * between(y, t, b));
    }
}

/* A solid arrowhead with its middle at (cx, cy), `size` pixels from base
 * to tip: 0 up, 1 down, 2 left, 3 right. */
static inline void arrow(int cx, int cy, int size, int way, uint32_t c) {
    if (fineness() > 1) {
        float mx = cx + 0.5f, my = cy + 0.5f;
        int top = cy - size / 2, bottom = cy + (size - 1) / 2 + 1, left = cx - size / 2, right = cx + (size - 1) / 2 + 1;
        switch (way) {
        case 0: slanted(top, top + size, mx, mx, mx - size, mx + size, c); break;
        case 1: slanted(bottom - size, bottom, mx - size, mx + size, mx, mx, c); break;
        case 2: slanted_across(left, left + size, my, my, my - size, my + size, c); break;
        default: slanted_across(right - size, right, my - size, my + size, my, my, c); break;
        }
        return;
    }
    for (int d = 0; d < size; d++) {
        switch (way) {
        case 0: fill((box){cx - d, cy - size / 2 + d, 2 * d + 1, 1}, c); break;
        case 1: fill((box){cx - d, cy + (size - 1) / 2 - d, 2 * d + 1, 1}, c); break;
        case 2: fill((box){cx - size / 2 + d, cy - d, 1, 2 * d + 1}, c); break;
        default: fill((box){cx + (size - 1) / 2 - d, cy - d, 1, 2 * d + 1}, c); break;
        }
    }
}

/* A tick `across` pixels wide with its top left at (x, y): it runs down
 * for `dip` pixels, then up to the right, `heavy` pixels thick. */
static inline void tick(int x, int y, int across, int dip, int heavy, uint32_t c) {
    int low = y + across - 1 - dip;
    if (fineness() > 1) {
        float turn = x + dip + 0.5f;
        slanted_across(x, turn, low - dip - 0.5f, low - dip - 0.5f + heavy, low, low + heavy, c);
        slanted_across(turn, x + across, low, low + heavy, y - 0.5f, y - 0.5f + heavy, c);
        return;
    }
    for (int i = 0; i < across; i++)
        fill((box){x + i, low - abs(i - dip), 1, heavy}, c);
}

/* A cross with its middle at (cx, cy), reaching `reach` pixels each way:
 * each stroke `wide` pixels across, starting `back` pixels left of the
 * diagonal. */
static inline void cross(int cx, int cy, int reach, int back, int wide, uint32_t c) {
    int s = fineness();
    if (s == 1) {
        for (int i = -reach; i <= reach; i++) {
            for (int w = 0; w < wide; w++) {
                blend(cx + i - back + w, cy + i, c, 1);
                blend(cx - i - back + w, cy + i, c, 1);
            }
        }
        return;
    }
    /* both strokes at once, so ink that is see-through is no darker where they meet */
    for (int y = (cy - reach) * s; y < (cy + reach + 1) * s; y++) {
        float off = (y + 0.5f) / s - cy - 0.5f;
        float a = (cx - back + off) * s, b = (cx - back - off) * s;
        for (int x = (cx - back - reach - 1) * s; x < (cx - back + reach + wide + 1) * s; x++) {
            float one = between(x, a, a + wide * s), other = between(x, b, b + wide * s);
            blend_fine(x, y, c, one > other ? one : other);
        }
    }
}

static inline int line_height(void) {
    return on->line_height(on->ctx);
}

static inline int text_width(const char *text) {
    return on->text_width(on->ctx, text);
}

static inline void words(const char *text, int x, int y, uint32_t c) {
    on->draw_text(on->ctx, text, x, y, c);
}

static inline int has(const press_part *p, uint32_t flag) {
    return (p->state & flag) != 0;
}

/* How light a colour is, 0 to 1. */
static inline float light(uint32_t c) {
    return (chan(c, 16) * 0.299f + chan(c, 8) * 0.587f + chan(c, 0) * 0.114f) / 255.0f;
}

/* Show what is drawn in a box as selected: what was light becomes one
 * colour and what was dark another, and the smooth edges of letters stay
 * smooth between the two. */
static inline void recolour(box r, uint32_t for_light, uint32_t for_dark) {
    int s = fineness();
    for (int y = r.y * s; y < (r.y + r.h) * s; y++)
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            uint32_t *p = fine_at(x, y);
            if (p) *p = mix(for_dark, for_light, light(*p));
        }
}

/* Show what is drawn in a box as out of use: faded towards a colour. */
static inline void fade(box r, uint32_t towards, float much) {
    int s = fineness();
    for (int y = r.y * s; y < (r.y + r.h) * s; y++)
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            uint32_t *p = fine_at(x, y);
            if (p) *p = mix(*p, towards, much);
        }
}

#endif
