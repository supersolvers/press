/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Two Thousand and Four: a style for press in the manner of the free
 * desktop of the autumn of 2004, which was warm where others were cool:
 * windows the colour of parchment, title bars of tan shading to brown,
 * and orange for what is selected and for work done. Its shapes are
 * plain: thin grey edges, corners barely rounded, faces flat and pale,
 * a little darker low down. Every pixel of it is drawn here; nothing is copied
 * from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=twothousandfour ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

COLOUR(PARCHMENT, 0xefebe7); /* what windows are made of */
FOLLOWER(EDGE, 0x9c968e);    /* round a button or a field */
#define EDGE follows(EDGE, PARCHMENT)
FOLLOWER(EDGE_DEEP, 0x5e5246);
#define EDGE_DEEP follows(EDGE_DEEP, PARCHMENT)
FOLLOWER(FACE_TOP, 0xf7f5f2);
#define FACE_TOP follows(FACE_TOP, PARCHMENT)
FOLLOWER(FACE_FOOT, 0xebe7e2);
#define FACE_FOOT follows(FACE_FOOT, PARCHMENT)
COLOUR(ORANGE, 0xe07b24);   /* what is selected */
FOLLOWER(ORANGE_LIGHT, 0xf6c07a);
#define ORANGE_LIGHT follows(ORANGE_LIGHT, ORANGE)
FOLLOWER(ORANGE_DEEP, 0xb85a12);
#define ORANGE_DEEP follows(ORANGE_DEEP, ORANGE)
COLOUR(BROWN, 0x6b4a2b);
FOLLOWER(GREY, 0xa39a8f);
#define GREY follows(GREY, PARCHMENT)
COLOUR(WHITE, 0xffffff);
COLOUR(BLACK, 0x000000);

/* The shades the period's widgets were made of, each following the
 * parchment: what a trough is, round a trough, round the parts of a scroll
 * bar, a tab behind, the dark of an arrow or a mark, and the soft pixel at
 * a corner. */
#define TROUGH like(PARCHMENT, 0xd6d1ca)
#define TROUGH_RIM like(PARCHMENT, 0xc7c2bb)
#define BAR_RIM like(PARCHMENT, 0xaaa59e)
#define BEHIND like(PARCHMENT, 0xdbd6cf)
#define INK like(PARCHMENT, 0x302d29)
#define SHADE like(PARCHMENT, 0xe3dfda)

/* A face, as the period's buttons were: flat and pale in a thin grey line,
 * the corners just off, a little darker for a few rows low down and light
 * again on its last row. Lighter under the pointer; held, flat and darker. */
static void face(box r, int held, int hot) {
    uint32_t top = held ? like(PARCHMENT, 0xdcd7d0) : hot ? like(PARCHMENT, 0xfcfbfa) : FACE_TOP;
    round_box(r, 1.5f, top, top, EDGE);
    if (held || r.h < 10) return;
    int band = r.h >= 20 ? 7 : 3;
    for (int i = 0; i < band; i++)
        fill((box){r.x + 1, r.y + r.h - 2 - band + i, r.w - 2, 1}, mix(top, hot ? like(PARCHMENT, 0xf1eeea) : FACE_FOOT, (float)(i + 1) / band));
}

static void dotted(box r) {
    for (int x = r.x; x < r.x + r.w; x++) {
        if ((x + r.y) % 2 == 0) px(x, r.y, INK);
        if ((x + r.y + r.h - 1) % 2 == 0) px(x, r.y + r.h - 1, INK);
    }
    for (int y = r.y; y < r.y + r.h; y++) {
        if ((y + r.x) % 2 == 0) px(r.x, y, INK);
        if ((y + r.x + r.w - 1) % 2 == 0) px(r.x + r.w - 1, y, INK);
    }
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){x - 2, r.y + (r.h - line) / 2 - 1, text_width(p->text) + 4, line + 2});
}

/* The one Return presses has a dark line round it, and a soft grey glow
 * three pixels deep outside that. */
static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    if (has(p, PRESS_STATE_DEFAULT)) {
        uint32_t glow[3] = {like(PARCHMENT, 0xf2f0ed), like(PARCHMENT, 0xdfdbd6), like(PARCHMENT, 0xb5b0a9)};
        for (int i = 0; i < 3; i++)
            round_line(inset(r, i), 2.5f - i * 0.5f, 1, glow[i]);
        r = inset(r, 3);
        face(r, held, has(p, PRESS_STATE_HOT));
        round_line(r, 1.5f, 1, INK);
    } else {
        face(r, held, has(p, PRESS_STATE_HOT));
    }
    words(p->text, r.x + (r.w - text_width(p->text)) / 2 + held, r.y + (r.h - line_height()) / 2 + held, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(inset(r, 3));
}

