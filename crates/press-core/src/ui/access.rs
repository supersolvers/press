// SPDX-License-Identifier: GPL-3.0-or-later
//! What assistive technology (a screen reader, a switch device) needs to
//! know about a window, and the things it may ask to have done.
//!
//! The engine describes each window as a flat list of nodes. A display
//! backend translates that into whatever the platform speaks.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessRole {
    Window,
    Group,
    Label,
    Button,
    Checkbox,
    TextField,
    ScrollBar,
    RadioButton,
    /// Its range gives its value and the furthest it goes.
    Slider,
    /// Its range gives how much is done; none while it shows only that work is going on.
    ProgressBar,
    /// A picture the program draws; its label says what it shows.
    Image,
    /// Its value is the title of the page showing; that page is its child.
    TabView,
    /// Its value is the text of the selected row.
    ListBox,
    /// A pop-up button; its value is the current choice.
    Popup,
    MenuBar,
    Menu,
    MenuItem,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessNode {
    pub handle: Handle,
    pub role: AccessRole,
    /// The name read out: a caption, a title, a label's text.
    pub label: String,
    /// The contents of a text field.
    pub value: Option<String>,
    /// Greyed prompt of an empty text field.
    pub placeholder: Option<String>,
    /// In window coordinates. Menu parts have no fixed place and report none.
    pub rect: Option<Rect>,
    pub enabled: bool,
    /// For checkboxes and for menu items that carry a check mark.
    pub checked: Option<bool>,
    /// The button that Return presses.
    pub is_default: bool,
    /// A text field of several lines.
    pub multiline: bool,
    /// A text field whose contents are hidden; it reports no value.
    pub password: bool,
    /// For a scroll bar: where it is, and the furthest it goes (from 0).
    pub range: Option<(i32, i32)>,
    /// The key that, with the command key, chooses a menu item.
    pub shortcut: Option<char>,
    pub children: Vec<Handle>,
    /// For a text field that is not a password: its text line by line as
    /// it is drawn, character by character, so that it can be read out and
    /// followed as closely as that.
    pub runs: Vec<AccessRun>,
    /// For such a field: where the selection began and where the caret
    /// is, each as a line and a character in it.
    pub selection: Option<((usize, usize), (usize, usize))>,
}

/// One line of a text field as it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessRun {
    /// Where the line's text is, in window coordinates.
    pub rect: Rect,
    /// The line's text, with the line break on the end if it ends in one.
    pub text: String,
    /// Each character as the reader takes it (a letter with its accents is
    /// one): how many bytes of the text it is, how far from the start of
    /// the line it begins, and how wide it is.
    pub chars: Vec<(u8, i32, i32)>,
    /// Which characters begin a word. A word runs up to the next, taking
    /// the space after it.
    pub word_starts: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessTree {
    pub root: Handle,
    /// The node with the keyboard; the window itself when no widget has it.
    pub focus: Handle,
    pub nodes: Vec<AccessNode>,
}

