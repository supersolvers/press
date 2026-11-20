/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Eighty-Eight: a style for press in the manner of the open workstation
 * look of 1988, which went its own way in nearly everything. Buttons are
 * lozenges, round at both ends; a button that opens a menu carries a
 * small triangle; a choice among several is a row of boxes, the chosen
 * one in a heavy line; a line of text to type on is a line and no box; a
 * scroll bar is a lift on a cable, with its two arrows riding on the car;
 * and a window is marked at its four corners with brackets to take hold
 * of. Every pixel of it is drawn here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=eightyeight ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

COLOUR(GROUND, 0xcccccc); /* what windows are made of */
COLOUR(LIT, 0xffffff);
FOLLOWER(PRESSED, 0xb0b0b0);
#define PRESSED follows(PRESSED, GROUND)
FOLLOWER(SHADE, 0x7f7f7f);
#define SHADE follows(SHADE, GROUND)
COLOUR(WHITE, 0xffffff);
COLOUR(BLACK, 0x000000);

/* A lozenge standing out of the window: light along its top, shade along
 * its foot. Pressed, it is the other way about and darker. */
static void lozenge(box r, int held) {
    float radius = r.h / 2.0f;
    round_fill(r, radius, held ? SHADE : LIT, held ? LIT : SHADE);
    round_fill(inset(r, 1), radius - 1, held ? PRESSED : GROUND, held ? PRESSED : GROUND);
}

static void edge(box r, uint32_t top_left, uint32_t bottom_right) {
    if (r.w <= 1 || r.h <= 1) return;
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, bottom_right);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, bottom_right);
    fill((box){r.x, r.y, r.w - 1, 1}, top_left);
    fill((box){r.x, r.y, 1, r.h - 1}, top_left);
}

/* A small triangle, outlined: here is a menu. */
static void menu_mark(int cx, int cy) {
    arrow(cx, cy, 5, 1, SHADE);
    arrow(cx, cy - 1, 4, 1, GROUND);
    fill((box){cx - 4, cy - 2, 9, 1}, SHADE);
}

static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    lozenge(r, held);
    if (has(p, PRESS_STATE_DEFAULT))
        /* a ring within the one Return presses */
        round_line(inset(r, 2), r.h / 2.0f - 2, 1, BLACK);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED)) {
        /* the keyboard is shown by a small mark under the middle of it */
        arrow(r.x + r.w / 2, r.y + r.h + 2, 4, 0, BLACK);
    }
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    int line = line_height(), held = has(p, PRESS_STATE_HELD);
    fill(b, held ? PRESSED : GROUND);
    edge(b, held ? SHADE : LIT, held ? LIT : SHADE);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick that runs well out of the top of its box */
        static const int rise[11] = {6, 7, 8, 7, 6, 5, 4, 3, 2, 1, 0};
        for (int i = 0; i < 11; i++)
            fill((box){b.x + 2 + i, b.y - 2 + rise[i], 1, 3}, BLACK);
    }
    words(p->text, r.x + 13 + 7, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED)) arrow(b.x + 6, b.y + 17, 4, 0, BLACK);
}

/* One of several: its words in a box, the chosen one's box heavy. */
static void radio(const press_part *p, box r) {
    int line = line_height(), held = has(p, PRESS_STATE_HELD);
    box b = {r.x, r.y - 2, r.w + 8, r.h + 4};
    fill(b, held ? PRESSED : GROUND);
    if (has(p, PRESS_STATE_CHECKED)) {
        outline(b, BLACK);
        outline(inset(b, 1), BLACK);
    } else {
        edge(b, LIT, SHADE);
    }
    words(p->text, r.x + 8, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED)) arrow(b.x + b.w / 2, b.y + b.h + 2, 4, 0, BLACK);
}

static void well(const press_part *p, box r) {
    fill(r, GROUND);
    if (p->a == 0) {
        /* a line to type on, and no more */
        fill((box){r.x + 2, r.y + r.h - 3, r.w - 4, 1}, SHADE);
        fill((box){r.x + 2, r.y + r.h - 2, r.w - 4, 1}, LIT);
        if (has(p, PRESS_STATE_FOCUSED)) fill((box){r.x + 2, r.y + r.h - 3, r.w - 4, 1}, BLACK);
        return;
    }
    edge(r, SHADE, LIT);
    if (has(p, PRESS_STATE_FOCUSED)) outline(inset(r, 1), BLACK);
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, mix(c, SHADE, 0.7f * (1 - light(c))));
        }
}

/* A lift on a cable: the cable runs from one anchor to the other, and
 * the car rides on it. The arrows stand at the ends of the shaft here,
 * each in a box of its own, since that is where this engine has them. */
