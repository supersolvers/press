/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Two Thousand and Twenty-Seven: a style for press in the manner of a
 * future seen from the year 2001, which arrived without ever going out of
 * fashion. Everything you press is a piece of lit, tinted gel; everything
 * you read lies on plain, solid ground. Windows are shells of gel in the
 * colour of what they are for, glowing a little when they are in front.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=twothousandtwentyseven ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

COLOUR(WINDOW, 0xeef3fa);   /* what a window is made of: pale and cool */
COLOUR(PAPER, 0xffffff);
COLOUR(INK, 0x0b1a33);      /* words: a navy near to black */
COLOUR(ACCENT, 0x2f8fff);   /* what is chosen, and gel that is lit */
FOLLOWER(EDGE, 0x9aaecb);
#define EDGE follows(EDGE, WINDOW)
FOLLOWER(GREY, 0x8a97ab);
#define GREY follows(GREY, WINDOW)
COLOUR(WHITE, 0xffffff);
COLOUR(BLACK, 0x000000);

/* A piece of gel by its colours: its top, the middle where it is deepest,
 * its foot (lighter again, lit from below), and its rim. */
typedef struct {
    uint32_t top, middle, bottom, edge;
} gel_of;

/* Clear gel, faintly blue, as the buttons are. */
static gel_of clear_gel(void) {
    gel_of g = {like(WINDOW, 0xffffff), like(WINDOW, 0xd9e4f3), like(WINDOW, 0xf4f8ff), like(WINDOW, 0x8ea6c8)};
    return g;
}

/* Gel of a colour. */
static gel_of tinted(uint32_t c) {
    gel_of g = {mix(c, WHITE, 0.45f), c, mix(c, WHITE, 0.30f), mix(c, BLACK, 0.38f)};
    return g;
}

/* Gel of a colour gone grey, for what is behind or out of use. */
static gel_of dulled(uint32_t c) {
    return tinted(mix(c, like(WINDOW, 0xaab6c8), 0.7f));
}

/* A piece of gel: shaded deepest in the middle and light again at its
 * foot, a glow inside it low down as if lit through, the light caught
 * along its top half, and a lit rim. */
static void gel(box r, float radius, gel_of g, float brighten, int sunk) {
    if (r.w <= 0 || r.h <= 0) return;
    uint32_t top = mix(g.top, WHITE, brighten), middle = mix(g.middle, WHITE, brighten), bottom = mix(g.bottom, WHITE, brighten);
    if (sunk) {
        top = mix(top, BLACK, 0.12f);
        middle = mix(middle, BLACK, 0.12f);
    } else {
        shadow((box){r.x, r.y + 1, r.w, r.h}, radius, 2.5f, BLACK, 0.18f);
    }
    int s = fineness();
    for (int y = r.y * s; y < (r.y + r.h) * s; y++) {
        float t = r.h > 1 ? ((y + 0.5f) / s - r.y) / r.h : 0;
        uint32_t c = t < 0.55f ? mix(top, middle, t / 0.55f) : mix(middle, bottom, (t - 0.55f) / 0.45f);
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            float cover = covered(outside_at(r, radius, (x + 0.5f) / s, (y + 0.5f) / s) * s);
            if (cover > 0) blend_fine(x, y, c, cover);
        }
    }
    round_line(r, radius, 1, g.edge);
    if (!sunk && r.h >= 6) {
        float in = r.h > 20 ? 2.5f : 1.5f;
        box shine = {r.x + (int)in, r.y + 1, r.w - 2 * (int)in, r.h / 2};
        float rr = radius - in;
        if (shine.w > 0 && shine.h > 0)
            round_fill(shine, rr > 0 ? rr : 0, with_alpha(WHITE, 0.75f), with_alpha(WHITE, 0.08f));
    }
}

