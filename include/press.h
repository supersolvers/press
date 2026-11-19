/* Generated from idl/press.toml by press-idl. Do not edit. */
/* SPDX-License-Identifier: GPL-3.0-or-later */
/**
 * press: a one-bit user interface toolkit with a flat C ABI.
 *
 * Every function returns a press_status_t; results come back through the last
 * argument. Objects are opaque 64-bit handles; 0 is never a valid handle.
 * Strings are UTF-8 with an explicit length and are only borrowed for the
 * call. All calls must come from the thread that called press_app_init.
 *
 * A string passed to one of your handlers is likewise only valid until the
 * handler returns; copy it to keep it. It is NUL-terminated as a convenience,
 * though the length is what counts.
 *
 * A function that returns a string copies it into `out`, a buffer of
 * `out_cap` bytes that you supply, and stores the string's full length in
 * bytes in `*out_len`. The copy is NUL-terminated and is cut short, on a
 * character boundary, if the buffer is too small: the whole string arrived
 * when *out_len < out_cap. Pass NULL and 0 to ask for the length alone.
 */
#ifndef PRESS_H
#define PRESS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define PRESS_API __declspec(dllimport)
#else
#define PRESS_API
#endif

#define PRESS_ABI_VERSION 1

/** Pass a NUL-terminated C string where a (pointer, length) pair is expected. */
#define PRESS_S(s) (s), strlen(s)

/** Result of every call. Zero is success. */
typedef int32_t press_status_t;
enum {
    PRESS_STATUS_OK = 0, /**< success */
    PRESS_STATUS_NOT_INITIALIZED = 1, /**< press_app_init has not been called on this thread */
    PRESS_STATUS_INVALID_HANDLE = 2, /**< the handle is null, stale or already destroyed */
    PRESS_STATUS_WRONG_TYPE = 3, /**< the handle refers to an object of another class */
    PRESS_STATUS_INVALID_ARGUMENT = 4, /**< an argument is out of range, null or not valid UTF-8 */
    PRESS_STATUS_INVALID_STATE = 5, /**< the call is not allowed in the current state */
    PRESS_STATUS_BACKEND = 6, /**< the display backend failed */
    PRESS_STATUS_IO = 7, /**< a file could not be read or written */
    PRESS_STATUS_INTERNAL = 8, /**< an internal error; this is a bug in press */
};

/** Direction in which a Stack lays out its children. */
typedef int32_t press_orientation_t;
enum {
    PRESS_ORIENTATION_VERTICAL = 0, /**< top to bottom */
    PRESS_ORIENTATION_HORIZONTAL = 1, /**< left to right */
};

/** Placement of children across a Stack's axis. */
typedef int32_t press_align_t;
enum {
    PRESS_ALIGN_START = 0, /**< left or top */
    PRESS_ALIGN_CENTER = 1, /**< centred */
    PRESS_ALIGN_END = 2, /**< right or bottom */
    PRESS_ALIGN_FILL = 3, /**< stretched to the full width or height */
};

/** A key that does not type a character. */
typedef int32_t press_key_t;
enum {
    PRESS_KEY_TAB = 1, /**< move the focus on; with Shift, back */
    PRESS_KEY_RETURN = 2, /**< press the default button */
    PRESS_KEY_ESCAPE = 3, /**< press the cancel button */
    PRESS_KEY_BACKSPACE = 4, /**< delete backwards */
    PRESS_KEY_DELETE = 5, /**< delete forwards */
    PRESS_KEY_LEFT = 6, /**< left arrow */
    PRESS_KEY_RIGHT = 7, /**< right arrow */
    PRESS_KEY_UP = 8, /**< up arrow */
    PRESS_KEY_DOWN = 9, /**< down arrow */
    PRESS_KEY_HOME = 10, /**< start of the line */
    PRESS_KEY_END = 11, /**< end of the line */
    PRESS_KEY_MENU = 12, /**< pull down or put away the menu bar (F10) */
    PRESS_KEY_CONTEXT = 13, /**< pop up the context menu of whatever has the keyboard (the Menu key, or Shift+F10) */
    PRESS_KEY_PAGE_UP = 14, /**< a page up; told only to a picture that hears every key */
    PRESS_KEY_PAGE_DOWN = 15, /**< a page down; told only to a picture that hears every key */
    PRESS_KEY_INSERT = 16, /**< Insert; told only to a picture that hears every key */
    PRESS_KEY_F1 = 21, /**< a function key, told only to a picture that hears every key; F2 is 22 and so on */
    PRESS_KEY_F2 = 22, /**< F2 */
    PRESS_KEY_F3 = 23, /**< F3 */
    PRESS_KEY_F4 = 24, /**< F4 */
    PRESS_KEY_F5 = 25, /**< F5 */
    PRESS_KEY_F6 = 26, /**< F6 */
    PRESS_KEY_F7 = 27, /**< F7 */
    PRESS_KEY_F8 = 28, /**< F8 */
    PRESS_KEY_F9 = 29, /**< F9 */
    PRESS_KEY_F10 = 30, /**< F10, which otherwise works the menu bar */
    PRESS_KEY_F11 = 31, /**< F11 */
    PRESS_KEY_F12 = 32, /**< F12 */
};

/** Modifier keys held during a key press. Combine with bitwise OR. */
typedef uint32_t press_modifier_t;
enum {
    PRESS_MODIFIER_SHIFT = 1, /**< extends the selection; reverses Tab */
    PRESS_MODIFIER_CONTROL = 2, /**< the command key: moves and deletes by word */
    PRESS_MODIFIER_ALT = 4, /**< reserved */
};

/** A small picture shown before an item's label in a list. */
typedef int32_t press_icon_t;
enum {
    PRESS_ICON_NONE = 0, /**< no picture */
    PRESS_ICON_FOLDER = 1, /**< a folder */
    PRESS_ICON_DOCUMENT = 2, /**< a sheet of paper with a corner turned down */
    PRESS_ICON_APPLICATION = 3, /**< a diamond */
};

/** How a line is drawn on a Picture. */
typedef int32_t press_line_style_t;
enum {
    PRESS_LINE_STYLE_SOLID = 0, /**< every pixel */
    PRESS_LINE_STYLE_DOTTED = 1, /**< every other pixel */
    PRESS_LINE_STYLE_DASHED = 2, /**< four pixels on and three off */
};

/** One of the bundled typefaces, each drawn in pixels for the one size it is used at. Every one has Western, Greek, Cyrillic and Hebrew letters of its own and the Chinese, Japanese and Korean characters they all share. */
typedef int32_t press_typeface_t;
enum {
    PRESS_TYPEFACE_CICERO = 0, /**< heavy and plain, 12 pixels: the face of the interface */
    PRESS_TYPEFACE_AGATE = 1, /**< small and light, 11 pixels, for a lot of words in a little room */
    PRESS_TYPEFACE_AGATE_BOLD = 2, /**< Agate made heavy */
    PRESS_TYPEFACE_BREVIER = 3, /**< light and airy, 12 pixels, for reading */
    PRESS_TYPEFACE_PICA = 4, /**< every character the same width, 12 pixels, for columns and program text */
    PRESS_TYPEFACE_ELITE = 5, /**< the same, heavy and larger, 16 pixels, in the manner of a terminal */
};

/** A short description of a status code. */
static inline const char *press_status_message(press_status_t status) {
    switch (status) {
    case PRESS_STATUS_OK: return "success";
    case PRESS_STATUS_NOT_INITIALIZED: return "press_app_init has not been called on this thread";
    case PRESS_STATUS_INVALID_HANDLE: return "the handle is null, stale or already destroyed";
    case PRESS_STATUS_WRONG_TYPE: return "the handle refers to an object of another class";
    case PRESS_STATUS_INVALID_ARGUMENT: return "an argument is out of range, null or not valid UTF-8";
    case PRESS_STATUS_INVALID_STATE: return "the call is not allowed in the current state";
    case PRESS_STATUS_BACKEND: return "the display backend failed";
    case PRESS_STATUS_IO: return "a file could not be read or written";
    case PRESS_STATUS_INTERNAL: return "an internal error; this is a bug in press";
    default: return "unknown status";
    }
}

