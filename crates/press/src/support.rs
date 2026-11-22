// SPDX-License-Identifier: GPL-3.0-or-later
//! Plumbing shared by every exported function: the per-thread runtime, the
//! panic guard, argument checks and the callback table.

use crate::abi::{self, Status};
use press_core::{Emitted, Error, Ui};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::os::fd::RawFd;
use std::sync::Mutex;
use std::thread::ThreadId;

pub use press_core::{Handle, Value};

pub type DestroyFn = unsafe extern "C" fn(userdata: *mut c_void);
pub type CallbackFn = unsafe extern "C" fn(userdata: *mut c_void);

/// A piece of work posted to the UI thread.
pub struct Work {
    function: CallbackFn,
    userdata: *mut c_void,
    destroy: Option<DestroyFn>,
}

// The caller of press_app_post promises its function and userdata may be
// used from the UI thread; that is the whole point of posting.
unsafe impl Send for Work {}

impl Work {
    pub fn new(function: Option<CallbackFn>, userdata: *mut c_void, destroy: Option<DestroyFn>) -> Result<Work, Status> {
        Ok(Work { function: function.ok_or(Status::InvalidArgument)?, userdata, destroy })
    }

    /// Do the work, then let its userdata go.
    pub fn run(self) {
        unsafe { (self.function)(self.userdata) };
        self.discard();
    }

    /// Let the userdata go without doing the work.
    pub fn discard(self) {
        if let Some(destroy) = self.destroy {
            unsafe { destroy(self.userdata) };
        }
    }
}

/// What other threads can reach: who the UI thread is, the work posted to
/// it, and the descriptor that wakes its event loop.
struct Shared {
    owner: ThreadId,
    queue: VecDeque<Work>,
    wake: RawFd,
}

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

fn shared() -> std::sync::MutexGuard<'static, Option<Shared>> {
    SHARED.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Queue work for the UI thread. Callable from any thread.
pub fn post(work: Work) -> Result<(), Status> {
    let mut guard = shared();
    let shared = guard.as_mut().ok_or(Status::NotInitialized)?;
    shared.queue.push_back(work);
    if shared.wake >= 0 {
        // under the lock, so the descriptor cannot be closed beneath us
        crate::backend::poke(shared.wake);
    }
    Ok(())
}

/// Take everything posted so far. Only the UI thread gets anything.
pub fn take_posted() -> Vec<Work> {
    match shared().as_mut() {
        Some(s) if s.owner == std::thread::current().id() => s.queue.drain(..).collect(),
        _ => Vec::new(),
    }
}

/// Tell posters which descriptor wakes the event loop, or -1 for none.
pub fn set_wake(fd: RawFd) {
    if let Some(s) = shared().as_mut().filter(|s| s.owner == std::thread::current().id()) {
        s.wake = fd;
    }
}

impl Drop for Runtime {
    /// Give up being the UI thread, on shutdown or when the thread ends.
    fn drop(&mut self) {
        let mut guard = shared();
        if guard.as_ref().is_some_and(|s| s.owner == std::thread::current().id()) {
            *guard = None;
        }
    }
}

impl From<Error> for Status {
    fn from(e: Error) -> Status {
        match e {
            Error::InvalidHandle => Status::InvalidHandle,
            Error::WrongType => Status::WrongType,
            Error::InvalidArgument => Status::InvalidArgument,
            Error::InvalidState => Status::InvalidState,
        }
    }
}

/// A connected event handler.
#[derive(Clone, Copy)]
struct Callback {
    function: *const (),
    userdata: *mut c_void,
    destroy: Option<DestroyFn>,
}

impl Callback {
    fn release(self) {
        if let Some(destroy) = self.destroy {
            unsafe { destroy(self.userdata) };
        }
    }
}

pub struct Runtime {
    pub ui: Ui,
    callbacks: HashMap<(Handle, u32), Callback>,
    pub quit: bool,
    pub running: bool,
    /// The clipboard used while no display backend is running.
    pub clipboard: String,
    /// Things other than text on that clipboard: each one's kind and itself.
    pub clipboard_data: Vec<(String, Vec<u8>)>,
    /// A display backend is running and looks after the clipboard itself.
    pub system_clipboard: bool,
}

