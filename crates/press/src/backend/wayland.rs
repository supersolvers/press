// SPDX-License-Identifier: GPL-3.0-or-later
//! Wayland backend: one xdg-toplevel per visible press window, drawn into
//! shared-memory buffers. The compositor is asked to draw the title bar and
//! frame (server-side decoration), so windows wear whatever the desktop uses.
//!
//!
//! Keys are interpreted with the compositor's keymap (libxkbcommon), so the
//! user's layout, dead keys and compose sequences work. Held keys repeat at
//! the rate the compositor asks for.
//!
//! Nothing here waits. [`Session::dispatch`] does whatever is ready and
//! returns, so the loop that waits can be press's own (`backend::run`) or
//! the host program's.
//!
//! Copy and paste use the Wayland selection, so they work with every other
//! application. Text is offered and accepted as UTF-8. Pipes are read and
//! written on short-lived threads so a slow peer cannot stall the interface.
//!
//! An open menu is an xdg-popup hung from its window, so it can reach past
//! the window's edge. It takes no grab: keys keep going to the window, and
//! the menu is put away when the window loses the keyboard. Pointer events
//! on the popup are translated back into the window's coordinates.
//!
//! Scale. One engine pixel is drawn as a square of `zoom x scale` device
//! pixels, never smoothed. `scale` is the whole-number buffer scale the
//! compositor asks for on a HiDPI output. `zoom` is the user's own
//! magnification, set with `PRESS_SCALE` (1 to 8), for people who want the
//! chunky pixels bigger than the screen needs.
//!
//! Where the compositor's scale is not a whole number (one and a half,
//! say) a picture drawn at a whole scale would be resampled and soften,
//! and one-pixel patterns cannot survive that. So there the picture is
//! drawn straight in the screen's own pixels: an engine pixel is a square
//! of `zoom` times the scale rounded to the nearest whole number, and the
//! compositor is told (fractional-scale and viewporter) that the picture
//! is already at the screen's size and is to be shown as it is. Things
//! come out a little larger or smaller than the compositor's scale asks
//! for, and sharp.
//!
//! The pointer is drawn by press too, in one bit and at the same scale.
//!
//! Input methods (for Chinese, Japanese, Korean and the like) are reached
//! through the text-input protocol: while a text field has the keyboard the
//! input method is switched on and told where the caret is, and the text it
//! composes and commits is passed to the engine.
//!
//! Drag and drop. Text dragged in from another program can be dropped on a
//! text field, and files on any window whose application listens for
//! them; selected text can be dragged out. Those drags copy; they never move.
//!
//! Rows pulled loose in a list are handed to the desktop as a drag too,
//! which is what lets them cross from one window of the program to
//! another. Dropped on a list they move, and the engine does the moving;
//! dropped on anything that takes text they arrive as their labels.
//! Dropped on a list in another press program they move there: that
//! program reads them, and when it has, they are removed here.
//!
//! The menu bar. On a desktop that shows menus itself, across the top of
//! the screen (KDE Plasma's Global Menu), each window's menus are handed to
//! it and the window draws no bar; see `appmenu`. Otherwise the bar is
//! drawn in the window. `PRESS_MENU_BAR=window` keeps it in the window
//! regardless.
//!

