/* SPDX-License-Identifier: GPL-3.0-or-later */
/* A tab view, a table, a tree view and progress bars.
 *
 *   cc examples/c/files.c -Iinclude -Ltarget/debug -lpress -o examples/c/files
 *   LD_LIBRARY_PATH=target/debug ./examples/c/files
 *
 * Two pages under tabs: a table of files, and a tree of folders. Below them
 * a progress bar follows the selection in the table, and a second one just
 * shows that something is going on. Left and Right turn the pages when the
 * tabs have the keyboard, and unfold and fold in the tree.
 *
 * In the table: click a heading to sort, drag the edge between two headings
 * to resize, hold Control or Shift to select several rows, press Return to
 * rename a file, and drag a row up or down to move it.
 *
 * With --snapshot NAME it renders both pages to PNG files without opening a window.
 */
#include <press.h>
#include <stdio.h>
#include <stdlib.h>

#define CHECK(call)                                                              \
    do {                                                                         \
        press_status_t status_ = (call);                                         \
        if (status_ != PRESS_STATUS_OK) {                                        \
            fprintf(stderr, "%s: %s\n", #call, press_status_message(status_));   \
            exit(1);                                                             \
        }                                                                        \
    } while (0)

static const char *const FILES[] = {
    "System\t198K\tsuitcase",       "Finder\t46K\tapplication", "Clipboard File\t2K\tdocument",
    "Note Pad File\t1K\tdocument",  "Scrapbook File\t12K\tdocument", "Printer\t38K\tdriver",
    "Control Panel\t8K\tdesk accessory",
};

struct state {
    press_label_t status;
    press_progress_bar_t used;
};

/* A picture of the program's own, drawn in text: X is black, a full stop
 * white, a space lets the row show through. This one is a disk. */
static const char DISK[] =
    "XXXXXXXXXX \n"
    "X.X....X.XX\n"
    "X.X....X..X\n"
    "X.XXXXXX..X\n"
    "X.........X\n"
    "X.XXXXXXX.X\n"
    "X.X.....X.X\n"
    "X.X.....X.X\n"
    "X.X.....X.X\n"
    "XXXXXXXXXXX";

static void on_moved(press_list_box_t self, int32_t index, int32_t position, void *userdata) {
    (void)self;
    (void)userdata;
    printf("row %d moved to place %d\n", index, position + 1);
}

/* The table reports the row; a table is a list box as far as events go. */
static void on_file(press_list_box_t self, int32_t index, void *userdata) {
    struct state *st = userdata;
    char name[32], text[64];
    size_t n = 0;
    (void)self;
    while (FILES[index][n] != '\t' && n < sizeof name - 1) {
        name[n] = FILES[index][n];
        n++;
    }
    name[n] = '\0';
    snprintf(text, sizeof text, "%s: file %d of 7", name, index + 1);
    CHECK(press_label_set_text(st->status, PRESS_S(text)));
    CHECK(press_progress_bar_set_value(st->used, index + 1));
    printf("%s\n", text);
}

/* A cell was edited in place. The table already holds the new text. */
static void on_renamed(press_list_box_t self, int32_t index, int32_t column, const char *text, size_t text_len, void *userdata) {
    (void)self;
    (void)text_len;
    (void)userdata;
    printf("row %d, column %d is now \"%s\"\n", index, column, text);
}

static void on_page(press_tab_view_t self, int32_t index, void *userdata) {
    (void)self;
    (void)userdata;
    printf("page %d\n", index);
}

int main(int argc, char **argv) {
    const char *snapshot = (argc == 3 && strcmp(argv[1], "--snapshot") == 0) ? argv[2] : NULL;
    press_window_t window;
    press_stack_t column;
    press_tab_view_t tabs;
    press_table_t table;
    press_tree_view_t tree;
    press_progress_bar_t busy;
    struct state st;
    int32_t system, fonts, apps;
    size_t i;

    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Files"), 340, 250, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 14));
    CHECK(press_stack_set_spacing(column, 10));

    /* a table: a list box with columns; a row's cells are separated by tabs */
    CHECK(press_table_new(&table));
    CHECK(press_table_add_column(table, PRESS_S("Name"), 130));
    CHECK(press_table_add_column(table, PRESS_S("Size"), 50));
    CHECK(press_table_add_column(table, PRESS_S("Kind"), 96));
    for (i = 0; i < sizeof FILES / sizeof *FILES; i++) {
        CHECK(press_list_box_add(table, PRESS_S(FILES[i])));
    }
    CHECK(press_list_box_on_changed(table, on_file, &st, NULL));
    CHECK(press_list_box_on_edited(table, on_renamed, NULL, NULL));
    CHECK(press_table_set_sortable(table, true));
    CHECK(press_list_box_set_multiple(table, true));
    CHECK(press_list_box_set_editable(table, true));
    CHECK(press_list_box_set_reorderable(table, true));
    CHECK(press_list_box_on_moved(table, on_moved, NULL, NULL));
    for (i = 0; i < sizeof FILES / sizeof *FILES; i++) {
        CHECK(press_list_box_set_icon(table, (int32_t)i, i < 2 ? PRESS_ICON_APPLICATION : PRESS_ICON_DOCUMENT));
    }
    CHECK(press_list_box_set_picture(table, 0, PRESS_S(DISK))); /* the System file gets our own picture */

    /* a tree: items hang from one another by index */
    CHECK(press_tree_view_new(&tree));
    CHECK(press_list_box_set_min_width(tree, 292));
    CHECK(press_tree_view_add_child(tree, -1, PRESS_S("System Folder"), &system));
    CHECK(press_tree_view_add_child(tree, system, PRESS_S("Fonts"), &fonts));
    {
        int32_t item;
        CHECK(press_tree_view_add_child(tree, fonts, PRESS_S("Cicero"), &item));
        CHECK(press_tree_view_add_child(tree, fonts, PRESS_S("Brevier"), &item));
        CHECK(press_tree_view_add_child(tree, system, PRESS_S("Desk Accessories"), &item));
        CHECK(press_tree_view_add_child(tree, -1, PRESS_S("Applications"), &apps));
        CHECK(press_tree_view_add_child(tree, apps, PRESS_S("Sketchpad"), &item));
        CHECK(press_tree_view_add_child(tree, apps, PRESS_S("Notepad"), &item));
        CHECK(press_tree_view_add_child(tree, -1, PRESS_S("Empty Folder"), &item));
    }
    CHECK(press_tree_view_set_expanded(tree, system, true));
    /* folders can be rearranged: drag a row between two others, or onto one to put it inside */
    CHECK(press_list_box_set_reorderable(tree, true));
    {
        int32_t item;
        /* nine items were added, numbered 0 to 8 in the order they went in */
        for (item = 0; item <= 8; item++) {
            /* folders hold things (4 is Desk Accessories, 8 the empty one); fonts are documents, 6 and 7 programs */
            bool folder = item == system || item == fonts || item == apps || item == 4 || item == 8;
            CHECK(press_list_box_set_icon(tree, item, folder ? PRESS_ICON_FOLDER : item >= 6 ? PRESS_ICON_APPLICATION : PRESS_ICON_DOCUMENT));
        }
    }

    CHECK(press_tab_view_new(&tabs));
    CHECK(press_tab_view_add(tabs, PRESS_S("Files"), table));
    CHECK(press_tab_view_add(tabs, PRESS_S("Folders"), tree));
    CHECK(press_tab_view_on_changed(tabs, on_page, NULL, NULL));

    CHECK(press_label_new(PRESS_S("Pick a file."), &st.status));
    CHECK(press_progress_bar_new(&st.used));
    CHECK(press_progress_bar_set_max(st.used, 7));
    CHECK(press_progress_bar_set_length(st.used, 312));
    CHECK(press_progress_bar_new(&busy));
    CHECK(press_progress_bar_set_busy(busy, true));
    CHECK(press_progress_bar_set_length(busy, 312));

    CHECK(press_stack_add(column, tabs));
    CHECK(press_stack_add(column, st.status));
    CHECK(press_stack_add(column, st.used));
    CHECK(press_stack_add(column, busy));
    CHECK(press_window_set_content(window, column));

    if (snapshot) {
        char path[512];
        CHECK(press_widget_focus(table));
        CHECK(press_table_sort_by(table, 1, false));           /* largest first */
        CHECK(press_window_type_text(window, PRESS_S("s")));   /* System */
        CHECK(press_window_type_text(window, PRESS_S("s")));   /* Scrapbook File */
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, PRESS_MODIFIER_SHIFT)); /* and the next */
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, PRESS_MODIFIER_SHIFT)); /* and the next */
        CHECK(press_list_box_edit(table, 3, 0));               /* rename Note Pad File */
        CHECK(press_window_type_text(window, PRESS_S("Notes")));
        snprintf(path, sizeof path, "%s-1.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0)); /* keep the new name */
        CHECK(press_widget_focus(tabs));
        CHECK(press_window_press_key(window, PRESS_KEY_RIGHT, 0)); /* turn to Folders */
        CHECK(press_window_press_key(window, PRESS_KEY_TAB, 0));
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));  /* System Folder */
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));  /* Fonts */
        CHECK(press_window_press_key(window, PRESS_KEY_RIGHT, 0)); /* unfold it */
        CHECK(press_window_press_key(window, PRESS_KEY_RIGHT, 0)); /* step in: Cicero */
        snprintf(path, sizeof path, "%s-2.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
    } else {
        CHECK(press_window_show(window));
        CHECK(press_app_run());
    }
    CHECK(press_app_shutdown());
    return 0;
}
