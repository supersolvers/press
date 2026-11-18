/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Eighty-Five: a style for press in the manner of the home computer
 * workbench of 1985, which had four colours and made a great deal of
 * them: blue for the screen and what is on it, white for lines and words,
 * black, and orange. Nothing is shaded and nothing is rounded. What is
 * chosen or held is shown by turning each colour into its opposite: blue
 * to orange, white to black. Every pixel of it is drawn here; nothing is
 * copied from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=eightyfive ./examples/c/controls
 */
#include "smooth.h"

#define BLUE rgb(0x0055aa)
#define WHITE rgb(0xffffff)
#define BLACK rgb(0x000022)
#define ORANGE rgb(0xff8800)

/* Every colour to its opposite, as that machine did to show a thing chosen. */
static void opposite(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, c == BLUE ? ORANGE : c == ORANGE ? BLUE : c == WHITE ? BLACK : c == BLACK ? WHITE : c);
        }
}

/* Out of use: dotted over with the colour behind it, so that it is half there. */
static void ghost(box r) {
    uint32_t behind = at(r.x, r.y);
    if (behind != BLUE && behind != WHITE) behind = BLUE;
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if ((x + y) % 2 == 0) px(x, y, behind);
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, WHITE);
    if (has(p, PRESS_STATE_FOCUSED))
        outline((box){x - 3, r.y + (r.h - line) / 2 - 2, text_width(p->text) + 6, line + 4}, ORANGE);
}

static void button(const press_part *p, box r) {
    /* words in a box of line; two lines round the one Return presses */
    box face = r;
    fill(r, BLUE);
    if (has(p, PRESS_STATE_DEFAULT)) {
        outline(r, ORANGE);
        face = inset(r, 2);
    }
    fill(face, WHITE);
    outline(face, BLACK);
    words(p->text, face.x + (face.w - text_width(p->text)) / 2, face.y + (face.h - line_height()) / 2, BLUE);
    if (has(p, PRESS_STATE_HELD))
        opposite(inset(face, 1));
    if (has(p, PRESS_STATE_FOCUSED))
        outline((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4}, ORANGE);
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 12) / 2, 12, 12};
    fill(b, has(p, PRESS_STATE_HELD) ? ORANGE : BLUE);
    outline(b, WHITE);
    if (has(p, PRESS_STATE_CHECKED)) {
        /* a tick, drawn with a square pen */
        static const int rise[8] = {3, 4, 5, 4, 3, 2, 1, 0};
        for (int i = 0; i < 8; i++)
            fill((box){b.x + 2 + i, b.y + 2 + rise[i], 1, 3}, WHITE);
    }
    label(p, r.x + 12 + 6, r);
}

static void radio(const press_part *p, box r) {
    /* a lozenge, filled when it is the one chosen */
    int top = r.y + (r.h - 11) / 2, held = has(p, PRESS_STATE_HELD);
    for (int i = 0; i < 6; i++) {
        fill((box){r.x + 5 - i, top + i, 2 * i + 1, 1}, held ? ORANGE : BLUE);
        fill((box){r.x + 5 - i, top + 10 - i, 2 * i + 1, 1}, held ? ORANGE : BLUE);
        px(r.x + 5 - i, top + i, WHITE);
        px(r.x + 5 + i, top + i, WHITE);
        px(r.x + 5 - i, top + 10 - i, WHITE);
        px(r.x + 5 + i, top + 10 - i, WHITE);
    }
    if (has(p, PRESS_STATE_CHECKED))
        for (int i = 0; i < 3; i++) {
            fill((box){r.x + 5 - i, top + 3 + i, 2 * i + 1, 1}, WHITE);
            fill((box){r.x + 5 - i, top + 7 - i, 2 * i + 1, 1}, WHITE);
        }
    label(p, r.x + 11 + 6, r);
}