/* A sphere of glass: lit from the top left, with a bright cap. */
static void orb(box r, uint32_t c, int sunk) {
    float cx = r.x + r.w / 2.0f, cy = r.y + r.h / 2.0f, rad = r.w / 2.0f;
    int s = fineness();
    uint32_t light_ = mix(c, WHITE, 0.6f), dark = mix(c, BLACK, sunk ? 0.55f : 0.42f);
    if (!sunk) shadow((box){r.x, r.y + 1, r.w, r.h}, rad, 2.0f, BLACK, 0.2f);
    for (int y = r.y * s; y < (r.y + r.h) * s; y++)
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            float fx = (x + 0.5f) / s, fy = (y + 0.5f) / s;
            float d = sqrtf((fx - cx) * (fx - cx) + (fy - cy) * (fy - cy)) - rad;
            float cover = covered(d * s);
            if (cover <= 0) continue;
            float lx = (fx - (cx - rad * 0.3f)) / rad, ly = (fy - (cy - rad * 0.35f)) / rad;
            float t = sqrtf(lx * lx + ly * ly) / 1.3f;
            uint32_t col = t < 0.5f ? mix(light_, c, t * 2) : mix(c, dark, (t - 0.5f) * 2);
            blend_fine(x, y, col, cover);
        }
    /* the cap of light across its top */
    float capw = rad * 0.62f, caph = rad * 0.36f, capy = cy - rad * 0.45f;
    for (int y = r.y * s; y < (int)((cy) * s); y++)
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            float fx = (x + 0.5f) / s, fy = (y + 0.5f) / s;
            float e = ((fx - cx) / capw) * ((fx - cx) / capw) + ((fy - capy) / caph) * ((fy - capy) / caph);
            if (e < 1) blend_fine(x, y, WHITE, (1 - e) * (sunk ? 0.3f : 0.75f) * (1 - (fy - (capy - caph)) / (2 * caph) * 0.6f));
        }
    round_line(r, rad, 1, with_alpha(mix(c, BLACK, 0.45f), 0.8f));
}

/* A soft glow round something: where the keyboard is. */
static void halo(box r, float radius) {
    shadow(r, radius, 4.0f, ACCENT, 0.7f);
}

static void words_middle(const press_part *p, box r, uint32_t c) {
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2, c);
}

static void button(const press_part *p, box r) {
    float radius = r.h / 2.0f;
    int held = has(p, PRESS_STATE_HELD), hot = has(p, PRESS_STATE_HOT), dflt = has(p, PRESS_STATE_DEFAULT);
    int live = has(p, PRESS_STATE_ENABLED);
    if (has(p, PRESS_STATE_FOCUSED)) halo(r, radius);
    gel_of g = !live ? dulled(like(WINDOW, 0xd6dde8)) : (dflt || held) ? tinted(held ? mix(ACCENT, BLACK, 0.15f) : ACCENT) : clear_gel();
    /* the one Return presses breathes, slowly */
    float brighten = dflt && !held ? 0.12f * (0.5f + 0.5f * sinf(p->a * 6.2831853f / 40.0f)) : hot && !held ? 0.25f : 0;
    gel(r, radius, g, brighten, held);
    int d = held ? 1 : 0;
    uint32_t c = !live ? GREY : (dflt || held) ? WHITE : INK;
    words_middle(p, (box){r.x, r.y + d, r.w, r.h}, c);
}

static void label(const press_part *p, int x, box r) {
    words(p->text, x, r.y + (r.h - line_height()) / 2, has(p, PRESS_STATE_ENABLED) ? INK : GREY);
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 16) / 2, 16, 16};
    int on_ = has(p, PRESS_STATE_CHECKED);
    if (has(p, PRESS_STATE_FOCUSED)) halo(b, 4);
    gel(b, 4, on_ ? tinted(ACCENT) : clear_gel(), 0, has(p, PRESS_STATE_HELD));
    if (on_) tick(b.x + 3, b.y + 3, 10, 3, 3, WHITE);
    label(p, r.x + 16 + 7, r);
}

static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 16) / 2, 16, 16};
    int on_ = has(p, PRESS_STATE_CHECKED);
    if (has(p, PRESS_STATE_FOCUSED)) halo(b, 8);
    if (on_) orb(b, ACCENT, has(p, PRESS_STATE_HELD));
    else gel(b, 8, clear_gel(), 0, has(p, PRESS_STATE_HELD));
    if (on_) round_fill((box){b.x + 5, b.y + 5, 6, 6}, 3, WHITE, WHITE);
    label(p, r.x + 16 + 7, r);
}

