/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Ninety: a style for press in the manner of the office desktops of 1990,
 * when grey buttons first stood out of white windows. A window is white
 * and what is written on it black; a button is a grey plate in a black
 * line with its corners off, lit at the top left; a check box is a plain
 * square with a cross in it; the title bar is navy with the title in the
 * middle, and the frame a thick grey band to take hold of. Every pixel of
 * it is drawn here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=ninety ./examples/c/controls
 */
#include "smooth.h"

#define GREY rgb(0xc0c0c0)
#define SHADE rgb(0x808080)
#define NAVY rgb(0x000080)
#define WHITE rgb(0xffffff)
#define BLACK rgb(0x000000)

/* A grey plate in a black line with the corners left off, lit along its
 * top and left and two pixels deep in shade along its bottom and right. */
static void plate(box r, int held) {
    fill((box){r.x + 1, r.y + 1, r.w - 2, r.h - 2}, GREY);
    fill((box){r.x + 1, r.y, r.w - 2, 1}, BLACK);
    fill((box){r.x + 1, r.y + r.h - 1, r.w - 2, 1}, BLACK);
    fill((box){r.x, r.y + 1, 1, r.h - 2}, BLACK);
    fill((box){r.x + r.w - 1, r.y + 1, 1, r.h - 2}, BLACK);
    if (held) {
        fill((box){r.x + 1, r.y + 1, r.w - 2, 1}, SHADE);
        fill((box){r.x + 1, r.y + 1, 1, r.h - 2}, SHADE);
        return;
    }
    fill((box){r.x + 1, r.y + 1, r.w - 3, 1}, WHITE);
    fill((box){r.x + 1, r.y + 1, 1, r.h - 3}, WHITE);
    for (int d = 0; d < 2; d++) {
        fill((box){r.x + 1 + d, r.y + r.h - 2 - d, r.w - 2 - d, 1}, SHADE);
        fill((box){r.x + r.w - 2 - d, r.y + 1 + d, 1, r.h - 2 - d}, SHADE);
    }
}

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

static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD), line = line_height(), wide = text_width(p->text);
    box face = r;
    if (has(p, PRESS_STATE_DEFAULT)) {
        /* a heavier line round the one Return presses */
        fill((box){r.x + 1, r.y, r.w - 2, r.h}, BLACK);
        fill((box){r.x, r.y + 1, r.w, r.h - 2}, BLACK);
        face = inset(r, 1);
    }
    plate(face, held);
    int tx = r.x + (r.w - wide) / 2 + held, ty = r.y + (r.h - line) / 2 + held;
    words(p->text, tx, ty, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){tx - 3, ty - 1, wide + 6, line + 2});
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){x - 2, r.y + (r.h - line) / 2 - 1, text_width(p->text) + 4, line + 2});
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 12) / 2, 12, 12};
    fill(b, WHITE);
    outline(b, BLACK);
    if (has(p, PRESS_STATE_HELD)) outline(inset(b, 1), BLACK);
    if (has(p, PRESS_STATE_CHECKED))
        for (int i = 1; i < 11; i++) {
            px(b.x + i, b.y + i, BLACK);
            px(b.x + 11 - i, b.y + i, BLACK);
        }
    label(p, r.x + 12 + 6, r);
}

static void radio(const press_part *p, box r) {
    static const char *shape[12] = {
        "....kkkk....", "..kk....kk..", ".k........k.", ".k........k.", "k..........k", "k..........k",
        "k..........k", "k..........k", ".k........k.", ".k........k.", "..kk....kk..", "....kkkk....",
    };
    int top = r.y + (r.h - 12) / 2;
    for (int y = 0; y < 12; y++)
        for (int x = 0; x < 12; x++)
            px(r.x + x, top + y, shape[y][x] == 'k' ? BLACK : (x > 0 && x < 11 && y > 0 && y < 11 && shape[y][x] == '.' ? at(r.x + x, top + y) : at(r.x + x, top + y)));
    if (has(p, PRESS_STATE_HELD)) {
        fill((box){r.x + 4, top + 1, 4, 1}, BLACK);
        fill((box){r.x + 4, top + 10, 4, 1}, BLACK);
        fill((box){r.x + 1, top + 4, 1, 4}, BLACK);
        fill((box){r.x + 10, top + 4, 1, 4}, BLACK);
    }
    if (has(p, PRESS_STATE_CHECKED)) {
        fill((box){r.x + 4, top + 3, 4, 6}, BLACK);
        fill((box){r.x + 3, top + 4, 6, 4}, BLACK);
    }
    label(p, r.x + 12 + 6, r);
}

