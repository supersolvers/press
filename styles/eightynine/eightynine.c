/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Eighty-Nine: a style for press in the manner of the workstation of 1989
 * whose screen had four greys and nothing else: black, dark grey, light
 * grey and white. Everything is a slab of light grey standing out of the
 * page or pressed into it, lit from the top left with a white edge and
 * shaded at the bottom right with a dark one and a black one. Title bars
 * are black. Every pixel of it is drawn here; nothing is copied from
 * anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=eightynine ./examples/c/controls
 */
#include "smooth.h"

COLOUR(LIGHT, 0xaaaaaa);
FOLLOWER(DARK, 0x555555);
#define DARK follows(DARK, LIGHT)
COLOUR(WHITE, 0xffffff);
COLOUR(BLACK, 0x000000);

static void edge(box r, uint32_t top_left, uint32_t bottom_right) {
    if (r.w <= 1 || r.h <= 1) return;
    fill((box){r.x, r.y + r.h - 1, r.w, 1}, bottom_right);
    fill((box){r.x + r.w - 1, r.y, 1, r.h}, bottom_right);
    fill((box){r.x, r.y, r.w - 1, 1}, top_left);
    fill((box){r.x, r.y, 1, r.h - 1}, top_left);
}

/* A slab standing out of the page. */
static void slab(box r, uint32_t face) {
    fill(r, face);
    edge(r, WHITE, BLACK);
    edge((box){r.x + 1, r.y + 1, r.w - 2, r.h - 2}, face, DARK);
}

/* A hollow pressed into it. */
static void hollow(box r, uint32_t inside) {
    fill(r, inside);
    edge(r, DARK, WHITE);
    edge(inset(r, 1), BLACK, LIGHT);
}

/* Two greys in a fine check: the bed something slides in. */
static void check(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, (x + y) % 2 == 0 ? DARK : LIGHT);
}

static void ring(box r) {
    for (int x = r.x; x < r.x + r.w; x += 2) {
        px(x, r.y, BLACK);
        px(x, r.y + r.h - 1, BLACK);
    }
    for (int y = r.y; y < r.y + r.h; y += 2) {
        px(r.x, y, BLACK);
        px(r.x + r.w - 1, y, BLACK);
    }
}

static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD), line = line_height();
    int wide = text_width(p->text), sign = has(p, PRESS_STATE_DEFAULT) ? 16 : 0;
    if (held) {
        /* pressed, it is white and flat */
        fill(r, WHITE);
        edge(r, BLACK, WHITE);
        edge(inset(r, 1), DARK, LIGHT);
    } else {
        slab(r, LIGHT);
    }
    int x = r.x + (r.w - wide - sign) / 2 + held, y = r.y + (r.h - line) / 2 + held;
    words(p->text, x, y, BLACK);
    if (sign) {
        /* the sign of the key that presses it: an arrow that goes down and back */
        int ax = x + wide + 6, ay = r.y + r.h / 2 + held;
        fill((box){ax + 8, ay - 4, 2, 6}, BLACK);
        fill((box){ax + 2, ay, 7, 2}, BLACK);
        arrow(ax + 2, ay, 4, 2, BLACK);
        fill((box){ax + 9, ay - 4, 1, 6}, DARK);
    }
    if (has(p, PRESS_STATE_FOCUSED))
        ring(inset(r, 3));
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){x - 2, r.y + (r.h - line) / 2 - 1, text_width(p->text) + 4, line + 2});
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 15) / 2, 15, 15};
    if (has(p, PRESS_STATE_HELD)) hollow(b, WHITE);
    else slab(b, LIGHT);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick with a shadow of its own */
        static const int rise[9] = {4, 5, 6, 5, 4, 3, 2, 1, 0};
        for (int i = 0; i < 9; i++) {
            fill((box){b.x + 4 + i, b.y + 3 + rise[i], 1, 3}, DARK);
            fill((box){b.x + 3 + i, b.y + 2 + rise[i], 1, 3}, BLACK);
        }
    }
    label(p, r.x + 15 + 6, r);
}

