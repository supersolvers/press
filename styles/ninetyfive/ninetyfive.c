/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Ninety-Five: a style for press in the manner of the grey, bevelled
 * desktops of the middle 1990s. It is a library of its own, written against
 * include/press_style.h and nothing else, and is here as much to show how a
 * style is written as to be worn.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=ninetyfive ./examples/c/controls
 */
#include "press_style.h"

#include <stddef.h>

#define FACE 0xffc0c0c0u   /* what everything is made of */
#define HILITE 0xffffffffu /* an edge the light falls on */
#define LIGHT 0xffdfdfdfu
#define SHADOW 0xff808080u /* an edge in shade */
#define DARK 0xff000000u
#define NAVY 0xff000080u   /* what is selected */
#define WHITE 0xffffffffu
#define BLACK 0xff000000u

typedef struct {
    int x, y, w, h;
} box;

static const press_surface *on;

static void px(int x, int y, uint32_t c) {
    if (x >= 0 && y >= 0 && x < on->width && y < on->height)
        on->pixels[y * on->width + x] = c;
}

static uint32_t at(int x, int y) {
    return (x >= 0 && y >= 0 && x < on->width && y < on->height) ? on->pixels[y * on->width + x] : 0;
}

static void fill(box r, uint32_t c) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, c);
}

static box inset(box r, int d) {
    box out = {r.x + d, r.y + d, r.w - 2 * d, r.h - 2 * d};
    if (out.w < 0) out.w = 0;
    if (out.h < 0) out.h = 0;
    return out;
}

/* A one-pixel edge: one colour along the top and left, another along the
 * bottom and right. */
static void edge(box r, uint32_t top_left, uint32_t bottom_right) {
    if (r.w <= 0 || r.h <= 0) return;
    fill((box){r.x, r.y, r.w - 1, 1}, top_left);
    fill((box){r.x, r.y, 1, r.h - 1}, top_left);
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, bottom_right);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, bottom_right);
}

/* Standing out of the page, as a button does. */
static void raised(box r) {
    fill(r, FACE);
    edge(r, HILITE, DARK);
    edge(inset(r, 1), LIGHT, SHADOW);
}

/* Set into the page, as a text field is. */
static void sunken(box r, uint32_t inside) {
    fill(r, inside);
    edge(r, SHADOW, HILITE);
    edge(inset(r, 1), DARK, LIGHT);
}

/* A dotted outline: where the keyboard is. */
static void dotted(box r) {
    for (int x = r.x; x < r.x + r.w; x++) {
        if ((x + r.y) % 2 == 0) px(x, r.y, BLACK);
        if ((x + r.y + r.h - 1) % 2 == 0) px(x, r.y + r.h - 1, BLACK);
    }
    for (int y = r.y; y < r.y + r.h; y++) {
        if ((y + r.x) % 2 == 0) px(r.x, y, BLACK);
        if ((y + r.x + r.w - 1) % 2 == 0) px(r.x + r.w - 1, y, BLACK);
    }
}

/* A solid arrowhead with its middle at (cx, cy): 0 up, 1 down, 2 left, 3 right. */
static void arrow(int cx, int cy, int way, uint32_t c) {
    for (int d = 0; d < 4; d++) {
        switch (way) {
        case 0: fill((box){cx - d, cy - 2 + d, 2 * d + 1, 1}, c); break;
        case 1: fill((box){cx - d, cy + 1 - d, 2 * d + 1, 1}, c); break;
        case 2: fill((box){cx - 2 + d, cy - d, 1, 2 * d + 1}, c); break;
        default: fill((box){cx + 1 - d, cy - d, 1, 2 * d + 1}, c); break;
        }
    }
}

static void words(const char *text, int x, int y, uint32_t c) {
    on->draw_text(on->ctx, text, x, y, c);
}

static int has(const press_part *p, uint32_t flag) {
    return (p->state & flag) != 0;
}

