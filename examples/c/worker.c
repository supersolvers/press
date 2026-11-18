/* SPDX-License-Identifier: GPL-3.0-or-later */
/* A worker thread reporting back to the interface.
 *
 *   cc examples/c/worker.c -Iinclude -Ltarget/debug -lpress -lpthread -o examples/c/worker
 *   LD_LIBRARY_PATH=target/debug ./examples/c/worker
 *
 * Only the UI thread may call press, with one exception: press_app_post.
 * The worker counts on its own thread and posts each step; the posted
 * function then runs on the UI thread, where it is free to change widgets.
 */
#include <press.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#define CHECK(call)                                                              \
    do {                                                                         \
        press_status_t status_ = (call);                                         \
        if (status_ != PRESS_STATUS_OK) {                                        \
            fprintf(stderr, "%s: %s\n", #call, press_status_message(status_));   \
            exit(1);                                                             \
        }                                                                        \
    } while (0)

enum { STEPS = 20 };

static press_label_t label;
static press_button_t start;

/* One step of progress, allocated by the worker and freed by press. */
struct progress {
    int step;
};

/* Runs on the UI thread. */
static void show_progress(void *userdata) {
    struct progress *p = userdata;
    char text[48];
    if (p->step < STEPS) {
        snprintf(text, sizeof text, "Working: %d of %d", p->step, STEPS);
    } else {
        snprintf(text, sizeof text, "Done.");
        CHECK(press_widget_set_enabled(start, true));
    }
    CHECK(press_label_set_text(label, PRESS_S(text)));
}

/* Runs on the worker thread: no press calls here except post. */
static void *work(void *arg) {
    int step;
    (void)arg;
    for (step = 1; step <= STEPS; step++) {
        struct progress *p = malloc(sizeof *p);
        usleep(100 * 1000); /* stands in for real work */
        p->step = step;
        if (press_app_post(show_progress, p, free) != PRESS_STATUS_OK) {
            free(p); /* the interface has shut down; nothing was taken */
            break;
        }
    }
    return NULL;
}

static void on_start(press_button_t self, void *userdata) {
    pthread_t thread;
    (void)userdata;
    CHECK(press_widget_set_enabled(self, false));
    if (pthread_create(&thread, NULL, work, NULL) == 0) {
        pthread_detach(thread);
    }
}

int main(void) {
    press_window_t window;
    press_stack_t column;

    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Worker"), 220, 90, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 16));
    CHECK(press_stack_set_spacing(column, 12));
    CHECK(press_label_new(PRESS_S("Press Start."), &label));
    CHECK(press_button_new(PRESS_S("Start"), &start));
    CHECK(press_button_set_is_default(start, true));
    CHECK(press_button_on_clicked(start, on_start, NULL, NULL));
    CHECK(press_stack_add(column, label));
    CHECK(press_stack_add(column, start));
    CHECK(press_window_set_content(window, column));
    CHECK(press_window_show(window));
    if (getenv("WORKER_AUTOSTART")) {
        on_start(start, NULL); /* for trying it without a pointer */
    }
    CHECK(press_app_run());
    CHECK(press_app_shutdown());
    return 0;
}
