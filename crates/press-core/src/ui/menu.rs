// SPDX-License-Identifier: GPL-3.0-or-later
//! The menu bar and its pull-down menus.
//!
//! A window may carry one menu bar, drawn across the top of its content.
//! Both ways of working a menu are supported: press on a title, drag to an
//! item and let go, as on the original Macintosh; or click the title, then
//! click the item. The keyboard works too.
//!
//! An item may carry a submenu, which opens beside it when the item is
//! pointed at, to any depth.
//!
//! Open menus hang below the bar and may reach past the window's edge, so
//! they are not drawn into the window. A display backend asks
//! [`Ui::popups`] where they go and [`Ui::render_popup`] for each picture,
//! and puts each on a surface of its own. Pointer events for it arrive in the window's
//! coordinates like any others.

use super::*;
use super::picture::OtherButton;

/// Height of the menu bar, including the rule under it.
#[cfg(test)]
pub(super) const BAR_HEIGHT: i32 = 20;
const BAR_INDENT: i32 = 8;
const TITLE_PAD: i32 = crate::style::MENU_TITLE_PAD;
#[cfg(test)]
pub(super) const ITEM_HEIGHT: i32 = 16;
pub(super) const ITEM_LEFT: i32 = crate::style::MENU_ITEM_LEFT; // room for a check mark
const ITEM_RIGHT: i32 = 12;
const SHORTCUT_GAP: i32 = 18;
const ARROW_WIDTH: i32 = 6; // the mark on an item that opens a submenu

// A chosen item blinks before the menu goes away: off, on, off, on.
const BLINK_STEP_MS: u64 = 45;
const BLINK_STEPS: u64 = 4;
const TITLE_FLASH_MS: u64 = 110;
/// A press let go within this long is a click rather than a hold.
const CLICK_MS: u64 = 300;

/// One menu on show: the pulled-down menu, or a submenu hanging off it.
pub(super) struct Level {
    pub menu: Handle,
    /// The item under the pointer or chosen with the arrow keys. In every
    /// level but the last it is the item whose submenu is the next level.
    pub hot: Option<Handle>,
}

/// The menu that is pulled down, and any submenus open from it.
pub(super) struct MenuOpen {
    pub window: Handle,
    /// The pulled-down menu first, then each open submenu in turn.
    pub levels: Vec<Level>,
    /// The pointer button is down: letting go over an item chooses it.
    pub held: bool,
    /// For a menu popped up away from the bar: where its corner is.
    pub at: Option<(i32, i32)>,
    /// The widget a context menu was popped up for.
    pub target: Option<Handle>,
    /// The least width of the first menu's items, for a pop-up button's
    /// list, which is never narrower than the button.
    pub min_inner: i32,
    /// When it opened, and whether the press that opened it is still down:
    /// letting that press go straight away is a click, not a choice.
    pub opened: u64,
    pub opening: bool,
}

/// What an item of the engine's own editing menu does to a text field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
    /// Not editing at all: one of a pop-up button's choices, by its index.
    Pick(usize),
    /// Nor this: one of the things a combo box might say, by its index.
    Fill(usize),
    /// Nor this: something asked of a window from its frame's own menu, by
    /// the number of the title bar's button that does the same.
    Frame(u8),
}

/// A menu's outline and the area of each of its items, in window coordinates.
type Layout = (Rect, Vec<(Handle, Rect)>);

/// Where a display backend should put an open menu or submenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupInfo {
    /// In the window's content coordinates. It may extend outside the window.
    pub rect: Rect,
    /// A word of help, not a menu: it takes no part in what the pointer does.
    pub tip: bool,
}

/// One line of a window's menus, for a desktop that shows the menu bar
/// itself. Entries come parent first: a top-level menu, then its items,
/// with the items of a submenu following the item they hang from.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuEntry {
    /// The menu (for a top-level title) or the menu item.
    pub handle: Handle,
    /// The entry this one is inside; `None` for a title in the bar.
    pub parent: Option<Handle>,
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub shortcut: Option<char>,
    pub separator: bool,
    /// A title, or an item with a submenu: it holds other entries.
    pub is_menu: bool,
}

/// How a shortcut is written beside its item, as the style would have it.
fn shortcut_text(key: char, form: i32) -> String {
    match form {
        1 => format!("Ctrl+{}", key.to_uppercase()),
        2 => format!("^{}", key.to_uppercase()),
        _ => format!("\u{2318}{}", key.to_uppercase()),
    }
}

impl Ui {
    // ------------------------------------------------------------ creation

    pub fn menu_bar_new(&mut self) -> Handle {
        self.add(Kind::MenuBar { menus: Vec::new() })
    }

    pub fn menu_new(&mut self, title: &str) -> Handle {
        self.add(Kind::Menu { title: title.into(), items: Vec::new() })
    }

    pub fn menu_item_new(&mut self, label: &str) -> Handle {
        self.add(Kind::MenuItem { label: label.into(), checked: false, shortcut: None, separator: false, submenu: None, action: None })
    }

    /// Hang a submenu off an item. The item then opens the submenu when it
    /// is pointed at, and no longer raises `activated` itself.
    pub fn menu_item_set_submenu(&mut self, item: Handle, menu: Handle) -> Result<()> {
        if !matches!(self.node(item)?.kind, Kind::MenuItem { separator: false, .. }) {
            return Err(Error::WrongType);
        }
        self.free(menu, "Menu")?;
        // a menu cannot be a submenu of one of its own items
        let mut up = Some(item);
        while let Some(at) = up {
            if at == menu {
                return Err(Error::InvalidArgument);
            }
            up = self.node(at)?.parent;
        }
        let old = match &mut self.node_mut(item)?.kind {
            Kind::MenuItem { submenu, .. } => submenu.replace(menu),
            _ => unreachable!(),
        };
        if let Some(old) = old.and_then(|o| self.nodes.get_mut(o)) {
            old.parent = None;
        }
        self.node_mut(menu)?.parent = Some(item);
        self.close_menu();
        Ok(())
    }

    /// Check that `child` is a free-standing object of the expected class.
    fn free(&self, child: Handle, class: &str) -> Result<()> {
        let node = self.node(child)?;
        if node.kind.class() != class {
            return Err(Error::WrongType);
        }
        if node.parent.is_some() {
            return Err(Error::InvalidState);
        }
        Ok(())
    }

    pub fn window_set_menu_bar(&mut self, window: Handle, bar: Handle) -> Result<()> {
        self.check_window(window)?;
        self.free(bar, "MenuBar")?;
        let old = match &mut self.node_mut(window)?.kind {
            Kind::Window { menu_bar, .. } => menu_bar.replace(bar),
            _ => unreachable!(),
        };
        if let Some(old) = old.and_then(|o| self.nodes.get_mut(o)) {
            old.parent = None;
        }
        self.node_mut(bar)?.parent = Some(window);
        self.menu_open = None;
        Ok(())
    }

    pub fn menu_bar_add(&mut self, bar: Handle, menu: Handle) -> Result<()> {
        if !matches!(self.node(bar)?.kind, Kind::MenuBar { .. }) {
            return Err(Error::WrongType);
        }
        self.free(menu, "Menu")?;
        if let Kind::MenuBar { menus } = &mut self.node_mut(bar)?.kind {
            menus.push(menu);
        }
        self.node_mut(menu)?.parent = Some(bar);
        Ok(())
    }

    pub fn menu_add(&mut self, menu: Handle, item: Handle) -> Result<()> {
        if !matches!(self.node(menu)?.kind, Kind::Menu { .. }) {
            return Err(Error::WrongType);
        }
        self.free(item, "MenuItem")?;
        if let Kind::Menu { items, .. } = &mut self.node_mut(menu)?.kind {
            items.push(item);
        }
        self.node_mut(item)?.parent = Some(menu);
        Ok(())
    }

    /// Append a dividing line. It belongs to the menu and has no handle of
    /// its own outside the engine.
    pub fn menu_add_separator(&mut self, menu: Handle) -> Result<()> {
        if !matches!(self.node(menu)?.kind, Kind::Menu { .. }) {
            return Err(Error::WrongType);
        }
        let line = self.add(Kind::MenuItem { label: String::new(), checked: false, shortcut: None, separator: true, submenu: None, action: None });
        self.menu_add(menu, line)
    }

    // ---------------------------------------------------------- properties

