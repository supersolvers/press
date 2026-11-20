/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Ninety-One: a style for press in the manner of the desktop of 1991 that
 * came in the ROM of a British computer. It is flat: pale grey windows,
 * every box a thin black line, and what is chosen shown by turning its
 * colours inside out. The window that has the keyboard is known by its
 * title bar, which goes from grey to cream. At the left of that bar are
 * a box that sends the window behind the others and a box that closes it;
 * at the right, one that makes it as large as it can be; and along the
 * foot runs a bar with a box at its end to pull the window larger by.
 * Every pixel of it is drawn here; nothing is copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=ninetyone ./examples/c/controls
 */
#include "smooth.h"

COLOUR(WHITE, 0xffffff);
COLOUR(PALE, 0xdddddd); /* what a window is made of */
FOLLOWER(GREY, 0xbbbbbb); /* a title bar that has not the keyboard */
#define GREY follows(GREY, PALE)
FOLLOWER(MID, 0x999999); /* the bed something slides in; words out of use */
#define MID follows(MID, PALE)
COLOUR(BLACK, 0x000000);
COLOUR(CREAM, 0xeeeebb); /* the title bar that has it */
COLOUR(RED, 0xdd0000);  /* where typing goes, and what is switched on */
COLOUR(GREEN, 0x00cc00);

/* A box of line on a ground. */
static void boxed(box r, uint32_t ground) {
    fill(r, ground);
    outline(r, BLACK);
}

/* Turn what is drawn there inside out: black for white, and each colour
 * for the one opposite it. */
static void opposite(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            px(x, y, at(x, y) ^ 0x00ffffffu);
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

/* A small picture from rows of letters: # in ink, anything else left. */
static void sign(const char *const *rows, int count, int x, int y, uint32_t ink) {
    for (int j = 0; j < count; j++)
        for (int i = 0; rows[j][i] != 0; i++)
            if (rows[j][i] == '#') px(x + i, y + j, ink);
}

static void button(const press_part *p, box r) {
    /* words in a box of line; the one Return presses has a second line
     * round it, with cream between the two */
    box face = r;
    if (has(p, PRESS_STATE_DEFAULT)) {
        boxed(r, CREAM);
        face = inset(r, 3);
    }
    boxed(face, PALE);
    words(p->text, face.x + (face.w - text_width(p->text)) / 2, face.y + (face.h - line_height()) / 2, BLACK);
    if (has(p, PRESS_STATE_HELD))
        opposite(inset(face, 1));
    if (has(p, PRESS_STATE_FOCUSED))
        ring(inset(face, 2));
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){x - 2, r.y + (r.h - line) / 2 - 1, text_width(p->text) + 4, line + 2});
}

static void checkbox(const press_part *p, box r) {
    /* a box, with a star in it when it is on */
    static const char *const star[9] = {
        "#...#...#",
        ".#..#..#.",
        "..#.#.#..",
        "...###...",
        "#########",
        "...###...",
        "..#.#.#..",
        ".#..#..#.",
        "#...#...#",
    };
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    boxed(b, WHITE);
    if (has(p, PRESS_STATE_CHECKED)) sign(star, 9, b.x + 2, b.y + 2, RED);
    if (has(p, PRESS_STATE_HELD)) opposite(inset(b, 1));
    label(p, r.x + 13 + 6, r);
}

static void radio(const press_part *p, box r) {
    static const char *const round[13] = {
        "....#####....",
        "..##.....##..",
        ".#.........#.",
        ".#.........#.",
        "#...........#",
        "#...........#",
        "#...........#",
        "#...........#",
        "#...........#",
        ".#.........#.",
        ".#.........#.",
        "..##.....##..",
        "....#####....",
    };
    static const char *const inside[11] = {
        "...#####...",
        ".#########.",
        ".#########.",
        "###########",
        "###########",
        "###########",
        "###########",
        "###########",
        ".#########.",
        ".#########.",
        "...#####...",
    };
    static const char *const dot[5] = {
        ".###.",
        "#####",
        "#####",
        "#####",
        ".###.",
    };
    int top = r.y + (r.h - 13) / 2, held = has(p, PRESS_STATE_HELD);
    sign(inside, 11, r.x + 1, top + 1, held ? BLACK : WHITE);
    sign(round, 13, r.x, top, BLACK);
    if (has(p, PRESS_STATE_CHECKED)) sign(dot, 5, r.x + 4, top + 4, RED);
    label(p, r.x + 13 + 6, r);
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (at(x, y) == BLACK) px(x, y, MID);
}

