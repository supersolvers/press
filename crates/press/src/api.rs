// SPDX-License-Identifier: GPL-3.0-or-later
//! The implementation behind each exported function. Names and signatures
//! are dictated by the generated `abi` module; a mismatch fails to compile.

use crate::abi::{Align, Icon, Key, LineStyle, Orientation, Status, Typeface, ABI_VERSION, MODIFIER_ALT, MODIFIER_CONTROL, MODIFIER_SHIFT};
use crate::backend;
use crate::support::{deliver, forget, settle_clipboard, with, Handle};
use press_core::{Mods, PointerKind};

type Result<T = ()> = std::result::Result<T, Status>;

// ------------------------------------------------------------------- App

pub fn app_init() -> Result {
    crate::support::init()
}

pub fn app_post(work: crate::support::Work) -> Result {
    crate::support::post(work)
}

pub fn app_run() -> Result {
    if backend::started() || with(|rt| Ok(std::mem::replace(&mut rt.running, true)))? {
        return Err(Status::InvalidState);
    }
    with(|rt| {
        rt.quit = false;
        Ok(())
    })?;
    let result = backend::run();
    let _ = with(|rt| {
        rt.running = false;
        Ok(())
    });
    result
}

pub fn app_copy_data(kind: &str, data: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.copy_data(kind, data)?))?;
    settle_clipboard();
    Ok(())
}

pub fn app_copy_also(kind: &str, data: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.copy_also(kind, data)?))?;
    settle_clipboard();
    Ok(())
}

pub fn app_quit() -> Result {
    with(|rt| {
        rt.quit = true;
        Ok(())
    })
}

pub fn app_start() -> Result {
    if with(|rt| Ok(rt.running))? {
        return Err(Status::InvalidState);
    }
    backend::start()
}

pub fn app_fd() -> Result<i32> {
    backend::fd()
}

pub fn app_dispatch() -> Result<bool> {
    if with(|rt| Ok(rt.running))? {
        return Err(Status::InvalidState); // run is doing the dispatching
    }
    backend::dispatch()
}

pub fn app_stop() -> Result {
    if with(|rt| Ok(rt.running))? || !backend::started() {
        return Err(Status::InvalidState);
    }
    backend::stop()
}

pub fn app_shutdown() -> Result {
    if with(|rt| Ok(rt.running)).unwrap_or(false) {
        return Err(Status::InvalidState);
    }
    if backend::started() {
        backend::stop()?;
    }
    crate::support::shutdown();
    Ok(())
}

pub fn app_set_style(name: &str) -> Result {
    if crate::style::fixed_by_user() {
        return Ok(()); // the user has said otherwise for this run
    }
    let style = crate::style::named(name.trim()).map_err(|why| {
        eprintln!("press: the style \"{name}\" is not in use: {why}");
        Status::InvalidArgument
    })?;
    with(|rt| {
        rt.ui.set_style(style);
        Ok(())
    })
}

pub fn app_abi_version() -> Result<u32> {
    Ok(ABI_VERSION)
}

// ---------------------------------------------------------------- Widget

pub fn widget_destroy(this: Handle) -> Result {
    let gone = with(|rt| Ok(rt.ui.destroy(this)?))?;
    forget(&gone);
    Ok(())
}

pub fn widget_set_enabled(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.set_enabled(this, value)?))
}

pub fn widget_get_enabled(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.enabled(this)?))
}

pub fn widget_set_typeface(this: Handle, face: Typeface) -> Result {
    let face = face_of(face);
    with(|rt| Ok(rt.ui.set_typeface(this, face)?))
}

fn face_of(face: Typeface) -> press_core::Face {
    match face {
        Typeface::Cicero => press_core::Face::Cicero,
        Typeface::Agate => press_core::Face::Agate,
        Typeface::AgateBold => press_core::Face::AgateBold,
        Typeface::Brevier => press_core::Face::Brevier,
        Typeface::Pica => press_core::Face::Pica,
        Typeface::Elite => press_core::Face::Elite,
    }
}

pub fn widget_set_min_size(this: Handle, width: i32, height: i32) -> Result {
    with(|rt| Ok(rt.ui.set_min_size(this, width, height)?))
}

pub fn widget_set_stretch(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.set_stretch(this, value)?))
}

pub fn widget_set_context_menu(this: Handle, menu: Handle) -> Result {
    with(|rt| Ok(rt.ui.set_context_menu(this, menu)?))
}

pub fn widget_focus(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.focus(this)?))
}

// ---------------------------------------------------------------- Window

pub fn window_new(title: &str, width: i32, height: i32) -> Result<Handle> {
    with(|rt| Ok(rt.ui.window_new(title, width, height)?))
}

pub fn window_set_title(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.window_set_title(this, value)?))
}

pub fn window_set_resizable(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.window_set_resizable(this, value)?))
}

pub fn window_paste_data(this: Handle, kind: &str) -> Result {
    with(|rt| Ok(rt.ui.window_paste_data(this, kind)?))?;
    settle_clipboard();
    Ok(())
}

pub fn window_accept_drops(this: Handle, kind: &str) -> Result {
    with(|rt| Ok(rt.ui.window_accept_drops(this, kind)?))
}

pub fn window_set_busy(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.window_set_busy(this, value)?))
}

pub fn window_set_tag(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.window_set_tag(this, value)?))
}

pub fn window_set_full_screen(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.window_set_full_screen(this, value)?))
}

pub fn window_get_full_screen(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.window_full_screen(this)?))
}

pub fn window_set_content(this: Handle, content: Handle) -> Result {
    with(|rt| Ok(rt.ui.window_set_content(this, content)?))
}

pub fn window_set_menu_bar(this: Handle, bar: Handle) -> Result {
    with(|rt| Ok(rt.ui.window_set_menu_bar(this, bar)?))
}