use crate::abi::Status;
use super::a11y::{self, Bridge};
use super::appmenu;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::{self, WpFractionalScaleV1};
use wayland_protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};
use wayland_protocols::xdg::toplevel_tag::v1::client::xdg_toplevel_tag_manager_v1::XdgToplevelTagManagerV1;
use wayland_protocols_plasma::appmenu::client::{org_kde_kwin_appmenu::OrgKdeKwinAppmenu, org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager};
use crate::support::{deliver, listening, with, Handle};
use press_core::{Canvas, Cursor, Key, Mods, PointerKind, Rect};
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{self, ContentHint, ContentPurpose, ZwpTextInputV3},
};
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity};
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd, OwnedFd, RawFd};
use std::sync::{mpsc, Arc};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    data_device_manager::{
        data_device::{DataDevice, DataDeviceHandler},
        data_offer::{DataOfferHandler, DragOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
        DataDeviceManagerState, WritePipe,
    },
    delegate_data_device, delegate_xdg_popup,
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RepeatInfo},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            popup::{Popup, PopupConfigure, PopupHandler},
            window::{DecorationMode, Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgPositioner, XdgShell, XdgSurface,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{
        wl_data_device::WlDataDevice, wl_data_device_manager::DndAction, wl_data_source::WlDataSource, wl_keyboard,
        wl_output, wl_pointer, wl_seat, wl_shm, wl_surface,
    },
    backend::WaylandError,
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
/// The side buttons: back and forward, by the names mice give them.
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;
const BTN_FORWARD: u32 = 0x115;
const BTN_BACK: u32 = 0x116;
const BTN_MIDDLE: u32 = 0x112;

/// The forms in which text is offered to and accepted from other
/// applications, best first. All are treated as UTF-8.
const TEXT_MIMES: [&str; 5] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain", "TEXT", "STRING"];

/// How rows dragged out of a list are offered to lists. Between two lists
/// of one program nothing is read in this form: seeing it says the drag is
/// ours, and the engine already knows which rows are on the move. Another
/// press program reads the rows from it, written out by the engine.
const ROWS_MIME: &str = "application/x-press-rows";

/// How a list of dragged files is offered: one URI per line.
const URI_LIST: &str = "text/uri-list";

/// Something being dragged over one of our windows.
struct DragIn {
    window: Handle,
    /// Where it is, in engine pixels.
    at: (i32, i32),
    /// The text form it offers, if any, and whether it offers files.
    text_mime: Option<String>,
    files: bool,
    /// The first kind the window has said it takes that is on offer.
    kind: Option<String>,
    /// The form we have said we would take where it now is.
    accepted: Option<String>,
}

/// What a drop delivered, read on a helper thread.
struct Dropped {
    window: Handle,
    at: (i32, i32),
    files: bool,
    /// Rows from another program's list, to go where the engine last marked.
    rows: bool,
    /// Files: the user asked for copies, not for the files to move.
    copy: bool,
    /// Neither text nor files: the kind of thing the window asked for.
    kind: Option<String>,
    bytes: Vec<u8>,
}

/// The most that is read of something pasted or dropped that is not text:
/// room for a large picture, and an end to anything that never stops.
const DATA_LIMIT: u64 = 64 * 1024 * 1024;

/// Turn a text/uri-list into local paths, one per line. Entries that are
/// not local files are left out.
/// A path as a text/uri-list wants it: bytes other than the plainest
/// written as a per cent sign and two hexadecimal digits.
fn uri_escaped(path: &str) -> String {
    path.bytes().map(|b| if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn paths_from_uri_list(list: &str) -> String {
    let mut paths = Vec::new();
    for line in list.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let Some(rest) = line.strip_prefix("file://") else { continue };
        // an optional host name comes before the path
        let Some(path) = rest.find('/').map(|i| &rest[i..]) else { continue };
        let (mut bytes, mut it) = (Vec::new(), path.bytes());
        while let Some(b) = it.next() {
            let hex = |c: Option<u8>| c.and_then(|c| (c as char).to_digit(16));
            match b {
                b'%' => match (hex(it.next()), hex(it.next())) {
                    (Some(hi), Some(lo)) => bytes.push((hi * 16 + lo) as u8),
                    _ => bytes.clear(),
                },
                _ => bytes.push(b),
            }
        }
        if let Ok(path) = String::from_utf8(bytes) {
            if !path.is_empty() && !path.contains('\n') {
                paths.push(path);
            }
        }
    }
    paths.join("\n")
}

/// The most that will be read from another application's clipboard.
const PASTE_LIMIT: u64 = 1024 * 1024;

/// How a window's engine pixels, the compositor's units and the screen's
/// own pixels stand to one another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Units {
    /// Screen pixels to an engine pixel, each way.
    k: i32,
    /// Screen pixels to one of the compositor's units, in 120ths, which is
    /// how the compositor gives a scale that is not a whole number.
    f: i32,
}

impl Units {
    /// At a whole-number scale.
    fn whole(zoom: i32, scale: i32) -> Units {
        Units { k: zoom * scale.max(1), f: 120 * scale.max(1) }
    }

    /// At a scale given in 120ths: engine pixels are whole screen pixels,
    /// as many as the nearest whole scale would make them.
    fn fine(zoom: i32, f: i32) -> Units {
        Units { k: zoom * ((f + 60) / 120).max(1), f: f.max(1) }
    }

    /// Whether the scale is not a whole number, so that the picture has to
    /// be drawn in screen pixels and the compositor told its size.
    fn is_fine(&self) -> bool {
        self.f % 120 != 0
    }

    /// The scale to give a buffer drawn for these.
    fn buffer_scale(&self) -> i32 {
        if self.is_fine() {
            1
        } else {
            self.f / 120
        }
    }

    /// A length in engine pixels, in the compositor's units, rounded up.
    fn out(&self, engine: i32) -> i32 {
        (engine * self.k * 120 + self.f - 1).div_euclid(self.f)
    }

    /// A place in engine pixels, in the compositor's units, rounded down.
    fn at(&self, engine: i32) -> i32 {
        (engine * self.k * 120).div_euclid(self.f)
    }

    /// A place in the compositor's units, in engine pixels.
    fn engine(&self, units: f64) -> i32 {
        (units * self.f as f64 / (120.0 * self.k as f64)).floor() as i32
    }

    /// A length in the compositor's units, in screen pixels, rounded as the
    /// compositor rounds it.
    fn device(&self, units: i32) -> i32 {
        (units * self.f + 60) / 120
    }

    /// A picture drawn in screen pixels, made the size the compositor will
    /// show it at (padded with nothing at the right and bottom), and that
    /// size in the compositor's units.
    fn fitted(&self, picture: Canvas) -> (Canvas, (i32, i32)) {
        let units = ((picture.width * 120 + self.f - 1) / self.f, (picture.height * 120 + self.f - 1) / self.f);
        let (w, h) = (self.device(units.0), self.device(units.1));
        if !self.is_fine() || (w, h) == (picture.width, picture.height) {
            return (picture, units);
        }
        let mut padded = Canvas::new(w, h);
        padded.pixels.fill(press_core::TRANSPARENT);
        padded.blit(&picture, 0, 0);
        (padded, units)
    }
}

/// Tell the compositor the least a window may be made, or the one size it
/// may be, in the compositor's units.
fn limit(window: &Window, handle: Handle, units: Units) {
    // a window of one size says so by having its least and its most the same
    // its own frame, if it has one, is part of what the compositor calls the window
    let (l, t, r, b) = with(|rt| Ok(rt.ui.frame_insets(handle))).unwrap_or_default();
    if let Some((fw, fh)) = with(|rt| Ok(rt.ui.window_fixed_size(handle)?)).ok().flatten() {
        let only = Some((units.out(fw + l + r) as u32, units.out(fh + t + b) as u32));
        window.set_min_size(only);
        window.set_max_size(only);
        return;
    }
    let (mw, mh) = with(|rt| Ok(rt.ui.window_min_size(handle)?)).unwrap_or((64, 32));
    window.set_min_size(Some((units.out(mw + l + r) as u32, units.out(mh + t + b) as u32)));
}

/// Say how a buffer just drawn for these units is to be shown: at a whole
/// scale, or as it is at a size in the compositor's units.
fn present(surface: &wl_surface::WlSurface, viewport: Option<&WpViewport>, units: Units, size: (i32, i32)) {
    surface.set_buffer_scale(units.buffer_scale());
    if let Some(viewport) = viewport {
        if units.is_fine() {
            viewport.set_destination(size.0.max(1), size.1.max(1));
        } else {
            viewport.set_destination(-1, -1);
        }
    }
}

/// A press window that is on screen.
struct Surface {
    handle: Handle,
    window: Window,
    title: String,
    buffer: Option<Buffer>,
    configured: bool,
    needs_draw: bool,
    /// The pulled-down menu and any submenus open from it, in that order.
    menus: Vec<MenuSurface>,
    /// The window as assistive technology sees it.
    bridge: Bridge,
    /// The window's menus as handed to the desktop's own menu bar, with
    /// the object that tells the desktop where to find them.
    published: Option<(appmenu::Published, OrgKdeKwinAppmenu)>,
    /// The whole-number scale of the output the window is on, and the
    /// finer one in 120ths if the compositor gives one.
    scale: i32,
    fine: Option<i32>,
    /// What follows from those.
    units: Units,
    /// For telling the compositor the size a picture is to be shown at,
    /// and for hearing the finer scale.
    viewport: Option<WpViewport>,
    _fraction: Option<WpFractionalScaleV1>,
    /// The size the compositor gave the window, in its own (logical) units.
    size: Option<(i32, i32)>,
    /// How far the window's own frame reaches past its content, in engine
    /// pixels: left, top, right, bottom. Nothing where the frame is the desktop's.
    inset: (i32, i32, i32, i32),
    /// The compositor has made it as large as it can be.
    zoomed: bool,
    /// Whether the window has asked the compositor to fill the screen, or
    /// been told by it that it does.
    full: bool,
}

/// A pulled-down menu on its own surface.
struct MenuSurface {
    popup: Popup,
    /// Where the engine says the menu is, in window coordinates.
    rect: Rect,
    /// Where the compositor actually put it, in its own units; it may slide
    /// a menu that would otherwise leave the screen.
    at: (i32, i32),
    buffer: Option<Buffer>,
    configured: bool,
    viewport: Option<WpViewport>,
    /// A word of help, not a menu: the pointer passes through it.
    tip: bool,
}

/// Input gathered while Wayland events are dispatched, delivered afterwards.
enum Pending {
    Pointer { window: Handle, x: i32, y: i32, kind: PointerKind },
    Close { window: Handle },
    Key { window: Handle, key: Key, mods: Mods },
    Text { window: Handle, text: String },
    /// The window gained or lost the keyboard.
    Active { window: Handle, active: bool },
    /// The right button was pressed: pop up a context menu.
    Context { window: Handle, x: i32, y: i32 },
    RightRelease { window: Handle, x: i32, y: i32 },
    Middle { window: Handle, x: i32, y: i32 },
    /// The back or forward button was pressed over a window.
    Navigate { window: Handle, forward: bool },
    MiddleRelease { window: Handle, x: i32, y: i32 },
    /// The wheel was turned over a window: so many lines down (or up).
    Wheel { window: Handle, x: i32, y: i32, lines: i32 },
    /// The compositor put a window's menu away.
    MenuDismissed { window: Handle },
    /// A drop's data has been read.
    Drop(Dropped),
    /// A drag of rows carried by the desktop has ended: dropped on one of
    /// our lists, or not.
    RowsDropped(bool),
    /// Another program took the rows we were dragging: they have left.
    RowsTaken,
    /// An input method finished a step: text to take back and to enter,
    /// and the draft now being composed.
    Ime { window: Handle, commit: String, before: usize, after: usize, preedit: String },
    /// Assistive technology asked for something to be done to a widget.
    Assist { window: Handle, target: Handle, request: a11y::Request },
    /// Clipboard text arrived for a field that asked to paste.
    Paste { field: Handle, text: String },
    /// The compositor says whether the window fills the screen.
    FullScreen { window: Handle, on: bool },
}

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor: CompositorState,
    xdg_shell: XdgShell,
    shm: Shm,
    pool: SlotPool,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    conn: Connection,
    /// The time of the dispatch in progress, in milliseconds.
    now: u64,
    /// The key being held down, and when it next repeats.
    repeat: Option<(KeyEvent, u64)>,
    /// How long a key is held before it repeats, and the gap between
    /// repeats; `None` when the compositor has turned repeating off.
    repeat_timing: Option<(u64, u64)>,
    /// The window the keyboard is in, and the modifiers currently held.
    key_focus: Option<Handle>,
    mods: Mods,
    /// Whether the logo key is down; it belongs to the desktop, not to text.
    logo: bool,
    /// Serial of the latest input event; the compositor wants one before it
    /// lets a client set the clipboard.
    serial: u32,
    /// The seat the pointer is of, which a window being moved or made
    /// larger by its own frame names to the compositor.
    seat: Option<wl_seat::WlSeat>,
    data_manager: Option<DataDeviceManagerState>,
    data_device: Option<DataDevice>,
    /// Present while this application owns the clipboard, with its text.
    copy_source: Option<CopyPasteSource>,
    copy_text: String,
    /// Where reader threads send text pasted from other applications.
    pasted: mpsc::Sender<(Handle, String)>,
    /// Written to by helper threads to bring on the next dispatch.
    wake: Arc<OwnedFd>,
    /// The desktop's own menu bar, if it has one: how to tell it about a
    /// window's menus, and the bus they are served on.
    appmenu_manager: Option<OrgKdeKwinAppmenuManager>,
    /// For telling the desktop the names windows are to be known by.
    tag_manager: Option<XdgToplevelTagManagerV1>,
    /// For scales that are not whole numbers: how the compositor says what
    /// the scale is, and how it is told the size a picture is to be shown at.
    fractions: Option<WpFractionalScaleManagerV1>,
    viewporter: Option<WpViewporter>,
    cursor_viewport: Option<WpViewport>,
    drag_viewport: Option<WpViewport>,
    menu_bus: Option<appmenu::Bus>,
    /// A drag passing over one of our windows.
    drag_in: Option<DragIn>,
    /// A drop whose data is still being read; it is finished when it arrives.
    dropping: Option<DragOffer>,
    dropped: mpsc::Sender<Dropped>,
    /// Text being dragged out of one of our text fields.
    drag_source: Option<DragSource>,
    drag_text: String,
    /// What was copied that is not text, and what rows being dragged carry
    /// besides themselves: the kind and the thing.
    copy_data: Vec<(String, Vec<u8>)>,
    row_data: Vec<(String, Vec<u8>)>,
    /// Rows being dragged out of one of our lists, and their text for
    /// anything that takes text and not rows.
    row_source: Option<DragSource>,
    row_text: String,
    /// The same rows written out in full, for another program's lists.
    row_payload: String,
    /// The files those rows stand for, as a text/uri-list.
    row_files: String,
    /// The drag over us is to be copied, not moved.
    drag_copy: bool,
    /// The rows are over a list that would have them.
    rows_here: bool,
    /// The rows now over us are another program's.
    rows_foreign: bool,
    /// Our rows were asked for as rows, and whoever asked means to keep
    /// them: when it has finished, they are gone from here.
    row_as_rows: bool,
    row_move: bool,
    /// The rows were dropped on one of our lists, so the end of the drag
    /// is not a cancellation.
    rows_dropped: bool,
    /// The input-method connection, where the compositor offers one.
    ime_manager: Option<ZwpTextInputManagerV3>,
    ime: Option<ZwpTextInputV3>,
    /// The surface the input method is attending to.
    ime_surface: Option<wl_surface::WlSurface>,
    /// What the input method has said since its last `done`.
    ime_step: ImeStep,
    /// The caret rectangle last sent, in the compositor's units; `None`
    /// while the input method is switched off.
    ime_caret: Option<Rect>,
    /// Handed to each window's accessibility bridge for its requests.
    assist: a11y::Requests,
    surfaces: Vec<Surface>,
    pending: Vec<Pending>,
    /// The user's magnification (PRESS_SCALE).
    zoom: i32,
    /// The surface the pointer picture is drawn on, and its buffer.
    cursor_surface: wl_surface::WlSurface,
    /// The picture that goes with rows being dragged, and what it is drawn in.
    drag_surface: wl_surface::WlSurface,
    drag_buffer: Option<Buffer>,
    cursor_buffer: Option<Buffer>,
    /// Serial of the pointer's latest arrival on one of our surfaces.
    enter_serial: u32,
    /// Where the pointer is: window, engine coordinates, and that window's scale.
    pointer_at: Option<(Handle, i32, i32, Units)>,
    /// Wheel movement not yet amounting to a whole line.
    wheel_rest: f64,
    /// The shape and size currently shown, to avoid setting it again.
    cursor_shown: Option<(Cursor, Units)>,
}

/// What an input method sends between two `done` events. It is applied all
/// at once, as the protocol requires.
#[derive(Default)]
struct ImeStep {
    commit: String,
    before: usize,
    after: usize,
    preedit: String,
}

/// The user's magnification: PRESS_SCALE, a whole number from 1 to 8.
fn zoom_from_env() -> i32 {
    std::env::var("PRESS_SCALE").ok().and_then(|v| v.trim().parse::<i32>().ok()).filter(|z| (1..=8).contains(z)).unwrap_or(1)
}

/// Magnify a canvas into a buffer of `width` x `height` device pixels, each
/// canvas pixel becoming a `k` x `k` square. Anything left over is white.
fn magnify(bytes: &mut [u8], width: i32, height: i32, canvas: &Canvas, k: i32, beyond: u32) {
    let (rows, _) = bytes.as_chunks_mut::<4>();
    for (y, row) in rows.chunks_mut(width.max(1) as usize).take(height.max(0) as usize).enumerate() {
        let cy = y as i32 / k;
        for (x, dst) in row.iter_mut().enumerate() {
            let cx = x as i32 / k;
            let pixel = if cx < canvas.width && cy < canvas.height {
                canvas.pixels[(cy * canvas.width + cx) as usize]
            } else {
                beyond
            };
            // the display wants colour already scaled by how solid it is
            let a = pixel >> 24;
            let pixel = if a == 0 || a == 255 {
                pixel
            } else {
                let scale = |shift: u32| (((pixel >> shift) & 0xff) * a / 255) << shift;
                (a << 24) | scale(16) | scale(8) | scale(0)
            };
            *dst = pixel.to_le_bytes();
        }
    }
}