/* A box with something in it that slides: a scroll bar's or a slider's. */
static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, BLUE);
    outline(r, WHITE);
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        fill(b, WHITE);
        outline(b, BLACK);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 5, vertical ? second : 2 + second, BLUE);
        if (!live) ghost(inset(b, 1));
        else if (p->a == second + 1) opposite(inset(b, 1));
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x + 2, p->b + 1, r.w - 4, p->c - 2} : (box){p->b + 1, r.y + 2, p->c - 2, r.h - 4};
        fill(t, WHITE);
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box knob = vertical ? (box){r.x + 2, p->a, r.w - 4, p->b} : (box){p->a, r.y + 2, p->b, r.h - 4};
    fill(r, BLUE);
    outline(r, WHITE);
    fill(knob, has(p, PRESS_STATE_HELD) ? ORANGE : WHITE);
    if (has(p, PRESS_STATE_FOCUSED))
        outline((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4}, ORANGE);
}

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    box b = {r.x, r.y + (front ? 0 : 2), r.w, r.h - (front ? 0 : 2)};
    fill(b, front ? WHITE : BLUE);
    outline(b, WHITE);
    words(p->text, r.x + 10, r.y + (r.h - line_height()) / 2 + (front ? 0 : 1), front ? BLUE : WHITE);
    if (front && has(p, PRESS_STATE_FOCUSED))
        outline(inset(b, 2), ORANGE);
}

/* The title bar of a window: white, ruled across in blue either side of
 * the title, with boxes at its ends. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height();
    fill(r, WHITE);
    outline(r, BLACK);
    for (int y = r.y + 3; y < r.y + p->a - 3; y += 2)
        fill((box){r.x + 2, y, r.w - 4, 1}, BLUE);
    int wide = text_width(p->text), tx = r.x + 30;
    fill((box){tx - 4, r.y + 1, wide + 8, p->a - 2}, WHITE);
    words(p->text, tx, r.y + (p->a - line) / 2, BLUE);
    fill((box){r.x + 1, r.y + p->a - 1, r.w - 2, 1}, BLACK);
    if (!active) ghost((box){r.x + 1, r.y + 1, r.w - 2, p->a - 2});
}

static void frame_button(const press_part *p, box r) {
    /* cleared of the ruling, then: a dot in a box to close; two sheets,
     * one before the other, for the buttons that bring a window to the
     * front (here, zoom it) and send it to the back (here, put it away) */
    fill((box){r.x - 2, r.y - 1, r.w + 4, r.h + 2}, WHITE);
    outline(r, BLUE);
    if (p->a == PRESS_FRAME_CLOSE) {
        fill(inset(r, 3), BLACK);
        fill(inset(r, 5), ORANGE);
    } else {
        int zoom = p->a == PRESS_FRAME_ZOOM;
        box back = {r.x + 2, r.y + 2, r.w - 7, r.h - 6}, fore = {r.x + 5, r.y + 4, r.w - 7, r.h - 6};
        fill(back, zoom ? BLACK : WHITE);
        outline(back, BLACK);
        fill(fore, zoom ? WHITE : BLACK);
        outline(fore, BLACK);
    }
    if (has(p, PRESS_STATE_HELD)) opposite(inset(r, 1));
    if (!has(p, PRESS_STATE_ENABLED)) ghost(r);
}

