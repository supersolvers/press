/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Ninety-Nine: a style for press in the manner of the platinum desktops of
 * the end of the 1990s: the black-and-white Macintosh in shades of grey.
 * Everything keeps its black outline and its rounded corners and gains a
 * face of light grey, lit from the top left; what is selected in a list is
 * tinted lavender, and in a menu deep blue. Pixels are whole and the type
 * is the engine's own, which was drawn for a desktop like this one. Every
 * pixel of it is drawn here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=ninetynine ./examples/c/controls
 */
#include "smooth.h"

#define PLATE rgb(0xdddddd)    /* what windows and buttons are made of */
#define LIT rgb(0xffffff)      /* an edge the light falls on */
#define SHADE rgb(0xaaaaaa)    /* an edge in shade */
#define DEEP rgb(0x777777)
#define PRESSED rgb(0x666666)
#define TINT rgb(0xccccff)     /* what is selected among the user's things */
#define TINT_DEEP rgb(0x9999cc)
#define ROYAL rgb(0x333399)    /* what is chosen in a menu */
#define WHITE rgb(0xffffff)
#define BLACK rgb(0x000000)

/* A lit edge along the top and left of a box, and a shaded one along its
 * bottom and right. */
static void lit_edge(box r, uint32_t top_left, uint32_t bottom_right) {
    if (r.w <= 1 || r.h <= 1) return;
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, bottom_right);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, bottom_right);
    fill((box){r.x, r.y, r.w - 1, 1}, top_left);
    fill((box){r.x, r.y, 1, r.h - 1}, top_left);
}

/* A plate: a black outline with its corners rounded off, and a face lit
 * from the top left. The three pixels beyond each corner are left alone. */
static void plate(box r, uint32_t face, uint32_t top_left, uint32_t bottom_right) {
    if (r.w < 6 || r.h < 6) {
        fill(r, face);
        outline(r, BLACK);
        return;
    }
    int right = r.x + r.w - 1, foot = r.y + r.h - 1;
    fill((box){r.x + 2, r.y + 1, r.w - 4, r.h - 2}, face);
    fill((box){r.x + 1, r.y + 2, r.w - 2, r.h - 4}, face);
    /* lit along the top and left, in shade along the bottom and right */
    fill((box){r.x + 2, foot - 1, r.w - 4, 1}, bottom_right);
    fill((box){right - 1, r.y + 2, 1, r.h - 4}, bottom_right);
    fill((box){r.x + 2, r.y + 1, r.w - 5, 1}, top_left);
    fill((box){r.x + 1, r.y + 2, 1, r.h - 5}, top_left);
    fill((box){r.x + 2, r.y, r.w - 4, 1}, BLACK);
    fill((box){r.x + 2, foot, r.w - 4, 1}, BLACK);
    fill((box){r.x, r.y + 2, 1, r.h - 4}, BLACK);
    fill((box){right, r.y + 2, 1, r.h - 4}, BLACK);
    px(r.x + 1, r.y + 1, BLACK);
    px(right - 1, r.y + 1, BLACK);
    px(r.x + 1, foot - 1, BLACK);
    px(right - 1, foot - 1, BLACK);
}

/* A ring two pixels heavy: this has the keyboard. */
static void ring(box r) {
    outline(r, ROYAL);
    outline(inset(r, 1), TINT_DEEP);
}

static void button(const press_part *p, box r) {
    box face = r;
    int held = has(p, PRESS_STATE_HELD);
    if (has(p, PRESS_STATE_DEFAULT)) {
        /* a heavy rim round the one Return presses */
        plate(r, rgb(0xbbbbbb), PLATE, DEEP);
        face = inset(r, 3);
    }
    if (held)
        plate(face, PRESSED, rgb(0x444444), rgb(0x888888));
    else
        plate(face, PLATE, LIT, SHADE);
    words(p->text, face.x + (face.w - text_width(p->text)) / 2, face.y + (face.h - line_height()) / 2, held ? WHITE : BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){r.x - 3, r.y - 3, r.w + 6, r.h + 6});
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){x - 3, r.y + (r.h - line) / 2 - 2, text_width(p->text) + 6, line + 4});
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 12) / 2, 12, 12};
    int held = has(p, PRESS_STATE_HELD);
    fill(b, held ? rgb(0x999999) : PLATE);
    lit_edge(inset(b, 1), held ? DEEP : LIT, held ? rgb(0xbbbbbb) : SHADE);
    outline(b, BLACK);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick made with a broad pen, which runs up out of the box */
        static const int rise[9] = {4, 5, 6, 5, 4, 3, 2, 1, 0};
        for (int i = 0; i < 9; i++) {
            fill((box){b.x + 2 + i, b.y + 1 + rise[i], 1, 3}, BLACK);
            px(b.x + 3 + i, b.y + 4 + rise[i], SHADE);
        }
    }
    label(p, r.x + 12 + 5, r);
}