pub fn window_fit(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.window_fit(this)?))
}

pub fn window_set_size(this: Handle, width: i32, height: i32) -> Result {
    with(|rt| Ok(rt.ui.window_set_size(this, width, height)?))
}

pub fn window_focused(this: Handle) -> Result<Handle> {
    with(|rt| Ok(rt.ui.window_focused(this)?.unwrap_or(Handle(0))))
}

pub fn window_show(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.window_show(this)?))
}

pub fn window_snapshot(this: Handle, path: &str, scale: i32) -> Result {
    if !(1..=16).contains(&scale) {
        return Err(Status::InvalidArgument);
    }
    let canvas = with(|rt| {
        // as fine as the picture is to be, for a style that draws fine: sharp, not magnified
        rt.ui.set_fineness(scale);
        // with its own frame, if it has one: the picture is of the window as it is shown
        // (and its shadow, on white as on a desktop of paper)
        let (window, shadow) = rt.ui.render_shadowed(this)?;
        let (left, top, ..) = rt.ui.frame_insets(this);
        let (left, top) = (left + shadow, top + shadow);
        // an open menu hangs over the window and may reach past its edges
        let popups = rt.ui.popups(this);
        if popups.is_empty() && top == 0 {
            rt.ui.set_fineness(1);
            return Ok(window);
        }
        let right = popups.iter().map(|p| left + p.rect.x + p.rect.w).max().unwrap_or(0);
        let bottom = popups.iter().map(|p| top + p.rect.y + p.rect.h).max().unwrap_or(0);
        // on white, which shows through where a frame's corners are rounded off
        let mut whole = press_core::Canvas::like(&window, window.width.max(right), window.height.max(bottom), press_core::WHITE);
        whole.blit(&window, 0, 0);
        for (index, at) in popups.iter().enumerate() {
            if let Some(menu) = rt.ui.render_popup(this, index) {
                whole.blit(&menu, left + at.rect.x, top + at.rect.y);
            }
        }
        rt.ui.set_fineness(1);
        Ok(whole)
    })?;
    // what was not drawn that fine is magnified to it, each pixel a square
    let canvas = canvas.scaled(scale / canvas.scale.max(1));
    let file = std::fs::File::create(path).map_err(|_| Status::Io)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), canvas.width as u32, canvas.height as u32);
    // a window with a photo in it is kept in colour; any other, as it is
    // all black and white, at one bit a pixel
    let colour = canvas.pixels.iter().any(|&p| p != press_core::WHITE && p != press_core::BLACK);
    if colour {
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| Status::Io)?;
        let rows: Vec<u8> = canvas.pixels.iter().flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8]).collect();
        return writer.write_image_data(&rows).map_err(|_| Status::Io);
    }
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::One);
    let mut writer = encoder.write_header().map_err(|_| Status::Io)?;
    let stride = (canvas.width as usize).div_ceil(8);
    let mut rows = vec![0u8; stride * canvas.height as usize];
    for y in 0..canvas.height as usize {
        for x in 0..canvas.width as usize {
            if canvas.pixels[y * canvas.width as usize + x] == press_core::WHITE {
                rows[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    writer.write_image_data(&rows).map_err(|_| Status::Io)
}

pub fn window_hover_at(this: Handle, x: i32, y: i32, milliseconds: i32) -> Result {
    let events = with(|rt| {
        let mut events = rt.ui.pointer(this, x, y, PointerKind::Move)?;
        events.extend(rt.ui.advance(milliseconds.max(0) as u64));
        Ok(events)
    })?;
    deliver(events);
    Ok(())
}

pub fn window_click_at(this: Handle, x: i32, y: i32) -> Result {
    let mut events = with(|rt| Ok(rt.ui.pointer(this, x, y, PointerKind::Press)?))?;
    // There is no clock to wait on, so a menu item chosen by the click
    // skips its blink and is reported at once.
    events.extend(with(|rt| {
        let mut events = rt.ui.pointer(this, x, y, PointerKind::Release)?;
        events.extend(rt.ui.finish_animations());
        Ok(events)
    })?);
    deliver(events);
    settle_clipboard();
    Ok(())
}

pub fn window_drag_at(this: Handle, x0: i32, y0: i32, x1: i32, y1: i32) -> Result {
    deliver(with(|rt| Ok(rt.ui.pointer(this, x0, y0, PointerKind::Press)?))?);
    for step in 1..=4 {
        let (x, y) = (x0 + (x1 - x0) * step / 4, y0 + (y1 - y0) * step / 4);
        deliver(with(|rt| Ok(rt.ui.pointer(this, x, y, PointerKind::Move)?))?);
    }
    deliver(with(|rt| Ok(rt.ui.pointer(this, x1, y1, PointerKind::Release)?))?);
    settle_clipboard();
    Ok(())
}

pub fn window_popup_menu(this: Handle, menu: Handle, x: i32, y: i32) -> Result {
    with(|rt| Ok(rt.ui.window_popup_menu(this, menu, x, y)?))
}

pub fn window_context_click_at(this: Handle, x: i32, y: i32) -> Result {
    deliver(with(|rt| Ok(rt.ui.context(this, x, y)?))?);
    deliver(with(|rt| Ok(rt.ui.right_release(this, x, y)?))?);
    Ok(())
}

pub fn window_side_click(this: Handle, forward: bool) -> Result {
    deliver(with(|rt| Ok(rt.ui.navigate(this, forward)?))?);
    Ok(())
}

/// Turn Modifier bits into the engine's form, refusing any that are not known.
fn mods_from(modifiers: u32) -> Result<Mods> {
    if modifiers & !(MODIFIER_SHIFT | MODIFIER_CONTROL | MODIFIER_ALT) != 0 {
        return Err(Status::InvalidArgument);
    }
    Ok(Mods { shift: modifiers & MODIFIER_SHIFT != 0, ctrl: modifiers & MODIFIER_CONTROL != 0, alt: modifiers & MODIFIER_ALT != 0 })
}

pub fn window_hold_modifiers(this: Handle, modifiers: u32) -> Result {
    let mods = mods_from(modifiers)?;
    with(|rt| {
        crate::support::expect(rt, this, "Window")?;
        rt.ui.set_modifiers(mods);
        Ok(())
    })
}

pub fn window_press_key(this: Handle, key: Key, modifiers: u32) -> Result {
    let mods = mods_from(modifiers)?;
    let key = match key {
        Key::Tab => press_core::Key::Tab,
        Key::Return => press_core::Key::Return,
        Key::Escape => press_core::Key::Escape,
        Key::Backspace => press_core::Key::Backspace,
        Key::Delete => press_core::Key::Delete,
        Key::Left => press_core::Key::Left,
        Key::Right => press_core::Key::Right,
        Key::Up => press_core::Key::Up,
        Key::Down => press_core::Key::Down,
        Key::Home => press_core::Key::Home,
        Key::End => press_core::Key::End,
        Key::Menu => press_core::Key::Menu,
        Key::Context => press_core::Key::Context,
        Key::PageUp => press_core::Key::PageUp,
        Key::PageDown => press_core::Key::PageDown,
        Key::Insert => press_core::Key::Insert,
        f => press_core::Key::F((f as i32 - 20) as u8),
    };
    // There is no clock to wait on here, so a button pressed from the
    // keyboard skips its flash and clicks at once.
    let events = with(|rt| {
        let mut events = rt.ui.key(this, key, mods)?;
        events.extend(rt.ui.finish_animations());
        Ok(events)
    })?;
    deliver(events);
    settle_clipboard();
    Ok(())
}

pub fn window_type_text(this: Handle, text: &str) -> Result {
    let events = with(|rt| {
        let mut events = rt.ui.text(this, text)?;
        events.extend(rt.ui.finish_animations());
        Ok(events)
    })?;
    deliver(events);
    settle_clipboard();
    Ok(())
}

// ----------------------------------------------------------------- Stack

pub fn stack_new(orientation: Orientation) -> Result<Handle> {
    let orientation = match orientation {
        Orientation::Vertical => press_core::Orientation::Vertical,
        Orientation::Horizontal => press_core::Orientation::Horizontal,
    };
    with(|rt| Ok(rt.ui.stack_new(orientation)))
}

pub fn stack_add(this: Handle, child: Handle) -> Result {
    with(|rt| Ok(rt.ui.stack_add(this, child)?))
}

pub fn stack_insert(this: Handle, child: Handle, index: i32) -> Result {
    with(|rt| Ok(rt.ui.stack_insert(this, child, index)?))
}

pub fn stack_set_spacing(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.stack_set_spacing(this, value)?))
}

pub fn stack_set_padding(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.stack_set_padding(this, value)?))
}