thread_local! {
    // One runtime per thread. A call from any other thread finds none and is
    // refused, which is how the single-UI-thread rule is enforced.
    static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

pub fn init() -> Result<(), Status> {
    // One UI thread per process: posted work needs to know where to go.
    {
        let me = std::thread::current().id();
        let mut guard = shared();
        match guard.as_ref() {
            Some(s) if s.owner != me => return Err(Status::InvalidState),
            Some(_) => {}
            None => *guard = Some(Shared { owner: me, queue: VecDeque::new(), wake: -1 }),
        }
    }
    RUNTIME.with(|r| {
        let mut slot = r.borrow_mut();
        if slot.is_none() {
            let mut ui = Ui::new();
            if let Some(style) = crate::style::choose() {
                ui.set_style(style);
            }
            // a style with a frame of its own has windows draw theirs, unless
            // the user would have every frame be the desktop's
            ui.set_own_frames(std::env::var("PRESS_FRAME").map_or(true, |v| v != "desktop"));
            *slot = Some(Runtime {
                ui,
                callbacks: HashMap::new(),
                quit: false,
                running: false,
                clipboard: String::new(),
                clipboard_data: Vec::new(),
                system_clipboard: false,
            });
        }
    });
    Ok(())
}

/// Run `f` with the runtime. The borrow is released before this returns, so
/// nothing done here may call application code; see [`deliver`] for that.
pub fn with<R>(f: impl FnOnce(&mut Runtime) -> Result<R, Status>) -> Result<R, Status> {
    RUNTIME.with(|r| {
        let mut slot = r.try_borrow_mut().map_err(|_| Status::Internal)?;
        f(slot.as_mut().ok_or(Status::NotInitialized)?)
    })
}

/// Turn a Result into a status code and stop panics at the ABI boundary;
/// unwinding into foreign code is undefined behaviour.
pub fn guard(f: impl FnOnce() -> Result<(), Status>) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let result = f();
        crate::backend::notice_changes();
        result
    }));
    match result {
        Ok(Ok(())) => Status::Ok as i32,
        Ok(Err(status)) => status as i32,
        Err(_) => Status::Internal as i32,
    }
}

/// Borrow a (pointer, length) pair as UTF-8 for the duration of the call.
pub fn str_arg<'a>(ptr: *const u8, len: usize) -> Result<&'a str, Status> {
    if len == 0 {
        return Ok("");
    }
    if ptr.is_null() || len > isize::MAX as usize {
        return Err(Status::InvalidArgument);
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).map_err(|_| Status::InvalidArgument)
}

/// Bytes passed in by the caller, lent for the call.
pub fn bytes_arg<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], Status> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() || len > isize::MAX as usize {
        return Err(Status::InvalidArgument);
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// Copy a string result into the caller's buffer: the full length always goes
/// to `out_len`, and as much of the text as fits goes to `out`, cut on a
/// character boundary and NUL-terminated.
pub fn str_out(value: &str, out: *mut u8, cap: usize, out_len: *mut usize) -> Result<(), Status> {
    if out_len.is_null() || (out.is_null() && cap != 0) {
        return Err(Status::InvalidArgument);
    }
    unsafe { out_len.write(value.len()) };
    if cap > 0 {
        let mut n = value.len().min(cap - 1);
        while !value.is_char_boundary(n) {
            n -= 1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr(), out, n);
            out.add(n).write(0);
        }
    }
    Ok(())
}

/// A copy of a string with a NUL on the end, for handing to a handler.
pub fn c_text(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 1);
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    bytes
}

/// Check that `this` is a live instance of `class`.
pub fn expect(rt: &Runtime, this: Handle, class: &str) -> Result<(), Status> {
    if rt.ui.is_a(this, class)? { Ok(()) } else { Err(Status::WrongType) }
}