/* A box to tick: a grey line, the corners just off, sunk a little (shade
 * along its top and left inside, light along its right and foot), and a
 * dark tick. */
static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    uint32_t inside = has(p, PRESS_STATE_HELD) ? like(PARCHMENT, 0xdcd7d0) : has(p, PRESS_STATE_HOT) ? WHITE : like(PARCHMENT, 0xf7f5f3);
    round_box(b, 1.5f, inside, inside, EDGE);
    fill((box){b.x + 1, b.y + 1, b.w - 2, 1}, SHADE);
    fill((box){b.x + 1, b.y + 1, 1, b.h - 2}, SHADE);
    fill((box){b.x + b.w - 2, b.y + 1, 1, b.h - 2}, WHITE);
    fill((box){b.x + 1, b.y + b.h - 2, b.w - 2, 1}, WHITE);
    if (has(p, PRESS_STATE_CHECKED))
        tick(b.x + 3, b.y + 3, 7, 2, 2, INK);
    label(p, r.x + 13 + 6, r);
}

/* A ring to choose by: grey, shaded inside from its top, white below, and
 * a dark spot in the middle of the one chosen. */
static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    uint32_t top = has(p, PRESS_STATE_HELD) ? like(PARCHMENT, 0xd3cec7) : SHADE;
    round_box(b, 6.5f, top, has(p, PRESS_STATE_HOT) || !has(p, PRESS_STATE_HELD) ? WHITE : like(PARCHMENT, 0xe6e2dd), EDGE);
    if (has(p, PRESS_STATE_CHECKED))
        round_fill((box){b.x + 4, b.y + 4, 5, 5}, 2.5f, like(PARCHMENT, 0x55524d), INK);
    label(p, r.x + 13 + 6, r);
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, mix(c, GREY, 0.7f * (1 - light(c))));
        }
}

/* A grip of dots, as the period's knobs had: a lattice along the knob, a
 * dot in the middle and then one either side, two pixels on each time, each
 * dot with a pale ring. */
static void grip(int cx, int cy, int length, int along_x) {
    for (int pass = 0; pass < 2; pass++)
        for (int i = -length / 2; i <= length / 2; i += 2) {
            int sides = ((i + length / 2) / 2) % 2;
            for (int k = sides ? -2 : 0; k <= (sides ? 2 : 0); k += 4) {
                int x = along_x ? cx + i : cx + k, y = along_x ? cy + k : cy + i;
                if (pass == 0) {
                    px(x - 1, y, TROUGH); px(x + 1, y, TROUGH); px(x, y - 1, TROUGH); px(x, y + 1, TROUGH);
                } else {
                    px(x, y, BAR_RIM);
                }
            }
        }
}

/* The part of a scroll bar that moves, and its arrow buttons: the colour
 * of the window in a grey line, a little shade inside along the right and
 * foot. */
static void bar_face(box r, int held, int hot) {
    uint32_t c = held ? like(PARCHMENT, 0xdcd7d0) : hot ? like(PARCHMENT, 0xf7f5f2) : PARCHMENT;
    round_box(r, 1.5f, c, c, BAR_RIM);
    fill((box){r.x + r.w - 2, r.y + 1, 1, r.h - 2}, like(PARCHMENT, 0xe7e3de));
    fill((box){r.x + 1, r.y + r.h - 2, r.w - 2, 1}, like(PARCHMENT, 0xe7e3de));
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    /* a grey trough with a paler line down each side */
    fill(r, TROUGH);
    if (vertical) {
        fill((box){r.x, r.y, 1, r.h}, TROUGH_RIM);
        fill((box){r.x + r.w - 1, r.y, 1, r.h}, TROUGH_RIM);
    } else {
        fill((box){r.x, r.y, r.w, 1}, TROUGH_RIM);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, TROUGH_RIM);
    }
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        bar_face(b, live && p->a == second + 1, live && p->d == second + 1);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? INK : GREY);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h};
        bar_face(t, 0, p->d == 3);
        int room = p->c - 10;
        if (room >= 5) grip(t.x + t.w / 2 - 1 + !vertical, t.y + t.h / 2 - !vertical, room < 17 ? room : 17, !vertical);
    }
}

