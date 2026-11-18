/* SPDX-License-Identifier: GPL-3.0-or-later */
/*
 * Two Thousand: a style for press in the manner of the office desktops of
 * the year 2000. They were those of the middle 1990s grown up: the same
 * bevels in a warmer grey, a deeper blue for what is selected, softer
 * edges where the light does not fall, and a typeface drawn smooth. So
 * this style is Ninety-Five told other colours, sizes and type.
 *
 *     make styles PROFILE=debug
 *     PRESS_STYLE=twothousand ./examples/c/controls
 */
#include "press_style.h"

#include <string.h>

#define STYLE_NAME "twothousand"
#define FACE 0xffd4d0c8u   /* what everything is made of: grey with some sand in it */
#define HILITE 0xffffffffu
#define LIGHT 0xffd4d0c8u  /* the inner light edge is the face itself */
#define SHADOW 0xff808080u
#define DARK 0xff404040u   /* the outer shade is not quite black */
#define NAVY 0xff0a246au   /* what is selected */
#define STYLE_SMOOTH_TYPE 1
/* The title bar's blue runs out to a pale one at the right, and its grey to silver. */
#define STYLE_TITLE_FADE 0xffa6caf0u
#define STYLE_TITLE_FADE_IDLE 0xffc0c0c0u

/* Rows and menus a little taller, for type that is. */
#define STYLE_METRICS(m) \
    do { \
        (m)->row_height = 17; \
        (m)->menu_item_height = 18; \
        (m)->menu_bar_height = 20; \
        (m)->popup_height = 21; \
        (m)->tab_height = 21; \
        (m)->menu_shadow = 0; \
    } while (0)

static int32_t typeface(uint32_t face, press_type *out) {
    const char *sans = "Tahoma,MS Sans Serif,Verdana,DejaVu Sans,Noto Sans";
    const char *fixed = "Lucida Console,Courier New,DejaVu Sans Mono,Liberation Mono";
    switch (face) {
    case PRESS_FACE_INTERFACE: strcpy(out->family, sans); out->size = 11; break;
    case PRESS_FACE_SMALL: strcpy(out->family, sans); out->size = 10; break;
    case PRESS_FACE_SMALL_BOLD: strcpy(out->family, sans); out->size = 10; out->bold = 1; break;
    case PRESS_FACE_READING: strcpy(out->family, sans); out->size = 11; break;
    case PRESS_FACE_FIXED: strcpy(out->family, fixed); out->size = 11; break;
    case PRESS_FACE_TITLE: strcpy(out->family, fixed); out->size = 14; out->bold = 1; break;
    case PRESS_FACE_FRAME: strcpy(out->family, sans); out->size = 11; out->bold = 1; break;
    default: return 0;
    }
    /* that desktop drew its letters in whole pixels, with no greys at their edges */
    out->crisp = 1;
    return 1;
}
#define STYLE_TYPEFACE typeface
/* A title in the menu bar stands out under the pointer: the engine is to say when it is there. */
#define STYLE_FLAT_MENU_BAR 1
#define STYLE_WANTS PRESS_WANTS_HOVER

#include "../ninetyfive/ninetyfive.c"
