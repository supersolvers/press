/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Two lists with rows dragged between them.
 *
 *   cc examples/c/shuffle.c -Iinclude -Ltarget/debug -lpress -o examples/c/shuffle
 *   LD_LIBRARY_PATH=target/debug ./examples/c/shuffle
 *
 * Songs on the left, a playlist on the right. Drag a song across to add it,
 * and up or down in the playlist to put it in order. Hold Control or Shift
 * to select several and they all go together. Drag one back to take it off.
 *
 * Each list reports what left it and what arrived, which is how a program
 * keeps its own records in step. Run from the top of the checkout so the
 * picture in assets/icons is found.
 *
 * With --tree the playlist is a tree with two folders, and songs can be
 * dropped onto a folder, between rows, or dragged back out.
 *
 * With --windows the playlist gets a window of its own, and songs are
 * dragged from one window to the other.
 *
 * With --snapshot FILE it renders to a PNG without opening a window.
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

static const char *const SONGS[] = {"Daisy Bell", "Greensleeves", "Scarborough Fair", "Shenandoah", "The Water Is Wide", "Loch Lomond"};

static void on_removed(press_list_box_t self, int32_t index, void *userdata) {
    (void)self;
    printf("%s: row %d left\n", (const char *)userdata, index);
}

static void on_received(press_list_box_t self, int32_t index, int32_t position, void *userdata) {
    (void)self;
    printf("%s: row %d arrived at place %d\n", (const char *)userdata, index, position + 1);
}

static void on_moved(press_list_box_t self, int32_t index, int32_t position, void *userdata) {
    (void)self;
    printf("%s: row %d moved to place %d\n", (const char *)userdata, index, position + 1);
}

/* A list that gives rows up, takes them in, and lets several go at once. */
static press_list_box_t make_list(const char *name) {
    press_list_box_t list;
    CHECK(press_list_box_new(&list));
    CHECK(press_list_box_set_min_width(list, 150));
    CHECK(press_list_box_set_rows(list, 6));
    CHECK(press_list_box_set_multiple(list, true));
    CHECK(press_list_box_set_reorderable(list, true));
    CHECK(press_list_box_set_accepts_rows(list, true));
    CHECK(press_list_box_on_removed(list, on_removed, (void *)name, NULL));
    CHECK(press_list_box_on_received(list, on_received, (void *)name, NULL));
    CHECK(press_list_box_on_moved(list, on_moved, (void *)name, NULL));
    return list;
}

int main(int argc, char **argv) {
    const char *snapshot = (argc == 3 && strcmp(argv[1], "--snapshot") == 0) ? argv[2] : NULL;
    bool two = argc == 2 && strcmp(argv[1], "--windows") == 0;
    press_window_t window, second = 0;
    press_stack_t column, headings, lists;
    press_label_t left_title, right_title;
    press_list_box_t songs, playlist;
    size_t i;

    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Playlist"), 350, 160, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 14));
    CHECK(press_stack_set_spacing(column, 6));
    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &headings));
    CHECK(press_stack_set_spacing(headings, 131));
    CHECK(press_label_new(PRESS_S("Songs"), &left_title));
    CHECK(press_label_new(PRESS_S("Playlist"), &right_title));
    CHECK(press_stack_add(headings, left_title));
    CHECK(press_stack_add(headings, right_title));
    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &lists));
    CHECK(press_stack_set_spacing(lists, 20));

    songs = make_list("songs");
    if (argc == 2 && strcmp(argv[1], "--tree") == 0) {
        /* a tree takes rows just as a list does, and can be told to in the same way */
        int32_t folder;
        CHECK(press_tree_view_new(&playlist));
        CHECK(press_list_box_set_min_width(playlist, 150));
        CHECK(press_list_box_set_rows(playlist, 6));
        CHECK(press_list_box_set_reorderable(playlist, true));
        CHECK(press_list_box_set_accepts_rows(playlist, true));
        CHECK(press_list_box_on_removed(playlist, on_removed, "playlist", NULL));
        CHECK(press_list_box_on_received(playlist, on_received, "playlist", NULL));
        CHECK(press_list_box_on_moved(playlist, on_moved, "playlist", NULL));
        CHECK(press_tree_view_add_child(playlist, -1, PRESS_S("Morning"), &folder));
        CHECK(press_list_box_set_icon(playlist, folder, PRESS_ICON_FOLDER));
        CHECK(press_tree_view_add_child(playlist, -1, PRESS_S("Evening"), &folder));
        CHECK(press_list_box_set_icon(playlist, folder, PRESS_ICON_FOLDER));
    } else {
        playlist = make_list("playlist");
    }
    for (i = 0; i < sizeof SONGS / sizeof *SONGS; i++) {
        CHECK(press_list_box_add(songs, PRESS_S(SONGS[i])));
        /* a picture read from a small PNG file, reduced to one bit */
        if (press_list_box_set_picture_file(songs, (int32_t)i, PRESS_S("assets/icons/note.png")) != PRESS_STATUS_OK) {
            CHECK(press_list_box_set_icon(songs, (int32_t)i, PRESS_ICON_DOCUMENT)); /* not run from the checkout */
        }
    }
    CHECK(press_stack_add(lists, songs));
    if (two) {
        /* the playlist in a window of its own: rows cross between windows just the same */
        CHECK(press_window_new(PRESS_S("Playlist"), 178, 124, &second));
        CHECK(press_window_set_title(window, PRESS_S("Songs")));
        {
            press_stack_t holder;
            CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &holder));
            CHECK(press_stack_set_padding(holder, 14));
            CHECK(press_stack_add(holder, playlist));
            CHECK(press_window_set_content(second, holder));
        }
    } else {
        CHECK(press_stack_add(lists, playlist));
    }
    CHECK(press_stack_add(column, headings));
    CHECK(press_stack_add(column, lists));
    CHECK(press_window_set_content(window, column));
    /* whatever the style, the window is as large as what is in it needs */
    CHECK(press_window_fit(window));

    if (snapshot) {
        /* without a pointer to drag with, do from the program what dragging would */
        CHECK(press_list_box_set_item_selected(songs, 1, true));
        CHECK(press_list_box_set_item_selected(songs, 3, true));
        CHECK(press_window_snapshot(window, PRESS_S(snapshot), 2));
    } else {
        CHECK(press_window_show(window));
        if (two) {
            CHECK(press_window_show(second));
        }
        CHECK(press_app_run());
    }
    CHECK(press_app_shutdown());
    return 0;
}
