// SPDX-License-Identifier: GPL-3.0-or-later
//! Accessibility: each window is published to assistive technology over
//! AT-SPI, the Linux desktop's accessibility bus, through AccessKit.
//!
//! AccessKit runs on a thread of its own and stays dormant until something
//! (a screen reader, say) asks for the window. Its requests arrive on that
//! thread, so they are sent across to the UI thread and the event loop is
//! woken; nothing here touches the engine directly.

use accesskit::{Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId, Role, TextDirection, TextPosition, TextSelection, Toggled, TreeId, TreeInfo, TreeUpdate};
use accesskit_unix::Adapter;
use press_core::{AccessRole, AccessTree, Handle};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

/// Something assistive technology asked to have done to a widget.
pub enum Request {
    Press,
    Focus,
    SetText(String),
    /// Move the caret or select text in a field: where the selection
    /// begins and where the caret goes, each a line and a character in it.
    Select((usize, usize), (usize, usize)),
}

/// A line of a text field has a node of its own, numbered from the
/// field's own number and the line's place in it.
fn run_id(field: Handle, line: usize) -> NodeId {
    NodeId((1 << 63) | ((field.0 & 0xffff_ffff) << 16) | (line as u64 & 0xffff))
}

/// Which line of its field a line's node is.
fn run_line(id: NodeId) -> usize {
    (id.0 & 0xffff) as usize
}

/// Where requests go: (window, widget, what).
pub type Requests = mpsc::Sender<(Handle, Handle, Request)>;

struct Wake {
    fd: Arc<OwnedFd>,
    /// Set when the window has just been asked for and must be described.
    fresh: Arc<AtomicBool>,
}

impl ActivationHandler for Wake {
    /// The description has to come from the UI thread, so none is given
    /// here; the UI thread is woken to supply it.
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.fresh.store(true, Ordering::SeqCst);
        super::poke(self.fd.as_raw_fd());
        None
    }
}

struct Nothing;

impl DeactivationHandler for Nothing {
    fn deactivate_accessibility(&mut self) {}
}

struct Forward {
    window: Handle,
    requests: Requests,
    fd: Arc<OwnedFd>,
}

impl ActionHandler for Forward {
    fn do_action(&mut self, request: ActionRequest) {
        let what = match (request.action, request.data) {
            (Action::Click, _) => Request::Press,
            (Action::Focus, _) => Request::Focus,
            (Action::SetValue | Action::ReplaceSelectedText, Some(ActionData::Value(text))) => Request::SetText(text.into()),
            (Action::SetTextSelection, Some(ActionData::SetTextSelection(to))) => {
                Request::Select((run_line(to.anchor.node), to.anchor.character_index), (run_line(to.focus.node), to.focus.character_index))
            }
            _ => return,
        };
        if self.requests.send((self.window, Handle(request.target_node.0), what)).is_ok() {
            super::poke(self.fd.as_raw_fd());
        }
    }
}

/// One window's presence on the accessibility bus.
pub struct Bridge {
    adapter: Adapter,
    fresh: Arc<AtomicBool>,
}

impl Bridge {
    pub fn new(window: Handle, requests: Requests, wake: Arc<OwnedFd>) -> Bridge {
        let fresh = Arc::new(AtomicBool::new(false));
        let adapter = Adapter::new(Wake { fd: wake.clone(), fresh: fresh.clone() }, Forward { window, requests, fd: wake }, Nothing);
        Bridge { adapter, fresh }
    }

    /// Whether the window has been asked for since this was last called.
    pub fn take_fresh(&self) -> bool {
        self.fresh.swap(false, Ordering::SeqCst)
    }

    /// Tell assistive technology whether the window has the keyboard.
    pub fn set_active(&mut self, active: bool) {
        self.adapter.update_window_focus_state(active);
    }

    /// Publish the window's current state. `describe` is only called if
    /// someone is listening; `zoom` turns engine pixels into screen units.
    pub fn publish(&mut self, zoom: i32, describe: impl FnOnce() -> Option<AccessTree>) {
        self.adapter.update_if_active(|| match describe() {
            Some(tree) => convert(&tree, zoom),
            // the window has gone: an empty window stands in until the bridge is dropped
            None => TreeUpdate {
                nodes: vec![(NodeId(u64::MAX), Node::new(Role::Window))],
                tree: Some(TreeInfo::new(NodeId(u64::MAX))),
                tree_id: TreeId::ROOT,
                focus: NodeId(u64::MAX),
            },
        });
    }
}

