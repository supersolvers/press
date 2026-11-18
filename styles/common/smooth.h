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
#include <stddef.h>

typedef struct {
    int x, y, w, h;
} box;

static const press_surface *on;

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

static inline uint32_t at(int x, int y) {
    return (x >= 0 && y >= 0 && x < on->width && y < on->height) ? on->pixels[y * on->width + x] : 0;
}

static inline void px(int x, int y, uint32_t c) {
    if (x >= 0 && y >= 0 && x < on->width && y < on->height)
        on->pixels[y * on->width + x] = c;
}

/* Lay a colour over a pixel, as far as cover (0 to 1) and the colour's own
 * alpha allow. A see-through pixel takes the colour that solid. */
static inline void blend(int x, int y, uint32_t c, float cover) {
    if (x < 0 || y < 0 || x >= on->width || y >= on->height) return;
    float a = cover * chan(c, 24) / 255.0f;
    if (a <= 0.002f) return;
    if (a > 1) a = 1;
    uint32_t under = on->pixels[y * on->width + x];
    float ua = chan(under, 24) / 255.0f;
    if (ua <= 0.002f) {
        on->pixels[y * on->width + x] = with_alpha(c, a);
        return;
    }
    float oa = a + ua * (1 - a);
    uint32_t out = (uint32_t)(oa * 255.0f + 0.5f) << 24;
    for (int shift = 0; shift <= 16; shift += 8) {
        float v = (chan(c, shift) * a + chan(under, shift) * ua * (1 - a)) / oa;
        out |= (uint32_t)(v + 0.5f) << shift;
    }
    on->pixels[y * on->width + x] = out;
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
    for (int y = 0; y < r.h; y++)
        fill((box){r.x, r.y + y, r.w, 1}, mix(top, bottom, r.h > 1 ? (float)y / (r.h - 1) : 0));
}

/* A one-pixel outline just inside the box. */
static inline void outline(box r, uint32_t c) {
    if (r.w <= 0 || r.h <= 0) return;
    fill((box){r.x, r.y, r.w, 1}, c);
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, c);
    fill((box){r.x, r.y, 1, r.h}, c);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, c);
}

/* How far the middle of a pixel is outside a box with rounded corners:
 * below zero inside it. */
static inline float outside(box r, float radius, int x, int y) {
    float half_w = r.w / 2.0f, half_h = r.h / 2.0f;
    if (radius > half_w) radius = half_w;
    if (radius > half_h) radius = half_h;
    float dx = fabsf(x + 0.5f - (r.x + half_w)) - (half_w - radius);
    float dy = fabsf(y + 0.5f - (r.y + half_h)) - (half_h - radius);
    float ox = dx > 0 ? dx : 0, oy = dy > 0 ? dy : 0;
    float in = dx > dy ? dx : dy;
    return sqrtf(ox * ox + oy * oy) + (in < 0 ? in : 0) - radius;
}

static inline float covered(float d) {
    float c = 0.5f - d;
    return c < 0 ? 0 : c > 1 ? 1 : c;
}

/* A box with rounded corners, smooth at the edge: one colour at its top
 * running to another at its bottom. */
static inline void round_fill(box r, float radius, uint32_t top, uint32_t bottom) {
    for (int y = r.y; y < r.y + r.h; y++) {
        uint32_t c = mix(top, bottom, r.h > 1 ? (float)(y - r.y) / (r.h - 1) : 0);
        for (int x = r.x; x < r.x + r.w; x++) {
            float cover = covered(outside(r, radius, x, y));
            if (cover > 0) blend(x, y, c, cover);
        }
    }
}

/* The outline of such a box, `thick` pixels wide, inside it. */
static inline void round_line(box r, float radius, float thick, uint32_t c) {
    for (int y = r.y; y < r.y + r.h; y++) {
        for (int x = r.x; x < r.x + r.w; x++) {
            float d = outside(r, radius, x, y);
            float cover = covered(d) - covered(d + thick);
            if (cover > 0) blend(x, y, c, cover);
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
    int reach = (int)spread + 1;
    for (int y = r.y - reach; y < r.y + r.h + reach; y++) {
        for (int x = r.x - reach; x < r.x + r.w + reach; x++) {
            float d = outside(r, radius, x, y);
            if (d <= -0.5f || d >= spread) continue;
            float t = 1 - (d < 0 ? 0 : d) / spread;
            blend(x, y, c, strength * t * t * (1 - covered(d)));
        }
    }
}

/* A solid arrowhead with its middle at (cx, cy), `size` pixels from base
 * to tip: 0 up, 1 down, 2 left, 3 right. */
static inline void arrow(int cx, int cy, int size, int way, uint32_t c) {
    for (int d = 0; d < size; d++) {
        switch (way) {
        case 0: fill((box){cx - d, cy - size / 2 + d, 2 * d + 1, 1}, c); break;
        case 1: fill((box){cx - d, cy + (size - 1) / 2 - d, 2 * d + 1, 1}, c); break;
        case 2: fill((box){cx - size / 2 + d, cy - d, 1, 2 * d + 1}, c); break;
        default: fill((box){cx + (size - 1) / 2 - d, cy - d, 1, 2 * d + 1}, c); break;
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
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, mix(for_dark, for_light, light(at(x, y))));
}

/* Show what is drawn in a box as out of use: faded towards a colour. */
static inline void fade(box r, uint32_t towards, float much) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, mix(at(x, y), towards, much));
}

#endif