static void select_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            if (c == BLACK) px(x, y, WHITE);
            else if (c == WHITE) px(x, y, NAVY);
        }
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (at(x, y) == BLACK) px(x, y, SHADE);
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, live ? GREY : WHITE);
    outline(r, BLACK);
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        int held = live && p->a == second + 1;
        fill(b, GREY);
        outline(b, BLACK);
        if (held) {
            fill((box){b.x + 1, b.y + 1, b.w - 2, 1}, SHADE);
            fill((box){b.x + 1, b.y + 1, 1, b.h - 2}, SHADE);
        } else {
            fill((box){b.x + 1, b.y + 1, b.w - 3, 1}, WHITE);
            fill((box){b.x + 1, b.y + 1, 1, b.h - 3}, WHITE);
            fill((box){b.x + 1, b.y + b.h - 2, b.w - 2, 1}, SHADE);
            fill((box){b.x + b.w - 2, b.y + 1, 1, b.h - 2}, SHADE);
        }
        arrow(b.x + b.w / 2 + held, b.y + b.h / 2 + held, 4, vertical ? second : 2 + second, live ? BLACK : SHADE);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h};
        fill(t, GREY);
        outline(t, BLACK);
        fill((box){t.x + 1, t.y + 1, t.w - 3, 1}, WHITE);
        fill((box){t.x + 1, t.y + 1, 1, t.h - 3}, WHITE);
        fill((box){t.x + 1, t.y + t.h - 2, t.w - 2, 1}, SHADE);
        fill((box){t.x + t.w - 2, t.y + 1, 1, t.h - 2}, SHADE);
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove = vertical ? (box){r.x + r.w / 2 - 2, r.y, 4, r.h} : (box){r.x, r.y + r.h / 2 - 2, r.w, 4};
    box knob = vertical ? (box){r.x, p->a, r.w, p->b} : (box){p->a, r.y, p->b, r.h};
    fill(groove, WHITE);
    outline(groove, BLACK);
    plate(knob, has(p, PRESS_STATE_HELD));
    if (has(p, PRESS_STATE_FOCUSED)) dotted((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4});
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    int top = r.y + (front ? 0 : 2), foot = r.y + r.h - 1;
    fill((box){r.x + 1, top + 1, r.w - 2, foot - top}, front ? WHITE : GREY);
    fill((box){r.x + 1, top, r.w - 2, 1}, BLACK);
    fill((box){r.x, top + 1, 1, foot - top}, BLACK);
    fill((box){r.x + r.w - 1, top + 1, 1, foot - top}, BLACK);
    words(p->text, r.x + 10, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), BLACK);
    if (front) {
        fill((box){r.x + 1, foot, r.w - 2, 1}, WHITE);
        if (has(p, PRESS_STATE_FOCUSED)) dotted((box){r.x + 4, r.y + 3, r.w - 8, r.h - 6});
    }
}

static void popup(const press_part *p, box r) {
    /* its choice in a white box, and a button apart from it with an arrow
     * and a line under: a list drops from here */
    int line = line_height(), held = has(p, PRESS_STATE_HELD);
    box field = {r.x, r.y, r.w - r.h - 4, r.h}, drop = {r.x + r.w - r.h, r.y, r.h, r.h};
    fill(r, WHITE);
    outline(field, BLACK);
    if (has(p, PRESS_STATE_FOCUSED)) {
        fill(inset(field, 2), NAVY);
        words(p->text, r.x + 5, r.y + (r.h - line) / 2, WHITE);
    } else {
        words(p->text, r.x + 5, r.y + (r.h - line) / 2, BLACK);
    }
    plate(drop, held);
    arrow(drop.x + drop.w / 2 + held, drop.y + drop.h / 2 - 2 + held, 4, 1, BLACK);
    fill((box){drop.x + drop.w / 2 - 3 + held, drop.y + drop.h / 2 + 3 + held, 7, 2}, BLACK);
}

/* A window's frame: a grey band all round to take hold of, ruled off at
 * the corners, and a navy bar across the top with the title in white in
 * its middle. Behind others the bar is white and the title black. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height(), b = p->b;
    box bar = {r.x + b, r.y + b, r.w - 2 * b, p->a - b - 1};
    fill(r, active ? GREY : WHITE);
    outline(r, BLACK);
    outline(inset(r, b - 1), BLACK);
    /* the corners of the band are marked off: they stretch two ways */
    int corner = p->a - 1;
    fill((box){r.x + corner, r.y, 1, b}, BLACK);
    fill((box){r.x + r.w - 1 - corner, r.y, 1, b}, BLACK);
    fill((box){r.x + corner, r.y + r.h - b, 1, b}, BLACK);
    fill((box){r.x + r.w - 1 - corner, r.y + r.h - b, 1, b}, BLACK);
    fill((box){r.x, r.y + corner, b, 1}, BLACK);
    fill((box){r.x + r.w - b, r.y + corner, b, 1}, BLACK);
    fill((box){r.x, r.y + r.h - 1 - corner, b, 1}, BLACK);
    fill((box){r.x + r.w - b, r.y + r.h - 1 - corner, b, 1}, BLACK);
    fill(bar, active ? NAVY : WHITE);
    fill((box){bar.x, bar.y + bar.h, bar.w, 1}, BLACK);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, bar.y + (bar.h - line) / 2, active ? WHITE : BLACK);
}