static void well(const press_part *p, box r) {
    float radius = p->a == 0 ? 6 : 4;
    if (has(p, PRESS_STATE_FOCUSED)) halo(r, radius);
    round_box(r, radius, PAPER, PAPER, has(p, PRESS_STATE_FOCUSED) ? mix(ACCENT, WHITE, 0.2f) : EDGE);
    /* set into the window: a shade along its inside top */
    box in = inset(r, 1);
    clip_to((box){in.x, in.y, in.w, 3});
    round_line(in, radius - 1, 1, like(WINDOW, 0xd3dceb));
    clip_off();
}

static void dim_in(box r) {
    fade(r, WINDOW, 0.55f);
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove, knob, done;
    if (vertical) {
        groove = (box){r.x + r.w / 2 - 3, r.y + p->b / 2, 6, r.h - p->b + 1};
        knob = (box){r.x + (r.w - p->b) / 2, p->a, p->b, p->b};
        done = (box){groove.x, p->a + p->b / 2, 6, groove.y + groove.h - (p->a + p->b / 2)};
    } else {
        groove = (box){r.x + p->b / 2, r.y + r.h / 2 - 3, r.w - p->b + 1, 6};
        knob = (box){p->a, r.y + (r.h - p->b) / 2, p->b, p->b};
        done = (box){groove.x, groove.y, p->a + p->b / 2 - groove.x, 6};
    }
    round_box(groove, 3, like(WINDOW, 0xc3cfe0), like(WINDOW, 0xe4ebf5), like(WINDOW, 0x9aaecb));
    /* the travel already made is lit */
    if (done.w > 0 && done.h > 0) round_fill(done, 3, mix(ACCENT, WHITE, 0.25f), ACCENT);
    if (has(p, PRESS_STATE_FOCUSED)) halo(knob, p->b / 2.0f);
    orb(knob, has(p, PRESS_STATE_HELD) ? ACCENT : like(WINDOW, 0xe6edf7), 0);
}

static void progress(const press_part *p, box r) {
    float radius = r.h / 2.0f;
    round_box(r, radius, like(WINDOW, 0xd4deec), like(WINDOW, 0xf2f6fc), EDGE);
    box done = r;
    if (!has(p, PRESS_STATE_BUSY)) done.w = (int)((long long)r.w * p->a / (p->b > 0 ? p->b : 1));
    if (done.w < r.h) {
        if (done.w <= 0) return;
        done.w = r.h;
    }
    gel(done, radius, tinted(ACCENT), 0, 0);
    /* light running along it while work goes on */
    int span = 48, at = ((p->phase * 3) % (done.w + span)) - span / 2;
    for (int y = done.y + 1; y < done.y + done.h - 1; y++)
        for (int x = done.x + 1; x < done.x + done.w - 1; x++) {
            float d = fabsf((float)(x - done.x - at)) / (span / 2.0f);
            if (d < 1 && outside(done, radius, x, y) < -1) blend(x, y, WHITE, 0.35f * (1 - d) * (1 - d));
        }
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    fill(r, WINDOW);
    if (length < 2 * thick) return;
    /* a slim channel between the two arrows */
    box track = vertical ? (box){r.x + thick / 2 - 3, r.y + thick, 6, r.h - 2 * thick} : (box){r.x + thick, r.y + thick / 2 - 3, r.w - 2 * thick, 6};
    round_fill(track, 3, like(WINDOW, 0xd6dfec), like(WINDOW, 0xe2e9f3));
    for (int second = 0; second < 2; second++) {
        int from = second ? (vertical ? r.y + r.h : r.x + r.w) - thick : (vertical ? r.y : r.x);
        box b = vertical ? (box){r.x, from, thick, thick} : (box){from, r.y, thick, thick};
        int held = live && p->a == second + 1, hot = live && p->d == second + 1;
        uint32_t c = !live ? like(WINDOW, 0xb8c3d4) : held ? mix(ACCENT, BLACK, 0.2f) : hot ? ACCENT : like(WINDOW, 0x5a6b85);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 3, vertical ? second : 2 + second, c);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x + 2, p->b, r.w - 4, p->c} : (box){p->b, r.y + 2, p->c, r.h - 4};
        int lit = p->d == 3 || p->a == 3;
        gel(t, (thick - 4) / 2.0f, tinted(lit ? ACCENT : mix(ACCENT, like(WINDOW, 0x8ea6c8), 0.45f)), 0, 0);
    }
}