/** An opaque reference to a press object. */
typedef uint64_t press_handle_t;
typedef press_handle_t press_widget_t;
typedef press_handle_t press_window_t;
typedef press_handle_t press_stack_t;
typedef press_handle_t press_label_t;
typedef press_handle_t press_button_t;
typedef press_handle_t press_checkbox_t;
typedef press_handle_t press_text_field_t;
typedef press_handle_t press_text_area_t;
typedef press_handle_t press_radio_button_t;
typedef press_handle_t press_slider_t;
typedef press_handle_t press_list_box_t;
typedef press_handle_t press_table_t;
typedef press_handle_t press_icon_view_t;
typedef press_handle_t press_tree_view_t;
typedef press_handle_t press_progress_bar_t;
typedef press_handle_t press_picture_t;
typedef press_handle_t press_tab_view_t;
typedef press_handle_t press_popup_button_t;
typedef press_handle_t press_scroll_view_t;
typedef press_handle_t press_scroll_bar_t;
typedef press_handle_t press_menu_bar_t;
typedef press_handle_t press_menu_t;
typedef press_handle_t press_menu_item_t;
typedef press_handle_t press_group_box_t;
typedef press_handle_t press_toolbar_t;
typedef press_handle_t press_status_bar_t;
typedef press_handle_t press_button_row_t;
typedef press_handle_t press_stepper_t;
typedef press_handle_t press_combo_box_t;

/** Called when a callback is dropped, so its userdata can be released. */
typedef void (*press_destroy_fn)(void *userdata);
/** A piece of work handed to press to be called later. It is passed as three
 * arguments: the function, its userdata, and an optional destroy notifier
 * that runs once the function has been called or will never be. */
typedef void (*press_callback_fn)(void *userdata);

/* ---- App: Process-wide state. A process has one UI thread, the one that called init, and every press call must come from it, except post. */

/** Start press and make the calling thread the UI thread. Safe to call again from that thread; refused from any other until it has shut down. */
PRESS_API press_status_t press_app_init(void);

/** Show the windows and process events until quit is called or the last window closes. With the headless backend this returns at once. */
PRESS_API press_status_t press_app_run(void);

/** For programs with an event loop of their own, in place of run: connect to the display without waiting. Then wait on the descriptor from fd and call dispatch whenever it is readable. */
PRESS_API press_status_t press_app_start(void);

/** A file descriptor that becomes readable whenever dispatch has work: input, an animation or key repeat coming due, or a change made through this API that needs drawing. Only poll it; never read from it. Valid from start until stop. */
PRESS_API press_status_t press_app_fd(int32_t *out);

/** Do everything that is ready, without waiting: handle input, call handlers, redraw. Returns false once quit has been called or no window is left showing. Not allowed from inside a handler. */
PRESS_API press_status_t press_app_dispatch(bool *out);

/** Disconnect from the display after start. Objects survive and can be shown again with start. */
PRESS_API press_status_t press_app_stop(void);

/** Hand over work to be done on the UI thread. This is the one function that may be called from any thread. The work is called from the next dispatch (or by run), in the order posted, and may call any press function. If this returns an error nothing was taken: neither the work nor its destroy notifier will be called. */
PRESS_API press_status_t press_app_post(press_callback_fn work, void *work_data, press_destroy_fn work_destroy);

/** Put something other than text on the clipboard: a picture, say. `kind` says what it is, as a media type such as image/png, and `data` is the thing itself, as the bytes of a file of that type. It takes the place of whatever was on the clipboard, and any program that understands the kind can paste it. Text fields look after copying text themselves. */
PRESS_API press_status_t press_app_copy_data(const char *kind, size_t kind_len, const uint8_t *data, size_t data_len);

/** Add another kind to what this program last copied, for programs that understand that one and not the other: a picture and, with it, its caption as text/plain;charset=utf-8, say. Whoever pastes takes the kind it likes best. A kind given twice keeps the later data. It may follow copy_data or text copied from a text field; if the clipboard has since passed to another program it starts afresh, as copy_data does. */
PRESS_API press_status_t press_app_copy_also(const char *kind, size_t kind_len, const uint8_t *data, size_t data_len);

/** Ask run to return, or dispatch to report false. */
PRESS_API press_status_t press_app_quit(void);

/** Destroy every object and release all callbacks. Posted work that has not been done is dropped, and its destroy notifier is called. */
PRESS_API press_status_t press_app_shutdown(void);

/** The ABI version of the loaded library. */
PRESS_API press_status_t press_app_abi_version(uint32_t *out);

/** Choose how the program looks: the name of a style (ninetyfive, say, looked for as libpress_style_ninetyfive.so where styles are kept), the path of a style's library, or eightyfour for black and white. A style with a frame of its own has the program's windows draw their title bars and edges themselves, and one that keeps its menus in the window keeps them there on any desktop. Call it after init and before making windows. It takes the place of the desktop's choice of style; PRESS_STYLE in the program's environment, which is the user saying otherwise for one run, takes the place of this. A style that cannot be had is refused with INVALID_ARGUMENT, and the program looks as it did. */
PRESS_API press_status_t press_app_set_style(const char *name, size_t name_len);

/* ---- Widget: Base of everything that can be placed in a window, menus included. Its functions accept any handle. */

/** Destroy the widget and everything inside it. Its handle becomes invalid. */
PRESS_API press_status_t press_widget_destroy(press_widget_t self);

/** Give the widget a context menu: it pops up when the widget, or anything inside it, is clicked with the right button, or with the context-menu key while it has the keyboard. A window can carry one for everything in it. The menu belongs to the widget from then on. A text field without one of its own pops up Cut, Copy, Paste and Select All. */
PRESS_API press_status_t press_widget_set_context_menu(press_widget_t self, press_menu_t menu);

/** Set the widget's text in another of the bundled typefaces. Labels, buttons, checkboxes, text fields, text areas and pictures have one; other widgets refuse it as the wrong type. A widget is the size its text needs in its typeface. */
PRESS_API press_status_t press_widget_set_typeface(press_widget_t self, press_typeface_t face);

/** Say how small the widget may be, at the least, in pixels. It is still as large as its kind makes it where that is larger, so this can make a button taller but not narrower than its label needs. Nought either way leaves that way to the widget. */
PRESS_API press_status_t press_widget_set_min_size(press_widget_t self, int32_t width, int32_t height);

/** Say whether the widget takes up spare room when its window is larger than its content needs. Without this it goes by kind: lists, tables, tree views, text areas, scroll views and tab views grow both ways; one-line text fields and progress bars grow across; sliders and scroll bars grow along their length; a stack grows if anything in it does; the rest keep their natural size. A widget told to grow does so along the stack it is in, so an empty label makes a spring that pushes its neighbours apart; one told not to keeps its natural size both ways. */
PRESS_API press_status_t press_widget_set_stretch(press_widget_t self, bool value);

/** Give the widget the keyboard. Only text fields, buttons and checkboxes that are enabled and in a window can take it. */
PRESS_API press_status_t press_widget_focus(press_widget_t self);

/** Disabled widgets are drawn greyed and ignore the pointer and the keyboard. */
PRESS_API press_status_t press_widget_set_enabled(press_widget_t self, bool value);

/** Disabled widgets are drawn greyed and ignore the pointer and the keyboard. */
PRESS_API press_status_t press_widget_get_enabled(press_widget_t self, bool *out);

/** A word of help, shown beside the pointer when it has rested on the widget a moment, and put away when it moves on or is pressed. An empty string takes it away. */
PRESS_API press_status_t press_widget_set_tooltip(press_widget_t self, const char *value, size_t value_len);

/* ---- Window (a Widget): A top-level window. The desktop draws its title bar and frame. */

/** Create a hidden window with the given content size in pixels. */
PRESS_API press_status_t press_window_new(const char *title, size_t title_len, int32_t width, int32_t height, press_window_t *out);

/** Place a widget in the window; it fills the content area. */
PRESS_API press_status_t press_window_set_content(press_window_t self, press_widget_t content);

/** Give the window a menu bar. It is drawn across the top and takes its height from the content area. */
PRESS_API press_status_t press_window_set_menu_bar(press_window_t self, press_menu_bar_t bar);

/** Make the window the size its content asks for, with room for its menu bar, in place of the size it was created with. Call it once the content and the menu bar are in place and before the window is first shown; a window that has been shown refuses. */
PRESS_API press_status_t press_window_fit(press_window_t self);

/** Make the window visible. */
PRESS_API press_status_t press_window_show(press_window_t self);

/** The widget in the window that has the keyboard, or none (a handle of 0) when nothing has. A window that has not been typed into yet answers with its first text field, which is where typing will go. */
PRESS_API press_status_t press_window_focused(press_window_t self, press_widget_t *out);

/** Render the window content to a PNG file, magnified by an integer scale: one bit a pixel, or in colour when the window shows a photo. An open menu is included, and the picture grows to fit it. Works with every backend. */
PRESS_API press_status_t press_window_snapshot(press_window_t self, const char *path, size_t path_len, int32_t scale);

