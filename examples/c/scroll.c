/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Scroll bars.
 *
 *   cc examples/c/scroll.c -Iinclude -Ltarget/debug -lpress -o examples/c/scroll
 *   LD_LIBRARY_PATH=target/debug ./examples/c/scroll
 *
 * A scroll bar scrolls nothing by itself: it holds a number and says when the
 * user changes it. Here a horizontal one sets a level shown in a label, a
 * text area shows the bar that comes built in, and a scroll view holds a
 * list of checkboxes too long for the window. Tab walks down the list and
 * the view follows.
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

static void on_level(press_scroll_bar_t self, int32_t value, void *userdata) {
    press_label_t label = *(press_label_t *)userdata;
    char text[32];
    (void)self;
    snprintf(text, sizeof text, "Level: %d of 10", value);
    CHECK(press_label_set_text(label, PRESS_S(text)));
    printf("level %d\n", value);
}

int main(int argc, char **argv) {
    const char *snapshot = (argc == 3 && strcmp(argv[1], "--snapshot") == 0) ? argv[2] : NULL;
    press_window_t window;
    press_stack_t column;
    press_label_t label;
    press_scroll_bar_t level;
    press_text_area_t story;
    press_scroll_view_t view;
    press_stack_t list;
    int i;
    const char *text =
        "Once there was a box that lived in a grey track between two arrows.\n\n"
        "Press an arrow and it moves a line. Hold the arrow and it keeps going.\n\n"
        "Click the track and it moves a page. Drag the box and it follows you.\n\n"
        "Turn the wheel and it moves three lines at a time.\n\n"
        "When everything fits, the box goes away and the track turns white.";

    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Scroll bars"), 300, 300, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 16));
    CHECK(press_stack_set_spacing(column, 10));

    CHECK(press_label_new(PRESS_S("Level: 3 of 10"), &label));
    CHECK(press_scroll_bar_new(PRESS_ORIENTATION_HORIZONTAL, &level));
    CHECK(press_scroll_bar_set_max(level, 10));
    CHECK(press_scroll_bar_set_page(level, 2));
    CHECK(press_scroll_bar_set_value(level, 3));
    CHECK(press_scroll_bar_set_length(level, 268));
    CHECK(press_scroll_bar_on_changed(level, on_level, &label, NULL));

    CHECK(press_text_area_new(PRESS_S(text), &story));
    CHECK(press_text_field_set_min_width(story, 268));
    CHECK(press_text_area_set_rows(story, 6));

    CHECK(press_stack_add(column, label));
    CHECK(press_stack_add(column, level));
    CHECK(press_stack_add(column, story));

    /* a scroll view: any widget, seen through an opening */
    CHECK(press_scroll_view_new(&view));
    CHECK(press_scroll_view_set_width(view, 268));
    CHECK(press_scroll_view_set_height(view, 86));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &list));
    CHECK(press_stack_set_padding(list, 8));
    for (i = 1; i <= 12; i++) {
        press_checkbox_t box;
        char name[32];
        snprintf(name, sizeof name, "Extension %d", i);
        CHECK(press_checkbox_new(PRESS_S(name), &box));
        CHECK(press_checkbox_set_checked(box, i % 3 == 0));
        CHECK(press_stack_add(list, box));
    }
    CHECK(press_scroll_view_set_content(view, list));
    CHECK(press_stack_add(column, view));
    CHECK(press_window_set_content(window, column));

    if (snapshot) {
        CHECK(press_window_click_at(window, 270, 48)); /* the right-hand arrow of the level */
        CHECK(press_window_click_at(window, 270, 48));
        CHECK(press_window_click_at(window, 277, 147)); /* the text area's down arrow */
        CHECK(press_window_click_at(window, 277, 147));
        /* Tab from the text area into the list and down it: the view follows */
        for (i = 0; i < 7; i++) {
            CHECK(press_window_press_key(window, PRESS_KEY_TAB, 0));
        }
        CHECK(press_window_snapshot(window, PRESS_S(snapshot), 2));
    } else {
        CHECK(press_window_show(window));
        CHECK(press_app_run());
    }
    CHECK(press_app_shutdown());
    return 0;
}
