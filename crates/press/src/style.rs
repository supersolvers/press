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
//!
//! Either may also be a theme, a file of text that names a style and says
//! what of it is to be otherwise: see [`crate::theme`]. `sage` is looked
//! for as a library first and then as `sage.toml`.

use crate::theme::Theme;
use press_core::{Metrics, Painter, Palette, Part, Style};
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The version of the interface below. A library made for an earlier
/// version is still worn, as it was written; one made for any other but
/// these is not used.
pub const ABI: u32 = 10;

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
    // From here on, only read by a style made for the fifth version or later.
    pub d: i32,
    /// The colour the engine would have drawn a small mark in.
    pub ink: u32,
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
    /// From version 9: how many pixels are kept along one of the
    /// interface's, and how many to a row. A style that has not asked to
    /// be drawn fine is never given more than one.
    scale: i32,
    stride: i32,
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
    // From here on, only in a style made for the second version or later.
    /// What it would have the engine do for it: see [`press_core::style::wants`].
    pub wants: u32,
    /// Say which of the machine's typefaces to write in where the interface
    /// would use one of its own families (numbered as [`FACES`] has them),
    /// and say 1; or say 0 to keep the family.
    pub typeface: Option<unsafe extern "C" fn(face: u32, out: *mut RawType) -> i32>,
    // From here on, only in a style made for the tenth version or later.
    /// The colours of its own that a theme may change, the last followed
    /// by one with no name; or null.
    pub colours: *const RawColour,
}

/// One of a style library's own colours, where the library keeps it.
#[repr(C)]
pub struct RawColour {
    pub name: *const c_char,
    pub value: *mut u32,
}

/// What each library's colours were when it was first loaded, by where its
/// list of them is: a library is loaded once however often it is asked
/// for, so a theme's colours would otherwise outlast the theme.
static AS_MADE: Mutex<Vec<(usize, Vec<u32>)>> = Mutex::new(Vec::new());

/// A typeface asked for by a style library.
#[repr(C)]
pub struct RawType {
    /// A family as fontconfig knows it, or several with commas between,
    /// with a NUL after.
    pub family: [c_char; 128],
    /// In pixels.
    pub size: i32,
    pub bold: i32,
    /// From the fifth version: not 0 for whole pixels, with no greys at
    /// the edges of the letters.
    pub crisp: i32,
}

/// The engine's families in the order a style library numbers them.
pub const FACES: [press_core::Face; 6] =
    [press_core::Face::Cicero, press_core::Face::Agate, press_core::Face::AgateBold, press_core::Face::Brevier, press_core::Face::Pica, press_core::Face::Elite];

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
    /// What is to be otherwise than the library has it.
    theme: Theme,
}