static void radio(const press_part *p, box r) {
    static const char *shape[12] = {
        "....kkkk....",
        "..kkwwwwkk..",
        ".kwwffffffk.",
        ".kwffffffsk.",
        "kwffffffffsk",
        "kwffffffffsk",
        "kwffffffffsk",
        "kwffffffffsk",
        ".kwffffffsk.",
        ".kffssssssk.",
        "..kksssskk..",
        "....kkkk....",
    };
    int top = r.y + (r.h - 12) / 2, held = has(p, PRESS_STATE_HELD);
    for (int y = 0; y < 12; y++) {
        for (int x = 0; x < 12; x++) {
            switch (shape[y][x]) {
            case 'k': px(r.x + x, top + y, BLACK); break;
            case 'w': px(r.x + x, top + y, held ? DEEP : LIT); break;
            case 's': px(r.x + x, top + y, held ? rgb(0xbbbbbb) : SHADE); break;
            case 'f': px(r.x + x, top + y, held ? rgb(0x999999) : PLATE); break;
            default: break;
            }
        }
    }
    if (has(p, PRESS_STATE_CHECKED)) {
        fill((box){r.x + 4, top + 5, 4, 2}, BLACK);
        fill((box){r.x + 5, top + 4, 2, 4}, BLACK);
    }
    label(p, r.x + 12 + 5, r);
}

/* What is selected. On paper it is tinted, and the words stay black; in a
 * menu, which is grey, it goes deep blue and the words white. */
static void select_in(const press_part *p, box r) {
    int menu = p->a == 2;
    for (int y = r.y; y < r.y + r.h; y++) {
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            if (menu) {
                if (c == PLATE) px(x, y, ROYAL);
                else if (c == BLACK) px(x, y, WHITE);
            } else if (c == WHITE) {
                px(x, y, TINT);
            }
        }
    }
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (at(x, y) == BLACK) px(x, y, rgb(0x888888));
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove, knob;
    if (vertical) {
        groove = (box){r.x + r.w / 2 - 3, r.y + p->b / 2, 6, r.h - p->b + 1};
        knob = (box){r.x + 1, p->a, r.w - 2, p->b};
    } else {
        groove = (box){r.x + p->b / 2, r.y + r.h / 2 - 3, r.w - p->b + 1, 6};
        knob = (box){p->a, r.y + 1, p->b, r.h - 2};
    }
    /* a channel cut into the window, and a plate that rides in it */
    lit_edge(groove, DEEP, LIT);
    fill(inset(groove, 1), rgb(0x999999));
    outline(inset(groove, 1), BLACK);
    plate(knob, has(p, PRESS_STATE_HELD) ? TINT_DEEP : TINT, LIT, TINT_DEEP);
    for (int i = -2; i <= 2; i += 2) {
        if (vertical) fill((box){knob.x + 3, knob.y + knob.h / 2 + i, knob.w - 6, 1}, ROYAL);
        else fill((box){knob.x + knob.w / 2 + i, knob.y + 3, 1, knob.h - 6}, ROYAL);
    }
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4});
}