pub fn stack_set_align(this: Handle, value: Align) -> Result {
    let value = match value {
        Align::Start => press_core::Align::Start,
        Align::Center => press_core::Align::Center,
        Align::End => press_core::Align::End,
        Align::Fill => press_core::Align::Fill,
    };
    with(|rt| Ok(rt.ui.stack_set_align(this, value)?))
}

// ----------------------------------------------------------------- Label

pub fn label_new(text: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.label_new(text)))
}

pub fn label_set_text(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.label_set_text(this, value)?))
}

// ---------------------------------------------------------------- Button

pub fn button_new(label: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.button_new(label)))
}

pub fn button_set_label(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.button_set_label(this, value)?))
}

pub fn button_set_is_default(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.button_set_default(this, value)?))
}

pub fn button_set_is_cancel(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.button_set_cancel(this, value)?))
}

// -------------------------------------------------------------- Checkbox

pub fn checkbox_new(label: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.checkbox_new(label)))
}

pub fn checkbox_set_label(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.checkbox_set_label(this, value)?))
}

pub fn checkbox_set_checked(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.checkbox_set_checked(this, value)?))
}

pub fn checkbox_get_checked(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.checkbox_checked(this)?))
}

// ------------------------------------------------------------- TextField

pub fn text_field_new(text: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.text_field_new(text)))
}

pub fn text_field_set_text(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.text_field_set_text(this, value)?))
}

pub fn text_field_get_text(this: Handle) -> Result<String> {
    with(|rt| Ok(rt.ui.text_field_text(this)?))
}

pub fn text_field_set_placeholder(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.text_field_set_placeholder(this, value)?))
}

pub fn text_field_set_min_width(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.text_field_set_min_width(this, value)?))
}

pub fn text_field_set_password(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.text_field_set_password(this, value)?))
}

pub fn text_area_new(text: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.text_area_new(text)))
}

pub fn text_field_set_align(this: Handle, value: Align) -> Result {
    let value = match value {
        Align::Center => press_core::Align::Center,
        Align::End => press_core::Align::End,
        Align::Start | Align::Fill => press_core::Align::Start,
    };
    with(|rt| Ok(rt.ui.text_field_set_align(this, value)?))
}

pub fn text_area_set_plain(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.text_area_set_plain(this, value)?))
}

pub fn text_area_set_rows(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.text_area_set_rows(this, value)?))
}

pub fn text_field_select_all(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.text_field_select_all(this)?))
}

pub fn text_field_select(this: Handle, from: i32, to: i32) -> Result {
    with(|rt| Ok(rt.ui.text_field_select(this, from, to)?))
}

pub fn text_field_selection_start(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.text_field_selection(this)?.0))
}

pub fn text_field_selection_end(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.text_field_selection(this)?.1))
}