/* A scroll bar: a grey bed in a box of line, a box with an arrow at each
 * end, and a pale box that slides between them. */
static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    boxed(r, live ? MID : PALE);
    for (int second = 0; second < 2; second++) {
        int from = second ? (vertical ? r.y + r.h : r.x + r.w) - thick : (vertical ? r.y : r.x);
        box b = vertical ? (box){r.x, from, thick, thick} : (box){from, r.y, thick, thick};
        boxed(b, PALE);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 5, vertical ? second : 2 + second, live ? BLACK : MID);
        if (live && p->a == second + 1) opposite(inset(b, 1));
    }
    if (p->b >= 0) {
        /* its ends lie on the lines of the boxes either side of it */
        box t = vertical ? (box){r.x, p->b - 1, r.w, p->c + 2} : (box){p->b - 1, r.y, p->c + 2, r.h};
        boxed(t, PALE);
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box knob = vertical ? (box){r.x, p->a, r.w, p->b} : (box){p->a, r.y, p->b, r.h};
    /* a thin bed, and a box across it to move */
    box bed = vertical ? (box){r.x + r.w / 2 - 3, r.y, 7, r.h} : (box){r.x, r.y + r.h / 2 - 3, r.w, 7};
    boxed(bed, MID);
    /* green behind the box: how far it has come */
    if (vertical) fill((box){bed.x + 1, knob.y, bed.w - 2, bed.y + bed.h - 1 - knob.y}, GREEN);
    else fill((box){bed.x + 1, bed.y + 1, knob.x - bed.x - 1, bed.h - 2}, GREEN);
    boxed(knob, PALE);
    if (has(p, PRESS_STATE_HELD)) opposite(inset(knob, 1));
    if (has(p, PRESS_STATE_FOCUSED))
        ring((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4});
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    box b = {r.x, r.y + (front ? 0 : 2), r.w, r.h + 1 - (front ? 0 : 2)};
    boxed(b, front ? PALE : GREY);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), BLACK);
    if (front) {
        /* the chosen tab runs into the pane below it */
        fill((box){r.x + 1, r.y + r.h - 1, r.w - 2, 2}, PALE);
        if (has(p, PRESS_STATE_FOCUSED)) ring((box){r.x + 3, r.y + 3, r.w - 6, r.h - 6});
    }
}

/* The sign of a list that pops up: a little menu, its top row dark. */
static void menu_sign(box b, int held) {
    boxed(b, PALE);
    box m = {b.x + (b.w - 9) / 2, b.y + (b.h - 9) / 2, 9, 9};
    boxed(m, WHITE);
    fill((box){m.x + 1, m.y + 1, m.w - 2, 2}, BLACK);
    fill((box){m.x + 2, m.y + 4, m.w - 4, 1}, BLACK);
    fill((box){m.x + 2, m.y + 6, m.w - 4, 1}, BLACK);
    if (held) opposite(inset(b, 1));
}

static void popup(const press_part *p, box r) {
    /* its choice in a box of paper, and beside that the sign */
    box choice = {r.x, r.y, r.w - r.h - 3, r.h};
    boxed(choice, WHITE);
    words(p->text, r.x + 6, r.y + (r.h - line_height()) / 2, BLACK);
    menu_sign((box){r.x + r.w - r.h, r.y, r.h, r.h}, has(p, PRESS_STATE_HELD));
    if (has(p, PRESS_STATE_FOCUSED)) ring(inset(choice, 2));
}