/// A connection to the compositor, with everything shown through it.
pub struct Session {
    queue: EventQueue<State>,
    qh: QueueHandle<State>,
    state: State,
    /// Text pasted from other applications comes back from reader threads here.
    arrivals: mpsc::Receiver<(Handle, String)>,
    /// What drops delivered, sent from reader threads.
    drops: mpsc::Receiver<Dropped>,
    /// Requests from assistive technology, sent from its thread.
    assists: mpsc::Receiver<(Handle, Handle, a11y::Request)>,
}

/// Connect to the compositor. `wake` is an eventfd that helper threads write
/// to when they have something for the next `dispatch`.
pub fn start(wake: Arc<OwnedFd>) -> Result<Session, Status> {
    let fail = |_| Status::Backend;
    let conn = Connection::connect_to_env().map_err(|_| Status::Backend)?;
    let (globals, queue) = registry_queue_init(&conn).map_err(|_| Status::Backend)?;
    let qh = queue.handle();
    let (pasted, arrivals) = mpsc::channel();
    let (assist, assists) = mpsc::channel();
    let (dropped, drops) = mpsc::channel();

    let shm = Shm::bind(&globals, &qh).map_err(|_| Status::Backend)?;
    let compositor = CompositorState::bind(&globals, &qh).map_err(|_| Status::Backend)?;
    // The desktop's menu bar is used when it has one, unless the user says otherwise.
    let in_window = std::env::var("PRESS_MENU_BAR").is_ok_and(|v| v == "window");
    let appmenu_manager: Option<OrgKdeKwinAppmenuManager> = if in_window { None } else { globals.bind(&qh, 1..=2, ()).ok() };
    let (cursor_surface, drag_surface) = (compositor.create_surface(&qh), compositor.create_surface(&qh));
    let viewporter: Option<WpViewporter> = globals.bind(&qh, 1..=1, ()).ok();
    let (cursor_viewport, drag_viewport) = (viewporter.as_ref().map(|v| v.get_viewport(&cursor_surface, &qh, ())), viewporter.as_ref().map(|v| v.get_viewport(&drag_surface, &qh, ())));
    let state = State {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        xdg_shell: XdgShell::bind(&globals, &qh).map_err(|_| Status::Backend)?,
        pool: SlotPool::new(256 * 256 * 4, &shm).map_err(fail)?,
        shm,
        pointer: None,
        keyboard: None,
        key_focus: None,
        mods: Mods::default(),
        logo: false,
        repeat: None,
        repeat_timing: Some((500, 33)),
        now: 0,
        serial: 0,
        seat: None,
        data_manager: DataDeviceManagerState::bind(&globals, &qh).ok(),
        data_device: None,
        copy_source: None,
        copy_text: String::new(),
        pasted,
        wake,
        menu_bus: appmenu_manager.as_ref().and_then(|_| appmenu::Bus::connect()),
        appmenu_manager,
        tag_manager: globals.bind(&qh, 1..=1, ()).ok(),
        // PRESS_FRACTIONAL=0 leaves a scale that is not a whole number to the
        // compositor, as before: the picture is the size asked for, and soft
        fractions: if std::env::var_os("PRESS_FRACTIONAL").is_some_and(|v| v == "0") { None } else { globals.bind(&qh, 1..=1, ()).ok() },
        viewporter,
        cursor_viewport,
        drag_viewport,
        drag_in: None,
        dropping: None,
        dropped,
        drag_source: None,
        drag_text: String::new(),
        copy_data: Vec::new(),
        row_data: Vec::new(),
        row_source: None,
        row_text: String::new(),
        row_payload: String::new(),
        row_files: String::new(),
        drag_copy: false,
        rows_here: false,
        rows_foreign: false,
        row_as_rows: false,
        row_move: false,
        rows_dropped: false,
        ime_manager: globals.bind(&qh, 1..=1, ()).ok(),
        ime: None,
        ime_surface: None,
        ime_step: ImeStep::default(),
        ime_caret: None,
        assist,
        surfaces: Vec::new(),
        pending: Vec::new(),
        zoom: zoom_from_env(),
        cursor_surface,
        drag_surface,
        drag_buffer: None,
        cursor_buffer: None,
        enter_serial: 0,
        pointer_at: None,
        wheel_rest: 0.0,
        cursor_shown: None,
        compositor,
        conn,
    };
    with(|rt| {
        rt.system_clipboard = true;
        Ok(())
    })?;
    Ok(Session { queue, qh, state, arrivals, assists, drops })
}

impl Drop for Session {
    fn drop(&mut self) {
        // Back to the in-process clipboard, which keeps what was last copied,
        // and to windows that carry their own menu bars.
        let _ = with(|rt| {
            for s in self.state.surfaces.iter().filter(|s| s.published.is_some()) {
                let _ = rt.ui.window_set_bar_external(s.handle, false);
            }
            rt.system_clipboard = false;
            if self.state.copy_source.is_some() {
                rt.clipboard = std::mem::take(&mut self.state.copy_text);
            }
            Ok(())
        });
    }
}

impl Session {
    /// The compositor socket: readable when `dispatch` has events to read.
    pub fn fd(&self) -> RawFd {
        self.state.conn.as_fd().as_raw_fd()
    }

    /// When a held key next repeats, if one is held.
    pub fn deadline(&self) -> Option<u64> {
        self.state.repeat.as_ref().map(|(_, at)| *at)
    }

    /// Do everything that is ready to be done, without waiting: read and
    /// handle input, run the application's handlers, redraw. Returns false
    /// once the application has quit or its last window has gone.
    pub fn dispatch(&mut self, now: u64) -> Result<bool, Status> {
        let (state, qh) = (&mut self.state, &self.qh);
        state.now = now;
        // Handle what is already queued, then read whatever has arrived.
        self.queue.dispatch_pending(state).map_err(|_| Status::Backend)?;
        let _ = self.queue.flush();
        if let Some(guard) = self.queue.prepare_read() {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err(Status::Backend),
            }
        }
        self.queue.dispatch_pending(state).map_err(|_| Status::Backend)?;
        for (field, text) in self.arrivals.try_iter() {
            state.pending.push(Pending::Paste { field, text });
        }
        for drop in self.drops.try_iter() {
            // the data is in: tell the source the drop is complete
            // (something pasted came by the same road, with no drag to finish)
            if let Some(offer) = state.dropping.take().filter(|_| drop.at != (-1, -1)) {
                offer.finish();
                offer.destroy();
            }
            state.pending.push(Pending::Drop(drop));
        }
        for (window, target, request) in self.assists.try_iter() {
            state.pending.push(Pending::Assist { window, target, request });
        }
        // A held key repeats. Falling far behind does not cause a burst.
        if let (Some((event, at)), Some((_, interval))) = (state.repeat.clone(), state.repeat_timing) {
            if now >= at {
                state.repeat = Some((event.clone(), (at + interval).max(now + 1)));
                state.key_down(event);
            }
        }

        // Advance the caret blink and the other animations; a button whose
        // keyboard flash has ended reports its click here.
        deliver(with(|rt| Ok(rt.ui.tick(now)))?);
        // Application handlers run here, with nothing borrowed.
        for p in std::mem::take(&mut state.pending) {
            let events = match p {
                Pending::Pointer { window, x, y, kind } => with(|rt| Ok(rt.ui.pointer(window, x, y, kind)?)),
                Pending::Close { window } => with(|rt| Ok(rt.ui.close_requested(window)?)),
                Pending::FullScreen { window, on } => with(|rt| Ok(rt.ui.window_note_full_screen(window, on)?)),
                Pending::Key { window, key, mods } => with(|rt| Ok(rt.ui.key(window, key, mods)?)),
                Pending::Text { window, text } => with(|rt| Ok(rt.ui.text(window, &text)?)),
                Pending::Paste { field, text } => with(|rt| Ok(rt.ui.paste_into(field, &text)?)),
                Pending::RowsDropped(landed) => with(|rt| Ok(rt.ui.row_drag_finish(landed))),
                Pending::RowsTaken => with(|rt| Ok(rt.ui.row_drag_taken())),
                Pending::Drop(drop) => with(|rt| {
                    // something that is neither text nor files: pasted if it was not dragged anywhere
                    if let Some(kind) = &drop.kind {
                        return Ok(match drop.at {
                            (-1, -1) => rt.ui.paste_data_into(drop.window, kind, drop.bytes)?,
                            (x, y) => rt.ui.drop_data(drop.window, kind, drop.bytes, x, y)?,
                        });
                    }
                    let text = String::from_utf8_lossy(&drop.bytes);
                    Ok(if drop.rows {
                        rt.ui.foreign_rows_drop(&text)
                    } else if drop.files {
                        rt.ui.drop_files(drop.window, &paths_from_uri_list(&text), drop.copy)?
                    } else {
                        rt.ui.drop_text(drop.window, drop.at.0, drop.at.1, &text)?
                    })
                }),
                Pending::Ime { window, commit, before, after, preedit } => with(|rt| {
                    let events = rt.ui.ime_commit(window, &commit, before, after)?;
                    rt.ui.ime_preedit(window, &preedit);
                    Ok(events)
                }),
                Pending::Assist { window, target, request } => with(|rt| {
                    // only for a widget that is still in the window it was asked of
                    if !rt.ui.accessible(window)?.nodes.iter().any(|n| n.handle == target) {
                        return Ok(Vec::new());
                    }
                    Ok(match request {
                        a11y::Request::Press => rt.ui.assistive_press(target)?,
                        a11y::Request::SetText(text) => rt.ui.assistive_set_text(target, &text)?,
                        a11y::Request::Select(anchor, focus) => rt.ui.assistive_select(target, anchor, focus)?,
                        a11y::Request::Focus => {
                            rt.ui.focus(target)?;
                            Vec::new()
                        }
                    })
                }),
                Pending::Context { window, x, y } => with(|rt| Ok(rt.ui.context(window, x, y)?)),
                Pending::RightRelease { window, x, y } => with(|rt| Ok(rt.ui.right_release(window, x, y)?)),
                Pending::Middle { window, x, y } => with(|rt| Ok(rt.ui.middle_press(window, x, y)?)),
                Pending::Navigate { window, forward } => with(|rt| Ok(rt.ui.navigate(window, forward)?)),
                Pending::MiddleRelease { window, x, y } => with(|rt| Ok(rt.ui.middle_release(window, x, y)?)),
                Pending::Wheel { window, x, y, lines } => with(|rt| Ok(rt.ui.wheel(window, x, y, lines)?)),
                Pending::MenuDismissed { window } => with(|rt| {
                    rt.ui.dismiss_menu(window);
                    Ok(Vec::new())
                }),
                Pending::Active { window, active } => {
                    if let Some(s) = state.surfaces.iter_mut().find(|s| s.handle == window) {
                        s.bridge.set_active(active);
                    }
                    with(|rt| {
                        rt.ui.window_set_active(window, active)?;
                        Ok(Vec::new())
                    })
                }
            };
            // a window destroyed by an earlier handler simply has no events
            deliver(events.unwrap_or_default());
        }
        // what was chosen from a window's own menu, or done to it by its
        // frame in passing, is done once the engine has been through the events
        while let Ok(Some((window, act))) = with(|rt| Ok(rt.ui.take_frame_ask())) {
            state.frame_act(window, act);
        }

