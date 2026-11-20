/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Ninety-Eight: a style for press in the manner of the media desktop of
 * 1998, known at a glance by its windows: the title is not a bar but a
 * yellow tab, as wide as the title needs, standing on the top left corner
 * of the window like the tab of a folder. Inside, everything is quiet:
 * light grey, thin dark lines, a little light at the top left of each
 * thing, and blue for where the keyboard is. Every pixel of it is drawn
 * here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=ninetyeight ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

COLOUR(PANEL, 0xd8d8d8);  /* what windows are made of */
COLOUR(LIT, 0xffffff);
FOLLOWER(SOFT, 0xb8b8b8);
#define SOFT follows(SOFT, PANEL)
FOLLOWER(SHADE, 0x989898);
#define SHADE follows(SHADE, PANEL)
FOLLOWER(LINE, 0x606060);  /* round a button */
#define LINE follows(LINE, PANEL)
COLOUR(YELLOW, 0xffcb00); /* the tab of the window in front */
FOLLOWER(YELLOW_LIT, 0xffe880);
#define YELLOW_LIT follows(YELLOW_LIT, YELLOW)
FOLLOWER(YELLOW_SHADE, 0xd2a000);
#define YELLOW_SHADE follows(YELLOW_SHADE, YELLOW)
COLOUR(KEY, 0x0000e5);    /* where the keyboard is */
COLOUR(BLUE, 0x3296ff);   /* work done */
COLOUR(WHITE, 0xffffff);
COLOUR(BLACK, 0x000000);

static void edge(box r, uint32_t top_left, uint32_t bottom_right) {
    if (r.w <= 1 || r.h <= 1) return;
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, bottom_right);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, bottom_right);
    fill((box){r.x, r.y, r.w - 1, 1}, top_left);
    fill((box){r.x, r.y, 1, r.h - 1}, top_left);
}

/* A plate in a thin dark line, lit at the top left. */
static void plate(box r, int held) {
    fill(r, held ? SOFT : like(PANEL, 0xe8e8e8));
    outline(r, LINE);
    edge(inset(r, 1), held ? SHADE : LIT, held ? PANEL : SOFT);
}

/* Something let into the window: in shade at the top left. */
static void let_in(box r, uint32_t inside) {
    fill(r, inside);
    edge(r, SHADE, LIT);
    edge(inset(r, 1), LINE, PANEL);
}

/* The keyboard is here: the label is underlined in blue. */
static void label(const press_part *p, int x, box r) {
    int line = line_height(), y = r.y + (r.h - line) / 2;
    words(p->text, x, y, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        fill((box){x, y + line - 1, text_width(p->text), 1}, KEY);
}

static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD), line = line_height(), wide = text_width(p->text);
    box face = r;
    fill(r, PANEL);
    if (has(p, PRESS_STATE_DEFAULT)) {
        /* a heavy black rim round the one Return presses, a pixel off it */
        outline(r, BLACK);
        outline(inset(r, 1), BLACK);
        face = inset(r, 3);
    }
    plate(face, held);
    int tx = r.x + (r.w - wide) / 2, ty = r.y + (r.h - line) / 2;
    words(p->text, tx, ty, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        fill((box){tx, ty + line - 1, wide, 1}, KEY);
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    let_in(b, has(p, PRESS_STATE_HELD) ? SOFT : WHITE);
    if (has(p, PRESS_STATE_FOCUSED)) outline(b, KEY);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a cross in blue, drawn with a broad pen */
        for (int i = 0; i < 7; i++) {
            fill((box){b.x + 3 + i, b.y + 3 + i, 2, 1}, like(BLUE, 0x1b52c8));
            fill((box){b.x + 9 - i, b.y + 3 + i, 2, 1}, like(BLUE, 0x1b52c8));
        }
    }
    label(p, r.x + 13 + 6, r);
}

