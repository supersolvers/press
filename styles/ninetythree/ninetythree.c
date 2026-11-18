/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Ninety-Three: a style for press in the manner of the common desktop of
 * the workstations of 1993. Everything is heavy and deep: each thing
 * stands two pixels out of a ground of slate blue or is sunk two pixels
 * into it, with a pale edge where the light falls and a dark one where it
 * does not; what is switched on is pressed in; a choice among several is
 * a diamond; where the keyboard is, a black line goes right round; and
 * the window in front has its whole frame in rose. Every pixel of it is
 * drawn here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=ninetythree ./examples/c/controls
 */
#include "smooth.h"

#define GROUND rgb(0xaeb2c3)  /* what windows are made of */
#define LIT rgb(0xdcdee5)
#define SHADE rgb(0x5d6069)
#define TROUGH rgb(0x9397a5)  /* what is pressed in, and the bed of a scroll bar */
#define PAPER rgb(0xfff7e9)
#define ROSE rgb(0xb24d7a)    /* the frame of the window in front */
#define ROSE_LIT rgb(0xe0b0c6)
#define ROSE_SHADE rgb(0x5e2840)
#define BLACK rgb(0x000000)
#define WHITE rgb(0xffffff)

/* Two pixels of edge: one colour along the top and left, another along
 * the bottom and right. */
static void bevel(box r, uint32_t top_left, uint32_t bottom_right) {
    for (int d = 0; d < 2; d++) {
        box b = inset(r, d);
        if (b.w <= 1 || b.h <= 1) return;
        fill((box){b.x, b.y + b.h - 1, b.w, 1}, bottom_right);
        fill((box){b.x + b.w - 1, b.y, 1, b.h}, bottom_right);
        fill((box){b.x, b.y, b.w - 1 - d, 1}, top_left);
        fill((box){b.x, b.y, 1, b.h - 1 - d}, top_left);
    }
}

static void raised(box r, uint32_t face) {
    fill(r, face);
    bevel(r, LIT, SHADE);
}

static void sunk(box r, uint32_t inside) {
    fill(r, inside);
    bevel(r, SHADE, LIT);
}

/* Where the keyboard is: a black line right round, a little way off. */
static void around(box r) {
    outline((box){r.x - 3, r.y - 3, r.w + 6, r.h + 6}, BLACK);
}

static void button(const press_part *p, box r) {
    box face = r;
    if (has(p, PRESS_STATE_DEFAULT)) {
        /* the one Return presses stands in a hollow of its own */
        sunk(r, GROUND);
        face = inset(r, 5);
    }
    if (has(p, PRESS_STATE_HELD)) sunk(face, TROUGH);
    else raised(face, GROUND);
    words(p->text, face.x + (face.w - text_width(p->text)) / 2, face.y + (face.h - line_height()) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED)) outline(inset(face, 3), BLACK);
}

static void label(const press_part *p, int x, box r, box mark) {
    words(p->text, x, r.y + (r.h - line_height()) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        outline((box){mark.x - 3, r.y - 2, x + text_width(p->text) + 3 - (mark.x - 3), r.h + 4}, BLACK);
}

static void checkbox(const press_part *p, box r) {
    /* a square that is pressed in while it is on */
    box b = {r.x, r.y + (r.h - 12) / 2, 12, 12};
    if (has(p, PRESS_STATE_CHECKED) || has(p, PRESS_STATE_HELD)) sunk(b, TROUGH);
    else raised(b, GROUND);
    label(p, r.x + 12 + 7, r, b);
}

static void radio(const press_part *p, box r) {
    /* a diamond, likewise */
    int on_ = has(p, PRESS_STATE_CHECKED) || has(p, PRESS_STATE_HELD), top = r.y + (r.h - 13) / 2;
    uint32_t upper = on_ ? SHADE : LIT, lower = on_ ? LIT : SHADE;
    for (int i = 0; i < 7; i++) {
        fill((box){r.x + 6 - i, top + i, 2 * i + 1, 1}, on_ ? TROUGH : GROUND);
        fill((box){r.x + 6 - i, top + 12 - i, 2 * i + 1, 1}, on_ ? TROUGH : GROUND);
        fill((box){r.x + 6 - i, top + i, 2, 1}, upper);
        fill((box){r.x + 5 + i, top + i, 2, 1}, upper);
        fill((box){r.x + 6 - i, top + 12 - i, 2, 1}, lower);
        fill((box){r.x + 5 + i, top + 12 - i, 2, 1}, lower);
    }
    label(p, r.x + 13 + 7, r, (box){r.x, top, 13, 13});
}

/* What is selected. On paper, ink and paper change places. In a menu,
 * which is of the ground, the item stands out as a button does. */
static void select_in(const press_part *p, box r) {
    if (p->a == 2) {
        bevel(r, LIT, SHADE);
        return;
    }
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            if (c == PAPER) px(x, y, BLACK);
            else if (c == BLACK) px(x, y, PAPER);
        }
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (at(x, y) == BLACK && (x + y) % 2 == 0) px(x, y, at(r.x, r.y) == BLACK ? GROUND : at(r.x, r.y));
}

