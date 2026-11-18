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
#define PRESS_STYLE_ABI 7

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
    /* from version 2 */
    int32_t scroll_bar;       /* a scroll bar's thickness, and the length of its arrow boxes */
    int32_t slider_knob;      /* the length of a slider's knob along its travel */
    int32_t tab_height;
    int32_t popup_height;
    int32_t row_height;       /* a row of a list, a table or a tree */
    int32_t menu_bar_height;  /* a window's menu bar, the rule under it included */
    int32_t menu_item_height;
    int32_t menu_shadow;      /* room kept right of a menu and below it for its shadow */
    /* from version 3: a window's own frame. With frame_title nothing, the
     * frame is left to the desktop the program is on. */
    int32_t frame_title;      /* height of the title bar, its top edge included */
    int32_t frame_border;     /* thickness of the frame along the sides and the bottom */
    int32_t frame_button_width;
    int32_t frame_button_height;
    int32_t frame_button_gap;   /* between two buttons */
    int32_t frame_button_inset; /* from the side of the frame to the first button */
    int32_t frame_button_rise;  /* how far below the middle of the title bar they sit */
    int32_t frame_left;       /* the buttons at the left of the title bar and at the right: */
    int32_t frame_right;      /* four bits each, the one nearest the side of the frame lowest.
                                 PRESS_FRAME_CLOSE, _MINIMIZE, _ZOOM: 0x231 at the right is
                                 minimize, zoom, close as read from the left */
    int32_t menu_in_window;   /* not 0: the menu bar stays in the window on every desktop */
    /* from version 4 */
    int32_t frame_tab;        /* not 0: the title bar is a tab at the left of the window's top,
                                 as wide as its buttons and the title with this much room
                                 either side of it; the rest of that strip is left see-through */
    /* from version 5 */
    int32_t shortcut_form;    /* how a menu's shortcuts are written: 0 the command key's sign
                                 and the letter, 1 Ctrl+ and the letter, 2 a caret and the letter */
    int32_t frame_foot;       /* thickness of the frame along the bottom, if not as along the sides */
    int32_t frame_double;     /* two clicks on a title bar: 0 zoom, 1 roll the window up, 2 put it away */
    int32_t scroll_arrows;    /* 0 an arrow at each end of a scroll bar; 1 both at the far end */
    int32_t tab_align;        /* 0 tabs from the left of a tab view; 1 in the middle */
    int32_t menu_border;      /* thickness of a menu's edge, inside which its items lie */
} press_metrics;

/* The buttons of a title bar. */
enum {
    PRESS_FRAME_CLOSE = 1,
    PRESS_FRAME_MINIMIZE = 2,
    PRESS_FRAME_ZOOM = 3,
    PRESS_FRAME_ROLL = 4,     /* roll the window up into its title bar, or let it down */
    PRESS_FRAME_MENU = 5      /* show the window's own menu; two clicks on it close the window */
};

/* Colours, as 0xAARRGGBB. The engine draws words and small marks itself,
 * in these. */
typedef struct press_palette {
    uint32_t window;          /* what a window is made of */
    uint32_t text;            /* words written straight on a window */
    uint32_t paper;           /* text fields and lists */
    uint32_t ink;
    uint32_t menu;            /* a menu bar and its menus */
    uint32_t menu_text;
    /* from version 5; any left 0 follows the one it is like */
    uint32_t list;            /* lists, tables and trees, where not on the paper */
    uint32_t list_ink;
    uint32_t pane;            /* what the pages of a tab view lie on, where not the window's */
    /* from version 6 */
    uint32_t tip;             /* a word of help beside the pointer, where not on the paper */
    uint32_t tip_text;
    /* from version 7 */
    uint32_t pointer;         /* the pointer's arrow, where not black */
    uint32_t pointer_edge;    /* and the edge round it, where not white */
} press_palette;

