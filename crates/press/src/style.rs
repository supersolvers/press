// SPDX-License-Identifier: GPL-3.0-or-later
//! Styles from outside: a look for the interface that lives in a shared
//! library of its own, written against `include/press_style.h`.
//!
//! Which style is chosen once, when the program starts: `PRESS_STYLE` says,
//! or failing that the first line of `press/style` in the user's
//! configuration folder, the desktop's choice for every program. Either is
//! a name (`ninetyfive` is looked for as `libpress_style_ninetyfive.so`) or
//! the path of a library. With neither, or if the library cannot be had,
//! the interface is black and white.

use press_core::{Metrics, Painter, Palette, Part, Style};
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The version of the interface below. A library that answers with another
/// is not used.
pub const ABI: u32 = 1;

/// A part to draw, as a style library is handed it. See
/// [`press_core::PartKind`] for what the numbers mean for each kind.
#[repr(C)]
pub struct RawPart {
    pub kind: u32,
    pub state: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub phase: i32,
    pub a: i32,
    pub b: i32,
    pub c: i32,
    /// The words on it, UTF-8 with a NUL after; never null.
    pub text: *const c_char,
}

/// What a style library draws on: the pixels, ARGB and a row after a row,
/// and the engine's own hand for words.
#[repr(C)]
pub struct RawSurface {
    pub pixels: *mut u32,
    pub width: i32,
    pub height: i32,
    pub ctx: *mut c_void,
    pub line_height: unsafe extern "C" fn(ctx: *mut c_void) -> i32,
    pub text_width: unsafe extern "C" fn(ctx: *mut c_void, text: *const c_char) -> i32,
    /// Draw words with the top of their line at `y`.
    pub draw_text: unsafe extern "C" fn(ctx: *mut c_void, text: *const c_char, x: i32, y: i32, color: u32),
}

/// What a style library gives of itself, from `press_style_entry`.
#[repr(C)]
pub struct RawStyle {
    pub abi: u32,
    pub name: *const c_char,
    /// Handed the black-and-white sizes, to change what it would have otherwise.
    pub metrics: Option<unsafe extern "C" fn(*mut Metrics)>,
    pub palette: Option<unsafe extern "C" fn(*mut Palette)>,
    /// Draw a part and say 1, or say 0 and have it drawn in black and white.
    pub draw: Option<unsafe extern "C" fn(*const RawPart, *const RawSurface) -> i32>,
}

/// What the callbacks of a surface are handed back.
struct Hand {
    canvas: *mut press_core::Canvas,
    font: *mut press_core::Font,
}

unsafe fn words<'a>(text: *const c_char) -> std::borrow::Cow<'a, str> {
    if text.is_null() {
        return "".into();
    }
    CStr::from_ptr(text).to_string_lossy()
}

unsafe extern "C" fn line_height(ctx: *mut c_void) -> i32 {
    (*(*(ctx as *mut Hand)).font).line_height()
}

unsafe extern "C" fn text_width(ctx: *mut c_void, text: *const c_char) -> i32 {
    (*(*(ctx as *mut Hand)).font).width(&words(text))
}

unsafe extern "C" fn draw_text(ctx: *mut c_void, text: *const c_char, x: i32, y: i32, color: u32) {
    let hand = &*(ctx as *mut Hand);
    (*hand.font).draw(&mut *hand.canvas, &words(text), x, y, color);
}

/// A style in a library that has been loaded. The library stays loaded for
/// as long as the program runs.
struct Foreign {
    name: String,
    raw: &'static RawStyle,
}

// The library's functions are only ever called from the UI thread; the
// engine asks for these marks because a style is shared by reference.
unsafe impl Send for Foreign {}
unsafe impl Sync for Foreign {}

impl Style for Foreign {
    fn name(&self) -> &str {
        &self.name
    }

    fn metrics(&self) -> Metrics {
        let mut m = press_core::OneBit::METRICS;
        if let Some(f) = self.raw.metrics {
            unsafe { f(&mut m) };
        }
        // whatever it says, nothing is smaller than a pixel or larger than a window
        let size = |v: i32| v.clamp(1, 512);
        let room = |v: i32| v.clamp(0, 512);
        Metrics {
            button_height: size(m.button_height),
            button_min_width: size(m.button_min_width),
            button_pad: room(m.button_pad),
            default_ring: room(m.default_ring),
            check_size: size(m.check_size),
            check_gap: room(m.check_gap),
            radio_size: size(m.radio_size),
            radio_gap: room(m.radio_gap),
            field_inset_x: size(m.field_inset_x),
            field_inset_y: size(m.field_inset_y),
            progress_height: size(m.progress_height),
            slider_thick: size(m.slider_thick),
        }
    }