/** Test hook: press and release the pointer at a point in the content area. An open menu is reached at the place it hangs, even outside the window. */
PRESS_API press_status_t press_window_click_at(press_window_t self, int32_t x, int32_t y);

/** Test hook: move the pointer to a point in the content area and leave it there for a number of milliseconds, as far as the toolkit's clock is concerned: long enough, and the word of help of what is under it shows. */
PRESS_API press_status_t press_window_hover_at(press_window_t self, int32_t x, int32_t y, int32_t milliseconds);

/** Test hook: press the pointer at one point in the content area, move it to another in four steps, and let it go there. */
PRESS_API press_status_t press_window_drag_at(press_window_t self, int32_t x0, int32_t y0, int32_t x1, int32_t y1);

/** Pop a menu up with its corner at a point in the content area. The menu may be free-standing or a context menu, not one in a menu bar or a submenu. Its items raise activated as usual. */
PRESS_API press_status_t press_window_popup_menu(press_window_t self, press_menu_t menu, int32_t x, int32_t y);

/** Test hook: press the right button at a point in the content area. */
PRESS_API press_status_t press_window_context_click_at(press_window_t self, int32_t x, int32_t y);

/** Test hook: press the pointer's back button, or its forward button, over the window. */
PRESS_API press_status_t press_window_side_click(press_window_t self, bool forward);

/** Test hook: say which Modifier keys are held during the clicks that follow. Pass 0 to let them go. */
PRESS_API press_status_t press_window_hold_modifiers(press_window_t self, uint32_t modifiers);

/** Test hook: press a key in the window, with any combination of Modifier bits. */
PRESS_API press_status_t press_window_press_key(press_window_t self, press_key_t key, uint32_t modifiers);

/** Test hook: type text into the window as if from the keyboard. */
PRESS_API press_status_t press_window_type_text(press_window_t self, const char *text, size_t text_len);

/** Ask for what is on the clipboard as a given kind (a media type such as image/png). The answer arrives a moment later as data_pasted, with nothing in it if the clipboard holds nothing of that kind. */
PRESS_API press_status_t press_window_paste_data(press_window_t self, const char *kind, size_t kind_len);

/** Say that things of a given kind (a media type such as image/png) may be dragged from other programs and dropped on the window. Call it once for each kind, the one liked best first. What is dropped arrives as data_dropped. */
PRESS_API press_status_t press_window_accept_drops(press_window_t self, const char *kind, size_t kind_len);

/** Text shown in the title bar. */
PRESS_API press_status_t press_window_set_title(press_window_t self, const char *value, size_t value_len);

/** Whether the user may change the window's size. Windows can be resized unless this is turned off, which suits a small tool that is all of a piece. Set it before the window is first shown. */
PRESS_API press_status_t press_window_set_resizable(press_window_t self, bool value);

/** Whether the window is waiting on something the program is doing. While it is, the pointer over it is an hourglass; the window still answers the pointer and the keyboard. */
PRESS_API press_status_t press_window_set_busy(press_window_t self, bool value);

/** A name by which the desktop can know this window again another day, so as to put it back where it was, at the size it was. It is not shown to anyone, so it is not translated, and it should be the same each time the program shows the same thing: a document's path, say, or "preferences". Set it before the window is first shown. Desktops that do not remember windows ignore it. */
PRESS_API press_status_t press_window_set_tag(press_window_t self, const char *value, size_t value_len);

/** Whether the window fills its screen, with no title bar or frame and in front of the menu bar, as a film or a slide show does. Turning it on asks the desktop, which may refuse; reading it says what the window last asked for or was told. While it is on the window draws no menu bar of its own, though its shortcuts still work. Turned off, the window goes back to where it was and its size. full_screen_changed says when the desktop changes it, as when the user leaves full screen by its own means. */
PRESS_API press_status_t press_window_set_full_screen(press_window_t self, bool value);

/** Whether the window fills its screen, with no title bar or frame and in front of the menu bar, as a film or a slide show does. Turning it on asks the desktop, which may refuse; reading it says what the window last asked for or was told. While it is on the window draws no menu bar of its own, though its shortcuts still work. Turned off, the window goes back to where it was and its size. full_screen_changed says when the desktop changes it, as when the user leaves full screen by its own means. */
PRESS_API press_status_t press_window_get_full_screen(press_window_t self, bool *out);