pub fn text_field_copy(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.text_field_copy(this)?))?;
    settle_clipboard();
    Ok(())
}

pub fn text_field_cut(this: Handle) -> Result {
    let events = with(|rt| Ok(rt.ui.text_field_cut(this)?))?;
    settle_clipboard();
    deliver(events);
    Ok(())
}

pub fn text_field_paste(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.text_field_request_paste(this)?))?;
    settle_clipboard();
    Ok(())
}

// ----------------------------------------------------------------- Menus

pub fn menu_bar_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.menu_bar_new()))
}

pub fn menu_bar_add(this: Handle, menu: Handle) -> Result {
    with(|rt| Ok(rt.ui.menu_bar_add(this, menu)?))
}

pub fn menu_new(title: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.menu_new(title)))
}

pub fn menu_set_title(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.menu_set_title(this, value)?))
}

pub fn menu_add(this: Handle, item: Handle) -> Result {
    with(|rt| Ok(rt.ui.menu_add(this, item)?))
}

pub fn menu_add_separator(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.menu_add_separator(this)?))
}

pub fn menu_item_new(label: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.menu_item_new(label)))
}

pub fn menu_item_set_label(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.menu_item_set_label(this, value)?))
}

pub fn menu_item_set_checked(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.menu_item_set_checked(this, value)?))
}

pub fn menu_item_get_checked(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.menu_item_checked(this)?))
}

pub fn menu_item_set_submenu(this: Handle, menu: Handle) -> Result {
    with(|rt| Ok(rt.ui.menu_item_set_submenu(this, menu)?))
}

pub fn menu_item_set_shortcut(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.menu_item_set_shortcut(this, value)?))
}

// ------------------------------------------------------------- ScrollBar

pub fn scroll_bar_new(orientation: Orientation) -> Result<Handle> {
    let orientation = match orientation {
        Orientation::Vertical => press_core::Orientation::Vertical,
        Orientation::Horizontal => press_core::Orientation::Horizontal,
    };
    with(|rt| Ok(rt.ui.scroll_bar_new(orientation)))
}

pub fn scroll_bar_set_value(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_bar_set_value(this, value)?))
}

pub fn scroll_bar_get_value(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.scroll_bar_value(this)?))
}

pub fn scroll_bar_set_max(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_bar_set_max(this, value)?))
}

pub fn scroll_bar_get_max(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.scroll_bar_max(this)?))
}

pub fn scroll_bar_set_page(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_bar_set_page(this, value)?))
}

pub fn scroll_bar_set_step(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_bar_set_step(this, value)?))
}

pub fn scroll_bar_set_length(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_bar_set_length(this, value)?))
}

// ------------------------------------------------------------ ScrollView

pub fn scroll_view_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.scroll_view_new()))
}

pub fn scroll_view_set_content(this: Handle, content: Handle) -> Result {
    with(|rt| Ok(rt.ui.scroll_view_set_content(this, content)?))
}

pub fn scroll_view_set_width(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_view_set_size(this, Some(value), None)?))
}

pub fn scroll_view_set_height(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_view_set_size(this, None, Some(value))?))
}

pub fn scroll_view_set_scroll_x(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_view_set_offset(this, Some(value), None)?))
}

pub fn scroll_view_get_scroll_x(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.scroll_view_offset(this)?.0))
}

pub fn scroll_view_set_scroll_y(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.scroll_view_set_offset(this, None, Some(value))?))
}

pub fn scroll_view_get_scroll_y(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.scroll_view_offset(this)?.1))
}

// ----------------------------------------------------------- PopupButton

pub fn popup_button_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.popup_button_new()))
}

pub fn popup_button_add(this: Handle, label: &str) -> Result {
    with(|rt| Ok(rt.ui.popup_button_add(this, label)?))
}

pub fn popup_button_clear(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.popup_button_clear(this)?))
}

pub fn popup_button_set_selected(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.popup_button_set_selected(this, value)?))
}

pub fn popup_button_get_selected(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.popup_button_selected(this)?))
}

// ----------------------------------------------------------- RadioButton

pub fn radio_button_new(label: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.radio_button_new(label)))
}

pub fn radio_button_set_label(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.radio_button_set_label(this, value)?))
}

pub fn radio_button_set_checked(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.radio_button_set_checked(this, value)?))
}

pub fn radio_button_get_checked(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.radio_button_checked(this)?))
}

// ---------------------------------------------------------------- Slider

pub fn slider_new(orientation: Orientation) -> Result<Handle> {
    let orientation = match orientation {
        Orientation::Vertical => press_core::Orientation::Vertical,
        Orientation::Horizontal => press_core::Orientation::Horizontal,
    };
    with(|rt| Ok(rt.ui.slider_new(orientation)))
}

pub fn slider_set_value(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.slider_set_value(this, value)?))
}

pub fn slider_get_value(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.slider_value(this)?))
}

pub fn slider_set_max(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.slider_set_max(this, value)?))
}

pub fn slider_get_max(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.slider_max(this)?))
}

pub fn slider_set_step(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.slider_set_step(this, value)?))
}

pub fn slider_set_length(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.slider_set_length(this, value)?))
}

// --------------------------------------------------------------- Divider

pub fn divider_new(orientation: Orientation) -> Result<Handle> {
    let orientation = match orientation {
        Orientation::Vertical => press_core::Orientation::Vertical,
        Orientation::Horizontal => press_core::Orientation::Horizontal,
    };
    with(|rt| Ok(rt.ui.divider_new(orientation)))
}

pub fn divider_set_position(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.divider_set_position(this, value)?))
}

pub fn divider_get_position(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.divider_position(this)?))
}

// --------------------------------------------------------------- ListBox

pub fn list_box_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.list_box_new()))
}

