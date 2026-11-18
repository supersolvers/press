// SPDX-License-Identifier: GPL-3.0-or-later
//! Exercises the exported C functions exactly as a foreign caller would.

use press::abi::*;
use std::ffi::c_void;

const OK: i32 = Status::Ok as i32;

/// A process has one UI thread at a time, and the test harness runs each
/// test on a thread of its own, so the tests take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn my_turn() -> std::sync::MutexGuard<'static, ()> {
    TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

unsafe fn s(text: &str) -> (*const u8, usize) {
    (text.as_ptr(), text.len())
}

unsafe extern "C" fn count(_this: u64, userdata: *mut c_void) {
    *(userdata as *mut u32) += 1;
}

/// Counts, and keeps the text the event brought.
static LAST_TEXT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

unsafe extern "C" fn count_text(_this: u64, text: *const u8, len: usize, userdata: *mut c_void) {
    *(userdata as *mut u32) += 1;
    assert_eq!(*text.add(len), 0); // NUL-terminated for C's benefit
    *LAST_TEXT.lock().unwrap() = String::from_utf8(std::slice::from_raw_parts(text, len).to_vec()).unwrap();
}

fn last_text() -> String {
    LAST_TEXT.lock().unwrap().clone()
}

unsafe extern "C" fn dropped(userdata: *mut c_void) {
    *(userdata as *mut u32) += 100;
}

/// A handler that destroys the very button that raised the event.
unsafe extern "C" fn destroy_self(this: u64, _userdata: *mut c_void) {
    assert_eq!(press_widget_destroy(this), OK);
}

/// Read a text field the way a C caller does, into a buffer of `cap` bytes.
unsafe fn read(field: u64, cap: usize) -> (i32, String, usize) {
    let mut buf = vec![0xAAu8; cap];
    let mut len = usize::MAX;
    let status = press_text_field_get_text(field, buf.as_mut_ptr(), cap, &mut len);
    let end = buf.iter().position(|b| *b == 0).unwrap_or(cap);
    (status, String::from_utf8(buf[..end].to_vec()).unwrap(), len)
}