        // pictures that stretch hear of the room they were given
        deliver(with(|rt| Ok(rt.ui.layout_events()))?);
        state.clipboard(qh);
        state.start_drag(qh);
        state.start_row_drag(qh);
        state.update_ime();
        state.update_cursor();
        state.sync(qh)?;
        if state.surfaces.is_empty() || with(|rt| Ok(rt.quit))? {
            let _ = self.queue.flush();
            return Ok(false);
        }
        if with(|rt| Ok(rt.ui.take_dirty()))? {
            state.surfaces.iter_mut().for_each(|s| s.needs_draw = true);
        }
        state.announce();
        state.draw();
        let _ = self.queue.flush();
        Ok(true)
    }
}

/// Fill a shared-memory buffer from a canvas; both are ARGB, the buffer in
/// the byte order Wayland expects.
fn copy_pixels(bytes: &mut [u8], pixels: &[u32]) {
    for (dst, px) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(pixels) {
        *dst = px.to_le_bytes();
    }
}

impl State {
    /// Make the set of on-screen surfaces match the engine's visible windows.
    fn sync(&mut self, qh: &QueueHandle<Self>) -> Result<(), Status> {
        let windows = with(|rt| Ok(rt.ui.windows()))?;
        self.surfaces.retain(|s| windows.iter().any(|w| w.handle == s.handle && w.visible));
        for w in windows.iter().filter(|w| w.visible) {
            match self.surfaces.iter_mut().find(|s| s.handle == w.handle) {
                Some(s) => {
                    if s.title != w.title {
                        s.window.set_title(w.title.clone());
                        s.title = w.title.clone();
                    }
                    // pass on the program's wish to fill the screen, or to stop
                    let want = with(|rt| Ok(rt.ui.window_full_screen(w.handle)?)).unwrap_or(s.full);
                    if want != s.full {
                        if want { s.window.set_fullscreen(None) } else { s.window.unset_fullscreen() }
                        s.full = want;
                    }
                }
                None => {
                    let surface = self.compositor.create_surface(qh);
                    // a window whose style has a frame of its own asks the desktop for none
                    let inset = with(|rt| Ok(rt.ui.frame_insets(w.handle))).unwrap_or_default();
                    let decorations = if inset.1 > 0 { WindowDecorations::RequestClient } else { WindowDecorations::RequestServer };
                    let window = self.xdg_shell.create_window(surface, decorations, qh);
                    window.set_title(w.title.clone());
                    window.set_app_id("org.press.app");
                    // the name it is to be known by goes before anything is shown,
                    // so that the desktop can put the window where it was last time
                    if let Some(tags) = self.tag_manager.as_ref().filter(|_| !w.tag.is_empty()) {
                        tags.set_toplevel_tag(window.xdg_toplevel(), w.tag.clone());
                    }
                    // no smaller than its content, wherever the menu bar ends up
                    let units = Units::whole(self.zoom, 1);
                    let least = |window: &Window| limit(window, w.handle, units);
                    least(&window);
                    let full = with(|rt| Ok(rt.ui.window_full_screen(w.handle)?)).unwrap_or(false);
                    if full {
                        window.set_fullscreen(None);
                    }
                    window.commit(); // the first configure arrives in response
                    // inactive until the compositor hands it the keyboard
                    let _ = with(|rt| Ok(rt.ui.window_set_active(w.handle, false)?));
                    // Hand the window's menus to the desktop's menu bar if it has one.
                    // (unless its style keeps its menus in the window)
                    let keeps_menus = with(|rt| Ok(rt.ui.menu_in_window())).unwrap_or(false);
                    let published = match (self.appmenu_manager.as_ref().filter(|_| !keeps_menus), &mut self.menu_bus) {
                        (Some(manager), Some(bus)) => bus.name().zip(bus.serve(w.handle, self.assist.clone(), self.wake.clone())).map(|(name, published)| {
                            let address = manager.create(window.wl_surface(), qh, ());
                            address.set_address(name, published.path.clone());
                            (published, address)
                        }),
                        _ => None,
                    };
                    if published.is_some() {
                        let _ = with(|rt| Ok(rt.ui.window_set_bar_external(w.handle, true)?));
                        least(&window);
                    }
                    let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(window.wl_surface(), qh, ()));
                    let fraction = self.fractions.as_ref().map(|f| f.get_fractional_scale(window.wl_surface(), qh, window.wl_surface().clone()));
                    self.surfaces.push(Surface {
                        handle: w.handle,
                        window,
                        title: w.title.clone(),
                        buffer: None,
                        configured: false,
                        needs_draw: false,
                        menus: Vec::new(),
                        published,
                        bridge: Bridge::new(w.handle, self.assist.clone(), self.wake.clone()),
                        scale: 1,
                        fine: None,
                        units,
                        viewport,
                        _fraction: fraction,
                        size: None,
                        inset,
                        zoomed: false,
                        full,
                    });
                }
            }
        }
        // Hang, move or take down each window's menus. Those that are
        // already up and in the right place are left alone.
        for s in self.surfaces.iter_mut().filter(|s| s.configured) {
            let want: Vec<(Rect, bool)> = with(|rt| Ok(rt.ui.popups(s.handle)))?.into_iter().map(|p| (p.rect, p.tip)).collect();
            let same = s.menus.iter().zip(&want).take_while(|(have, want)| (have.rect, have.tip) == **want).count();
            while s.menus.len() > same {
                s.menus.pop(); // dropping a popup removes it from the screen; the newest goes first
            }
            let (units, inset) = (s.units, s.inset);
            for (rect, tip) in want.into_iter().skip(same) {
                // the engine measures from the corner of the content, the compositor from the frame's
                let placed = Rect::new(rect.x + inset.0, rect.y + inset.1, rect.w, rect.h);
                let Ok(positioner) = XdgPositioner::new(&self.xdg_shell) else { break };
                positioner.set_size(units.out(rect.w), units.out(rect.h));
                // the anchor has to lie inside the window; a submenu's corner may not
                let (w, h) = s.size.unwrap_or((i32::MAX, i32::MAX));
                let (ax, ay) = (units.at(placed.x).clamp(0, (w - 1).max(0)), units.at(placed.y).clamp(0, (h - 1).max(0)));
                positioner.set_anchor_rect(ax, ay, 1, 1);
                positioner.set_offset(units.at(placed.x) - ax, units.at(placed.y) - ay);
                positioner.set_anchor(Anchor::TopLeft);
                positioner.set_gravity(Gravity::BottomRight);
                // a menu is slid back onto the screen; a word of help goes above the pointer where below will not do
                positioner.set_constraint_adjustment(ConstraintAdjustment::SlideX | if tip { ConstraintAdjustment::FlipY } else { ConstraintAdjustment::SlideY });
                match Popup::new(s.window.xdg_surface(), &positioner, qh, &self.compositor, &self.xdg_shell) {
                    Ok(popup) => {
                        let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(popup.wl_surface(), qh, ()));
                        if tip {
                            // nothing of it takes the pointer: what is under it goes on being pointed at
                            if let Ok(nowhere) = Region::new(&self.compositor) {
                                popup.wl_surface().set_input_region(Some(nowhere.wl_region()));
                            }
                        }
                        s.menus.push(MenuSurface { popup, rect, at: (units.at(placed.x), units.at(placed.y)), buffer: None, configured: false, viewport, tip });
                    }
                    Err(_) => break,
                }
            }
        }
        Ok(())
    }

    /// Bring assistive technology up to date with every window that has
    /// changed or has just been asked for. Costs nothing when none is running.
    fn announce(&mut self) {
        let zoom = self.zoom;
        for s in &mut self.surfaces {
            if let (true, Some((published, _))) = (s.needs_draw, &s.published) {
                published.update(with(|rt| Ok(rt.ui.menu_model(s.handle))).unwrap_or_default());
            }
            if s.bridge.take_fresh() || s.needs_draw {
                let window = s.handle;
                s.bridge.publish(zoom, || with(|rt| Ok(rt.ui.accessible(window)?)).ok());
            }
        }
    }

    fn draw(&mut self) {
        for s in self.surfaces.iter_mut().filter(|s| s.configured && s.needs_draw) {
            let Ok(canvas) = with(|rt| Ok(rt.ui.render_framed(s.handle)?)) else { continue };
            s.inset = with(|rt| Ok(rt.ui.frame_insets(s.handle))).unwrap_or_default();
            let (units, k) = (s.units, s.units.k);
            // The buffer is exactly the size the compositor gave the window,
            // in the screen's own pixels, even when that is not a whole
            // number of engine pixels.
            // (a window rolled up into its title bar is as small as its picture, whatever it was given)
            let rolled = with(|rt| Ok(rt.ui.window_rolled(s.handle))).unwrap_or(false);
            let size = s.size.filter(|_| !rolled).unwrap_or((units.out(canvas.width), units.out(canvas.height)));
            let (w, h) = (units.device(size.0), units.device(size.1));
            let Ok((buffer, bytes)) = self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) else { continue };
            // beyond a frame of its own a window is see-through, as outside its corners
            magnify(bytes, w, h, &canvas, k, if s.inset.1 > 0 { press_core::TRANSPARENT } else { press_core::WHITE });
            let surface = s.window.wl_surface();
            present(surface, s.viewport.as_ref(), units, size);
            surface.damage_buffer(0, 0, w, h);
            if buffer.attach_to(surface).is_ok() {
                s.window.commit();
                s.buffer = Some(buffer); // keep it alive until the next frame replaces it
                s.needs_draw = false;
            }
            for (index, menu) in s.menus.iter_mut().enumerate().filter(|(_, m)| m.configured) {
                let Some(canvas) = with(|rt| Ok(rt.ui.render_popup(s.handle, index))).ok().flatten() else { continue };
                // as large as was asked for the menu, whatever of that the picture does not fill left clear
                let size = (units.out(canvas.width), units.out(canvas.height));
                let (w, h) = (units.device(size.0), units.device(size.1));
                let Ok((buffer, bytes)) = self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) else { continue };
                magnify(bytes, w, h, &canvas, k, press_core::TRANSPARENT);
                let surface = menu.popup.wl_surface();
                present(surface, menu.viewport.as_ref(), units, size);
                surface.damage_buffer(0, 0, w, h);
                if buffer.attach_to(surface).is_ok() {
                    surface.commit();
                    menu.buffer = Some(buffer);
                }
            }
        }
    }

    fn handle_for(&self, surface: &wl_surface::WlSurface) -> Option<Handle> {
        self.surfaces.iter().find(|s| s.window.wl_surface() == surface).map(|s| s.handle)
    }

    /// The window a pointer position belongs to, and the position in that
    /// window's coordinates. A point on a menu's surface, or on the part of
    /// the window a slid menu covers, is moved to where the engine believes
    /// the menu to be.
    ///
    /// `x` and `y` are in the compositor's units; the result is in engine
    /// pixels, followed by how the window's pixels stand to the screen's.
    fn locate(&self, surface: &wl_surface::WlSurface, x: f64, y: f64) -> Option<(Handle, i32, i32, Units)> {
        for s in &self.surfaces {
            let u = s.units;
            if let Some(menu) = s.menus.iter().find(|m| m.popup.wl_surface() == surface) {
                let rect = menu.rect;
                return Some((s.handle, u.engine(x) + rect.x, u.engine(y) + rect.y, u));
            }
            if s.window.wl_surface() == surface {
                // the newest menu is on top
                let (px, py) = (x.floor() as i32, y.floor() as i32);
                if let Some(m) = s.menus.iter().rev().filter(|m| !m.tip).find(|m| Rect::new(m.at.0, m.at.1, u.out(m.rect.w), u.out(m.rect.h)).contains(px, py)) {
                    return Some((s.handle, u.engine(x - m.at.0 as f64) + m.rect.x, u.engine(y - m.at.1 as f64) + m.rect.y, u));
                }
                return Some((s.handle, u.engine(x) - s.inset.0, u.engine(y) - s.inset.1, u));
            }
        }
        None
    }

    /// If the user has begun dragging selected text, offer it to the desktop.
    fn start_drag(&mut self, qh: &QueueHandle<Self>) {
        let Some(text) = with(|rt| Ok(rt.ui.take_drag())).ok().flatten() else { return };
        let origin = self.pointer_at.and_then(|(window, ..)| self.surfaces.iter().find(|s| s.handle == window));
        let (Some(manager), Some(device), Some(origin)) = (&self.data_manager, &self.data_device, origin) else { return };
        let source = manager.create_drag_and_drop_source(qh, TEXT_MIMES, DndAction::Copy);
        // a picture of the text goes with the pointer
        let units = self.pointer_at.map_or(Units::whole(self.zoom, 1), |at| at.3);
        let picture = with(|rt| Ok(rt.ui.text_drag_picture(&text))).ok().flatten().map(|canvas| canvas.scaled(units.k));
        // the serial of the press that began it shows the compositor it is the user's doing
        source.start_drag(device, origin.window.wl_surface(), picture.as_ref().map(|_| &self.drag_surface), self.serial);
        self.drag_source = Some(source);
        self.drag_text = text;
        if let Some(canvas) = picture {
            self.show_drag_picture(canvas, units);
        }
    }

    /// Put a picture, drawn in the screen's pixels, on the surface that
    /// goes about with a drag.
    fn show_drag_picture(&mut self, canvas: Canvas, units: Units) {
        let (canvas, size) = units.fitted(canvas);
        let (w, h) = (canvas.width, canvas.height);
        if let Ok((buffer, bytes)) = self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) {
            copy_pixels(bytes, &canvas.pixels);
            present(&self.drag_surface, self.drag_viewport.as_ref(), units, size);
            self.drag_surface.damage_buffer(0, 0, w, h);
            if buffer.attach_to(&self.drag_surface).is_ok() {
                self.drag_surface.commit();
                self.drag_buffer = Some(buffer);
            }
        }
    }

    /// Tell a drag whether it may be dropped where it now is, and as what.
    fn answer_drag(&mut self) {
        let Some(drag) = &mut self.drag_in else { return };
        let Some(offer) = self.data_device.as_ref().and_then(|d| d.data().drag_offer()) else { return };
        let over_text = with(|rt| Ok(rt.ui.drag_over(drag.window, Some(drag.at)))).unwrap_or(false);
        drag.accepted = if over_text && drag.text_mime.is_some() {
            drag.text_mime.clone()
        } else if drag.kind.is_some() && listening(drag.window, "Window.data_dropped") {
            // a kind the window asked for by name comes before files in general
            drag.kind.clone()
        } else if drag.files && listening(drag.window, "Window.files_dropped") {
            Some(URI_LIST.to_string())
        } else {
            None
        };
        offer.accept_mime_type(offer.serial, drag.accepted.clone());
        // Files may be copied or moved, and moved unless the user says
        // otherwise; the desktop settles which. Text is always a copy.
        if drag.accepted.as_deref() == Some(URI_LIST) {
            offer.set_actions(DndAction::Copy | DndAction::Move, DndAction::Move);
        } else {
            let action = if drag.accepted.is_some() { DndAction::Copy } else { DndAction::None };
            offer.set_actions(action, action);
        }
    }

    /// Switch the input method on while a text field has the keyboard, keep
    /// it told where the caret is, and switch it off otherwise.
    fn update_ime(&mut self) {
        let Some(ime) = &self.ime else { return };
        // only the window the input method is attending to can use it
        let window = self.ime_surface.as_ref().and_then(|s| self.handle_for(s)).filter(|w| Some(*w) == self.key_focus);
        let units = window.and_then(|w| self.surfaces.iter().find(|s| s.handle == w)).map_or(Units::whole(self.zoom, 1), |s| s.units);
        let caret = window
            .and_then(|w| with(|rt| Ok(rt.ui.ime_target(w).map(|r| (r, rt.ui.frame_insets(w))))).ok().flatten())
            .map(|(r, inset)| Rect::new(units.at(r.x + inset.0), units.at(r.y + inset.1), units.out(r.w), units.out(r.h)));
        if caret == self.ime_caret {
            return;
        }
        match caret {
            Some(r) => {
                if self.ime_caret.is_none() {
                    ime.enable();
                    ime.set_content_type(ContentHint::None, ContentPurpose::Normal);
                }
                ime.set_cursor_rectangle(r.x, r.y, r.w, r.h);
            }
            None => ime.disable(),
        }
        ime.commit();
        self.ime_caret = caret;
    }

    /// Show the pointer shape the engine wants where the pointer now is.
    fn update_cursor(&mut self) {
        let (Some(pointer), Some((window, x, y, units))) = (&self.pointer, self.pointer_at) else { return };
        let Ok(cursor) = with(|rt| Ok(rt.ui.cursor_at(window, x, y))) else { return };
        if self.cursor_shown == Some((cursor, units)) {
            return;
        }
        let image = cursor.image();
        let (canvas, size) = units.fitted(image.canvas.scaled(units.k));
        let (w, h) = (canvas.width, canvas.height);
        let Ok((buffer, bytes)) = self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) else { return };
        copy_pixels(bytes, &canvas.pixels);
        present(&self.cursor_surface, self.cursor_viewport.as_ref(), units, size);
        self.cursor_surface.damage_buffer(0, 0, w, h);
        if buffer.attach_to(&self.cursor_surface).is_ok() {
            self.cursor_surface.commit();
            // the hot spot is given in the compositor's units
            pointer.set_cursor(self.enter_serial, Some(&self.cursor_surface), units.at(image.hot.0), units.at(image.hot.1));
            self.cursor_buffer = Some(buffer);
            self.cursor_shown = Some((cursor, units));
        }
    }

    /// Carry out the copies and pastes the engine has queued.
    fn clipboard(&mut self, qh: &QueueHandle<Self>) {
        let (copy, paste) = with(|rt| Ok((rt.ui.take_copy(), rt.ui.take_paste_request()))).unwrap_or_default();
        // things other than text: offered as their own kinds, with the text
        // last copied if they were added to it
        let copies = with(|rt| Ok(rt.ui.take_copy_data())).unwrap_or_default();
        if !copies.is_empty() {
            if self.copy_source.is_none() {
                // the clipboard has passed to another program: nothing of ours is left to add to
                self.copy_text.clear();
                self.copy_data.clear();
            }
            if copies.first().is_some_and(|(also, ..)| !also) {
                self.copy_text.clear();
            }
            press_core::ui::settle_copies(&mut self.copy_data, copies);
            if let (Some(manager), Some(device)) = (&self.data_manager, &self.data_device) {
                let kinds = self.copy_data.iter().map(|(kind, _)| kind.as_str());
                let text = TEXT_MIMES.into_iter().filter(|m| !self.copy_text.is_empty() && !self.copy_data.iter().any(|(kind, _)| kind == m));
                let source = manager.create_copy_paste_source(qh, kinds.chain(text));
                source.set_selection(device, self.serial);
                self.copy_source = Some(source);
            }
        }
        for (window, kind) in with(|rt| Ok(rt.ui.take_paste_data_requests())).unwrap_or_default() {
            self.paste_data(window, kind);
        }
        if let Some(text) = copy {
            self.copy_text = text;
            self.copy_data.clear();
            if let (Some(manager), Some(device)) = (&self.data_manager, &self.data_device) {
                let source = manager.create_copy_paste_source(qh, TEXT_MIMES);
                source.set_selection(device, self.serial);
                self.copy_source = Some(source); // replacing it withdraws the previous offer
            }
        }
        let Some(field) = paste else { return };
        if self.copy_source.is_some() || self.data_device.is_none() {
            // Our own text (or no system clipboard at all): no need to go
            // round by the compositor. Text the program copied as a kind
            // beside something else will do where a field copied none.
            let beside = || self.copy_data.iter().find(|(kind, _)| TEXT_MIMES.contains(&kind.as_str())).map(|(_, data)| String::from_utf8_lossy(data).into_owned());
            let text = if self.copy_text.is_empty() { beside().unwrap_or_default() } else { self.copy_text.clone() };
            deliver(with(|rt| Ok(rt.ui.paste_into(field, &text)?)).unwrap_or_default());
            return;
        }
        let Some(offer) = self.data_device.as_ref().and_then(|d| d.data().selection_offer()) else { return };
        let mime = offer.with_mime_types(|have| TEXT_MIMES.iter().find(|m| have.iter().any(|h| h == *m)).map(|m| m.to_string()));
        let Some(pipe) = mime.and_then(|m| offer.receive(m).ok()) else { return };
        // The other application writes when the compositor passes the request
        // on, which happens after this returns; read on a thread meanwhile.
        let _ = self.conn.flush();
        let (pasted, wake) = (self.pasted.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = std::fs::File::from(OwnedFd::from(pipe)).take(PASTE_LIMIT).read_to_end(&mut bytes);
            let _ = pasted.send((field, String::from_utf8_lossy(&bytes).into_owned()));
            super::poke(wake.as_raw_fd());
        });
    }

    /// A window wants what is on the clipboard as some kind: ours if we
    /// were the one to copy it, and otherwise read from whoever was. The
    /// answer is delivered as a drop that was not dragged anywhere.
    fn paste_data(&mut self, window: Handle, kind: String) {
        let answer = move |bytes: Vec<u8>, kind: String| Dropped { window, at: (-1, -1), files: false, rows: false, copy: false, kind: Some(kind), bytes };
        if self.copy_source.is_some() || self.data_device.is_none() {
            let ours = self.copy_data.iter().find(|(have, _)| *have == kind).map(|(_, data)| data.clone()).unwrap_or_default();
            let _ = self.dropped.send(answer(ours, kind));
            super::poke(self.wake.as_raw_fd());
            return;
        }
        let offer = self.data_device.as_ref().and_then(|d| d.data().selection_offer());
        let pipe = offer.filter(|o| o.with_mime_types(|have| have.contains(&kind))).and_then(|o| o.receive(kind.clone()).ok());
        let _ = self.conn.flush();
        let (dropped, wake) = (self.dropped.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(pipe) = pipe {
                let _ = std::fs::File::from(OwnedFd::from(pipe)).take(DATA_LIMIT).read_to_end(&mut bytes);
            }
            let _ = dropped.send(answer(bytes, kind));
            super::poke(wake.as_raw_fd());
        });
    }

    /// A key went down, or is repeating.
    fn key_down(&mut self, event: KeyEvent) {
        let Some(window) = self.key_focus else { return };
        let mods = self.mods;
        let key = match event.keysym {
            Keysym::Tab | Keysym::ISO_Left_Tab | Keysym::KP_Tab => Some(Key::Tab),
            Keysym::Return | Keysym::KP_Enter => Some(Key::Return),
            Keysym::Escape => Some(Key::Escape),
            Keysym::BackSpace => Some(Key::Backspace),
            Keysym::Delete | Keysym::KP_Delete => Some(Key::Delete),
            Keysym::Left | Keysym::KP_Left => Some(Key::Left),
            Keysym::Right | Keysym::KP_Right => Some(Key::Right),
            Keysym::Up | Keysym::KP_Up => Some(Key::Up),
            Keysym::Down | Keysym::KP_Down => Some(Key::Down),
            Keysym::Home | Keysym::KP_Home => Some(Key::Home),
            Keysym::End | Keysym::KP_End => Some(Key::End),
            Keysym::Menu => Some(Key::Context),
            Keysym::F10 if mods.shift => Some(Key::Context),
            Keysym::F10 => Some(Key::Menu),
            Keysym::Page_Up | Keysym::KP_Page_Up => Some(Key::PageUp),
            Keysym::Page_Down | Keysym::KP_Page_Down => Some(Key::PageDown),
            Keysym::Insert | Keysym::KP_Insert => Some(Key::Insert),
            sym if (Keysym::F1.raw()..=Keysym::F12.raw()).contains(&sym.raw()) => Some(Key::F((sym.raw() - Keysym::F1.raw() + 1) as u8)),
            // with the command key held a letter is a shortcut, not text
            sym if mods.ctrl => sym.key_char().map(|c| Key::Char(c.to_ascii_lowercase())),
            // with Alt, a character for a picture that hears every key
            sym if mods.alt && !self.logo => sym.key_char().filter(|c| !c.is_control()).map(Key::Char),
            _ => None,
        };
        if let Some(key) = key {
            self.pending.push(Pending::Key { window, key, mods });
        } else if !mods.ctrl && !mods.alt && !self.logo {
            if let Some(text) = event.utf8.filter(|t| !t.is_empty() && !t.chars().any(char::is_control)) {
                self.pending.push(Pending::Text { window, text });
            }
        }
    }
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        self.serial = serial;
        self.key_focus = self.handle_for(surface);
        if let Some(window) = self.key_focus {
            self.pending.push(Pending::Active { window, active: true });
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _serial: u32) {
        self.repeat = None;
        if let Some(window) = self.key_focus.take() {
            self.pending.push(Pending::Active { window, active: false });
        }
    }

    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, serial: u32, event: KeyEvent) {
        self.serial = serial;
        // it starts repeating after the delay the compositor asks for
        self.repeat = self.repeat_timing.map(|(delay, _)| (event.clone(), self.now + delay));
        self.key_down(event);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _serial: u32, event: KeyEvent) {
        if self.repeat.as_ref().is_some_and(|(held, _)| held.raw_code == event.raw_code) {
            self.repeat = None;
        }
    }

    fn update_repeat_info(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, info: RepeatInfo) {
        self.repeat_timing = match info {
            RepeatInfo::Repeat { rate, delay } => Some((delay as u64, (1000 / rate.get() as u64).max(1))),
            RepeatInfo::Disable => None,
        };
        if self.repeat_timing.is_none() {
            self.repeat = None;
        }
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _layout: u32,
    ) {
        self.mods = Mods { shift: modifiers.shift, ctrl: modifiers.ctrl, alt: modifiers.alt };
        // clicks need them too: Control adds to a list's selection, Shift stretches it
        let mods = self.mods;
        let _ = with(|rt| {
            rt.ui.set_modifiers(mods);
            Ok(())
        });
        self.logo = modifiers.logo;
    }
}