static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, GROUND);
    /* the cable */
    box cable = vertical ? (box){r.x + thick / 2 - 1, r.y + thick, 3, r.h - 2 * thick} : (box){r.x + thick, r.y + thick / 2 - 1, r.w - 2 * thick, 3};
    for (int y = cable.y; y < cable.y + cable.h; y++)
        for (int x = cable.x; x < cable.x + cable.w; x++)
            px(x, y, (x + y) % 2 == 0 ? SHADE : GROUND);
    for (int second = 0; second < 2; second++) {
        box b = inset(vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick}, 1);
        int held = live && p->a == second + 1;
        fill(b, held ? PRESSED : GROUND);
        edge(b, held ? SHADE : LIT, held ? LIT : SHADE);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? BLACK : SHADE);
    }
    if (p->b >= 0) {
        /* the car: a box with a dimple in its middle to drag it by */
        box car = inset(vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h}, 1);
        fill(car, GROUND);
        edge(car, LIT, SHADE);
        outline((box){car.x - 1, car.y - 1, car.w + 2, car.h + 2}, with_alpha(BLACK, 0.35f));
        box dimple = {car.x + car.w / 2 - 3, car.y + car.h / 2 - 3, 6, 6};
        if (car.w >= 10 && car.h >= 10) {
            round_fill(dimple, 3, SHADE, LIT);
            round_fill(inset(dimple, 1), 2, PRESSED, PRESSED);
        }
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box bar = vertical ? (box){r.x + r.w / 2 - 3, r.y + 1, 6, r.h - 2} : (box){r.x + 1, r.y + r.h / 2 - 3, r.w - 2, 6};
    box knob = vertical ? (box){r.x + 1, p->a, r.w - 2, p->b} : (box){p->a, r.y + 1, p->b, r.h - 2};
    /* a round-ended bar, dark for as far as has been gone */
    round_fill(bar, 3, SHADE, LIT);
    round_fill(inset(bar, 1), 2, GROUND, GROUND);
    if (vertical) round_fill((box){bar.x + 1, knob.y, bar.w - 2, bar.y + bar.h - 1 - knob.y}, 2, BLACK, BLACK);
    else round_fill((box){bar.x + 1, bar.y + 1, knob.x - bar.x + 2, bar.h - 2}, 2, BLACK, BLACK);
    round_fill(knob, 4, LIT, SHADE);
    round_fill(inset(knob, 1), 3, has(p, PRESS_STATE_HELD) ? PRESSED : GROUND, has(p, PRESS_STATE_HELD) ? PRESSED : GROUND);
    if (has(p, PRESS_STATE_FOCUSED)) arrow(knob.x + knob.w / 2, r.y + r.h + 2, 4, 0, BLACK);
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    box b = {r.x, r.y + (front ? 0 : 2), r.w, r.h - (front ? 0 : 2) + 1};
    fill(b, front ? GROUND : PRESSED);
    edge(b, LIT, SHADE);
    if (front) fill((box){r.x + 1, r.y + r.h - 1, r.w - 2, 2}, GROUND);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), BLACK);
    if (front && has(p, PRESS_STATE_FOCUSED)) arrow(r.x + r.w / 2, r.y + r.h - 4, 3, 0, BLACK);
}

/* A window's frame: a thin line round it, a band across the top with the
 * title in its middle, and at each of the four corners a bracket, to take
 * hold of and make the window larger or smaller by. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height(), reach = 12;
    fill(r, GROUND);
    outline(r, BLACK);
    edge(inset(r, 1), LIT, SHADE);
    /* the window that has the keyboard has its band a shade darker and ruled off */
    box band = {r.x + 2, r.y + 2, r.w - 4, p->a - 4};
    if (active) {
        fill(band, PRESSED);
        fill((box){band.x, band.y, band.w, 1}, SHADE);
        fill((box){band.x, band.y + band.h - 1, band.w, 1}, LIT);
    }
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (p->a - line) / 2, active ? BLACK : SHADE);
    /* the brackets */
    for (int corner = 0; corner < 4; corner++) {
        int right = corner & 1, foot = corner & 2;
        int x = right ? r.x + r.w - reach : r.x, y = foot ? r.y + r.h - reach : r.y;
        fill((box){x, foot ? r.y + r.h - 3 : r.y, reach, 3}, GROUND);
        fill((box){right ? r.x + r.w - 3 : r.x, y, 3, reach}, GROUND);
        outline((box){x, foot ? r.y + r.h - 3 : r.y, reach, 3}, BLACK);
        outline((box){right ? r.x + r.w - 3 : r.x, y, 3, reach}, BLACK);
        px(right ? r.x + r.w - 2 : r.x + 1, foot ? r.y + r.h - 2 : r.y + 1, LIT);
    }
}

/* The one button of the band: at its left, a box with the mark of a menu,
 * which here closes the window. */
