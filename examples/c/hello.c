/* SPDX-License-Identifier: GPL-3.0-or-later */
/* A small dialog built through the C ABI.
 *
 *   cc examples/c/hello.c -Iinclude -Ltarget/debug -lpress -o examples/c/hello
 *   LD_LIBRARY_PATH=target/debug ./examples/c/hello
 *
 * Type a name and press Return. Tab moves between the controls, Space
 * presses the one with the dashed ring, Escape cancels. The menus work by
 * press-drag-release, by click-then-click, from the keyboard (F10) and by
 * their shortcuts.
 *
 * With --snapshot NAME it renders to PNG files without opening a window,
 * types and presses keys through the test hooks and prints what the
 * handlers saw.
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
    press_text_field_t name;
    press_label_t status;
    press_button_t ok;
    press_checkbox_t check;
    press_menu_item_t loud;
    press_menu_item_t greetings[2]; /* the Options > Greeting submenu */
    press_popup_button_t greeting_popup; /* the same choice, as a pop-up button */
    int greeting;
    bool shout;
};

static const char *const GREETINGS[2] = {"Hello", "Good day"};

static void on_ok(press_button_t self, void *userdata) {
    struct state *st = userdata;
    char name[64], text[96];
    size_t len;
    (void)self;
    /* the copy is cut to fit and always NUL-terminated; len is the full length */
    CHECK(press_text_field_get_text(st->name, name, sizeof name, &len));
    snprintf(text, sizeof text, "%s, %s%s", GREETINGS[st->greeting], name, st->shout ? "!!!" : ".");
    CHECK(press_label_set_text(st->status, PRESS_S(text)));
    printf("ok: \"%s\" (%zu bytes)\n", name, len);
}

static void on_cancel(press_button_t self, void *userdata) {
    (void)self;
    (void)userdata;
    printf("cancel\n");
    CHECK(press_app_quit());
}

/* The event brings the new text with it: OK needs a name to greet. */
static void on_changed(press_text_field_t self, const char *text, size_t text_len, void *userdata) {
    struct state *st = userdata;
    (void)self;
    (void)text;
    CHECK(press_widget_set_enabled(st->ok, text_len > 0));
}

/* Files dragged in from a file manager: the paths come one per line. */
static void on_files(press_window_t self, const char *paths, size_t paths_len, bool copy, void *userdata) {
    struct state *st = userdata;
    char text[64];
    size_t i, count = 1;
    (void)self;
    (void)copy; /* this program only reads them, so copied or moved is all one */
    for (i = 0; i < paths_len; i++) {
        count += paths[i] == '\n';
    }
    snprintf(text, sizeof text, "%zu file%s dropped.", count, count == 1 ? "" : "s");
    CHECK(press_label_set_text(st->status, PRESS_S(text)));
    printf("dropped:\n%s\n", paths);
}

/* The checkbox and the Options menu item show the same setting. */
static void set_shout(struct state *st, bool shout) {
    st->shout = shout;
    CHECK(press_checkbox_set_checked(st->check, shout));
    CHECK(press_menu_item_set_checked(st->loud, shout));
    printf("loud: %s\n", shout ? "on" : "off");
}

static void on_toggled(press_checkbox_t self, bool checked, void *userdata) {
    (void)self;
    set_shout(userdata, checked);
}

static void on_loud(press_menu_item_t self, void *userdata) {
    struct state *st = userdata;
    (void)self;
    set_shout(st, !st->shout);
}

/* The greeting can be set from the Options menu or the pop-up button;
 * whichever is used, the other is brought into line. */
static void set_greeting(struct state *st, int index) {
    int i;
    st->greeting = index;
    for (i = 0; i < 2; i++) {
        CHECK(press_menu_item_set_checked(st->greetings[i], i == index));
    }
    CHECK(press_popup_button_set_selected(st->greeting_popup, index));
    printf("greeting: %s\n", GREETINGS[index]);
}

static void on_greeting(press_menu_item_t self, void *userdata) {
    struct state *st = userdata;
    set_greeting(st, st->greetings[1] == self);
}

static void on_greeting_popup(press_popup_button_t self, int32_t index, void *userdata) {
    (void)self;
    set_greeting(userdata, index);
}

static void on_quit(press_menu_item_t self, void *userdata) {
    (void)self;
    (void)userdata;
    printf("quit\n");
    CHECK(press_app_quit());
}

/* Edit menu: each item acts on the name field. */
static void on_cut(press_menu_item_t self, void *userdata) {
    (void)self;
    CHECK(press_text_field_cut(*(press_text_field_t *)userdata));
}

static void on_copy(press_menu_item_t self, void *userdata) {
    (void)self;
    CHECK(press_text_field_copy(*(press_text_field_t *)userdata));
}

static void on_paste(press_menu_item_t self, void *userdata) {
    (void)self;
    CHECK(press_text_field_paste(*(press_text_field_t *)userdata));
}

