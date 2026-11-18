/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Two Thousand and One: a style for press in the manner of the blue, green
 * and sand desktops of 2001. Rounded buttons shaded from top to bottom,
 * a glow where the pointer is, smooth type, soft shadows under menus.
 * Every pixel of it is drawn here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=twothousandone ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

#define SAND rgb(0xece9d8)      /* what windows are made of */
#define EDGE rgb(0x003c74)      /* round a button */
#define FIELD_EDGE rgb(0x7f9db9)
#define BOX_EDGE rgb(0x1c5180)  /* round a checkbox */
#define GLOW rgb(0xf8b330)      /* under the pointer */
#define GLOW_DEEP rgb(0xe59700)
#define BLUE rgb(0x316ac5)      /* what is selected */
#define SKY rgb(0xc3d5fd)       /* scroll bars and the like */
#define SKY_LIGHT rgb(0xe3ecff)
#define SKY_EDGE rgb(0x98b1e4)
#define SKY_INK rgb(0x4d6185)
#define GREEN rgb(0x21a121)
#define GREEN_LIGHT rgb(0x7fe07f)
#define GREY rgb(0xaca899)      /* out of use, and thin lines */
#define WHITE rgb(0xffffff)
#define BLACK rgb(0x000000)

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

/* A label, with a dotted box round it when the keyboard is there. */
static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){x - 2, r.y + (r.h - line) / 2 - 1, text_width(p->text) + 4, line + 2});
}

static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    if (held)
        round_box(r, 3, rgb(0xe2e1d9), rgb(0xf0efe9), EDGE);
    else
        round_box(r, 3, WHITE, rgb(0xd9d4c7), EDGE);
    /* a glow just inside the edge: gold under the pointer, blue on the
     * button Return presses or the keyboard is on */
    if (!held && has(p, PRESS_STATE_HOT)) {
        round_line(inset(r, 1), 2, 2, GLOW);
        fill_over((box){r.x + 3, r.y + r.h - 3, r.w - 6, 2}, with_alpha(GLOW_DEEP, 0.7f));
    } else if (!held && (has(p, PRESS_STATE_DEFAULT) || has(p, PRESS_STATE_FOCUSED))) {
        round_line(inset(r, 1), 2, 2, rgb(0x9dbdf4));
        fill_over((box){r.x + 3, r.y + r.h - 3, r.w - 6, 2}, with_alpha(rgb(0x6f93d8), 0.8f));
    }
    int line = line_height(), wide = text_width(p->text);
    words(p->text, r.x + (r.w - wide) / 2 + held, r.y + (r.h - line) / 2 + held, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(inset(r, 3));
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    gradient(b, has(p, PRESS_STATE_HELD) ? rgb(0xb0b0a7) : rgb(0xdcdcd7), WHITE);
    outline(b, BOX_EDGE);
    if (has(p, PRESS_STATE_HOT))
        round_line(inset(b, 1), 0, 2, GLOW);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick, three pixels heavy */
        static const int rise[7] = {2, 3, 4, 3, 2, 1, 0};
        for (int i = 0; i < 7; i++)
            fill((box){b.x + 3 + i, b.y + 3 + rise[i], 1, 3}, GREEN);
    }
    label(p, r.x + 13 + 5, r);
}

static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    round_box(b, 6.5f, has(p, PRESS_STATE_HELD) ? rgb(0xb0b0a7) : rgb(0xdcdcd7), WHITE, BOX_EDGE);
    if (has(p, PRESS_STATE_HOT))
        round_line(inset(b, 1), 5.5f, 2, GLOW);
    if (has(p, PRESS_STATE_CHECKED))
        round_fill((box){b.x + 4, b.y + 4, 5, 5}, 2.5f, GREEN_LIGHT, GREEN);
    label(p, r.x + 13 + 5, r);
}

/* What is out of use has its dark gone pale; what was light is left. */
static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, mix(c, GREY, 0.75f * (1 - light(c))));
        }
}

