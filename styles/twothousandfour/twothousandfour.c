/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Two Thousand and Four: a style for press in the manner of the free
 * desktop of the autumn of 2004, which was warm where others were cool:
 * windows the colour of parchment, title bars of tan shading to brown,
 * and orange for what is selected and for work done. Its shapes are
 * plain: thin dark edges, corners barely rounded, faces shaded a little
 * from top to bottom. Every pixel of it is drawn here; nothing is copied
 * from anywhere.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=twothousandfour ./examples/c/controls
 */
#include "smooth.h"

#include <string.h>

COLOUR(PARCHMENT, 0xefebe7); /* what windows are made of */
FOLLOWER(EDGE, 0x8d8072);    /* round a button or a field */
#define EDGE follows(EDGE, PARCHMENT)
FOLLOWER(EDGE_DEEP, 0x5e5246);
#define EDGE_DEEP follows(EDGE_DEEP, PARCHMENT)
FOLLOWER(FACE_TOP, 0xfbf9f7);
#define FACE_TOP follows(FACE_TOP, PARCHMENT)
FOLLOWER(FACE_FOOT, 0xe1dad1);
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

/* A face: shaded from top to bottom in a thin dark edge, its corners just off. */
static void face(box r, int held, int hot) {
    if (held) round_box(r, 2.5f, like(PARCHMENT, 0xd3ccc2), like(PARCHMENT, 0xe3ddd5), EDGE_DEEP);
    else if (hot) round_box(r, 2.5f, WHITE, like(PARCHMENT, 0xece6de), EDGE);
    else round_box(r, 2.5f, FACE_TOP, FACE_FOOT, EDGE);
    if (!held) fill_over((box){r.x + 2, r.y + 1, r.w - 4, 1}, with_alpha(WHITE, 0.8f));
}

static void dotted(box r) {
    for (int x = r.x; x < r.x + r.w; x++) {
        if ((x + r.y) % 2 == 0) px(x, r.y, EDGE_DEEP);
        if ((x + r.y + r.h - 1) % 2 == 0) px(x, r.y + r.h - 1, EDGE_DEEP);
    }
    for (int y = r.y; y < r.y + r.h; y++) {
        if ((y + r.x) % 2 == 0) px(r.x, y, EDGE_DEEP);
        if ((y + r.x + r.w - 1) % 2 == 0) px(r.x + r.w - 1, y, EDGE_DEEP);
    }
}

static void label(const press_part *p, int x, box r) {
    int line = line_height();
    words(p->text, x, r.y + (r.h - line) / 2, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted((box){x - 2, r.y + (r.h - line) / 2 - 1, text_width(p->text) + 4, line + 2});
}

static void button(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    if (has(p, PRESS_STATE_DEFAULT)) {
        /* a darker line round the one Return presses */
        round_fill(r, 3.5f, EDGE_DEEP, EDGE_DEEP);
        face(inset(r, 1), held, has(p, PRESS_STATE_HOT));
    } else {
        face(r, held, has(p, PRESS_STATE_HOT));
    }
    words(p->text, r.x + (r.w - text_width(p->text)) / 2 + held, r.y + (r.h - line_height()) / 2 + held, BLACK);
    if (has(p, PRESS_STATE_FOCUSED))
        dotted(inset(r, 3));
}

static void checkbox(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    fill(b, has(p, PRESS_STATE_HELD) ? like(PARCHMENT, 0xddd6cd) : WHITE);
    outline(b, has(p, PRESS_STATE_HOT) ? EDGE_DEEP : EDGE);
    fill((box){b.x + 1, b.y + 1, b.w - 2, 1}, like(PARCHMENT, 0xddd8d2));
    if (has(p, PRESS_STATE_CHECKED)) {
        tick(b.x + 3, b.y + 3, 7, 2, 3, like(PARCHMENT, 0x2e2a26));
    }
    label(p, r.x + 13 + 6, r);
}

static void radio(const press_part *p, box r) {
    box b = {r.x, r.y + (r.h - 13) / 2, 13, 13};
    round_box(b, 6.5f, has(p, PRESS_STATE_HELD) ? like(PARCHMENT, 0xddd6cd) : like(PARCHMENT, 0xf2efec), WHITE, has(p, PRESS_STATE_HOT) ? EDGE_DEEP : EDGE);
    if (has(p, PRESS_STATE_CHECKED))
        round_fill((box){b.x + 4, b.y + 4, 5, 5}, 2.5f, like(PARCHMENT, 0x4a433c), like(PARCHMENT, 0x2e2a26));
    label(p, r.x + 13 + 6, r);
}

static void dim_in(box r) {
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint32_t c = at(x, y);
            px(x, y, mix(c, GREY, 0.7f * (1 - light(c))));
        }
}