/* A slider: a grey trough as deep as the knob, and the knob a pale block
 * in a grey line with a grip of dots. Nothing marks the way gone. */
static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), thick = 15;
    box groove, knob;
    if (vertical) {
        groove = (box){r.x + (r.w - thick) / 2, r.y, thick, r.h};
        knob = (box){groove.x, p->a, thick, p->b};
    } else {
        groove = (box){r.x, r.y + (r.h - thick) / 2, r.w, thick};
        knob = (box){p->a, groove.y, p->b, thick};
    }
    round_box(groove, 1.5f, TROUGH, TROUGH, TROUGH_RIM);
    uint32_t c = has(p, PRESS_STATE_HELD) ? like(PARCHMENT, 0xdcd7d0) : has(p, PRESS_STATE_HOT) ? like(PARCHMENT, 0xfcfbfa) : FACE_TOP;
    round_box(knob, 1.5f, c, c, EDGE);
    int length = (vertical ? knob.h : knob.w) - 16;
    if (length >= 5) grip(knob.x + knob.w / 2 - vertical, knob.y + knob.h / 2 - !vertical, length < 15 ? length : 15, !vertical);
    if (has(p, PRESS_STATE_FOCUSED)) dotted(r);
}

static void progress(const press_part *p, box r) {
    box in = inset(r, 1);
    round_box(r, 2.5f, like(PARCHMENT, 0xddd6cc), like(PARCHMENT, 0xf3f0ec), EDGE);
    box done = in;
    if (has(p, PRESS_STATE_BUSY)) {
        /* a piece that goes to and fro */
        int span = in.w - in.w / 4, at_ = (p->phase * 6) % (2 * (span > 0 ? span : 1));
        done.x = in.x + (at_ < span ? at_ : 2 * span - at_);
        done.w = in.w / 4;
    } else {
        done.w = (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1));
    }
    if (done.w <= 0) return;
    gradient(done, ORANGE_LIGHT, ORANGE);
    fill((box){done.x, done.y + done.h - 1, done.w, 1}, ORANGE_DEEP);
    fill((box){done.x + done.w - 1, done.y, 1, done.h}, ORANGE_DEEP);
}

/* Tabs: the one in front of a piece with its pane, pale in a grey line,
 * its top corners just off; the others two pixels lower and greyer, a
 * darker line along the top and a light one under it, a little darker low
 * down. */
static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    int foot = r.y + r.h - 1;
    if (front) {
        round_box((box){r.x, r.y, r.w, r.h + 3}, 1.5f, FACE_TOP, FACE_TOP, EDGE);
        fill((box){r.x + 1, foot, r.w - 2, 4}, FACE_TOP);
    } else {
        int top = r.y + 2;
        fill((box){r.x, top, r.w, foot - top + 1}, BEHIND);
        for (int i = 0; i < 7 && foot - 7 + i > top + 1; i++)
            fill((box){r.x + 1, foot - 7 + i, r.w - 2, 1}, mix(BEHIND, like(PARCHMENT, 0xd0cbc4), (float)(i + 1) / 7));
        fill((box){r.x, top, r.w, 1}, like(PARCHMENT, 0x8a857e));
        fill((box){r.x + 1, top + 1, r.w - 2, 1}, like(PARCHMENT, 0xe9e6e2));
        fill((box){r.x, top, 1, foot - top + 1}, EDGE);
        fill((box){r.x + r.w - 1, top, 1, foot - top + 1}, EDGE);
    }
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), BLACK);
    if (front && has(p, PRESS_STATE_FOCUSED))
        dotted((box){r.x + 3, r.y + 3, r.w - 6, r.h - 5});
}

/* A pop-up button: a face, a grey line twenty pixels in from its right,
 * and a small dark arrow beyond it. */
static void popup(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    face(r, held, has(p, PRESS_STATE_HOT));
    words(p->text, r.x + 8, r.y + (r.h - line_height()) / 2, BLACK);
    fill((box){r.x + r.w - 21, r.y + 4, 1, r.h - 8}, EDGE);
    arrow(r.x + r.w - 11, r.y + r.h / 2, 4, 1, INK);
    if (has(p, PRESS_STATE_FOCUSED)) dotted(inset(r, 3));
}

