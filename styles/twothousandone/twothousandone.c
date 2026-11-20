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

COLOUR(SAND, 0xece9d8);     /* what windows are made of */
FOLLOWER(EDGE, 0x003c74);    /* round a button */
#define EDGE follows(EDGE, BLUE)
FOLLOWER(FIELD_EDGE, 0x7f9db9);
#define FIELD_EDGE follows(FIELD_EDGE, BLUE)
FOLLOWER(BOX_EDGE, 0x1c5180); /* round a checkbox */
#define BOX_EDGE follows(BOX_EDGE, BLUE)
COLOUR(GLOW, 0xf8b330);     /* under the pointer */
FOLLOWER(GLOW_DEEP, 0xe59700);
#define GLOW_DEEP follows(GLOW_DEEP, GLOW)
COLOUR(BLUE, 0x316ac5);     /* what is selected */
FOLLOWER(SKY, 0xc3d5fd);     /* scroll bars and the like */
#define SKY follows(SKY, BLUE)
FOLLOWER(SKY_LIGHT, 0xe3ecff);
#define SKY_LIGHT follows(SKY_LIGHT, BLUE)
FOLLOWER(SKY_EDGE, 0x98b1e4);
#define SKY_EDGE follows(SKY_EDGE, BLUE)
FOLLOWER(SKY_INK, 0x4d6185);
#define SKY_INK follows(SKY_INK, BLUE)
COLOUR(GREEN, 0x21a121);
FOLLOWER(GREEN_LIGHT, 0x7fe07f);
#define GREEN_LIGHT follows(GREEN_LIGHT, GREEN)
FOLLOWER(GREY, 0xaca899);    /* out of use, and thin lines */
#define GREY follows(GREY, SAND)
COLOUR(WHITE, 0xffffff);
COLOUR(BLACK, 0x000000);

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
        round_box(r, 3, like(SAND, 0xe2e1d9), like(SAND, 0xf0efe9), EDGE);
    else
        round_box(r, 3, WHITE, like(SAND, 0xd9d4c7), EDGE);
    /* a glow just inside the edge: gold under the pointer, blue on the
     * button Return presses or the keyboard is on */
    if (!held && has(p, PRESS_STATE_HOT)) {
        round_line(inset(r, 1), 2, 2, GLOW);
        fill_over((box){r.x + 3, r.y + r.h - 3, r.w - 6, 2}, with_alpha(GLOW_DEEP, 0.7f));
    } else if (!held && (has(p, PRESS_STATE_DEFAULT) || has(p, PRESS_STATE_FOCUSED))) {
        round_line(inset(r, 1), 2, 2, like(BLUE, 0x9dbdf4));
        fill_over((box){r.x + 3, r.y + r.h - 3, r.w - 6, 2}, with_alpha(like(BLUE, 0x6f93d8), 0.8f));
    }
    int line = line_height(), wide = text_width(p->text);
    words(p->text, r.x + (r.w - wide) / 2 + held, r.y + (r.h - line) / 2 + held, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(inset(r, 3));
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    gradient(b, has(p, PRESS_STATE_HELD) ? like(SAND, 0xb0b0a7) : like(SAND, 0xdcdcd7), WHITE);
    outline(b, BOX_EDGE);
    if (has(p, PRESS_STATE_HOT))
        round_line(inset(b, 1), 0, 2, GLOW);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick, three pixels heavy */
        tick(b.x + 3, b.y + 3, 7, 2, 3, GREEN);
    }
    label(p, r.x + 13 + 5, r);
}