pub fn list_box_add(this: Handle, label: &str) -> Result {
    with(|rt| Ok(rt.ui.list_box_add(this, label)?))
}

pub fn list_box_clear(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.list_box_clear(this)?))
}

pub fn list_box_set_selected(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_selected(this, value)?))
}

pub fn list_box_get_selected(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.list_box_selected(this)?))
}

pub fn list_box_set_rows(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_rows(this, value)?))
}

pub fn list_box_set_min_width(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_min_width(this, value)?))
}

// ------------------------------------------------------ Table, TreeView

pub fn table_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.table_new()))
}

pub fn table_add_column(this: Handle, title: &str, width: i32) -> Result {
    with(|rt| Ok(rt.ui.table_add_column(this, title, width)?))
}

pub fn icon_view_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.icon_view_new()))
}

pub fn icon_view_set_position(this: Handle, index: i32, x: i32, y: i32) -> Result {
    with(|rt| Ok(rt.ui.icon_view_set_position(this, index, x, y)?))
}

pub fn tree_view_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.tree_view_new()))
}

pub fn tree_view_add_child(this: Handle, parent: i32, label: &str) -> Result<i32> {
    with(|rt| Ok(rt.ui.tree_view_add_child(this, parent, label)?))
}

pub fn tree_view_set_expanded(this: Handle, index: i32, expanded: bool) -> Result {
    with(|rt| Ok(rt.ui.tree_view_set_expanded(this, index, expanded)?))
}

pub fn tree_view_set_foldable(this: Handle, index: i32, foldable: bool) -> Result {
    with(|rt| Ok(rt.ui.tree_view_set_foldable(this, index, foldable)?))
}

// ------------------------------------------------- ProgressBar, TabView

pub fn progress_bar_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.progress_bar_new()))
}

pub fn progress_bar_set_value(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.progress_bar_set_value(this, value)?))
}

pub fn progress_bar_get_value(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.progress_bar_value(this)?))
}

pub fn progress_bar_set_max(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.progress_bar_set_max(this, value)?))
}

pub fn progress_bar_get_max(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.progress_bar_max(this)?))
}

pub fn progress_bar_set_busy(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.progress_bar_set_busy(this, value)?))
}

pub fn progress_bar_set_length(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.progress_bar_set_length(this, value)?))
}

pub fn tab_view_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.tab_view_new()))
}

pub fn tab_view_add(this: Handle, title: &str, content: Handle) -> Result {
    with(|rt| Ok(rt.ui.tab_view_add(this, title, content)?))
}

pub fn tab_view_set_selected(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.tab_view_set_selected(this, value)?))
}

pub fn tab_view_get_selected(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.tab_view_selected(this)?))
}

// ------------------------------------------- more for lists and tables

pub fn list_box_set_multiple(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_multiple(this, value)?))
}

pub fn list_box_set_editable(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_editable(this, value)?))
}

pub fn list_box_is_selected(this: Handle, index: i32) -> Result<bool> {
    with(|rt| Ok(rt.ui.list_box_is_selected(this, index)?))
}

pub fn list_box_set_item_selected(this: Handle, index: i32, selected: bool) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_item_selected(this, index, selected)?))
}

pub fn list_box_set_drag_data(this: Handle, index: i32, kind: &str, data: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_drag_data(this, index, kind, data)?))
}

pub fn list_box_add_drag_data(this: Handle, index: i32, kind: &str, data: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.list_box_add_drag_data(this, index, kind, data)?))
}

pub fn list_box_set_file(this: Handle, index: i32, path: &str) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_file(this, index, path)?))
}

pub fn list_box_set_icon(this: Handle, index: i32, icon: Icon) -> Result {
    let icon = match icon {
        Icon::None => press_core::Icon::None,
        Icon::Folder => press_core::Icon::Folder,
        Icon::Document => press_core::Icon::Document,
        Icon::Application => press_core::Icon::Application,
    };
    with(|rt| Ok(rt.ui.list_box_set_icon(this, index, icon)?))
}

pub fn list_box_edit(this: Handle, index: i32, column: i32) -> Result {
    with(|rt| Ok(rt.ui.list_box_edit(this, index, column)?))
}

pub fn table_set_sortable(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.table_set_sortable(this, value)?))
}

pub fn table_sort_by(this: Handle, column: i32, ascending: bool) -> Result {
    with(|rt| Ok(rt.ui.table_sort_by(this, column, ascending)?))
}

pub fn table_mark_order(this: Handle, column: i32) -> Result {
    with(|rt| Ok(rt.ui.table_mark_order(this, column)?))
}

pub fn table_set_column_align(this: Handle, column: i32, align: Align) -> Result {
    let align = match align {
        Align::Center => press_core::Align::Center,
        Align::End => press_core::Align::End,
        Align::Start | Align::Fill => press_core::Align::Start,
    };
    with(|rt| Ok(rt.ui.table_set_column_align(this, column, align)?))
}

pub fn table_set_column_typeface(this: Handle, column: i32, face: Typeface) -> Result {
    let face = face_of(face);
    with(|rt| Ok(rt.ui.table_set_column_typeface(this, column, face)?))
}

pub fn list_box_set_picture(this: Handle, index: i32, art: &str) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_picture(this, index, art)?))
}

pub fn list_box_set_reorderable(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_reorderable(this, value)?))
}

pub fn list_box_set_flush(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_flush(this, value)?))
}

pub fn list_box_move_item(this: Handle, index: i32, position: i32) -> Result {
    with(|rt| Ok(rt.ui.list_box_move_item(this, index, position)?))
}

pub fn list_box_position_of(this: Handle, index: i32) -> Result<i32> {
    with(|rt| Ok(rt.ui.list_box_position_of(this, index)?))
}

pub fn list_box_remove(this: Handle, index: i32) -> Result {
    with(|rt| Ok(rt.ui.list_box_remove(this, index)?))
}