/* A window's frame, laid out as the window manager of the autumn of 2004
 * laid out its frames: square, in a line of black. In front, a line of
 * light along the top, then the title bar shading from tan to brown, the
 * title in the middle in white over a dark shadow, and a dark line under
 * it; down the sides and along the foot a line of light, two of pale tan,
 * a dark line inside and, right and foot, a line of shade. Behind others
 * it is flat, the colour of the window, the title grey and no line inside. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height(), t = p->a, b = p->b;
    uint32_t light = active ? like(ORANGE, 0xf0e0cc) : WHITE;
    uint32_t edge = active ? like(ORANGE, 0xd3c2ab) : PARCHMENT;
    uint32_t shade = active ? like(BROWN, 0x7b664d) : like(PARCHMENT, 0xa8a49f);
    uint32_t top = like(ORANGE, 0xc4a37c), foot = like(BROWN, 0x87704f);
    fill(r, BLACK);
    box in = {r.x + 1, r.y + 1, r.w - 2, r.h - 2};
    fill(in, edge);
    /* the title bar: a line of light, then shaded, or flat behind */
    fill((box){in.x, in.y, in.w, 1}, light);
    for (int y = in.y + 1; y < r.y + t - 1; y++)
        fill((box){in.x + 1, y, in.w - 1, 1}, active ? mix(top, foot, (float)(y - in.y - 1) / (t - 4)) : PARCHMENT);
    /* light down the left, shade down the right and along the foot */
    fill((box){in.x, in.y, 1, in.h}, light);
    fill((box){in.x + in.w - 1, r.y + t - 1, 1, in.h - t + 1}, shade);
    fill((box){in.x, in.y + in.h - 1, in.w, 1}, shade);
    /* a dark line round the window's own room, in front */
    if (active) {
        fill((box){r.x + b - 1, r.y + t - 1, r.w - 2 * b + 2, 1}, BLACK);
        outline((box){r.x + b - 1, r.y + t - 1, r.w - 2 * b + 2, r.h - t - b + 2}, BLACK);
    }
    int tx = r.x + (r.w - text_width(p->text)) / 2, ty = r.y + (t - line) / 2;
    if (active) {
        words(p->text, tx + 1, ty + 1, with_alpha(like(BROWN, 0x2e2216), 0.8f));
        words(p->text, tx, ty, WHITE);
    } else {
        words(p->text, tx, ty, GREY);
    }
}

/* The buttons at the right of the title bar: square, a little darker than
 * the bar inside a rim of light, with a sign in white two pixels thick.
 * Behind others they are only a grey rim and a grey sign. */