impl WindowHandler for State {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if let Some(handle) = self.handle_for(window.wl_surface()) {
            self.pending.push(Pending::Close { window: handle });
        }
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window, configure: WindowConfigure, _serial: u32) {
        let Some(s) = self.surfaces.iter_mut().find(|s| &s.window == window) else { return };
        if let (Some(w), Some(h)) = configure.new_size {
            let (w, h) = (w.get() as i32, h.get() as i32);
            s.size = Some((w, h));
            // as many engine pixels as fit whole in the screen pixels given
            let u = s.units;
            let (l, t, r, b) = with(|rt| Ok(rt.ui.frame_insets(s.handle))).unwrap_or_default();
            let _ = with(|rt| Ok(rt.ui.window_resize(s.handle, (u.device(w) / u.k - l - r).max(1), (u.device(h) / u.k - t - b).max(1))?));
        }
        // a desktop that will draw the frame whatever it is asked is let: one frame is enough
        if configure.decoration_mode == DecorationMode::Server && s.inset.1 > 0 {
            let _ = with(|rt| {
                rt.ui.set_own_frames(false);
                Ok(())
            });
            s.inset = (0, 0, 0, 0);
        }
        s.zoomed = configure.is_maximized();
        let _ = with(|rt| {
            rt.ui.window_set_zoomed(s.handle, s.zoomed);
            Ok(())
        });
        // whether it fills the screen is the compositor's to say, whatever was asked
        s.full = configure.is_fullscreen();
        self.pending.push(Pending::FullScreen { window: s.handle, on: s.full });
        s.configured = true;
        s.needs_draw = true;
    }
}

