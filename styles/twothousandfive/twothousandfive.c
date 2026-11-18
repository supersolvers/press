/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Two Thousand and Five: a style for press in the manner of the glossy
 * desktops of 2005. Buttons are lozenges of clear or blue gel with the
 * light caught along their tops, the one Return presses throbs, windows are
 * finely striped, and menus cast a soft shadow. Every pixel of it is drawn
 * here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=twothousandfive ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

#define WINDOW rgb(0xececec)
#define STRIPE rgb(0xe4e4e4)
#define BLUE rgb(0x3875d7)      /* what is selected */
#define HALO rgb(0x5a9cf0)      /* round what has the keyboard */
#define EDGE rgb(0x767676)
#define GREY rgb(0x9a9a9a)
#define WHITE rgb(0xffffff)
#define BLACK rgb(0x000000)

/* The colours of a piece of gel, from its top to its bottom, and its edge. */
typedef struct {
    uint32_t top, middle, bottom, edge;
} gel_of;

static const gel_of CLEAR = {0xfff8f8f8u, 0xffe0e0e0u, 0xfffcfcfcu, 0xff767676u};
static const gel_of WATER = {0xff8dbff6u, 0xff3585e8u, 0xffa6d8ffu, 0xff2c4fa0u};
static const gel_of DEEP = {0xff6a9fe0u, 0xff2468c8u, 0xff86bdf0u, 0xff223f86u};
static const gel_of DULL = {0xfff4f4f4u, 0xffe6e6e6u, 0xfff6f6f6u, 0xffb0b0b0u};

/* A piece of gel: shaded from its top, darkest in the middle and light
 * again at its foot, with the light caught along its top and a little
 * shadow under it. */
static void gel(box r, float radius, gel_of g, float brighten) {
    uint32_t top = mix(g.top, WHITE, brighten), middle = mix(g.middle, WHITE, brighten), bottom = mix(g.bottom, WHITE, brighten);
    shadow((box){r.x, r.y + 1, r.w, r.h}, radius, 2.5f, BLACK, 0.2f);
    for (int y = r.y; y < r.y + r.h; y++) {
        float t = r.h > 1 ? (float)(y - r.y) / (r.h - 1) : 0;
        uint32_t c = t < 0.5f ? mix(top, middle, t * 2) : mix(middle, bottom, (t - 0.5f) * 2);
        for (int x = r.x; x < r.x + r.w; x++) {
            float cover = covered(outside(r, radius, x, y));
            if (cover > 0) blend(x, y, c, cover);
        }
    }
    round_line(r, radius, 1, g.edge);
    box shine = {r.x + 2, r.y + 1, r.w - 4, r.h / 2 - 1};
    if (shine.w > 0 && shine.h > 0)
        round_fill(shine, radius - 1 > 0 ? radius - 1 : 0, with_alpha(WHITE, 0.8f), with_alpha(WHITE, 0.1f));
}

/* A glow round a box: this is where the keyboard is. */
static void halo(box r, float radius) {
    shadow(r, radius, 3.5f, HALO, 0.95f);
}

static void button(const press_part *p, box r) {
    float radius = r.h / 2.0f;
    gel_of g = CLEAR;
    float brighten = 0;
    if (has(p, PRESS_STATE_HELD)) {
        g = DEEP;
    } else if (has(p, PRESS_STATE_DEFAULT)) {
        /* the one Return presses throbs, slowly */
        g = WATER;
        brighten = 0.28f * (0.5f + 0.5f * sinf(p->a * 6.2831853f / 26.0f));
    }
    if (has(p, PRESS_STATE_FOCUSED))
        halo(r, radius);
    gel(r, radius, g, brighten);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2, BLACK);
}

static void label(const press_part *p, int x, box r) {
    words(p->text, x, r.y + (r.h - line_height()) / 2, BLACK);
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 14) / 2, 14, 14};
    int on_ = has(p, PRESS_STATE_CHECKED);
    if (has(p, PRESS_STATE_FOCUSED))
        halo(b, 3);
    gel(b, 3, has(p, PRESS_STATE_HELD) ? DEEP : on_ ? WATER : CLEAR, 0);
    if (on_) {
        /* a tick that runs up out of the box, as a pen would leave it */
        static const int rise[9] = {4, 5, 6, 5, 4, 3, 2, 1, 0};
        for (int i = 0; i < 9; i++)
            fill((box){b.x + 3 + i, b.y + 2 + rise[i], 1, 3}, BLACK);
    }
    label(p, r.x + 14 + 6, r);
}