/* The parts a window is drawn from. */
enum {
    PRESS_PART_BACKGROUND = 1,      /* the whole window, before anything else */
    PRESS_PART_BUTTON = 2,          /* with its label. DEFAULT, HELD, FOCUSED, HOT. a: counts up
                                       twenty times a second for a style that wants PULSE */
    PRESS_PART_CHECKBOX = 3,        /* with its label. CHECKED, HELD, FOCUSED */
    PRESS_PART_RADIO = 4,           /* with its label. CHECKED, HELD, FOCUSED */
    PRESS_PART_WELL = 5,            /* the box round a text field or list, and its paper. FOCUSED.
                                       a: 0 a line of text, 1 a list, 2 an area of text */
    PRESS_PART_SELECT = 6,          /* show what is already drawn there as selected.
                                       a: 0 a row of a list, 1 text, 2 an item of a menu */
    PRESS_PART_DIM = 7,             /* show what is already drawn there as out of use */
    PRESS_PART_FOCUS = 8,           /* a ring: this has the keyboard */
    PRESS_PART_SLIDER = 9,          /* VERTICAL, FOCUSED, HELD. a: where the knob starts; b: its length */
    PRESS_PART_PROGRESS = 10,       /* BUSY, or a of b done */
    PRESS_PART_SCROLL_BAR = 11,     /* VERTICAL; ENABLED if there is anywhere to go. a: 1 or 2 while
                                       the first or second arrow is held; b: where the sliding box
                                       starts, or -1 for none; c: its length; d: what the pointer is
                                       over, for a style that wants HOVER: 1 and 2 the arrows, 3 the
                                       box. The arrow boxes are as long as the bar is thick, one at
                                       each end, or both at the far end if scroll_arrows is 1 */
    PRESS_PART_SCROLL_CORNER = 12,  /* where two scroll bars meet */
    PRESS_PART_TAB = 13,            /* with its title. FRONT for the chosen one, which is drawn
                                       after the pane; FOCUSED; HOT. a: 1 the first of the row,
                                       2 the last, 3 the only one */
    PRESS_PART_TAB_PANE = 14,       /* the framed area the pages show in */
    PRESS_PART_POPUP = 15,          /* a pop-up button and its choice. FOCUSED; HELD while its list is up */
    PRESS_PART_MENU_BAR = 16,       /* across the top of a window, before its titles */
    PRESS_PART_MENU_TITLE = 17,     /* one title. HELD while its menu is down; ENABLED; HOT */
    PRESS_PART_MENU_FRAME = 18,     /* a menu's outline and shadow, on a picture of their own: the
                                       items are b pixels inside (menu_border), and the last a columns and rows
                                       (menu_shadow of them) are the shadow's, see-through until
                                       drawn on. A pixel's top byte says how solid it is, so a
                                       shadow can be soft */
    PRESS_PART_MENU_SEPARATOR = 19, /* a dividing line, in the room of one item */
    PRESS_PART_VIEW_FRAME = 20,     /* the box round a scroll view */
    PRESS_PART_HEADER = 21,         /* the row of headings over a table, before the headings */
    PRESS_PART_HEADER_CELL = 22,    /* one heading, before its words. HOT */
    PRESS_PART_FRAME = 23,          /* the frame of a window that draws its own: the title bar with
                                       the title, and the edges. The rectangle is the whole window,
                                       see-through until drawn on (leave its corners so to round
                                       them); the content is laid over the middle afterwards.
                                       a: the title bar's height; b: the edges' thickness;
                                       c: the title bar's width (the window's, unless a tab).
                                       ENABLED for the window in front; CHECKED for one made as
                                       large as it can be */
    PRESS_PART_FRAME_BUTTON = 24,   /* a button in a title bar. a: PRESS_FRAME_CLOSE and so on.
                                       HOT, HELD; ENABLED and CHECKED as for the frame */
    /* from version 5: small marks the engine used to draw itself, and
     * still does for a style that returns 0 for them. Draw them in ink. */
    PRESS_PART_MENU_CHECK = 25,     /* beside a menu's item that is switched on: the room at its left */
    PRESS_PART_MENU_ARROW = 26,     /* at the right of an item that opens another menu: the item's rectangle */
    PRESS_PART_SORT_ARROW = 27,     /* in a table's heading the rows are in the order of: its middle
                                       is at (a, b). CHECKED for smallest first */
    PRESS_PART_TREE_FOLD = 28,      /* before a row of a tree with rows under it: its middle is at
                                       (a, b). CHECKED when they are showing */
    PRESS_PART_CARET = 29,          /* where typing goes: one pixel wide, as tall as the line */
    /* from version 6. A style that returns 0 for these has them drawn
     * plainly in its own colours: a line cut into the ground they lie on. */
    PRESS_PART_GROUP_BOX = 30,      /* a box round things that belong together, with its title:
                                       before what is in it. c: the colour it lies on; its words
                                       are in ink */
    PRESS_PART_TOOLBAR = 31,        /* a bar of tools, before the tools. c as for a box. The tools
                                       are buttons, which the engine draws with no edge until the
                                       pointer is over them or down on them */
    PRESS_PART_STATUS_BAR = 32,     /* a bar of news along the foot of a window. c as for a box */
    PRESS_PART_STEPPER = 34,        /* the two arrows of a stepper, one over the other: up, then
                                     * down. a: 1 while the lower is held, 2 the upper; d: likewise
                                     * the one under the pointer. c as for a box. Left undrawn, it
                                     * is two of the style's buttons with arrows on them */
    PRESS_PART_COMBO_BUTTON = 35,   /* the button at the end of a combo box. HELD while its list is
                                     * down; HOT. Left undrawn, it is a button with an arrow on it */
    PRESS_PART_STATUS_PANEL = 36,   /* one cell of a status bar that keeps each thing it says in a
                                     * cell of its own. a: which, from 0. c as for a box */
    PRESS_PART_TOOLTIP = 33         /* a word of help beside the pointer, with its words. c: its
                                       ground, and ink its words' colour, both from the palette */
};