/** The desktop has put the window into full screen, or taken it out, whether the program asked or not: whether it is now. */
typedef void (*press_window_full_screen_changed_fn)(press_window_t self, bool full_screen, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_window_on_full_screen_changed(press_window_t self, press_window_full_screen_changed_fn callback, void *userdata, press_destroy_fn destroy);

/** Files were dragged from another program, or from a list whose items stand for files, and dropped on the window. The handler is given their paths, one per line, and whether the user asked for copies (by holding Control, on most desktops) rather than for the files to be moved. What is done with them is up to the handler. Without a handler the window does not accept files. */
typedef void (*press_window_files_dropped_fn)(press_window_t self, const char *paths, size_t paths_len, bool copy, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_window_on_files_dropped(press_window_t self, press_window_files_dropped_fn callback, void *userdata, press_destroy_fn destroy);

/** The answer to paste_data: the kind asked for, and what the clipboard held of that kind, which is nothing if it held none. The data is lent for the length of the call. */
typedef void (*press_window_data_pasted_fn)(press_window_t self, const char *kind, size_t kind_len, const uint8_t *data, size_t data_len, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_window_on_data_pasted(press_window_t self, press_window_data_pasted_fn callback, void *userdata, press_destroy_fn destroy);

/** Something of a kind named with accept_drops was dragged from another program and dropped on the window: its kind, the thing itself, and where in the window's content it was let go. The data is lent for the length of the call. */
typedef void (*press_window_data_dropped_fn)(press_window_t self, const char *kind, size_t kind_len, const uint8_t *data, size_t data_len, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_window_on_data_dropped(press_window_t self, press_window_data_dropped_fn callback, void *userdata, press_destroy_fn destroy);

/** The pointer's back or forward button, on the side of many a mouse, was pressed over the window: forward says which. What it does is the program's to say; without a handler nothing happens. */
typedef void (*press_window_navigate_fn)(press_window_t self, bool forward, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_window_on_navigate(press_window_t self, press_window_navigate_fn callback, void *userdata, press_destroy_fn destroy);

/** The user asked to close the window. Without a handler the application quits. */
typedef void (*press_window_close_requested_fn)(press_window_t self, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_window_on_close_requested(press_window_t self, press_window_close_requested_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- Stack (a Widget): Lays out its children in a row or a column. */

/** Create an empty stack. */
PRESS_API press_status_t press_stack_new(press_orientation_t orientation, press_stack_t *out);

/** Append a child. A widget can have only one parent. */
PRESS_API press_status_t press_stack_add(press_stack_t self, press_widget_t child);

/** Put a child at a place among the stack's children, counting from 0, with those after it moving along; a place past the end is the end. A child already in this stack is moved to the place, which is how children are put in another order. */
PRESS_API press_status_t press_stack_insert(press_stack_t self, press_widget_t child, int32_t index);

/** Gap between children, in pixels. */
PRESS_API press_status_t press_stack_set_spacing(press_stack_t self, int32_t value);

/** Margin around the children, in pixels. */
PRESS_API press_status_t press_stack_set_padding(press_stack_t self, int32_t value);

/** Placement of children across the axis. */
PRESS_API press_status_t press_stack_set_align(press_stack_t self, press_align_t value);

/* ---- Label (a Widget): A line of text. */

/** Create a label. */
PRESS_API press_status_t press_label_new(const char *text, size_t text_len, press_label_t *out);

/** The text shown. */
PRESS_API press_status_t press_label_set_text(press_label_t self, const char *value, size_t value_len);

/* ---- Button (a Widget): A push button. */

/** Create a button. */
PRESS_API press_status_t press_button_new(const char *label, size_t label_len, press_button_t *out);

/** Give the button a picture, in colour, before its label: the bytes of a PNG file, up to 256 pixels each way, with see-through parts if it has them. A button with a picture and an empty label is the picture alone. No bytes take the picture away. */
PRESS_API press_status_t press_button_set_image(press_button_t self, const uint8_t *png, size_t png_len);

/** The button's caption. */
PRESS_API press_status_t press_button_set_label(press_button_t self, const char *value, size_t value_len);

/** Mark the default button of a dialog: it gets the heavy outline and answers the Return key. */
PRESS_API press_status_t press_button_set_is_default(press_button_t self, bool value);

/** Mark the button that answers the Escape key. */
PRESS_API press_status_t press_button_set_is_cancel(press_button_t self, bool value);

/** Whether the button lies flat, with no edge of its own until the pointer is over it or down on it, and is no wider than its label needs: a tool in a toolbar. */
PRESS_API press_status_t press_button_set_flat(press_button_t self, bool value);

/** The button was pressed, with the pointer or from the keyboard. */
typedef void (*press_button_clicked_fn)(press_button_t self, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_button_on_clicked(press_button_t self, press_button_clicked_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- Checkbox (a Widget): A box that is either checked or not, with a caption. */

/** Create an unchecked checkbox. */
PRESS_API press_status_t press_checkbox_new(const char *label, size_t label_len, press_checkbox_t *out);

/** The box's caption. */
PRESS_API press_status_t press_checkbox_set_label(press_checkbox_t self, const char *value, size_t value_len);

/** Whether the box is checked. */
PRESS_API press_status_t press_checkbox_set_checked(press_checkbox_t self, bool value);

/** Whether the box is checked. */
PRESS_API press_status_t press_checkbox_get_checked(press_checkbox_t self, bool *out);

/** The user changed the box. */
typedef void (*press_checkbox_toggled_fn)(press_checkbox_t self, bool checked, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_checkbox_on_toggled(press_checkbox_t self, press_checkbox_toggled_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- TextField (a Widget): One line of editable text. */

/** Create a text field holding the given text. */
PRESS_API press_status_t press_text_field_new(const char *text, size_t text_len, press_text_field_t *out);

/** Select all of the text. */
PRESS_API press_status_t press_text_field_select_all(press_text_field_t self);

/** Select from one place in the text to another, each a byte offset into its UTF-8: `to` is where the caret goes, and is scrolled into view, so a selection made backwards has `to` first. The same place twice puts the caret there with nothing selected. A place inside a character moves to its start, and one past the end to the end. Made by the program, it does not raise selected. */
PRESS_API press_status_t press_text_field_select(press_text_field_t self, int32_t from, int32_t to);

/** Where the selection begins, as a byte offset into the text's UTF-8; with nothing selected, where the caret is. */
PRESS_API press_status_t press_text_field_selection_start(press_text_field_t self, int32_t *out);

/** Where the selection ends, as a byte offset into the text's UTF-8; with nothing selected, where the caret is. */
PRESS_API press_status_t press_text_field_selection_end(press_text_field_t self, int32_t *out);

/** Copy the selected text to the clipboard. With nothing selected the clipboard is left alone. */
PRESS_API press_status_t press_text_field_copy(press_text_field_t self);

/** Copy the selected text to the clipboard and delete it. Raises changed. */
PRESS_API press_status_t press_text_field_cut(press_text_field_t self);

/** Replace the selection with the clipboard's text. Raises changed; on a display the text may arrive a moment after this returns. */
PRESS_API press_status_t press_text_field_paste(press_text_field_t self);

/** The text in the field. Setting it does not raise changed. */
PRESS_API press_status_t press_text_field_set_text(press_text_field_t self, const char *value, size_t value_len);

/** The text in the field. Setting it does not raise changed. */
PRESS_API press_status_t press_text_field_get_text(press_text_field_t self, char *out, size_t out_cap, size_t *out_len);

/** Greyed text shown while the field is empty. */
PRESS_API press_status_t press_text_field_set_placeholder(press_text_field_t self, const char *value, size_t value_len);

/** The narrowest the field may be laid out, in pixels. */
PRESS_API press_status_t press_text_field_set_min_width(press_text_field_t self, int32_t value);

/** Show a bullet in place of each character. A password cannot be copied, cut or dragged out, input methods are kept away from it, and assistive technology is not told what it holds. Not available on a text area. */
PRESS_API press_status_t press_text_field_set_password(press_text_field_t self, bool value);

/** The user edited the text. The handler is given the text as it now stands. */
typedef void (*press_text_field_changed_fn)(press_text_field_t self, const char *text, size_t text_len, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_text_field_on_changed(press_text_field_t self, press_text_field_changed_fn callback, void *userdata, press_destroy_fn destroy);

/** The user pressed Return in the field. The handler is given the text. The default button, if any, is pressed as well. */
typedef void (*press_text_field_submitted_fn)(press_text_field_t self, const char *text, size_t text_len, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_text_field_on_submitted(press_text_field_t self, press_text_field_submitted_fn callback, void *userdata, press_destroy_fn destroy);

/** The user moved the caret or changed the selection: by typing, the keys, the pointer or a paste. The handler is given where the selection begins and ends, as byte offsets into the text's UTF-8, the start first; equal, they are the caret. It comes after changed when both happen. A password field does not raise it. */
typedef void (*press_text_field_selected_fn)(press_text_field_t self, int32_t start, int32_t end, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_text_field_on_selected(press_text_field_t self, press_text_field_selected_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- TextArea (a TextField): Several lines of editable text, wrapped at word gaps to the width of the area. It keeps tabs, each four spaces wide, and holds up to two megabytes, where a text field holds 64 kilobytes. It is a TextField: every TextField function and event works on it, except that Return starts a new line, so it never raises submitted and does not press the default button. */

/** Create a text area holding the given text. Line breaks in the text are kept. */
PRESS_API press_status_t press_text_area_new(const char *text, size_t text_len, press_text_area_t *out);

/** The height of the area, in lines of text. Longer text scrolls. */
PRESS_API press_status_t press_text_area_set_rows(press_text_area_t self, int32_t value);

/* ---- RadioButton (a Widget): One of a group of choices, of which one at most is chosen. Radio buttons that share a parent (the same Stack) are a group. The arrow keys move through the group, choosing as they go. */

/** Create an unchosen radio button. */
PRESS_API press_status_t press_radio_button_new(const char *label, size_t label_len, press_radio_button_t *out);

/** The button's caption. */
PRESS_API press_status_t press_radio_button_set_label(press_radio_button_t self, const char *value, size_t value_len);

/** Whether this is the chosen one. Setting it true unchooses the others in its group. Setting it does not raise chosen. */
PRESS_API press_status_t press_radio_button_set_checked(press_radio_button_t self, bool value);

/** Whether this is the chosen one. Setting it true unchooses the others in its group. Setting it does not raise chosen. */
PRESS_API press_status_t press_radio_button_get_checked(press_radio_button_t self, bool *out);

/** The user chose this button. Choosing the one already chosen raises nothing. */
typedef void (*press_radio_button_chosen_fn)(press_radio_button_t self, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_radio_button_on_chosen(press_radio_button_t self, press_radio_button_chosen_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- Slider (a Widget): A number picked by position: a knob that slides along a groove. Its value runs from 0 to max; a vertical slider has its largest value at the top. */

/** Create a slider with a value of 0 and a max of 100. */
PRESS_API press_status_t press_slider_new(press_orientation_t orientation, press_slider_t *out);

/** Where the knob is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range. */
PRESS_API press_status_t press_slider_set_value(press_slider_t self, int32_t value);

/** Where the knob is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range. */
PRESS_API press_status_t press_slider_get_value(press_slider_t self, int32_t *out);

/** The largest value, at least 1. */
PRESS_API press_status_t press_slider_set_max(press_slider_t self, int32_t value);

/** The largest value, at least 1. */
PRESS_API press_status_t press_slider_get_max(press_slider_t self, int32_t *out);

/** How far an arrow key or a notch of the wheel moves the value. */
PRESS_API press_status_t press_slider_set_step(press_slider_t self, int32_t value);

/** The length of the slider in pixels. It is always 16 pixels thick. */
PRESS_API press_status_t press_slider_set_length(press_slider_t self, int32_t value);

/** The user moved the knob. */
typedef void (*press_slider_changed_fn)(press_slider_t self, int32_t value, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_slider_on_changed(press_slider_t self, press_slider_changed_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- ListBox (a Widget): Rows of text, one of which may be selected. It scrolls when there are more rows than fit. The arrow keys move the selection, and typing a letter jumps to the next row that starts with it. */

/** Create an empty list box, five rows high. */
PRESS_API press_status_t press_list_box_new(press_list_box_t *out);

/** Append a row. */
PRESS_API press_status_t press_list_box_add(press_list_box_t self, const char *label, size_t label_len);

/** Remove every row. */
PRESS_API press_status_t press_list_box_clear(press_list_box_t self);

/** Whether an item is selected. With multiple selection several may be. */
PRESS_API press_status_t press_list_box_is_selected(press_list_box_t self, int32_t index, bool *out);

/** Select or unselect one item, leaving the others as they are where several may be selected. Does not raise changed. */
PRESS_API press_status_t press_list_box_set_item_selected(press_list_box_t self, int32_t index, bool selected);

/** Give an item a small picture before its label. Once any item has one, every label moves over so that they line up. */
PRESS_API press_status_t press_list_box_set_icon(press_list_box_t self, int32_t index, press_icon_t icon);

/** Say that an item stands for a file, by its full path; an empty path takes that away. Such an item can be dragged out of its list, even one whose rows cannot be rearranged, and dropped on another window or program, which is offered the file. A window with a files_dropped handler hears of it that way. */
PRESS_API press_status_t press_list_box_set_file(press_list_box_t self, int32_t index, const char *path, size_t path_len);

/** Give an item something to carry when it is dragged out of its list: `kind` says what it is, as a media type such as image/png, and `data` is the thing itself. Such an item can be dragged out even of a list whose rows cannot be rearranged, and a program that accepts the kind is given the data where it is dropped. An empty kind takes it away. */
PRESS_API press_status_t press_list_box_set_drag_data(press_list_box_t self, int32_t index, const char *kind, size_t kind_len, const uint8_t *data, size_t data_len);

/** Give an item another kind to carry when it is dragged out, besides what set_drag_data gave it: where it is dropped takes the kind it likes best. A kind given twice keeps the later data. */
PRESS_API press_status_t press_list_box_add_drag_data(press_list_box_t self, int32_t index, const char *kind, size_t kind_len, const uint8_t *data, size_t data_len);

/** Give an item a picture of your own in place of a stock icon. The picture is drawn in text, one line for each row of pixels: X or # is black, a full stop is white, and anything else lets the row show through. Up to 14 pixels each way. An empty string takes the picture away. */
PRESS_API press_status_t press_list_box_set_picture(press_list_box_t self, int32_t index, const char *art, size_t art_len);

/** Give an item a picture read from a PNG file of up to 14 pixels each way. It is reduced to one bit: dark pixels become black, light ones white, and transparent ones let the row show through. */
PRESS_API press_status_t press_list_box_set_picture_file(press_list_box_t self, int32_t index, const char *path, size_t path_len);

/** Remove an item. Items after it move down by one index. In a tree view, whatever hangs from it goes too. Does not raise removed. */
PRESS_API press_status_t press_list_box_remove(press_list_box_t self, int32_t index);

/** Move an item to a place among the rows as shown, counting from 0. A sorted table stops being sorted. Does not raise moved. */
PRESS_API press_status_t press_list_box_move_item(press_list_box_t self, int32_t index, int32_t position);

/** Where an item is among the rows as shown, counting from 0; -1 if it is folded away in a tree. */
PRESS_API press_status_t press_list_box_position_of(press_list_box_t self, int32_t index, int32_t *out);

/** Start editing an item's text in place, whether or not the list is editable. In a table, column says which cell; elsewhere pass 0. */
PRESS_API press_status_t press_list_box_edit(press_list_box_t self, int32_t index, int32_t column);

/** Give an item a picture in colour, in place of its icon: the bytes of a PNG file, up to 16 pixels each way, with see-through parts if it has them. No bytes take the picture away. */
PRESS_API press_status_t press_list_box_set_image(press_list_box_t self, int32_t index, const uint8_t *png, size_t png_len);

/** The index of the selected row, counting from 0; -1 when none is. Setting it scrolls the row into view and does not raise changed. */
PRESS_API press_status_t press_list_box_set_selected(press_list_box_t self, int32_t value);

/** The index of the selected row, counting from 0; -1 when none is. Setting it scrolls the row into view and does not raise changed. */
PRESS_API press_status_t press_list_box_get_selected(press_list_box_t self, int32_t *out);

/** Let several items be selected at once: Control with a click adds or removes one, Shift with a click or an arrow stretches the selection, a drag stretches it, and Control+A takes everything. selected then gives the item last selected. */
PRESS_API press_status_t press_list_box_set_multiple(press_list_box_t self, bool value);

/** Let the user edit an item's text in place. Return then edits the current item, as renaming a file does, where it would otherwise open it; a double click still opens it. */
PRESS_API press_status_t press_list_box_set_editable(press_list_box_t self, bool value);

/** Let rows dragged out of another list be dropped here: from the same window, another window of the program, or a list in another press program. The rows leave that list, which must be reorderable, and join this one, keeping their text and pictures. A tree view can take rows too: dropped onto an item they hang from it, dropped between two they go there. An item dragged out of a tree takes its whole branch: into another tree the branch keeps its shape, into a plain list it is laid out flat. */
PRESS_API press_status_t press_list_box_set_accepts_rows(press_list_box_t self, bool value);

/** Let the user drag a row to a new place: press it and pull it up or down, and a line shows where it will land. Where several rows are selected, pulling one moves them all. Items keep their indices; only the order they are shown in changes. In a tree view an item can also be dropped onto another, to hang from it, and takes along whatever hangs from it. Rows dragged onto something that takes text, here or in another program, arrive as their labels. */
PRESS_API press_status_t press_list_box_set_reorderable(press_list_box_t self, bool value);

/** The height of the list, in rows. */
PRESS_API press_status_t press_list_box_set_rows(press_list_box_t self, int32_t value);

/** The narrowest the list may be laid out, scroll bar included, in pixels. */
PRESS_API press_status_t press_list_box_set_min_width(press_list_box_t self, int32_t value);

/** The user selected a different row. */
typedef void (*press_list_box_changed_fn)(press_list_box_t self, int32_t index, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_list_box_on_changed(press_list_box_t self, press_list_box_changed_fn callback, void *userdata, press_destroy_fn destroy);

/** The user opened a row: a double click, or Return. The default button, if any, is pressed as well after Return. */
typedef void (*press_list_box_activated_fn)(press_list_box_t self, int32_t index, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_list_box_on_activated(press_list_box_t self, press_list_box_activated_fn callback, void *userdata, press_destroy_fn destroy);

/** The user dragged an item to a new place. index is the item, unchanged; position is where it now is among the rows. */
typedef void (*press_list_box_moved_fn)(press_list_box_t self, int32_t index, int32_t position, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_list_box_on_moved(press_list_box_t self, press_list_box_moved_fn callback, void *userdata, press_destroy_fn destroy);

/** The user dragged an item out of this list into another. index is the index it had; items after it have moved down by one. When several leave together the highest index is reported first, so each index is right as it is reported. */
typedef void (*press_list_box_removed_fn)(press_list_box_t self, int32_t index, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_list_box_on_removed(press_list_box_t self, press_list_box_removed_fn callback, void *userdata, press_destroy_fn destroy);

/** The user dropped an item here from another list. index is its index in this list, which is the last; position is where it is among the rows. */
typedef void (*press_list_box_received_fn)(press_list_box_t self, int32_t index, int32_t position, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_list_box_on_received(press_list_box_t self, press_list_box_received_fn callback, void *userdata, press_destroy_fn destroy);

/** The user edited an item's text in place and kept the change. The list already holds the new text; column is 0 except in a table. */
typedef void (*press_list_box_edited_fn)(press_list_box_t self, int32_t index, int32_t column, const char *text, size_t text_len, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_list_box_on_edited(press_list_box_t self, press_list_box_edited_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- Table (a ListBox): A list box with columns under a row of headings. It is a ListBox: rows are added with ListBox.add, with the cells of a row separated by tab characters, and every ListBox function and event works on it. */

/** Create an empty table with no columns. */
PRESS_API press_status_t press_table_new(press_table_t *out);

/** Add a column: its heading and its width in pixels. A cell too long for its column is cut short. The user can drag the edge between two headings to change a width. */
PRESS_API press_status_t press_table_add_column(press_table_t self, const char *title, size_t title_len, int32_t width);

/** Sort by a column, smallest first or largest first, or with -1 show the rows in the order they were added. */
PRESS_API press_status_t press_table_sort_by(press_table_t self, int32_t column, bool ascending);

/** Let the user sort the rows by clicking a column's heading; a second click turns the order round. Cells that start with numbers are ordered as numbers. Sorting changes the order rows are shown in, never their indices. */
PRESS_API press_status_t press_table_set_sortable(press_table_t self, bool value);

/* ---- IconView (a ListBox): A list box that shows its items as large pictures with their names beneath, in rows that fill its width, as a folder's window does. It is a ListBox: every ListBox function and event works on it. An item's picture goes by its icon (a folder, a document or an application), or is the program's own picture drawn double size. In a label with tabs only what comes before the first is shown. The arrow keys go from icon to icon. With set_reorderable on, the user can put icons where they like. */

/** Create an empty icon view, three rows of icons high. */
PRESS_API press_status_t press_icon_view_new(press_icon_view_t *out);

/** Put an item's icon at a place in the view, measured in pixels from the top left of all the icons (not of the view, once it has been scrolled). A negative place puts it back in line with the rest. Clearing the view forgets every place. */
PRESS_API press_status_t press_icon_view_set_position(press_icon_view_t self, int32_t index, int32_t x, int32_t y);

/** The user dragged an icon to a new place in the view, which they can do once ListBox.set_reorderable has been turned on for it. index is the item; x and y are where it now is, as for set_position. Several selected icons dragged together are each reported. The order of the items does not change. */
typedef void (*press_icon_view_placed_fn)(press_icon_view_t self, int32_t index, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_icon_view_on_placed(press_icon_view_t self, press_icon_view_placed_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- TreeView (a ListBox): A list box whose rows hang from one another and can be folded away. It is a ListBox: an item is known by its index, in the order items were added, and every ListBox function and event works on it. ListBox.add adds an item at the top level. Right and Left unfold and fold from the keyboard. */

/** Create an empty tree view. */
PRESS_API press_status_t press_tree_view_new(press_tree_view_t *out);

/** Add an item under another, given by its index, or at the top level with -1. Returns the new item's index. Items start folded. */
PRESS_API press_status_t press_tree_view_add_child(press_tree_view_t self, int32_t parent, const char *label, size_t label_len, int32_t *out);

/** The index of the item an item hangs from, or -1 at the top level. It changes when the user drags the item elsewhere in a reorderable tree. */
PRESS_API press_status_t press_tree_view_parent_of(press_tree_view_t self, int32_t index, int32_t *out);

/** Unfold or fold an item to show or hide what hangs from it. */
PRESS_API press_status_t press_tree_view_set_expanded(press_tree_view_t self, int32_t index, bool expanded);

/* ---- ProgressBar (a Widget): Shows how far along some work is. It fills from the left as value goes from 0 to max. */

/** Create a progress bar with a value of 0 and a max of 100. */
PRESS_API press_status_t press_progress_bar_new(press_progress_bar_t *out);

/** How much is done, from 0 to max. A value out of range is brought into range. */
PRESS_API press_status_t press_progress_bar_set_value(press_progress_bar_t self, int32_t value);

/** How much is done, from 0 to max. A value out of range is brought into range. */
PRESS_API press_status_t press_progress_bar_get_value(press_progress_bar_t self, int32_t *out);

/** The value that means finished, at least 1. */
PRESS_API press_status_t press_progress_bar_set_max(press_progress_bar_t self, int32_t value);

/** The value that means finished, at least 1. */
PRESS_API press_status_t press_progress_bar_get_max(press_progress_bar_t self, int32_t *out);

/** Show moving stripes in place of an amount, for work whose length is not known. */
PRESS_API press_status_t press_progress_bar_set_busy(press_progress_bar_t self, bool value);

/** The length of the bar in pixels. */
PRESS_API press_status_t press_progress_bar_set_length(press_progress_bar_t self, int32_t value);

/* ---- Picture (a Widget): A white box of a size the program chooses, for the program to draw on with lines, boxes and words: a graph, a chart, a diagram. What is drawn is kept, and drawn again whenever the window is, so draw once and again only when something changes. Places are in the picture's own pixels, from its top left corner; what falls outside is not shown. The pointer pressed, dragged and let go in it, and the wheel turned over it, are told with the place. Words are drawn in the picture's typeface, which set_typeface changes. */

/** Create a blank picture of the given size in pixels, from 1 to 4096 each way. */
PRESS_API press_status_t press_picture_new(int32_t width, int32_t height, press_picture_t *out);

/** Rub out everything drawn, leaving it white. */
PRESS_API press_status_t press_picture_clear(press_picture_t self);

/** Draw a line one pixel thick from one point to another, both ends included. */
PRESS_API press_status_t press_picture_line(press_picture_t self, int32_t x0, int32_t y0, int32_t x1, int32_t y1, press_line_style_t style);

/** Draw a box with its top left corner at a point: an outline one pixel thick, or filled in black. */
PRESS_API press_status_t press_picture_box(press_picture_t self, int32_t x, int32_t y, int32_t width, int32_t height, bool filled);

/** Fill a box with a pattern of black and white: `pattern` is eight bytes, a row of an 8 by 8 tile each, the leftmost pixel in the top bit, a set bit black and a clear one white. It covers what was there, so eight noughts rub the box out and eight 255s black it in; 170 and 85 by turns are a half-tone grey. The tile is laid from the picture's corner, so boxes side by side in one pattern join without a seam. Lines, outlines and words are always drawn over patterns, whichever was asked for first. */
PRESS_API press_status_t press_picture_fill(press_picture_t self, int32_t x, int32_t y, int32_t width, int32_t height, const uint8_t *pattern, size_t pattern_len);

/** Lay down a picture of your own making, a pixel a bit, with its top left corner at a point: `bits` is rows of `width` pixels, each row padded to whole bytes, the leftmost pixel in the top bit of a byte, set for black. It covers what was there, as fill does. For a scene drawn pixel by pixel, such as shading or a game's world, this is one call where dots would be thousands. */
PRESS_API press_status_t press_picture_image(press_picture_t self, int32_t x, int32_t y, int32_t width, int32_t height, const uint8_t *bits, size_t bits_len);

/** Draw what follows in a colour, 0xRRGGBB: lines, boxes outlined and filled, and words. A picture draws in black until told otherwise, and again once it is cleared. */
PRESS_API press_status_t press_picture_set_color(press_picture_t self, uint32_t rgb);

/** Fill a box with one colour, 0xRRGGBB. It covers what was there, as fill does; lines, boxes and words lie on it wherever in the order they were drawn. */
PRESS_API press_status_t press_picture_fill_color(press_picture_t self, int32_t x, int32_t y, int32_t width, int32_t height, uint32_t rgb);

/** Lay down a picture in colour with its top left corner at a point: `rgb` is rows of `width` pixels, three bytes each, red, green and blue. press is black and white, and this is its one exception, for what is a photograph by nature, such as a camera's picture; offer it in black and white too. It covers what was there, as image does. */
PRESS_API press_status_t press_picture_photo(press_picture_t self, int32_t x, int32_t y, int32_t width, int32_t height, const uint8_t *rgb, size_t rgb_len);

/** Draw words in black with the left end of their line at x and its top at y. */
PRESS_API press_status_t press_picture_text(press_picture_t self, int32_t x, int32_t y, const char *text, size_t text_len);

/** How wide words would be drawn, in pixels, so that they can be placed: centred under a mark, say, or kept clear of an edge. */
PRESS_API press_status_t press_picture_text_width(press_picture_t self, const char *text, size_t text_len, int32_t *out);

/** How tall a line of words is, in pixels. */
PRESS_API press_status_t press_picture_line_height(press_picture_t self, int32_t *out);

/** Draw words in black in a typeface of their own, whatever the picture's, with the left end of their line at x and its top at y: a heading over plain words, say, or program text in an even face. */
PRESS_API press_status_t press_picture_text_in(press_picture_t self, int32_t x, int32_t y, const char *text, size_t text_len, press_typeface_t face);

/** How wide words would be drawn in a given typeface, in pixels. */
PRESS_API press_status_t press_picture_text_width_in(press_picture_t self, const char *text, size_t text_len, press_typeface_t face, int32_t *out);

/** How tall a line of words is in a given typeface, in pixels. */
PRESS_API press_status_t press_picture_line_height_in(press_picture_t self, press_typeface_t face, int32_t *out);

/** Change the picture's size, from 1 to 4096 pixels each way. What is drawn is kept; what falls outside is not shown. */
PRESS_API press_status_t press_picture_set_size(press_picture_t self, int32_t width, int32_t height);

/** What the picture shows, in words, for those who cannot see it: "a graph of y = sin x from -10 to 10", say. Screen readers read it out. */
PRESS_API press_status_t press_picture_set_description(press_picture_t self, const char *value, size_t value_len);

/** Let the picture have the keyboard, for a game or a drawing worked by keys: Tab stops at it, a press in it takes the keyboard there, and it is ringed while it has it. The arrow keys and their kin are then told by key_pressed, and what is typed by typed. */
PRESS_API press_status_t press_picture_set_takes_keys(press_picture_t self, bool value);

/** Let the picture hear every key, for a terminal or an emulator of another machine: it takes the keyboard as takes_keys does, and each key is told by key_down with the modifiers held, Tab, Return, Escape, F1 to F12, Page Up and Down, Insert, and Control or Alt with a character among them. Control and Shift with a character still choose a menu entry with that shortcut. What is typed without Control or Alt still comes by typed, and key_pressed is not raised. */
PRESS_API press_status_t press_picture_set_raw_keys(press_picture_t self, bool value);

/** Let the picture hear the right button, for a remote desktop or a game: pressed in the picture, it raises right_pressed, and no context menu pops up there. Where the pointer goes while it is held is told by dragged, and letting it go by right_released. */
PRESS_API press_status_t press_picture_set_right_button(press_picture_t self, bool value);

/** Let the picture hear the middle button, as right_button does the right: middle_pressed, where the pointer goes while it is held by dragged, and middle_released. */
PRESS_API press_status_t press_picture_set_middle_button(press_picture_t self, bool value);

/** The right button was pressed in a picture that hears it (right_button): where, in the picture's pixels. Until it is let go the picture hears where the pointer goes, by dragged, even outside. */
typedef void (*press_picture_right_pressed_fn)(press_picture_t self, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_right_pressed(press_picture_t self, press_picture_right_pressed_fn callback, void *userdata, press_destroy_fn destroy);

/** The right button, pressed in the picture, was let go: where, in the picture's pixels, which may be outside it. */
typedef void (*press_picture_right_released_fn)(press_picture_t self, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_right_released(press_picture_t self, press_picture_right_released_fn callback, void *userdata, press_destroy_fn destroy);

/** The middle button was pressed in a picture that hears it (middle_button): where, in the picture's pixels. Until it is let go the picture hears where the pointer goes, by dragged, even outside. */
typedef void (*press_picture_middle_pressed_fn)(press_picture_t self, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_middle_pressed(press_picture_t self, press_picture_middle_pressed_fn callback, void *userdata, press_destroy_fn destroy);

/** The middle button, pressed in the picture, was let go: where, in the picture's pixels, which may be outside it. */
typedef void (*press_picture_middle_released_fn)(press_picture_t self, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_middle_released(press_picture_t self, press_picture_middle_released_fn callback, void *userdata, press_destroy_fn destroy);

/** A key was pressed while a picture that hears every key had the keyboard: its Key value, or 0 for a character key; the character, as a Unicode code point, when Control or Alt was held with one (Control gives it in lower case), and otherwise 0; and the Modifier bits held. */
typedef void (*press_picture_key_down_fn)(press_picture_t self, int32_t key, int32_t character, int32_t modifiers, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_key_down(press_picture_t self, press_picture_key_down_fn callback, void *userdata, press_destroy_fn destroy);

/** A picture told to stretch (with set_stretch) was given room of another size by its window, the first time it is laid out included: the width and height it now fills, in pixels. Draw it again for that size; its own size, from new or set_size, is the least it asks for. */
typedef void (*press_picture_resized_fn)(press_picture_t self, int32_t width, int32_t height, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_resized(press_picture_t self, press_picture_resized_fn callback, void *userdata, press_destroy_fn destroy);

/** A key that types nothing was pressed while the picture had the keyboard: an arrow, Home, End, Backspace or Delete, as its Key value. Tab, Return and Escape keep their usual work. */
typedef void (*press_picture_key_pressed_fn)(press_picture_t self, int32_t key, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_key_pressed(press_picture_t self, press_picture_key_pressed_fn callback, void *userdata, press_destroy_fn destroy);

/** Something was typed while the picture had the keyboard: a letter, a figure, a space, or whatever an input method made of the keys. */
typedef void (*press_picture_typed_fn)(press_picture_t self, const char *text, size_t text_len, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_typed(press_picture_t self, press_picture_typed_fn callback, void *userdata, press_destroy_fn destroy);

/** The pointer was pressed in the picture: where, and 1 for a single click, 2 for the second of a double click, and so on. Until it is let go the picture hears where it moves, even outside. */
typedef void (*press_picture_pressed_fn)(press_picture_t self, int32_t x, int32_t y, int32_t clicks, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_pressed(press_picture_t self, press_picture_pressed_fn callback, void *userdata, press_destroy_fn destroy);

/** The pointer moved while held after a press in the picture. The place may be outside it. */
typedef void (*press_picture_dragged_fn)(press_picture_t self, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_dragged(press_picture_t self, press_picture_dragged_fn callback, void *userdata, press_destroy_fn destroy);

/** The pointer pressed in the picture was let go. */
typedef void (*press_picture_released_fn)(press_picture_t self, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_released(press_picture_t self, press_picture_released_fn callback, void *userdata, press_destroy_fn destroy);

/** The wheel was turned over the picture: lines down, or up when negative (three to a notch of an ordinary wheel), and where the pointer was. */
typedef void (*press_picture_scrolled_fn)(press_picture_t self, int32_t lines, int32_t x, int32_t y, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_picture_on_scrolled(press_picture_t self, press_picture_scrolled_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- TabView (a Widget): Several pages of widgets in the space of one, with a row of tabs along the top to choose between them. Left and Right turn the pages from the keyboard. */

/** Create a tab view with no pages. */
PRESS_API press_status_t press_tab_view_new(press_tab_view_t *out);

/** Add a page: the word on its tab, and the widget (usually a Stack) shown when it is chosen. */
PRESS_API press_status_t press_tab_view_add(press_tab_view_t self, const char *title, size_t title_len, press_widget_t content);

/** The index of the page showing; -1 when there are no pages. Setting it does not raise changed. */
PRESS_API press_status_t press_tab_view_set_selected(press_tab_view_t self, int32_t value);

/** The index of the page showing; -1 when there are no pages. Setting it does not raise changed. */
PRESS_API press_status_t press_tab_view_get_selected(press_tab_view_t self, int32_t *out);

/** The user turned to a different page. */
typedef void (*press_tab_view_changed_fn)(press_tab_view_t self, int32_t index, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_tab_view_on_changed(press_tab_view_t self, press_tab_view_changed_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- PopupButton (a Widget): A box showing one of a list of choices. Pressing it pops the whole list up over it, with the current choice under the pointer: drag to another and let go, or click and then click. From the keyboard, Space pops the list up. */

/** Create a pop-up button with no choices. */
PRESS_API press_status_t press_popup_button_new(press_popup_button_t *out);

/** Append a choice. The first one added starts as the current choice. */
PRESS_API press_status_t press_popup_button_add(press_popup_button_t self, const char *label, size_t label_len);

/** Remove every choice. */
PRESS_API press_status_t press_popup_button_clear(press_popup_button_t self);

/** The index of the current choice, counting from 0; -1 when there are no choices. Setting it does not raise changed. */
PRESS_API press_status_t press_popup_button_set_selected(press_popup_button_t self, int32_t value);

/** The index of the current choice, counting from 0; -1 when there are no choices. Setting it does not raise changed. */
PRESS_API press_status_t press_popup_button_get_selected(press_popup_button_t self, int32_t *out);

/** The user picked a different choice. Picking the current one again raises nothing. */
typedef void (*press_popup_button_changed_fn)(press_popup_button_t self, int32_t index, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_popup_button_on_changed(press_popup_button_t self, press_popup_button_changed_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- ScrollView (a Widget): A framed opening onto a widget that may be larger than the space it is given. It has a scroll bar down its right-hand side, and one along the bottom when the content is too wide. A widget inside that takes the keyboard is scrolled into view. */

/** Create an empty scroll view, 200 by 120 pixels. */
PRESS_API press_status_t press_scroll_view_new(press_scroll_view_t *out);

/** Put a widget in the view; usually a Stack of others. It is laid out at the size it asks for, or the size of the opening if that is larger. */
PRESS_API press_status_t press_scroll_view_set_content(press_scroll_view_t self, press_widget_t content);

/** The width the view asks for, frame and scroll bar included, in pixels. */
PRESS_API press_status_t press_scroll_view_set_width(press_scroll_view_t self, int32_t value);

/** The height the view asks for, frame included, in pixels. */
PRESS_API press_status_t press_scroll_view_set_height(press_scroll_view_t self, int32_t value);

/** How far the content is scrolled to the left, in pixels. A value past the end is brought back. */
PRESS_API press_status_t press_scroll_view_set_scroll_x(press_scroll_view_t self, int32_t value);

/** How far the content is scrolled to the left, in pixels. A value past the end is brought back. */
PRESS_API press_status_t press_scroll_view_get_scroll_x(press_scroll_view_t self, int32_t *out);

/** How far the content is scrolled up, in pixels. A value past the end is brought back. */
PRESS_API press_status_t press_scroll_view_set_scroll_y(press_scroll_view_t self, int32_t value);

/** How far the content is scrolled up, in pixels. A value past the end is brought back. */
PRESS_API press_status_t press_scroll_view_get_scroll_y(press_scroll_view_t self, int32_t *out);

/* ---- ScrollBar (a Widget): A scroll bar: an arrow at each end and a box that slides between them. Its value runs from 0 to max. It scrolls nothing by itself; connect changed and move your content. A text area has one of its own. */

/** Create a scroll bar with a value of 0, a max of 100 and a page of 10. */
PRESS_API press_status_t press_scroll_bar_new(press_orientation_t orientation, press_scroll_bar_t *out);

/** Where the box is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range. */
PRESS_API press_status_t press_scroll_bar_set_value(press_scroll_bar_t self, int32_t value);

/** Where the box is, from 0 to max. Setting it does not raise changed; a value out of range is brought into range. */
PRESS_API press_status_t press_scroll_bar_get_value(press_scroll_bar_t self, int32_t *out);

/** The largest value: how far the content can scroll. With a max of 0 there is nothing to scroll and the bar goes blank. */
PRESS_API press_status_t press_scroll_bar_set_max(press_scroll_bar_t self, int32_t value);

/** The largest value: how far the content can scroll. With a max of 0 there is nothing to scroll and the bar goes blank. */
PRESS_API press_status_t press_scroll_bar_get_max(press_scroll_bar_t self, int32_t *out);

/** How much is in view at once, in the units of value. It sets the size of the box, and a click in the track moves by a page less a step. */
PRESS_API press_status_t press_scroll_bar_set_page(press_scroll_bar_t self, int32_t value);

/** How far an arrow, or one notch of the wheel's three, moves the value. */
PRESS_API press_status_t press_scroll_bar_set_step(press_scroll_bar_t self, int32_t value);

/** The length of the bar in pixels. It is always 16 pixels thick. */
PRESS_API press_status_t press_scroll_bar_set_length(press_scroll_bar_t self, int32_t value);

/** The user moved the box, with an arrow, the track, a drag or the wheel. */
typedef void (*press_scroll_bar_changed_fn)(press_scroll_bar_t self, int32_t value, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_scroll_bar_on_changed(press_scroll_bar_t self, press_scroll_bar_changed_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- MenuBar (a Widget): The row of menu titles across the top of a window. Attach it with Window.set_menu_bar. */

/** Create an empty menu bar. */
PRESS_API press_status_t press_menu_bar_new(press_menu_bar_t *out);

/** Append a menu. A menu can be in only one bar. */
PRESS_API press_status_t press_menu_bar_add(press_menu_bar_t self, press_menu_t menu);

/* ---- Menu (a Widget): A pull-down menu. A disabled menu has a greyed title and cannot be pulled down. */

/** Create an empty menu. */
PRESS_API press_status_t press_menu_new(const char *title, size_t title_len, press_menu_t *out);

/** Append an item. An item can be in only one menu. */
PRESS_API press_status_t press_menu_add(press_menu_t self, press_menu_item_t item);

/** Append a dividing line. */
PRESS_API press_status_t press_menu_add_separator(press_menu_t self);

/** The word shown in the menu bar. */
PRESS_API press_status_t press_menu_set_title(press_menu_t self, const char *value, size_t value_len);

/* ---- MenuItem (a Widget): A command in a menu. A disabled item is greyed and cannot be chosen. */

/** Create a menu item. */
PRESS_API press_status_t press_menu_item_new(const char *label, size_t label_len, press_menu_item_t *out);

/** Hang a submenu off the item. It opens beside the item when the item is pointed at, and the item no longer raises activated. The submenu's own title is not shown. A menu can hang in only one place. */
PRESS_API press_status_t press_menu_item_set_submenu(press_menu_item_t self, press_menu_t menu);

/** The item's text. */
PRESS_API press_status_t press_menu_item_set_label(press_menu_item_t self, const char *value, size_t value_len);

/** Show a check mark beside the item. Choosing the item does not change it. */
PRESS_API press_status_t press_menu_item_set_checked(press_menu_item_t self, bool value);

/** Show a check mark beside the item. Choosing the item does not change it. */
PRESS_API press_status_t press_menu_item_get_checked(press_menu_item_t self, bool *out);

/** One character that, typed with Control, chooses the item; empty for none. In a text field, Control with A, C, X or V edits the text instead. */
PRESS_API press_status_t press_menu_item_set_shortcut(press_menu_item_t self, const char *value, size_t value_len);

/** The user chose the item, from the menu or with its shortcut. */
typedef void (*press_menu_item_activated_fn)(press_menu_item_t self, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_menu_item_on_activated(press_menu_item_t self, press_menu_item_activated_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- GroupBox (a Stack): A box with a title, round things that belong together. It is a Stack: every Stack function works on it, and its children are laid out inside the box. */

/** Create an empty box. With an empty title it is a box and no more. */
PRESS_API press_status_t press_group_box_new(const char *title, size_t title_len, press_orientation_t orientation, press_group_box_t *out);

/** The words at the top of the box. */
PRESS_API press_status_t press_group_box_set_title(press_group_box_t self, const char *value, size_t value_len);

/* ---- Toolbar (a Stack): A bar of tools, for the top of a window: a Stack that lays its children out in a row, close together, on a bar drawn by the style. Buttons in it are usually flat. */

/** Create an empty toolbar. */
PRESS_API press_status_t press_toolbar_new(press_toolbar_t *out);

/* ---- StatusBar (a Stack): A bar of news, for the foot of a window: a Stack that lays its children out in a row on a bar drawn by the style. Labels are what it usually holds. */

/** Create an empty status bar. */
PRESS_API press_status_t press_status_bar_new(press_status_bar_t *out);

/** Whether each child is shown in a cell of its own, the last reaching the end of the bar. At first they are all on one strip. */
PRESS_API press_status_t press_status_bar_set_panels(press_status_bar_t self, bool value);

/* ---- ButtonRow (a Stack): The row of buttons at the foot of a dialog: a Stack that lays its children out in a row at its far end, in the order the style of the program has them. Add the button that agrees first, then the one that calls it off, then any others; where the style has OK before Cancel they are shown as added, and where it has Cancel before OK, last first. */

/** Create an empty row of buttons. */
PRESS_API press_status_t press_button_row_new(press_button_row_t *out);

/* ---- Stepper (a Widget): A whole number with a pair of arrows beside it, one over the other, to step it up and down. An arrow held down steps again and again. The arrow keys step it too when it has the keyboard. */

/** Create a stepper at 0, which goes from 0 to 100 by ones. */
PRESS_API press_status_t press_stepper_new(press_stepper_t *out);

/** The number. Setting it does not raise changed; a value out of range is brought into range. */
PRESS_API press_status_t press_stepper_set_value(press_stepper_t self, int32_t value);

/** The number. Setting it does not raise changed; a value out of range is brought into range. */
PRESS_API press_status_t press_stepper_get_value(press_stepper_t self, int32_t *out);

/** The least the number may be. */
PRESS_API press_status_t press_stepper_set_min(press_stepper_t self, int32_t value);

/** The most the number may be. */
PRESS_API press_status_t press_stepper_set_max(press_stepper_t self, int32_t value);

/** How far one press of an arrow moves the number, at least 1. */
PRESS_API press_status_t press_stepper_set_step(press_stepper_t self, int32_t value);

/** The user stepped the number. */
typedef void (*press_stepper_changed_fn)(press_stepper_t self, int32_t value, void *userdata);
/** Connect a handler (replacing any earlier one); pass NULL to disconnect. */
PRESS_API press_status_t press_stepper_on_changed(press_stepper_t self, press_stepper_changed_fn callback, void *userdata, press_destroy_fn destroy);

/* ---- ComboBox (a TextField): A text field with a list of things it might say, dropped from a button at its end. Choosing one puts it in the field and raises changed; anything else may be typed. It is a TextField: every TextField function and event works on it. */

/** Create a combo box holding the given text, with no choices yet. */
PRESS_API press_status_t press_combo_box_new(const char *text, size_t text_len, press_combo_box_t *out);

/** Append a choice to the list. */
PRESS_API press_status_t press_combo_box_add(press_combo_box_t self, const char *choice, size_t choice_len);

/** Take every choice away. The text in the field stays. */
PRESS_API press_status_t press_combo_box_clear(press_combo_box_t self);

#ifdef __cplusplus
}
#endif

#endif /* PRESS_H */