static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 14) / 2, 14, 14};
    int on_ = has(p, PRESS_STATE_CHECKED);
    if (has(p, PRESS_STATE_FOCUSED))
        halo(b, 7);
    gel(b, 7, has(p, PRESS_STATE_HELD) ? DEEP : on_ ? WATER : CLEAR, 0);
    if (on_)
        round_fill((box){b.x + 5, b.y + 5, 4, 4}, 2, BLACK, BLACK);
    label(p, r.x + 14 + 6, r);
}

static void well(const press_part *p, box r) {
    if (has(p, PRESS_STATE_FOCUSED))
        halo(inset(r, 1), 1);
    fill(r, WHITE);
    outline(r, rgb(0xb4b4b4));
    /* set into the window: its top edge in shade, and a little shade under that */
    fill((box){r.x, r.y, r.w, 1}, rgb(0x8a8a8a));
    fill((box){r.x + 1, r.y + 1, r.w - 2, 1}, rgb(0xe2e2e2));
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, mix(c, GREY, 0.7f * (1 - light(c))));
        }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove, knob;
    if (vertical) {
        groove = (box){r.x + r.w / 2 - 2, r.y + p->b / 2, 5, r.h - p->b + 1};
        knob = (box){r.x + (r.w - p->b) / 2, p->a, p->b, p->b};
    } else {
        groove = (box){r.x + p->b / 2, r.y + r.h / 2 - 2, r.w - p->b + 1, 5};
        knob = (box){p->a, r.y + (r.h - p->b) / 2, p->b, p->b};
    }
    round_box(groove, 2.5f, rgb(0x9c9c9c), rgb(0xe2e2e2), rgb(0x6e6e6e));
    if (has(p, PRESS_STATE_FOCUSED))
        halo(knob, p->b / 2.0f);
    gel(knob, p->b / 2.0f, has(p, PRESS_STATE_HELD) ? WATER : CLEAR, 0);
}

static void progress(const press_part *p, box r) {
    float radius = r.h / 2.0f;
    round_box(r, radius, rgb(0xd6d6d6), WHITE, rgb(0x7e7e7e));
    box done = r;
    if (!has(p, PRESS_STATE_BUSY))
        done.w = (int)((long long)r.w * p->a / (p->b > 0 ? p->b : 1));
    if (done.w < r.h) {
        if (done.w <= 0) return;
        done.w = r.h; /* never less than a bead */
    }
    gel(done, radius, WATER, 0);
    /* lighter bands slanting across it, which move while work goes on */
    for (int y = done.y + 1; y < done.y + done.h - 1; y++)
        for (int x = done.x + 1; x < done.x + done.w - 1; x++)
            if (((x + y - p->phase * 2) % 16 + 16) % 16 < 8 && outside(done, radius, x, y) < -1)
                blend(x, y, WHITE, 0.22f);
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    /* the track is a trough, in shade along one side */
    fill(r, rgb(0xf2f2f2));
    outline(r, rgb(0xb8b8b8));
    if (vertical) {
        fill((box){r.x + 1, r.y + 1, 2, r.h - 2}, rgb(0xdadada));
        fill((box){r.x + 3, r.y + 1, 2, r.h - 2}, rgb(0xe8e8e8));
    } else {
        fill((box){r.x + 1, r.y + 1, r.w - 2, 2}, rgb(0xdadada));
        fill((box){r.x + 1, r.y + 3, r.w - 2, 2}, rgb(0xe8e8e8));
    }
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        int held = live && p->a == second + 1;
        gel(b, 2, !live ? DULL : held ? WATER : CLEAR, 0);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? rgb(0x303030) : GREY);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x + 1, p->b, r.w - 2, p->c} : (box){p->b, r.y + 1, p->c, r.h - 2};
        gel(t, (thick - 2) / 2.0f, WATER, 0);
    }
}

static void tab(const press_part *p, box r) {
    /* tabs are a row of buttons that stand just above the pane */
    box b = {r.x, r.y, r.w, r.h - 4};
    int front = has(p, PRESS_STATE_FRONT);
    if (front && has(p, PRESS_STATE_FOCUSED))
        halo(b, 4);
    gel(b, 4, front ? WATER : CLEAR, 0);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, b.y + (b.h - line_height()) / 2, BLACK);
}