/* A window's frame: a black line round it, and across the top a bar with
 * the title in the middle, cream for the window that has the keyboard and
 * grey for the others. Along the foot, a bar as of a window that shows
 * all there is to see, with a box at its right to pull the window larger. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height();
    box bar = {r.x, r.y, r.w, p->a};
    fill(r, BLACK);
    fill(inset(bar, 1), active ? CREAM : GREY);
    /* a line after the two boxes at the left, and one before the box at the right */
    fill((box){r.x + 2 * p->a - 2, r.y, 1, p->a}, BLACK);
    fill((box){r.x + r.w - p->a, r.y, 1, p->a}, BLACK);
    /* the title in the middle of what is left */
    int left = r.x + 2 * p->a - 1, room = r.w - 3 * p->a + 1;
    clip_to((box){left, r.y, room, p->a});
    words(p->text, left + (room - text_width(p->text)) / 2, r.y + (p->a - line) / 2, BLACK);
    clip_off();
    int foot = r.h - p->a - 1 > 12 ? 12 : 0;
    if (foot && !has(p, PRESS_STATE_CHECKED)) {
        box f = {r.x, r.y + r.h - foot - 1, r.w, foot + 1};
        boxed(f, MID);
        boxed((box){f.x, f.y, f.w - foot, f.h}, PALE);
        /* the box to pull by: a small square in the corner of a larger */
        box pull = {f.x + f.w - foot - 1, f.y, foot + 1, foot + 1};
        boxed(pull, PALE);
        outline((box){pull.x + 3, pull.y + 3, 7, 7}, BLACK);
        boxed((box){pull.x + 3, pull.y + 3, 4, 4}, GREY);
    } else if (foot) {
        fill((box){r.x + 1, r.y + r.h - foot, r.w - 2, foot - 1}, PALE);
    }
}