static void tab(const press_part *p, box r) {
    /* each tab is a capsule of its own, the chosen one lit */
    box b = {r.x + 2, r.y + 2, r.w - 4, r.h - 6};
    int front = has(p, PRESS_STATE_FRONT);
    if (front && has(p, PRESS_STATE_FOCUSED)) halo(b, b.h / 2.0f);
    gel(b, b.h / 2.0f, front ? tinted(ACCENT) : clear_gel(), has(p, PRESS_STATE_HOT) && !front ? 0.25f : 0, 0);
    words_middle(p, b, front ? WHITE : INK);
}

static void popup(const press_part *p, box r) {
    float radius = r.h / 2.0f;
    if (has(p, PRESS_STATE_FOCUSED)) halo(r, radius);
    gel(r, radius, clear_gel(), has(p, PRESS_STATE_HOT) ? 0.2f : 0, 0);
    /* the end that opens it is an orb of lit gel */
    box cap = {r.x + r.w - r.h + 3, r.y + 3, r.h - 6, r.h - 6};
    orb(cap, ACCENT, has(p, PRESS_STATE_HELD));
    arrow(cap.x + cap.w / 2, cap.y + cap.h / 2 + 1, 3, 1, WHITE);
    words(p->text, r.x + 12, r.y + (r.h - line_height()) / 2, INK);
}

/* The tint of a frame: the one the desktop gives it (d is 1 when it does),
 * or the style's own. */
static uint32_t frame_tint(const press_part *p) {
    return p->d == 1 ? (0xff000000u | p->ink) : ACCENT;
}

/* A window's frame: a shell of gel in the window's colour, the title rail
 * along its top lit from above, a small orb of the colour at its left, the
 * title beside it, and a glow round the whole when it is in front. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), zoomed = has(p, PRESS_STATE_CHECKED);
    uint32_t tint = frame_tint(p);
    float radius = zoomed ? 4 : 12;
    if (active && !zoomed) shadow(r, radius, 9, mix(tint, WHITE, 0.25f), 0.55f);
    gel_of g = active ? tinted(mix(tint, WHITE, 0.18f)) : dulled(tint);
    /* the shell, with the light caught along the rail rather than half the window */
    int s = fineness();
    for (int y = r.y * s; y < (r.y + r.h) * s; y++) {
        float fy = (y + 0.5f) / s - r.y;
        float t = p->a > 1 ? fy / p->a : 1;
        uint32_t c = t < 1 ? mix(g.top, g.middle, t) : mix(g.middle, g.bottom, (fy - p->a) / (r.h - p->a > 1 ? r.h - p->a : 1));
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            float cover = covered(outside_at(r, radius, (x + 0.5f) / s, (y + 0.5f) / s) * s);
            if (cover > 0) blend_fine(x, y, c, cover);
        }
    }
    box shine = {r.x + 3, r.y + 2, r.w - 6, p->a / 2};
    if (shine.w > 0 && shine.h > 0) round_fill(shine, radius - 3 > 0 ? radius - 3 : 0, with_alpha(WHITE, active ? 0.7f : 0.45f), with_alpha(WHITE, 0.1f));
    round_line(r, radius, 1, with_alpha(mix(tint, WHITE, 0.6f), 0.9f));
    /* the line where the content will lie */
    box in = {r.x + p->b - 1, r.y + p->a - 1, r.w - 2 * p->b + 2, r.h - p->a - p->b + 2};
    if (in.w > 0 && in.h > 0) outline(in, with_alpha(mix(tint, BLACK, 0.4f), 0.5f));
    /* the window's mark and its title */
    int side = 14, my = r.y + (p->a - side) / 2;
    orb((box){r.x + p->b + 9, my, side, side}, active ? mix(tint, WHITE, 0.35f) : mix(tint, like(WINDOW, 0xaab6c8), 0.6f), 0);
    int tx = r.x + p->b + 9 + side + 9, ty = r.y + (p->a - line_height()) / 2;
    uint32_t ink = active ? mix(tint, INK, 0.85f) : mix(INK, WHITE, 0.35f);
    words(p->text, tx, ty + 1, with_alpha(WHITE, 0.55f));
    words(p->text, tx, ty, ink);
}