/// Keys, focus, text fields and string results, driven through the ABI.
unsafe fn keyboard_and_text(window: u64) {
    let (mut column, mut field, mut ok) = (0u64, 0u64, 0u64);
    assert_eq!(press_stack_new(Orientation::Vertical as i32, &mut column), OK);
    let (p, n) = s("");
    assert_eq!(press_text_field_new(p, n, &mut field), OK);
    let (p, n) = s("OK");
    assert_eq!(press_button_new(p, n, &mut ok), OK);
    assert_eq!(press_button_set_is_default(ok, true), OK);
    assert_eq!(press_stack_add(column, field), OK);
    assert_eq!(press_stack_add(column, ok), OK);
    assert_eq!(press_window_set_content(window, column), OK);

    let (mut changed, mut submitted, mut clicked) = (0u32, 0u32, 0u32);
    let data = |c: &mut u32| c as *mut u32 as *mut c_void;
    assert_eq!(press_text_field_on_changed(field, Some(count_text), data(&mut changed), None), OK);
    assert_eq!(press_text_field_on_submitted(field, Some(count_text), data(&mut submitted), None), OK);
    assert_eq!(press_button_on_clicked(ok, Some(count), data(&mut clicked), None), OK);

    // typing lands in the first text field without anyone focusing it
    let (p, n) = s("naïve");
    assert_eq!(press_window_type_text(window, p, n), OK);
    assert_eq!(changed, 1);
    assert_eq!(last_text(), "naïve"); // the event carried the text
    assert_eq!(read(field, 64), (OK, "naïve".to_string(), 6));

    // string results: length query, exact fit, and a cut that respects UTF-8
    let mut len = 0usize;
    assert_eq!(press_text_field_get_text(field, std::ptr::null_mut(), 0, &mut len), OK);
    assert_eq!(len, 6);
    assert_eq!(read(field, 7), (OK, "naïve".to_string(), 6));
    assert_eq!(read(field, 4), (OK, "na".to_string(), 6)); // room for 3 bytes would split the ï
    assert_eq!(read(field, 1), (OK, String::new(), 6));
    assert_eq!(press_text_field_get_text(field, std::ptr::null_mut(), 8, &mut len), Status::InvalidArgument as i32);
    assert_eq!(press_text_field_get_text(field, std::ptr::null_mut(), 0, std::ptr::null_mut()), Status::InvalidArgument as i32);
    assert_eq!(press_text_field_get_text(ok, std::ptr::null_mut(), 0, &mut len), Status::WrongType as i32);

    // keys and modifiers
    assert_eq!(press_window_press_key(window, Key::Home as i32, MODIFIER_SHIFT), OK);
    assert_eq!(press_window_press_key(window, Key::Delete as i32, 0), OK);
    assert_eq!((changed, read(field, 64).2), (2, 0));
    assert_eq!(last_text(), ""); // an empty string is still a valid pointer
    assert_eq!(press_window_press_key(window, 99, 0), Status::InvalidArgument as i32);
    assert_eq!(press_window_press_key(window, Key::Tab as i32, 0x80), Status::InvalidArgument as i32);
    assert_eq!(press_window_press_key(ok, Key::Tab as i32, 0), Status::WrongType as i32);

    // Return submits the field and presses the default button
    assert_eq!(press_window_press_key(window, Key::Return as i32, 0), OK);
    assert_eq!((submitted, clicked), (1, 1));

    // Tab to the button, Space presses it; a set from the program raises nothing
    assert_eq!(press_window_press_key(window, Key::Tab as i32, 0), OK);
    let (p, n) = s(" ");
    assert_eq!(press_window_type_text(window, p, n), OK);
    assert_eq!(clicked, 2);
    let (p, n) = s("set");
    assert_eq!(press_text_field_set_text(field, p, n), OK);
    assert_eq!(changed, 2);

    // cut, copy and paste; with no display running the clipboard is in-process
    let (p, n) = s("cut me");
    assert_eq!(press_text_field_set_text(field, p, n), OK);
    assert_eq!(press_text_field_select_all(field), OK);
    assert_eq!(press_text_field_cut(field), OK);
    assert_eq!((changed, read(field, 64).1), (3, String::new()));
    assert_eq!(press_text_field_paste(field), OK);
    assert_eq!(press_text_field_paste(field), OK);
    assert_eq!((changed, read(field, 64).1), (5, "cut mecut me".to_string()));
    assert_eq!(press_text_field_copy(field), OK); // nothing selected: clipboard untouched
    assert_eq!(press_text_field_select_all(field), OK);
    assert_eq!(press_text_field_paste(field), OK);
    assert_eq!(read(field, 64).1, "cut me");
    assert_eq!(press_text_field_copy(ok), Status::WrongType as i32);

    // focus rules
    assert_eq!(press_widget_focus(field), OK);
    assert_eq!(press_widget_focus(column), Status::WrongType as i32);
    assert_eq!(press_widget_set_enabled(ok, false), OK);
    assert_eq!(press_widget_focus(ok), Status::InvalidState as i32);
}

#[test]
fn abi_round_trip() {
    let _turn = my_turn();
    unsafe {
        // nothing works before init
        let mut h = 0u64;
        let (p, n) = s("x");
        assert_eq!(press_label_new(p, n, &mut h), Status::NotInitialized as i32);

        assert_eq!(press_app_init(), OK);
        let mut version = 0u32;
        assert_eq!(press_app_abi_version(&mut version), OK);
        assert_eq!(version, ABI_VERSION);

        let (mut window, mut button) = (0u64, 0u64);
        let (p, n) = s("Test");
        assert_eq!(press_window_new(p, n, 120, 60, &mut window), OK);
        let (p, n) = s("Go");
        assert_eq!(press_button_new(p, n, &mut button), OK);
        assert_eq!(press_window_set_content(window, button), OK);

        // argument checking
        assert_eq!(press_button_new(p, n, std::ptr::null_mut()), Status::InvalidArgument as i32);
        assert_eq!(press_button_new([0xffu8, 0xfe].as_ptr(), 2, &mut h), Status::InvalidArgument as i32);
        assert_eq!(press_stack_new(99, &mut h), Status::InvalidArgument as i32);
        assert_eq!(press_label_set_text(button, p, n), Status::WrongType as i32);
        assert_eq!(press_widget_set_enabled(0, true), Status::InvalidHandle as i32);
        assert_eq!(press_window_new(p, n, 0, 10, &mut h), Status::InvalidArgument as i32);

        // a click reaches the handler; replacing it runs the old destroy notifier
        let mut counter = 0u32;
        let data = &mut counter as *mut u32 as *mut c_void;
        assert_eq!(press_button_on_clicked(button, Some(count), data, Some(dropped)), OK);
        assert_eq!(press_window_click_at(window, 10, 10), OK);
        assert_eq!(counter, 1);
        assert_eq!(press_button_on_clicked(button, Some(destroy_self), std::ptr::null_mut(), None), OK);
        assert_eq!(counter, 101);

        // a handler may destroy its own widget; the handle is then stale
        assert_eq!(press_window_click_at(window, 10, 10), OK);
        let mut enabled = false;
        assert_eq!(press_widget_get_enabled(button, &mut enabled), Status::InvalidHandle as i32);

        // destroying an object releases its callbacks
        let mut check = 0u64;
        let (p, n) = s("Option");
        assert_eq!(press_checkbox_new(p, n, &mut check), OK);
        let mut released = 0u32;
        let data = &mut released as *mut u32 as *mut c_void;
        assert_eq!(press_checkbox_on_toggled(check, None, data, Some(dropped)), OK); // disconnect: nothing stored
        assert_eq!(press_widget_destroy(check), OK);
        assert_eq!(released, 0);

        keyboard_and_text(window);

        assert_eq!(press_app_shutdown(), OK);
        assert_eq!(press_app_quit(), Status::NotInitialized as i32);
    }
}