fn convert(tree: &AccessTree, zoom: i32) -> TreeUpdate {
    let id = |h: Handle| NodeId(h.0);
    let z = zoom as f64;
    // the lines of text fields, each a node under its field
    let mut lines: Vec<(NodeId, Node)> = Vec::new();
    let mut nodes: Vec<(NodeId, Node)> = tree
        .nodes
        .iter()
        .map(|n| {
            let mut node = Node::new(match n.role {
                AccessRole::Window => Role::Window,
                AccessRole::Group => Role::Group,
                AccessRole::Label => Role::Label,
                AccessRole::Button if n.is_default => Role::DefaultButton,
                AccessRole::Button => Role::Button,
                AccessRole::Checkbox => Role::CheckBox,
                AccessRole::TextField if n.password => Role::PasswordInput,
                AccessRole::TextField if n.multiline => Role::MultilineTextInput,
                AccessRole::TextField => Role::TextInput,
                AccessRole::ScrollBar => Role::ScrollBar,
                AccessRole::Popup => Role::ComboBox,
                AccessRole::RadioButton => Role::RadioButton,
                AccessRole::ProgressBar => Role::ProgressIndicator,
                AccessRole::Image => Role::Image,
                AccessRole::TabView => Role::TabPanel,
                AccessRole::Slider => Role::Slider,
                AccessRole::ListBox => Role::ListBox,
                AccessRole::MenuBar => Role::MenuBar,
                AccessRole::Menu => Role::Menu,
                AccessRole::MenuItem if n.checked.is_some() => Role::MenuItemCheckBox,
                AccessRole::MenuItem => Role::MenuItem,
            });
            // a label's text is its value; everything else has a name
            match n.role {
                AccessRole::Label => node.set_value(n.label.clone()),
                _ if !n.label.is_empty() => node.set_label(n.label.clone()),
                _ => {}
            }
            if let Some(value) = &n.value {
                node.set_value(value.clone());
            }
            if let Some(placeholder) = &n.placeholder {
                node.set_placeholder(placeholder.clone());
            }
            if let Some(r) = n.rect {
                let z = zoom as f64;
                node.set_bounds(accesskit::Rect { x0: r.x as f64 * z, y0: r.y as f64 * z, x1: (r.x + r.w) as f64 * z, y1: (r.y + r.h) as f64 * z });
            }
            if let Some((value, max)) = n.range {
                node.set_numeric_value(value as f64);
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(max as f64);
            }
            if let Some(checked) = n.checked {
                node.set_toggled(Toggled::from(checked));
            }
            if let Some(key) = n.shortcut {
                node.set_keyboard_shortcut(format!("Ctrl+{}", key.to_uppercase()));
            }
            if !n.enabled {
                node.set_disabled();
            }
            match n.role {
                AccessRole::Button | AccessRole::Checkbox | AccessRole::MenuItem | AccessRole::RadioButton => {
                    node.add_action(Action::Click);
                    node.add_action(Action::Focus);
                }
                AccessRole::TextField => {
                    node.add_action(Action::Focus);
                    node.add_action(Action::SetValue);
                }
                _ => {}
            }
            node.set_children(n.children.iter().map(|c| id(*c)).collect::<Vec<_>>());
            if !n.runs.is_empty() {
                // the text line by line and character by character, so that it
                // can be read out a character, a word or a line at a time
                for (line, run) in n.runs.iter().enumerate() {
                    let mut text = Node::new(Role::TextRun);
                    text.set_value(run.text.clone());
                    text.set_text_direction(TextDirection::LeftToRight);
                    let r = run.rect;
                    text.set_bounds(accesskit::Rect { x0: r.x as f64 * z, y0: r.y as f64 * z, x1: (r.x + r.w) as f64 * z, y1: (r.y + r.h) as f64 * z });
                    text.set_character_lengths(run.chars.iter().map(|c| c.0).collect::<Vec<u8>>());
                    text.set_character_positions(run.chars.iter().map(|c| (c.1 as f64 * z) as f32).collect::<Vec<f32>>());
                    text.set_character_widths(run.chars.iter().map(|c| (c.2 as f64 * z) as f32).collect::<Vec<f32>>());
                    text.set_word_starts(run.word_starts.clone());
                    lines.push((run_id(n.handle, line), text));
                }
                node.set_children((0..n.runs.len()).map(|line| run_id(n.handle, line)).collect::<Vec<_>>());
                if let Some((anchor, focus)) = n.selection {
                    let place = |(line, index): (usize, usize)| TextPosition { node: run_id(n.handle, line), character_index: index };
                    node.set_text_selection(TextSelection { anchor: place(anchor), focus: place(focus) });
                }
                node.add_action(Action::SetTextSelection);
            }
            (id(n.handle), node)
        })
        .collect();
    nodes.extend(lines);
    let mut info = TreeInfo::new(id(tree.root));
    info.toolkit_name = Some("press".into());
    info.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
    TreeUpdate { nodes, tree: Some(info), tree_id: TreeId::ROOT, focus: id(tree.focus) }
}