static void radio(const press_part *p, box r) {
    static const char *shape[15] = {
        ".....wwwww.....",
        "...wwlllllkk...",
        "..wllllllllkk..",
        ".wllllllllllkk.",
        ".wlllllllllldk.",
        "wlllllllllllldk",
        "wlllllllllllldk",
        "wlllllllllllldk",
        "wlllllllllllldk",
        "wlllllllllllldk",
        ".wlllllllllldk.",
        ".klllllllllddk.",
        "..kkllllllddk..",
        "...kkdddddkk...",
        ".....kkkkk.....",
    };
    int top = r.y + (r.h - 15) / 2;
    for (int y = 0; y < 15; y++)
        for (int x = 0; x < 15; x++)
            switch (shape[y][x]) {
            case 'w': px(r.x + x, top + y, WHITE); break;
            case 'l': px(r.x + x, top + y, has(p, PRESS_STATE_HELD) ? WHITE : LIGHT); break;
            case 'd': px(r.x + x, top + y, DARK); break;
            case 'k': px(r.x + x, top + y, BLACK); break;
            default: break;
            }
    if (has(p, PRESS_STATE_CHECKED)) {
        fill((box){r.x + 5, top + 6, 5, 3}, BLACK);
        fill((box){r.x + 6, top + 5, 3, 5}, BLACK);
        px(r.x + 6, top + 6, WHITE);
    }
    label(p, r.x + 15 + 6, r);
}

/* What is selected changes places with its ground: on paper, which is
 * white, it goes grey; in a menu, which is grey, it goes white. */
static void select_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            if (c == WHITE) px(x, y, LIGHT);
            else if (c == LIGHT) px(x, y, WHITE);
        }
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (at(x, y) == BLACK) px(x, y, DARK);
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    hollow(r, LIGHT);
    if (live) check(inset(r, 2));
    for (int second = 0; second < 2; second++) {
        /* both arrows are together at the far end of the bar */
        int from = (vertical ? r.y + r.h : r.x + r.w) - (2 - second) * thick + (second ? 0 : 2);
        box b = inset(vertical ? (box){r.x, from, thick, thick} : (box){from, r.y, thick, thick}, 2);
        int held = live && p->a == second + 1;
        if (held) fill(b, WHITE);
        else slab(b, LIGHT);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? BLACK : DARK);
    }
    if (p->b >= 0) {
        box t = inset(vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h}, 2);
        slab(t, LIGHT);
        /* a dimple in the middle of it, to take hold of */
        box dimple = {t.x + t.w / 2 - 3, t.y + t.h / 2 - 3, 6, 6};
        if (t.w >= 10 && t.h >= 10) {
            fill(inset(dimple, 1), DARK);
            fill((box){dimple.x + 1, dimple.y, 4, 1}, BLACK);
            fill((box){dimple.x, dimple.y + 1, 1, 4}, BLACK);
            fill((box){dimple.x + 1, dimple.y + 5, 4, 1}, WHITE);
            fill((box){dimple.x + 5, dimple.y + 1, 1, 4}, WHITE);
        }
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box knob = inset(vertical ? (box){r.x, p->a, r.w, p->b} : (box){p->a, r.y, p->b, r.h}, 2);
    hollow(r, LIGHT);
    check(inset(r, 2));
    slab(knob, has(p, PRESS_STATE_HELD) ? WHITE : LIGHT);
    if (vertical) fill((box){knob.x + 3, knob.y + knob.h / 2, knob.w - 6, 1}, DARK);
    else fill((box){knob.x + knob.w / 2, knob.y + 3, 1, knob.h - 6}, DARK);
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4});
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    box b = {r.x, r.y + (front ? 0 : 2), r.w, r.h + 2 - (front ? 0 : 2)};
    slab(b, front ? LIGHT : DARK);
    words(p->text, r.x + 10, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), front ? BLACK : WHITE);
    if (front) {
        /* the chosen tab runs into the pane below it */
        fill((box){r.x + 1, r.y + r.h - 2, r.w - 3, 4}, LIGHT);
        if (has(p, PRESS_STATE_FOCUSED)) ring((box){r.x + 4, r.y + 3, r.w - 8, r.h - 6});
    }
}

static void popup(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    if (held) fill(r, WHITE);
    else slab(r, LIGHT);
    words(p->text, r.x + 8, r.y + (r.h - line_height()) / 2, BLACK);
    /* a little slab at its end: the sign of a list that pops up */
    box sign = {r.x + r.w - 17, r.y + r.h / 2 - 3, 10, 6};
    slab(sign, LIGHT);
    if (has(p, PRESS_STATE_FOCUSED)) ring(inset(r, 3));
}