static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    round_box(b, 6.5f, has(p, PRESS_STATE_HELD) ? like(SAND, 0xb0b0a7) : like(SAND, 0xdcdcd7), WHITE, BOX_EDGE);
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
    if (live && held == 2) {
        /* under the pointer: lighter */
        round_box(r, 2, WHITE, like(BLUE, 0xd6e3ff), SKY_EDGE);
        return;
    }
    if (!live)
        round_box(r, 2, like(SAND, 0xf6f5f0), like(SAND, 0xe6e4dc), like(SAND, 0xd0cec4));
    else if (held)
        round_box(r, 2, like(BLUE, 0x9fb9f2), like(BLUE, 0xb7cdfb), SKY_EDGE);
    else
        round_box(r, 2, SKY_LIGHT, SKY, SKY_EDGE);
    if (live && !held)
        round_line(inset(r, 1), 1, 1, with_alpha(WHITE, 0.8f));
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, like(SAND, 0xf6f5f1));
    if (vertical) fill((box){r.x, r.y, 1, r.h}, like(SAND, 0xe8e6df));
    else fill((box){r.x, r.y, r.w, 1}, like(SAND, 0xe8e6df));
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        int held = live && p->a == second + 1;
        sky_box(b, held ? 1 : p->d == second + 1 ? 2 : 0, live);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? SKY_INK : GREY);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h};
        sky_box(t, p->d == 3 ? 2 : 0, 1);
        /* ridges to take hold of, if there is room */
        if (p->c >= 16) {
            for (int i = -3; i <= 3; i += 2) {
                box ridge = vertical ? (box){t.x + 5, t.y + t.h / 2 + i, t.w - 10, 1} : (box){t.x + t.w / 2 + i, t.y + 5, 1, t.h - 10};
                fill(ridge, like(BLUE, 0x8cb0f8));
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
    round_box(groove, 1.5f, like(SAND, 0xe7eaea), WHITE, like(SAND, 0x9d9c99));
    round_box(knob, 2.5f, WHITE, like(SAND, 0xd5d5c9), like(BLUE, 0x778892));
    /* a line of colour at each end of the knob: gold under the pointer */
    uint32_t tip = has(p, PRESS_STATE_HOT) || has(p, PRESS_STATE_HELD) ? GLOW : like(GREEN, 0x3cb43c);
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
    round_box(r, 3, like(SAND, 0xf2f2f2), WHITE, like(SAND, 0x686868));
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
        gradient((box){b.x, b.y, b.w, b.h / 2}, like(GREEN, 0xc4f2c0), like(GREEN, 0x2ed13a));
        gradient((box){b.x, b.y + b.h / 2, b.w, b.h - b.h / 2}, like(GREEN, 0x2ed13a), like(GREEN, 0x9ce89a));
    }
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT), line = line_height();
    int top = r.y + (front ? 0 : 2), foot = r.y + r.h - 1;
    box shape = {r.x, top, r.w, foot - top + 4};
    if (front) {
        round_box(shape, 3, WHITE, like(SAND, 0xfcfcfe), like(SAND, 0x919b9c));
        /* a gold line along the top of the chosen tab */
        fill((box){r.x + 2, top, r.w - 4, 1}, like(GLOW, 0xe68b2c));
        fill((box){r.x + 1, top + 1, r.w - 2, 1}, like(GLOW, 0xffc73c));
        /* and nothing between it and the pane */
        fill((box){r.x + 1, foot, r.w - 2, 4}, like(SAND, 0xfcfcfe));
    } else {
        round_box(shape, 3, WHITE, like(SAND, 0xeae8dc), like(SAND, 0x919b9c));
        if (has(p, PRESS_STATE_HOT)) {
            /* under the pointer, a tab behind takes the gold line of the one in front */
            fill((box){r.x + 2, top, r.w - 4, 1}, like(GLOW, 0xe68b2c));
            fill((box){r.x + 1, top + 1, r.w - 2, 1}, like(GLOW, 0xffc73c));
        }
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
        fill((box){r.x + 2, r.y + 2, drop.x - r.x - 3, r.h - 4}, like(BLUE, 0xdde6f7));
    words(p->text, r.x + 5, r.y + (r.h - line) / 2, BLACK);
    sky_box(drop, held, 1);
    if (has(p, PRESS_STATE_HOT) && !held)
        round_line(drop, 2, 1, like(BLUE, 0x6a8fd8));
    arrow(drop.x + drop.w / 2, drop.y + drop.h / 2, 4, 1, SKY_INK);
}

