/* SPDX-License-Identifier: GPL-3.0-or-later */
/* press inside a program that has its own event loop.
 *
 *   cc examples/c/hostloop.c -Iinclude -Ltarget/debug -lpress -o examples/c/hostloop
 *   LD_LIBRARY_PATH=target/debug ./examples/c/hostloop
 *
 * The loop below is a plain poll() over two descriptors: press's, and a
 * one-second timer that belongs to the program. press never blocks; it is
 * given a turn whenever its descriptor is readable. The timer's handler
 * changes a label from outside any press callback, and the change is drawn
 * because making it marks the press descriptor readable.
 */
#include <press.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/timerfd.h>
#include <unistd.h>

#define CHECK(call)                                                              \
    do {                                                                         \
        press_status_t status_ = (call);                                         \
        if (status_ != PRESS_STATUS_OK) {                                        \
            fprintf(stderr, "%s: %s\n", #call, press_status_message(status_));   \
            exit(1);                                                             \
        }                                                                        \
    } while (0)

static void on_reset(press_button_t self, void *userdata) {
    (void)self;
    *(int *)userdata = 0;
}

int main(void) {
    press_window_t window;
    press_stack_t column;
    press_label_t label;
    press_button_t reset;
    struct itimerspec every_second = {{1, 0}, {1, 0}};
    struct pollfd fds[2];
    int seconds = 0, turns = 0;
    bool running = true;

    CHECK(press_app_init());
    CHECK(press_window_new(PRESS_S("Host loop"), 220, 90, &window));
    CHECK(press_stack_new(PRESS_ORIENTATION_VERTICAL, &column));
    CHECK(press_stack_set_padding(column, 16));
    CHECK(press_stack_set_spacing(column, 12));
    CHECK(press_label_new(PRESS_S("0 seconds"), &label));
    CHECK(press_button_new(PRESS_S("Reset"), &reset));
    CHECK(press_button_on_clicked(reset, on_reset, &seconds, NULL));
    CHECK(press_stack_add(column, label));
    CHECK(press_stack_add(column, reset));
    CHECK(press_window_set_content(window, column));
    CHECK(press_window_show(window));

    CHECK(press_app_start());
    fds[0].events = POLLIN;
    CHECK(press_app_fd(&fds[0].fd));
    fds[1].events = POLLIN;
    fds[1].fd = timerfd_create(CLOCK_MONOTONIC, 0);
    timerfd_settime(fds[1].fd, 0, &every_second, NULL);

    while (running) {
        if (poll(fds, 2, -1) < 0) {
            continue; /* interrupted by a signal */
        }
        if (fds[1].revents & POLLIN) {
            /* the program's own event: nothing to do with press */
            char text[32];
            uint64_t expirations;
            if (read(fds[1].fd, &expirations, sizeof expirations) < 0) {
                break;
            }
            seconds++;
            snprintf(text, sizeof text, "%d second%s", seconds, seconds == 1 ? "" : "s");
            CHECK(press_label_set_text(label, PRESS_S(text)));
        }
        if (fds[0].revents & POLLIN) {
            CHECK(press_app_dispatch(&running));
            turns++;
        }
    }
    printf("%d seconds, %d turns given to press\n", seconds, turns);
    close(fds[1].fd);
    CHECK(press_app_stop());
    CHECK(press_app_shutdown());
    return 0;
}