/* Two more things are drawn with parts there already: a stepper's pair of
 * arrows is a scroll bar lying on its side with nothing to slide (b is -1),
 * as long as two of its arrow boxes; and a combo box is a well with a small
 * button over its end, on which the engine draws an arrow. */

/* States, added together in press_part.state. */
enum {
    PRESS_STATE_ENABLED = 1,
    PRESS_STATE_HELD = 2,      /* the pointer is down on it, or its menu is showing */
    PRESS_STATE_FOCUSED = 4,   /* it has the keyboard */
    PRESS_STATE_DEFAULT = 8,   /* Return presses it */
    PRESS_STATE_CHECKED = 16,
    PRESS_STATE_VERTICAL = 32,
    PRESS_STATE_FRONT = 64,
    PRESS_STATE_BUSY = 128,
    PRESS_STATE_HOT = 256      /* the pointer is over it: only for a style that wants HOVER */
};

/* What a style would have the engine do for it, added together in
 * press_style.wants. */
enum {
    PRESS_WANTS_HOVER = 1,     /* buttons and the like are drawn again as the pointer comes
                                  and goes, with HOT set */
    PRESS_WANTS_PULSE = 2      /* a default button is drawn again twenty times a second */
};

/* The engine's own families, which a style may have written in one of the
 * machine's typefaces instead. */
enum {
    PRESS_FACE_INTERFACE = 0,  /* menus, buttons, labels: nearly everything */
    PRESS_FACE_SMALL = 1,
    PRESS_FACE_SMALL_BOLD = 2,
    PRESS_FACE_READING = 3,    /* long text */
    PRESS_FACE_FIXED = 4,      /* every letter the same width */
    PRESS_FACE_TITLE = 5,      /* large and heavy */
    PRESS_FACE_FRAME = 6       /* from version 5: the title of a window's own frame */
};

/* A typeface of the machine's, drawn smooth from its outlines. */
typedef struct press_type {
    char family[128];          /* as fontconfig knows it; several, with commas between, are
                                  tried in turn, and the nearest thing is used if none is here */
    int32_t size;              /* in pixels */
    int32_t bold;
    int32_t crisp;             /* from version 5: not 0 for whole pixels, no greys at the edges */
} press_type;

/* A family may also be one of the engine's own, to stand in the place of
 * another: "press:cicero", "press:agate", "press:agate-bold",
 * "press:brevier", "press:pica" (fixed width), "press:elite". Its size is
 * then its own. */

/* One part to draw. */
typedef struct press_part {
    uint32_t kind;
    uint32_t state;
    int32_t x, y, width, height;
    int32_t phase;            /* counts up as time passes, for anything that moves */
    int32_t a, b, c;          /* what these mean depends on the kind */
    const char *text;         /* its words, UTF-8; never NULL */
    /* from version 5 */
    int32_t d;
    uint32_t ink;             /* the colour the engine would have drawn a small mark in */
} press_part;

/* What to draw on. pixels[y * width + x] is 0xAARRGGBB, not multiplied by
 * its alpha. The three functions
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
    /* from version 2 */
    uint32_t wants;           /* PRESS_WANTS_... */
    /* Fill in the typeface to use in place of a family and return 1, or
     * return 0 to keep the family, in its own pixels. */
    int32_t (*typeface)(uint32_t face, press_type *out);
} press_style;

/* The one function a style library exports. What it returns must stay
 * where it is for as long as the program runs. */
const press_style *press_style_entry(void);

#ifdef __cplusplus
}
#endif

#endif