/* A raised piece of a scroll bar: an arrow box or the sliding box. */
static void sky_box(box r, int held, int live) {
    if (!live)
        round_box(r, 2, rgb(0xf6f5f0), rgb(0xe6e4dc), rgb(0xd0cec4));
    else if (held)
        round_box(r, 2, rgb(0x9fb9f2), rgb(0xb7cdfb), SKY_EDGE);
    else
        round_box(r, 2, SKY_LIGHT, SKY, SKY_EDGE);
    if (live && !held)
        round_line(inset(r, 1), 1, 1, with_alpha(WHITE, 0.8f));
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, rgb(0xf6f5f1));
    if (vertical) fill((box){r.x, r.y, 1, r.h}, rgb(0xe8e6df));
    else fill((box){r.x, r.y, r.w, 1}, rgb(0xe8e6df));
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        int held = live && p->a == second + 1;
        sky_box(b, held, live);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? SKY_INK : GREY);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h};
        sky_box(t, 0, 1);
        /* ridges to take hold of, if there is room */
        if (p->c >= 16) {
            for (int i = -3; i <= 3; i += 2) {
                box ridge = vertical ? (box){t.x + 5, t.y + t.h / 2 + i, t.w - 10, 1} : (box){t.x + t.w / 2 + i, t.y + 5, 1, t.h - 10};
                fill(ridge, rgb(0x8cb0f8));
                if (vertical) fill((box){ridge.x + 1, ridge.y + 1, ridge.w, 1}, with_alpha(WHITE, 1));
                else fill((box){ridge.x + 1, ridge.y + 1, 1, ridge.h}, with_alpha(WHITE, 1));
            }
        }
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove, knob;
    if (vertical) {
        groove = (box){r.x + r.w / 2 - 2, r.y + p->b / 2, 4, r.h - p->b + 1};
        knob = (box){r.x + 2, p->a, r.w - 4, p->b};
    } else {
        groove = (box){r.x + p->b / 2, r.y + r.h / 2 - 2, r.w - p->b + 1, 4};
        knob = (box){p->a, r.y + 2, p->b, r.h - 4};
    }
    round_box(groove, 1.5f, rgb(0xe7eaea), WHITE, rgb(0x9d9c99));
    round_box(knob, 2.5f, WHITE, rgb(0xd5d5c9), rgb(0x778892));
    /* a line of colour at each end of the knob: gold under the pointer */
    uint32_t tip = has(p, PRESS_STATE_HOT) || has(p, PRESS_STATE_HELD) ? GLOW : rgb(0x3cb43c);
    if (vertical) {
        fill_over((box){knob.x + 1, knob.y + 1, 2, knob.h - 2}, tip);
        fill_over((box){knob.x + knob.w - 3, knob.y + 1, 2, knob.h - 2}, tip);
    } else {
        fill_over((box){knob.x + 1, knob.y + 1, knob.w - 2, 2}, tip);
        fill_over((box){knob.x + 1, knob.y + knob.h - 3, knob.w - 2, 2}, tip);
    }
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(r);
}

static void progress(const press_part *p, box r) {
    box in = inset(r, 3);
    int block = 6, step = block + 2;
    round_box(r, 3, rgb(0xf2f2f2), WHITE, rgb(0x686868));
    int from = 0, to;
    if (has(p, PRESS_STATE_BUSY)) {
        /* a few blocks that travel along and come round again */
        int count = in.w / step + 1;
        from = (p->phase * 2 % (count > 0 ? count : 1)) * step;
        to = from + 4 * step;
    } else {
        to = (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1));
    }
    for (int x = from; x < to && x < in.w; x += step) {
        int w = x + block > in.w ? in.w - x : block;
        box b = {in.x + x, in.y, w, in.h};
        gradient((box){b.x, b.y, b.w, b.h / 2}, rgb(0xc4f2c0), rgb(0x2ed13a));
        gradient((box){b.x, b.y + b.h / 2, b.w, b.h - b.h / 2}, rgb(0x2ed13a), rgb(0x9ce89a));
    }
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT), line = line_height();
    int top = r.y + (front ? 0 : 2), foot = r.y + r.h - 1;
    box shape = {r.x, top, r.w, foot - top + 4};
    if (front) {
        round_box(shape, 3, WHITE, rgb(0xfcfcfe), rgb(0x919b9c));
        /* a gold line along the top of the chosen tab */
        fill((box){r.x + 2, top, r.w - 4, 1}, rgb(0xe68b2c));
        fill((box){r.x + 1, top + 1, r.w - 2, 1}, rgb(0xffc73c));
        /* and nothing between it and the pane */
        fill((box){r.x + 1, foot, r.w - 2, 4}, rgb(0xfcfcfe));
    } else {
        round_box(shape, 3, WHITE, rgb(0xeae8dc), rgb(0x919b9c));
    }
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line) / 2 + (front ? 0 : 1), BLACK);
    if (front && has(p, PRESS_STATE_FOCUSED))
        dotted((box){r.x + 3, r.y + 3, r.w - 6, r.h - 6});
}