impl PopupHandler for State {
    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup, config: PopupConfigure) {
        for s in &mut self.surfaces {
            if let Some(menu) = s.menus.iter_mut().find(|m| &m.popup == popup) {
                menu.at = config.position;
                menu.configured = true;
                s.needs_draw = true;
            }
        }
    }

    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if let Some(s) = self.surfaces.iter().find(|s| s.menus.iter().any(|m| &m.popup == popup && !m.tip)) {
            self.pending.push(Pending::MenuDismissed { window: s.handle });
        }
    }
}

impl PointerHandler for State {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for event in events {
            let Some((window, x, y, units)) = self.locate(&event.surface, event.position.0, event.position.1) else { continue };
            match event.kind {
                PointerEventKind::Press { serial, .. } => self.serial = serial,
                PointerEventKind::Enter { serial } => {
                    // a pointer arriving on a surface has no shape until given one
                    self.enter_serial = serial;
                    self.cursor_shown = None;
                }
                _ => {}
            }
            self.pointer_at = match event.kind {
                PointerEventKind::Leave { .. } => None,
                _ => Some((window, x, y, units)),
            };
            if let PointerEventKind::Axis { vertical, .. } = &event.kind {
                // one notch of an ordinary wheel is three lines
                self.wheel_rest += vertical.absolute / 5.0;
                let lines = self.wheel_rest.trunc();
                self.wheel_rest -= lines;
                if lines != 0.0 {
                    self.pending.push(Pending::Wheel { window, x, y, lines: lines as i32 });
                }
                continue;
            }
            if let PointerEventKind::Press { button: BTN_RIGHT, .. } = event.kind {
                self.pending.push(Pending::Context { window, x, y });
                continue;
            }
            if let PointerEventKind::Release { button: BTN_RIGHT, .. } = event.kind {
                self.pending.push(Pending::RightRelease { window, x, y });
                continue;
            }
            match event.kind {
                PointerEventKind::Press { button: BTN_SIDE | BTN_BACK, .. } => {
                    self.pending.push(Pending::Navigate { window, forward: false });
                    continue;
                }
                PointerEventKind::Press { button: BTN_EXTRA | BTN_FORWARD, .. } => {
                    self.pending.push(Pending::Navigate { window, forward: true });
                    continue;
                }
                PointerEventKind::Press { button: BTN_MIDDLE, .. } => {
                    self.pending.push(Pending::Middle { window, x, y });
                    continue;
                }
                PointerEventKind::Release { button: BTN_MIDDLE, .. } => {
                    self.pending.push(Pending::MiddleRelease { window, x, y });
                    continue;
                }
                _ => {}
            }
            let kind = match event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => PointerKind::Move,
                PointerEventKind::Leave { .. } => PointerKind::Leave,
                PointerEventKind::Press { button: BTN_LEFT, .. } => PointerKind::Press,
                PointerEventKind::Release { button: BTN_LEFT, .. } => PointerKind::Release,
                _ => continue,
            };
            // a window with a frame of its own: the frame has first refusal
            let framed = self.surfaces.iter().find(|s| s.handle == window && s.window.wl_surface() == &event.surface).map(|s| s.inset).filter(|inset| inset.1 > 0);
            if let Some(inset) = framed {
                match with(|rt| Ok(rt.ui.frame_pointer(window, x + inset.0, y + inset.1, kind))) {
                    Ok(press_core::FrameAct::Inside(..)) | Err(_) => {}
                    Ok(act) => {
                        self.frame_act(window, act);
                        continue;
                    }
                }
            }
            self.pending.push(Pending::Pointer { window, x, y, kind });
        }
    }
}

