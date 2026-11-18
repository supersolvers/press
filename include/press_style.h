/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Writing a style for press: a look for the interface, in a shared library
 * of its own.
 *
 * The engine knows what a button is; a style knows what one looks like. A
 * style says three things: its sizes, its colours, and how each part of a
 * window is drawn. Whatever it does not draw is drawn in black and white,
 * so a style can be written a part at a time.
 *
 * Build a shared library named libpress_style_NAME.so that exports
 * press_style_entry, and put it in one of
 *
 *     a folder named in PRESS_STYLE_PATH
 *     styles/ beside libpress.so, or beside it
 *     ~/.local/lib/press/styles
 *     /usr/local/lib/press/styles, /usr/lib/press/styles
 *
 * A program wears it when PRESS_STYLE=NAME (or the path of the library) is
 * in its environment; every program does when the first line of
 * ~/.config/press/style is NAME.
 *
 * This file is written by hand, and is the whole of the interface.
 */
#ifndef PRESS_STYLE_H
#define PRESS_STYLE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The version of this interface, for press_style.abi. */
#define PRESS_STYLE_ABI 1

/* Sizes, in pixels. */
typedef struct press_metrics {
    int32_t button_height;
    int32_t button_min_width;
    int32_t button_pad;       /* room either side of a button's label */
    int32_t default_ring;     /* how much larger all round a default button is */
    int32_t check_size;
    int32_t check_gap;        /* between a checkbox and its label */
    int32_t radio_size;
    int32_t radio_gap;
    int32_t field_inset_x;    /* from a text field's edge to its text */
    int32_t field_inset_y;
    int32_t progress_height;
    int32_t slider_thick;     /* a slider's size across its travel */
} press_metrics;

/* Colours, as 0xAARRGGBB. The engine draws words and small marks itself,
 * in these. */
typedef struct press_palette {
    uint32_t window;          /* what a window is made of */
    uint32_t text;            /* words written straight on a window */
    uint32_t paper;           /* text fields and lists */
    uint32_t ink;
    uint32_t menu;            /* a menu bar and its menus */
    uint32_t menu_text;
} press_palette;

/* The parts a window is drawn from. */
enum {
    PRESS_PART_BACKGROUND = 1,      /* the whole window, before anything else */
    PRESS_PART_BUTTON = 2,          /* with its label. DEFAULT, HELD, FOCUSED */
    PRESS_PART_CHECKBOX = 3,        /* with its label. CHECKED, HELD, FOCUSED */
    PRESS_PART_RADIO = 4,           /* with its label. CHECKED, HELD, FOCUSED */
    PRESS_PART_WELL = 5,            /* the box round a text field or list, and its paper. FOCUSED */
    PRESS_PART_SELECT = 6,          /* show what is already drawn there as selected */
    PRESS_PART_DIM = 7,             /* show what is already drawn there as out of use */
    PRESS_PART_FOCUS = 8,           /* a ring: this has the keyboard */
    PRESS_PART_SLIDER = 9,          /* VERTICAL, FOCUSED, HELD. a: where the knob starts; b: its length */
    PRESS_PART_PROGRESS = 10,       /* BUSY, or a of b done */
    PRESS_PART_SCROLL_BAR = 11,     /* VERTICAL; ENABLED if there is anywhere to go. a: 1 or 2 while
                                       the first or second arrow is held; b: where the sliding box
                                       starts, or -1 for none; c: its length. The arrow boxes are
                                       as long as the bar is thick */
    PRESS_PART_SCROLL_CORNER = 12,  /* where two scroll bars meet */
    PRESS_PART_TAB = 13,            /* with its title. FRONT for the chosen one, which is drawn
                                       after the pane; FOCUSED */
    PRESS_PART_TAB_PANE = 14,       /* the framed area the pages show in */
    PRESS_PART_POPUP = 15,          /* a pop-up button and its choice. FOCUSED; HELD while its list is up */
    PRESS_PART_MENU_BAR = 16,       /* across the top of a window, before its titles */
    PRESS_PART_MENU_TITLE = 17,     /* one title. HELD while its menu is down; ENABLED */
    PRESS_PART_MENU_FRAME = 18,     /* a menu's outline and shadow, on a picture of their own: the
                                       items are one pixel inside, and the last column and row are
                                       the shadow's. Pixels set to 0 show the desktop */
    PRESS_PART_MENU_SEPARATOR = 19, /* a dividing line, in the room of one item */
    PRESS_PART_VIEW_FRAME = 20,     /* the box round a scroll view */
    PRESS_PART_HEADER = 21,         /* the row of headings over a table, before the headings */
    PRESS_PART_HEADER_CELL = 22     /* one heading, before its words */
};

/* States, added together in press_part.state. */
enum {
    PRESS_STATE_ENABLED = 1,
    PRESS_STATE_HELD = 2,      /* the pointer is down on it, or its menu is showing */
    PRESS_STATE_FOCUSED = 4,   /* it has the keyboard */
    PRESS_STATE_DEFAULT = 8,   /* Return presses it */
    PRESS_STATE_CHECKED = 16,
    PRESS_STATE_VERTICAL = 32,
    PRESS_STATE_FRONT = 64,
    PRESS_STATE_BUSY = 128
};

/* One part to draw. */
typedef struct press_part {
    uint32_t kind;
    uint32_t state;
    int32_t x, y, width, height;
    int32_t phase;            /* counts up as time passes, for anything that moves */
    int32_t a, b, c;          /* what these mean depends on the kind */
    const char *text;         /* its words, UTF-8; never NULL */
} press_part;

/* What to draw on. pixels[y * width + x] is 0xAARRGGBB. The three functions
 * are the engine's own hand for words, in the typeface the part is in; pass
 * them ctx. */
typedef struct press_surface {
    uint32_t *pixels;
    int32_t width, height;
    void *ctx;
    int32_t (*line_height)(void *ctx);
    int32_t (*text_width)(void *ctx, const char *text);
    /* the top of the line of words is at y */
    void (*draw_text)(void *ctx, const char *text, int32_t x, int32_t y, uint32_t color);
} press_surface;

/* A style. Any of the three functions may be NULL. */
typedef struct press_style {
    uint32_t abi;             /* PRESS_STYLE_ABI */
    const char *name;
    /* Both are handed the black-and-white values: change what differs. */
    void (*metrics)(press_metrics *metrics);
    void (*palette)(press_palette *palette);
    /* Draw the part and return 1, or return 0 to have it drawn in black
     * and white. Draw nothing outside the surface. */
    int32_t (*draw)(const press_part *part, const press_surface *on);
} press_style;

/* The one function a style library exports. What it returns must stay
 * where it is for as long as the program runs. */
const press_style *press_style_entry(void);

#ifdef __cplusplus
}
#endif

#endif