static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    round_fill(b, 6.5f, SHADE, LIT);
    round_fill(inset(b, 1), 5.5f, LINE, LINE);
    round_fill(inset(b, 2), 4.5f, has(p, PRESS_STATE_HELD) ? SOFT : WHITE, has(p, PRESS_STATE_HELD) ? SOFT : WHITE);
    if (has(p, PRESS_STATE_CHECKED))
        round_fill(inset(b, 4), 2.5f, like(BLUE, 0x4d8cf0), like(BLUE, 0x1b52c8));
    if (has(p, PRESS_STATE_FOCUSED)) round_line(b, 6.5f, 1, KEY);
    label(p, r.x + 13 + 6, r);
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, mix(c, SHADE, 0.7f * (1 - light(c))));
        }
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, live ? like(PANEL, 0xc4c4c4) : PANEL);
    outline(r, LINE);
    if (live) {
        if (vertical) fill((box){r.x + 1, r.y + 1, 1, r.h - 2}, SHADE);
        else fill((box){r.x + 1, r.y + 1, r.w - 2, 1}, SHADE);
    }
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        plate(b, live && p->a == second + 1);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? BLACK : SHADE);
    }
    if (p->b >= 0)
        plate(vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h}, 0);
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove = vertical ? (box){r.x + r.w / 2 - 3, r.y + 2, 6, r.h - 4} : (box){r.x + 2, r.y + r.h / 2 - 3, r.w - 4, 6};
    box knob = vertical ? (box){r.x + 1, p->a, r.w - 2, p->b} : (box){p->a, r.y + 1, p->b, r.h - 2};
    let_in(groove, like(PANEL, 0x9c9c9c));
    plate(knob, has(p, PRESS_STATE_HELD));
    if (has(p, PRESS_STATE_FOCUSED)) outline(knob, KEY);
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT), line = line_height();
    int top = r.y + (front ? 0 : 3), foot = r.y + r.h - 1;
    box shape = {r.x, top, r.w, foot - top + 5};
    round_box(shape, 4, front ? PANEL : like(PANEL, 0xc8c8c8), front ? PANEL : like(PANEL, 0xc8c8c8), LINE);
    round_line(inset(shape, 1), 3, 1, with_alpha(WHITE, front ? 0.9f : 0.5f));
    if (front) fill((box){r.x + 1, foot, r.w - 2, 5}, PANEL);
    else fill((box){r.x, foot, r.w, 5}, PANEL);
    int tx = r.x + (r.w - text_width(p->text)) / 2, ty = r.y + (r.h - line) / 2 + (front ? 0 : 2);
    words(p->text, tx, ty, BLACK);
    if (front && has(p, PRESS_STATE_FOCUSED))
        fill((box){tx, ty + line - 1, text_width(p->text), 1}, KEY);
}

static void popup(const press_part *p, box r) {
    /* a plate with its choice, and a small arrow down at the end of it */
    plate(r, has(p, PRESS_STATE_HELD));
    label(p, r.x + 8, r);
    arrow(r.x + r.w - 11, r.y + r.h / 2, 4, 1, BLACK);
}

/* A window's frame: a grey rim round the content and, standing on its top
 * left corner, the tab: yellow for the window in front, with the title in
 * it. The strip beside the tab is left clear. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height(), b = p->b;
    box body = {r.x, r.y + p->a - b, r.w, r.h - p->a + b};
    box tab_ = {r.x, r.y, p->c, p->a - b + 1};
    uint32_t face = active ? YELLOW : like(PANEL, 0xe8e8e8), lit = active ? YELLOW_LIT : WHITE, shade = active ? YELLOW_SHADE : SOFT;
    /* the rim: a dark line, light, two of the panel, and a line of shade within */
    fill(body, PANEL);
    outline(body, LINE);
    edge(inset(body, 1), LIT, SHADE);
    outline(inset(body, b - 1), SHADE);
    /* the tab */
    fill(tab_, face);
    edge((box){tab_.x + 1, tab_.y + 1, tab_.w - 2, tab_.h}, lit, shade);
    fill((box){tab_.x, tab_.y, tab_.w, 1}, LINE);
    fill((box){tab_.x, tab_.y, 1, tab_.h}, LINE);
    fill((box){tab_.x + tab_.w - 1, tab_.y, 1, tab_.h}, LINE);
    words(p->text, r.x + 24, r.y + (p->a - b - line) / 2 + 1, active ? BLACK : like(PANEL, 0x505050));
}

/* The boxes in the tab: at its left the one that closes, a small square;
 * at its right the one that zooms, a small square before a large. */