impl Foreign {
    /// Ask the library which typeface it would have for a numbered use.
    fn asked_type(&self, number: u32) -> Option<press_core::Type> {
        if let Some(said) = self.theme.typeface(number) {
            return Some(said);
        }
        // (a style said nothing of type before the second version, nor of
        // a frame's title before the fifth)
        if self.raw.abi < if number as usize >= FACES.len() { 5 } else { 2 } {
            return None;
        }
        let ask = self.raw.typeface?;
        let mut out = RawType { family: [0; 128], size: 0, bold: 0, crisp: 0 };
        if unsafe { ask(number, &mut out) } == 0 {
            return None;
        }
        out.family[127] = 0;
        let family = unsafe { CStr::from_ptr(out.family.as_ptr()) }.to_string_lossy().trim().to_string();
        let own = family.starts_with("press:");
        (crate::theme::family_is_plain(&family) && (own || out.size > 0)).then(|| press_core::Type { family, size: out.size, bold: out.bold != 0, crisp: self.raw.abi >= 5 && out.crisp != 0 })
    }
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
        let mut m = press_core::EightyFour::METRICS;
        if let Some(f) = self.raw.metrics {
            unsafe { f(&mut m) };
        }
        self.theme.resize(&mut m);
        // whatever either says, nothing is smaller than a pixel or larger than a window
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
            scroll_bar: m.scroll_bar.clamp(8, 64),
            slider_knob: m.slider_knob.clamp(3, 64),
            tab_height: m.tab_height.clamp(8, 128),
            popup_height: m.popup_height.clamp(8, 128),
            row_height: m.row_height.clamp(8, 128),
            menu_bar_height: m.menu_bar_height.clamp(8, 128),
            menu_item_height: m.menu_item_height.clamp(8, 128),
            menu_shadow: m.menu_shadow.clamp(0, 32),
            frame_title: if self.raw.abi >= 3 { m.frame_title.clamp(0, 96) } else { 0 },
            frame_border: m.frame_border.clamp(0, 32),
            frame_button_width: m.frame_button_width.clamp(0, 96),
            frame_button_height: m.frame_button_height.clamp(0, 96),
            frame_button_gap: m.frame_button_gap.clamp(0, 64),
            frame_button_inset: m.frame_button_inset.clamp(0, 64),
            frame_button_rise: m.frame_button_rise.clamp(-32, 32),
            frame_left: m.frame_left & 0xffff,
            frame_right: m.frame_right & 0xffff,
            menu_in_window: i32::from(self.raw.abi >= 3 && m.menu_in_window != 0),
            frame_tab: if self.raw.abi >= 4 { m.frame_tab.clamp(0, 64) } else { 0 },
            shortcut_form: if self.raw.abi >= 5 { m.shortcut_form.clamp(0, 2) } else { 0 },
            frame_foot: if self.raw.abi >= 5 { m.frame_foot.clamp(0, 48) } else { 0 },
            frame_double: if self.raw.abi >= 5 { m.frame_double.clamp(0, 2) } else { 0 },
            scroll_arrows: if self.raw.abi >= 5 { m.scroll_arrows.clamp(0, 1) } else { 0 },
            tab_align: if self.raw.abi >= 5 { m.tab_align.clamp(0, 1) } else { 0 },
            menu_border: if self.raw.abi >= 5 { m.menu_border.clamp(1, 8) } else { 1 },
            frame_shadow: if self.raw.abi >= 8 { m.frame_shadow.clamp(0, 48) } else { 0 },
            ok_leads: i32::from(self.raw.abi >= 8 && m.ok_leads != 0),
        }
    }

    fn palette(&self) -> Palette {
        // what a style leaves unsaid of the later colours follows what it
        // says of the earlier: lists are on the paper, a pane is of the window
        let mut p = Palette { list: 0, list_ink: 0, pane: 0, tip: 0, tip_text: 0, pointer: 0, pointer_edge: 0, ..press_core::EightyFour::PALETTE };
        if let Some(f) = self.raw.palette {
            unsafe { f(&mut p) };
        }
        self.theme.repaint(&mut p);
        let said = self.raw.abi >= 5;
        Palette {
            list: if said && p.list != 0 { p.list } else { p.paper },
            list_ink: if said && p.list_ink != 0 { p.list_ink } else { p.ink },
            pane: if said && p.pane != 0 { p.pane } else { p.window },
            // a word of help is on the paper, in its ink, unless the style says
            tip: if self.raw.abi >= 6 && p.tip != 0 { p.tip } else { p.paper },
            tip_text: if self.raw.abi >= 6 && p.tip_text != 0 { p.tip_text } else { p.ink },
            // the pointer is black edged with white unless the style says
            pointer: if self.raw.abi >= 7 && p.pointer != 0 { p.pointer } else { press_core::BLACK },
            pointer_edge: if self.raw.abi >= 7 && p.pointer_edge != 0 { p.pointer_edge } else { press_core::WHITE },
            ..p
        }
    }

    fn wants(&self) -> u32 {
        // (asking to be drawn fine came with the surface that says how fine)
        let known = if self.raw.abi >= 9 { !0 } else { !press_core::style::wants::FINE };
        if self.raw.abi >= 2 { self.raw.wants & known } else { 0 }
    }

    fn typeface(&self, face: press_core::Face) -> Option<press_core::Type> {
        let number = FACES.iter().position(|f| *f == face)? as u32;
        self.asked_type(number)
    }

    fn frame_typeface(&self) -> Option<press_core::Type> {
        // the title of a window's own frame is numbered after the families
        self.asked_type(FACES.len() as u32)
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
            d: part.d,
            ink: part.ink,
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
                scale: (*canvas).scale,
                stride: (*canvas).width * (*canvas).scale,
            };
            draw(&raw, &surface) != 0
        }
    }
}

/// Put a library's own colours as it made them, and then as a theme would
/// have them. Says what the theme asked for that the library has not.
unsafe fn colour(raw: &RawStyle, theme: Option<&Theme>) -> Vec<String> {
    let mut own: Vec<(String, *mut u32)> = Vec::new();
    if raw.abi >= 10 && !raw.colours.is_null() {
        // (a list with no end to it is not followed for ever)
        for i in 0..256 {
            let entry = &*raw.colours.add(i);
            if entry.name.is_null() || entry.value.is_null() {
                break;
            }
            own.push((words(entry.name).into_owned(), entry.value));
        }
        let mut as_made = AS_MADE.lock().unwrap_or_else(|e| e.into_inner());
        let key = raw.colours as usize;
        if !as_made.iter().any(|(k, _)| *k == key) {
            as_made.push((key, own.iter().map(|(_, at)| **at).collect()));
        }
        let (_, made) = as_made.iter().find(|(k, _)| *k == key).expect("just put there");
        for ((_, at), was) in own.iter().zip(made) {
            **at = *was;
        }
    }
    let mut doubts = Vec::new();
    for (name, value) in theme.map(|t| t.colours.as_slice()).unwrap_or_default() {
        match own.iter().find(|(n, _)| n == name) {
            Some((_, at)) => **at = *value,
            None if own.is_empty() => doubts.push(format!("the style has no colours a theme can change, so not {name}")),
            None => doubts.push(format!("the style has no colour named {name}; it has: {}", own.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", "))),
        }
    }
    doubts
}