/* A window's frame: a black line round it and a black bar across its top,
 * with the title in white in the middle. Behind others the bar is grey
 * and the title black. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height();
    box bar = {r.x + 1, r.y + 1, r.w - 2, p->a - 2};
    fill(r, BLACK);
    fill(bar, active ? BLACK : LIGHT);
    edge(bar, active ? DARK : WHITE, active ? BLACK : DARK);
    /* along the bottom, a bar to make the window larger by: its ends are for both ways at once */
    box foot = {r.x + 1, r.y + r.h - 8, r.w - 2, 7};
    slab(foot, LIGHT);
    for (int end = 0; end < 2; end++) {
        int x = end ? foot.x + foot.w - 28 : foot.x + 27;
        fill((box){x, foot.y + 1, 1, foot.h - 2}, DARK);
        fill((box){x + 1, foot.y + 1, 1, foot.h - 2}, WHITE);
    }
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, bar.y + (bar.h - line) / 2, active ? WHITE : BLACK);
}

/* The buttons of the bar: at the left, put away (a little window of its
 * own on it); at the right, close (a cross). */
static void frame_button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    if (held) fill(r, WHITE);
    else slab(r, LIGHT);
    box in = inset(r, 4);
    if (p->a == PRESS_FRAME_CLOSE) {
        for (int i = 0; i < in.w; i++) {
            fill((box){in.x + i, in.y + i, 2, 1}, BLACK);
            fill((box){in.x + in.w - 1 - i, in.y + i, 2, 1}, BLACK);
        }
    } else if (p->a == PRESS_FRAME_MINIMIZE) {
        outline(in, BLACK);
        fill((box){in.x, in.y, in.w, 2}, BLACK);
    } else {
        outline(in, BLACK);
        outline(inset(in, 2), BLACK);
    }
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, LIGHT); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL: hollow(r, p->a == 1 ? LIGHT : WHITE); break; /* a list is of the window's grey, and what is chosen in it white */
    case PRESS_PART_SELECT: select_in(r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: ring(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 2);
        hollow(r, LIGHT);
        if (has(p, PRESS_STATE_BUSY)) {
            for (int y = in.y; y < in.y + in.h; y++)
                for (int x = in.x; x < in.x + in.w; x++)
                    px(x, y, ((x + y - p->phase) % 8 + 8) % 8 < 4 ? DARK : LIGHT);
        } else {
            fill((box){in.x, in.y, (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1)), in.h}, DARK);
        }
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: fill(r, LIGHT); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE: slab(r, LIGHT); break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        fill(r, LIGHT);
        fill((box){r.x, r.y, r.w, 1}, WHITE);
        fill((box){r.x, r.y + r.h - 2, r.w, 1}, DARK);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) fill((box){r.x, r.y + 1, r.w, r.h - 2}, WHITE);
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, has(p, PRESS_STATE_ENABLED) ? BLACK : DARK);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        edge(f, WHITE, BLACK);
        fill((box){f.x + f.w, f.y + p->a, p->a, f.h}, with_alpha(BLACK, 0.5f));
        fill((box){f.x + p->a, f.y + f.h, f.w - p->a, p->a}, with_alpha(BLACK, 0.5f));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x, r.y + r.h / 2 - 1, r.w, 1}, DARK);
        fill((box){r.x, r.y + r.h / 2, r.w, 1}, WHITE);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, LIGHT);
        edge(r, DARK, WHITE);
        break;
    case PRESS_PART_HEADER: fill(r, LIGHT); break;
    case PRESS_PART_HEADER_CELL: slab(r, LIGHT); break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 22;
    m->button_min_width = 64;
    m->button_pad = 12;
    m->default_ring = 0; /* the one Return presses is known by its sign, not its size */
    m->check_size = 15;
    m->check_gap = 6;
    m->radio_size = 15;
    m->radio_gap = 6;
    m->progress_height = 14;
    m->slider_thick = 18;
    m->slider_knob = 15;
    m->scroll_bar = 18;
    m->popup_height = 21;
    m->menu_item_height = 18;
    m->menu_shadow = 2;
    /* a frame of its own, with its menus in it */
    m->frame_title = 23;
    m->frame_border = 1;
    m->frame_button_width = 15;
    m->frame_button_height = 15;
    m->frame_button_gap = 3;
    m->frame_button_inset = 4;
    m->frame_button_rise = 0;
    m->frame_left = 0x2;  /* put away */
    m->frame_right = 0x1; /* close */
    m->menu_in_window = 1;
    m->frame_foot = 9;
    m->scroll_arrows = 1;
}

static void palette(press_palette *p) {
    p->window = LIGHT;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = LIGHT;
    p->menu_text = BLACK;
    p->list = LIGHT;
    p->list_ink = BLACK;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"light", &LIGHT},
    {"dark", &DARK_SAID},
    {"white", &WHITE},
    {"black", &BLACK},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "eightynine", metrics, palette, draw, 0, NULL, colours};

const press_style *press_style_entry(void) {
    return &style;
}