#[test]
fn other_threads_are_refused() {
    let _turn = my_turn();
    unsafe {
        assert_eq!(press_app_init(), OK);
        assert_eq!(press_app_init(), OK); // again from the same thread is fine
        let status = std::thread::spawn(|| press_app_quit()).join().unwrap();
        assert_eq!(status, Status::NotInitialized as i32);
        // while this thread is the UI thread, no other can become it
        let status = std::thread::spawn(|| press_app_init()).join().unwrap();
        assert_eq!(status, Status::InvalidState as i32);
        assert_eq!(press_app_shutdown(), OK);
        // once it has shut down, another may
        let status = std::thread::spawn(|| (press_app_init(), press_app_shutdown())).join().unwrap();
        assert_eq!(status, (OK, OK));
    }
}

/// Whether a descriptor is readable within `timeout` milliseconds.
fn readable(fd: i32, timeout: i32) -> bool {
    let mut poll = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    unsafe { libc::poll(&mut poll, 1, timeout) == 1 }
}

/// A host event loop driving press through its descriptor. Runs headless so
/// it needs no display; the descriptor behaves the same either way.
#[test]
fn a_host_loop_drives_press_through_one_descriptor() {
    let _turn = my_turn();
    unsafe {
        std::env::set_var("PRESS_BACKEND", "headless");
        assert_eq!(press_app_init(), OK);
        let (mut fd, mut alive) = (-1i32, false);
        assert_eq!(press_app_fd(&mut fd), Status::InvalidState as i32); // not started
        assert_eq!(press_app_dispatch(&mut alive), Status::InvalidState as i32);
        assert_eq!(press_app_stop(), Status::InvalidState as i32);

        let (mut window, mut label, mut field, mut column) = (0u64, 0u64, 0u64, 0u64);
        let (p, n) = s("Loop");
        assert_eq!(press_window_new(p, n, 200, 80, &mut window), OK);
        assert_eq!(press_stack_new(Orientation::Vertical as i32, &mut column), OK);
        assert_eq!(press_label_new(p, n, &mut label), OK);
        assert_eq!(press_stack_add(column, label), OK);
        assert_eq!(press_window_set_content(window, column), OK);
        assert_eq!(press_window_show(window), OK);

        assert_eq!(press_app_start(), OK);
        assert_eq!(press_app_start(), Status::InvalidState as i32);
        assert_eq!(press_app_run(), Status::InvalidState as i32);
        assert_eq!(press_app_fd(&mut fd), OK);
        assert!(fd >= 0);

        // readable at once: there are windows to put up
        assert!(readable(fd, 0));
        assert_eq!(press_app_dispatch(&mut alive), OK);
        assert!(alive);
        // then quiet: nothing is moving and nothing has changed
        assert!(!readable(fd, 50));

        // a change made from outside a dispatch brings the loop round
        let (p, n) = s("changed");
        assert_eq!(press_label_set_text(label, p, n), OK);
        assert!(readable(fd, 0));
        assert_eq!(press_app_dispatch(&mut alive), OK);
        assert!(!readable(fd, 50));

        // a blinking caret needs a turn about twice a second, with no input at all
        let (p, n) = s("");
        assert_eq!(press_text_field_new(p, n, &mut field), OK);
        assert_eq!(press_stack_add(column, field), OK);
        assert_eq!(press_widget_focus(field), OK);
        assert_eq!(press_app_dispatch(&mut alive), OK);
        assert!(!readable(fd, 100));
        assert!(readable(fd, 1000));
        assert_eq!(press_app_dispatch(&mut alive), OK);

        // quit is reported by the next dispatch
        assert_eq!(press_app_quit(), OK);
        assert!(readable(fd, 0));
        assert_eq!(press_app_dispatch(&mut alive), OK);
        assert!(!alive);

        assert_eq!(press_app_stop(), OK);
        assert_eq!(press_app_fd(&mut fd), Status::InvalidState as i32);
        // it can be started again, and shutdown stops it
        assert_eq!(press_app_start(), OK);
        assert_eq!(press_app_shutdown(), OK);
    }
}