static void scroll_bar(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL), live = has(p, PRESS_STATE_ENABLED);
    int thick = vertical ? r.w : r.h, length = vertical ? r.h : r.w;
    if (length < 2 * thick) return;
    fill(r, like(PARCHMENT, 0xd9d2c8));
    outline(r, like(PARCHMENT, 0xb5ab9e));
    for (int second = 0; second < 2; second++) {
        box b = vertical ? (box){r.x, second ? r.y + r.h - thick : r.y, thick, thick} : (box){second ? r.x + r.w - thick : r.x, r.y, thick, thick};
        face(b, live && p->a == second + 1, live && p->d == second + 1);
        arrow(b.x + b.w / 2, b.y + b.h / 2, 4, vertical ? second : 2 + second, live ? like(PARCHMENT, 0x3a342e) : GREY);
    }
    if (p->b >= 0) {
        box t = vertical ? (box){r.x, p->b, r.w, p->c} : (box){p->b, r.y, p->c, r.h};
        face(t, 0, p->d == 3);
        if (p->c >= 18) {
            for (int i = -3; i <= 3; i += 3) {
                if (vertical) {
                    fill((box){t.x + 4, t.y + t.h / 2 + i, t.w - 8, 1}, like(PARCHMENT, 0xa89d90));
                    fill((box){t.x + 4, t.y + t.h / 2 + i + 1, t.w - 8, 1}, WHITE);
                } else {
                    fill((box){t.x + t.w / 2 + i, t.y + 4, 1, t.h - 8}, like(PARCHMENT, 0xa89d90));
                    fill((box){t.x + t.w / 2 + i + 1, t.y + 4, 1, t.h - 8}, WHITE);
                }
            }
        }
    }
}