static void popup(const press_part *p, box r) {
    int line = line_height(), held = has(p, PRESS_STATE_HELD);
    box drop = {r.x + r.w - 1 - (r.h - 2), r.y + 1, r.h - 2, r.h - 2};
    fill(r, WHITE);
    outline(r, FIELD_EDGE);
    if (has(p, PRESS_STATE_FOCUSED))
        fill((box){r.x + 2, r.y + 2, drop.x - r.x - 3, r.h - 4}, rgb(0xdde6f7));
    words(p->text, r.x + 5, r.y + (r.h - line) / 2, BLACK);
    sky_box(drop, held, 1);
    if (has(p, PRESS_STATE_HOT) && !held)
        round_line(drop, 2, 1, rgb(0x6a8fd8));
    arrow(drop.x + drop.w / 2, drop.y + drop.h / 2, 4, 1, SKY_INK);
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, SAND); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        fill(r, WHITE);
        outline(r, FIELD_EDGE);
        break;
    case PRESS_PART_SELECT: recolour(r, BLUE, WHITE); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: dotted(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: progress(p, r); break;
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, SAND); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, rgb(0xfcfcfe));
        outline(r, rgb(0x919b9c));
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR: fill(r, SAND); break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) fill(r, BLUE);
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : GREY);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        box frame = {r.x, r.y, r.w - p->a, r.h - p->a};
        shadow((box){frame.x + 2, frame.y + 2, frame.w - 2, frame.h - 2}, 0, (float)p->a, BLACK, 0.45f);
        outline(frame, GREY);
        break;
    }
    case PRESS_PART_MENU_SEPARATOR: fill((box){r.x + 2, r.y + r.h / 2, r.w - 4, 1}, GREY); break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, SAND);
        outline(r, FIELD_EDGE);
        break;
    case PRESS_PART_HEADER:
        fill(r, rgb(0xebeadb));
        fill((box){r.x, r.y + r.h - 3, r.w, 1}, rgb(0xe2decd));
        fill((box){r.x, r.y + r.h - 2, r.w, 1}, rgb(0xd6d2c2));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, rgb(0xcbc7b8));
        break;
    case PRESS_PART_HEADER_CELL:
        fill((box){r.x + r.w - 2, r.y + 3, 1, r.h - 7}, rgb(0xc7c5b2));
        fill((box){r.x + r.w - 1, r.y + 3, 1, r.h - 7}, WHITE);
        break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 23;
    m->button_min_width = 75;
    m->button_pad = 12;
    m->default_ring = 0;
    m->check_size = 13;
    m->check_gap = 5;
    m->radio_size = 13;
    m->radio_gap = 5;
    m->field_inset_x = 5;
    m->field_inset_y = 4;
    m->progress_height = 16;
    m->slider_thick = 21;
    m->scroll_bar = 17;
    m->slider_knob = 11;
    m->tab_height = 21;
    m->popup_height = 21;
    m->row_height = 17;
    m->menu_bar_height = 20;
    m->menu_item_height = 19;
    m->menu_shadow = 4;
}

static void palette(press_palette *p) {
    p->window = SAND;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = WHITE;
    p->menu_text = BLACK;
}

/* The faces of that desktop where the machine has them, and the nearest
 * thing to them where it has not. */
static int32_t typeface(uint32_t face, press_type *out) {
    const char *sans = "Tahoma,Verdana,DejaVu Sans,Noto Sans";
    const char *fixed = "Lucida Console,DejaVu Sans Mono,Liberation Mono";
    switch (face) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 11; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 10; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 10; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 11; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 11; break;
    case PRESS_FACE_TITLE: strcpy(out->family, fixed); out->size = 14; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

static const press_style style = {PRESS_STYLE_ABI, "twothousandone", metrics, palette, draw, PRESS_WANTS_HOVER, typeface};

const press_style *press_style_entry(void) {
    return &style;
}
