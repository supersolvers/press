// SPDX-License-Identifier: GPL-3.0-or-later
//! Display backends and the event pump that drives them.
//!
//! `PRESS_BACKEND=headless` runs without a display (windows can still be
//! rendered with `press_window_snapshot`); otherwise Wayland is used.
//!
//! The pump never waits by itself. It offers one file descriptor that
//! becomes readable whenever [`dispatch`] has something to do: input from
//! the display, an animation step or key repeat coming due, a helper thread
//! finishing, work posted from another thread, or the application having changed something that needs
//! drawing. Whoever owns the event loop waits on that descriptor and calls
//! `dispatch`. [`run`] is the loop for programs that have none of their own.

use crate::abi::Status;
use crate::support::{deliver, with};
use std::cell::{Cell, RefCell};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

mod a11y;
mod appmenu;
mod wayland;

struct Pump {
    /// What the host waits on: an epoll set of the descriptors below.
    epoll: OwnedFd,
    /// Fires when the next animation step or key repeat is due.
    timer: OwnedFd,
    /// Written to when work appears from outside the display connection.
    wake: Arc<OwnedFd>,
    /// The display connection; `None` with the headless backend.
    session: Option<wayland::Session>,
}

thread_local! {
    static PUMP: RefCell<Option<Pump>> = const { RefCell::new(None) };
    // A copy of the wake descriptor that can be reached while the pump is
    // borrowed, which it is for the whole of a dispatch.
    static WAKE: Cell<RawFd> = const { Cell::new(-1) };
    // Set for the length of a dispatch. What handlers change then is drawn
    // by that same dispatch, so it needs no wake-up of its own.
    static DISPATCHING: Cell<bool> = const { Cell::new(false) };
}

/// Milliseconds on a clock that never goes back, shared by every session.
fn clock() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn headless() -> Result<bool, Status> {
    match std::env::var("PRESS_BACKEND").as_deref() {
        Ok("headless") => Ok(true),
        Ok("wayland") | Err(_) => Ok(false),
        Ok(_) => Err(Status::Backend),
    }
}

fn owned(fd: RawFd) -> Result<OwnedFd, Status> {
    if fd < 0 {
        return Err(Status::Backend);
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn watch(epoll: &OwnedFd, fd: RawFd) -> Result<(), Status> {
    let mut event = libc::epoll_event { events: libc::EPOLLIN as u32, u64: fd as u64 };
    match unsafe { libc::epoll_ctl(epoll.as_raw_fd(), libc::EPOLL_CTL_ADD, fd, &mut event) } {
        0 => Ok(()),
        _ => Err(Status::Backend),
    }
}

/// Make an eventfd readable. Safe to call from any thread.
pub(crate) fn poke(fd: RawFd) {
    let one = 1u64;
    unsafe { libc::write(fd, (&one as *const u64).cast(), 8) };
}

/// Empty an eventfd or timerfd so it stops reporting readable.
fn drain(fd: &OwnedFd) {
    let mut count = 0u64;
    unsafe { libc::read(fd.as_raw_fd(), (&mut count as *mut u64).cast(), 8) };
}

/// Set the timer to fire `after` milliseconds from now, or never.
fn arm(timer: &OwnedFd, after: Option<u64>) {
    let mut spec: libc::itimerspec = unsafe { std::mem::zeroed() };
    if let Some(ms) = after.map(|ms| ms.max(1)) {
        spec.it_value.tv_sec = (ms / 1000) as libc::time_t;
        spec.it_value.tv_nsec = ((ms % 1000) * 1_000_000) as libc::c_long;
    }
    unsafe { libc::timerfd_settime(timer.as_raw_fd(), 0, &spec, std::ptr::null_mut()) };
}

pub fn started() -> bool {
    WAKE.with(|w| w.get() >= 0)
}

/// Connect to the display and make the pump's descriptor available.
pub fn start() -> Result<(), Status> {
    if started() {
        return Err(Status::InvalidState);
    }
    with(|_| Ok(()))?; // refuse before init
    let flags = libc::EFD_NONBLOCK | libc::EFD_CLOEXEC;
    let epoll = owned(unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) })?;
    let timer = owned(unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC) })?;
    let wake = Arc::new(owned(unsafe { libc::eventfd(0, flags) })?);
    watch(&epoll, timer.as_raw_fd())?;
    watch(&epoll, wake.as_raw_fd())?;
    let session = if headless()? { None } else { Some(wayland::start(wake.clone())?) };
    if let Some(session) = &session {
        watch(&epoll, session.fd())?;
    }
    with(|rt| {
        rt.quit = false;
        Ok(())
    })?;
    poke(wake.as_raw_fd()); // the first dispatch has windows to put up
    WAKE.with(|w| w.set(wake.as_raw_fd()));
    crate::support::set_wake(wake.as_raw_fd());
    PUMP.with(|p| *p.borrow_mut() = Some(Pump { epoll, timer, wake, session }));
    Ok(())
}