static void slider(const press_part *p, box r) {
    int vertical = has(p, PRESS_STATE_VERTICAL);
    box groove, knob;
    if (vertical) {
        groove = (box){r.x + r.w / 2 - 2, r.y + 2, 5, r.h - 4};
        knob = (box){r.x + 1, p->a, r.w - 2, p->b};
    } else {
        groove = (box){r.x + 2, r.y + r.h / 2 - 2, r.w - 4, 5};
        knob = (box){p->a, r.y + 1, p->b, r.h - 2};
    }
    round_box(groove, 2, like(PARCHMENT, 0xcfc7bc), like(PARCHMENT, 0xe6e0d8), EDGE);
    /* the part of the way gone is orange */
    if (vertical) fill((box){groove.x + 1, knob.y, groove.w - 2, groove.y + groove.h - 1 - knob.y}, ORANGE);
    else fill((box){groove.x + 1, groove.y + 1, knob.x - groove.x, groove.h - 2}, ORANGE);
    face(knob, has(p, PRESS_STATE_HELD), has(p, PRESS_STATE_HOT));
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

static void tab(const press_part *p, box r) {
    int front = has(p, PRESS_STATE_FRONT);
    int top = r.y + (front ? 0 : 3), foot = r.y + r.h - 1;
    box shape = {r.x, top, r.w, foot - top + 4};
    round_box(shape, 3, front ? like(PARCHMENT, 0xf8f6f3) : like(PARCHMENT, 0xe2dcd3), front ? PARCHMENT : like(PARCHMENT, 0xd6cfc5), EDGE);
    if (front) fill((box){r.x + 1, foot, r.w - 2, 4}, PARCHMENT);
    words(p->text, r.x + (r.w - text_width(p->text)) / 2, r.y + (r.h - line_height()) / 2 + (front ? 0 : 2), BLACK);
    if (front && has(p, PRESS_STATE_FOCUSED))
        dotted((box){r.x + 3, r.y + 3, r.w - 6, r.h - 6});
}

static void popup(const press_part *p, box r) {
    int held = has(p, PRESS_STATE_HELD);
    face(r, held, has(p, PRESS_STATE_HOT));
    words(p->text, r.x + 8, r.y + (r.h - line_height()) / 2, BLACK);
    /* a line, and an arrow down beyond it */
    fill((box){r.x + r.w - 20, r.y + 4, 1, r.h - 8}, like(PARCHMENT, 0xb5ab9e));
    fill((box){r.x + r.w - 19, r.y + 4, 1, r.h - 8}, WHITE);
    arrow(r.x + r.w - 10, r.y + r.h / 2, 4, 1, like(PARCHMENT, 0x3a342e));
    if (has(p, PRESS_STATE_FOCUSED)) dotted(inset(r, 3));
}

/* A window's frame: tan at the top of the title bar shading to brown at
 * its foot, the top corners rounded off, the title in the middle in a
 * darker brown with light under it. Behind others it is the colour of
 * the window, and the title grey. */
static void frame(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), line = line_height();
    uint32_t top = active ? like(ORANGE, 0xeed0a4) : like(PARCHMENT, 0xf3f0ec), foot = active ? like(ORANGE, 0xc98f58) : like(PARCHMENT, 0xdcd5cc), edge = active ? BROWN : like(PARCHMENT, 0x8f8579);
    round_fill((box){r.x, r.y, r.w, p->a + 8}, 6, edge, edge);
    fill((box){r.x, r.y + p->a, r.w, r.h - p->a}, edge);
    box shape = {r.x + 1, r.y + 1, r.w - 2, p->a + 8};
    for (int y = r.y + 1; y < r.y + p->a; y++) {
        uint32_t c = mix(top, foot, (float)(y - r.y - 1) / (p->a - 2));
        for (int x = r.x + 1; x < r.x + r.w - 1; x++) {
            float cover = covered(outside(shape, 5, x, y));
            if (cover > 0) blend(x, y, c, cover);
        }
    }
    fill_over((box){r.x + 5, r.y + 1, r.w - 10, 1}, with_alpha(WHITE, 0.55f));
    /* the edges are of the bar's own colour, a line of light inside the dark */
    fill((box){r.x + 1, r.y + p->a, p->b - 1, r.h - p->a - 1}, foot);
    fill((box){r.x + r.w - p->b, r.y + p->a, p->b - 1, r.h - p->a - 1}, foot);
    fill((box){r.x + 1, r.y + r.h - p->b, r.w - 2, p->b - 1}, foot);
    int tx = r.x + (r.w - text_width(p->text)) / 2, ty = r.y + (p->a - line) / 2;
    if (active) words(p->text, tx, ty + 1, with_alpha(WHITE, 0.5f));
    words(p->text, tx, ty, active ? like(BROWN, 0x4a2f17) : GREY);
}

/* The buttons at the right of the title bar: small, of the bar's own
 * colour, with a sign in dark brown; the one that closes is deeper. */