impl State {
    /// Do what the user asked of a window by its own frame.
    fn frame_act(&mut self, window: Handle, act: press_core::FrameAct) {
        use press_core::FrameAct;
        use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;
        let Some(s) = self.surfaces.iter_mut().find(|s| s.handle == window) else { return };
        s.needs_draw = true;
        let toplevel = s.window.xdg_toplevel();
        match (act, &self.seat) {
            // the serial of the press shows the compositor it is the user's doing
            (FrameAct::Move, Some(seat)) => toplevel._move(seat, self.serial),
            (FrameAct::Resize { left, right, top, bottom }, Some(seat)) => {
                let edge = match (left, right, top, bottom) {
                    (true, _, true, _) => ResizeEdge::TopLeft,
                    (_, true, true, _) => ResizeEdge::TopRight,
                    (true, _, _, true) => ResizeEdge::BottomLeft,
                    (_, true, _, true) => ResizeEdge::BottomRight,
                    (true, ..) => ResizeEdge::Left,
                    (_, true, ..) => ResizeEdge::Right,
                    (_, _, true, _) => ResizeEdge::Top,
                    _ => ResizeEdge::Bottom,
                };
                toplevel.resize(seat, self.serial, edge);
            }
            (FrameAct::Close, _) => self.pending.push(Pending::Close { window }),
            (FrameAct::Minimize, _) => s.window.set_minimized(),
            (FrameAct::Zoom, _) => {
                if s.zoomed { s.window.unset_maximized() } else { s.window.set_maximized() }
            }
            _ => {}
        }
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        self.seat.get_or_insert_with(|| seat.clone());
        if self.data_device.is_none() {
            self.data_device = self.data_manager.as_ref().map(|m| m.get_data_device(qh, &seat));
        }
        if self.ime.is_none() {
            self.ime = self.ime_manager.as_ref().map(|m| m.get_text_input(&seat, qh, ()));
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seat_state.get_keyboard(qh, &seat, None).ok();
        }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
            self.key_focus = None;
            self.repeat = None;
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl Dispatch<XdgToplevelTagManagerV1, ()> for State {
    fn event(_: &mut Self, _: &XdgToplevelTagManagerV1, _: <XdgToplevelTagManagerV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<OrgKdeKwinAppmenuManager, ()> for State {
    fn event(_: &mut Self, _: &OrgKdeKwinAppmenuManager, _: <OrgKdeKwinAppmenuManager as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<OrgKdeKwinAppmenu, ()> for State {
    fn event(_: &mut Self, _: &OrgKdeKwinAppmenu, _: <OrgKdeKwinAppmenu as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwpTextInputManagerV3, ()> for State {
    fn event(_: &mut Self, _: &ZwpTextInputManagerV3, _: <ZwpTextInputManagerV3 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwpTextInputV3, ()> for State {
    fn event(state: &mut Self, _: &ZwpTextInputV3, event: zwp_text_input_v3::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        use zwp_text_input_v3::Event;
        match event {
            Event::Enter { surface } => {
                state.ime_surface = Some(surface);
                state.ime_caret = None; // it must be switched on afresh each time it arrives
            }
            Event::Leave { .. } => {
                state.ime_surface = None;
                state.ime_caret = None;
                state.ime_step = ImeStep::default();
                if let Some(window) = state.key_focus {
                    // whatever was being composed is abandoned
                    state.pending.push(Pending::Ime { window, commit: String::new(), before: 0, after: 0, preedit: String::new() });
                }
            }
            Event::PreeditString { text, .. } => state.ime_step.preedit = text.unwrap_or_default(),
            Event::CommitString { text } => state.ime_step.commit = text.unwrap_or_default(),
            Event::DeleteSurroundingText { before_length, after_length } => {
                state.ime_step.before = before_length as usize;
                state.ime_step.after = after_length as usize;
            }
            Event::Done { .. } => {
                let step = std::mem::take(&mut state.ime_step);
                if let Some(window) = state.key_focus {
                    state.pending.push(Pending::Ime {
                        window,
                        commit: step.commit,
                        before: step.before,
                        after: step.after,
                        preedit: step.preedit,
                    });
                }
            }
            _ => {}
        }
    }
}

impl State {
    /// The desktop has finished with a drag of rows. `completed` says the
    /// other side saw it through. Dropped on one of our own lists, the
    /// engine has already moved them. Taken as rows by another program's
    /// list, which keeps them, they leave ours. Otherwise the lists stay as
    /// they were: rows given to something as text are copies.
    fn row_drag_ended(&mut self, source: &WlDataSource, completed: bool) {
        if self.row_source.as_ref().is_some_and(|s| s.inner() == source) {
            self.row_source = None;
            self.rows_here = false;
            let taken = completed && std::mem::take(&mut self.row_as_rows) && std::mem::take(&mut self.row_move);
            if std::mem::take(&mut self.rows_dropped) {
                return;
            }
            self.pending.push(if taken { Pending::RowsTaken } else { Pending::RowsDropped(false) });
        }
    }
}

/// Write what another program asked for down the pipe it gave, without
/// holding anything up while it reads.
fn write_out(pipe: WritePipe, bytes: Vec<u8>) {
    std::thread::spawn(move || {
        let _ = std::fs::File::from(OwnedFd::from(pipe)).write_all(&bytes);
    });
}

impl DataSourceHandler for State {
    /// Another application wants the text we copied.
    fn send_request(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource, mime: String, pipe: WritePipe) {
        let carried = |held: &[(String, Vec<u8>)]| held.iter().find(|(kind, _)| *kind == mime).map(|(_, data)| data.clone());
        let text = if self.copy_source.as_ref().is_some_and(|s| s.inner() == source) {
            // what was copied may be something other than text
            if let Some(data) = carried(&self.copy_data) {
                return write_out(pipe, data);
            }
            self.copy_text.clone()
        } else if self.drag_source.as_ref().is_some_and(|s| s.inner() == source) {
            self.drag_text.clone()
        } else if self.row_source.as_ref().is_some_and(|s| s.inner() == source) {
            if let Some(data) = carried(&self.row_data) {
                return write_out(pipe, data); // what the rows carry besides themselves
            }
            if mime == ROWS_MIME {
                // another program's list wants the rows themselves
                self.row_as_rows = true;
                self.row_payload.clone()
            } else if mime == URI_LIST {
                self.row_files.clone() // the files the rows stand for
            } else {
                self.row_text.clone() // somewhere that takes text is being given the rows' labels
            }
        } else {
            return;
        };
        write_out(pipe, text.into_bytes());
    }

    /// Someone else copied: the clipboard is no longer ours.
    fn cancelled(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource) {
        if self.copy_source.as_ref().is_some_and(|s| s.inner() == source) {
            self.copy_source = None;
        }
        // a drag dropped where nothing would take it is cancelled too
        if self.drag_source.as_ref().is_some_and(|s| s.inner() == source) {
            self.drag_source = None;
        }
        self.row_drag_ended(source, false);
    }

    /// Whoever took our drag has finished reading it.
    fn dnd_finished(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource) {
        if self.drag_source.as_ref().is_some_and(|s| s.inner() == source) {
            self.drag_source = None;
        }
        self.row_drag_ended(source, true);
    }

    fn accept_mime(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource, _: Option<String>) {}
    fn dnd_dropped(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource) {}
    /// What whoever the drag is over means to do with it.
    fn action(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource, action: DndAction) {
        if self.row_source.as_ref().is_some_and(|s| s.inner() == source) {
            self.row_move = action.contains(DndAction::Move);
        }
    }
}

impl State {
    /// Whether the drag now over us is rows, and if so whether they could
    /// be dropped at this point. Our own rows are settled inside the
    /// program and nothing is read from the offer; another program's are
    /// read from it when they are dropped.
    fn rows_over(&mut self, offer: &DragOffer, window: Handle, x: i32, y: i32) -> bool {
        let rows = offer.with_mime_types(|have| have.iter().any(|h| h == ROWS_MIME));
        let ours = rows && self.row_source.is_some();
        self.rows_foreign = rows && !ours;
        self.rows_here = rows
            && with(|rt| Ok(if ours { rt.ui.row_drag_over(window, Some((x, y))) } else { rt.ui.foreign_rows_over(window, Some((x, y))) })).unwrap_or(false);
        if self.rows_here {
            offer.accept_mime_type(offer.serial, Some(ROWS_MIME.to_string()));
            offer.set_actions(DndAction::Move, DndAction::Move);
        } else if ours {
            // not over a list that would have them; they may still be text to a field
            let _ = with(|rt| Ok(rt.ui.row_drag_over(window, None)));
        }
        self.rows_here
    }

    /// Note what a drag over one of our windows carries, and answer it.
    fn drag_at(&mut self, offer: &DragOffer, window: Handle, x: i32, y: i32) {
        if self.rows_over(offer, window, x, y) {
            if let Some(drag) = self.drag_in.take() {
                let _ = with(|rt| Ok(rt.ui.drag_over(drag.window, None)));
            }
            return;
        }
        match self.drag_in.as_mut().filter(|d| d.window == window) {
            Some(drag) => drag.at = (x, y),
            None => {
                let text_mime = offer.with_mime_types(|have| TEXT_MIMES.iter().find(|m| have.iter().any(|h| h == *m)).map(|m| m.to_string()));
                let files = offer.with_mime_types(|have| have.iter().any(|h| h == URI_LIST));
                let wanted = with(|rt| Ok(rt.ui.window_accepts(window))).unwrap_or_default();
                let kind = offer.with_mime_types(|have| wanted.iter().find(|k| have.contains(k)).cloned());
                self.drag_in = Some(DragIn { window, at: (x, y), text_mime, files, kind, accepted: None });
            }
        }
        self.answer_drag();
    }

    /// If rows have come loose in a list, hand the drag to the desktop, so
    /// that it can go on over other windows. It offers itself as rows, for
    /// this program's lists, and as text, for anything else.
    fn start_row_drag(&mut self, qh: &QueueHandle<Self>) {
        if self.row_source.is_some() || !with(|rt| Ok(rt.ui.row_dragging())).unwrap_or(false) {
            return;
        }
        let origin = self.pointer_at.and_then(|(window, ..)| self.surfaces.iter().find(|s| s.handle == window));
        let (Some(manager), Some(device), Some(origin)) = (&self.data_manager, &self.data_device, origin) else { return };
        // rows that stand for files are offered as those files too
        let files = with(|rt| Ok(rt.ui.row_drag_files())).unwrap_or_default();
        self.row_files = files.iter().map(|path| format!("file://{}\r\n", uri_escaped(path))).collect();
        // and as whatever else the program has given them to carry
        self.row_data = with(|rt| Ok(rt.ui.row_drag_data())).unwrap_or_default();
        let carried: Vec<String> = self.row_data.iter().map(|(kind, _)| kind.clone()).collect();
        let mimes = std::iter::once(ROWS_MIME).chain(carried.iter().map(String::as_str)).chain((!files.is_empty()).then_some(URI_LIST)).chain(TEXT_MIMES.into_iter().filter(|m| !carried.iter().any(|c| c == m)));
        let source = manager.create_drag_and_drop_source(qh, mimes, DndAction::Copy | DndAction::Move);
        // a picture of the rows goes with the pointer wherever the desktop takes them
        let units = self.pointer_at.map_or(Units::whole(self.zoom, 1), |at| at.3);
        let picture = with(|rt| Ok(rt.ui.row_drag_picture())).ok().flatten().map(|canvas| canvas.scaled(units.k));
        source.start_drag(device, origin.window.wl_surface(), picture.as_ref().map(|_| &self.drag_surface), self.serial);
        if let Some(canvas) = picture {
            self.show_drag_picture(canvas, units);
        }
        // what the rows are, written out now, while the drag is as it began
        (self.row_text, self.row_payload) = with(|rt| Ok((rt.ui.row_drag_text(), rt.ui.row_drag_payload()))).unwrap_or_default();
        self.row_source = Some(source);
        (self.rows_dropped, self.row_as_rows, self.row_move) = (false, false, false);
    }
}

impl DataDeviceHandler for State {
    /// A drag has come over one of our surfaces.
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice, x: f64, y: f64, surface: &wl_surface::WlSurface) {
        let Some(offer) = self.data_device.as_ref().and_then(|d| d.data().drag_offer()) else { return };
        let Some((window, x, y, _)) = self.locate(surface, x, y) else { return };
        self.drag_in = None;
        self.drag_at(&offer, window, x, y);
    }

    fn motion(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice, x: f64, y: f64) {
        let Some(offer) = self.data_device.as_ref().and_then(|d| d.data().drag_offer()) else { return };
        let Some((window, x, y, _)) = self.locate(&offer.surface, x, y) else { return };
        self.drag_at(&offer, window, x, y);
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice) {
        if let Some(drag) = self.drag_in.take() {
            let _ = with(|rt| Ok(rt.ui.drag_over(drag.window, None)));
        }
        if let Some(any) = self.surfaces.first().map(|s| s.handle) {
            // rows, ours or another program's, are over none of our windows now
            // (unless they have just been dropped here: the desktop says "left"
            // straight after "dropped", and the drop is still to be carried out)
            let (ours, foreign) = (self.row_source.is_some() && !self.rows_dropped, self.rows_foreign);
            let _ = with(|rt| {
                if ours {
                    rt.ui.row_drag_over(any, None);
                }
                if foreign {
                    rt.ui.foreign_rows_over(any, None);
                }
                Ok(())
            });
            (self.rows_here, self.rows_foreign) = (false, false);
        }
    }

    /// It was let go over us. Rows are put where the mark showed. Anything
    /// else is read on a helper thread; the result is acted on, and the drop
    /// finished, when it arrives.
    fn drop_performed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice) {
        let Some(offer) = self.data_device.as_ref().and_then(|d| d.data().drag_offer()) else { return };
        if self.rows_here && self.rows_foreign {
            // another program's rows: read what it wrote, and take them in when it arrives
            (self.rows_here, self.rows_foreign) = (false, false);
            let window = self.surfaces.first().map(|s| s.handle);
            let (Some(window), Ok(pipe)) = (window, offer.receive(ROWS_MIME.to_string())) else {
                offer.destroy();
                return;
            };
            let _ = self.conn.flush();
            let (dropped, wake) = (self.dropped.clone(), self.wake.clone());
            self.dropping = Some(offer);
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = std::fs::File::from(OwnedFd::from(pipe)).take(PASTE_LIMIT).read_to_end(&mut bytes);
                let _ = dropped.send(Dropped { window, at: (0, 0), files: false, rows: true, copy: false, kind: None, bytes });
                super::poke(wake.as_raw_fd());
            });
            return;
        }
        if self.rows_here {
            self.rows_here = false;
            self.rows_dropped = true;
            self.pending.push(Pending::RowsDropped(true));
            offer.finish();
            offer.destroy();
            return;
        }
        let Some(drag) = self.drag_in.take() else { return };
        let _ = with(|rt| Ok(rt.ui.drag_over(drag.window, None)));
        let Some(pipe) = drag.accepted.clone().and_then(|mime| offer.receive(mime).ok()) else {
            offer.destroy();
            return;
        };
        let _ = self.conn.flush();
        let (dropped, wake) = (self.dropped.clone(), self.wake.clone());
        let files = drag.accepted.as_deref() == Some(URI_LIST);
        let kind = drag.kind.clone().filter(|k| drag.accepted.as_ref() == Some(k));
        let copy = self.drag_copy;
        self.dropping = Some(offer);
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let most = if kind.is_some() { DATA_LIMIT } else { PASTE_LIMIT };
            let _ = std::fs::File::from(OwnedFd::from(pipe)).take(most).read_to_end(&mut bytes);
            let _ = dropped.send(Dropped { window: drag.window, at: drag.at, files, rows: false, copy, kind, bytes });
            super::poke(wake.as_raw_fd());
        });
    }

    fn selection(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice) {}
}

impl DataOfferHandler for State {
    fn source_actions(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &mut DragOffer, _: DndAction) {}
    /// What the desktop has settled is to be done with the drag over us.
    fn selected_action(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &mut DragOffer, action: DndAction) {
        self.drag_copy = action.contains(DndAction::Copy) && !action.contains(DndAction::Move);
    }
}

impl State {
    /// A window's scale has changed, in whole numbers or in the finer
    /// measure: work out afresh how its pixels stand to the screen's, and
    /// make everything that hangs on that suit.
    fn rescale(&mut self, surface: &wl_surface::WlSurface) {
        let zoom = self.zoom;
        let Some(s) = self.surfaces.iter_mut().find(|s| s.window.wl_surface() == surface) else { return };
        let units = match s.fine {
            Some(f) => Units::fine(zoom, f),
            None => Units::whole(zoom, s.scale),
        };
        if units == s.units {
            return;
        }
        s.units = units;
        s.needs_draw = true;
        s.menus.clear(); // they were sized for the old scale, and are hung again
        limit(&s.window, s.handle, units);
        if let Some((w, h)) = s.size {
            let (l, t, r, b) = with(|rt| Ok(rt.ui.frame_insets(s.handle))).unwrap_or_default();
            let _ = with(|rt| Ok(rt.ui.window_resize(s.handle, (units.device(w) / units.k - l - r).max(1), (units.device(h) / units.k - t - b).max(1))?));
        }
        self.cursor_shown = None;
    }
}

// The compositor says what scale suits a window, in 120ths: 180 is one and a half.
impl Dispatch<WpFractionalScaleV1, wl_surface::WlSurface> for State {
    fn event(state: &mut Self, _: &WpFractionalScaleV1, event: wp_fractional_scale_v1::Event, surface: &wl_surface::WlSurface, _: &Connection, _: &QueueHandle<Self>) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            if let Some(s) = state.surfaces.iter_mut().find(|s| s.window.wl_surface() == surface) {
                s.fine = Some(scale as i32);
            }
            state.rescale(surface);
        }
    }
}

wayland_client::delegate_noop!(State: WpFractionalScaleManagerV1);
wayland_client::delegate_noop!(State: WpViewporter);
wayland_client::delegate_noop!(State: WpViewport);

impl CompositorHandler for State {
    /// The window moved to an output with a different scale.
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, factor: i32) {
        if let Some(s) = self.surfaces.iter_mut().find(|s| s.window.wl_surface() == surface) {
            if s.scale != factor.max(1) {
                s.scale = factor.max(1);
                self.rescale(surface);
            }
        }
    }
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(State);
delegate_output!(State);
delegate_shm!(State);
delegate_seat!(State);
delegate_pointer!(State);
delegate_keyboard!(State);
delegate_data_device!(State);
delegate_xdg_shell!(State);
delegate_xdg_window!(State);
delegate_xdg_popup!(State);
delegate_registry!(State);

#[cfg(test)]
mod tests {
    use super::paths_from_uri_list;

    #[test]
    fn dragged_files_arrive_as_plain_paths() {
        let list = "# a comment\r\nfile:///home/u/a%20b.txt\r\nfile://host/tmp/caf%C3%A9\r\nhttps://example.org/x\r\nfile:///bad%zz\r\n";
        assert_eq!(paths_from_uri_list(list), "/home/u/a b.txt\n/tmp/café");
        assert_eq!(paths_from_uri_list(""), "");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn engine_pixels_are_whole_screen_pixels_at_any_scale() {
        // whole scales are as they always were: a unit is `zoom` engine pixels
        let two = Units::whole(1, 2);
        assert_eq!((two.k, two.buffer_scale(), two.is_fine()), (2, 2, false));
        assert_eq!((two.out(100), two.at(7), two.engine(7.9), two.device(100)), (100, 7, 7, 200));
        assert_eq!(Units::fine(1, 240), two);
        // one and a half: an engine pixel is two screen pixels, the nearest whole number
        let half = Units::fine(1, 180);
        assert_eq!((half.k, half.buffer_scale(), half.is_fine()), (2, 1, true));
        // 300 engine pixels are 600 screen pixels, which is 400 of the compositor's units
        assert_eq!((half.out(300), half.device(400)), (400, 600));
        // a place in units comes back to the engine pixel it is in
        assert_eq!((half.engine(0.0), half.engine(1.4), half.engine(399.9)), (0, 1, 299));
        // one and a quarter rounds down to one, one and three quarters up to two
        assert_eq!((Units::fine(1, 150).k, Units::fine(1, 210).k, Units::fine(2, 150).k), (1, 2, 2));
        // what is asked of the compositor is never less than the picture needs
        for f in [150, 180, 210, 270] {
            let u = Units::fine(1, f);
            for engine in 1..400 {
                assert!(u.device(u.out(engine)) >= engine * u.k, "{f} {engine}");
            }
        }
        // a picture is padded to the size the compositor will show it at
        let (padded, size) = half.fitted(Canvas::new(21, 9));
        assert_eq!((size, padded.width, padded.height), ((14, 6), 21, 9));
        let (padded, size) = half.fitted(Canvas::new(22, 10));
        assert_eq!((size, padded.width, padded.height), ((15, 7), 23, 11));
    }
}