impl Ui {
    /// Describe a window and everything in it.
    pub fn accessible(&mut self, window: Handle) -> Result<AccessTree> {
        let (w, h) = match &self.node(window)?.kind {
            Kind::Window { width, height, .. } => (*width, *height),
            _ => return Err(Error::WrongType),
        };
        self.arrange(window, Rect::new(0, 0, w, h));
        let focus = self.resolve_focus(window).unwrap_or(window);
        let mut nodes = Vec::new();
        let mut pending = vec![window];
        while let Some(at) = pending.pop() {
            let Some(node) = self.nodes.get(at) else { continue };
            let mut out = AccessNode {
                handle: at,
                role: AccessRole::Group,
                label: String::new(),
                value: None,
                placeholder: None,
                rect: Some(node.rect),
                enabled: node.enabled,
                checked: None,
                is_default: false,
                multiline: false,
                password: false,
                range: None,
                shortcut: None,
                children: Vec::new(),
                runs: Vec::new(),
                selection: None,
            };
            match &node.kind {
                Kind::Window { title, content, menu_bar, .. } => {
                    out.role = AccessRole::Window;
                    out.label = title.clone();
                    out.children = menu_bar.iter().chain(content.iter()).copied().collect();
                }
                Kind::Stack { children, .. } => out.children = children.clone(),
                Kind::ScrollView { content, .. } => out.children = content.iter().copied().collect(),
                Kind::Label { text } => {
                    out.role = AccessRole::Label;
                    out.label = text.clone();
                }
                Kind::Button { label, is_default, .. } => {
                    out.role = AccessRole::Button;
                    out.label = label.clone();
                    out.is_default = *is_default;
                }
                Kind::Checkbox { label, checked } => {
                    out.role = AccessRole::Checkbox;
                    out.label = label.clone();
                    out.checked = Some(*checked);
                }
                Kind::TextField { edit, placeholder, multiline, password, .. } => {
                    out.role = AccessRole::TextField;
                    out.multiline = *multiline;
                    out.password = *password;
                    // what is in a password field is nobody's business but the user's
                    out.value = (!*password).then(|| edit.text().to_string());
                    out.placeholder = Some(placeholder.clone()).filter(|p| !p.is_empty());
                }
                Kind::Picture { description, .. } => {
                    out.role = AccessRole::Image;
                    out.label = description.clone();
                }
                Kind::Progress { value, max, busy, .. } => {
                    out.role = AccessRole::ProgressBar;
                    out.range = (!*busy).then_some((*value, *max));
                }
                Kind::Tabs { pages, selected } => {
                    out.role = AccessRole::TabView;
                    out.value = pages.get(*selected).map(|p| p.0.clone());
                    out.children = pages.get(*selected).map(|p| p.1).into_iter().collect();
                }
                Kind::Radio { label, checked } => {
                    out.role = AccessRole::RadioButton;
                    out.label = label.clone();
                    out.checked = Some(*checked);
                }
                Kind::Slider { value, max, .. } => {
                    out.role = AccessRole::Slider;
                    out.range = Some((*value, *max));
                }
                // a stepper is told of as a slider is: a number and how far it goes
                Kind::Spin { value, max, .. } => {
                    out.role = AccessRole::Slider;
                    out.range = Some((*value, *max));
                }
                Kind::List { .. } => {
                    out.role = AccessRole::ListBox;
                    out.value = self.list_label(at);
                }
                Kind::Popup { .. } => {
                    out.role = AccessRole::Popup;
                    out.value = self.popup_label(at);
                }
                Kind::ScrollBar { value, max, .. } => {
                    out.role = AccessRole::ScrollBar;
                    out.range = Some((*value, *max));
                }
                Kind::MenuBar { menus } => {
                    out.role = AccessRole::MenuBar;
                    out.rect = Some(Rect::new(0, 0, w, self.look.m.menu_bar_height));
                    out.children = menus.clone();
                }
                Kind::Menu { title, items } => {
                    out.role = AccessRole::Menu;
                    out.label = title.clone();
                    out.rect = None;
                    // dividing lines say nothing and are left out
                    out.children = items
                        .iter()
                        .copied()
                        .filter(|i| matches!(self.nodes.get(*i).map(|n| &n.kind), Some(Kind::MenuItem { separator: false, .. })))
                        .collect();
                }
                Kind::MenuItem { label, checked, shortcut, submenu, .. } => {
                    out.label = label.clone();
                    out.rect = None;
                    match submenu.filter(|m| self.nodes.get(*m).is_some()) {
                        // an item with a submenu is presented as that menu
                        Some(sub) => {
                            out.role = AccessRole::Menu;
                            out.children = self
                                .items_of(sub)
                                .into_iter()
                                .filter(|i| matches!(self.nodes.get(*i).map(|n| &n.kind), Some(Kind::MenuItem { separator: false, .. })))
                                .collect();
                        }
                        None => {
                            out.role = AccessRole::MenuItem;
                            out.checked = Some(*checked).filter(|c| *c);
                            out.shortcut = *shortcut;
                        }
                    }
                }
            }
            pending.extend(out.children.iter().rev());
            nodes.push(out);
        }
        // a field's text line by line, for reading out; never a password's
        for node in nodes.iter_mut().filter(|n| n.role == AccessRole::TextField && !n.password) {
            (node.runs, node.selection) = self.field_runs(node.handle);
        }
        Ok(AccessTree { root: window, focus, nodes })
    }