/* At the left of the bar, the box of the window's own menu, which here
 * closes it: a bar with a shadow. At the right, an arrow down to put the
 * window away and an arrow up to make it large. */
static void frame_button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    if (p->a == PRESS_FRAME_MENU) {
        fill(r, GREY);
        fill((box){r.x + r.w, r.y, 1, r.h}, BLACK);
        box bar = {r.x + 3, r.y + r.h / 2 - 1, r.w - 7, 3};
        fill((box){bar.x + 1, bar.y + 1, bar.w, bar.h}, SHADE);
        fill(bar, WHITE);
        outline(bar, BLACK);
        if (held) select_in(r);
        return;
    }
    fill((box){r.x - 1, r.y, 1, r.h}, BLACK);
    fill(r, GREY);
    if (held) {
        fill((box){r.x, r.y, r.w, 1}, SHADE);
        fill((box){r.x, r.y, 1, r.h}, SHADE);
    } else {
        fill((box){r.x, r.y, r.w - 1, 1}, WHITE);
        fill((box){r.x, r.y, 1, r.h - 1}, WHITE);
        for (int d = 0; d < 2; d++) {
            fill((box){r.x + d, r.y + r.h - 1 - d, r.w - d, 1}, SHADE);
            fill((box){r.x + r.w - 1 - d, r.y + d, 1, r.h - d}, SHADE);
        }
    }
    int cx = r.x + r.w / 2 + held, cy = r.y + r.h / 2 + held;
    if (p->a == PRESS_FRAME_MINIMIZE) {
        arrow(cx, cy, 4, 1, BLACK);
    } else if (has(p, PRESS_STATE_CHECKED)) {
        /* made large already: an arrow each way, to put it back */
        arrow(cx, cy - 3, 3, 0, BLACK);
        arrow(cx, cy + 3, 3, 1, BLACK);
    } else {
        arrow(cx, cy, 4, 0, BLACK);
    }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, WHITE); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        fill(r, WHITE);
        outline(inset(r, 1), BLACK);
        break;
    case PRESS_PART_SELECT: select_in(r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: dotted(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 1);
        fill(r, WHITE);
        outline(r, BLACK);
        if (has(p, PRESS_STATE_BUSY)) {
            for (int y = in.y; y < in.y + in.h; y++)
                for (int x = in.x; x < in.x + in.w; x++)
                    if (((x + y - p->phase) % 8 + 8) % 8 < 4) px(x, y, NAVY);
        } else {
            fill((box){in.x, in.y, (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1)), in.h}, NAVY);
        }
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER:
        fill(r, GREY);
        outline(r, BLACK);
        break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, WHITE);
        outline(r, BLACK);
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        fill(r, WHITE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) fill((box){r.x, r.y, r.w, r.h}, NAVY);
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : SHADE);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(f, BLACK);
        fill((box){f.x + f.w, f.y + 2, p->a, f.h - 1}, BLACK);
        fill((box){f.x + 2, f.y + f.h, f.w - 1, p->a}, BLACK);
        break;
    }
    case PRESS_PART_MENU_SEPARATOR: fill((box){r.x, r.y + r.h / 2, r.w, 1}, BLACK); break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, WHITE);
        outline(r, BLACK);
        break;
    case PRESS_PART_HEADER:
        fill(r, GREY);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_HEADER_CELL:
        fill((box){r.x, r.y, r.w - 1, 1}, WHITE);
        fill((box){r.x, r.y, 1, r.h - 1}, WHITE);
        fill((box){r.x + r.w - 2, r.y + 1, 1, r.h - 2}, SHADE);
        fill((box){r.x + r.w - 1, r.y, 1, r.h}, BLACK);
        break;
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
    m->default_ring = 0;
    m->check_size = 12;
    m->radio_size = 12;
    m->field_inset_y = 4;
    m->progress_height = 14;
    m->slider_thick = 18;
    m->slider_knob = 11;
    m->scroll_bar = 17;
    m->popup_height = 20;
    m->menu_item_height = 18;
    m->menu_shadow = 1;
    /* a frame of its own, with its menus in it */
    m->frame_title = 24;
    m->ok_leads = 1; /* OK, then Cancel */
    m->frame_border = 4;
    m->frame_button_width = 19;
    m->frame_button_height = 19;
    m->frame_button_gap = 1;
    m->frame_button_inset = 4;
    m->frame_button_rise = 1;
    m->frame_left = 0x5;   /* the window's own menu; two clicks on it close the window */
    m->frame_right = 0x23; /* put away, then make large, nearest the edge */
    m->menu_in_window = 1;
    m->shortcut_form = 1;  /* Ctrl+L */
}

static void palette(press_palette *p) {
    p->window = WHITE;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = WHITE;
    p->menu_text = BLACK;
    p->pointer = 0xffffffffu; /* the arrow is white, edged in black */
    p->pointer_edge = 0xff000000u;
}

static const press_style style = {PRESS_STYLE_ABI, "ninety", metrics, palette, draw, 0, NULL};

const press_style *press_style_entry(void) {
    return &style;
}