static void frame_button(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), held = has(p, PRESS_STATE_HELD), hot = has(p, PRESS_STATE_HOT);
    uint32_t ink = active ? WHITE : like(PARCHMENT, 0xb8b4ae);
    if (active) {
        uint32_t top = like(BROWN, 0x826e55), foot = like(BROWN, 0x675743);
        if (held) top = foot = like(BROWN, 0x55483a);
        else if (hot) top = mix(top, WHITE, 0.18f), foot = mix(foot, WHITE, 0.18f);
        for (int y = 1; y < r.h - 1; y++)
            fill((box){r.x + 1, r.y + y, r.w - 2, 1}, mix(top, foot, (float)(y - 1) / (r.h - 3)));
        outline(r, with_alpha(WHITE, 0.32f));
    } else {
        outline(r, like(PARCHMENT, 0xcbc7c2));
    }
    int d = held;
    switch (p->a) {
    case PRESS_FRAME_CLOSE:
        cross(r.x + r.w / 2 + d, r.y + r.h / 2 + d, 5, 0, 2, ink);
        break;
    case PRESS_FRAME_MINIMIZE: fill((box){r.x + 3 + d, r.y + r.h - 5 + d, r.w - 6, 2}, ink); break;
    default:
        if (has(p, PRESS_STATE_CHECKED)) {
            /* made large: a smaller window, to go back to */
            outline((box){r.x + 5 + d, r.y + 5 + d, r.w - 10, r.h - 10}, ink);
            fill((box){r.x + 5 + d, r.y + 5 + d, r.w - 10, 2}, ink);
        } else {
            outline((box){r.x + 3 + d, r.y + 3 + d, r.w - 6, r.h - 6}, ink);
            fill((box){r.x + 3 + d, r.y + 3 + d, r.w - 6, 2}, ink);
        }
    }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, PARCHMENT); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        /* white in a grey line, the corners just off */
        round_box(r, 1.5f, WHITE, WHITE, EDGE);
        break;
    case PRESS_PART_SELECT: recolour(r, ORANGE, WHITE); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: dotted(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: progress(p, r); break;
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, PARCHMENT); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, FACE_TOP);
        outline(r, EDGE);
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        /* flat, the colour of the window, with a soft line along its top */
        fill(r, PARCHMENT);
        fill((box){r.x, r.y, r.w, 1}, like(PARCHMENT, 0xdcd8d3));
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) {
            /* flat, in a darker line, down to the menu */
            fill((box){r.x, r.y + 1, r.w, r.h - 1}, ORANGE);
            outline((box){r.x, r.y + 1, r.w, r.h - 1}, ORANGE_DEEP);
        }
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : GREY);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        /* a light grey line, and no shadow */
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(f, like(PARCHMENT, 0xc4c0ba));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x + 2, r.y + r.h / 2, r.w - 4, 1}, like(PARCHMENT, 0x97938d));
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, PARCHMENT);
        outline(r, EDGE);
        break;
    case PRESS_PART_HEADER:
        /* the colour of the window, a little darker low down, a dark line under */
        fill(r, PARCHMENT);
        for (int i = 0; i < 7 && i < r.h - 2; i++)
            fill((box){r.x, r.y + r.h - 9 + i, r.w, 1}, mix(PARCHMENT, like(PARCHMENT, 0xe5e1dc), (float)(i + 1) / 7));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, like(PARCHMENT, 0x97938d));
        break;
    case PRESS_PART_HEADER_CELL:
        /* each a button of its own: two dark lines between two */
        if (has(p, PRESS_STATE_HOT)) fill((box){r.x, r.y, r.w - 2, r.h - 1}, like(PARCHMENT, 0xf7f5f2));
        fill((box){r.x + r.w - 2, r.y, 2, r.h - 1}, like(PARCHMENT, 0x97938d));
        break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 30;
    m->button_min_width = 76;
    m->button_pad = 12;
    m->default_ring = 3;
    m->check_size = 13;
    m->check_gap = 6;
    m->radio_size = 13;
    m->radio_gap = 6;
    m->field_inset_x = 5;
    m->field_inset_y = 5;
    m->progress_height = 18;
    m->slider_thick = 15;
    m->scroll_bar = 15;
    m->slider_knob = 31;
    m->tab_height = 27;
    m->popup_height = 30;
    m->row_height = 23;
    m->menu_bar_height = 24;
    m->menu_item_height = 23;
    m->menu_shadow = 0;
    /* a frame of its own, with its menus in it */
    m->frame_title = 23;
    m->frame_border = 5;
    m->frame_button_width = 17;
    m->frame_button_height = 17;
    m->frame_button_gap = 2;
    m->frame_button_inset = 5;
    m->frame_button_rise = 0;
    m->frame_left = 0;
    m->frame_right = 0x231; /* put away, make large, close */
    m->menu_in_window = 1;
    m->shortcut_form = 1;   /* Ctrl+L */
}

static void palette(press_palette *p) {
    p->window = PARCHMENT;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = FACE_TOP;
    p->menu_text = BLACK;
    p->tip = 0xfffff5b5u; /* a word of help is on yellow */
    p->tip_text = 0xff000000u;
}

/* The face of that desktop, of which the one most machines have now is the child. */
static int32_t typeface(uint32_t face_, press_type *out) {
    const char *sans = "Bitstream Vera Sans,DejaVu Sans,Noto Sans";
    const char *fixed = "Bitstream Vera Sans Mono,DejaVu Sans Mono,Liberation Mono";
    switch (face_) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 13; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 11; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 11; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 13; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 12; break;
    case PRESS_FACE_TITLE: strcpy(out->family, fixed); out->size = 15; out->bold = 1; break;
    case PRESS_FACE_FRAME: strcpy(out->family, sans); out->size = 13; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"parchment", &PARCHMENT},
    {"edge", &EDGE_SAID},
    {"edge_deep", &EDGE_DEEP_SAID},
    {"face_top", &FACE_TOP_SAID},
    {"face_foot", &FACE_FOOT_SAID},
    {"orange", &ORANGE},
    {"orange_light", &ORANGE_LIGHT_SAID},
    {"orange_deep", &ORANGE_DEEP_SAID},
    {"brown", &BROWN},
    {"grey", &GREY_SAID},
    {"white", &WHITE},
    {"black", &BLACK},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "twothousandfour", metrics, palette, draw, PRESS_WANTS_HOVER | PRESS_WANTS_FINE, typeface, colours};

const press_style *press_style_entry(void) {
    return &style;
}