static void progress(const press_part *p, box r) {
    box in = inset(r, 1);
    fill(in, SHADE);
    fill((box){in.x, in.y, in.w, 1}, DEEP);
    outline(r, BLACK);
    if (has(p, PRESS_STATE_BUSY)) {
        /* a barber's pole, turning */
        for (int y = in.y; y < in.y + in.h; y++)
            for (int x = in.x; x < in.x + in.w; x++)
                px(x, y, ((x + y - p->phase) % 16 + 16) % 16 < 8 ? TINT : ROYAL);
        fill((box){in.x, in.y, in.w, 1}, with_alpha(WHITE, 1));
        return;
    }
    int done = (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1));
    if (done <= 0) return;
    /* a bar of the tint, round like a rod: light along its top, deep at its foot */
    gradient((box){in.x, in.y, done, in.h / 2}, TINT, rgb(0x6666cc));
    gradient((box){in.x, in.y + in.h / 2, done, in.h - in.h / 2}, rgb(0x6666cc), ROYAL);
    if (done < in.w) fill((box){in.x + done, in.y, 1, in.h}, BLACK);
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    /* with somewhere to go the track is a trough in shade; with nowhere, flat */
    fill(r, live ? SHADE : PLATE);
    if (live) {
        if (vertical) fill((box){r.x + 1, r.y, 2, r.h}, DEEP);
        else fill((box){r.x, r.y + 1, r.w, 2}, DEEP);
    }
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        int held = live && p->a == second + 1;
        fill(b, held ? PRESSED : PLATE);
        lit_edge(inset(b, 1), held ? rgb(0x444444) : LIT, held ? rgb(0x888888) : live ? SHADE : PLATE);
        outline(b, BLACK);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, !live ? SHADE : held ? WHITE : BLACK);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h};
        fill(t, TINT);
        lit_edge(inset(t, 1), LIT, TINT_DEEP);
        outline(t, BLACK);
        /* ridges to take hold of */
        if (p->c >= 14) {
            for (int i = -3; i <= 3; i += 2) {
                if (vertical) {
                    fill((box){t.x + 4, t.y + t.h / 2 + i, t.w - 8, 1}, ROYAL);
                    fill((box){t.x + 4, t.y + t.h / 2 + i - 1, t.w - 8, 1}, LIT);
                } else {
                    fill((box){t.x + t.w / 2 + i, t.y + 4, 1, t.h - 8}, ROYAL);
                    fill((box){t.x + t.w / 2 + i - 1, t.y + 4, 1, t.h - 8}, LIT);
                }
            }
        }
    }
    outline(r, BLACK);
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    int top = r.y + (front ? 0 : 2), foot = r.y + r.h - 1, right = r.x + r.w - 1;
    uint32_t face = front ? PLATE : rgb(0xbbbbbb);
    /* a tab is a plate with no bottom */
    fill((box){r.x + 2, top + 1, r.w - 4, foot - top}, face);
    fill((box){r.x + 1, top + 2, r.w - 2, foot - top - 1}, face);
    fill((box){r.x + 2, top + 1, r.w - 5, 1}, front ? LIT : PLATE);
    fill((box){r.x + 1, top + 2, 1, foot - top - 2}, front ? LIT : PLATE);
    fill((box){right - 1, top + 2, 1, foot - top - 2}, front ? SHADE : rgb(0x999999));
    fill((box){r.x + 2, top, r.w - 4, 1}, BLACK);
    fill((box){r.x, top + 2, 1, foot - top - 1}, BLACK);
    fill((box){right, top + 2, 1, foot - top - 1}, BLACK);
    px(r.x + 1, top + 1, BLACK);
    px(right - 1, top + 1, BLACK);
    words(p->text, r.x + 10, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), BLACK);
    if (front) {
        /* the chosen tab and the pane are one piece */
        fill((box){r.x + 1, foot, r.w - 2, 2}, PLATE);
        px(r.x + 1, foot, LIT);
        px(r.x + 1, foot + 1, LIT);
        if (has(p, PRESS_STATE_FOCUSED))
            ring((box){r.x + 4, r.y + 3, r.w - 8, r.h - 6});
    }
}

static void popup(const press_part *p, box r) {
    int cap = r.h, held = has(p, PRESS_STATE_HELD);
    plate(r, held ? PRESSED : PLATE, held ? rgb(0x444444) : LIT, held ? rgb(0x888888) : SHADE);
    /* the end that opens it is set off by a line, with an arrow each way */
    box end = {r.x + r.w - cap, r.y + 1, cap - 1, r.h - 2};
    fill((box){end.x, end.y, 1, end.h}, BLACK);
    if (!held) {
        fill((box){end.x + 1, end.y + 1, end.w - 2, end.h - 2}, rgb(0xcccccc));
        fill((box){end.x + 1, end.y, 1, end.h - 1}, LIT);
        fill((box){end.x + 1, end.y, end.w - 2, 1}, LIT);
    }
    arrow(end.x + end.w / 2 + 1, r.y + r.h / 2 - 4, 4, 0, held ? WHITE : BLACK);
    arrow(end.x + end.w / 2 + 1, r.y + r.h / 2 + 4, 4, 1, held ? WHITE : BLACK);
    words(p->text, r.x + 8, r.y + (r.h - line_height()) / 2, held ? WHITE : BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){r.x - 3, r.y - 3, r.w + 6, r.h + 6});
}

/* A window's frame: a black line round a rim of plate, lit from the top
 * left, with the title in the middle of its top and ridges running out
 * from the title either way, to take hold of. Behind others it is plain. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height();
    box inner = {r.x + p->b - 1, r.y + p->a - 1, r.w - 2 * p->b + 2, r.h - p->a - p->b + 2};
    int rolled = has(p, PRESS_STATE_HELD);
    fill(r, active ? rgb(0xcccccc) : PLATE);
    if (active) lit_edge(inset(r, 1), LIT, DEEP);
    outline(r, BLACK);
    /* the content sits in a black line of its own; rolled up, there is none to */
    if (!rolled) {
        outline(inner, BLACK);
        if (active) lit_edge((box){inner.x - 1, inner.y - 1, inner.w + 2, inner.h + 2}, DEEP, LIT);
    }
    int wide = text_width(p->text), tx = r.x + (r.w - wide) / 2, ty = r.y + (p->a - 1 - line) / 2 + 1;
    if (active) {
        for (int y = r.y + 5; y < r.y + p->a - 5; y += 2) {
            fill((box){r.x + 6, y, r.w - 12, 1}, LIT);
            fill((box){r.x + 6, y + 1, r.w - 12, 1}, DEEP);
        }
        fill((box){tx - 6, r.y + 3, wide + 12, p->a - 6}, rgb(0xcccccc));
    }
    words(p->text, tx, ty, active ? BLACK : rgb(0x888888));
}