/* The glass orbs at the right of the rail: close, make large, put away.
 * Each wears its mark always, and lights in the window's colour (close in
 * coral) under the pointer. */
static void frame_button(const press_part *p, box r) {
    int hot = has(p, PRESS_STATE_HOT), held = has(p, PRESS_STATE_HELD), active = has(p, PRESS_STATE_ENABLED);
    uint32_t glass = active ? like(WINDOW, 0xe8f0fb) : like(WINDOW, 0xcfd8e5);
    uint32_t c = hot ? (p->a == PRESS_FRAME_CLOSE ? 0xffff6b5au : frame_tint(p)) : glass;
    orb(r, c, held);
    int cx = r.x + r.w / 2, cy = r.y + r.h / 2;
    uint32_t ink = hot ? WHITE : with_alpha(INK, active ? 0.8f : 0.45f);
    switch (p->a) {
    case PRESS_FRAME_CLOSE: cross(cx, cy, 3, 0, 2, ink); break;
    case PRESS_FRAME_MINIMIZE: fill_over((box){cx - 4, cy + 1, 8, 2}, ink); break;
    case PRESS_FRAME_ZOOM:
        outline((box){cx - 4, cy - 4, 8, 8}, ink);
        outline((box){cx - 3, cy - 3, 6, 6}, ink);
        break;
    default: round_fill((box){cx - 2, cy - 2, 4, 4}, 2, ink, ink); break;
    }
}

static void menu_frame(const press_part *p, box r) {
    /* a panel of frosted glass, rounded, standing off on a soft shadow */
    box panel = {r.x, r.y, r.w - p->a, r.h - p->a};
    shadow((box){panel.x + 2, panel.y + 3, panel.w - 2, panel.h - 2}, 8, (float)p->a, like(INK, 0x0b1a33), 0.45f);
    /* the corners the menu's colour filled square are cut round again */
    int s = fineness();
    for (int y = panel.y * s; y < (panel.y + panel.h) * s; y++)
        for (int x = panel.x * s; x < (panel.x + panel.w) * s; x++) {
            float d = outside_at(panel, 8, (x + 0.5f) / s, (y + 0.5f) / s) * s;
            if (d > -0.5f) {
                uint32_t *px_ = fine_at(x, y);
                if (px_) *px_ = with_alpha(*px_, chan(*px_, 24) / 255.0f * (1 - covered(-d)));
            }
        }
    round_line(panel, 8, 1, with_alpha(mix(ACCENT, WHITE, 0.4f), 0.9f));
}