static int32_t draw(const press_part *p, const press_surface *surface) {
    box r = {p->x, p->y, p->width, p->height};
    on = surface;
    switch (p->kind) {
    case PRESS_PART_BACKGROUND: fill(r, BLUE); break;
    case PRESS_PART_BUTTON: button(p, r); break;
    case PRESS_PART_CHECKBOX: checkbox(p, r); break;
    case PRESS_PART_RADIO: radio(p, r); break;
    case PRESS_PART_WELL:
        fill(r, BLUE);
        outline(inset(r, 1), WHITE);
        if (has(p, PRESS_STATE_FOCUSED)) outline(r, ORANGE);
        break;
    case PRESS_PART_SELECT: opposite(r); break;
    case PRESS_PART_DIM: ghost(r); break;
    case PRESS_PART_FOCUS: outline(r, ORANGE); break;
    case PRESS_PART_SLIDER: slider(p, r); break;
    case PRESS_PART_PROGRESS: {
        box in = inset(r, 2);
        fill(r, BLUE);
        outline(r, WHITE);
        if (has(p, PRESS_STATE_BUSY)) {
            for (int y = in.y; y < in.y + in.h; y++)
                for (int x = in.x; x < in.x + in.w; x++)
                    if (((x + y - p->phase) % 8 + 8) % 8 < 4) px(x, y, ORANGE);
        } else {
            fill((box){in.x, in.y, (int)((long long)in.w * p->a / (p->b > 0 ? p->b : 1)), in.h}, ORANGE);
        }
        break;
    }
    case PRESS_PART_SCROLL_BAR: scroll_bar(p, r); break;
    case PRESS_PART_SCROLL_CORNER:
        fill(r, BLUE);
        outline(r, WHITE);
        break;
    case PRESS_PART_TAB: tab(p, r); break;
    case PRESS_PART_TAB_PANE:
        fill(r, BLUE);
        outline(r, WHITE);
        break;
    case PRESS_PART_POPUP: {
        /* its choice in a box, and at the end of it the sign for more below */
        int line = line_height();
        fill(r, WHITE);
        outline(r, BLACK);
        words(p->text, r.x + 18, r.y + (r.h - line) / 2, BLUE);
        fill((box){r.x + r.w - r.h, r.y, 1, r.h}, BLACK);
        arrow(r.x + r.w - r.h / 2, r.y + r.h / 2, 5, 1, BLUE);
        if (has(p, PRESS_STATE_HELD)) opposite(inset(r, 1));
        if (has(p, PRESS_STATE_FOCUSED)) outline((box){r.x - 2, r.y - 2, r.w + 4, r.h + 4}, ORANGE);
        break;
    }
    case PRESS_PART_MENU_BAR:
        fill(r, WHITE);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, BLACK);
        break;
    case PRESS_PART_MENU_TITLE:
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, BLUE);
        if (!has(p, PRESS_STATE_ENABLED)) ghost(r);
        else if (has(p, PRESS_STATE_HELD)) opposite(r);
        break;
    case PRESS_PART_MENU_FRAME: {
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(f, BLACK);
        fill((box){f.x + f.w, f.y, p->a, f.h}, BLACK);
        fill((box){f.x, f.y + f.h, f.w + p->a, p->a}, BLACK);
        break;
    }
    case PRESS_PART_MENU_SEPARATOR:
        for (int x = r.x + 1; x < r.x + r.w - 1; x += 2) px(x, r.y + r.h / 2, BLACK);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, BLUE);
        outline(r, WHITE);
        break;
    case PRESS_PART_HEADER:
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, WHITE);
        break;
    case PRESS_PART_HEADER_CELL: fill((box){r.x + r.w - 1, r.y, 1, r.h}, WHITE); break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 20;
    m->default_ring = 2;
    m->radio_size = 11;
    m->progress_height = 12;
    m->slider_thick = 14;
    m->slider_knob = 12;
    m->popup_height = 19;
    m->menu_shadow = 1;
    /* a frame of its own; its menus are the screen's where a desktop has a bar for them */
    m->frame_title = 18;
    m->frame_border = 2;
    m->frame_button_width = 19;
    m->frame_button_height = 14;
    m->frame_button_gap = 3;
    m->frame_button_inset = 4;
    m->frame_button_rise = 0;
    m->frame_left = 0x1;
    m->frame_right = 0x32;
    m->menu_in_window = 0;
}

static void palette(press_palette *p) {
    /* words are white on the blue, in windows and in the things in them;
     * menus are the other way about */
    p->window = BLUE;
    p->text = WHITE;
    p->paper = BLUE;
    p->ink = WHITE;
    p->menu = WHITE;
    p->menu_text = BLUE;
}

/* That machine wrote in one face with every letter the same width: the
 * engine's own of that kind stands in for its heavy one, everywhere. */
static int32_t typeface(uint32_t face, press_type *out) {
    if (face != PRESS_FACE_INTERFACE && face != PRESS_FACE_FRAME && face != PRESS_FACE_READING) return 0;
    const char *own = "press:pica";
    for (int i = 0; own[i] != 0 || (out->family[i] = 0); i++) out->family[i] = own[i];
    return 1;
}

static const press_style style = {PRESS_STYLE_ABI, "eightyfive", metrics, palette, draw, 0, typeface};

const press_style *press_style_entry(void) {
    return &style;
}