/* The boxes in the title bar: close at the left; at the right, zoom and
 * the one that rolls the window up. Behind others a window shows none. */
static void frame_button(const press_part *p, box r) {
    if (!has(p, PRESS_STATE_ENABLED)) return;
    int held = has(p, PRESS_STATE_HELD);
    /* a gap in the ridges for it to sit in */
    fill((box){r.x - 2, r.y - 1, r.w + 4, r.h + 2}, rgb(0xcccccc));
    lit_edge(r, DEEP, LIT);
    fill(inset(r, 1), held ? PRESSED : PLATE);
    outline(inset(r, 1), BLACK);
    if (!held) lit_edge(inset(r, 2), LIT, SHADE);
    if (p->a == PRESS_FRAME_ZOOM) {
        /* a smaller box in its corner */
        fill((box){r.x + 2, r.y + 7, 6, 1}, BLACK);
        fill((box){r.x + 7, r.y + 2, 1, 6}, BLACK);
    } else if (p->a == PRESS_FRAME_ROLL) {
        /* lines across it: the window rolled up into its title */
        fill((box){r.x + 2, r.y + 5, r.w - 4, 1}, BLACK);
        fill((box){r.x + 2, r.y + 7, r.w - 4, 1}, BLACK);
    }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, PLATE); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        /* paper in a black frame, let into the window; with the keyboard,
         * the frame is ringed */
        fill(r, WHITE);
        outline(inset(r, 1), BLACK);
        if (has(p, PRESS_STATE_FOCUSED)) outline(r, ROYAL);
        else lit_edge(r, SHADE, LIT);
        break;
    case PRESS_PART_SELECT: select_in(p, r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: ring(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: progress(p, r); break;
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER:
        fill(r, PLATE);
        outline(r, BLACK);
        break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, PLATE);
        lit_edge(inset(r, 1), LIT, SHADE);
        outline(r, BLACK);
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        fill(r, PLATE);
        fill((box){r.x, r.y, r.w, 1}, LIT);
        fill((box){r.x, r.y + r.h - 2, r.w, 1}, SHADE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) fill(r, ROYAL);
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : rgb(0x888888));
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        /* a black line round it and a plain shadow, darker than what it falls on */
        box frame = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(frame, BLACK);
        fill((box){frame.x + frame.w, frame.y + p->a, p->a, frame.h}, with_alpha(BLACK, 0.4f));
        fill((box){frame.x + p->a, frame.y + frame.h, frame.w - p->a, p->a}, with_alpha(BLACK, 0.4f));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x + 1, r.y + r.h / 2 - 1, r.w - 2, 1}, SHADE);
        fill((box){r.x + 1, r.y + r.h / 2, r.w - 2, 1}, LIT);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, PLATE);
        outline(r, BLACK);
        break;
    case PRESS_PART_HEADER:
        fill(r, PLATE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_HEADER_CELL:
        lit_edge((box){r.x, r.y, r.w - 1, r.h - 1}, LIT, SHADE);
        fill((box){r.x + r.w - 1, r.y, 1, r.h}, BLACK);
        break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 20;
    m->button_min_width = 59;
    m->button_pad = 12;
    m->default_ring = 3;
    m->check_size = 12;
    m->check_gap = 5;
    m->radio_size = 12;
    m->radio_gap = 5;
    m->progress_height = 14;
    m->slider_thick = 16;
    m->slider_knob = 11;
    m->popup_height = 20;
    m->menu_item_height = 17;
    m->menu_shadow = 2;
    /* a frame of its own; its menus go to the top of the screen where a desktop has them there */
    m->frame_title = 22;
    m->frame_border = 6;
    m->frame_button_width = 13;
    m->frame_button_height = 13;
    m->frame_button_gap = 5;
    m->frame_button_inset = 9;
    m->frame_button_rise = 0;
    m->frame_left = 0x1;   /* close */
    m->frame_right = 0x34; /* zoom, then roll up, nearest the edge */
    m->menu_in_window = 0;
    m->frame_double = 1;   /* two clicks on the title roll the window up */
}

static void palette(press_palette *p) {
    p->window = PLATE;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = PLATE;
    p->menu_text = BLACK;
}

/* No typeface is asked for: the engine's own families are in whole pixels,
 * as that desktop's were, and its heavy one is the face of the interface. */
static const press_style style = {PRESS_STYLE_ABI, "ninetynine", metrics, palette, draw, 0, NULL};

const press_style *press_style_entry(void) {
    return &style;
}