    pub fn menu_set_title(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Menu { title, .. } => {
                *title = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn menu_item_set_label(&mut self, h: Handle, value: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::MenuItem { label, .. } => {
                *label = value.into();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn menu_item_set_checked(&mut self, h: Handle, value: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::MenuItem { checked, .. } => {
                *checked = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn menu_item_checked(&self, h: Handle) -> Result<bool> {
        match &self.node(h)?.kind {
            Kind::MenuItem { checked, .. } => Ok(*checked),
            _ => Err(Error::WrongType),
        }
    }

    /// Set the key that, with the command key, chooses the item: one
    /// character, or nothing to clear it.
    pub fn menu_item_set_shortcut(&mut self, h: Handle, value: &str) -> Result<()> {
        let mut chars = value.chars();
        let key = match (chars.next(), chars.next()) {
            (None, _) => None,
            (Some(c), None) if !c.is_control() && !c.is_whitespace() => c.to_lowercase().next(),
            _ => return Err(Error::InvalidArgument),
        };
        match &mut self.node_mut(h)?.kind {
            Kind::MenuItem { shortcut, .. } => {
                *shortcut = key;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    // ------------------------------------------------- an external menu bar

    /// Say whether the desktop is showing this window's menu bar itself.
    /// While it is, the window draws no bar, and is made shorter by the
    /// bar's height so that its content keeps the room it had (and taller
    /// again when the bar comes back), so a window fitted to its content
    /// stays fitted. Shortcuts keep working.
    pub fn window_set_bar_external(&mut self, window: Handle, external: bool) -> Result<()> {
        let bar_height = self.look.m.menu_bar_height;
        match &mut self.node_mut(window)?.kind {
            Kind::Window { bar_external, menu_bar, height, .. } => {
                if *bar_external != external && menu_bar.is_some() {
                    *height = if external { (*height - bar_height).max(1) } else { *height + bar_height };
                }
                *bar_external = external;
            }
            _ => return Err(Error::WrongType),
        }
        self.dismiss_menu(window);
        Ok(())
    }

    /// Whether the window draws no menu bar of its own: the desktop shows
    /// it elsewhere, or the window fills the screen.
    fn bar_is_external(&self, window: Handle) -> bool {
        matches!(self.nodes.get(window).map(|n| &n.kind), Some(Kind::Window { bar_external: true, .. } | Kind::Window { full_screen: true, .. }))
    }

    /// The whole of a window's menus, for a desktop to show.
    pub fn menu_model(&self, window: Handle) -> Vec<MenuEntry> {
        let mut out = Vec::new();
        // (entry, parent, the menu whose items are its children)
        let mut pending: Vec<(Handle, Option<Handle>)> = self.menus_of(window).into_iter().rev().map(|m| (m, None)).collect();
        while let Some((h, parent)) = pending.pop() {
            let Some(node) = self.nodes.get(h) else { continue };
            let (label, checked, shortcut, separator, inside) = match &node.kind {
                Kind::Menu { title, .. } => (title.clone(), false, None, false, Some(h)),
                Kind::MenuItem { label, checked, shortcut, separator, submenu, .. } => {
                    (label.clone(), *checked, *shortcut, *separator, submenu.filter(|m| self.nodes.get(*m).is_some()))
                }
                _ => continue,
            };
            out.push(MenuEntry { handle: h, parent, label, enabled: node.enabled, checked, shortcut, separator, is_menu: inside.is_some() });
            if let Some(menu) = inside {
                pending.extend(self.items_of(menu).into_iter().rev().map(|item| (item, Some(h))));
            }
        }
        out
    }

    // ------------------------------------------------------- pop-up menus
    //
    // A menu can also be popped up at a point: as the context menu of a
    // widget, with the right button or the context-menu key, or wherever the
    // program asks. It is worked like a pulled-down menu from then on.

    /// Give a widget (or a window) a context menu. The menu belongs to it
    /// from then on and is destroyed with it.
    pub fn set_context_menu(&mut self, widget: Handle, menu: Handle) -> Result<()> {
        if self.node(widget)?.kind.is_menu_part() {
            return Err(Error::WrongType);
        }
        self.free(menu, "Menu")?;
        let old = self.node_mut(widget)?.context_menu.replace(menu);
        if let Some(old) = old.and_then(|o| self.nodes.get_mut(o)) {
            old.parent = None;
        }
        self.node_mut(menu)?.parent = Some(widget);
        Ok(())
    }

    /// The menu to pop up for a widget: its own, or failing that the
    /// editing menu for a text field, or failing that the nearest one
    /// belonging to something it is inside.
    pub(super) fn context_for(&mut self, widget: Handle) -> Option<Handle> {
        let mut up = Some(widget);
        while let Some(at) = up {
            let node = self.nodes.get(at)?;
            if let Some(menu) = node.context_menu.filter(|m| self.nodes.get(*m).is_some_and(|n| n.enabled)) {
                return Some(menu);
            }
            if at == widget && matches!(node.kind, Kind::TextField { .. }) {
                return Some(self.edit_menu_for(widget));
            }
            up = node.parent;
        }
        None
    }

    /// The engine's own Cut, Copy, Paste menu, set up for one text field.
    fn edit_menu_for(&mut self, field: Handle) -> Handle {
        let menu = match self.edit_menu.filter(|m| self.nodes.get(*m).is_some()) {
            Some(menu) => menu,
            None => {
                let menu = self.menu_new("");
                for (label, key, action) in [
                    (Some("Cut"), 'x', Some(EditAction::Cut)),
                    (Some("Copy"), 'c', Some(EditAction::Copy)),
                    (Some("Paste"), 'v', Some(EditAction::Paste)),
                    (None, ' ', None),
                    (Some("Select All"), 'a', Some(EditAction::SelectAll)),
                ] {
                    let item = self.add(Kind::MenuItem {
                        label: label.unwrap_or_default().into(),
                        checked: false,
                        shortcut: label.map(|_| key),
                        separator: label.is_none(),
                        submenu: None,
                        action,
                    });
                    let _ = self.menu_add(menu, item);
                }
                self.edit_menu = Some(menu);
                menu
            }
        };
        // grey out what cannot be done to this field as it stands
        let (selected, empty, secret) = match self.nodes.get(field).map(|n| &n.kind) {
            Some(Kind::TextField { edit, password, .. }) => (edit.has_selection(), edit.text().is_empty(), *password),
            _ => (false, true, false),
        };
        for item in self.items_of(menu) {
            let Some(node) = self.nodes.get_mut(item) else { continue };
            if let Kind::MenuItem { action: Some(action), .. } = &node.kind {
                node.enabled = match action {
                    EditAction::Cut | EditAction::Copy => selected && !secret,
                    EditAction::Paste | EditAction::Pick(_) | EditAction::Fill(_) | EditAction::Frame(_) => true,
                    EditAction::SelectAll => !empty,
                };
            }
        }
        menu
    }

    /// Pop a menu up with its corner at a point in a window.
    pub(super) fn open_popup(&mut self, window: Handle, menu: Handle, x: i32, y: i32, target: Option<Handle>) {
        self.menu_open = Some(MenuOpen { window, levels: vec![Level { menu, hot: None }], held: false, at: Some((x, y)), target, min_inner: 0, opened: self.now, opening: false });
        self.menu_blink = None;
        self.title_flash = None;
        self.dirty = true;
    }

    /// Pop a menu up at a point because the program asks. The menu may be
    /// free-standing or a context menu; not one in a menu bar or a submenu.
    pub fn window_popup_menu(&mut self, window: Handle, menu: Handle, x: i32, y: i32) -> Result<()> {
        self.check_window(window)?;
        let node = self.node(menu)?;
        if !matches!(node.kind, Kind::Menu { .. }) {
            return Err(Error::WrongType);
        }
        if node.parent.and_then(|p| self.nodes.get(p)).is_some_and(|p| p.kind.is_menu_part()) {
            return Err(Error::InvalidState);
        }
        self.open_popup(window, menu, x, y, None);
        Ok(())
    }

    /// The widget drawn at a point, of any kind, innermost first.
    fn widget_at(&self, window: Handle, x: i32, y: i32) -> Option<Handle> {
        let mut found = None;
        let mut pending = self.children(window);
        while let Some(h) = pending.pop() {
            let Some(node) = self.nodes.get(h).filter(|n| n.enabled && n.rect.contains(x, y)) else { continue };
            found = Some(h);
            // through a scroll view, only what shows in its opening
            if !matches!(node.kind, Kind::ScrollView { .. }) || self.view_port(h).is_some_and(|p| p.contains(x, y)) {
                pending = self.children(h);
            } else {
                pending.clear();
            }
        }
        found
    }

    /// The right button was pressed at a point: pop up the context menu of
    /// whatever is there. A text field takes the keyboard first, so the
    /// menu acts on the field that was clicked. A picture that hears the
    /// right button is told of it instead.
    pub fn context(&mut self, window: Handle, x: i32, y: i32) -> Result<Vec<Emitted>> {
        let (w, h) = match &self.node(window)?.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            _ => return Err(Error::WrongType),
        };
        self.arrange(window, Rect::new(0, 0, w, h));
        if self.menu_open.as_ref().is_some_and(|m| m.window == window) {
            self.close_menu(); // a second right click starts again
        }
        let widget = self.widget_at(window, x, y).unwrap_or(window);
        if let Some(out) = self.picture_button_press(widget, x, y, OtherButton::Right) {
            return Ok(out);
        }
        let Some(menu) = self.context_for(widget) else { return Ok(Vec::new()) };
        if matches!(self.nodes.get(widget).map(|n| &n.kind), Some(Kind::TextField { .. })) && self.live_focus(window) != Some(widget) && self.focusable(widget) {
            self.set_focus(window, Some(widget), false);
            // the menu was set up before the field's selection was looked at afresh
            let _ = self.context_for(widget);
        }
        self.open_popup(window, menu, x, y, Some(widget));
        Ok(Vec::new())
    }

    /// The middle button was pressed at a point: told to a picture there
    /// that hears it. Nothing else in press takes the middle button.
    pub fn middle_press(&mut self, window: Handle, x: i32, y: i32) -> Result<Vec<Emitted>> {
        let (w, h) = match &self.node(window)?.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            _ => return Err(Error::WrongType),
        };
        self.arrange(window, Rect::new(0, 0, w, h));
        let widget = self.widget_at(window, x, y).unwrap_or(window);
        Ok(self.picture_button_press(widget, x, y, OtherButton::Middle).unwrap_or_default())
    }

    // ------------------------------------------------------------ geometry

    pub(super) fn menu_bar_of(&self, window: Handle) -> Option<Handle> {
        match self.nodes.get(window).map(|n| &n.kind) {
            Some(Kind::Window { menu_bar, .. }) => menu_bar.filter(|b| self.nodes.get(*b).is_some()),
            _ => None,
        }
    }

    pub(super) fn menus_of(&self, window: Handle) -> Vec<Handle> {
        match self.menu_bar_of(window).and_then(|b| self.nodes.get(b)).map(|n| &n.kind) {
            Some(Kind::MenuBar { menus }) => menus.clone(),
            _ => Vec::new(),
        }
    }

    pub(super) fn items_of(&self, menu: Handle) -> Vec<Handle> {
        match self.nodes.get(menu).map(|n| &n.kind) {
            Some(Kind::Menu { items, .. }) => items.clone(),
            _ => Vec::new(),
        }
    }

    pub(super) fn submenu_of(&self, item: Handle) -> Option<Handle> {
        match self.nodes.get(item).map(|n| &n.kind) {
            Some(Kind::MenuItem { submenu, .. }) => submenu.filter(|m| self.nodes.get(*m).is_some()),
            _ => None,
        }
    }

    /// Every item of a menu, and of its submenus, however deep.
    fn items_deep(&self, menu: Handle) -> Vec<Handle> {
        let mut out = Vec::new();
        let mut pending = vec![menu];
        while let Some(m) = pending.pop() {
            for item in self.items_of(m) {
                out.push(item);
                pending.extend(self.submenu_of(item));
            }
        }
        out
    }

    /// Enabled, and so is everything it hangs from.
    pub(super) fn menu_enabled(&self, h: Handle) -> bool {
        let mut up = Some(h);
        while let Some(at) = up {
            match self.nodes.get(at) {
                Some(n) if matches!(n.kind, Kind::Window { .. }) => return true,
                Some(n) if n.enabled => up = n.parent,
                _ => return false,
            }
        }
        true
    }

    /// An item that can be pointed at: enabled and not a dividing line.
    fn choosable(&self, item: Handle) -> bool {
        matches!(self.nodes.get(item).map(|n| &n.kind), Some(Kind::MenuItem { separator: false, .. })) && self.menu_enabled(item)
    }

    /// Each menu's title area in the bar.
    fn bar_titles(&mut self, window: Handle) -> Vec<(Handle, Rect)> {
        let mut x = BAR_INDENT;
        let mut out = Vec::new();
        for menu in self.menus_of(window) {
            let Some(Kind::Menu { title, .. }) = self.nodes.get(menu).map(|n| &n.kind) else { continue };
            let title = title.clone();
            let w = self.font.width(&title) + 2 * TITLE_PAD;
            out.push((menu, Rect::new(x, 0, w, self.look.m.menu_bar_height - 1)));
            x += w;
        }
        out
    }

    /// The menu whose title in a window's own menu bar a point is over.
    pub(super) fn bar_title_at(&mut self, window: Handle, x: i32, y: i32) -> Option<Handle> {
        if self.menu_bar_of(window).is_none() || self.bar_is_external(window) {
            return None;
        }
        self.bar_titles(window).into_iter().find(|(_, rect)| rect.contains(x, y)).map(|(menu, _)| menu)
    }

    /// Lay a menu out with its top-left corner at a point.
    fn layout(&mut self, menu: Handle, x: i32, y: i32, min_inner: i32) -> Layout {
        let items = self.items_of(menu);
        let (mut label_w, mut key_w) = (0, 0);
        for item in &items {
            if let Some(Kind::MenuItem { label, shortcut, submenu, .. }) = self.nodes.get(*item).map(|n| &n.kind) {
                let (label, shortcut, has_sub) = (label.clone(), *shortcut, submenu.is_some());
                label_w = label_w.max(self.font.width(&label));
                if has_sub {
                    key_w = key_w.max(ARROW_WIDTH); // the arrow sits where a shortcut would
                } else if let Some(key) = shortcut {
                    key_w = key_w.max(self.font.width(&shortcut_text(key, self.look.m.shortcut_form)));
                }
            }
        }
        let inner = ITEM_LEFT + label_w + if key_w > 0 { SHORTCUT_GAP + key_w } else { 0 } + ITEM_RIGHT;
        let inner = inner.max(min_inner);
        let edge = self.look.m.menu_border;
        let frame = Rect::new(x, y, inner + 2 * edge, items.len() as i32 * self.look.m.menu_item_height + 2 * edge);
        let rects = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| (item, Rect::new(frame.x + edge, frame.y + edge + i as i32 * self.look.m.menu_item_height, inner, self.look.m.menu_item_height)))
            .collect();
        (frame, rects)
    }

    /// Where a menu hangs when pulled down from the bar.
    fn menu_geometry(&mut self, window: Handle, menu: Handle) -> Option<Layout> {
        let title = self.bar_titles(window).into_iter().find(|(m, _)| *m == menu)?.1;
        Some(self.layout(menu, title.x, self.look.m.menu_bar_height - 1, title.w + 16))
    }

    /// The open menu of a window, if it is still there to be shown.
    fn open_in(&self, window: Handle) -> Option<&MenuOpen> {
        self.menu_open.as_ref().filter(|m| m.window == window && m.levels.first().is_some_and(|l| self.nodes.get(l.menu).is_some()))
    }

    /// The layout of every level on show: the pulled-down menu, then each
    /// submenu beside the item it hangs from.
    fn open_layouts(&mut self, window: Handle) -> Vec<Layout> {
        let Some((menus, at, min_inner)) = self.open_in(window).map(|m| (m.levels.iter().map(|l| l.menu).collect::<Vec<_>>(), m.at, m.min_inner)) else {
            return Vec::new();
        };
        let mut out: Vec<Layout> = Vec::new();
        for menu in menus {
            let layout = match (out.last(), at) {
                // popped up at a point, or pulled down from its title in the bar
                (None, Some((x, y))) => Some(self.layout(menu, x, y, min_inner)),
                (None, None) => self.menu_geometry(window, menu),
                (Some((frame, items)), _) => {
                    // beside its parent item, overlapping the parent's edge a little
                    let parent = items.iter().find(|(item, _)| self.submenu_of(*item) == Some(menu)).map(|(_, r)| *r);
                    let right = frame.x + frame.w;
                    let edge = self.look.m.menu_border;
                    parent.map(|r| self.layout(menu, right - 2 - edge, r.y - edge, 0))
                }
            };
            match layout {
                Some(layout) => out.push(layout),
                None => break, // the chain was broken by a change to the menus
            }
        }
        out
    }

    /// Where the window's open menu and submenus go, shadows included, in
    /// the order they opened.
    pub fn popups(&mut self, window: Handle) -> Vec<PopupInfo> {
        let shadow = self.look.m.menu_shadow;
        let mut all: Vec<PopupInfo> = self.open_layouts(window).into_iter().map(|(f, _)| PopupInfo { rect: Rect::new(f.x, f.y, f.w + shadow, f.h + shadow), tip: false }).collect();
        // a word of help hangs over the window as a menu does; there is never one while a menu is down
        if let (true, Some((rect, _))) = (all.is_empty(), self.tip_place(window)) {
            all.push(PopupInfo { rect, tip: true });
        }
        all
    }

    // ------------------------------------------------------------- drawing

    pub(super) fn paint_bar(&mut self, window: Handle, canvas: &mut Canvas) {
        if self.menu_bar_of(window).is_none() || self.bar_is_external(window) {
            return;
        }
        self.part(canvas, Part::new(PartKind::MenuBar, Rect::new(0, 0, canvas.width, self.look.m.menu_bar_height)));
        let lit = self.open_in(window).filter(|m| m.at.is_none()).and_then(|m| m.levels.first()).map(|l| l.menu).or(self.title_flash.map(|(m, _)| m));
        for (menu, rect) in self.bar_titles(window) {
            let Some(Kind::Menu { title, .. }) = self.nodes.get(menu).map(|n| &n.kind) else { continue };
            let (title, enabled) = (title.clone(), self.menu_enabled(menu));
            let part = Part::new(PartKind::MenuTitle, rect).text(&title);
            self.part(canvas, part.when(state::ENABLED, enabled).when(state::HELD, lit == Some(menu)).when(state::HOT, self.bar_hot == Some(menu)));
        }
    }

    /// The picture of one of the window's open menus, by its place in
    /// [`Ui::popups`]. Pixels outside its outline and shadow are transparent.
    pub fn render_popup(&mut self, window: Handle, index: usize) -> Option<Canvas> {
        if self.open_in(window).is_none() {
            return self.render_tip(window).filter(|_| index == 0);
        }
        let hot = self.open_in(window)?.levels.get(index)?.hot;
        let (frame, items) = self.open_layouts(window).into_iter().nth(index)?;
        let line = self.font.line_height();
        // a chosen item blinks, starting dark-on-light
        let blink = self.menu_blink.map(|(item, start)| (item, ((self.now - start) / BLINK_STEP_MS) % 2 == 1));
        let ink = self.look.pal.menu_text;
        // the menu itself is of the menu's colour; the room kept for its
        // shadow shows the desktop until the style draws there
        let shadow = self.look.m.menu_shadow;
        let mut canvas = self.sheet(frame.w + shadow, frame.h + shadow, crate::canvas::TRANSPARENT);
        canvas.fill(Rect::new(0, 0, frame.w, frame.h), self.look.pal.menu);
        self.part(&mut canvas, Part::new(PartKind::MenuFrame, Rect::new(0, 0, frame.w + shadow, frame.h + shadow)).numbers(shadow, self.look.m.menu_border, 0));
        for (item, rect) in items {
            let r = Rect::new(rect.x - frame.x, rect.y - frame.y, rect.w, rect.h);
            let Some(Kind::MenuItem { label, checked, shortcut, separator, submenu, .. }) = self.nodes.get(item).map(|n| &n.kind) else { continue };
            let (label, checked, shortcut, separator, has_sub) = (label.clone(), *checked, *shortcut, *separator, submenu.is_some());
            if separator {
                self.part(&mut canvas, Part::new(PartKind::MenuSeparator, r));
                continue;
            }
            let y = r.y + (r.h - line) / 2;
            if checked {
                self.part(&mut canvas, Part::new(PartKind::MenuCheck, Rect::new(r.x, r.y, ITEM_LEFT, r.h)).ink(ink));
            }
            self.font.draw(&mut canvas, &label, r.x + ITEM_LEFT, y, ink);
            if has_sub {
                self.part(&mut canvas, Part::new(PartKind::MenuArrow, r).ink(ink));
            } else if let Some(key) = shortcut {
                let text = shortcut_text(key, self.look.m.shortcut_form);
                let w = self.font.width(&text);
                self.font.draw(&mut canvas, &text, r.x + r.w - ITEM_RIGHT - w + 4, y, ink);
            }
            if !self.choosable(item) {
                self.part(&mut canvas, Part::new(PartKind::Dim, r));
            } else if hot == Some(item) && blink.is_none_or(|(blinking, on)| blinking != item || on) {
                self.part(&mut canvas, Part::new(PartKind::Select, r).numbers(2, 0, 0));
            }
        }
        Some(canvas)
    }

    // --------------------------------------------------------------- input

    fn open_menu(&mut self, window: Handle, menu: Handle, held: bool) {
        self.menu_open = Some(MenuOpen { window, levels: vec![Level { menu, hot: None }], held, at: None, target: None, min_inner: 0, opened: self.now, opening: false });
        self.title_flash = None;
        self.dirty = true;
    }

    pub(super) fn close_menu(&mut self) {
        if self.menu_open.take().is_some() {
            self.dirty = true;
        }
        self.menu_blink = None;
    }

    /// The display put the window's menu away, or wants it gone.
    pub fn dismiss_menu(&mut self, window: Handle) {
        if self.menu_open.as_ref().is_some_and(|m| m.window == window) {
            self.close_menu();
        }
    }

    /// Point at an item of one level (or at nothing). Deeper levels close,
    /// and if the item has a submenu, that opens as the next level.
    fn set_hot(&mut self, level: usize, hot: Option<Handle>) {
        let submenu = hot.filter(|h| self.choosable(*h)).and_then(|h| self.submenu_of(h));
        let Some(open) = &mut self.menu_open else { return };
        let Some(at) = open.levels.get_mut(level) else { return };
        if at.hot == hot {
            return;
        }
        at.hot = hot;
        open.levels.truncate(level + 1);
        if let Some(menu) = submenu {
            open.levels.push(Level { menu, hot: None });
        }
        self.dirty = true;
    }

    /// Start the blink that ends with the item being chosen.
    fn choose(&mut self, item: Handle) {
        self.menu_blink = Some((item, self.now));
        self.dirty = true;
    }

    /// End a blink now: put the menus away and report the choice.
    pub(super) fn finish_menu_blink(&mut self) -> Vec<Emitted> {
        let Some((item, _)) = self.menu_blink.take() else { return Vec::new() };
        let (target, window) = (self.menu_open.as_ref().and_then(|m| m.target), self.menu_open.as_ref().map(|m| m.window));
        self.close_menu();
        if !self.choosable(item) {
            return Vec::new(); // destroyed or disabled while it blinked
        }
        match (self.nodes.get(item).map(|n| &n.kind), target) {
            // the engine's own editing menu acts on the field it was popped up for
            (Some(Kind::MenuItem { action: Some(action), .. }), Some(field)) => {
                let action = *action;
                let done = match action {
                    EditAction::Cut => self.text_field_cut(field),
                    EditAction::Copy => self.text_field_copy(field).map(|_| Vec::new()),
                    EditAction::Paste => self.text_field_request_paste(field).map(|_| Vec::new()),
                    EditAction::SelectAll => self.text_field_select_all(field).map(|_| Vec::new()),
                    EditAction::Pick(index) => Ok(self.popup_pick(field, index)),
                    EditAction::Fill(index) => Ok(self.combo_fill(field, index)),
                    EditAction::Frame(_) => Ok(Vec::new()),
                };
                done.unwrap_or_default()
            }
            // from the menu of a window's own frame: the display does it
            (Some(Kind::MenuItem { action: Some(EditAction::Frame(which)), .. }), None) => {
                let which = *which;
                if let Some(window) = window {
                    self.frame_ask(window, which);
                }
                Vec::new()
            }
            (Some(Kind::MenuItem { action: Some(_), .. }), None) => Vec::new(),
            _ => vec![Emitted { target: item, event: "MenuItem.activated", args: vec![] }],
        }
    }

    /// Give the menu bar first refusal of a pointer event. `None` means it
    /// was not the bar's business and the window's widgets should have it.
    pub(super) fn menu_pointer(&mut self, window: Handle, x: i32, y: i32, kind: PointerKind) -> Option<Vec<Emitted>> {
        // a menu popped up away from the bar needs no bar to exist
        let popup = self.open_in(window).is_some_and(|m| m.at.is_some());
        if !popup {
            self.menu_bar_of(window)?;
            if self.bar_is_external(window) {
                return None; // the desktop works the menus
            }
        }
        let done = Some(Vec::new());
        if self.menu_blink.is_some() {
            return done; // a choice is being shown; nothing else happens meanwhile
        }
        let over_title = if popup {
            None // while it is up, the bar is just somewhere else to click
        } else {
            self.bar_titles(window).into_iter().find(|(_, r)| r.contains(x, y)).map(|(m, _)| m).filter(|m| self.menu_enabled(*m))
        };
        let Some((menu, held)) = self.open_in(window).map(|m| (m.levels[0].menu, m.held)) else {
            if !(0..self.look.m.menu_bar_height).contains(&y) || self.pressed.is_some() {
                return None;
            }
            if let (PointerKind::Press, Some(menu)) = (kind, over_title) {
                self.open_menu(window, menu, true);
            }
            return done;
        };
        // The deepest level under the pointer, and the item there if it can be pointed at.
        let layouts = self.open_layouts(window);
        let under = layouts.iter().enumerate().rev().find(|(_, (frame, _))| frame.contains(x, y)).map(|(i, (_, items))| {
            let item = items.iter().find(|(_, r)| r.contains(x, y)).map(|(item, _)| *item).filter(|item| self.choosable(*item));
            (i, item)
        });
        let deepest = layouts.len().saturating_sub(1);
        // whatever this event is, a release after it no longer ends the opening press
        let ends_opening = kind == PointerKind::Release;
        let set_held = |ui: &mut Ui, held: bool| {
            if let Some(open) = &mut ui.menu_open {
                open.held = held;
            }
        };
        match kind {
            PointerKind::Press => match (over_title, under) {
                (Some(title), _) if title == menu => self.close_menu(),
                (Some(title), _) => self.open_menu(window, title, true),
                (None, Some((level, item))) => {
                    self.set_hot(level, item);
                    set_held(self, true);
                }
                (None, None) => self.close_menu(), // a click anywhere else puts the menus away
            },
            PointerKind::Move => match (over_title, under) {
                // sliding along the bar pulls down each menu in turn
                (Some(title), _) if title != menu => self.open_menu(window, title, held),
                (_, Some((level, item))) => self.set_hot(level, item),
                // off the menus: nothing in the last one stays pointed at
                _ => self.set_hot(deepest, None),
            },
            PointerKind::Leave => self.set_hot(deepest, None),
            // A pop-up button's list opens under the pointer, so the release
            // that ends a plain click lands on an item. Coming straight after
            // the press it is taken as a click, and the list stays up.
            PointerKind::Release if self.menu_open.as_ref().is_some_and(|m| m.opening && self.now - m.opened < CLICK_MS) => set_held(self, false),
            PointerKind::Release if held => match under {
                // an item with a submenu is a way in, not a choice
                Some((_, Some(item))) if self.submenu_of(item).is_none() => self.choose(item),
                // a click on the title, or on something that cannot be chosen, leaves the menu down
                Some(_) => set_held(self, false),
                None if over_title == Some(menu) => set_held(self, false),
                None => self.close_menu(),
            },
            PointerKind::Release => {}
        }
        if ends_opening {
            if let Some(open) = &mut self.menu_open {
                open.opening = false;
            }
        }
        done
    }

    /// The level the arrow keys work in: the last one, unless that is a
    /// submenu the pointer or keyboard has not yet gone into.
    fn key_level(&self) -> usize {
        match self.menu_open.as_ref().map(|m| m.levels.as_slice()) {
            Some([.., last]) if last.hot.is_some() => self.menu_open.as_ref().map_or(0, |m| m.levels.len() - 1),
            Some(levels) => levels.len().saturating_sub(2),
            None => 0,
        }
    }

    /// Move the highlight of a level to its next choosable item.
    fn step_hot(&mut self, level: usize, forward: bool) {
        let Some((menu, hot)) = self.menu_open.as_ref().and_then(|m| m.levels.get(level)).map(|l| (l.menu, l.hot)) else { return };
        let items: Vec<Handle> = self.items_of(menu).into_iter().filter(|i| self.choosable(*i)).collect();
        if items.is_empty() {
            return;
        }
        let n = items.len();
        let next = match (hot.and_then(|h| items.iter().position(|i| *i == h)), forward) {
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
            (None, true) => 0,
            (None, false) => n - 1,
        };
        self.set_hot(level, Some(items[next]));
    }

    /// Give the menus first refusal of a key. `typing` says a text field has
    /// the keyboard, in which case its own editing shortcuts win.
    pub(super) fn menu_key(&mut self, window: Handle, key: Key, mods: Mods, typing: bool) -> Option<Vec<Emitted>> {
        let open = self.open_in(window).map(|m| (m.levels[0].menu, m.at.is_some()));
        if open.is_none() {
            self.menu_bar_of(window)?;
        }
        let done = Some(Vec::new());
        let enabled: Vec<Handle> = self.menus_of(window).into_iter().filter(|m| self.menu_enabled(*m)).collect();
        if let Some((menu, popup)) = open {
            if self.menu_blink.is_some() {
                return done;
            }
            let level = self.key_level();
            let hot = self.menu_open.as_ref().and_then(|m| m.levels.get(level)).and_then(|l| l.hot);
            let has_sub = hot.is_some_and(|h| self.submenu_of(h).is_some());
            let switch = |ui: &mut Ui, forward: bool| {
                // a pop-up has no neighbours in a bar to move along to
                if !enabled.is_empty() && !popup {
                    let n = enabled.len();
                    let at = enabled.iter().position(|m| *m == menu).unwrap_or(0);
                    ui.open_menu(window, enabled[if forward { (at + 1) % n } else { (at + n - 1) % n }], false);
                    ui.step_hot(0, true);
                }
            };
            match key {
                Key::Menu => self.close_menu(),
                // Escape and Left back out of a submenu before anything else
                Key::Escape | Key::Left if level > 0 => {
                    if let Some(open) = &mut self.menu_open {
                        open.levels.truncate(level);
                        self.dirty = true;
                    }
                }
                Key::Escape => self.close_menu(),
                Key::Left => switch(self, false),
                Key::Down => self.step_hot(level, true),
                Key::Up => self.step_hot(level, false),
                // Right and Return go into a submenu; otherwise Right moves along the bar
                Key::Right | Key::Return if has_sub => {
                    // pointing at the item again reopens a submenu that was backed out of
                    self.set_hot(level, None);
                    self.set_hot(level, hot);
                    self.step_hot(level + 1, true);
                }
                Key::Right => switch(self, true),
                Key::Return => {
                    if let Some(item) = hot {
                        self.choose(item);
                    }
                }
                _ => {}
            }
            return done; // an open menu takes every key
        }
        match key {
            Key::Menu if !self.bar_is_external(window) => {
                let first = *enabled.first()?;
                self.open_menu(window, first, false);
                self.step_hot(0, true);
                done
            }
            Key::Char(c) if mods.ctrl && !mods.alt && !(typing && "acxv".contains(c)) => {
                let item = enabled.iter().flat_map(|m| self.items_deep(*m)).find(|i| {
                    self.choosable(*i)
                        && matches!(self.nodes.get(*i).map(|n| &n.kind), Some(Kind::MenuItem { shortcut: Some(k), submenu: None, .. }) if *k == c)
                })?;
                // the title of the menu it is under flashes to show where the command lives
                let mut top = item;
                while let Some(parent) = self.nodes.get(top).and_then(|n| n.parent).filter(|p| !matches!(self.nodes.get(*p).map(|n| &n.kind), Some(Kind::MenuBar { .. }))) {
                    top = parent;
                }
                self.title_flash = Some((top, self.now + TITLE_FLASH_MS));
                self.dirty = true;
                Some(vec![Emitted { target: item, event: "MenuItem.activated", args: vec![] }])
            }
            _ => None,
        }
    }

    /// Typed text while a menu is down: a space does what Return does.
    pub(super) fn menu_text(&mut self, window: Handle, text: &str) -> Option<Vec<Emitted>> {
        self.open_in(window)?;
        if text == " " {
            return self.menu_key(window, Key::Return, Mods::default(), false);
        }
        Some(Vec::new())
    }

    // ---------------------------------------------------------------- time

    /// Advance the menu animations.
    pub(super) fn menu_tick(&mut self) -> Vec<Emitted> {
        let mut out = Vec::new();
        if let Some((_, start)) = self.menu_blink {
            self.dirty = true;
            if self.now >= start + BLINK_STEP_MS * BLINK_STEPS {
                out = self.finish_menu_blink();
            }
        }
        if self.title_flash.is_some_and(|(_, until)| until <= self.now) {
            self.title_flash = None;
            self.dirty = true;
        }
        out
    }

    pub(super) fn menu_deadline(&self) -> Option<u64> {
        let blink = self.menu_blink.map(|(_, start)| start + ((self.now - start) / BLINK_STEP_MS + 1) * BLINK_STEP_MS);
        let flash = self.title_flash.map(|(_, until)| until);
        blink.into_iter().chain(flash).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::TRANSPARENT;

    struct Fixture {
        ui: Ui,
        win: Handle,
        file: Handle,
        edit: Handle,
        quit: Handle,
        copy: Handle,
        paste: Handle,
        field: Handle,
    }

    /// A window with File (New, a line, Quit) and Edit (Copy, Paste) menus
    /// over a text field.
    fn fixture() -> Fixture {
        let mut ui = Ui::new();
        let win = ui.window_new("Menus", 240, 120).unwrap();
        let bar = ui.menu_bar_new();
        let (file, edit) = (ui.menu_new("File"), ui.menu_new("Edit"));
        let (new, quit) = (ui.menu_item_new("New"), ui.menu_item_new("Quit"));
        let (copy, paste) = (ui.menu_item_new("Copy"), ui.menu_item_new("Paste"));
        ui.menu_item_set_shortcut(quit, "Q").unwrap();
        ui.menu_item_set_shortcut(copy, "c").unwrap();
        ui.menu_add(file, new).unwrap();
        ui.menu_add_separator(file).unwrap();
        ui.menu_add(file, quit).unwrap();
        ui.menu_add(edit, copy).unwrap();
        ui.menu_add(edit, paste).unwrap();
        ui.menu_bar_add(bar, file).unwrap();
        ui.menu_bar_add(bar, edit).unwrap();
        ui.window_set_menu_bar(win, bar).unwrap();
        let field = ui.text_field_new("text");
        ui.window_set_content(win, field).unwrap();
        Fixture { ui, win, file, edit, quit, copy, paste, field }
    }

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };
    const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false };

    fn centre(r: Rect) -> (i32, i32) {
        (r.x + r.w / 2, r.y + r.h / 2)
    }

    impl Fixture {
        fn title(&mut self, menu: Handle) -> (i32, i32) {
            centre(self.ui.bar_titles(self.win).into_iter().find(|(m, _)| *m == menu).unwrap().1)
        }
        fn item(&mut self, menu: Handle, item: Handle) -> (i32, i32) {
            centre(self.ui.menu_geometry(self.win, menu).unwrap().1.into_iter().find(|(i, _)| *i == item).unwrap().1)
        }
        fn pointer(&mut self, at: (i32, i32), kind: PointerKind) -> Vec<Emitted> {
            self.ui.pointer(self.win, at.0, at.1, kind).unwrap()
        }
        fn click(&mut self, at: (i32, i32)) {
            assert!(self.pointer(at, PointerKind::Press).is_empty());
            assert!(self.pointer(at, PointerKind::Release).is_empty());
        }
        fn chosen(&mut self) -> Vec<Handle> {
            self.ui.finish_animations().into_iter().filter(|e| e.event == "MenuItem.activated").map(|e| e.target).collect()
        }
    }

    #[test]
    fn click_the_title_then_click_the_item() {
        let mut f = fixture();
        assert_eq!(f.ui.popups(f.win), []);
        let title = f.title(f.file);
        f.click(title);
        let popups = f.ui.popups(f.win);
        assert_eq!(popups.len(), 1, "the menu stays down after a click");
        assert_eq!(popups[0].rect.y, BAR_HEIGHT - 1); // hangs from the bar's rule
        let quit = f.item(f.file, f.quit);
        f.click(quit);
        // the item blinks before anything is reported
        assert!(!f.ui.popups(f.win).is_empty());
        assert!(f.ui.tick(BLINK_STEP_MS * BLINK_STEPS - 1).is_empty());
        let ev = f.ui.tick(BLINK_STEP_MS * BLINK_STEPS);
        assert_eq!(ev, vec![Emitted { target: f.quit, event: "MenuItem.activated", args: vec![] }]);
        assert_eq!(f.ui.popups(f.win), []);
        assert_eq!(f.ui.next_deadline(), None); // nothing is left moving
    }

    #[test]
    fn press_drag_and_release_as_on_the_original() {
        let mut f = fixture();
        let (file, edit) = (f.title(f.file), f.title(f.edit));
        f.pointer(file, PointerKind::Press);
        f.pointer(edit, PointerKind::Move); // sliding along the bar changes menu
        let paste = f.item(f.edit, f.paste);
        f.pointer(paste, PointerKind::Move);
        f.pointer(paste, PointerKind::Release);
        assert_eq!(f.chosen(), [f.paste]);
        // letting go away from any item just puts the menu away
        f.pointer(file, PointerKind::Press);
        f.pointer((200, 100), PointerKind::Move);
        f.pointer((200, 100), PointerKind::Release);
        assert_eq!(f.ui.popups(f.win), []);
        assert!(f.chosen().is_empty());
    }

    #[test]
    fn clicking_away_dismisses_without_touching_the_window() {
        let mut f = fixture();
        let title = f.title(f.edit);
        f.click(title);
        f.ui.text_field_set_text(f.field, "untouched").unwrap();
        f.click((30, 40)); // over the text field, but the menu takes the click
        assert_eq!(f.ui.popups(f.win), []);
        f.ui.text(f.win, "x").unwrap();
        assert_eq!(f.ui.text_field_text(f.field).unwrap(), "x"); // still all selected: no caret was placed
        // a second click on the title also puts the menu away
        f.click(title);
        f.click(title);
        assert_eq!(f.ui.popups(f.win), []);
    }

    #[test]
    fn disabled_parts_cannot_be_chosen() {
        let mut f = fixture();
        f.ui.set_enabled(f.quit, false).unwrap();
        let (title, quit) = (f.title(f.file), f.item(f.file, f.quit));
        f.click(title);
        f.click(quit);
        assert!(f.chosen().is_empty());
        assert!(!f.ui.popups(f.win).is_empty()); // a click on a dead item leaves the menu down
        f.ui.key(f.win, Key::Escape, NONE).unwrap();
        assert!(f.ui.key(f.win, Key::Char('q'), CTRL).unwrap().is_empty());
        f.ui.set_enabled(f.file, false).unwrap();
        f.click(title);
        assert_eq!(f.ui.popups(f.win), []);
    }

    #[test]
    fn the_keyboard_works_the_menus() {
        let mut f = fixture();
        f.ui.key(f.win, Key::Menu, NONE).unwrap();
        assert!(!f.ui.popups(f.win).is_empty());
        f.ui.key(f.win, Key::Down, NONE).unwrap(); // from New, over the dividing line, to Quit
        f.ui.key(f.win, Key::Return, NONE).unwrap();
        assert_eq!(f.chosen(), [f.quit]);
        f.ui.key(f.win, Key::Menu, NONE).unwrap();
        f.ui.key(f.win, Key::Right, NONE).unwrap();
        f.ui.key(f.win, Key::Up, NONE).unwrap(); // wraps to the last item
        // typing goes nowhere while a menu is down, except Space, which chooses
        assert!(f.ui.text(f.win, "zzz").unwrap().is_empty());
        f.ui.text(f.win, " ").unwrap();
        assert_eq!(f.chosen(), [f.paste]);
        assert_eq!(f.ui.text_field_text(f.field).unwrap(), "text");
        f.ui.key(f.win, Key::Menu, NONE).unwrap();
        f.ui.key(f.win, Key::Escape, NONE).unwrap();
        assert_eq!(f.ui.popups(f.win), []);
    }

    #[test]
    fn shortcuts_choose_items_but_yield_to_text_editing() {
        let mut f = fixture();
        let ev = f.ui.key(f.win, Key::Char('q'), CTRL).unwrap();
        assert_eq!(ev, vec![Emitted { target: f.quit, event: "MenuItem.activated", args: vec![] }]);
        assert_eq!(f.ui.next_deadline(), Some(TITLE_FLASH_MS)); // the File title is lit for a moment
        // Control+C in a text field copies; it does not reach Edit > Copy
        assert!(f.ui.key(f.win, Key::Char('c'), CTRL).unwrap().is_empty());
        assert_eq!(f.ui.take_copy().as_deref(), Some("text"));
        // with the keyboard on something else, the menu item gets it
        let button = f.ui.button_new("B");
        f.ui.destroy(f.field).unwrap();
        f.ui.window_set_content(f.win, button).unwrap();
        f.ui.focus(button).unwrap();
        assert_eq!(f.ui.key(f.win, Key::Char('c'), CTRL).unwrap()[0].target, f.copy);
        assert_eq!(f.ui.menu_item_set_shortcut(f.copy, "ab"), Err(Error::InvalidArgument));
    }

    #[test]
    fn the_bar_takes_its_height_from_the_content() {
        let mut f = fixture();
        f.ui.render(f.win).unwrap();
        // the text field starts below the bar: a click just under the rule lands in it
        f.ui.text_field_set_text(f.field, "abc").unwrap();
        f.click((200, BAR_HEIGHT + 8));
        f.ui.text(f.win, "!").unwrap();
        assert_eq!(f.ui.text_field_text(f.field).unwrap(), "abc!");
        // menu parts are not widgets to lay out, and each has one home
        let stack = f.ui.stack_new(Orientation::Vertical);
        assert_eq!(f.ui.stack_add(stack, f.copy), Err(Error::WrongType));
        assert_eq!(f.ui.menu_add(f.file, f.copy), Err(Error::InvalidState));
        assert_eq!(f.ui.menu_add(f.file, stack), Err(Error::WrongType));
    }

    #[test]
    fn destroying_or_deactivating_puts_the_menu_away() {
        let mut f = fixture();
        let title = f.title(f.edit);
        f.click(title);
        f.ui.window_set_active(f.win, false).unwrap();
        assert_eq!(f.ui.popups(f.win), []);
        f.ui.window_set_active(f.win, true).unwrap();
        f.click(title);
        let copy = f.item(f.edit, f.copy);
        f.click(copy); // blinking...
        f.ui.destroy(f.edit).unwrap(); // ...when its menu is destroyed
        assert_eq!(f.ui.popups(f.win), []);
        assert!(f.ui.tick(10_000).is_empty());
        assert_eq!(f.ui.class_of(f.copy), Err(Error::InvalidHandle));
        // the picture of an open menu keeps to black, white and clear
        let title = f.title(f.file);
        f.click(title);
        let picture = f.ui.render_popup(f.win, 0).unwrap();
        assert!(picture.pixels.iter().all(|p| [BLACK, WHITE, TRANSPARENT].contains(p)));
        assert_eq!(picture.get(picture.width - 1, 0), TRANSPARENT);
    }

    /// The fixture with a submenu: Edit > Find > (Find Next, a line, Options > Whole Word).
    fn with_submenus() -> (Fixture, Handle, Handle, Handle, Handle) {
        let mut f = fixture();
        let find = f.ui.menu_item_new("Find");
        let (sub, deeper) = (f.ui.menu_new(""), f.ui.menu_new(""));
        let (next, options, whole) = (f.ui.menu_item_new("Find Next"), f.ui.menu_item_new("Options"), f.ui.menu_item_new("Whole Word"));
        f.ui.menu_item_set_shortcut(next, "g").unwrap();
        f.ui.menu_add(sub, next).unwrap();
        f.ui.menu_add_separator(sub).unwrap();
        f.ui.menu_add(sub, options).unwrap();
        f.ui.menu_add(deeper, whole).unwrap();
        f.ui.menu_item_set_submenu(options, deeper).unwrap();
        f.ui.menu_add(f.edit, find).unwrap();
        f.ui.menu_item_set_submenu(find, sub).unwrap();
        (f, find, next, options, whole)
    }

    /// The middle of an item in one of the open levels.
    fn open_item(f: &mut Fixture, level: usize, item: Handle) -> (i32, i32) {
        centre(f.ui.open_layouts(f.win)[level].1.iter().find(|(i, _)| *i == item).unwrap().1)
    }

    #[test]
    fn pointing_at_an_item_opens_its_submenu() {
        let (mut f, find, next, options, whole) = with_submenus();
        let title = f.title(f.edit);
        f.pointer(title, PointerKind::Press);
        assert_eq!(f.ui.popups(f.win).len(), 1);
        let at = open_item(&mut f, 0, find);
        f.pointer(at, PointerKind::Move);
        let popups = f.ui.popups(f.win);
        assert_eq!(popups.len(), 2);
        // the submenu hangs to the right of its parent, level with the item
        assert!(popups[1].rect.x > popups[0].rect.x + popups[0].rect.w - 8 && (popups[1].rect.y - at.1).abs() < 16);
        // on into a third level, then choose from it by letting go
        let at = open_item(&mut f, 1, options);
        f.pointer(at, PointerKind::Move);
        assert_eq!(f.ui.popups(f.win).len(), 3);
        let at = open_item(&mut f, 2, whole);
        f.pointer(at, PointerKind::Move);
        f.pointer(at, PointerKind::Release);
        assert_eq!(f.chosen(), [whole]);
        assert_eq!(f.ui.popups(f.win), []);

        // moving back to a plain item of the parent closes the submenu
        f.click(title);
        let at = open_item(&mut f, 0, find);
        f.pointer(at, PointerKind::Move);
        let copy = f.copy;
        let copy = open_item(&mut f, 0, copy);
        f.pointer(copy, PointerKind::Move);
        assert_eq!(f.ui.popups(f.win).len(), 1);
        // a click on the parent item itself chooses nothing and leaves things open
        let at = open_item(&mut f, 0, find);
        f.click(at);
        assert!(f.chosen().is_empty());
        assert_eq!(f.ui.popups(f.win).len(), 2);
        let at = open_item(&mut f, 1, next);
        f.click(at);
        assert_eq!(f.chosen(), [next]);
        // each level has its own picture
        f.click(title);
        let at = open_item(&mut f, 0, find);
        f.pointer(at, PointerKind::Move);
        assert!(f.ui.render_popup(f.win, 1).is_some() && f.ui.render_popup(f.win, 2).is_none());
    }

    #[test]
    fn the_keyboard_goes_in_and_out_of_submenus() {
        let (mut f, _find, next, _options, whole) = with_submenus();
        f.ui.key(f.win, Key::Menu, NONE).unwrap();
        f.ui.key(f.win, Key::Right, NONE).unwrap(); // Edit, on Copy
        f.ui.key(f.win, Key::Up, NONE).unwrap(); // wraps to Find; its submenu shows
        assert_eq!(f.ui.popups(f.win).len(), 2);
        f.ui.key(f.win, Key::Right, NONE).unwrap(); // into it, on Find Next
        f.ui.key(f.win, Key::Down, NONE).unwrap(); // over the line to Options
        f.ui.key(f.win, Key::Return, NONE).unwrap(); // Return goes in as well
        assert_eq!(f.ui.popups(f.win).len(), 3);
        f.ui.key(f.win, Key::Left, NONE).unwrap(); // back out one level
        assert_eq!(f.ui.popups(f.win).len(), 2);
        f.ui.key(f.win, Key::Right, NONE).unwrap();
        f.ui.key(f.win, Key::Return, NONE).unwrap();
        assert_eq!(f.chosen(), [whole]);
        // Escape backs out level by level before closing
        f.ui.key(f.win, Key::Menu, NONE).unwrap();
        f.ui.key(f.win, Key::Right, NONE).unwrap();
        f.ui.key(f.win, Key::Up, NONE).unwrap();
        f.ui.key(f.win, Key::Right, NONE).unwrap();
        f.ui.key(f.win, Key::Escape, NONE).unwrap();
        assert_eq!(f.ui.popups(f.win).len(), 1);
        f.ui.key(f.win, Key::Escape, NONE).unwrap();
        assert_eq!(f.ui.popups(f.win), []);
        // a shortcut reaches an item inside a submenu
        assert_eq!(f.ui.key(f.win, Key::Char('g'), CTRL).unwrap()[0].target, next);
    }

    #[test]
    fn submenu_rules() {
        let (mut f, find, next, options, _) = with_submenus();
        let sub = f.ui.submenu_of(find).unwrap();
        let other = f.ui.menu_new("");
        assert_eq!(f.ui.menu_item_set_submenu(next, sub), Err(Error::InvalidState)); // already hangs off Find
        assert_eq!(f.ui.menu_item_set_submenu(next, f.copy), Err(Error::WrongType));
        assert_eq!(f.ui.menu_item_set_submenu(other, other), Err(Error::WrongType)); // not an item
        // disabling the parent item greys out everything beneath it
        f.ui.set_enabled(find, false).unwrap();
        assert!(f.ui.key(f.win, Key::Char('g'), CTRL).unwrap().is_empty());
        f.ui.set_enabled(find, true).unwrap();
        // destroying the item takes its whole submenu with it
        f.ui.destroy(find).unwrap();
        assert_eq!(f.ui.class_of(sub), Err(Error::InvalidHandle));
        assert_eq!(f.ui.class_of(options), Err(Error::InvalidHandle));
        // a submenu shows up for assistive technology as a menu holding its items
        let (mut f, find, next, options, _) = with_submenus();
        let tree = f.ui.accessible(f.win).unwrap();
        let node = tree.nodes.iter().find(|n| n.handle == find).unwrap();
        assert_eq!((node.role, node.children.clone()), (AccessRole::Menu, vec![next, options]));
    }

    #[test]
    fn a_desktop_can_show_the_menu_bar_instead() {
        let (mut f, find, next, _options, whole) = with_submenus();
        f.ui.menu_item_set_checked(f.paste, true).unwrap();
        f.ui.set_enabled(f.copy, false).unwrap();
        // the model lists everything, parents first
        let model = f.ui.menu_model(f.win);
        let labels: Vec<&str> = model.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["File", "New", "", "Quit", "Edit", "Copy", "Paste", "Find", "Find Next", "", "Options", "Whole Word"]);
        let entry = |h: Handle| model.iter().find(|e| e.handle == h).unwrap();
        assert_eq!((entry(f.file).parent, entry(f.file).is_menu), (None, true));
        assert_eq!((entry(f.quit).parent, entry(f.quit).shortcut), (Some(f.file), Some('q')));
        assert!(entry(f.paste).checked && !entry(f.copy).enabled);
        assert_eq!((entry(find).is_menu, entry(next).parent), (true, Some(find)));
        assert!(model[2].separator && model[2].parent == Some(f.file));
        assert_eq!(entry(whole).parent.map(|p| entry(p).label.as_str()), Some("Options"));

        // with the bar shown elsewhere, the window gives its space to the content
        f.ui.text_field_set_text(f.field, "abc").unwrap();
        f.ui.window_set_bar_external(f.win, true).unwrap();
        let top = f.ui.render(f.win).unwrap();
        assert!((0..top.width).any(|x| top.get(x, BAR_HEIGHT - 1) == WHITE)); // no rule across the window
        f.click((200, 8)); // where the bar was is now the text field
        f.ui.text(f.win, "!").unwrap();
        assert_eq!(f.ui.text_field_text(f.field).unwrap(), "abc!");
        // the window's own menus stay shut, but shortcuts and choices still work
        let title = f.title(f.file);
        f.click(title);
        f.ui.key(f.win, Key::Menu, NONE).unwrap();
        assert_eq!(f.ui.popups(f.win), []);
        assert_eq!(f.ui.key(f.win, Key::Char('q'), CTRL).unwrap()[0].target, f.quit);
        assert_eq!(f.ui.assistive_press(next).unwrap()[0].target, next);
    }

    #[test]
    fn a_fitted_window_stays_fitted_when_its_bar_goes_elsewhere() {
        let mut f = fixture();
        f.ui.window_fit(f.win).unwrap();
        let (content_w, content_h) = f.ui.measure(f.field);
        let fitted = f.ui.render(f.win).unwrap();
        assert_eq!(fitted.height, content_h + BAR_HEIGHT);
        // the desktop takes the bar: the window is that much shorter, with no strip left over
        f.ui.window_set_bar_external(f.win, true).unwrap();
        let shorter = f.ui.render(f.win).unwrap();
        assert_eq!((shorter.width, shorter.height), (fitted.width, content_h));
        assert_eq!(f.ui.window_min_size(f.win).unwrap().1, content_h.max(32));
        f.ui.window_set_bar_external(f.win, true).unwrap();
        assert_eq!(f.ui.render(f.win).unwrap().height, content_h, "told twice, shortened once");
        // and taller again when the bar comes back
        f.ui.window_set_bar_external(f.win, false).unwrap();
        assert_eq!(f.ui.render(f.win).unwrap().height, content_h + BAR_HEIGHT);
        // a window fitted while the bar is elsewhere has no room for one
        f.ui.window_set_bar_external(f.win, true).unwrap();
        f.ui.window_fit(f.win).unwrap();
        assert_eq!(f.ui.render(f.win).unwrap().height, content_h.max(32));
        // one that cannot be resized keeps the size it now is
        f.ui.window_set_resizable(f.win, false).unwrap();
        assert_eq!(f.ui.window_fixed_size(f.win).unwrap(), Some((content_w.max(64), content_h.max(32))));
    }

    #[test]
    fn a_window_in_full_screen_draws_no_bar() {
        let mut f = fixture();
        f.ui.window_show(f.win).unwrap();
        assert_eq!(f.ui.window_full_screen(f.win), Ok(false));
        f.ui.window_set_full_screen(f.win, true).unwrap();
        assert_eq!(f.ui.window_full_screen(f.win), Ok(true));
        // the content starts at the very top, and the bar's menus do not pull down
        f.click((200, 8));
        f.ui.text(f.win, "x").unwrap();
        assert_eq!(f.ui.text_field_text(f.field).unwrap(), "textx");
        let title = f.title(f.file);
        f.click(title);
        assert_eq!(f.ui.popups(f.win), []);
        // but the shortcuts still work
        assert_eq!(f.ui.key(f.win, Key::Char('q'), CTRL).unwrap()[0].target, f.quit);
        // the desktop's word is passed on only when it is news
        assert!(f.ui.window_note_full_screen(f.win, true).unwrap().is_empty());
        let left = f.ui.window_note_full_screen(f.win, false).unwrap();
        assert_eq!((left[0].target, left[0].event, left[0].args.clone()), (f.win, "Window.full_screen_changed", vec![Value::Bool(false)]));
        assert_eq!(f.ui.window_full_screen(f.win), Ok(false));
        let top = f.ui.render(f.win).unwrap();
        assert!((0..top.width).all(|x| top.get(x, BAR_HEIGHT - 1) == BLACK || x > 0)); // the bar is back
        assert_eq!(f.ui.window_set_full_screen(f.field, true), Err(Error::WrongType));
    }

    #[test]
    fn a_text_field_pops_up_an_editing_menu() {
        let mut f = fixture();
        f.ui.render(f.win).unwrap();
        // right click in the field: Cut, Copy, Paste, a line, Select All
        f.ui.context(f.win, 60, BAR_HEIGHT + 10).unwrap();
        let popups = f.ui.popups(f.win);
        assert_eq!((popups.len(), popups[0].rect.x, popups[0].rect.y), (1, 60, BAR_HEIGHT + 10));
        let items = f.ui.open_layouts(f.win)[0].1.clone();
        assert_eq!(items.len(), 5);
        // the bar's titles are not lit: this menu is not one of theirs
        let top = f.ui.render(f.win).unwrap();
        assert!((0..top.width).all(|x| top.get(x, 2) == WHITE));
        // Select All, then Cut from a second pop-up, then Paste from a third
        f.click(centre(items[4].1));
        assert!(f.chosen().is_empty()); // the engine's own items report nothing
        f.ui.context(f.win, 60, BAR_HEIGHT + 10).unwrap();
        f.click(centre(items[0].1));
        let cut = f.ui.finish_animations();
        assert_eq!(cut[0].args, vec![Value::Str(String::new())]); // the field reports its change
        assert_eq!(f.ui.take_copy().as_deref(), Some("text"));
        f.ui.context(f.win, 60, BAR_HEIGHT + 10).unwrap();
        // with nothing selected Cut is greyed and cannot be chosen
        f.click(centre(items[0].1));
        assert!(f.ui.finish_animations().is_empty() && !f.ui.popups(f.win).is_empty());
        f.click(centre(items[2].1));
        f.ui.finish_animations();
        assert_eq!(f.ui.take_paste_request(), Some(f.field));
        // the context-menu key does the same for whatever has the keyboard
        f.ui.key(f.win, Key::Context, NONE).unwrap();
        assert_eq!(f.ui.popups(f.win).len(), 1);
        f.ui.key(f.win, Key::Context, NONE).unwrap(); // and puts it away again
        assert_eq!(f.ui.popups(f.win), []);
    }

    #[test]
    fn widgets_can_carry_context_menus_of_their_own() {
        let mut f = fixture();
        let button = f.ui.button_new("B");
        f.ui.destroy(f.field).unwrap();
        f.ui.window_set_content(f.win, button).unwrap();
        let (menu, item) = (f.ui.menu_new(""), f.ui.menu_item_new("Rename"));
        let (sub, deep) = (f.ui.menu_new(""), f.ui.menu_item_new("Deep"));
        let more = f.ui.menu_item_new("More");
        f.ui.menu_add(menu, item).unwrap();
        f.ui.menu_add(menu, more).unwrap();
        f.ui.menu_add(sub, deep).unwrap();
        f.ui.menu_item_set_submenu(more, sub).unwrap();
        f.ui.set_context_menu(f.win, menu).unwrap();
        f.ui.render(f.win).unwrap();
        // anywhere in the window reaches the window's menu, through the button
        f.ui.context(f.win, 100, 60).unwrap();
        let at = centre(f.ui.open_layouts(f.win)[0].1[0].1);
        f.click(at);
        assert_eq!(f.chosen(), [item]);
        // the keyboard works it, submenus and all; Left and Right do not wander off to the bar
        f.ui.context(f.win, 100, 60).unwrap();
        f.ui.key(f.win, Key::Down, NONE).unwrap();
        f.ui.key(f.win, Key::Left, NONE).unwrap();
        f.ui.key(f.win, Key::Down, NONE).unwrap();
        f.ui.key(f.win, Key::Right, NONE).unwrap();
        assert_eq!(f.ui.popups(f.win).len(), 2);
        f.ui.key(f.win, Key::Return, NONE).unwrap();
        assert_eq!(f.chosen(), [deep]);
        // a click outside puts it away and is swallowed
        f.ui.context(f.win, 100, 60).unwrap();
        f.click((5, 100));
        assert_eq!(f.ui.popups(f.win), []);
        // the program can pop a menu up wherever it likes
        f.ui.window_popup_menu(f.win, menu, 30, 40).unwrap();
        assert_eq!(f.ui.popups(f.win)[0].rect.x, 30);
        f.ui.key(f.win, Key::Escape, NONE).unwrap();
        assert_eq!(f.ui.window_popup_menu(f.win, f.file, 0, 0), Err(Error::InvalidState)); // one of the bar's
        assert_eq!(f.ui.window_popup_menu(f.win, sub, 0, 0), Err(Error::InvalidState)); // a submenu
        // a widget's own menu wins over the window's, and goes when the widget goes
        let own = f.ui.menu_new("");
        let own_item = f.ui.menu_item_new("Own");
        f.ui.menu_add(own, own_item).unwrap();
        f.ui.set_context_menu(button, own).unwrap();
        assert_eq!(f.ui.set_context_menu(f.file, own), Err(Error::WrongType));
        f.ui.context(f.win, 20, BAR_HEIGHT + 5).unwrap();
        let at = centre(f.ui.open_layouts(f.win)[0].1[0].1);
        f.click(at);
        assert_eq!(f.chosen(), [own_item]);
        f.ui.destroy(button).unwrap();
        assert_eq!(f.ui.class_of(own_item), Err(Error::InvalidHandle));
    }
}