static void popup(const press_part *p, box r) {
    float radius = 5;
    box cap = {r.x + r.w - r.h, r.y, r.h, r.h};
    if (has(p, PRESS_STATE_FOCUSED))
        halo(r, radius);
    gel(r, radius, CLEAR, 0);
    /* the end that opens it is of blue gel, with an arrow each way */
    for (int y = cap.y; y < cap.y + cap.h; y++) {
        float t = cap.h > 1 ? (float)(y - cap.y) / (cap.h - 1) : 0;
        gel_of g = has(p, PRESS_STATE_HELD) ? DEEP : WATER;
        uint32_t c = t < 0.5f ? mix(g.top, g.middle, t * 2) : mix(g.middle, g.bottom, (t - 0.5f) * 2);
        for (int x = cap.x; x < cap.x + cap.w; x++) {
            float cover = covered(outside(r, radius, x, y));
            if (cover > 0) blend(x, y, c, cover);
        }
    }
    round_fill((box){cap.x + 1, cap.y + 1, cap.w - 3, cap.h / 2 - 1}, 3, with_alpha(WHITE, 0.7f), with_alpha(WHITE, 0.1f));
    round_line(r, radius, 1, rgb(0x5a5a5a));
    arrow(cap.x + cap.w / 2, cap.y + cap.h / 2 - 4, 4, 0, BLACK);
    arrow(cap.x + cap.w / 2, cap.y + cap.h / 2 + 4, 4, 1, BLACK);
    words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, BLACK);
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND:
        /* fine stripes across, two pixels and two */
        for (int y = r.y; y < r.y + r.h; y++)
            fill((box){r.x, y, r.w, 1}, y % 4 < 2 ? WINDOW : STRIPE);
        break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL: well(p, r); break;
    case PRESS_PART_SELECT: recolour(r, BLUE, WHITE); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: halo(r, 2); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: progress(p, r); break;
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, rgb(0xf2f2f2)); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE: {
        /* a box let into the window, a shade darker than it */
        round_box(r, 5, rgb(0xdcdcdc), rgb(0xe6e6e6), rgb(0xa8a8a8));
        break;
    }
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        gradient((box){r.x, r.y, r.w, r.h / 2}, WHITE, rgb(0xededed));
        gradient((box){r.x, r.y + r.h / 2, r.w, r.h - r.h / 2 - 1}, rgb(0xe2e2e2), rgb(0xf4f4f4));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, rgb(0x8c8c8c));
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) gradient((box){r.x, r.y, r.w, r.h}, rgb(0x5d8fe8), rgb(0x2a5fd0));
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : GREY);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        /* no line round a menu: it is known by the shadow it casts */
        box frame = {r.x, r.y, r.w - p->a, r.h - p->a};
        shadow((box){frame.x + 1, frame.y + 2, frame.w - 2, frame.h - 2}, 3, (float)p->a, BLACK, 0.5f);
        outline(frame, with_alpha(BLACK, 0.28f));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR: fill((box){r.x + 1, r.y + r.h / 2, r.w - 2, 1}, rgb(0xd4d4d4)); break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, WINDOW);
        outline(r, rgb(0x9e9e9e));
        break;
    case PRESS_PART_HEADER:
        gradient((box){r.x, r.y, r.w, r.h - 1}, WHITE, rgb(0xe4e4e4));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, rgb(0xa5a5a5));
        break;
    case PRESS_PART_HEADER_CELL: fill((box){r.x + r.w - 1, r.y + 2, 1, r.h - 5}, rgb(0xc4c4c4)); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 21;
    m->button_min_width = 70;
    m->button_pad = 15;
    m->default_ring = 0;
    m->check_size = 14;
    m->check_gap = 6;
    m->radio_size = 14;
    m->radio_gap = 6;
    m->field_inset_x = 5;
    m->field_inset_y = 4;
    m->progress_height = 12;
    m->slider_thick = 19;
    m->scroll_bar = 15;
    m->slider_knob = 15;
    m->tab_height = 23;
    m->popup_height = 21;
    m->row_height = 18;
    m->menu_bar_height = 22;
    m->menu_item_height = 18;
    m->menu_shadow = 9;
}

static void palette(press_palette *p) {
    p->window = WINDOW;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = 0xf4ffffffu; /* a menu lets a little of what is under it show */
    p->menu_text = BLACK;
}

static int32_t typeface(uint32_t face, press_type *out) {
    const char *sans = "Lucida Grande,Lucida Sans Unicode,Lucida Sans,DejaVu Sans";
    const char *fixed = "Monaco,Menlo,DejaVu Sans Mono,Liberation Mono";
    switch (face) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 13; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 11; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 11; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 13; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 12; break;
    case PRESS_FACE_TITLE: strcpy(out->family, fixed); out->size = 15; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

static const press_style style = {PRESS_STYLE_ABI, "twothousandfive", metrics, palette, draw, PRESS_WANTS_PULSE, typeface};

const press_style *press_style_entry(void) {
    return &style;
}