static void select_(const press_part *p, box r) {
    if (p->a == 1) {
        recolour(r, mix(ACCENT, WHITE, 0.72f), INK);
        return;
    }
    /* a row or a menu's item: a capsule of lit gel, the words white on it */
    box b = p->a == 2 ? (box){r.x + 3, r.y + 1, r.w - 6, r.h - 2} : r;
    float radius = p->a == 2 ? b.h / 2.0f : 4;
    int s = fineness();
    for (int y = r.y * s; y < (r.y + r.h) * s; y++)
        for (int x = r.x * s; x < (r.x + r.w) * s; x++) {
            uint32_t *q = fine_at(x, y);
            if (!q) continue;
            float cover = covered(outside_at(b, radius, (x + 0.5f) / s, (y + 0.5f) / s) * s);
            float t = ((y + 0.5f) / s - b.y) / (b.h > 0 ? b.h : 1);
            uint32_t ground = t < 0.5f ? mix(mix(ACCENT, WHITE, 0.3f), ACCENT, t * 2) : mix(ACCENT, mix(ACCENT, BLACK, 0.12f), (t - 0.5f) * 2);
            uint32_t lit = mix(WHITE, ground, light(*q));
            *q = mix(*q, (lit & 0x00ffffffu) | (*q & 0xff000000u), cover);
        }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, WINDOW); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL: well(p, r); break;
    case PRESS_PART_SELECT: select_(p, r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: halo(r, 4); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: progress(p, r); break;
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, WINDOW); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE: round_box(r, 8, like(WINDOW, 0xf7f9fd), like(WINDOW, 0xe9eff8), EDGE); break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        gradient((box){r.x, r.y, r.w, r.h - 1}, like(WINDOW, 0xfbfdff), like(WINDOW, 0xe3eaf5));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, EDGE);
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        box b = {r.x + 1, r.y + 2, r.w - 2, r.h - 5};
        if (lit) gel(b, b.h / 2.0f, tinted(ACCENT), 0, 0);
        else if (has(p, PRESS_STATE_HOT) && has(p, PRESS_STATE_ENABLED)) round_fill(b, b.h / 2.0f, with_alpha(ACCENT, 0.16f), with_alpha(ACCENT, 0.16f));
        words(p->text, r.x + 10, r.y + (r.h - line_height()) / 2 - 1, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? INK : GREY);
        break;
    }
    case PRESS_PART_MENU_FRAME: menu_frame(p, r); break;
    case PRESS_PART_MENU_SEPARATOR: fill((box){r.x + 10, r.y + r.h / 2, r.w - 20, 1}, like(WINDOW, 0xd0d9e7)); break;
    case PRESS_PART_MENU_CHECK: tick(r.x + 4, r.y + (r.h - 8) / 2, 8, 2, 2, p->ink); break;
    case PRESS_PART_MENU_ARROW: arrow(r.x + r.w - 10, r.y + r.h / 2, 3, 3, p->ink); break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, PAPER);
        round_line(r, 4, 1, EDGE);
        break;
    case PRESS_PART_HEADER:
        gradient((box){r.x, r.y, r.w, r.h - 1}, like(WINDOW, 0xfbfdff), like(WINDOW, 0xe6ecf6));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, EDGE);
        break;
    case PRESS_PART_HEADER_CELL:
        if (has(p, PRESS_STATE_HOT)) fill_over(r, with_alpha(ACCENT, 0.12f));
        fill((box){r.x + r.w - 1, r.y + 4, 1, r.h - 8}, like(WINDOW, 0xc6d1e2));
        break;
    case PRESS_PART_CARET: fill(r, ACCENT); break;
    case PRESS_PART_GROUP_BOX: {
        int top = p->text[0] ? line_height() + 2 : 0;
        box in = {r.x, r.y + top, r.w, r.h - top};
        round_box(in, 8, like(WINDOW, 0xf6f9fd), like(WINDOW, 0xeaf0f8), like(WINDOW, 0xc4cfe0));
        if (p->text[0]) words(p->text, r.x + 10, r.y, mix(ACCENT, INK, 0.5f));
        break;
    }
    case PRESS_PART_TOOLBAR:
        gradient((box){r.x, r.y, r.w, r.h - 1}, like(WINDOW, 0xf8fbff), like(WINDOW, 0xe8eef7));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, like(WINDOW, 0xc4cfe0));
        break;
    case PRESS_PART_STATUS_BAR:
        gradient(r, like(WINDOW, 0xe8eef7), like(WINDOW, 0xf4f7fc));
        fill((box){r.x, r.y, r.w, 1}, like(WINDOW, 0xc4cfe0));
        break;
    case PRESS_PART_STATUS_PANEL:
        if (p->a > 0) fill((box){r.x - 3, r.y + 3, 1, r.h - 6}, like(WINDOW, 0xc4cfe0));
        break;
    case PRESS_PART_STEPPER: {
        box s = {r.x + (r.w - 16) / 2, r.y + (r.h - 26) / 2, 16, 26};
        gel(s, 8, clear_gel(), 0, 0);
        if (p->a) {
            box half = {s.x, p->a == 2 ? s.y : s.y + 13, 16, 13};
            clip_to(half);
            gel(s, 8, tinted(ACCENT), 0, 1);
            clip_off();
        }
        fill((box){s.x + 3, s.y + 13, 10, 1}, with_alpha(INK, 0.2f));
        arrow(s.x + 8, s.y + 7, 3, 0, p->a == 2 ? WHITE : INK);
        arrow(s.x + 8, s.y + 19, 3, 1, p->a == 1 ? WHITE : INK);
        break;
    }
    case PRESS_PART_COMBO_BUTTON: {
        box b = {r.x + 2, r.y + 2, r.w - 4, r.h - 4};
        int side = b.w < b.h ? b.w : b.h;
        box o = {b.x + (b.w - side) / 2, b.y + (b.h - side) / 2, side, side};
        orb(o, ACCENT, has(p, PRESS_STATE_HELD));
        arrow(o.x + o.w / 2, o.y + o.h / 2 + 1, 3, 1, WHITE);
        break;
    }
    case PRESS_PART_TOOLTIP: {
        round_box(r, 6, like(WINDOW, 0xffffff), like(WINDOW, 0xe8f1ff), mix(ACCENT, WHITE, 0.3f));
        words(p->text, r.x + 7, r.y + (r.h - line_height()) / 2, INK);
        break;
    }
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 26;
    m->button_min_width = 78;
    m->button_pad = 18;
    m->default_ring = 0;
    m->check_size = 16;
    m->check_gap = 7;
    m->radio_size = 16;
    m->radio_gap = 7;
    m->field_inset_x = 8;
    m->field_inset_y = 5;
    m->progress_height = 14;
    m->slider_thick = 22;
    m->scroll_bar = 14;
    m->slider_knob = 20;
    m->tab_height = 30;
    m->popup_height = 26;
    m->row_height = 22;
    m->menu_bar_height = 26;
    m->menu_item_height = 24;
    m->menu_shadow = 10;
    m->menu_border = 5;
    /* a shell of gel round the window, the title on a rail along its top */
    m->frame_title = 34;
    m->frame_shadow = 12;
    m->frame_border = 4;
    m->frame_button_width = 18;
    m->frame_button_height = 18;
    m->frame_button_gap = 6;
    m->frame_button_inset = 12;
    m->frame_button_rise = 0;
    m->frame_left = 0;
    m->frame_right = 0x231; /* close, make large, put away, from the right */
    m->menu_in_window = 1;
    m->scroll_arrows = 0;
    m->tab_align = 1;
}

