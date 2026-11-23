/* SPDX-License-Identifier: GPL-3.0-or-later */
/* A text area and a password field.
 *
 *   cc examples/c/notes.c -Iinclude -Ltarget/debug -lpress -o examples/c/notes
 *   LD_LIBRARY_PATH=target/debug ./examples/c/notes
 *
 * The note is a text area: it wraps, Return starts a new line, and the
 * arrow keys and the wheel move through it. The passphrase is a password
 * field: it shows bullets and cannot be copied. Save is the default button,
 * but Return only reaches it when the keyboard is not in the note.
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

struct state {
    press_text_area_t note;
    press_text_field_t pass;
    press_label_t status;
};

/* A text area is a text field: the same functions read it. */
static void on_save(press_button_t self, void *userdata) {
    struct state *st = userdata;
    char text[64];
    size_t note_len, pass_len, lines = 1, i;
    char *note;
    (void)self;
    CHECK(press_text_field_get_text(st->note, NULL, 0, &note_len));
    CHECK(press_text_field_get_text(st->pass, NULL, 0, &pass_len));
    note = malloc(note_len + 1);
    CHECK(press_text_field_get_text(st->note, note, note_len + 1, &note_len));
    for (i = 0; i < note_len; i++) {
        lines += note[i] == '\n';
    }
    free(note);
    if (pass_len == 0) {
        snprintf(text, sizeof text, "Enter a passphrase first.");
    } else {
        snprintf(text, sizeof text, "Saved %zu line%s.", lines, lines == 1 ? "" : "s");
    }
    CHECK(press_label_set_text(st->status, PRESS_S(text)));
    printf("%s\n", text);
}

int main(int argc, char **argv) {
    const char *snapshot = (argc == 3 && strcmp(argv[1], "--snapshot") == 0) ? argv[2] : NULL;
    press_window_t window;
    press_stack_t column, row;
    press_label_t heading, pass_label;
    press_button_t save;
    struct state st;

    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Notes"), 300, 214, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 16));
    CHECK(press_stack_set_spacing(column, 10));

    CHECK(press_label_new(PRESS_S("Note:"), &heading));
    CHECK(press_text_area_new(PRESS_S(""), &st.note));
    CHECK(press_text_field_set_placeholder(st.note, PRESS_S("Write something worth keeping")));
    CHECK(press_text_field_set_min_width(st.note, 268));
    CHECK(press_text_area_set_rows(st.note, 5));

    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &row));
    CHECK(press_stack_set_align(row, PRESS_ALIGN_CENTER));
    CHECK(press_label_new(PRESS_S("Passphrase:"), &pass_label));
    CHECK(press_text_field_new(PRESS_S(""), &st.pass));
    CHECK(press_text_field_set_password(st.pass, true));
    CHECK(press_text_field_set_min_width(st.pass, 180));
    CHECK(press_stack_add(row, pass_label));
    CHECK(press_stack_add(row, st.pass));

    CHECK(press_label_new(PRESS_S("Not saved."), &st.status));
    CHECK(press_button_new(PRESS_S("Save"), &save));
    CHECK(press_button_set_is_default(save, true));
    CHECK(press_button_on_clicked(save, on_save, &st, NULL));

    CHECK(press_stack_add(column, heading));
    CHECK(press_stack_add(column, st.note));
    CHECK(press_stack_add(column, row));
    CHECK(press_stack_add(column, st.status));
    CHECK(press_stack_add(column, save));
    CHECK(press_window_set_content(window, column));
    /* whatever the style, the window is as large as what is in it needs */
    CHECK(press_window_fit(window));

    if (snapshot) {
        CHECK(press_window_type_text(window, PRESS_S("Dear diary, the pixels were square today and")));
        CHECK(press_window_type_text(window, PRESS_S(" every one of them was either black or white.")));
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0)); /* a new line, not Save */
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0));
        CHECK(press_window_type_text(window, PRESS_S("It was a good day.")));
        CHECK(press_window_press_key(window, PRESS_KEY_UP, PRESS_MODIFIER_SHIFT)); /* select back a line */
        CHECK(press_window_press_key(window, PRESS_KEY_UP, PRESS_MODIFIER_SHIFT));
        CHECK(press_window_press_key(window, PRESS_KEY_TAB, 0)); /* on to the passphrase */
        CHECK(press_window_type_text(window, PRESS_S("moof!")));
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0)); /* here Return is Save */
        CHECK(press_window_snapshot(window, PRESS_S(snapshot), 2));
    } else {
        CHECK(press_window_show(window));
        CHECK(press_app_run());
    }
    CHECK(press_app_shutdown());
    return 0;
}
