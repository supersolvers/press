/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Radio buttons, a slider and a list box.
 *
 *   cc examples/c/controls.c -Iinclude -Ltarget/debug -lpress -o examples/c/controls
 *   LD_LIBRARY_PATH=target/debug ./examples/c/controls
 *
 * Pick a typeface from the list, a style with the radio buttons and a size
 * with the slider; the line at the bottom says what was picked. Everything
 * works from the keyboard: Tab between the controls, arrows within them.
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

static const char *const FACES[] = {"Cicero", "Agate", "Brevier", "Pica", "Elite", "Fleuron", "Vignette", "Cartouche"};
static const char *const STYLES[] = {"Plain", "Bold", "Italic"};

struct state {
    press_label_t summary;
    press_radio_button_t styles[3];
    int face, style, size;
};

static void show(struct state *st) {
    char text[80];
    snprintf(text, sizeof text, "%s %s, %d point", FACES[st->face], STYLES[st->style], st->size);
    CHECK(press_label_set_text(st->summary, PRESS_S(text)));
    printf("%s\n", text);
}

static void on_face(press_list_box_t self, int32_t index, void *userdata) {
    struct state *st = userdata;
    (void)self;
    st->face = index;
    show(st);
}

static void on_style(press_radio_button_t self, void *userdata) {
    struct state *st = userdata;
    int i;
    for (i = 0; i < 3; i++) {
        if (st->styles[i] == self) {
            st->style = i;
        }
    }
    show(st);
}

static void on_size(press_slider_t self, int32_t value, void *userdata) {
    struct state *st = userdata;
    (void)self;
    st->size = 9 + value; /* the slider runs 0 to 15: 9 to 24 point */
    show(st);
}

int main(int argc, char **argv) {
    const char *snapshot = (argc == 3 && strcmp(argv[1], "--snapshot") == 0) ? argv[2] : NULL;
    press_window_t window;
    press_stack_t column, row, style_group, size_row;
    press_list_box_t faces;
    press_slider_t size;
    press_label_t size_label;
    struct state st = {0};
    size_t i;

    st.size = 12;
    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Controls"), 300, 190, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 16));
    CHECK(press_stack_set_spacing(column, 12));

    /* the list and the radio group, side by side */
    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &row));
    CHECK(press_stack_set_spacing(row, 20));
    CHECK(press_list_box_new(&faces));
    for (i = 0; i < sizeof FACES / sizeof *FACES; i++) {
        CHECK(press_list_box_add(faces, PRESS_S(FACES[i])));
    }
    CHECK(press_list_box_set_selected(faces, 0));
    CHECK(press_list_box_on_changed(faces, on_face, &st, NULL));
    CHECK(press_stack_add(row, faces));

    /* radio buttons that share a stack are one group */
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &style_group));
    for (i = 0; i < 3; i++) {
        CHECK(press_radio_button_new(PRESS_S(STYLES[i]), &st.styles[i]));
        CHECK(press_radio_button_on_chosen(st.styles[i], on_style, &st, NULL));
        CHECK(press_stack_add(style_group, st.styles[i]));
    }
    CHECK(press_radio_button_set_checked(st.styles[0], true));
    CHECK(press_stack_add(row, style_group));

    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &size_row));
    CHECK(press_stack_set_align(size_row, PRESS_ALIGN_CENTER));
    CHECK(press_label_new(PRESS_S("Size:"), &size_label));
    CHECK(press_slider_new(PRESS_ORIENTATION_HORIZONTAL, &size));
    CHECK(press_slider_set_max(size, 15));
    CHECK(press_slider_set_value(size, 3));
    CHECK(press_slider_set_length(size, 220));
    CHECK(press_slider_on_changed(size, on_size, &st, NULL));
    CHECK(press_stack_add(size_row, size_label));
    CHECK(press_stack_add(size_row, size));

    CHECK(press_label_new(PRESS_S("Cicero Plain, 12 point"), &st.summary));
    CHECK(press_stack_add(column, row));
    CHECK(press_stack_add(column, size_row));
    CHECK(press_stack_add(column, st.summary));
    CHECK(press_window_set_content(window, column));
    /* whatever the style, the window is as large as what is in it needs */
    CHECK(press_window_fit(window));

    if (snapshot) {
        CHECK(press_widget_focus(faces));
        CHECK(press_window_type_text(window, PRESS_S("v")));   /* jumps to Vignette */
        CHECK(press_window_press_key(window, PRESS_KEY_TAB, 0)); /* on to the radio group */
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0)); /* Bold */
        CHECK(press_window_press_key(window, PRESS_KEY_TAB, 0)); /* the group is one stop: on to the slider */
        for (i = 0; i < 6; i++) {
            CHECK(press_window_press_key(window, PRESS_KEY_RIGHT, 0));
        }
        CHECK(press_window_snapshot(window, PRESS_S(snapshot), 2));
    } else {
        CHECK(press_window_show(window));
        CHECK(press_app_run());
    }
    CHECK(press_app_shutdown());
    return 0;
}