/* An arrow that is itself a little hill or hollow: lit on the sides the
 * light reaches. */
static void hill(box b, int way, int held) {
    int cx = b.x + b.w / 2, cy = b.y + b.h / 2, size = (b.w < b.h ? b.w : b.h) / 2 - 1;
    arrow(cx, cy, size + 1, way, held ? LIT : SHADE);
    arrow(way >= 2 ? cx : cx - 1, way >= 2 ? cy - 1 : cy, size, way, held ? SHADE : LIT);
    arrow(cx, cy, size - 1, way, held ? TROUGH : GROUND);
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    sunk(r, TROUGH);
    for (int second = 0; second < 2; second++) {
        box b = inset(vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick}, 2);
        hill(b, vertical ? second : 2 + second, live && p->a == second + 1);
    }
    if (p->b >= 0)
        raised(inset(vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h}, 2), GROUND);
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box knob = inset(vertical ? (box){r.x, p->a, r.w, p->b} : (box){p->a, r.y, p->b, r.h}, 2);
    sunk(r, TROUGH);
    raised(knob, GROUND);
    /* a groove across the middle of the knob */
    if (vertical) {
        fill((box){knob.x + 2, knob.y + knob.h / 2 - 1, knob.w - 4, 1}, SHADE);
        fill((box){knob.x + 2, knob.y + knob.h / 2, knob.w - 4, 1}, LIT);
    } else {
        fill((box){knob.x + knob.w / 2 - 1, knob.y + 2, 1, knob.h - 4}, SHADE);
        fill((box){knob.x + knob.w / 2, knob.y + 2, 1, knob.h - 4}, LIT);
    }
    if (has(p, PRESS_STATE_FOCUSED)) around(r);
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    box b = {r.x, r.y + (front ? 0 : 3), r.w, r.h - (front ? 0 : 3) + 2};
    raised(b, front ? GROUND : TROUGH);
    if (front) fill((box){r.x + 2, r.y + r.h - 2, r.w - 4, 4}, GROUND);
    words(p->text, r.x + 10, r.y + (r.h - line_height()) / 2 + (front ? 0 : 2), BLACK);
    if (front && has(p, PRESS_STATE_FOCUSED)) outline((box){r.x + 4, r.y + 4, r.w - 8, r.h - 8}, BLACK);
}

/* A window's frame: a heavy band all round, notched near each corner, in
 * rose for the window in front and in the ground's own colour behind; and
 * across the top, each standing out of the band on its own, the button of
 * the window's menu, the title, and the buttons that make it small and large. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height(), b = p->b;
    uint32_t face = active ? ROSE : GROUND, lit = active ? ROSE_LIT : LIT, shade = active ? ROSE_SHADE : SHADE;
    fill(r, face);
    bevel(r, lit, shade);
    bevel(inset(r, b - 2), shade, lit);
    /* the notches: either side of them is a corner, which stretches two ways */
    int corner = p->a;
    for (int side = 0; side < 2; side++) {
        int x = side ? r.x + r.w - 1 - corner : r.x + corner;
        int y = side ? r.y + r.h - 1 - corner : r.y + corner;
        for (int end = 0; end < 2; end++) {
            fill((box){x, end ? r.y + r.h - b + 1 : r.y + 1, 1, b - 2}, shade);
            fill((box){x + 1, end ? r.y + r.h - b + 1 : r.y + 1, 1, b - 2}, lit);
            fill((box){end ? r.x + r.w - b + 1 : r.x + 1, y, b - 2, 1}, shade);
            fill((box){end ? r.x + r.w - b + 1 : r.x + 1, y + 1, b - 2, 1}, lit);
        }
    }
    /* the title, standing out of the band between the buttons */
    box title = {r.x + b + 19, r.y + b, r.w - 2 * b - 19 - 38, p->a - b - 1};
    fill(title, face);
    bevel(title, lit, shade);
    words(p->text, title.x + (title.w - text_width(p->text)) / 2, title.y + (title.h - line) / 2, active ? WHITE : BLACK);
}