/// The descriptor to wait on.
pub fn fd() -> Result<i32, Status> {
    PUMP.with(|p| {
        let pump = p.try_borrow().map_err(|_| Status::InvalidState)?;
        Ok(pump.as_ref().ok_or(Status::InvalidState)?.epoll.as_raw_fd())
    })
}

/// Do everything that is ready, without waiting. Returns false once the
/// application has quit or has no window left to show.
pub fn dispatch() -> Result<bool, Status> {
    PUMP.with(|p| {
        // a handler that calls dispatch finds the pump busy and is refused
        let mut slot = p.try_borrow_mut().map_err(|_| Status::InvalidState)?;
        let pump = slot.as_mut().ok_or(Status::InvalidState)?;
        DISPATCHING.with(|d| d.set(true));
        let _done = Finally(|| DISPATCHING.with(|d| d.set(false)));
        drain(&pump.wake);
        drain(&pump.timer);
        // Work posted from other threads is done first, so that what it
        // changes is drawn by this same dispatch.
        for work in crate::support::take_posted() {
            work.run();
        }
        let now = clock();
        let alive = match &mut pump.session {
            Some(session) => session.dispatch(now)?,
            None => {
                deliver(with(|rt| Ok(rt.ui.tick(now)))?);
                // pictures that stretch hear of the room they were given, with no display too
                deliver(with(|rt| Ok(rt.ui.layout_events()))?);
                with(|rt| {
                    rt.ui.take_dirty(); // there is nothing to draw on
                    Ok(!rt.quit && rt.ui.windows().iter().any(|w| w.visible))
                })?
            }
        };
        let due = with(|rt| Ok(rt.ui.next_deadline()))?.into_iter().chain(pump.session.as_ref().and_then(|s| s.deadline())).min();
        arm(&pump.timer, due.map(|at| at.saturating_sub(clock())));
        Ok(alive)
    })
}

/// Runs a closure when dropped, however the scope is left.
struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)()
    }
}

/// Disconnect from the display. Refused from inside a dispatch.
pub fn stop() -> Result<(), Status> {
    let pump = PUMP.with(|p| Ok::<_, Status>(p.try_borrow_mut().map_err(|_| Status::InvalidState)?.take()))?;
    WAKE.with(|w| w.set(-1));
    crate::support::set_wake(-1); // before the descriptor is closed
    drop(pump); // outside the borrow: closing the session touches the runtime
    Ok(())
}

/// Called after every exported function: if the application has changed
/// something from outside a dispatch, make the descriptor readable so the
/// host's loop comes round and the change is drawn.
pub fn notice_changes() {
    let fd = WAKE.with(|w| w.get());
    if fd >= 0 && !DISPATCHING.with(|d| d.get()) && with(|rt| Ok(rt.ui.is_dirty() || rt.quit)).unwrap_or(false) {
        poke(fd);
    }
}

/// The loop for programs without one: start, wait and dispatch until the
/// application quits, stop. With the headless backend it returns at once.
pub fn run() -> Result<(), Status> {
    if headless()? {
        return Ok(());
    }
    start()?;
    let result = (|| {
        let mut poll = libc::pollfd { fd: fd()?, events: libc::POLLIN, revents: 0 };
        while dispatch()? {
            let ready = unsafe { libc::poll(&mut poll, 1, -1) };
            if ready < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                return Err(Status::Backend);
            }
        }
        Ok(())
    })();
    stop()?;
    result
}