/* A window's frame: blue, deep at the edges and light along the top of
 * the title bar, with the top corners rounded off and the title in white.
 * Behind others it is paler. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height();
    uint32_t deep = active ? like(BLUE, 0x0a3fcf) : like(BLUE, 0x7a8fd0), mid = active ? like(BLUE, 0x0f62e8) : like(BLUE, 0x8ba3e0), high = active ? like(BLUE, 0x4d9bff) : like(BLUE, 0xa9bdf0);
    /* the whole shape, rounded at the top only */
    round_fill((box){r.x, r.y, r.w, p->a + 8}, 7, deep, deep);
    fill((box){r.x, r.y + p->a, r.w, r.h - p->a}, deep);
    box bar = {r.x + 1, r.y + 1, r.w - 2, p->a - 1};
    for (int y = 0; y < bar.h; y++) {
        float t = (float)y / (bar.h - 1);
        uint32_t c = t < 0.12f ? mix(high, mid, t / 0.12f) : t < 0.85f ? mix(mid, mix(mid, deep, 0.35f), (t - 0.12f) / 0.73f) : mix(mix(mid, deep, 0.35f), deep, (t - 0.85f) / 0.15f);
        for (int x = bar.x; x < bar.x + bar.w; x++) {
            float cover = covered(outside((box){r.x + 1, r.y + 1, r.w - 2, p->a + 8}, 6, x, bar.y + y));
            if (cover > 0) blend(x, bar.y + y, c, cover);
        }
    }
    /* the edges: a lighter line just inside the dark one */
    fill((box){r.x + 1, r.y + p->a, 1, r.h - p->a - 1}, mid);
    fill((box){r.x + r.w - 2, r.y + p->a, 1, r.h - p->a - 1}, mid);
    fill((box){r.x + 1, r.y + r.h - 2, r.w - 2, 1}, mid);
    int ty = r.y + (p->a - line) / 2 + 1;
    words(p->text, r.x + 9, ty + 1, with_alpha(like(BLUE, 0x0a1883), active ? 0.9f : 0.3f));
    words(p->text, r.x + 8, ty, active ? WHITE : like(BLUE, 0xd8e4f8));
}

/* The buttons at the right of the title bar: squares of blue gel with a
 * white sign, and the one that closes red. */
static void frame_button(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), held = has(p, PRESS_STATE_HELD), hot = has(p, PRESS_STATE_HOT);
    uint32_t top, bottom;
    if (p->a == PRESS_FRAME_CLOSE) {
        top = held ? rgb(0xa8301c) : hot ? rgb(0xf28a72) : rgb(0xe8674b);
        bottom = held ? rgb(0xc8432a) : rgb(0xc03418);
    } else {
        top = held ? like(BLUE, 0x1c3fb0) : hot ? like(BLUE, 0x7fb2ff) : like(BLUE, 0x4d8cf5);
        bottom = held ? like(BLUE, 0x2f5fd8) : like(BLUE, 0x1c50c8);
    }
    if (!active) {
        top = mix(top, like(BLUE, 0xb8c4e8), 0.55f);
        bottom = mix(bottom, like(BLUE, 0xb8c4e8), 0.55f);
    }
    round_box(r, 3, top, bottom, WHITE);
    int cx = r.x + r.w / 2, cy = r.y + r.h / 2;
    switch (p->a) {
    case PRESS_FRAME_CLOSE:
        cross(cx, cy, 4, 1, 3, WHITE);
        break;
    case PRESS_FRAME_MINIMIZE: fill((box){cx - 5, cy + 3, 8, 3}, WHITE); break;
    default:
        if (has(p, PRESS_STATE_CHECKED)) {
            outline((box){cx - 2, cy - 5, 8, 7}, WHITE);
            fill((box){cx - 2, cy - 5, 8, 2}, WHITE);
            fill((box){cx - 5, cy - 1, 8, 7}, bottom);
            outline((box){cx - 5, cy - 1, 8, 7}, WHITE);
            fill((box){cx - 5, cy - 1, 8, 2}, WHITE);
        } else {
            outline((box){cx - 5, cy - 5, 11, 11}, WHITE);
            fill((box){cx - 5, cy - 5, 11, 3}, WHITE);
        }
        break;
    }
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
        fill(r, like(SAND, 0xfcfcfe));
        outline(r, like(SAND, 0x919b9c));
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
        fill(r, like(SAND, 0xebeadb));
        fill((box){r.x, r.y + r.h - 3, r.w, 1}, like(SAND, 0xe2decd));
        fill((box){r.x, r.y + r.h - 2, r.w, 1}, like(SAND, 0xd6d2c2));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, like(SAND, 0xcbc7b8));
        break;
    case PRESS_PART_HEADER_CELL:
        if (has(p, PRESS_STATE_HOT)) {
            /* under the pointer a heading lightens, with gold along its foot */
            fill((box){r.x, r.y, r.w, r.h - 3}, like(SAND, 0xfaf9f4));
            fill((box){r.x, r.y + r.h - 3, r.w, 1}, like(GLOW, 0xf9a900));
            fill((box){r.x, r.y + r.h - 2, r.w, 1}, like(GLOW, 0xf8b330));
            fill((box){r.x, r.y + r.h - 1, r.w, 1}, like(GLOW, 0xe59700));
            break;
        }
        fill((box){r.x + r.w - 2, r.y + 3, 1, r.h - 7}, like(SAND, 0xc7c5b2));
        fill((box){r.x + r.w - 1, r.y + 3, 1, r.h - 7}, WHITE);
        break;
    case PRESS_PART_TREE_FOLD: {
        /* a little box with a minus in it, or a plus when there is more to show */
        box b = {p->a - 4, p->b - 4, 9, 9};
        gradient(b, WHITE, like(SAND, 0xc3bab0));
        outline(b, like(BLUE, 0x7898b5));
        fill((box){b.x + 2, b.y + 4, 5, 1}, BLACK);
        if (!has(p, PRESS_STATE_CHECKED)) fill((box){b.x + 4, b.y + 2, 1, 5}, BLACK);
        break;
    }
    case PRESS_PART_GROUP_BOX: {
        /* a thin rounded line, its title in blue across the top of it */
        int top = p->text[0] ? line_height() / 2 : 0;
        round_line((box){r.x, r.y + top, r.w, r.h - top}, 4, 1, like(SAND, 0xd0d0bf));
        if (p->text[0]) {
            fill((box){r.x + 7, r.y, text_width(p->text) + 6, line_height()}, (uint32_t)p->c);
            words(p->text, r.x + 10, r.y, like(BLUE, 0x0046d5));
        }
        break;
    }
    case PRESS_PART_STATUS_BAR:
        /* a soft shade along the top, and a grip of dots in the corner */
        fill((box){r.x, r.y, r.w, 1}, like(SAND, 0xc5c2b2));
        fill((box){r.x, r.y + 1, r.w, 1}, like(SAND, 0xe3e0d0));
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j <= i; j++) {
                box dot = {r.x + r.w - 5 - 4 * j, r.y + r.h - 13 + 4 * i, 2, 2};
                fill((box){dot.x + 1, dot.y + 1, 2, 2}, WHITE);
                fill(dot, like(SAND, 0xb8b4a2));
            }
        }
        break;
    case PRESS_PART_STATUS_PANEL:
        /* cells are told apart by a line between them, cut into the bar */
        if (p->a > 0) {
            fill((box){r.x - 2, r.y + 1, 1, r.h - 3}, like(SAND, 0xc5c2b2));
            fill((box){r.x - 1, r.y + 1, 1, r.h - 3}, WHITE);
        }
        break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
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
    /* a frame of its own, with its menus in it */
    m->frame_title = 29;
    m->ok_leads = 1; /* OK, then Cancel */
    m->frame_border = 4;
    m->frame_button_width = 21;
    m->frame_button_height = 21;
    m->frame_button_gap = 2;
    m->frame_button_inset = 6;
    m->frame_button_rise = 1;
    m->frame_left = 0;
    m->frame_right = 0x231; /* put away, make large, close */
    m->menu_in_window = 1;
    m->shortcut_form = 1;   /* Ctrl+L */
}