pub fn list_box_set_accepts_rows(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.list_box_set_accepts_rows(this, value)?))
}

/// Read a small PNG and turn it into the text form pictures are given in:
/// X for dark, a full stop for light, a space for see-through.
fn picture_from_png(path: &str) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|_| Status::Io)?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    // whatever the file holds, have it as eight bits a channel
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| Status::Io)?;
    let mut pixels = vec![0u8; reader.output_buffer_size().ok_or(Status::Io)?];
    let frame = reader.next_frame(&mut pixels).map_err(|_| Status::Io)?;
    let (width, height) = (frame.width as usize, frame.height as usize);
    if width > 14 || height > 14 {
        return Err(Status::InvalidArgument); // a picture has to fit in a row
    }
    let channels = frame.color_type.samples();
    let mut art = String::new();
    for row in pixels[..frame.buffer_size()].chunks(frame.line_size).take(height) {
        for px in row.chunks(channels).take(width) {
            // brightness, and how solid: grey or colour, with or without alpha
            let (light, alpha) = match channels {
                1 => (px[0] as u32, 255),
                2 => (px[0] as u32, px[1]),
                3 => ((px[0] as u32 * 3 + px[1] as u32 * 6 + px[2] as u32) / 10, 255),
                _ => ((px[0] as u32 * 3 + px[1] as u32 * 6 + px[2] as u32) / 10, px[3]),
            };
            art.push(if alpha < 128 { ' ' } else if light < 128 { 'X' } else { '.' });
        }
        art.push('\n');
    }
    Ok(art)
}

pub fn list_box_set_picture_file(this: Handle, index: i32, path: &str) -> Result {
    // check the item first, so that a bad index is not reported as a bad file
    with(|rt| Ok(rt.ui.list_box_is_selected(this, index).map(|_| ())?))?;
    let art = picture_from_png(path)?;
    with(|rt| Ok(rt.ui.list_box_set_picture(this, index, &art)?))
}

pub fn tree_view_parent_of(this: Handle, index: i32) -> Result<i32> {
    with(|rt| Ok(rt.ui.tree_view_parent_of(this, index)?))
}

// ---------------------------------------------------------------- Picture

pub fn picture_new(width: i32, height: i32) -> Result<Handle> {
    with(|rt| Ok(rt.ui.picture_new(width, height)?))
}

pub fn picture_clear(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.picture_clear(this)?))
}

pub fn picture_line(this: Handle, x0: i32, y0: i32, x1: i32, y1: i32, style: LineStyle) -> Result {
    let style = match style {
        LineStyle::Solid => press_core::LineStyle::Solid,
        LineStyle::Dotted => press_core::LineStyle::Dotted,
        LineStyle::Dashed => press_core::LineStyle::Dashed,
    };
    with(|rt| Ok(rt.ui.picture_line(this, x0, y0, x1, y1, style)?))
}

pub fn picture_box(this: Handle, x: i32, y: i32, width: i32, height: i32, filled: bool) -> Result {
    with(|rt| Ok(rt.ui.picture_box(this, x, y, width, height, filled)?))
}

pub fn picture_text(this: Handle, x: i32, y: i32, text: &str) -> Result {
    with(|rt| Ok(rt.ui.picture_text(this, x, y, text)?))
}

pub fn picture_text_width(this: Handle, text: &str) -> Result<i32> {
    with(|rt| Ok(rt.ui.picture_text_width(this, text)?))
}

pub fn picture_line_height(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.picture_line_height(this)?))
}

pub fn picture_text_in(this: Handle, x: i32, y: i32, text: &str, face: Typeface) -> Result {
    with(|rt| Ok(rt.ui.picture_text_in(this, x, y, text, face_of(face))?))
}

pub fn picture_text_width_in(this: Handle, text: &str, face: Typeface) -> Result<i32> {
    with(|rt| Ok(rt.ui.picture_text_width_in(this, text, face_of(face))?))
}

pub fn picture_line_height_in(this: Handle, face: Typeface) -> Result<i32> {
    with(|rt| Ok(rt.ui.picture_line_height_in(this, face_of(face))?))
}

pub fn picture_set_description(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.picture_set_description(this, value)?))
}

pub fn picture_fill(this: Handle, x: i32, y: i32, width: i32, height: i32, pattern: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.picture_fill(this, x, y, width, height, pattern)?))
}

pub fn picture_set_takes_keys(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.picture_set_takes_keys(this, value)?))
}

pub fn picture_set_middle_button(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.picture_set_middle_button(this, value)?))
}

pub fn picture_set_right_button(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.picture_set_right_button(this, value)?))
}

pub fn picture_set_raw_keys(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.picture_set_raw_keys(this, value)?))
}

pub fn picture_set_size(this: Handle, width: i32, height: i32) -> Result {
    with(|rt| Ok(rt.ui.picture_set_size(this, width, height)?))
}

pub fn picture_image(this: Handle, x: i32, y: i32, width: i32, height: i32, bits: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.picture_image(this, x, y, width, height, bits)?))
}

pub fn picture_set_color(this: Handle, rgb: u32) -> Result {
    with(|rt| Ok(rt.ui.picture_set_color(this, rgb & 0xff_ffff)?))
}

pub fn picture_fill_color(this: Handle, x: i32, y: i32, width: i32, height: i32, rgb: u32) -> Result {
    with(|rt| Ok(rt.ui.picture_fill_color(this, x, y, width, height, rgb & 0xff_ffff)?))
}

pub fn picture_photo(this: Handle, x: i32, y: i32, width: i32, height: i32, rgb: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.picture_photo(this, x, y, width, height, rgb)?))
}

// ------------------------------------------------------------------ what a
// whole program wants: boxes, bars, tools, pictures, help, steppers, combos