static void frame_button(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), held = has(p, PRESS_STATE_HELD);
    uint32_t top = active ? YELLOW_LIT : WHITE, foot = active ? YELLOW_SHADE : SOFT;
    if (held) { uint32_t t = top; top = foot; foot = t; }
    if (p->a == PRESS_FRAME_CLOSE) {
        gradient(r, top, foot);
        outline(r, LINE);
    } else {
        box large = {r.x + 3, r.y + 3, r.w - 3, r.h - 3}, small = {r.x, r.y, r.w - 5, r.h - 5};
        gradient(large, top, foot);
        outline(large, LINE);
        gradient(small, top, foot);
        outline(small, LINE);
    }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, PANEL); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        let_in(r, WHITE);
        /* with the keyboard, it is ringed in blue */
        if (has(p, PRESS_STATE_FOCUSED)) {
            outline(r, KEY);
            outline(inset(r, 1), KEY);
        }
        break;
    case PRESS_PART_SELECT: recolour(r, like(PANEL, 0x9c9c9c), BLACK); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: outline(r, KEY); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 2);
        let_in(r, WHITE);
        box done = in;
        if (has(p, PRESS_STATE_BUSY)) {
            int span = in.w - in.w / 4, at_ = (p->phase * 6) % (2 * (span > 0 ? span : 1));
            done.x = in.x + (at_ < span ? at_ : 2 * span - at_);
            done.w = in.w / 4;
        } else {
            done.w = (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1));
        }
        if (done.w > 0) {
            fill(done, BLUE);
            edge(done, like(BLUE, 0x8cc3ff), like(BLUE, 0x1e6ec8));
        }
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, PANEL); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, PANEL);
        outline(r, LINE);
        edge(inset(r, 1), LIT, SOFT);
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        fill(r, PANEL);
        fill((box){r.x, r.y, r.w, 1}, LIT);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, SHADE);
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) fill((box){r.x, r.y + 1, r.w, r.h - 2}, like(PANEL, 0x9c9c9c));
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, has(p, PRESS_STATE_ENABLED) ? BLACK : SHADE);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(f, LINE);
        fill((box){f.x + f.w, f.y + p->a, p->a, f.h}, with_alpha(BLACK, 0.3f));
        fill((box){f.x + p->a, f.y + f.h, f.w - p->a, p->a}, with_alpha(BLACK, 0.3f));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x + 1, r.y + r.h / 2 - 1, r.w - 2, 1}, SHADE);
        fill((box){r.x + 1, r.y + r.h / 2, r.w - 2, 1}, LIT);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, PANEL);
        outline(r, LINE);
        break;
    case PRESS_PART_HEADER:
        fill(r, PANEL);
        fill((box){r.x, r.y, r.w, 1}, LIT);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, SHADE);
        break;
    case PRESS_PART_HEADER_CELL:
        fill((box){r.x + r.w - 2, r.y + 2, 1, r.h - 4}, SHADE);
        fill((box){r.x + r.w - 1, r.y + 2, 1, r.h - 4}, LIT);
        break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 24;
    m->button_min_width = 75;
    m->button_pad = 12;
    m->default_ring = 3;
    m->check_size = 13;
    m->check_gap = 6;
    m->radio_size = 13;
    m->radio_gap = 6;
    m->field_inset_x = 5;
    m->field_inset_y = 4;
    m->progress_height = 16;
    m->slider_thick = 18;
    m->scroll_bar = 15;
    m->slider_knob = 11;
    m->tab_height = 22;
    m->popup_height = 22;
    m->row_height = 17;
    m->menu_bar_height = 20;
    m->menu_item_height = 18;
    m->menu_shadow = 2;
    /* a frame of its own, the title a tab on its corner, and its menus in it */
    m->frame_title = 24;
    m->frame_border = 5;
    m->frame_button_width = 13;
    m->frame_button_height = 13;
    m->frame_button_gap = 4;
    m->frame_button_inset = 5;
    m->frame_button_rise = -2;
    m->frame_left = 0x1;  /* close */
    m->frame_right = 0x3; /* zoom */
    m->menu_in_window = 1;
    m->frame_tab = 6;
    m->frame_double = 2; /* two clicks on the tab put the window away */
}

static void palette(press_palette *p) {
    p->window = PANEL;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = PANEL;
    p->menu_text = BLACK;
    p->tip = 0xffffffd8u; /* a word of help is on pale yellow */
    p->tip_text = 0xff000000u;
}

static int32_t typeface(uint32_t face, press_type *out) {
    const char *sans = "Swis721 BT,Helvetica,Nimbus Sans,Liberation Sans,DejaVu Sans";
    const char *fixed = "Courier10 BT,Courier New,Liberation Mono,DejaVu Sans Mono";
    switch (face) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 12; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 10; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 10; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 12; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 12; break;
    case PRESS_FACE_TITLE: strcpy(out->family, fixed); out->size = 15; out->bold = 1; break;
    case PRESS_FACE_FRAME: strcpy(out->family, sans); out->size = 12; out->bold = 1; break;
    default: return 0;
    }
    return 1;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"panel", &PANEL},
    {"lit", &LIT},
    {"soft", &SOFT_SAID},
    {"shade", &SHADE_SAID},
    {"line", &LINE_SAID},
    {"yellow", &YELLOW},
    {"yellow_lit", &YELLOW_LIT_SAID},
    {"yellow_shade", &YELLOW_SHADE_SAID},
    {"key", &KEY},
    {"blue", &BLUE},
    {"white", &WHITE},
    {"black", &BLACK},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "ninetyeight", metrics, palette, draw, 0, typeface, colours};

const press_style *press_style_entry(void) {
    return &style;
}