/// What a posted piece of work is given: the label to change and a log of
/// what happened, shared with the test.
struct Job {
    label: u64,
    text: &'static str,
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

unsafe extern "C" fn do_job(userdata: *mut c_void) {
    let job = &*(userdata as *const Job);
    // posted work runs on the UI thread, so it may call press
    let status = press_label_set_text(job.label, job.text.as_ptr(), job.text.len());
    job.log.lock().unwrap().push(format!("ran {} on {:?} -> {status}", job.text, std::thread::current().id()));
}

unsafe extern "C" fn free_job(userdata: *mut c_void) {
    let job = Box::from_raw(userdata as *mut Job);
    job.log.lock().unwrap().push(format!("freed {}", job.text));
}

#[test]
fn work_posted_from_other_threads_runs_on_the_ui_thread() {
    let _turn = my_turn();
    unsafe {
        std::env::set_var("PRESS_BACKEND", "headless");
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let job = |label: u64, text: &'static str| Box::into_raw(Box::new(Job { label, text, log: log.clone() })) as usize;

        // before init there is nowhere to post to, and nothing is taken
        let orphan = job(0, "orphan");
        assert_eq!(press_app_post(Some(do_job), orphan as *mut c_void, Some(free_job)), Status::NotInitialized as i32);
        drop(Box::from_raw(orphan as *mut Job));

        assert_eq!(press_app_init(), OK);
        assert_eq!(press_app_post(None, std::ptr::null_mut(), None), Status::InvalidArgument as i32);
        let (mut window, mut label) = (0u64, 0u64);
        let (p, n) = s("start");
        assert_eq!(press_window_new(p, n, 100, 40, &mut window), OK);
        assert_eq!(press_label_new(p, n, &mut label), OK);
        assert_eq!(press_window_set_content(window, label), OK);
        assert_eq!(press_window_show(window), OK);
        assert_eq!(press_app_start(), OK);
        let (mut fd, mut alive) = (-1, false);
        assert_eq!(press_app_fd(&mut fd), OK);
        assert_eq!(press_app_dispatch(&mut alive), OK);
        assert!(!readable(fd, 50));

        // two workers post; the loop is woken, and the work runs here, in order
        let ui = std::thread::current().id();
        for text in ["one", "two"] {
            let posted = job(label, text);
            let status = std::thread::spawn(move || press_app_post(Some(do_job), posted as *mut c_void, Some(free_job))).join().unwrap();
            assert_eq!(status, OK);
        }
        assert!(log.lock().unwrap().is_empty()); // nothing runs on the posting thread
        assert!(readable(fd, 1000));
        assert_eq!(press_app_dispatch(&mut alive), OK);
        let expected: Vec<String> =
            ["one", "two"].iter().flat_map(|t| [format!("ran {t} on {ui:?} -> {OK}"), format!("freed {t}")]).collect();
        assert_eq!(*log.lock().unwrap(), expected);
        assert!(!readable(fd, 50));

        // work still waiting at shutdown is not run, but its userdata is released
        log.lock().unwrap().clear();
        let late = job(label, "late");
        assert_eq!(press_app_post(Some(do_job), late as *mut c_void, Some(free_job)), OK);
        assert_eq!(press_app_shutdown(), OK);
        assert_eq!(*log.lock().unwrap(), ["freed late"]);
    }
}