static void frame_button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    fill(r, held ? PRESSED : GROUND);
    edge(r, held ? SHADE : LIT, held ? LIT : SHADE);
    if (p->a == PRESS_FRAME_MENU) {
        menu_mark(r.x + r.w / 2, r.y + r.h / 2);
    } else if (p->a == PRESS_FRAME_ZOOM) {
        outline(inset(r, 3), BLACK);
    } else {
        fill((box){r.x + 3, r.y + r.h - 5, r.w - 6, 2}, BLACK);
    }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, GROUND); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL: well(p, r); break;
    case PRESS_PART_SELECT: recolour(r, like(GROUND, 0x555555), WHITE); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: outline(r, BLACK); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 2);
        round_fill(r, r.h / 2.0f, SHADE, LIT);
        round_fill(inset(r, 1), r.h / 2.0f - 1, GROUND, GROUND);
        box done = in;
        if (has(p, PRESS_STATE_BUSY)) {
            int span = in.w - in.w / 4, at_ = (p->phase * 6) % (2 * (span > 0 ? span : 1));
            done.x = in.x + (at_ < span ? at_ : 2 * span - at_);
            done.w = in.w / 4;
        } else {
            done.w = (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1));
        }
        if (done.w >= done.h) round_fill(done, done.h / 2.0f, BLACK, BLACK);
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, GROUND); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, GROUND);
        edge(r, LIT, SHADE);
        break;
    case PRESS_PART_POPUP: {
        /* a button that opens a menu: a lozenge with the mark, and its choice beside */
        int held = has(p, PRESS_STATE_HELD);
        box mark = {r.x, r.y, r.h + 6, r.h};
        fill(r, GROUND);
        lozenge(mark, held);
        menu_mark(mark.x + mark.w / 2, mark.y + mark.h / 2 + 1);
        words(p->text, mark.x + mark.w + 6, r.y + (r.h - line_height()) / 2, BLACK);
        if (has(p, PRESS_STATE_FOCUSED)) arrow(mark.x + mark.w / 2, r.y + r.h + 2, 4, 0, BLACK);
        break;
    }
    case PRESS_PART_MENU_BAR:
        fill(r, GROUND);
        fill((box){r.x, r.y + r.h - 2, r.w, 1}, SHADE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, LIT);
        break;
    case PRESS_PART_MENU_TITLE: {
        /* each title is a button that opens a menu, and looks it */
        int held = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        box b = {r.x + 2, r.y + 2, r.w - 4, r.h - 5};
        lozenge(b, held);
        words(p->text, r.x + 9, r.y + (r.h - 1 - line_height()) / 2, has(p, PRESS_STATE_ENABLED) ? BLACK : SHADE);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(f, BLACK);
        fill((box){f.x + f.w, f.y + p->a, p->a, f.h}, with_alpha(BLACK, 0.4f));
        fill((box){f.x + p->a, f.y + f.h, f.w - p->a, p->a}, with_alpha(BLACK, 0.4f));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR: break; /* a gap and nothing in it */
    case PRESS_PART_VIEW_FRAME:
        fill(r, GROUND);
        edge(r, SHADE, LIT);
        break;
    case PRESS_PART_HEADER:
        fill(r, GROUND);
        fill((box){r.x, r.y + r.h - 2, r.w, 1}, SHADE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, LIT);
        break;
    case PRESS_PART_HEADER_CELL: break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 22;
    m->button_min_width = 60;
    m->button_pad = 14;
    m->default_ring = 0;
    m->check_size = 13;
    m->check_gap = 7;
    m->radio_size = 6; /* the room before its words, in their box */
    m->radio_gap = 2;
    m->field_inset_x = 4;
    m->field_inset_y = 5;
    m->progress_height = 12;
    m->slider_thick = 18;
    m->scroll_bar = 17;
    m->slider_knob = 11;
    m->tab_height = 22;
    m->popup_height = 20;
    m->row_height = 17;
    m->menu_bar_height = 24;
    m->menu_item_height = 18;
    m->menu_shadow = 3;
    /* a frame of its own, with its menus in it */
    m->frame_title = 24;
    m->ok_leads = 1; /* OK, then Cancel */
    m->frame_border = 3;
    m->frame_button_width = 16;
    m->frame_button_height = 14;
    m->frame_button_gap = 3;
    m->frame_button_inset = 7;
    m->frame_button_rise = 0;
    m->frame_left = 0x5; /* the window's own menu; two clicks on it close the window */
    m->frame_right = 0;
    m->menu_in_window = 1;
}

static void palette(press_palette *p) {
    /* what is typed and listed is on the window's own grey */
    p->window = GROUND;
    p->text = BLACK;
    p->paper = GROUND;
    p->ink = BLACK;
    p->menu = GROUND;
    p->menu_text = BLACK;
}

static int32_t typeface(uint32_t face, press_type *out) {
    const char *sans = "Lucida Sans,Lucida Grande,Luxi Sans,DejaVu Sans";
    const char *fixed = "Lucida Sans Typewriter,Luxi Mono,DejaVu Sans Mono";
    switch (face) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 12; out->bold = 1; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 10; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 10; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 12; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 12; break;
    case PRESS_FACE_TITLE: strcpy(out->family, fixed); out->size = 15; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"ground", &GROUND},
    {"lit", &LIT},
    {"pressed", &PRESSED_SAID},
    {"shade", &SHADE_SAID},
    {"white", &WHITE},
    {"black", &BLACK},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "eightyeight", metrics, palette, draw, 0, typeface, colours};

const press_style *press_style_entry(void) {
    return &style;
}