static void frame_button(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), held = has(p, PRESS_STATE_HELD);
    uint32_t face = active ? ROSE : GROUND, lit = active ? ROSE_LIT : LIT, shade = active ? ROSE_SHADE : SHADE;
    fill(r, face);
    bevel(r, held ? shade : lit, held ? lit : shade);
    box mark;
    if (p->a == PRESS_FRAME_MENU) mark = (box){r.x + 4, r.y + r.h / 2 - 2, r.w - 8, 4};          /* a bar: the window's menu */
    else if (p->a == PRESS_FRAME_MINIMIZE) mark = (box){r.x + r.w / 2 - 2, r.y + r.h / 2 - 2, 4, 4}; /* a small square */
    else mark = inset(r, 4);                                                                       /* a large one */
    fill(mark, face);
    int in = p->a == PRESS_FRAME_ZOOM && has(p, PRESS_STATE_CHECKED);
    fill((box){mark.x, mark.y + mark.h - 1, mark.w, 1}, in ? lit : shade);
    fill((box){mark.x + mark.w - 1, mark.y, 1, mark.h}, in ? lit : shade);
    fill((box){mark.x, mark.y, mark.w - 1, 1}, in ? shade : lit);
    fill((box){mark.x, mark.y, 1, mark.h - 1}, in ? shade : lit);
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, GROUND); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        sunk(r, PAPER);
        if (has(p, PRESS_STATE_FOCUSED)) outline(r, BLACK);
        break;
    case PRESS_PART_SELECT: select_in(p, r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: outline(r, BLACK); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 2);
        sunk(r, TROUGH);
        if (has(p, PRESS_STATE_BUSY)) {
            for (int y = in.y; y < in.y + in.h; y++)
                for (int x = in.x; x < in.x + in.w; x++)
                    if (((x + y - p->phase) % 12 + 12) % 12 < 6) px(x, y, ROSE);
        } else {
            box done = {in.x, in.y, (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1)), in.h};
            if (done.w > 3) {
                fill(done, ROSE);
                bevel(done, ROSE_LIT, ROSE_SHADE);
            }
        }
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, GROUND); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE: raised(r, GROUND); break;
    case PRESS_PART_POPUP: {
        /* a button with its choice, and a little bar standing out at its end */
        if (has(p, PRESS_STATE_HELD)) sunk(r, TROUGH);
        else raised(r, GROUND);
        words(p->text, r.x + 8, r.y + (r.h - line_height()) / 2, BLACK);
        raised((box){r.x + r.w - 20, r.y + r.h / 2 - 4, 12, 8}, GROUND);
        if (has(p, PRESS_STATE_FOCUSED)) outline(inset(r, 3), BLACK);
        break;
    }
    case PRESS_PART_MENU_BAR: raised(r, GROUND); break;
    case PRESS_PART_MENU_TITLE:
        if (has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED))
            bevel((box){r.x + 2, r.y + 2, r.w - 4, r.h - 4}, LIT, SHADE);
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, BLACK);
        if (!has(p, PRESS_STATE_ENABLED)) dim_in(r);
        break;
    case PRESS_PART_MENU_FRAME:
        /* a menu stands two pixels out of whatever it is over, as everything does */
        bevel(r, LIT, SHADE);
        break;
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x, r.y + r.h / 2 - 1, r.w, 1}, SHADE);
        fill((box){r.x, r.y + r.h / 2, r.w, 1}, LIT);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, GROUND);
        fill((box){r.x, r.y, r.w, 1}, SHADE);
        fill((box){r.x, r.y, 1, r.h}, SHADE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, LIT);
        fill((box){r.x + r.w - 1, r.y, 1, r.h}, LIT);
        break;
    case PRESS_PART_HEADER: fill(r, GROUND); break;
    case PRESS_PART_HEADER_CELL: raised(r, GROUND); break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 24;
    m->button_min_width = 64;
    m->button_pad = 12;
    m->default_ring = 5;
    m->check_size = 12;
    m->check_gap = 7;
    m->radio_size = 13;
    m->radio_gap = 7;
    m->field_inset_x = 6;
    m->field_inset_y = 5;
    m->progress_height = 16;
    m->slider_thick = 20;
    m->scroll_bar = 18;
    m->slider_knob = 15;
    m->tab_height = 24;
    m->popup_height = 24;
    m->row_height = 17;
    m->menu_bar_height = 24;
    m->menu_item_height = 20;
    /* a frame of its own, with its menus in it */
    m->frame_title = 27;
    m->frame_border = 7;
    m->frame_button_width = 18;
    m->frame_button_height = 19;
    m->frame_button_gap = 1;
    m->frame_button_inset = 7;
    m->frame_button_rise = 3;
    m->frame_left = 0x5;   /* the window's own menu; two clicks on it close the window */
    m->frame_right = 0x23; /* small, then large, nearest the edge */
    m->menu_in_window = 1;
    m->shortcut_form = 1;  /* Ctrl+L */
    m->menu_border = 2;
    m->menu_shadow = 0;
}

static void palette(press_palette *p) {
    p->window = GROUND;
    p->text = BLACK;
    p->paper = PAPER;
    p->ink = BLACK;
    p->menu = GROUND;
    p->menu_text = BLACK;
}

/* No typeface is asked for: that desktop's was in whole pixels, as the engine's own is. */
static const press_style style = {PRESS_STYLE_ABI, "ninetythree", metrics, palette, draw, 0, NULL};

const press_style *press_style_entry(void) {
    return &style;
}