static void button(const press_part *p, box r) {
    int line = on->line_height(on->ctx), wide = on->text_width(on->ctx, p->text);
    int held = has(p, PRESS_STATE_HELD);
    box face = r;
    fill(r, FACE);
    if (has(p, PRESS_STATE_DEFAULT) || has(p, PRESS_STATE_FOCUSED)) {
        /* a black line all round the one Return presses */
        edge(r, DARK, DARK);
        face = inset(r, 1);
    }
    if (held) {
        edge(face, SHADOW, SHADOW);
    } else {
        edge(face, HILITE, DARK);
        edge(inset(face, 1), LIGHT, SHADOW);
    }
    /* the label goes down and across a pixel with the button */
    words(p->text, r.x + (r.w - wide) / 2 + held, r.y + (r.h - line) / 2 + held, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(inset(r, 4));
}

static void checkbox(const press_part *p, box r) {
    int line = on->line_height(on->ctx), size = 13;
    box b = {r.x, r.y + (r.h - size) / 2, size, size};
    sunken(b, has(p, PRESS_STATE_HELD) ? FACE : WHITE);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick, three pixels heavy */
        static const int rise[7] = {2, 3, 4, 3, 2, 1, 0};
        for (int i = 0; i < 7; i++)
            fill((box){b.x + 3 + i, b.y + 3 + rise[i], 1, 3}, BLACK);
    }
    words(p->text, r.x + size + 5, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){r.x + size + 3, r.y + (r.h - line) / 2 - 1, on->text_width(on->ctx, p->text) + 4, line + 2});
}

static void radio(const press_part *p, box r) {
    static const char *shape[12] = {
        "....ssss....",
        "..ssddddss..",
        ".sddffffllw.",
        ".sdfffffflw.",
        "sdfffffffflw",
        "sdfffffffflw",
        "sdfffffffflw",
        "sdfffffffflw",
        ".sdfffffflw.",
        ".sllffffllw.",
        "..wwllllww..",
        "....wwww....",
    };
    int line = on->line_height(on->ctx), top = r.y + (r.h - 12) / 2;
    uint32_t inside = has(p, PRESS_STATE_HELD) ? FACE : WHITE;
    for (int y = 0; y < 12; y++) {
        for (int x = 0; x < 12; x++) {
            switch (shape[y][x]) {
            case 's': px(r.x + x, top + y, SHADOW); break;
            case 'd': px(r.x + x, top + y, DARK); break;
            case 'l': px(r.x + x, top + y, LIGHT); break;
            case 'w': px(r.x + x, top + y, HILITE); break;
            case 'f': px(r.x + x, top + y, inside); break;
            default: break;
            }
        }
    }
    if (has(p, PRESS_STATE_CHECKED)) {
        fill((box){r.x + 5, top + 4, 2, 4}, BLACK);
        fill((box){r.x + 4, top + 5, 4, 2}, BLACK);
    }
    words(p->text, r.x + 12 + 6, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){r.x + 12 + 4, r.y + (r.h - line) / 2 - 1, on->text_width(on->ctx, p->text) + 4, line + 2});
}

/* What is selected is white on navy; anything else there keeps its colour. */
static void select_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++) {
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            if (c == BLACK) px(x, y, WHITE);
            else if (c == WHITE || c == FACE) px(x, y, NAVY);
        }
    }
}

/* What is out of use has its black gone grey. */
static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (at(x, y) == BLACK) px(x, y, SHADOW);
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove, knob;
    if (vertical) {
        groove = (box){r.x + r.w / 2 - 2, r.y + p->b / 2, 4, r.h - p->b + 1};
        knob = (box){r.x + 1, p->a, r.w - 2, p->b};
    } else {
        groove = (box){r.x + p->b / 2, r.y + r.h / 2 - 2, r.w - p->b + 1, 4};
        knob = (box){p->a, r.y + 1, p->b, r.h - 2};
    }
    fill(r, FACE);
    sunken(groove, BLACK);
    raised(knob);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(r);
}