static void on_select_all(press_menu_item_t self, void *userdata) {
    (void)self;
    CHECK(press_text_field_select_all(*(press_text_field_t *)userdata));
}

/* Create an item, give it a shortcut and a handler, and append it. */
static press_menu_item_t add_item(press_menu_t menu, const char *label, const char *key,
                                  press_menu_item_activated_fn handler, void *userdata) {
    press_menu_item_t item;
    CHECK(press_menu_item_new(PRESS_S(label), &item));
    CHECK(press_menu_item_set_shortcut(item, PRESS_S(key)));
    CHECK(press_menu_item_on_activated(item, handler, userdata, NULL));
    CHECK(press_menu_add(menu, item));
    return item;
}

int main(int argc, char **argv) {
    const char *snapshot = (argc == 3 && strcmp(argv[1], "--snapshot") == 0) ? argv[2] : NULL;
    press_window_t window;
    press_stack_t column, buttons, greeting_row;
    press_label_t greeting_label;
    press_label_t title, status;
    press_text_field_t name;
    press_checkbox_t check;
    press_button_t ok, cancel;
    press_menu_bar_t bar;
    press_menu_t file, edit, options, greeting;
    press_menu_item_t greeting_item;
    struct state st = {0};
    uint32_t abi;

    CHECK(press_app_init());
    CHECK(press_app_abi_version(&abi));
    if (abi != PRESS_ABI_VERSION) {
        fprintf(stderr, "libpress has ABI %u, this program was built for %d\n", abi, PRESS_ABI_VERSION);
        return 1;
    }

    CHECK(press_window_new(PRESS_S("Hello from C"), 300, 228, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 16));
    CHECK(press_stack_set_spacing(column, 12));

    CHECK(press_label_new(PRESS_S("What is your name?"), &title));
    CHECK(press_text_field_new(PRESS_S(""), &name));
    CHECK(press_text_field_set_placeholder(name, PRESS_S("Type a name")));
    CHECK(press_text_field_set_min_width(name, 268));
    CHECK(press_checkbox_new(PRESS_S("Say it loudly"), &check));
    CHECK(press_label_new(PRESS_S("Press Return to say hello."), &status));

    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &buttons));
    CHECK(press_stack_set_spacing(buttons, 12));
    CHECK(press_stack_set_align(buttons, PRESS_ALIGN_CENTER));
    CHECK(press_button_new(PRESS_S("Cancel"), &cancel));
    CHECK(press_button_set_is_cancel(cancel, true));
    CHECK(press_button_new(PRESS_S("OK"), &ok));
    CHECK(press_button_set_is_default(ok, true));
    CHECK(press_widget_set_enabled(ok, false));
    CHECK(press_stack_add(buttons, cancel));
    CHECK(press_stack_add(buttons, ok));

    CHECK(press_stack_add(column, title));
    CHECK(press_stack_add(column, name));
    CHECK(press_stack_add(column, check));

    /* a pop-up button: one choice showing, the rest a press away */
    CHECK(press_stack_new(PRESS_ORIENTATION_HORIZONTAL, &greeting_row));
    CHECK(press_stack_set_align(greeting_row, PRESS_ALIGN_CENTER));
    CHECK(press_label_new(PRESS_S("Greeting:"), &greeting_label));
    CHECK(press_popup_button_new(&st.greeting_popup));
    CHECK(press_popup_button_add(st.greeting_popup, PRESS_S(GREETINGS[0])));
    CHECK(press_popup_button_add(st.greeting_popup, PRESS_S(GREETINGS[1])));
    CHECK(press_popup_button_on_changed(st.greeting_popup, on_greeting_popup, &st, NULL));
    CHECK(press_stack_add(greeting_row, greeting_label));
    CHECK(press_stack_add(greeting_row, st.greeting_popup));
    CHECK(press_stack_add(column, greeting_row));
    CHECK(press_stack_add(column, status));
    CHECK(press_stack_add(column, buttons));
    CHECK(press_window_set_content(window, column));

    st.name = name;
    st.status = status;
    st.ok = ok;
    st.check = check;

    CHECK(press_menu_bar_new(&bar));
    CHECK(press_menu_new(PRESS_S("File"), &file));
    CHECK(press_menu_new(PRESS_S("Edit"), &edit));
    CHECK(press_menu_new(PRESS_S("Options"), &options));
    add_item(file, "Quit", "Q", on_quit, NULL);
    add_item(edit, "Cut", "X", on_cut, &st.name);
    add_item(edit, "Copy", "C", on_copy, &st.name);
    add_item(edit, "Paste", "V", on_paste, &st.name);
    CHECK(press_menu_add_separator(edit));
    add_item(edit, "Select All", "A", on_select_all, &st.name);
    st.loud = add_item(options, "Say It Loudly", "L", on_loud, &st);
    /* a submenu: an item that opens another menu beside it */
    CHECK(press_menu_new(PRESS_S(""), &greeting));
    st.greetings[0] = add_item(greeting, GREETINGS[0], "", on_greeting, &st);
    st.greetings[1] = add_item(greeting, GREETINGS[1], "", on_greeting, &st);
    CHECK(press_menu_item_set_checked(st.greetings[0], true));
    CHECK(press_menu_item_new(PRESS_S("Greeting"), &greeting_item));
    CHECK(press_menu_item_set_submenu(greeting_item, greeting));
    CHECK(press_menu_add(options, greeting_item));
    CHECK(press_menu_bar_add(bar, file));
    CHECK(press_menu_bar_add(bar, edit));
    CHECK(press_menu_bar_add(bar, options));
    CHECK(press_window_set_menu_bar(window, bar));
    /* whatever the style, the window is as large as what is in it needs,
     * its menu bar with it */
    CHECK(press_window_fit(window));

    /* A context menu for the whole window: right-click anywhere but the name
     * field, which pops up its own Cut, Copy and Paste. */
    {
        press_menu_t context;
        CHECK(press_menu_new(PRESS_S(""), &context));
        add_item(context, "Say It Loudly", "", on_loud, &st);
        CHECK(press_menu_add_separator(context));
        add_item(context, "Quit", "", on_quit, NULL);
        CHECK(press_widget_set_context_menu(window, context));
    }
    CHECK(press_button_on_clicked(ok, on_ok, &st, NULL));
    CHECK(press_button_on_clicked(cancel, on_cancel, NULL, NULL));
    CHECK(press_text_field_on_changed(name, on_changed, &st, NULL));
    CHECK(press_checkbox_on_toggled(check, on_toggled, &st, NULL));
    CHECK(press_window_on_files_dropped(window, on_files, &st, NULL));

    if (snapshot) {
        char path[512];
        snprintf(path, sizeof path, "%s-1.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        /* the first text field has the keyboard, as in a classic dialog */
        CHECK(press_window_type_text(window, PRESS_S("Clarus the dogcow")));
        CHECK(press_window_press_key(window, PRESS_KEY_LEFT, PRESS_MODIFIER_CONTROL | PRESS_MODIFIER_SHIFT));
        snprintf(path, sizeof path, "%s-2.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        CHECK(press_window_press_key(window, PRESS_KEY_BACKSPACE, 0)); /* deletes the selected word */
        CHECK(press_window_type_text(window, PRESS_S("Dogcow"))); /* typing would have replaced it anyway */
        CHECK(press_window_press_key(window, PRESS_KEY_TAB, 0)); /* on to the checkbox */
        CHECK(press_window_type_text(window, PRESS_S(" ")));     /* Space presses it */
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0)); /* the default button */
        snprintf(path, sizeof path, "%s-3.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        /* the menus from the keyboard: pull down File, across to Edit, down to Copy */
        CHECK(press_window_press_key(window, PRESS_KEY_MENU, 0));
        CHECK(press_window_press_key(window, PRESS_KEY_RIGHT, 0));
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));
        snprintf(path, sizeof path, "%s-4.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        CHECK(press_window_press_key(window, PRESS_KEY_ESCAPE, 0));
        /* into a submenu: Options > Greeting > Good day */
        CHECK(press_window_press_key(window, PRESS_KEY_MENU, 0));
        CHECK(press_window_press_key(window, PRESS_KEY_LEFT, 0));  /* wraps round to Options */
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));  /* Greeting */
        CHECK(press_window_press_key(window, PRESS_KEY_RIGHT, 0)); /* into its submenu */
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));
        snprintf(path, sizeof path, "%s-5.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0));
        /* right-click the name field: the built-in editing menu */
        CHECK(press_widget_focus(name));
        CHECK(press_window_context_click_at(window, 150, 62));
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));
        CHECK(press_window_press_key(window, PRESS_KEY_DOWN, 0));
        snprintf(path, sizeof path, "%s-6.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        CHECK(press_window_press_key(window, PRESS_KEY_ESCAPE, 0));
        /* the pop-up button: its list comes up over it, current choice first */
        CHECK(press_widget_focus(st.greeting_popup));
        CHECK(press_window_type_text(window, PRESS_S(" ")));
        snprintf(path, sizeof path, "%s-7.png", snapshot);
        CHECK(press_window_snapshot(window, PRESS_S(path), 2));
        CHECK(press_window_press_key(window, PRESS_KEY_UP, 0));
        CHECK(press_window_press_key(window, PRESS_KEY_RETURN, 0));
        /* a stale handle must be refused, not crash */
        CHECK(press_widget_destroy(status));
        printf("stale handle: %s\n", press_status_message(press_label_set_text(status, PRESS_S("x"))));
        printf("wrong type: %s\n", press_status_message(press_label_set_text(ok, PRESS_S("x"))));
    } else {
        CHECK(press_window_show(window));
        CHECK(press_app_run());
    }
    CHECK(press_app_shutdown());
    return 0;
}