    /// Do what a click would, at the request of assistive technology. There
    /// is no pointer to watch, so nothing flashes or blinks first.
    pub fn assistive_press(&mut self, h: Handle) -> Result<Vec<Emitted>> {
        let enabled = {
            let mut up = Some(h);
            let mut all = true;
            while let Some(at) = up {
                let node = self.node(at)?;
                all &= node.enabled || matches!(node.kind, Kind::Window { .. });
                up = node.parent;
            }
            all
        };
        if !enabled {
            return Err(Error::InvalidState);
        }
        self.dirty = true;
        match &mut self.nodes.get_mut(h).ok_or(Error::InvalidHandle)?.kind {
            Kind::Button { .. } => Ok(vec![Emitted { target: h, event: "Button.clicked", args: vec![] }]),
            Kind::Checkbox { checked, .. } => {
                *checked = !*checked;
                Ok(vec![Emitted { target: h, event: "Checkbox.toggled", args: vec![Value::Bool(*checked)] }])
            }
            Kind::Radio { .. } => Ok(self.radio_pick(h)),
            Kind::MenuItem { separator: false, submenu: None, .. } => {
                self.close_menu();
                Ok(vec![Emitted { target: h, event: "MenuItem.activated", args: vec![] }])
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Replace a text field's contents at the request of assistive
    /// technology (dictation, for instance). Unlike a set from the program,
    /// this is the user's doing, so `changed` is raised.
    /// Assistive technology moves the caret or selects text in a field:
    /// from where the selection begins to where the caret goes, each a
    /// line and a character in it, as `AccessNode::selection` gives them.
    pub fn assistive_select(&mut self, h: Handle, anchor: (usize, usize), focus: (usize, usize)) -> Result<Vec<Emitted>> {
        let (from, to) = (self.field_offset(h, anchor), self.field_offset(h, focus));
        let edit = self.editor_mut(h)?;
        edit.move_to(from, false);
        edit.move_to(to, true);
        Ok(Vec::new())
    }

    pub fn assistive_set_text(&mut self, h: Handle, text: &str) -> Result<Vec<Emitted>> {
        self.editor_mut(h)?.set_text(text);
        Ok(vec![self.field_event(h, "TextField.changed")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_is_described_for_a_screen_reader() {
        let mut ui = Ui::new();
        let win = ui.window_new("Save", 240, 140).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        let field = ui.text_field_new("draft");
        ui.text_field_set_placeholder(field, "Name").unwrap();
        let check = ui.checkbox_new("Backup");
        let ok = ui.button_new("OK");
        ui.button_set_default(ok, true).unwrap();
        ui.set_enabled(ok, false).unwrap();
        for w in [field, check, ok] {
            ui.stack_add(col, w).unwrap();
        }
        ui.window_set_content(win, col).unwrap();
        let bar = ui.menu_bar_new();
        let file = ui.menu_new("File");
        let quit = ui.menu_item_new("Quit");
        ui.menu_item_set_shortcut(quit, "q").unwrap();
        ui.menu_add_separator(file).unwrap();
        ui.menu_add(file, quit).unwrap();
        ui.menu_bar_add(bar, file).unwrap();
        ui.window_set_menu_bar(win, bar).unwrap();

        let tree = ui.accessible(win).unwrap();
        let find = |h: Handle| tree.nodes.iter().find(|n| n.handle == h).unwrap();
        assert_eq!((tree.root, tree.focus), (win, field)); // the first text field has the keyboard
        assert_eq!(find(win).children, [bar, col]);
        assert_eq!((find(win).role, find(win).label.as_str()), (AccessRole::Window, "Save"));
        assert_eq!(find(field).value.as_deref(), Some("draft"));
        assert_eq!(find(field).placeholder.as_deref(), Some("Name"));
        assert_eq!(find(check).checked, Some(false));
        assert!(find(ok).is_default && !find(ok).enabled);
        assert_eq!(find(file).children, [quit]); // the dividing line is left out
        assert_eq!(find(quit).shortcut, Some('q'));
        // every node is listed once, and every child named is in the list
        assert_eq!(tree.nodes.len(), 8);
        assert!(tree.nodes.iter().flat_map(|n| &n.children).all(|c| tree.nodes.iter().any(|n| n.handle == *c)));
        // widgets sit below the menu bar
        assert!(find(field).rect.unwrap().y >= BAR_HEIGHT);

        // what assistive technology may ask for
        assert_eq!(ui.assistive_press(check).unwrap()[0].args, vec![Value::Bool(true)]);
        assert_eq!(ui.assistive_press(quit).unwrap()[0].event, "MenuItem.activated");
        assert_eq!(ui.assistive_press(ok), Err(Error::InvalidState)); // disabled
        assert_eq!(ui.assistive_press(field), Err(Error::WrongType));
        let ev = ui.assistive_set_text(field, "spoken");
        assert_eq!(ev.unwrap()[0].args, vec![Value::Str("spoken".into())]);
    }

    #[test]
    fn a_field_is_given_character_by_character() {
        let mut ui = Ui::new();
        let win = ui.window_new("Read", 240, 160).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        let field = ui.text_field_new("cafe\u{301} noir");
        let secret = ui.text_field_new("hunter2");
        ui.text_field_set_password(secret, true).unwrap();
        let area = ui.text_area_new("one two\nthree");
        for w in [field, secret, area] {
            ui.stack_add(col, w).unwrap();
        }
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        let tree = ui.accessible(win).unwrap();
        let of = |h: Handle| tree.nodes.iter().find(|n| n.handle == h).unwrap();

        // one line; the e and its accent are one character of three bytes
        let run = &of(field).runs[0];
        assert_eq!((of(field).runs.len(), run.text.as_str()), (1, "cafe\u{301} noir"));
        assert_eq!(run.chars.iter().map(|c| c.0).collect::<Vec<_>>(), [1, 1, 1, 3, 1, 1, 1, 1, 1]);
        assert_eq!(run.chars.iter().map(|c| c.0 as usize).sum::<usize>(), run.text.len());
        // each begins where the one before ends
        assert!(run.chars.windows(2).all(|p| p[0].1 + p[0].2 == p[1].1));
        // two words, the second starting after the space
        assert_eq!(run.word_starts, [0, 5]);
        // it has the keyboard, so all of it is selected: from the first character to after the ninth
        assert_eq!(of(field).selection, Some(((0, 0), (0, 9))));

        // a password is given as nothing at all
        assert!(of(secret).runs.is_empty() && of(secret).selection.is_none() && of(secret).value.is_none());

        // an area: a run to a line, the first ending in its line break
        let lines: Vec<&str> = of(area).runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(lines, ["one two\n", "three"]);
        assert_eq!(of(area).runs[0].chars.last(), Some(&(1, of(area).runs[0].rect.w, 0)));
        assert!(of(area).runs[1].rect.y > of(area).runs[0].rect.y);

        // assistive technology selects "noir", then puts the caret in the area's second line
        ui.assistive_select(field, (0, 5), (0, 9)).unwrap();
        let tree = ui.accessible(win).unwrap();
        assert_eq!(tree.nodes.iter().find(|n| n.handle == field).unwrap().selection, Some(((0, 5), (0, 9))));
        ui.assistive_select(area, (1, 2), (1, 2)).unwrap();
        let tree = ui.accessible(win).unwrap();
        assert_eq!(tree.nodes.iter().find(|n| n.handle == area).unwrap().selection, Some(((1, 2), (1, 2))));
    }
}