static void frame_button(const press_part *p, box r) {
    int active = has(p, PRESS_STATE_ENABLED), held = has(p, PRESS_STATE_HELD), hot = has(p, PRESS_STATE_HOT);
    uint32_t top = !active ? like(PARCHMENT, 0xf6f3ef) : p->a == PRESS_FRAME_CLOSE ? like(ORANGE, 0xf0b070) : like(ORANGE, 0xf6dfbd);
    uint32_t foot = !active ? like(PARCHMENT, 0xddd6cd) : p->a == PRESS_FRAME_CLOSE ? like(ORANGE, 0xc8702a) : like(ORANGE, 0xd29a62);
    if (hot) top = mix(top, WHITE, 0.5f);
    if (held) round_box(r, 3, foot, top, active ? BROWN : GREY);
    else round_box(r, 3, top, foot, active ? BROWN : GREY);
    uint32_t ink = active ? like(PARCHMENT, 0x3c2410) : GREY;
    int cx = r.x + r.w / 2, cy = r.y + r.h / 2;
    switch (p->a) {
    case PRESS_FRAME_CLOSE:
        cross(cx, cy, 3, 0, 2, ink);
        break;
    case PRESS_FRAME_MINIMIZE: fill((box){cx - 3, cy + 2, 7, 2}, ink); break;
    default:
        if (has(p, PRESS_STATE_CHECKED)) {
            outline((box){cx - 2, cy - 2, 5, 5}, ink);
        } else {
            outline((box){cx - 3, cy - 3, 8, 7}, ink);
            fill((box){cx - 3, cy - 3, 8, 2}, ink);
        }
        break;
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
        fill(r, WHITE);
        outline(r, has(p, PRESS_STATE_FOCUSED) ? ORANGE_DEEP : EDGE);
        fill((box){r.x + 1, r.y + 1, r.w - 2, 1}, like(PARCHMENT, 0xdcd6cf));
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
        fill(r, PARCHMENT);
        outline(r, EDGE);
        fill((box){r.x + 1, r.y + 1, r.w - 2, 1}, WHITE);
        fill((box){r.x + 1, r.y + 1, 1, r.h - 2}, WHITE);
        break;
    case PRESS_PART_POPUP: popup(p, r); break;
    case PRESS_PART_MENU_BAR:
        gradient(r, like(PARCHMENT, 0xf6f3f0), like(PARCHMENT, 0xe6e0d9));
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, like(PARCHMENT, 0xb5ab9e));
        break;
    case PRESS_PART_MENU_TITLE: {
        int lit = has(p, PRESS_STATE_HELD) && has(p, PRESS_STATE_ENABLED);
        if (lit) {
            gradient((box){r.x, r.y + 1, r.w, r.h - 2}, like(ORANGE, 0xee9a45), ORANGE);
            outline((box){r.x, r.y + 1, r.w, r.h - 2}, ORANGE_DEEP);
        }
        words(p->text, r.x + 9, r.y + (r.h - line_height()) / 2, lit ? WHITE : has(p, PRESS_STATE_ENABLED) ? BLACK : GREY);
        break;
    }
    case PRESS_PART_MENU_FRAME: {
        box f = {r.x, r.y, r.w - p->a, r.h - p->a};
        outline(f, like(PARCHMENT, 0x6f6457));
        fill((box){f.x + f.w, f.y + p->a, p->a, f.h}, with_alpha(BLACK, 0.25f));
        fill((box){f.x + p->a, f.y + f.h, f.w - p->a, p->a}, with_alpha(BLACK, 0.25f));
        break;
    }
    case PRESS_PART_MENU_SEPARATOR:
        fill((box){r.x + 2, r.y + r.h / 2 - 1, r.w - 4, 1}, like(PARCHMENT, 0xc4bbb0));
        fill((box){r.x + 2, r.y + r.h / 2, r.w - 4, 1}, WHITE);
        break;
    case PRESS_PART_VIEW_FRAME:
        fill(r, PARCHMENT);
        outline(r, EDGE);
        break;
    case PRESS_PART_HEADER:
        gradient(r, FACE_TOP, FACE_FOOT);
        fill((box){r.x, r.y + r.h - 1, r.w, 1}, EDGE);
        break;
    case PRESS_PART_HEADER_CELL:
        if (has(p, PRESS_STATE_HOT)) gradient((box){r.x, r.y, r.w, r.h - 1}, WHITE, like(PARCHMENT, 0xece6de));
        fill((box){r.x + r.w - 2, r.y + 2, 1, r.h - 5}, like(PARCHMENT, 0xb5ab9e));
        fill((box){r.x + r.w - 1, r.y + 2, 1, r.h - 5}, WHITE);
        break;
    case PRESS_PART_FRAME: frame(p, r); break;
    case PRESS_PART_FRAME_BUTTON: frame_button(p, r); break;
    default: return 0;
    }
    return 1;
}

static void metrics(press_metrics *m) {
    m->button_height = 26;
    m->button_min_width = 76;
    m->button_pad = 12;
    m->default_ring = 0;
    m->check_size = 13;
    m->check_gap = 6;
    m->radio_size = 13;
    m->radio_gap = 6;
    m->field_inset_x = 5;
    m->field_inset_y = 5;
    m->progress_height = 18;
    m->slider_thick = 18;
    m->scroll_bar = 15;
    m->slider_knob = 13;
    m->tab_height = 24;
    m->popup_height = 26;
    m->row_height = 19;
    m->menu_bar_height = 22;
    m->menu_item_height = 21;
    m->menu_shadow = 2;
    /* a frame of its own, with its menus in it */
    m->frame_title = 23;
    m->frame_border = 4;
    m->frame_button_width = 16;
    m->frame_button_height = 16;
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
    p->menu = like(PARCHMENT, 0xf7f4f1);
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