static void palette(press_palette *p) {
    p->pane = like(SAND, 0xfcfcfe); /* the pages of a tab view are nearly white */
    p->window = SAND;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = WHITE;
    p->menu_text = BLACK;
    p->tip = 0xffffffe1u; /* a word of help is on pale yellow */
    p->tip_text = 0xff000000u;
    p->pointer = 0xffffffffu; /* the arrow is white, edged in black */
    p->pointer_edge = 0xff000000u;
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
    case PRESS_FACE_FRAME: strcpy(out->family, "Trebuchet MS,Tahoma,Verdana,DejaVu Sans"); out->size = 13; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"sand", &SAND},
    {"edge", &EDGE_SAID},
    {"field_edge", &FIELD_EDGE_SAID},
    {"box_edge", &BOX_EDGE_SAID},
    {"glow", &GLOW},
    {"glow_deep", &GLOW_DEEP_SAID},
    {"blue", &BLUE},
    {"sky", &SKY_SAID},
    {"sky_light", &SKY_LIGHT_SAID},
    {"sky_edge", &SKY_EDGE_SAID},
    {"sky_ink", &SKY_INK_SAID},
    {"green", &GREEN},
    {"green_light", &GREEN_LIGHT_SAID},
    {"grey", &GREY_SAID},
    {"white", &WHITE},
    {"black", &BLACK},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "twothousandone", metrics, palette, draw, PRESS_WANTS_HOVER | PRESS_WANTS_FINE, typeface, colours};

const press_style *press_style_entry(void) {
    return &style;
}