    fn palette(&self) -> Palette {
        let mut p = press_core::OneBit::PALETTE;
        if let Some(f) = self.raw.palette {
            unsafe { f(&mut p) };
        }
        p
    }

    fn draw(&self, part: &Part, painter: &mut Painter) -> bool {
        let Some(draw) = self.raw.draw else { return false };
        let text = CString::new(part.text.replace('\0', " ")).unwrap_or_default();
        let raw = RawPart {
            kind: part.kind as u32,
            state: part.state,
            x: part.rect.x,
            y: part.rect.y,
            width: part.rect.w,
            height: part.rect.h,
            phase: part.phase,
            a: part.a,
            b: part.b,
            c: part.c,
            text: text.as_ptr(),
        };
        let canvas: *mut press_core::Canvas = &mut *painter.canvas;
        let mut hand = Hand { canvas, font: &mut *painter.font };
        unsafe {
            let surface = RawSurface {
                pixels: (*canvas).pixels.as_mut_ptr(),
                width: (*canvas).width,
                height: (*canvas).height,
                ctx: &mut hand as *mut Hand as *mut c_void,
                line_height,
                text_width,
                draw_text,
            };
            draw(&raw, &surface) != 0
        }
    }
}

/// Load a style from a library.
fn load(path: &Path) -> Result<Arc<dyn Style>, String> {
    let c_path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| "its name has a NUL in it".to_string())?;
    unsafe {
        let lib = libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        if lib.is_null() {
            let why = libc::dlerror();
            return Err(if why.is_null() { "it could not be loaded".into() } else { CStr::from_ptr(why).to_string_lossy().into_owned() });
        }
        let entry = libc::dlsym(lib, c"press_style_entry".as_ptr());
        if entry.is_null() {
            return Err("it has no press_style_entry".into());
        }
        let entry: unsafe extern "C" fn() -> *const RawStyle = std::mem::transmute(entry);
        let raw = entry();
        if raw.is_null() {
            return Err("it gave no style".into());
        }
        let raw: &'static RawStyle = &*raw;
        if raw.abi != ABI {
            return Err(format!("it was made for version {} of the style interface, and this is version {ABI}", raw.abi));
        }
        let name = if raw.name.is_null() { path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default() } else { words(raw.name).into_owned() };
        Ok(Arc::new(Foreign { name, raw }))
    }
}

/// The folder this library was loaded from, where styles built beside it are.
fn beside_me() -> Option<PathBuf> {
    unsafe {
        let mut info: libc::Dl_info = std::mem::zeroed();
        if libc::dladdr(beside_me as *const c_void, &mut info) == 0 || info.dli_fname.is_null() {
            return None;
        }
        let path = PathBuf::from(CStr::from_ptr(info.dli_fname).to_string_lossy().into_owned());
        path.parent().map(Path::to_path_buf)
    }
}

/// Where a style is looked for by name, the likeliest place first.
fn folders() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::env::var_os("PRESS_STYLE_PATH").map(|v| std::env::split_paths(&v).collect()).unwrap_or_default();
    if let Some(dir) = beside_me() {
        out.push(dir.join("styles"));
        out.push(dir.join("press/styles"));
        out.push(dir);
    }
    if let Some(home) = std::env::var_os("HOME") {
        out.push(Path::new(&home).join(".local/lib/press/styles"));
    }
    out.push("/usr/local/lib/press/styles".into());
    out.push("/usr/lib/press/styles".into());
    out
}

/// What has been chosen: by this program's environment, or for every
/// program by the desktop.
fn chosen() -> Option<String> {
    if let Ok(named) = std::env::var("PRESS_STYLE") {
        return Some(named.trim().to_string());
    }
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config")))?;
    let text = std::fs::read_to_string(config.join("press/style")).ok()?;
    text.lines().next().map(|l| l.trim().to_string())
}

/// The style the interface should wear, if not the one it was born with.
pub fn choose() -> Option<Arc<dyn Style>> {
    let named = chosen().filter(|n| !n.is_empty() && n != "one-bit")?;
    let found = if named.contains('/') {
        load(Path::new(&named))
    } else {
        let file = format!("libpress_style_{named}.so");
        match folders().into_iter().map(|d| d.join(&file)).find(|p| p.is_file()) {
            Some(path) => load(&path),
            None => Err("no library of that name was found".into()),
        }
    };
    match found {
        Ok(style) => Some(style),
        Err(why) => {
            eprintln!("press: the style \"{named}\" is not in use: {why}");
            None
        }
    }
}