/// Load a style from a library, to be worn as a theme would have it if
/// there is one: `called` is then the name the theme was chosen by.
fn load(path: &Path, theme: Option<(&str, Theme)>) -> Result<Arc<dyn Style>, String> {
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
        if raw.abi == 0 || raw.abi > ABI {
            return Err(format!("it was made for version {} of the style interface, and this is version {ABI}", raw.abi));
        }
        let mut name = if raw.name.is_null() { path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default() } else { words(raw.name).into_owned() };
        let doubts = colour(raw, theme.as_ref().map(|(_, t)| t));
        let theme = match theme {
            Some((called, theme)) => {
                for doubt in theme.doubts.iter().chain(&doubts) {
                    eprintln!("press: in the theme \"{called}\": {doubt}");
                }
                name = called.to_string();
                theme
            }
            None => Theme::default(),
        };
        Ok(Arc::new(Foreign { name, raw, theme }))
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

/// Where a theme is looked for by name: with the styles, in a folder named
/// themes beside them, and where the user's and the machine's own are kept.
fn theme_folders() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in std::env::var_os("PRESS_STYLE_PATH").map(|v| std::env::split_paths(&v).collect::<Vec<_>>()).unwrap_or_default() {
        out.push(dir.join("themes"));
        out.push(dir);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home.as_ref().map(|h| h.join(".config"))) {
        out.push(config.join("press/themes"));
    }
    if let Some(dir) = beside_me() {
        out.push(dir.join("themes"));
        out.push(dir.join("press/themes"));
        // (installed, the library is in lib and the themes in share)
        out.push(dir.join("../share/press/themes"));
    }
    if let Some(home) = home {
        out.push(home.join(".local/share/press/themes"));
    }
    out.push("/usr/local/share/press/themes".into());
    out.push("/usr/share/press/themes".into());
    out
}

/// The library of a style by its name.
fn library(named: &str) -> Option<PathBuf> {
    let file = format!("libpress_style_{named}.so");
    folders().into_iter().map(|d| d.join(&file)).find(|p| p.is_file())
}

/// A style worn as the theme in a file would have it.
fn themed(file: &Path) -> Result<Arc<dyn Style>, String> {
    let text = std::fs::read_to_string(file).map_err(|why| format!("{} cannot be read: {why}", file.display()))?;
    let theme = Theme::parse(&text).map_err(|why| format!("{}: {why}", file.display()))?;
    let called = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    // a theme is of a style from a library: it names one, or says where one
    // is, from the folder the theme is in
    let library = if theme.style.contains('/') {
        file.parent().unwrap_or(Path::new(".")).join(&theme.style)
    } else if theme.style == "eightyfour" {
        return Err(format!("{}: black and white is as it is, and has no themes", file.display()));
    } else {
        library(&theme.style).ok_or_else(|| format!("{}: no library was found for its style, {}", file.display(), theme.style))?
    };
    load(&library, Some((&called, theme)))
}

/// A style by its name, or by the path of its library; or a theme, by the
/// name or the path of its file.
pub fn named(named: &str) -> Result<Arc<dyn Style>, String> {
    // black and white is in the engine
    if named == "eightyfour" {
        return Ok(Arc::new(press_core::EightyFour));
    }
    if named.contains('/') {
        return if named.ends_with(".toml") { themed(Path::new(named)) } else { load(Path::new(named), None) };
    }
    if let Some(path) = library(named) {
        return load(&path, None);
    }
    let file = format!("{named}.toml");
    match theme_folders().into_iter().map(|d| d.join(&file)).find(|p| p.is_file()) {
        Some(path) => themed(&path),
        None => Err("no library of that name was found, and no theme".into()),
    }
}

/// Whether the user has said, for this run, which style to wear: that is
/// not for the program to change.
pub fn fixed_by_user() -> bool {
    std::env::var_os("PRESS_STYLE").is_some()
}

/// The style the interface should wear, if not the one it was born with.
pub fn choose() -> Option<Arc<dyn Style>> {
    let named = chosen().filter(|n| !n.is_empty() && n != "eightyfour")?;
    let found = self::named(&named);
    match found {
        Ok(style) => Some(style),
        Err(why) => {
            eprintln!("press: the style \"{named}\" is not in use: {why}");
            None
        }
    }
}