/// Connect, replace or (with a null callback) disconnect an event handler.
pub fn connect(
    this: Handle,
    class: &str,
    event: u32,
    function: Option<*const ()>,
    userdata: *mut c_void,
    destroy: Option<DestroyFn>,
) -> Result<(), Status> {
    let old = with(|rt| {
        expect(rt, this, class)?;
        Ok(match function {
            Some(function) => rt.callbacks.insert((this, event), Callback { function, userdata, destroy }),
            None => rt.callbacks.remove(&(this, event)),
        })
    })?;
    if let Some(old) = old {
        old.release(); // outside the borrow: the notifier may call back in
    }
    Ok(())
}

/// Whether the application has a handler for an event of an object.
pub fn listening(target: Handle, event: &str) -> bool {
    abi::event_id(event).is_some_and(|id| with(|rt| Ok(rt.callbacks.contains_key(&(target, id)))).unwrap_or(false))
}

/// Drop the handlers of destroyed objects and run their destroy notifiers.
pub fn forget(gone: &[Handle]) {
    let dropped = with(|rt| {
        let keys: Vec<_> = rt.callbacks.keys().filter(|(h, _)| gone.contains(h)).copied().collect();
        Ok(keys.into_iter().filter_map(|k| rt.callbacks.remove(&k)).collect::<Vec<_>>())
    })
    .unwrap_or_default();
    for cb in dropped {
        cb.release();
    }
}

/// Tear the runtime down, releasing every callback.
pub fn shutdown() {
    let runtime = RUNTIME.with(|r| r.try_borrow_mut().ok().and_then(|mut slot| slot.take()));
    if let Some(mut rt) = runtime {
        for (_, cb) in std::mem::take(&mut rt.callbacks) {
            cb.release();
        }
        // work that will now never be done still has its userdata released
        for work in take_posted() {
            work.discard();
        }
    }
}

/// Carry out the copies and pastes the engine has queued, against the
/// in-process clipboard. While a display backend is running this does
/// nothing: the backend serves them from the system clipboard instead.
pub fn settle_clipboard() {
    let events = with(|rt| {
        if rt.system_clipboard {
            return Ok(Vec::new());
        }
        if let Some(text) = rt.ui.take_copy() {
            rt.clipboard = text;
            rt.clipboard_data.clear();
        }
        let copies = rt.ui.take_copy_data();
        if copies.first().is_some_and(|(also, ..)| !also) {
            rt.clipboard.clear();
        }
        press_core::ui::settle_copies(&mut rt.clipboard_data, copies);
        let mut events = match rt.ui.take_paste_request() {
            Some(field) => {
                let text = rt.clipboard.clone();
                rt.ui.paste_into(field, &text).unwrap_or_default()
            }
            None => Vec::new(),
        };
        for (window, kind) in rt.ui.take_paste_data_requests() {
            let data = rt.clipboard_data.iter().find(|(have, _)| *have == kind).map(|(_, data)| data.clone()).unwrap_or_default();
            events.extend(rt.ui.paste_data_into(window, &kind, data).unwrap_or_default());
        }
        // a rich text widget that found no RTF asks again, for plain text
        if let Some(field) = rt.ui.take_paste_request() {
            let text = rt.clipboard.clone();
            events.extend(rt.ui.paste_into(field, &text).unwrap_or_default());
        }
        Ok(events)
    });
    deliver(events.unwrap_or_default());
}

/// Hand engine events to the application. Runs with no borrow held, so a
/// handler is free to call any press function, including destroying the
/// object that raised the event.
pub fn deliver(mut events: Vec<Emitted>) {
    // carets and selections the user moved are told after what changed
    events.extend(with(|rt| Ok(rt.ui.selection_events())).unwrap_or_default());
    for e in events {
        let Some(id) = abi::event_id(e.event) else { continue };
        let found = with(|rt| Ok(rt.callbacks.get(&(e.target, id)).copied())).ok().flatten();
        match found {
            Some(cb) => unsafe { abi::invoke(id, cb.function, e.target.0, &e.args, cb.userdata) },
            // Closing a window nobody is listening to ends the application.
            None if e.event == "Window.close_requested" => {
                let _ = with(|rt| {
        rt.quit = true;
        Ok(())
    });
            }
            None => {}
        }
    }
}