/// The picture in the bytes of a PNG file, as the engine keeps pictures:
/// colour with how solid each pixel is. None for no bytes at all.
fn image_from_png(png: &[u8]) -> Result<Option<press_core::Canvas>> {
    if png.is_empty() {
        return Ok(None);
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
    // whatever the file holds, have it as eight bits a channel
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| Status::InvalidArgument)?;
    let mut pixels = vec![0u8; reader.output_buffer_size().ok_or(Status::InvalidArgument)?];
    let frame = reader.next_frame(&mut pixels).map_err(|_| Status::InvalidArgument)?;
    let (width, height, channels) = (frame.width as i32, frame.height as i32, frame.color_type.samples());
    if width < 1 || height < 1 || width > 1024 || height > 1024 {
        return Err(Status::InvalidArgument);
    }
    let mut canvas = press_core::Canvas::new(width, height);
    for (i, px) in pixels.chunks(channels).take((width * height) as usize).enumerate() {
        // grey or colour, with or without how solid
        let (r, g, b, a) = match channels {
            1 => (px[0], px[0], px[0], 255),
            2 => (px[0], px[0], px[0], px[1]),
            3 => (px[0], px[1], px[2], 255),
            _ => (px[0], px[1], px[2], px[3]),
        };
        canvas.pixels[i] = if a == 0 { 0 } else { (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32 };
    }
    Ok(Some(canvas))
}

pub fn widget_set_tooltip(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.set_tooltip(this, value)?))
}

pub fn button_set_key(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.button_set_key(this, value)?))
}

pub fn stack_set_backdrop(this: Handle, pattern: &[u8]) -> Result {
    with(|rt| Ok(rt.ui.stack_set_backdrop(this, pattern)?))
}

pub fn button_set_down(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.button_set_down(this, value)?))
}

pub fn button_set_flat(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.button_set_flat(this, value)?))
}

pub fn button_set_image(this: Handle, png: &[u8]) -> Result {
    let image = image_from_png(png)?;
    with(|rt| Ok(rt.ui.button_set_image(this, image)?))
}

pub fn list_box_set_image(this: Handle, index: i32, png: &[u8]) -> Result {
    let image = image_from_png(png)?;
    with(|rt| Ok(rt.ui.list_box_set_image(this, index, image)?))
}

pub fn list_box_set_large_image(this: Handle, index: i32, png: &[u8]) -> Result {
    let image = image_from_png(png)?;
    with(|rt| Ok(rt.ui.list_box_set_large_image(this, index, image)?))
}

pub fn group_box_new(title: &str, orientation: Orientation) -> Result<Handle> {
    let orientation = match orientation {
        Orientation::Vertical => press_core::Orientation::Vertical,
        Orientation::Horizontal => press_core::Orientation::Horizontal,
    };
    with(|rt| Ok(rt.ui.group_box_new(title, orientation)))
}

pub fn group_box_set_title(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.group_box_set_title(this, value)?))
}

pub fn toolbar_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.toolbar_new()))
}

pub fn button_row_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.button_row_new()))
}

pub fn status_bar_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.status_bar_new()))
}

pub fn status_bar_set_panels(this: Handle, value: bool) -> Result {
    with(|rt| Ok(rt.ui.status_bar_set_panels(this, value)?))
}

pub fn stepper_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.stepper_new()))
}

pub fn stepper_set_value(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.stepper_set(this, 0, value)?))
}

pub fn stepper_get_value(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.stepper_value(this)?))
}

pub fn stepper_set_min(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.stepper_set(this, 1, value)?))
}

pub fn stepper_set_max(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.stepper_set(this, 2, value)?))
}

pub fn stepper_set_step(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.stepper_set(this, 3, value)?))
}

pub fn combo_box_new(text: &str) -> Result<Handle> {
    with(|rt| Ok(rt.ui.combo_box_new(text)))
}

pub fn combo_box_add(this: Handle, choice: &str) -> Result {
    with(|rt| Ok(rt.ui.combo_box_add(this, choice)?))
}

pub fn combo_box_clear(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.combo_box_clear(this)?))
}

// ----------------------------------------------------------------- RichText

pub fn rich_text_new() -> Result<Handle> {
    with(|rt| Ok(rt.ui.rich_text_new()))
}

pub fn rich_text_set_rtf(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.rich_text_set_rtf(this, value)?))
}

pub fn rich_text_get_rtf(this: Handle) -> Result<String> {
    with(|rt| Ok(rt.ui.rich_text_rtf(this)?))
}

pub fn rich_text_set_text(this: Handle, value: &str) -> Result {
    with(|rt| Ok(rt.ui.rich_text_set_text(this, value)?))
}

pub fn rich_text_get_text(this: Handle) -> Result<String> {
    with(|rt| Ok(rt.ui.rich_text_text(this)?))
}

pub fn rich_text_set_wrap_width(this: Handle, value: i32) -> Result {
    with(|rt| Ok(rt.ui.rich_text_set_wrap(this, value)?))
}

fn rich_look<T>(this: Handle, get: impl FnOnce(&press_core::rich::Look) -> T) -> Result<T> {
    with(|rt| Ok(get(&rt.ui.rich_text_look(this)?)))
}

fn rich_restyle(this: Handle, change: impl Fn(&mut press_core::rich::Look)) -> Result {
    with(|rt| Ok(rt.ui.rich_text_restyle(this, change)?))
}

pub fn rich_text_set_bold(this: Handle, value: bool) -> Result {
    rich_restyle(this, |l| l.bold = value)
}

pub fn rich_text_get_bold(this: Handle) -> Result<bool> {
    rich_look(this, |l| l.bold)
}

pub fn rich_text_set_italic(this: Handle, value: bool) -> Result {
    rich_restyle(this, |l| l.italic = value)
}