static void palette(press_palette *p) {
    p->window = WINDOW;
    p->text = INK;
    p->paper = PAPER;
    p->ink = INK;
    p->menu = 0xf8f6f9fdu; /* frosted: a little of what is under it shows */
    p->menu_text = INK;
    p->tip = 0xfff4f8ffu;
    p->tip_text = INK;
}

static int32_t typeface(uint32_t face, press_type *out) {
    /* plain words in a clear sans; titles in a round, wide face */
    const char *sans = "IBM Plex Sans,Clear Sans,Cantarell,DejaVu Sans";
    const char *round = "Comfortaa,Varela Round,Nunito,DejaVu Sans";
    const char *fixed = "DM Mono,IBM Plex Mono,DejaVu Sans Mono";
    switch (face) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 14; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 12; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 12; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 15; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 13; break;
    case PRESS_FACE_TITLE: strcpy(out->family, round); out->size = 20; out->bold = 1; break;
    case PRESS_FACE_FRAME: strcpy(out->family, round); out->size = 15; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"window", &WINDOW},
    {"paper", &PAPER},
    {"ink", &INK},
    {"accent", &ACCENT},
    {"edge", &EDGE_SAID},
    {"grey", &GREY_SAID},
    {"white", &WHITE},
    {"black", &BLACK},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "twothousandtwentyseven", metrics, palette, draw, PRESS_WANTS_FINE | PRESS_WANTS_PULSE | PRESS_WANTS_HOVER, typeface, colours};

const press_style *press_style_entry(void) {
    return &style;
}