/* The boxes of the bar. */
static void frame_button(const press_part *p, box r) {
    static const char *const cross[9] = {
        "##.....##",
        "###...###",
        ".###.###.",
        "..#####..",
        "...###...",
        "..#####..",
        ".###.###.",
        "###...###",
        "##.....##",
    };
    fill(r, PALE);
    int cx = r.x + r.w / 2, cy = r.y + r.h / 2;
    switch (p->a) {
    case PRESS_FRAME_CLOSE: sign(cross, 9, cx - 4, cy - 4, BLACK); break;
    case PRESS_FRAME_MINIMIZE:
        /* behind the others: one square behind another */
        outline((box){cx - 5, cy - 5, 8, 8}, BLACK);
        boxed((box){cx - 2, cy - 2, 8, 8}, GREY);
        break;
    default:
        /* as large as it can be: a square, and the smaller one it was */
        outline((box){cx - 5, cy - 5, 11, 11}, BLACK);
        if (has(p, PRESS_STATE_CHECKED)) boxed((box){cx - 5, cy, 6, 6}, GREY);
        else boxed((box){cx - 5, cy - 5, 8, 8}, GREY);
        break;
    }
    if (has(p, PRESS_STATE_HELD)) opposite(r);
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, PALE); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL: boxed(r, WHITE); break;
    case PRESS_PART_SELECT: opposite(r); break;
    case PRESS_PART_DIM: dim_in(r); break;
    case PRESS_PART_FOCUS: ring(r); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 1);
        boxed(r, WHITE);
        if (has(p, PRESS_STATE_BUSY)) {
            for (int y = in.y; y < in.y + in.h; y++)
                for (int x = in.x; x < in.x + in.w; x++)
                    px(x, y, ((x + y - p->phase) % 8 + 8) % 8 < 4 ? GREEN : WHITE);
        } else {
            fill((box){in.x, in.y, (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1)), in.h}, GREEN);
        }
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER: boxed(r, PALE); break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE: boxed(r, PALE); break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_COMBO_BUTTON: menu_sign(r, has(p, PRESS_STATE_HELD)); break;
    case PRESS_PART_MENU_BAR:
        fill(r, PALE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_MENU_TITLE:
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, has(p, PRESS_STATE_ENABLED) ? BLACK : MID);
        if (has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED)) opposite((box){r.x, r.y, r.w, r.h - 1});
        break;
    case PRESS_PART_MENU_FRAME: outline((box){r.x, r.y, r.w - p->a, r.h - p->a}, BLACK); break;
    case PRESS_PART_MENU_SEPARATOR:
        for (int x = r.x + 1; x < r.x + r.w - 1; x += 2) px(x, r.y + r.h / 2, BLACK);
        break;
    case PRESS_PART_VIEW_FRAME: boxed(r, PALE); break;
    case PRESS_PART_HEADER:
        fill(r, GREY);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_HEADER_CELL: fill((box){r.x + r.w - 1, r.y, 1, r.h}, BLACK); break;
    case PRESS_PART_CARET:
        /* red, with a foot and a head to it */
        fill(r, RED);
        fill((box){r.x - 1, r.y, 3, 1}, RED);
        fill((box){r.x - 1, r.y + r.h - 1, 3, 1}, RED);
        break;
    case PRESS_PART_GROUP_BOX: {
        int top = p->text[0] ? line_height() / 2 : 0;
        outline((box){r.x, r.y + top, r.w, r.h - top}, BLACK);
        if (p->text[0]) {
            fill((box){r.x + 7, r.y, text_width(p->text) + 6, line_height()}, (uint32_t)p->c);
            words(p->text, r.x + 10, r.y, BLACK);
        }
        break;
    }
    case PRESS_PART_TOOLBAR: fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK); break;
    case PRESS_PART_STATUS_BAR: fill((box){r.x, r.y, r.w, 1}, BLACK); break;
    case PRESS_PART_STATUS_PANEL:
        if (p->a > 0) fill((box){r.x, r.y, 1, r.h}, BLACK);
        break;
    case PRESS_PART_TOOLTIP:
        boxed(r, (uint32_t)p->c);
        words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2, p->ink);
        break;
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
    m->default_ring = 3;
    m->check_size = 13;
    m->check_gap = 6;
    m->radio_size = 13;
    m->radio_gap = 6;
    m->progress_height = 14;
    m->slider_thick = 17;
    m->slider_knob = 11;
    m->scroll_bar = 17;
    m->popup_height = 21;
    m->menu_item_height = 18;
    m->menu_shadow = 0;
    m->menu_border = 1;
    /* a frame of its own; its menus are the screen's where a desktop has a bar for them */
    m->frame_title = 21;
    m->frame_border = 1;
    m->frame_foot = 13;
    m->frame_button_width = 19;
    m->frame_button_height = 19;
    m->frame_button_gap = 1;
    m->frame_button_inset = 1;
    m->frame_button_rise = 0;
    m->frame_left = 0x12;  /* behind the others, then close */
    m->frame_right = 0x3;  /* as large as it can be */
    m->menu_in_window = 0;
    m->ok_leads = 0; /* Cancel, then OK */
    m->shortcut_form = 2; /* a caret and the letter */
}

static void palette(press_palette *p) {
    p->window = PALE;
    p->text = BLACK;
    p->paper = WHITE;
    p->ink = BLACK;
    p->menu = WHITE;
    p->menu_text = BLACK;
    p->tip = CREAM;
    p->tip_text = BLACK;
    p->pointer = 0xff00bbffu; /* the arrow is pale blue, edged in dark */
    p->pointer_edge = 0xff004499u;
}

/* That machine wrote on its desktop in one face with every letter the
 * same width: the engine's own of that kind stands in for it. */
static int32_t typeface(uint32_t face, press_type *out) {
    if (face != PRESS_FACE_INTERFACE && face != PRESS_FACE_FRAME && face != PRESS_FACE_READING) return 0;
    const char *own = "press:pica";
    for (int i = 0; own[i] != 0 || (out->family[i] = 0); i++) out->family[i] = own[i];
    return 1;
}

/* What a theme may change, by these names. */
static const press_colour colours[] = {
    {"white", &WHITE},
    {"pale", &PALE},
    {"grey", &GREY_SAID},
    {"mid", &MID_SAID},
    {"black", &BLACK},
    {"cream", &CREAM},
    {"red", &RED},
    {"green", &GREEN},
    {NULL, NULL},
};

static const press_style style = {PRESS_STYLE_ABI, "ninetyone", metrics, palette, draw, 0, typeface, colours};

const press_style *press_style_entry(void) {
    return &style;
}