static void progress(const press_part *p, box r) {
    box in = inset(r, 2);
    int block = 8, step = block + 2;
    fill(r, FACE);
    edge(r, SHADOW, HILITE);
    if (has(p, PRESS_STATE_BUSY)) {
        /* three blocks that travel along and come round again */
        int count = in.w / step + 1, lead = p->phase % (count > 0 ? count : 1);
        for (int i = 0; i < 3; i++) {
            int x = in.x + ((lead + i) % count) * step;
            int w = x + block > in.x + in.w ? in.x + in.w - x : block;
            fill((box){x, in.y, w, in.h}, NAVY);
        }
        return;
    }
    int done = (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1));
    for (int x = 0; x < done; x += step) {
        int w = x + block > done ? done - x : block;
        fill((box){in.x + x, in.y, w, in.h}, NAVY);
    }
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    /* the track is a fine check of white and grey */
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, (x + y) % 2 == 0 ? HILITE : FACE);
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        int held = live && p->a == second + 1;
        int cx = b.x + b.w / 2 + held, cy = b.y + b.h / 2 + held;
        int way = vertical ? second : 2 + second;
        if (held) {
            fill(b, FACE);
            edge(b, SHADOW, SHADOW);
        } else {
            raised(b);
        }
        if (live) {
            arrow(cx, cy, way, BLACK);
        } else {
            /* cut into the button: grey, with light catching its far edge */
            arrow(cx + 1, cy + 1, way, HILITE);
            arrow(cx, cy, way, SHADOW);
        }
    }
    if (p->b >= 0)
        raised(vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h});
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT), line = on->line_height(on->ctx);
    int top = r.y + (front ? 0 : 2), foot = r.y + r.h - 1;
    /* a tab is a button with its corners off and no bottom */
    fill((box){r.x + 1, top + 1, r.w - 2, foot - top - 1}, FACE);
    fill((box){r.x + 2, top, r.w - 4, 1}, HILITE);
    px(r.x + 1, top + 1, HILITE);
    fill((box){r.x, top + 2, 1, foot - top - 2}, HILITE);
    px(r.x + r.w - 2, top + 1, DARK);
    fill((box){r.x + r.w - 1, top + 2, 1, foot - top - 2}, DARK);
    fill((box){r.x + r.w - 2, top + 2, 1, foot - top - 2}, SHADOW);
    words(p->text, r.x + 10, r.y + (r.h - line) / 2 + (front ? 0 : 1), BLACK);
    if (front) {
        /* the chosen tab and the pane are one piece of card */
        fill((box){r.x + 1, foot, r.w - 3, 2}, FACE);
        px(r.x, foot, HILITE);
        if (has(p, PRESS_STATE_FOCUSED))
            dotted((box){r.x + 4, r.y + 3, r.w - 8, r.h - 6});
    }
}

static void popup(const press_part *p, box r) {
    int line = on->line_height(on->ctx), held = has(p, PRESS_STATE_HELD);
    box drop = {r.x + r.w - 2 - 16, r.y + 2, 16, r.h - 4};
    sunken(r, WHITE);
    words(p->text, r.x + 18, r.y + (r.h - line) / 2, BLACK);
    if (held) {
        fill(drop, FACE);
        edge(drop, SHADOW, SHADOW);
    } else {
        raised(drop);
    }
    arrow(drop.x + drop.w / 2 + held, drop.y + drop.h / 2 + held, 1, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){r.x + 3, r.y + 3, r.w - 6 - 17, r.h - 6});
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, FACE); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL: sunken(r, WHITE); break;
    case PRESS_PART_SELECT: select_in(r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: dotted(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: progress(p, r); break;
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, FACE); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, FACE);
        edge(r, HILITE, DARK);
        edge((box){r.x + 1, r.y + 1, r.w - 2, r.h - 2}, FACE, SHADOW);
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR: fill(r, FACE); break;
    case PRESS_PART_MENU_TITLE: {
        int line = on->line_height(on->ctx), lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) fill(r, NAVY);
        words(p->text, r.x + 9, r.y + (r.h - line) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : SHADOW);
        break;
    }
    case PRESS_PART_MENU_FRAME:
        /* light along the top and left, shade inside the bottom and right,
         * and the shadow's own row and column a hard black edge */
        edge((box){r.x, r.y, r.w - 1, r.h - 1}, HILITE, SHADOW);
        fill((box){r.x + r.w - 1, r.y, 1, r.h}, DARK);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, DARK);
        break;
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x + 1, r.y + r.h / 2 - 1, r.w - 2, 1}, SHADOW);
        fill((box){r.x + 1, r.y + r.h / 2, r.w - 2, 1}, HILITE);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, FACE);
        edge(r, SHADOW, HILITE);
        break;
    case PRESS_PART_HEADER: fill(r, FACE); break;
    case PRESS_PART_HEADER_CELL: raised(r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 23;
    m->button_min_width = 75;
    m->button_pad = 10;
    m->default_ring = 0; /* the black line goes inside the button, not round it */
    m->check_size = 13;
    m->check_gap = 5;
    m->field_inset_y = 4;
    m->progress_height = 16;
    m->slider_thick = 20;
}

static void palette(press_palette *p) {
    p->window = FACE;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = FACE;
    p->menu_text = BLACK;
}

static const press_style style = {PRESS_STYLE_ABI, "ninetyfive", metrics, palette, draw};

const press_style *press_style_entry(void) {
    return &style;
}