pub fn rich_text_get_italic(this: Handle) -> Result<bool> {
    rich_look(this, |l| l.italic)
}

pub fn rich_text_set_underline(this: Handle, value: bool) -> Result {
    rich_restyle(this, |l| l.underline = value)
}

pub fn rich_text_get_underline(this: Handle) -> Result<bool> {
    rich_look(this, |l| l.underline)
}

pub fn rich_text_set_strikeout(this: Handle, value: bool) -> Result {
    rich_restyle(this, |l| l.strike = value)
}

pub fn rich_text_get_strikeout(this: Handle) -> Result<bool> {
    rich_look(this, |l| l.strike)
}

pub fn rich_text_set_font(this: Handle, value: &str) -> Result {
    if value.trim().is_empty() {
        return Err(Status::InvalidArgument);
    }
    let family = value.trim().to_string();
    rich_restyle(this, move |l| l.family = family.clone())
}

pub fn rich_text_get_font(this: Handle) -> Result<String> {
    rich_look(this, |l| l.family.clone())
}

pub fn rich_text_set_size(this: Handle, value: i32) -> Result {
    if !(1..=1600).contains(&value) {
        return Err(Status::InvalidArgument);
    }
    rich_restyle(this, |l| l.size = (value * 2) as u16)
}

pub fn rich_text_get_size(this: Handle) -> Result<i32> {
    rich_look(this, |l| (l.size as i32 + 1) / 2)
}

pub fn rich_text_set_color(this: Handle, value: u32) -> Result {
    rich_restyle(this, |l| l.colour = value & 0xff_ffff)
}

pub fn rich_text_get_color(this: Handle) -> Result<u32> {
    rich_look(this, |l| l.colour)
}

pub fn rich_text_set_justify(this: Handle, value: Align) -> Result {
    use press_core::rich::Justify;
    let justify = match value {
        Align::Start => Justify::Left,
        Align::Center => Justify::Centre,
        Align::End => Justify::Right,
        Align::Fill => Justify::Full,
    };
    with(|rt| Ok(rt.ui.rich_text_reset_paragraphs(this, |p| p.justify = justify)?))
}

pub fn rich_text_get_justify(this: Handle) -> Result<Align> {
    use press_core::rich::Justify;
    with(|rt| {
        Ok(match rt.ui.rich_text_paragraph(this)?.justify {
            Justify::Left => Align::Start,
            Justify::Centre => Align::Center,
            Justify::Right => Align::End,
            Justify::Full => Align::Fill,
        })
    })
}

pub fn rich_text_set_bullet(this: Handle, value: bool) -> Result {
    with(|rt| {
        Ok(rt.ui.rich_text_reset_paragraphs(this, |p| {
            p.bullet = value;
            // a bullet hangs a quarter inch out from its words, as word processors set it
            if value && p.left == 0 && p.first == 0 {
                (p.left, p.first) = (360, -360);
            } else if !value && p.left == 360 && p.first == -360 {
                (p.left, p.first) = (0, 0);
            }
        })?)
    })
}

pub fn rich_text_get_bullet(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.rich_text_paragraph(this)?.bullet))
}

pub fn rich_text_set_indents(this: Handle, left: i32, right: i32, first: i32) -> Result {
    use press_core::rich::pixels_to_twips;
    with(|rt| {
        Ok(rt.ui.rich_text_reset_paragraphs(this, |p| {
            (p.left, p.right, p.first) = (pixels_to_twips(left.max(0)), pixels_to_twips(right.max(0)), pixels_to_twips(first));
        })?)
    })
}

pub fn rich_text_indent_left(this: Handle) -> Result<i32> {
    with(|rt| Ok(press_core::rich::twips_to_pixels(rt.ui.rich_text_paragraph(this)?.left)))
}

pub fn rich_text_indent_right(this: Handle) -> Result<i32> {
    with(|rt| Ok(press_core::rich::twips_to_pixels(rt.ui.rich_text_paragraph(this)?.right)))
}

pub fn rich_text_indent_first(this: Handle) -> Result<i32> {
    with(|rt| Ok(press_core::rich::twips_to_pixels(rt.ui.rich_text_paragraph(this)?.first)))
}

pub fn rich_text_select(this: Handle, from: i32, to: i32) -> Result {
    with(|rt| Ok(rt.ui.rich_text_select(this, from, to)?))
}

pub fn rich_text_selection_start(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.rich_text_selection(this)?.0))
}

pub fn rich_text_selection_end(this: Handle) -> Result<i32> {
    with(|rt| Ok(rt.ui.rich_text_selection(this)?.1))
}

pub fn rich_text_select_all(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.rich_text_select_all(this)?))
}

pub fn rich_text_insert(this: Handle, text: &str) -> Result {
    with(|rt| Ok(rt.ui.rich_text_insert(this, text)?))
}

pub fn rich_text_copy(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.rich_text_copy(this)?))?;
    settle_clipboard();
    Ok(())
}

pub fn rich_text_cut(this: Handle) -> Result {
    let events = with(|rt| Ok(rt.ui.rich_text_cut(this)?))?;
    settle_clipboard();
    deliver(events);
    Ok(())
}

pub fn rich_text_paste(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.rich_text_paste(this)?))?;
    settle_clipboard();
    Ok(())
}

pub fn rich_text_undo(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.rich_text_undo(this)?))
}

pub fn rich_text_redo(this: Handle) -> Result {
    with(|rt| Ok(rt.ui.rich_text_redo(this)?))
}

pub fn rich_text_can_undo(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.rich_text_can_undo(this)?))
}

pub fn rich_text_can_redo(this: Handle) -> Result<bool> {
    with(|rt| Ok(rt.ui.rich_text_can_redo(this)?))
}
