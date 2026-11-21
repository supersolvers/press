// SPDX-License-Identifier: GPL-3.0-or-later
//! The menu bar at the top of the screen.
//!
//! A desktop that shows applications' menus itself (KDE Plasma's Global
//! Menu, for one) fetches them over D-Bus with the DBusMenu protocol and is
//! told where to look through a small Wayland protocol. When such a desktop
//! is there, each window's menus are published this way and the window
//! stops drawing a bar of its own.
//!
//! The D-Bus side runs on a thread belonging to the bus library. It only
//! ever reads a copy of the menus kept here, and sends the user's choices
//! across to the UI thread as assistive requests are sent.

use super::a11y::{Request, Requests};
use press_core::{Handle, MenuEntry};
use std::collections::HashMap;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedValue, Structure, Value};

const INTERFACE: &str = "com.canonical.dbusmenu";

/// An entry in DBusMenu's nested form: its id, its properties, and its
/// children, each wrapped as a variant.
type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// What the bus thread serves from.
#[derive(Default)]
struct Model {
    revision: u32,
    entries: Vec<MenuEntry>,
}

/// DBusMenu knows items by small numbers; 0 is the bar itself. A handle's
/// low half is unique among live objects and fits.
fn id_of(h: Handle) -> i32 {
    (h.0 & 0x7fff_ffff) as i32
}

fn owned<'a>(value: impl Into<Value<'a>>) -> OwnedValue {
    value.into().try_to_owned().expect("a plain value")
}

impl Model {
    fn children(&self, id: i32) -> impl Iterator<Item = &MenuEntry> {
        self.entries.iter().filter(move |e| e.parent.map_or(0, id_of) == id)
    }

    fn properties(&self, id: i32) -> HashMap<String, OwnedValue> {
        let mut props = HashMap::new();
        if id == 0 {
            props.insert("children-display".into(), owned("submenu"));
            return props;
        }
        let Some(e) = self.entries.iter().find(|e| id_of(e.handle) == id) else { return props };
        if e.separator {
            props.insert("type".into(), owned("separator"));
            return props;
        }
        // an underscore marks an access key in DBusMenu, so real ones are doubled
        props.insert("label".into(), owned(e.label.replace('_', "__")));
        if !e.enabled {
            props.insert("enabled".into(), owned(false));
        }
        if e.is_menu {
            props.insert("children-display".into(), owned("submenu"));
        } else {
            if e.checked {
                props.insert("toggle-type".into(), owned("checkmark"));
                props.insert("toggle-state".into(), owned(1i32));
            }
            if let Some(key) = e.shortcut {
                // a named key is chosen alone, by its name; anything else with Control
                let keys = match press_core::ui::named_shortcut(key) {
                    Some(name) => vec![if name == "Backspace" { "BackSpace".to_string() } else { name.to_string() }],
                    None => vec!["Control".to_string(), key.to_string()],
                };
                props.insert("shortcut".into(), owned(vec![keys]));
            }
        }
        props
    }

    /// An entry and, down to `depth` levels (-1 for all), what is inside it,
    /// in the nested (id, properties, children) form DBusMenu uses.
    fn layout(&self, id: i32, depth: i32) -> Layout {
        let children = if depth == 0 {
            Vec::new()
        } else {
            self.children(id).map(|c| owned(Structure::from(self.layout(id_of(c.handle), depth - 1)))).collect()
        };
        (id, self.properties(id), children)
    }
}

/// The D-Bus object for one window's menus.
struct Served {
    model: Arc<Mutex<Model>>,
    window: Handle,
    requests: Requests,
    wake: Arc<OwnedFd>,
}

impl Served {
    fn chosen(&self, id: i32) {
        let target = self.model.lock().unwrap().entries.iter().find(|e| id_of(e.handle) == id && !e.is_menu && !e.separator).map(|e| e.handle);
        if let Some(target) = target {
            if self.requests.send((self.window, target, Request::Press)).is_ok() {
                super::poke(self.wake.as_raw_fd());
            }
        }
    }
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl Served {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property)]
    fn text_direction(&self) -> String {
        "ltr".into()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "normal".into()
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_layout(&self, parent_id: i32, recursion_depth: i32, _property_names: Vec<String>) -> (u32, Layout) {
        let model = self.model.lock().unwrap();
        (model.revision, model.layout(parent_id, recursion_depth))
    }

    fn get_group_properties(&self, ids: Vec<i32>, _property_names: Vec<String>) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        let model = self.model.lock().unwrap();
        ids.into_iter().map(|id| (id, model.properties(id))).collect()
    }

    fn get_property(&self, id: i32, name: String) -> zbus::fdo::Result<OwnedValue> {
        self.model.lock().unwrap().properties(id).remove(&name).ok_or_else(|| zbus::fdo::Error::InvalidArgs(name))
    }

    fn event(&self, id: i32, event_id: String, _data: Value<'_>, _timestamp: u32) {
        if event_id == "clicked" {
            self.chosen(id);
        }
    }

    fn event_group(&self, events: Vec<(i32, String, Value<'_>, u32)>) -> Vec<i32> {
        for (id, event_id, ..) in events {
            if event_id == "clicked" {
                self.chosen(id);
            }
        }
        Vec::new()
    }

    /// Nothing is built on demand, so nothing changes when a menu opens.
    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }
}

/// The connection to the session bus, shared by every window.
pub struct Bus {
    connection: Connection,
    next: u32,
}

impl Bus {
    /// Connect, if a desktop is there that shows menus itself.
    pub fn connect() -> Option<Bus> {
        let connection = Connection::session().ok()?;
        let dbus = zbus::blocking::fdo::DBusProxy::new(&connection).ok()?;
        // The press desktop always shows menus itself. Elsewhere, the sign
        // that the desktop does is the registrar that Plasma's Global Menu runs.
        let press = std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.eq_ignore_ascii_case("press"));
        let registrar = "com.canonical.AppMenu.Registrar".try_into().ok()?;
        (press || dbus.name_has_owner(registrar).ok()?).then_some(Bus { connection, next: 0 })
    }

    /// The name by which the desktop can reach this program on the bus.
    pub fn name(&self) -> Option<String> {
        self.connection.unique_name().map(|n| n.to_string())
    }

    /// Start serving a window's menus. Returns the published menus and the
    /// object path the desktop should be told.
    pub fn serve(&mut self, window: Handle, requests: Requests, wake: Arc<OwnedFd>) -> Option<Published> {
        self.next += 1;
        let path = format!("/org/press/MenuBar/{}", self.next);
        let model = Arc::new(Mutex::new(Model::default()));
        let served = Served { model: model.clone(), window, requests, wake };
        self.connection.object_server().at(path.as_str(), served).ok()?;
        Some(Published { connection: self.connection.clone(), path, model })
    }
}

/// One window's menus as published on the bus.
pub struct Published {
    connection: Connection,
    pub path: String,
    model: Arc<Mutex<Model>>,
}

impl Published {
    /// Bring the published menus up to date, and tell the desktop if they
    /// have changed.
    pub fn update(&self, entries: Vec<MenuEntry>) {
        let revision = {
            let mut model = self.model.lock().unwrap();
            if model.entries == entries && model.revision > 0 {
                return;
            }
            model.entries = entries;
            model.revision += 1;
            model.revision
        };
        let _ = self.connection.emit_signal(None::<()>, self.path.as_str(), INTERFACE, "LayoutUpdated", &(revision, 0i32));
    }
}

impl Drop for Published {
    fn drop(&mut self) {
        let _ = self.connection.object_server().remove::<Served, _>(self.path.as_str());
    }
}
