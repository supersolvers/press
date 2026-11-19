// SPDX-License-Identifier: GPL-3.0-or-later
//! The list box and its two elaborations: the table, which is a list with
//! columns under a row of headings, and the tree view, whose rows hang from
//! one another and can be folded away.
//!
//! All three keep their rows as one flat list of labels; an item is known
//! by its place in that list, which never changes. What differs is which
//! items are showing, in what order, and how a row is drawn, and that is
//! all gathered into [`Ui::list_rows`]. Sorting a table changes the order
//! rows are shown in and nothing else.

use super::scroll::Bar;
#[cfg(test)]
use super::scroll::BAR;
use super::*;
use crate::text::Editor;

#[cfg(test)]
pub(super) const ROW_HEIGHT: i32 = 16;
const INDENT: i32 = 14; // per level of a tree
const MARK: i32 = 12; // room for a tree's fold mark
const ICON: i32 = 16; // room for an item's icon
const COLUMN_MIN: i32 = 24;
const GRAB: i32 = 3; // how near a column's edge counts as on it

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListMode {
    List,
    Table,
    Tree,
    /// Items shown as large pictures with their names beneath, in rows
    /// that fill the width.
    Icons,
}

/// The room each item of an icon view is given.
const CELL: (i32, i32) = (92, 56);

/// A small picture shown before an item's label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Icon {
    #[default]
    None,
    Folder,
    Document,
    Application,
}

// Twelve by twelve, drawn here. X is black; the row's own white shows through the rest.
const FOLDER: [&str; 11] = [
    " XXXX       ",
    "X    X      ",
    "XXXXXXXXXXXX",
    "X          X",
    "X          X",
    "X          X",
    "X          X",
    "X          X",
    "X          X",
    "X          X",
    "XXXXXXXXXXXX",
];
const DOCUMENT: [&str; 12] = [
    " XXXXXXX    ",
    " X     XX   ",
    " X     X X  ",
    " X     XXXX ",
    " X        X ",
    " X        X ",
    " X        X ",
    " X        X ",
    " X        X ",
    " X        X ",
    " X        X ",
    " XXXXXXXXXX ",
];
const APPLICATION: [&str; 12] = [
    "     XX     ",
    "    X  X    ",
    "   X    X   ",
    "  X      X  ",
    " X        X ",
    "X    XX    X",
    "X    XX    X",
    " X        X ",
    "  X      X  ",
    "   X    X   ",
    "    X  X    ",
    "     XX     ",
];

/// What is kept for each item besides its label.
/// A picture of the program's own for an item: rows of pixels.
type Art = std::rc::Rc<Vec<Vec<u8>>>;

#[derive(Debug, Clone, Default)]
pub(super) struct ItemMeta {
    /// In a tree: the item this one hangs from.
    parent: Option<usize>,
    /// In a tree: whether what hangs from it is showing.
    open: bool,
    /// Whether it is one of the selected items.
    picked: bool,
    icon: Icon,
    /// A picture of the program's own, in place of a stock icon: rows of
    /// pixels, each 0 for see-through, 1 for black, 2 for white.
    art: Option<std::rc::Rc<Vec<Vec<u8>>>>,
    /// A picture in colour, in place of either.
    image: Option<std::rc::Rc<Canvas>>,
    /// The file the item stands for, if it stands for one.
    file: Option<String>,
    /// What the item carries when dragged out of its list: each thing's
    /// kind and the thing itself.
    data: Vec<(String, std::rc::Rc<Vec<u8>>)>,
    /// In an icon view: where it has been put, if it has been put
    /// somewhere rather than left in line. From the top left of all the
    /// icons, which is not the top left of the view once it is scrolled.
    at: Option<(i32, i32)>,
}

impl ItemMeta {
    fn has_picture(&self) -> bool {
        self.icon != Icon::None || self.art.is_some() || self.image.is_some()
    }
}

/// The largest picture that fits before a label.
const ART_MAX: usize = 14;

/// A cell being edited in place.
pub(super) struct ListEdit {
    pub list: Handle,
    item: usize,
    column: usize,
    edit: Editor,
}

/// Rows on the move: the list they were picked up in, the row that was
/// pressed, and the list and the place among its rows where they would
/// land if let go now.
#[derive(Clone, Copy)]
pub(super) struct RowDrag {
    pub from: Handle,
    item: usize,
    pub to: Handle,
    gap: usize,
    /// In a tree: the item they would be dropped onto, to hang from it,
    /// rather than into the gap.
    onto: Option<usize>,
    /// Whether the pointer is somewhere they could land. It is not while
    /// the drag is outside every window that would have them.
    live: bool,
    /// Icons being put somewhere in their own view: where the pointer is,
    /// in the window, and where in the pressed icon's cell it took hold.
    point: (i32, i32),
    grip: (i32, i32),
}

/// A row on its way into a list: what it says and shows, and which of the
/// rows arriving with it (by its place among them) it hangs from.
#[derive(Clone)]
struct Arrival {
    label: String,
    icon: Icon,
    art: Option<std::rc::Rc<Vec<Vec<u8>>>>,
    open: bool,
    parent: Option<usize>,
}

/// The first line of rows written out for another program.
const PAYLOAD_HEAD: &str = "press-rows 1";
/// What separates the fields of a row in that form.
const FIELD: char = '\u{1f}';
/// The most rows that will be taken in from another program at once.
const PAYLOAD_ROWS: usize = 10_000;

/// How far a pressed row must be pulled before it comes loose to be moved.
const PULL: i32 = 4;

/// A column whose edge is being dragged: the table, the column, and where
/// the pointer and the width were when it was taken.
pub(super) type ColumnDrag = (Handle, usize, i32, i32);

/// One row as shown: which item it is, how far in it sits, and whether it
/// carries a fold mark (and if so whether it is open).
struct Row {
    item: usize,
    depth: i32,
    fold: Option<bool>,
}

/// How a click or key changes the selection.
#[derive(Clone, Copy, PartialEq)]
enum Pick {
    /// This item and no other.
    Only,
    /// Add it or take it away, leaving the rest (Control).
    Toggle,
    /// Everything from where the selection was last started to here (Shift).
    Extend,
}

/// Order two cells: by the number they start with if both start with one,
/// otherwise as text without regard to case.
fn compare_cells(a: &str, b: &str) -> std::cmp::Ordering {
    let number = |s: &str| {
        let digits: String = s.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        digits.parse::<f64>().ok()
    };
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

fn cell(text: &str, column: usize) -> &str {
    text.split('\t').nth(column).unwrap_or("")
}

impl Ui {
    fn list_new(&mut self, mode: ListMode) -> Handle {
        self.add(Kind::List {
            items: Vec::new(),
            selected: None,
            scroll: 0,
            rows: 5,
            min_width: 140,
            mode,
            columns: Vec::new(),
            meta: Vec::new(),
            multiple: false,
            editable: false,
            sortable: false,
            sort: None,
            anchor: None,
            order: Vec::new(),
            reorderable: false,
            accepts: false,
        })
    }

    pub fn list_box_new(&mut self) -> Handle {
        self.list_new(ListMode::List)
    }

    /// A list box with columns. A row's cells are separated by tabs.
    pub fn table_new(&mut self) -> Handle {
        self.list_new(ListMode::Table)
    }

    /// A list box whose rows can hang from one another.
    pub fn icon_view_new(&mut self) -> Handle {
        let h = self.list_new(ListMode::Icons);
        if let Some(Kind::List { rows, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *rows = 3; // rows of icons, which are much taller than rows of text
        }
        h
    }

    /// Where an icon is among all the icons of its view: where it was put,
    /// or failing that its place in line. `place` is its position in order.
    fn icon_cell(&self, h: Handle, place: usize, item: usize) -> (i32, i32) {
        let across = self.icons_across(h);
        let put = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { meta, .. }) => meta.get(item).and_then(|m| m.at),
            _ => None,
        };
        put.unwrap_or(((place as i32 % across) * CELL.0, (place as i32 / across) * CELL.1))
    }

    /// Put an icon at a place in its view, measured from the top left of
    /// all the icons, or with a negative place back in line with the rest.
    pub fn icon_view_set_position(&mut self, h: Handle, index: i32, x: i32, y: i32) -> Result<()> {
        if x > 100_000 || y > 100_000 {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, mode: ListMode::Icons, .. } => {
                meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?.at = (x >= 0 && y >= 0).then_some((x, y));
            }
            _ => return Err(Error::WrongType),
        }
        self.list_settle(h);
        Ok(())
    }

    /// Icons let go in their own view: put them where they were dropped,
    /// keeping them as they were to one another, and say where each now is.
    fn icons_place(&mut self, drag: RowDrag) -> Vec<Emitted> {
        let h = drag.from;
        let Some((area, ..)) = self.list_shape(h) else { return Vec::new() };
        let rows: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        let scroll = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { scroll, .. }) => *scroll,
            _ => 0,
        };
        let cell = |ui: &Ui, item: usize| rows.iter().position(|r| *r == item).map(|place| ui.icon_cell(h, place, item));
        let Some(held) = cell(self, drag.item) else { return Vec::new() };
        // how far the pressed icon goes is how far they all go
        let to = (drag.point.0 - area.x - drag.grip.0, drag.point.1 - area.y + scroll * CELL.1 - drag.grip.1);
        let moving: Vec<(usize, (i32, i32))> = self.moving_rows(h, drag.item).into_iter().filter_map(|item| Some((item, cell(self, item)?))).collect();
        let mut out = Vec::new();
        for (item, was) in moving {
            let now = ((was.0 + to.0 - held.0).max(0), (was.1 + to.1 - held.1).max(0));
            if let Some(Kind::List { meta, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
                meta[item].at = Some(now);
            }
            out.push(Emitted { target: h, event: "IconView.placed", args: vec![Value::I32(item as i32), Value::I32(now.0), Value::I32(now.1)] });
        }
        self.list_settle(h);
        out
    }

    /// How many icons an icon view shows across.
    fn icons_across(&self, h: Handle) -> i32 {
        let Some(node) = self.nodes.get(h) else { return 1 };
        ((Self::area_bar_rect(node.rect, self.look.m.scroll_bar).x - node.rect.x - 2) / CELL.0).max(1)
    }

    pub fn tree_view_new(&mut self) -> Handle {
        self.list_new(ListMode::Tree)
    }

    /// Append a row. In a tree it goes at the top level.
    pub fn list_box_add(&mut self, h: Handle, label: &str) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { items, meta, order, .. } => {
                order.push(items.len());
                items.push(label.to_string());
                meta.push(ItemMeta::default());
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn list_box_clear(&mut self, h: Handle) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { items, meta, selected, scroll, anchor, order, .. } => {
                items.clear();
                meta.clear();
                order.clear();
                *selected = None;
                *anchor = None;
                *scroll = 0;
            }
            _ => return Err(Error::WrongType),
        }
        self.list_edit = self.list_edit.take().filter(|e| e.list != h);
        Ok(())
    }

    /// The index of the current item: the one last selected, or -1 if none is.
    pub fn list_box_selected(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::List { selected, .. } => Ok(selected.map_or(-1, |s| s as i32)),
            _ => Err(Error::WrongType),
        }
    }

    /// Select one item from the program and no other, or none with -1, and
    /// bring it into view; in a tree, whatever it hangs from is unfolded.
    /// Nothing is emitted.
    pub fn list_box_set_selected(&mut self, h: Handle, index: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { items, selected, meta, anchor, .. } => {
                if index < -1 || index >= items.len() as i32 {
                    return Err(Error::InvalidArgument);
                }
                *selected = (index >= 0).then_some(index as usize);
                *anchor = *selected;
                for (i, m) in meta.iter_mut().enumerate() {
                    m.picked = Some(i) == *selected;
                }
                let mut up = selected.and_then(|s| meta[s].parent);
                while let Some(parent) = up {
                    meta[parent].open = true;
                    up = meta[parent].parent;
                }
            }
            _ => return Err(Error::WrongType),
        }
        self.list_reveal(h);
        Ok(())
    }

    /// Whether an item is selected. With multiple selection several may be.
    pub fn list_box_is_selected(&self, h: Handle, index: i32) -> Result<bool> {
        match &self.node(h)?.kind {
            Kind::List { meta, .. } => meta.get(index.max(0) as usize).filter(|_| index >= 0).map(|m| m.picked).ok_or(Error::InvalidArgument),
            _ => Err(Error::WrongType),
        }
    }

    /// Select or unselect one item from the program, leaving the others as
    /// they are where the list allows several. Nothing is emitted.
    pub fn list_box_set_item_selected(&mut self, h: Handle, index: i32, on: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, selected, multiple, .. } => {
                let at = index.max(0) as usize;
                if index < 0 || at >= meta.len() {
                    return Err(Error::InvalidArgument);
                }
                if !*multiple {
                    meta.iter_mut().for_each(|m| m.picked = false);
                }
                meta[at].picked = on;
                if on {
                    *selected = Some(at);
                } else if *selected == Some(at) {
                    *selected = meta.iter().position(|m| m.picked);
                }
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Set one of a list's switches.
    fn list_flag(&mut self, h: Handle, pick: impl FnOnce(&mut Kind) -> Option<&mut bool>, on: bool) -> Result<()> {
        *pick(&mut self.node_mut(h)?.kind).ok_or(Error::WrongType)? = on;
        Ok(())
    }

    /// Let several items be selected at once: Control adds and removes,
    /// Shift extends. Turning it off keeps only the current item selected.
    pub fn list_box_set_multiple(&mut self, h: Handle, on: bool) -> Result<()> {
        self.list_flag(h, |k| if let Kind::List { multiple, .. } = k { Some(multiple) } else { None }, on)?;
        if let Kind::List { meta, selected, .. } = &mut self.node_mut(h)?.kind {
            if !on {
                for (i, m) in meta.iter_mut().enumerate() {
                    m.picked = Some(i) == *selected;
                }
            }
        }
        Ok(())
    }

    /// Let the user edit an item's text in place, with Return.
    pub fn list_box_set_editable(&mut self, h: Handle, on: bool) -> Result<()> {
        self.list_flag(h, |k| if let Kind::List { editable, .. } = k { Some(editable) } else { None }, on)
    }

    /// Let the user drag rows to a new place. In a tree a row can also be
    /// dropped onto another to hang from it.
    pub fn list_box_set_reorderable(&mut self, h: Handle, on: bool) -> Result<()> {
        self.list_flag(h, |k| if let Kind::List { reorderable, .. } = k { Some(reorderable) } else { None }, on)
    }

    /// Let rows dragged out of another list be dropped here. The rows leave
    /// that list and join this one.
    pub fn list_box_set_accepts_rows(&mut self, h: Handle, on: bool) -> Result<()> {
        self.list_flag(h, |k| if let Kind::List { accepts, .. } = k { Some(accepts) } else { None }, on)
    }

    /// Take one item out, closing the gap in every record that goes by index.
    fn list_take(&mut self, h: Handle, index: usize) -> Option<(String, ItemMeta)> {
        self.list_edit = self.list_edit.take().filter(|e| e.list != h);
        let Some(Kind::List { items, meta, order, selected, anchor, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) else { return None };
        if index >= items.len() {
            return None;
        }
        let taken = (items.remove(index), meta.remove(index));
        order.retain(|o| *o != index);
        order.iter_mut().filter(|o| **o > index).for_each(|o| *o -= 1);
        let shift = |slot: Option<usize>| match slot {
            Some(s) if s == index => None,
            Some(s) if s > index => Some(s - 1),
            other => other,
        };
        *selected = shift(*selected);
        *anchor = shift(*anchor);
        for m in meta.iter_mut() {
            m.parent = match m.parent {
                Some(p) if p == index => None,
                Some(p) if p > index => Some(p - 1),
                other => other,
            };
        }
        if selected.is_none() {
            *selected = meta.iter().position(|m| m.picked);
        }
        Some(taken)
    }

    /// Remove an item from the program. Items after it move down by one
    /// index. In a tree, whatever hangs from it goes too. Nothing is emitted.
    pub fn list_box_remove(&mut self, h: Handle, index: i32) -> Result<()> {
        let doomed = match &self.node(h)?.kind {
            Kind::List { meta, .. } => {
                if index < 0 || index as usize >= meta.len() {
                    return Err(Error::InvalidArgument);
                }
                // the item and everything beneath it
                let mut doomed = vec![index as usize];
                let mut at = 0;
                while at < doomed.len() {
                    let parent = doomed[at];
                    doomed.extend(meta.iter().enumerate().filter(|(_, m)| m.parent == Some(parent)).map(|(i, _)| i));
                    at += 1;
                }
                doomed
            }
            _ => return Err(Error::WrongType),
        };
        // highest first, so that each index is still right when its turn comes
        let mut doomed = doomed;
        doomed.sort_unstable_by(|a, b| b.cmp(a));
        for i in doomed {
            self.list_take(h, i);
        }
        self.list_settle(h);
        Ok(())
    }

    /// The item a tree's item hangs from, or -1 at the top level.
    pub fn tree_view_parent_of(&self, h: Handle, index: i32) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::List { mode: ListMode::Tree, meta, .. } => {
                meta.get(index.max(0) as usize).filter(|_| index >= 0).map(|m| m.parent.map_or(-1, |p| p as i32)).ok_or(Error::InvalidArgument)
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Move items of a tree: into the gap above a row, to share what that
    /// row hangs from, or onto a row, to hang from it. An item takes
    /// everything beneath it along. Nothing can be moved into itself.
    fn tree_move(&mut self, h: Handle, moving: &[usize], gap: usize, onto: Option<usize>) -> Vec<Emitted> {
        let rows: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        let Some(Kind::List { meta, order, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) else { return Vec::new() };
        let under = |meta: &[ItemMeta], mut item: usize, root: usize| loop {
            match meta[item].parent {
                Some(p) if p == root => return true,
                Some(p) => item = p,
                None => return false,
            }
        };
        // of the rows on the move, only the ones not already travelling with another
        let roots: Vec<usize> = moving.iter().copied().filter(|m| !moving.iter().any(|other| under(meta, *m, *other))).collect();
        let (parent, before) = match onto {
            Some(p) => (Some(p), None),
            None => match rows.get(gap) {
                Some(below) => (meta[*below].parent, Some(*below)),
                None => (None, None),
            },
        };
        let into_itself = parent.is_some_and(|p| roots.iter().any(|r| *r == p || under(meta, p, *r)));
        if into_itself || before.is_some_and(|b| roots.contains(&b)) {
            return Vec::new();
        }
        let was: (Vec<usize>, Vec<Option<usize>>) = (order.clone(), meta.iter().map(|m| m.parent).collect());
        order.retain(|i| !roots.contains(i));
        let at = before.and_then(|b| order.iter().position(|i| *i == b)).unwrap_or(order.len());
        order.splice(at..at, roots.iter().copied());
        for root in &roots {
            meta[*root].parent = parent;
        }
        if let Some(p) = parent {
            meta[p].open = true; // so that what was dropped can be seen
        }
        if was == (order.clone(), meta.iter().map(|m| m.parent).collect()) {
            return Vec::new();
        }
        self.list_settle(h);
        self.list_reveal(h);
        let shown: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        roots
            .iter()
            .map(|item| {
                let place = shown.iter().position(|i| i == item).map_or(-1, |p| p as i32);
                Emitted { target: h, event: "ListBox.moved", args: vec![Value::I32(*item as i32), Value::I32(place)] }
            })
            .collect()
    }

    /// Where rows being dragged would land if let go at a point in a window:
    /// the list, the gap among its rows, and in a tree the item they would
    /// hang from if dropped onto one. `None` if nothing there would have them.
    fn row_target(&mut self, window: Handle, x: i32, y: i32, from: Option<Handle>) -> Option<(Handle, usize, Option<usize>)> {
        // rows that only leave their list, to be dropped as files, have no place within it
        let from = from.filter(|f| self.list_reorderable(*f));
        let to = match self.hit(window, x, y) {
            Some(over) if Some(over) == from => over,
            // any list or tree that takes rows
            Some(over) if matches!(self.nodes.get(over).map(|n| &n.kind), Some(Kind::List { accepts: true, .. })) => over,
            // anywhere else in their own window, they are still being moved within their own list
            // icons are put where they are let go, which has to be in their view
            _ if from.is_some_and(|f| self.list_is(f, ListMode::Icons)) => return None,
            // anywhere else in their own window, they are still being moved within their own list
            _ => from.filter(|f| self.window_of(*f) == Some(window))?,
        };
        if self.list_is(to, ListMode::Icons) {
            // nothing lands between icons: their own go where they are dropped, and others' are not taken
            return (Some(to) == from).then_some((to, 0, None));
        }
        let tree = self.list_is(to, ListMode::Tree);
        let (area, fit, count) = self.list_shape(to)?;
        let rows: Vec<usize> = self.list_rows(to).iter().map(|r| r.item).collect();
        let Some(Kind::List { scroll, .. }) = self.nodes.get_mut(to).map(|n| &mut n.kind) else { return None };
        // past the top or bottom the list scrolls to bring more into reach
        if y < area.y {
            *scroll = (*scroll - 1).max(0);
        } else if y >= area.y + fit * self.look.m.row_height {
            *scroll = (*scroll + 1).min((count - fit).max(0));
        }
        let down = *scroll * self.look.m.row_height + y - area.y;
        // in a tree, the middle of a row means onto it; its edges mean the gaps beside it
        let onto = (tree && (self.look.m.row_height / 4..self.look.m.row_height * 3 / 4).contains(&down.rem_euclid(self.look.m.row_height)))
            .then(|| rows.get(down.div_euclid(self.look.m.row_height).max(0) as usize).copied())
            .flatten()
            .filter(|_| down >= 0);
        let gap = (down + self.look.m.row_height / 2).div_euclid(self.look.m.row_height).clamp(0, count) as usize;
        Some((to, gap, onto))
    }

    /// Whether rows have come loose and are being dragged.
    pub fn row_dragging(&self) -> bool {
        self.row_drag.is_some()
    }

    /// The text of the rows being dragged, a line each, for dropping
    /// somewhere that takes text and not rows.
    pub fn row_drag_text(&self) -> String {
        let Some(drag) = self.row_drag else { return String::new() };
        let Some(Kind::List { items, .. }) = self.nodes.get(drag.from).map(|n| &n.kind) else { return String::new() };
        self.moving_rows(drag.from, drag.item).iter().filter_map(|i| items.get(*i)).cloned().collect::<Vec<_>>().join("\n")
    }

    /// The rows being dragged are over a point in a window, or (`None`) over
    /// none of this program's windows. A display backend calls this once the
    /// desktop has taken the drag over, which is how rows cross from one
    /// window to another. Returns whether they could be dropped there.
    pub fn row_drag_over(&mut self, window: Handle, at: Option<(i32, i32)>) -> bool {
        let Some(mut drag) = self.row_drag else { return false };
        let target = at.and_then(|(x, y)| {
            let size = self.nodes.get(window).map(|n| &n.kind).and_then(|k| if let Kind::Window { width, height, .. } = k { Some((*width, *height)) } else { None })?;
            self.arrange(window, Rect::new(0, 0, size.0, size.1));
            self.row_target(window, x, y, Some(drag.from))
        });
        drag.live = target.is_some();
        if let Some((to, gap, onto)) = target {
            (drag.to, drag.gap, drag.onto) = (to, gap, onto);
        }
        if let Some(at) = at {
            drag.point = at;
        }
        self.row_drag = Some(drag);
        self.dirty = true;
        drag.live
    }

    /// The drag is over: put the rows where they were dropped, or with
    /// `dropped` false leave everything as it was.
    pub fn row_drag_finish(&mut self, dropped: bool) -> Vec<Emitted> {
        self.list_drag = None;
        self.dirty = true;
        if !dropped {
            self.row_drag = None;
            self.list_collapse = None;
            return Vec::new();
        }
        self.row_drop()
    }

    /// The rows that move together when one is dragged: every selected row
    /// if the pressed one is among several, otherwise just that one. In the
    /// order they are showing.
    fn moving_rows(&self, h: Handle, pressed: usize) -> Vec<usize> {
        let Some(Kind::List { meta, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return Vec::new() };
        if !meta.get(pressed).is_some_and(|m| m.picked) {
            return vec![pressed];
        }
        self.list_rows(h).iter().map(|r| r.item).filter(|i| meta[*i].picked).collect()
    }

    /// Move rows to a gap between the rows of their own list.
    fn rows_reorder(&mut self, h: Handle, moving: &[usize], gap: usize) -> Vec<Emitted> {
        let shown: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        // the gap counts rows that are about to be lifted out; take them off
        let before = shown.iter().take(gap).filter(|i| moving.contains(i)).count();
        let mut order: Vec<usize> = shown.iter().copied().filter(|i| !moving.contains(i)).collect();
        let at = (gap - before).min(order.len());
        order.splice(at..at, moving.iter().copied());
        if order == shown {
            return Vec::new();
        }
        if let Some(Kind::List { order: stored, sort, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *stored = order;
            *sort = None;
        }
        self.list_reveal(h);
        moving.iter().enumerate().map(|(k, item)| Emitted { target: h, event: "ListBox.moved", args: vec![Value::I32(*item as i32), Value::I32((at + k) as i32)] }).collect()
    }

    /// Move rows out of one list and into another: into a gap between its
    /// rows or, where it is a tree, onto one of them. What leaves a tree
    /// takes its whole branch. Arriving in a tree the branch keeps its shape;
    /// arriving in a plain list it is laid out flat, one row after another.
    fn rows_transfer(&mut self, from: Handle, to: Handle, moving: &[usize], gap: usize, onto: Option<usize>) -> Vec<Emitted> {
        let mut out = Vec::new();
        let (from_tree, to_tree) = (self.list_is(from, ListMode::Tree), self.list_is(to, ListMode::Tree));
        // Everything that travels, parents before children, each with the
        // index of its parent if that is travelling too.
        let travelling: Vec<(usize, Option<usize>)> = {
            let Some(Kind::List { meta, order, .. }) = self.nodes.get(from).map(|n| &n.kind) else { return out };
            let under = |mut item: usize, root: usize| loop {
                match meta[item].parent {
                    Some(p) if p == root => return true,
                    Some(p) => item = p,
                    None => return false,
                }
            };
            let roots: Vec<usize> = moving.iter().copied().filter(|m| !from_tree || !moving.iter().any(|other| under(*m, *other))).collect();
            let mut list = Vec::new();
            let mut pending: Vec<(usize, Option<usize>)> = roots.into_iter().rev().map(|r| (r, None)).collect();
            while let Some((item, parent)) = pending.pop() {
                list.push((item, parent));
                if from_tree {
                    pending.extend(order.iter().rev().filter(|c| meta.get(**c).is_some_and(|m| m.parent == Some(item))).map(|c| (*c, Some(item))));
                }
            }
            list
        };
        // Where they are going: what they will hang from, and what they go in front of.
        let (parent, before, shown) = self.landing(to, gap, onto);
        // Lift them out, highest index first so each index holds until its turn.
        let mut by_index: Vec<usize> = travelling.iter().map(|t| t.0).collect();
        by_index.sort_unstable_by(|a, b| b.cmp(a));
        let mut lifted: Vec<(usize, String, ItemMeta)> = Vec::new();
        for index in by_index {
            if let Some((label, meta)) = self.list_take(from, index) {
                out.push(Emitted { target: from, event: "ListBox.removed", args: vec![Value::I32(index as i32)] });
                lifted.push((index, label, meta));
            }
        }
        let _ = to_tree;
        // hand them over in travelling order, each knowing which of the others it hangs from
        let arrivals: Vec<Arrival> = travelling
            .iter()
            .filter_map(|(old, old_parent)| {
                let (_, label, was) = lifted.iter().find(|(index, ..)| index == old)?.clone();
                let parent = old_parent.and_then(|p| travelling.iter().position(|(o, _)| *o == p));
                Some(Arrival { label, icon: was.icon, art: was.art, open: was.open, parent })
            })
            .collect();
        self.list_settle(from);
        out.extend(self.rows_arrive(to, arrivals, parent, before, &shown));
        out
    }

    /// Where rows dropped into a list at a gap, or onto an item of a tree,
    /// are to go: what they will hang from, and the item they go in front of.
    fn landing(&self, to: Handle, gap: usize, onto: Option<usize>) -> (Option<usize>, Option<usize>, Vec<usize>) {
        let shown: Vec<usize> = self.list_rows(to).iter().map(|r| r.item).collect();
        let Some(Kind::List { meta, mode, .. }) = self.nodes.get(to).map(|n| &n.kind) else { return (None, None, shown) };
        let (parent, before) = match (*mode == ListMode::Tree, onto, shown.get(gap)) {
            (true, Some(p), _) if p < meta.len() => (Some(p), None),
            (true, _, Some(below)) => (meta[*below].parent, Some(*below)),
            (false, _, below) => (None, below.copied()),
            _ => (None, None),
        };
        (parent, before, shown)
    }

    /// Put rows into a list: new items, in front of `before` and hanging
    /// from `parent`. In a tree a branch keeps its shape; in a plain list it
    /// is laid out flat. What was dragged arrives selected, with the keyboard.
    fn rows_arrive(&mut self, to: Handle, arrivals: Vec<Arrival>, parent: Option<usize>, before: Option<usize>, shown: &[usize]) -> Vec<Emitted> {
        let to_tree = self.list_is(to, ListMode::Tree);
        let mut arrived = Vec::new();
        if let Some(Kind::List { items, meta, order, sort, selected, anchor, .. }) = self.nodes.get_mut(to).map(|n| &mut n.kind) {
            if !to_tree {
                // a plain list shows its rows in a fixed order from now on, so the newcomers stay put
                *order = shown.to_vec();
                *sort = None;
            }
            meta.iter_mut().for_each(|m| m.picked = false);
            let at = before.and_then(|b| order.iter().position(|i| *i == b)).unwrap_or(order.len());
            let first = items.len(); // arrival k becomes item first + k
            let mut placed = 0;
            for (k, arrival) in arrivals.into_iter().enumerate() {
                let index = first + k;
                items.push(arrival.label);
                // the head of a branch hangs where it was dropped; the rest from what they hung from before
                let head = arrival.parent.is_none();
                let hangs_from = match (to_tree, head) {
                    (false, _) => None,
                    (true, true) => parent,
                    (true, false) => arrival.parent.filter(|p| *p < k).map(|p| first + p),
                };
                // it keeps its picture; what was dragged arrives selected
                meta.push(ItemMeta { parent: hangs_from, open: to_tree && arrival.open, picked: head || !to_tree, icon: arrival.icon, image: None, art: arrival.art, file: None, data: Vec::new(), at: None });
                if head || !to_tree {
                    order.insert(at + placed, index);
                    placed += 1;
                    *selected = Some(index);
                    *anchor = Some(index);
                } else {
                    order.push(index); // among its brothers and sisters, in the order they came
                }
                arrived.push(index);
            }
            if let Some(p) = parent.filter(|_| !arrived.is_empty()) {
                meta[p].open = true; // so that what was dropped can be seen
            }
        }
        self.list_settle(to);
        self.list_reveal(to);
        let places: Vec<usize> = self.list_rows(to).iter().map(|r| r.item).collect();
        let out = arrived
            .into_iter()
            .map(|index| {
                let place = places.iter().position(|i| *i == index).map_or(-1, |p| p as i32);
                Emitted { target: to, event: "ListBox.received", args: vec![Value::I32(index as i32), Value::I32(place)] }
            })
            .collect();
        // the keyboard goes with the rows
        if let Some(window) = self.window_of(to) {
            if self.focusable(to) {
                self.set_focus(window, Some(to), false);
            }
        }
        out
    }

    // ------------------------------------------- rows between programs
    //
    // Rows dragged to a list in another program travel as text: a line to
    // say what it is, then a line for each row. Each program's engine knows
    // only its own half of the drag.

    /// What is travelling in the present drag, parents before children,
    /// each with how deep in its branch it sits.
    fn travelling(&self) -> Vec<(usize, i32)> {
        let Some(drag) = self.row_drag else { return Vec::new() };
        let Some(Kind::List { meta, order, mode, .. }) = self.nodes.get(drag.from).map(|n| &n.kind) else { return Vec::new() };
        let moving = self.moving_rows(drag.from, drag.item);
        if *mode != ListMode::Tree {
            return moving.into_iter().map(|i| (i, 0)).collect();
        }
        let under = |mut item: usize, root: usize| loop {
            match meta[item].parent {
                Some(p) if p == root => return true,
                Some(p) => item = p,
                None => return false,
            }
        };
        let mut out = Vec::new();
        let mut pending: Vec<(usize, i32)> = moving.iter().copied().filter(|m| !moving.iter().any(|o| under(*m, *o))).rev().map(|r| (r, 0)).collect();
        while let Some((item, depth)) = pending.pop() {
            out.push((item, depth));
            pending.extend(order.iter().rev().filter(|c| meta.get(**c).is_some_and(|m| m.parent == Some(item))).map(|c| (*c, depth + 1)));
        }
        out
    }

    /// The rows being dragged, written out for another program to take in.
    pub fn row_drag_payload(&self) -> String {
        let Some(drag) = self.row_drag else { return String::new() };
        let Some(Kind::List { items, meta, .. }) = self.nodes.get(drag.from).map(|n| &n.kind) else { return String::new() };
        let mut out = String::from(PAYLOAD_HEAD);
        for (item, depth) in self.travelling() {
            let m = &meta[item];
            let art = m.art.as_ref().map(|rows| rows.iter().map(|r| r.iter().map(|p| [' ', 'X', '.'][*p as usize % 3]).collect::<String>()).collect::<Vec<_>>().join("/")).unwrap_or_default();
            // a label may hold anything; the separators are escaped out of it
            let label = items[item].replace('\\', "\\\\").replace('\n', "\\n").replace(FIELD, "\\u");
            out.push_str(&format!("\n{depth}{FIELD}{}{FIELD}{}{FIELD}{art}{FIELD}{label}", m.icon as u8, m.open as u8));
        }
        out
    }

    /// The rows being dragged were taken by another program: they have left
    /// their list for good.
    pub fn row_drag_taken(&mut self) -> Vec<Emitted> {
        let Some(drag) = self.row_drag else { return self.row_drag_finish(false) };
        let mut leaving: Vec<usize> = self.travelling().into_iter().map(|(i, _)| i).collect();
        self.row_drag_finish(false);
        leaving.sort_unstable_by(|a, b| b.cmp(a)); // highest first, so each index holds until its turn
        let mut out = Vec::new();
        for index in leaving {
            if self.list_take(drag.from, index).is_some() {
                out.push(Emitted { target: drag.from, event: "ListBox.removed", args: vec![Value::I32(index as i32)] });
            }
        }
        self.list_settle(drag.from);
        out
    }

    /// Rows from another program are being dragged over a point in a
    /// window, or (`None`) have gone. Returns whether they could be dropped
    /// there, and shows where they would land.
    pub fn foreign_rows_over(&mut self, window: Handle, at: Option<(i32, i32)>) -> bool {
        let target = at.and_then(|(x, y)| {
            let size = self.nodes.get(window).map(|n| &n.kind).and_then(|k| if let Kind::Window { width, height, .. } = k { Some((*width, *height)) } else { None })?;
            self.arrange(window, Rect::new(0, 0, size.0, size.1));
            self.row_target(window, x, y, None)
        });
        if self.foreign_drop != target {
            self.foreign_drop = target;
            self.dirty = true;
        }
        target.is_some()
    }

    /// Rows from another program were dropped where the mark last showed.
    /// `payload` is what that program wrote; it is not trusted, and anything
    /// in it that does not make sense is left out.
    pub fn foreign_rows_drop(&mut self, payload: &str) -> Vec<Emitted> {
        let Some((to, gap, onto)) = self.foreign_drop.take() else { return Vec::new() };
        self.dirty = true;
        let mut lines = payload.split('\n');
        if lines.next() != Some(PAYLOAD_HEAD) || self.nodes.get(to).is_none() {
            return Vec::new();
        }
        let mut arrivals: Vec<Arrival> = Vec::new();
        let mut depths: Vec<i32> = Vec::new();
        for line in lines.take(PAYLOAD_ROWS) {
            let fields: Vec<&str> = line.splitn(5, FIELD).collect();
            let [depth, icon, open, art, label] = fields[..] else { continue };
            let depth = depth.parse::<i32>().unwrap_or(0).clamp(0, depths.last().map_or(0, |d| d + 1));
            let icon = match icon {
                "1" => Icon::Folder,
                "2" => Icon::Document,
                "3" => Icon::Application,
                _ => Icon::None,
            };
            let rows: Vec<Vec<u8>> = art.split('/').filter(|r| !r.is_empty()).map(|r| r.chars().map(|c| match c { 'X' => 1, '.' => 2, _ => 0 }).collect()).collect();
            let art = (!rows.is_empty() && rows.len() <= ART_MAX && rows.iter().all(|r| r.len() <= ART_MAX)).then(|| std::rc::Rc::new(rows));
            // undo the escaping: a backslash and the character after it are one character
            let mut text = String::new();
            let mut chars = label.chars();
            while let Some(c) = chars.next() {
                if c != '\\' {
                    text.push(c);
                    continue;
                }
                match chars.next() {
                    Some('n') => text.push('\n'),
                    Some('u') => text.push(FIELD),
                    Some(other) => text.push(other),
                    None => break,
                }
            }
            // it hangs from the nearest row before it that is one step less deep
            let parent = (depth > 0).then(|| depths.iter().rposition(|d| *d == depth - 1)).flatten();
            arrivals.push(Arrival { label: text, icon, art, open: open == "1", parent });
            depths.push(depth);
        }
        let (parent, before, shown) = self.landing(to, gap, onto);
        self.rows_arrive(to, arrivals, parent, before, &shown)
    }

    /// Where an item is among the rows as shown, counting from 0, or -1 if
    /// it is not showing.
    pub fn list_box_position_of(&self, h: Handle, index: i32) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::List { items, .. } if index < 0 || index >= items.len() as i32 => Err(Error::InvalidArgument),
            Kind::List { .. } => Ok(self.list_rows(h).iter().position(|r| r.item == index as usize).map_or(-1, |p| p as i32)),
            _ => Err(Error::WrongType),
        }
    }

    /// Move an item to a place among the rows from the program. A sorted
    /// table stops being sorted, keeping the order it was showing.
    pub fn list_box_move_item(&mut self, h: Handle, index: i32, position: i32) -> Result<()> {
        match &self.node(h)?.kind {
            Kind::List { mode: ListMode::Tree, .. } => return Err(Error::InvalidState),
            Kind::List { items, .. } if index < 0 || index >= items.len() as i32 || position < 0 || position >= items.len() as i32 => {
                return Err(Error::InvalidArgument);
            }
            Kind::List { .. } => {}
            _ => return Err(Error::WrongType),
        }
        self.list_move(h, index as usize, position as usize);
        Ok(())
    }

    /// Put an item at a place among the rows. Returns whether it moved.
    fn list_move(&mut self, h: Handle, item: usize, position: usize) -> bool {
        // start from the order as shown, so that moving a row of a sorted table makes sense
        let mut shown: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        let Some(from) = shown.iter().position(|i| *i == item) else { return false };
        let to = position.min(shown.len() - 1);
        if from == to {
            return false;
        }
        shown.remove(from);
        shown.insert(to, item);
        if let Some(Kind::List { order, sort, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *order = shown;
            *sort = None;
        }
        self.list_reveal(h);
        true
    }

    /// Let the user sort a table by clicking a column's heading.
    pub fn table_set_sortable(&mut self, h: Handle, on: bool) -> Result<()> {
        self.list_flag(h, |k| if let Kind::List { mode: ListMode::Table, sortable, .. } = k { Some(sortable) } else { None }, on)
    }

    /// Sort a table by a column from the program, or put the rows back in
    /// the order they were added with -1.
    pub fn table_sort_by(&mut self, h: Handle, column: i32, ascending: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { mode: ListMode::Table, columns, sort, .. } => {
                if column < -1 || column >= columns.len() as i32 {
                    return Err(Error::InvalidArgument);
                }
                *sort = (column >= 0).then_some((column as usize, ascending));
            }
            _ => return Err(Error::WrongType),
        }
        self.list_reveal(h);
        Ok(())
    }

    /// Give an item a small picture before its label.
    pub fn list_box_set_icon(&mut self, h: Handle, index: i32, icon: Icon) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, .. } => {
                meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?.icon = icon;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Say that an item stands for a file (or, with an empty path, that it
    /// no longer does). Such an item can be dragged out of its list, and is
    /// offered to wherever it is dropped as that file.
    pub fn list_box_set_file(&mut self, h: Handle, index: i32, path: &str) -> Result<()> {
        if path.contains('\n') || (!path.is_empty() && !path.starts_with('/')) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, .. } => {
                meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?.file = (!path.is_empty()).then(|| path.to_string());
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Give an item something to carry when it is dragged out of its list.
    pub fn list_box_set_drag_data(&mut self, h: Handle, index: i32, kind: &str, data: &[u8]) -> Result<()> {
        if !kind.is_empty() && !(kind.len() <= 255 && kind.chars().all(|c| c.is_ascii_graphic())) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, .. } => {
                meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?.data = (!kind.is_empty()).then(|| (kind.to_string(), std::rc::Rc::new(data.to_vec()))).into_iter().collect();
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Give an item another kind to carry besides what it has.
    pub fn list_box_add_drag_data(&mut self, h: Handle, index: i32, kind: &str, data: &[u8]) -> Result<()> {
        if kind.is_empty() || kind.len() > 255 || !kind.chars().all(|c| c.is_ascii_graphic()) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, .. } => {
                let carried = &mut meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?.data;
                carried.retain(|(have, _)| have != kind);
                carried.push((kind.to_string(), std::rc::Rc::new(data.to_vec())));
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// What the rows being dragged carry besides themselves, kind by kind:
    /// the first that carries anything speaks for them all.
    pub fn row_drag_data(&self) -> Vec<(String, Vec<u8>)> {
        let Some(drag) = self.row_drag else { return Vec::new() };
        let Some(Kind::List { meta, .. }) = self.nodes.get(drag.from).map(|n| &n.kind) else { return Vec::new() };
        let carrier = self.travelling().iter().find_map(|(i, _)| meta.get(*i).filter(|m| !m.data.is_empty()));
        carrier.map(|m| m.data.iter().map(|(kind, data)| (kind.clone(), data.to_vec())).collect()).unwrap_or_default()
    }

    /// The files the rows being dragged stand for, if they all stand for one.
    pub fn row_drag_files(&self) -> Vec<String> {
        let Some(drag) = self.row_drag else { return Vec::new() };
        let Some(Kind::List { meta, .. }) = self.nodes.get(drag.from).map(|n| &n.kind) else { return Vec::new() };
        let files: Vec<Option<String>> = self.travelling().iter().map(|(i, _)| meta.get(*i).and_then(|m| m.file.clone())).collect();
        if files.iter().all(Option::is_some) { files.into_iter().flatten().collect() } else { Vec::new() }
    }

    /// A picture of the rows being dragged, to show under the pointer once
    /// they have left their window: each one's icon and name, as in the
    /// list, inside a dotted outline. The desktop shows through elsewhere.
    pub fn row_drag_picture(&mut self) -> Option<Canvas> {
        const MOST: usize = 8;
        let drag = self.row_drag?;
        let Some(Kind::List { items, meta, .. }) = self.nodes.get(drag.from).map(|n| &n.kind) else { return None };
        let rows: Vec<(String, Icon, Option<Art>)> = self
            .travelling()
            .iter()
            .filter_map(|(i, _)| Some((items.get(*i)?.split('\t').next().unwrap_or("").to_string(), meta.get(*i)?.icon, meta.get(*i)?.art.clone())))
            .collect();
        if rows.is_empty() {
            return None;
        }
        let pictured = rows.iter().any(|(_, icon, art)| *icon != Icon::None || art.is_some());
        let label_x = 4 + if pictured { ICON } else { 0 };
        let more = rows.len().saturating_sub(MOST);
        let last = if more > 0 { Some(format!("and {more} more")) } else { None };
        let named: Vec<&str> = rows.iter().take(MOST).map(|(name, ..)| name.as_str()).chain(last.as_deref()).collect();
        let widest = named.iter().map(|text| self.font.width(text)).max().unwrap_or(0).min(220);
        let shown = rows.len().min(MOST) + last.iter().count();
        let (w, h) = (label_x + widest + 6, shown as i32 * self.look.m.row_height + 2);
        let mut canvas = Canvas::new(w, h);
        canvas.pixels.fill(crate::canvas::TRANSPARENT);
        canvas.fill(Rect::new(1, 1, w - 2, h - 2), WHITE);
        let text_y = (self.look.m.row_height - self.font.line_height()) / 2;
        for (n, (name, icon, art)) in rows.iter().take(MOST).enumerate() {
            let y = 1 + n as i32 * self.look.m.row_height;
            match art {
                Some(art) => Self::draw_art(&mut canvas, art, 3, y, self.look.m.row_height),
                None => Self::draw_icon(&mut canvas, *icon, 3, y, self.look.m.row_height, BLACK),
            }
            self.draw_cell(&mut canvas, name, label_x, y + text_y, widest + 2);
        }
        if let Some(last) = last {
            self.draw_cell(&mut canvas, &last, label_x, 1 + MOST as i32 * self.look.m.row_height + text_y, widest + 2);
        }
        canvas.ants(Rect::new(0, 0, w, h), 0);
        Some(canvas)
    }

    /// Whether an item can be pulled loose from its list: where rows can be
    /// moved about, any can; elsewhere, one that stands for a file.
    fn row_comes_loose(&self, h: Handle, item: usize) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::List { reorderable, meta, .. }) if *reorderable || meta.get(item).is_some_and(|m| m.file.is_some() || !m.data.is_empty()))
    }

    fn list_reorderable(&self, h: Handle) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::List { reorderable: true, .. }))
    }

    pub fn list_box_set_rows(&mut self, h: Handle, value: i32) -> Result<()> {
        if !(2..=1000).contains(&value) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { rows, .. } => {
                *rows = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn list_box_set_min_width(&mut self, h: Handle, value: i32) -> Result<()> {
        if !(3 * self.look.m.scroll_bar..=16384).contains(&value) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { min_width, .. } => {
                *min_width = value;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Add a column to a table: its heading and its width in pixels.
    pub fn table_add_column(&mut self, h: Handle, title: &str, width: i32) -> Result<()> {
        if !(8..=4096).contains(&width) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { mode: ListMode::Table, columns, .. } => {
                columns.push((title.to_string(), width));
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Add an item to a tree under another (or at the top with -1). Returns
    /// the new item's index.
    pub fn tree_view_add_child(&mut self, h: Handle, parent: i32, label: &str) -> Result<i32> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { mode: ListMode::Tree, items, meta, order, .. } => {
                if parent < -1 || parent >= items.len() as i32 {
                    return Err(Error::InvalidArgument);
                }
                order.push(items.len());
                items.push(label.to_string());
                meta.push(ItemMeta { parent: (parent >= 0).then_some(parent as usize), ..ItemMeta::default() });
                Ok(items.len() as i32 - 1)
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Unfold or fold an item of a tree from the program.
    pub fn tree_view_set_expanded(&mut self, h: Handle, index: i32, open: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::List { mode: ListMode::Tree, meta, .. } => {
                meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?.open = open;
            }
            _ => return Err(Error::WrongType),
        }
        self.list_settle(h);
        Ok(())
    }

    pub fn tree_view_expanded(&self, h: Handle, index: i32) -> Result<bool> {
        match &self.node(h)?.kind {
            Kind::List { mode: ListMode::Tree, meta, .. } => meta.get(index.max(0) as usize).filter(|_| index >= 0).map(|m| m.open).ok_or(Error::InvalidArgument),
            _ => Err(Error::WrongType),
        }
    }

    // ------------------------------------------------------------- shape

    /// The rows that are showing, top to bottom.
    fn list_rows(&self, h: Handle) -> Vec<Row> {
        let Some(Kind::List { items, mode, meta, sort, order, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return Vec::new() };
        if *mode != ListMode::Tree {
            let mut order: Vec<usize> = order.clone();
            if let Some((column, ascending)) = *sort {
                // a stable sort: rows that compare equal keep the order they were added in
                order.sort_by(|a, b| {
                    let by = compare_cells(cell(&items[*a], column), cell(&items[*b], column));
                    if ascending { by } else { by.reverse() }
                });
            }
            return order.into_iter().map(|item| Row { item, depth: 0, fold: None }).collect();
        }
        // children of each item, in the order they were added
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); items.len()];
        let mut top = Vec::new();
        // in the order kept for them, which dragging can change
        for i in order.iter().copied().filter(|i| *i < meta.len()) {
            match meta[i].parent {
                Some(p) => children[p].push(i),
                None => top.push(i),
            }
        }
        let mut out = Vec::new();
        let mut pending: Vec<(usize, i32)> = top.into_iter().rev().map(|i| (i, 0)).collect();
        while let Some((item, depth)) = pending.pop() {
            let open = meta[item].open;
            out.push(Row { item, depth, fold: (!children[item].is_empty()).then_some(open) });
            if open {
                pending.extend(children[item].iter().rev().map(|c| (*c, depth + 1)));
            }
        }
        out
    }

    fn list_is(&self, h: Handle, wanted: ListMode) -> bool {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::List { mode, .. }) if *mode == wanted)
    }

    /// The height of a table's row of headings.
    fn list_header(&self, h: Handle) -> i32 {
        if self.list_is(h, ListMode::Table) { self.look.m.row_height + 1 } else { 0 }
    }

    /// The area the rows are drawn in, how many fit, and how many there are.
    fn list_shape(&self, h: Handle) -> Option<(Rect, i32, i32)> {
        let node = self.nodes.get(h).filter(|n| matches!(n.kind, Kind::List { .. }))?;
        let (bar, header) = (Self::area_bar_rect(node.rect, self.look.m.scroll_bar), self.list_header(h));
        let rows = Rect::new(node.rect.x + 2, node.rect.y + 2 + header, (bar.x - node.rect.x - 2).max(0), (node.rect.h - 4 - header).max(0));
        if self.list_is(h, ListMode::Icons) {
            // counted in rows of icons, each holding as many as fit across
            let across = self.icons_across(h);
            let in_line = (self.list_rows(h).len() as i32 + across - 1) / across;
            // and far enough to reach any that have been put lower down
            let lowest = match &node.kind {
                Kind::List { meta, .. } => meta.iter().filter_map(|m| m.at).map(|at| (at.1 + 2 * CELL.1 - 1) / CELL.1).max().unwrap_or(0),
                _ => 0,
            };
            return Some((rows, (rows.h / CELL.1).max(1), in_line.max(lowest)));
        }
        Some((rows, (rows.h / self.look.m.row_height).max(1), self.list_rows(h).len() as i32))
    }

    /// A list's scroll bar, for the shared scroll bar code.
    pub(super) fn list_bar(&self, h: Handle, enabled: bool) -> Option<Bar> {
        let (_, fit, count) = self.list_shape(h)?;
        let node = self.nodes.get(h)?;
        let Kind::List { scroll, .. } = &node.kind else { return None };
        let max = (count - fit).max(0);
        Some(Bar { rect: Self::area_bar_rect(node.rect, self.look.m.scroll_bar), vertical: true, value: (*scroll).clamp(0, max), max, page: fit, step: 1, enabled, together: self.look.m.scroll_arrows == 1 })
    }

    /// After rows appear or disappear: keep the scroll position in range,
    /// and move the selection off anything no longer showing, onto what it
    /// hung from.
    fn list_settle(&mut self, h: Handle) {
        let showing: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        let Some((_, fit, count)) = self.list_shape(h) else { return };
        if let Some(Kind::List { selected, scroll, meta, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            for i in 0..meta.len() {
                if meta[i].picked && !showing.contains(&i) {
                    meta[i].picked = false;
                    let mut up = meta[i].parent;
                    while up.is_some_and(|p| !showing.contains(&p)) {
                        up = up.and_then(|p| meta[p].parent);
                    }
                    if let Some(p) = up {
                        meta[p].picked = true;
                    }
                    if *selected == Some(i) {
                        *selected = up;
                    }
                }
            }
            *scroll = (*scroll).clamp(0, (count - fit).max(0));
        }
        self.dirty = true;
    }

    /// Scroll just far enough to show the current item.
    fn list_reveal(&mut self, h: Handle) {
        let rows: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        let Some((area, fit, count)) = self.list_shape(h) else { return };
        // A list that has not been laid out has no height to count rows in,
        // so go by the rows it asks for; it would otherwise seem to fit one
        // row and scroll its selection to the top, hiding what is above.
        let fit = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { rows, .. }) if area.h == 0 => (*rows).max(1),
            _ => fit,
        };
        let icons = self.list_is(h, ListMode::Icons);
        // in an icon view the row is the row of icons the item is in, wherever it was put
        let icon_row = |ui: &Ui, s: usize| rows.iter().position(|r| *r == s).map(|place| ui.icon_cell(h, place, s).1 / CELL.1);
        let seen = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { selected: Some(s), .. }) if icons => icon_row(self, *s),
            Some(Kind::List { selected: Some(s), .. }) => rows.iter().position(|r| r == s).map(|r| r as i32),
            _ => None,
        };
        if let Some(Kind::List { scroll, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            *scroll = (*scroll).clamp(0, (count - fit).max(0));
            if let Some(row) = seen {
                *scroll = (*scroll).min(row).max(row - fit + 1);
            }
        }
        self.dirty = true;
    }

    /// Where a row's label starts, from the left of the rows area.
    fn label_x(&self, h: Handle, row: &Row) -> i32 {
        let Some(Kind::List { meta, mode, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return 4 };
        let mut x = 4 + row.depth * INDENT;
        if *mode == ListMode::Tree {
            x += MARK;
        }
        if meta.iter().any(ItemMeta::has_picture) {
            x += ICON; // every row leaves room once any has an icon, so labels line up
        }
        x
    }

    /// The item at a point, and whether the point is on its fold mark.
    fn list_item_at(&self, h: Handle, x: i32, y: i32) -> Option<(usize, bool)> {
        let (area, ..) = self.list_shape(h)?;
        let Some(Kind::List { scroll, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        if !area.contains(x, y) {
            return None;
        }
        let rows = self.list_rows(h);
        if self.list_is(h, ListMode::Icons) {
            // the last drawn is in front, so it is looked for first
            let (px, py) = (x - area.x, y - area.y + scroll * CELL.1);
            let found = rows.iter().enumerate().rev().find(|(place, row)| {
                let (cx, cy) = self.icon_cell(h, *place, row.item);
                Rect::new(cx, cy, CELL.0, CELL.1).contains(px, py)
            })?;
            return Some((found.1.item, false));
        }
        let row = rows.get((scroll + (y - area.y) / self.look.m.row_height) as usize)?;
        let mark_x = area.x + 2 + row.depth * INDENT;
        Some((row.item, row.fold.is_some() && (mark_x..mark_x + MARK).contains(&x)))
    }

    /// Change the selection because the user did, and say so if anything changed.
    fn list_pick(&mut self, h: Handle, item: usize, how: Pick) -> Vec<Emitted> {
        let order: Vec<usize> = self.list_rows(h).iter().map(|r| r.item).collect();
        let changed = match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::List { meta, selected, multiple, anchor, .. }) if item < meta.len() => {
                let before: Vec<bool> = meta.iter().map(|m| m.picked).collect();
                let was = *selected;
                match (if *multiple { how } else { Pick::Only }, *anchor) {
                    (Pick::Toggle, _) => {
                        meta[item].picked = !meta[item].picked;
                        *anchor = Some(item);
                    }
                    (Pick::Extend, Some(from)) => {
                        // everything between where the selection began and here, as shown
                        let (a, b) = (order.iter().position(|i| *i == from), order.iter().position(|i| *i == item));
                        if let (Some(a), Some(b)) = (a, b) {
                            meta.iter_mut().for_each(|m| m.picked = false);
                            for i in &order[a.min(b)..=a.max(b)] {
                                meta[*i].picked = true;
                            }
                        }
                    }
                    _ => {
                        meta.iter_mut().for_each(|m| m.picked = false);
                        meta[item].picked = true;
                        *anchor = Some(item);
                    }
                }
                *selected = Some(item);
                was != *selected || before != meta.iter().map(|m| m.picked).collect::<Vec<_>>()
            }
            _ => return Vec::new(),
        };
        self.list_reveal(h);
        if changed {
            vec![Emitted { target: h, event: "ListBox.changed", args: vec![Value::I32(item as i32)] }]
        } else {
            Vec::new()
        }
    }

    fn list_activate(&self, h: Handle) -> Vec<Emitted> {
        match self.list_box_selected(h) {
            Ok(item) if item >= 0 => vec![Emitted { target: h, event: "ListBox.activated", args: vec![Value::I32(item)] }],
            _ => Vec::new(),
        }
    }

    /// Fold or unfold an item of a tree because the user did.
    fn list_fold(&mut self, h: Handle, item: usize, open: bool) {
        if let Some(Kind::List { meta, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            if let Some(m) = meta.get_mut(item) {
                m.open = open;
            }
        }
        self.list_settle(h);
    }

    /// The text of the current item; a table's cells are run together.
    pub(super) fn list_label(&self, h: Handle) -> Option<String> {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { items, selected, .. }) => selected.and_then(|s| items.get(s)).map(|t| t.replace('\t', ", ")),
            _ => None,
        }
    }

    // ------------------------------------------------------- the headings

    /// Each column's left edge and width, from the left of the rows area.
    fn column_spans(&self, h: Handle) -> Vec<(i32, i32)> {
        let Some(Kind::List { columns, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return Vec::new() };
        let mut x = 0;
        columns
            .iter()
            .map(|(_, w)| {
                let span = (x, *w);
                x += w;
                span
            })
            .collect()
    }

    /// What a point in a table's row of headings is on: the right-hand edge
    /// of a column (to drag), or a heading (to sort by).
    /// The heading of a table a point is over, if the widget is a table.
    pub(super) fn heading_over(&self, h: Handle, x: i32, y: i32) -> Option<(usize, bool)> {
        matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::List { .. })).then(|| self.heading_at(h, x, y)).flatten()
    }

    fn heading_at(&self, h: Handle, x: i32, y: i32) -> Option<(usize, bool)> {
        let (area, ..) = self.list_shape(h)?;
        let header = self.list_header(h);
        if header == 0 || !(area.y - header..area.y).contains(&y) || !(area.x..area.x + area.w).contains(&x) {
            return None;
        }
        let spans = self.column_spans(h);
        if let Some(edge) = spans.iter().position(|(start, w)| (x - (area.x + start + w)).abs() <= GRAB) {
            return Some((edge, true));
        }
        spans.iter().position(|(start, w)| (area.x + start..area.x + start + w).contains(&x)).map(|c| (c, false))
    }

    /// Whether the pointer is over the edge of a column, which can be dragged.
    pub(super) fn over_column_edge(&self, h: Handle, x: i32, y: i32) -> bool {
        self.column_drag.is_some_and(|d| d.0 == h) || matches!(self.heading_at(h, x, y), Some((_, true)))
    }

    // ----------------------------------------------------------- drawing

    pub(super) fn measure_list(&self, h: Handle) -> (i32, i32) {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { rows, min_width, mode: ListMode::Icons, .. }) => ((*min_width).max(3 * CELL.0 + self.look.m.scroll_bar + 4), rows * CELL.1 + 4),
            Some(Kind::List { rows, min_width, columns, .. }) => {
                // a table is as wide as its columns need
                let wide = columns.iter().map(|c| c.1).sum::<i32>() + self.look.m.scroll_bar + 4;
                ((*min_width).max(if columns.is_empty() { 0 } else { wide }), rows * self.look.m.row_height + 4 + self.list_header(h))
            }
            _ => (0, 0),
        }
    }

    /// Draw text into a cell, cut short if it will not fit.
    fn draw_cell(&mut self, clip: &mut Canvas, text: &str, x: i32, y: i32, width: i32) {
        let mut shown = String::new();
        let mut used = 0;
        for ch in text.chars() {
            used += self.font.advance(ch);
            if used > width {
                break;
            }
            shown.push(ch);
        }
        let ink = self.look.pal.list_ink;
        self.font.draw(clip, &shown, x, y, ink);
    }

    /// Give an item a picture of the program's own. `art` is the picture
    /// drawn in text, a line for each row of pixels: X or # is black, a full
    /// stop is white, and anything else lets the row show through. Up to 14
    /// pixels each way. An empty string takes the picture away.
    /// Give an item a picture in colour, up to 16 pixels each way; none
    /// takes it away. It is shown in place of any other picture the item has.
    pub fn list_box_set_image(&mut self, h: Handle, index: i32, image: Option<Canvas>) -> Result<()> {
        if image.as_ref().is_some_and(|i| i.width > 16 || i.height > 16) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, .. } => {
                let m = meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?;
                m.image = image.map(std::rc::Rc::new);
                self.dirty = true;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn list_box_set_picture(&mut self, h: Handle, index: i32, art: &str) -> Result<()> {
        let rows: Vec<Vec<u8>> = art
            .lines()
            .map(|line| {
                line.chars()
                    .map(|c| match c {
                        'X' | 'x' | '#' => 1,
                        '.' => 2,
                        _ => 0,
                    })
                    .collect()
            })
            .collect();
        if rows.len() > ART_MAX || rows.iter().any(|r| r.len() > ART_MAX) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::List { meta, .. } => {
                let m = meta.get_mut(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?;
                m.art = (!rows.is_empty()).then(|| std::rc::Rc::new(rows));
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    fn draw_art(clip: &mut Canvas, art: &[Vec<u8>], x: i32, y: i32, row: i32) {
        let top = y + (row - art.len() as i32) / 2;
        for (dy, row) in art.iter().enumerate() {
            for (dx, pixel) in row.iter().enumerate() {
                match pixel {
                    1 => clip.set(x + dx as i32, top + dy as i32, BLACK),
                    2 => clip.set(x + dx as i32, top + dy as i32, WHITE),
                    _ => {}
                }
            }
        }
    }

    fn draw_icon(clip: &mut Canvas, icon: Icon, x: i32, y: i32, row: i32, ink: u32) {
        let shape: &[&str] = match icon {
            Icon::None => return,
            Icon::Folder => &FOLDER,
            Icon::Document => &DOCUMENT,
            Icon::Application => &APPLICATION,
        };
        let top = y + (row - shape.len() as i32) / 2;
        for (dy, row) in shape.iter().enumerate() {
            for (dx, b) in row.bytes().enumerate() {
                if b == b'X' {
                    clip.set(x + dx as i32, top + dy as i32, ink);
                }
            }
        }
    }

    /// The large picture for an item of an icon view: the program's own,
    /// drawn double size, or the stock one for its kind.
    fn big_picture(meta: &ItemMeta) -> Canvas {
        use crate::pictures::{picture, Picture};
        match (&meta.art, meta.icon) {
            (Some(art), _) => {
                let mut small = Canvas::new(art.iter().map(Vec::len).max().unwrap_or(1).max(1) as i32, art.len().max(1) as i32);
                small.pixels.fill(crate::canvas::TRANSPARENT);
                Self::draw_art(&mut small, art, 0, 0, art.len() as i32);
                small.scaled(2)
            }
            (None, Icon::Folder) => picture(Picture::Folder),
            (None, Icon::Application) => picture(Picture::Application),
            (None, _) => picture(Picture::Document),
        }
    }

    /// Draw a list shown as icons.
    fn paint_icons(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        let Some((area, fit, count)) = self.list_shape(h) else { return };
        let (rows, across) = (self.list_rows(h), self.icons_across(h));
        let Some(Kind::List { items, scroll, meta, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (items, meta, scroll) = (items.clone(), meta.clone(), (*scroll).clamp(0, (count - fit).max(0)));
        self.part(canvas, Part::new(PartKind::Well, rect).numbers(1, 0, 0).when(state::FOCUSED, live));
        let editing = self.list_edit.as_ref().filter(|e| e.list == h).map(|e| e.item);
        let mut clip = Canvas::like(canvas, area.w.max(1), area.h.max(1), self.look.pal.list);
        let _ = across;
        for (place, row) in rows.iter().enumerate() {
            let (x, y) = self.icon_cell(h, place, row.item);
            let y = y - scroll * CELL.1 + 2;
            if y + CELL.1 < 0 || y > (fit + 1) * CELL.1 {
                continue; // scrolled out of sight
            }
            let name = items[row.item].split('\t').next().unwrap_or("");
            let art = Self::big_picture(&meta[row.item]);
            let drawn = crate::pictures::labelled(&mut self.font, name, &art, CELL.0, meta[row.item].picked);
            clip.blit(&drawn, x, y);
            if editing == Some(row.item) {
                self.paint_edit(h, &mut clip, y + crate::pictures::SIDE + 2, area.w);
            }
        }
        canvas.blit(&clip, area.x, area.y);
        self.bar_draw(h, false, canvas);
    }

    pub(super) fn paint_list(&mut self, h: Handle, rect: Rect, live: bool, canvas: &mut Canvas) {
        if self.list_is(h, ListMode::Icons) {
            return self.paint_icons(h, rect, live, canvas);
        }
        let Some((area, fit, count)) = self.list_shape(h) else { return };
        let rows = self.list_rows(h);
        let (header, spans) = (self.list_header(h), self.column_spans(h));
        let Some(Kind::List { items, scroll, columns, meta, sort, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (items, columns, meta, sort) = (items.clone(), columns.clone(), meta.clone(), *sort);
        let scroll = (*scroll).clamp(0, (count - fit).max(0));
        let line = self.font.line_height();
        let (paper, ink) = (self.look.pal.list, self.look.pal.list_ink);
        self.part(canvas, Part::new(PartKind::Well, rect).numbers(1, 0, 0).when(state::FOCUSED, live));
        // headings and rows share one sheet so that both are clipped to the list
        let mut clip = Canvas::like(canvas, area.w.max(1), (area.h + header).max(1), paper);
        if header > 0 {
            self.part(&mut clip, Part::new(PartKind::Header, Rect::new(0, 0, area.w, header)));
            // the heading the pointer is over, for a style that lights it
            let hot = if self.hover == Some(h) && (1000..2000).contains(&self.hover_piece) { Some((self.hover_piece - 1000) as usize) } else { None };
            for (i, (x, width)) in spans.iter().enumerate() {
                self.part(&mut clip, Part::new(PartKind::HeaderCell, Rect::new(*x, 0, *width, header)).when(state::HOT, hot == Some(i)));
            }
        }
        let text_y = (self.look.m.row_height - line) / 2;
        let has_icons = meta.iter().any(ItemMeta::has_picture);
        for (i, ((title, _), (x, width))) in columns.iter().zip(&spans).enumerate() {
            self.draw_cell(&mut clip, title, x + 4, text_y, width - 8 - if sort.is_some_and(|s| s.0 == i) { 10 } else { 0 });
            if let Some((_, ascending)) = sort.filter(|s| s.0 == i) {
                // a mark at the column's right
                let (cx, cy) = (x + width - 9, self.look.m.row_height / 2);
                let mark = Part::new(PartKind::SortArrow, Rect::new(*x, 0, *width, header)).numbers(cx, cy, 0).ink(ink);
                self.part(&mut clip, mark.when(state::CHECKED, ascending));
            }
        }
        let editing = self.list_edit.as_ref().filter(|e| e.list == h).map(|e| (e.item, e.column));
        for (at, row) in rows.iter().enumerate().skip(scroll as usize).take(fit as usize + 1) {
            let y = header + (at as i32 - scroll) * self.look.m.row_height;
            let text = &items[row.item];
            let label_x = self.label_x(h, row);
            if let Some(open) = row.fold {
                let (cx, cy) = (4 + row.depth * INDENT + 4, y + self.look.m.row_height / 2);
                let mark = Part::new(PartKind::TreeFold, Rect::new(cx - 5, cy - 5, 11, 11)).numbers(cx, cy, 0).ink(ink);
                self.part(&mut clip, mark.when(state::CHECKED, open));
            }
            if has_icons {
                match (&meta[row.item].image, &meta[row.item].art) {
                    // a picture in colour, in the middle of the row's height
                    (Some(image), _) => clip.blit(image, label_x - ICON, y + (self.look.m.row_height - image.height) / 2),
                    (None, Some(art)) => Self::draw_art(&mut clip, art, label_x - ICON, y, self.look.m.row_height),
                    (None, None) => Self::draw_icon(&mut clip, meta[row.item].icon, label_x - ICON, y, self.look.m.row_height, ink),
                }
            }
            if columns.is_empty() {
                self.draw_cell(&mut clip, text, label_x, y + text_y, area.w - label_x - 2);
            } else {
                for (i, (cell, (x, width))) in text.split('\t').zip(&spans).enumerate() {
                    // the first column makes room for the icon
                    let inset = if i == 0 { label_x } else { 4 };
                    self.draw_cell(&mut clip, cell, x + inset, y + text_y, width - inset - 4);
                }
            }
            if meta[row.item].picked {
                self.part(&mut clip, Part::new(PartKind::Select, Rect::new(0, y, area.w, self.look.m.row_height)));
                // a picture in colour keeps its colours, on whatever shows the row is chosen
                if let Some(image) = &meta[row.item].image {
                    clip.blit(image, label_x - ICON, y + (self.look.m.row_height - image.height) / 2);
                }
            }
            if editing.is_some_and(|(item, _)| item == row.item) {
                self.paint_edit(h, &mut clip, y, area.w);
            }
        }
        // where rows on the move would land, whether they are ours or another program's
        let mark = self.row_drag.filter(|d| d.to == h && d.live).map(|d| (d.gap, d.onto)).or(self.foreign_drop.filter(|f| f.0 == h).map(|f| (f.1, f.2)));
        if let Some((gap, onto)) = mark {
            match onto.and_then(|item| rows.iter().position(|r| r.item == item)) {
                // a heavy box round a row: the rows being moved would hang from it
                Some(at) => {
                    let r = Rect::new(0, header + (at as i32 - scroll) * self.look.m.row_height, area.w, self.look.m.row_height);
                    clip.frame(r, ink);
                    clip.frame(r.inset(1), ink);
                }
                // a heavy line between two rows: where they would land
                None => {
                    let y = header + (gap as i32 - scroll) * self.look.m.row_height;
                    clip.fill(Rect::new(0, (y - 1).max(header), area.w, 2), ink);
                }
            }
        }
        canvas.blit(&clip, area.x, area.y - header);
        self.bar_draw(h, false, canvas);
    }

    // --------------------------------------------------- editing in place

    /// The cell being edited: its left edge and width, from the left of the rows area.
    fn edit_span(&self, h: Handle, width: i32) -> Option<(i32, i32)> {
        let edit = self.list_edit.as_ref().filter(|e| e.list == h)?;
        let rows = self.list_rows(h);
        if self.list_is(h, ListMode::Icons) {
            // the name under the icon, the width of its cell
            let place = rows.iter().position(|r| r.item == edit.item)?;
            return Some((self.icon_cell(h, place, edit.item).0 + 2, CELL.0 - 4));
        }
        let row = rows.iter().find(|r| r.item == edit.item)?;
        let label_x = self.label_x(h, row);
        let spans = self.column_spans(h);
        Some(match spans.get(edit.column) {
            Some((x, w)) if edit.column == 0 => (x + label_x - 3, w - label_x + 2),
            Some((x, w)) => (x + 1, w - 2),
            None => (label_x - 3, width - label_x + 2),
        })
    }

    fn paint_edit(&mut self, h: Handle, clip: &mut Canvas, y: i32, width: i32) {
        let Some((x, w)) = self.edit_span(h, width) else { return };
        let Some(edit) = self.list_edit.as_ref().map(|e| e.edit.clone()) else { return };
        // a box of paper over the row, which may be selected, with the text in ink on it
        let (paper, ink) = (self.look.pal.paper, self.look.pal.ink);
        let cell_box = Rect::new(x, y, w, self.look.m.row_height);
        clip.fill(cell_box, paper);
        clip.frame(cell_box, ink);
        let mut inner = Canvas::like(clip, (w - 4).max(1), self.look.m.row_height - 2, paper);
        let line = self.font.line_height();
        // what an input method is still composing shows where it will go, underlined
        let pre = self.preedit.as_ref().filter(|(f, _)| *f == h).map(|(_, t)| t.clone()).unwrap_or_default();
        if pre.is_empty() {
            self.font.draw(&mut inner, edit.text(), 1, (self.look.m.row_height - 2 - line) / 2, ink);
        } else {
            let (from, to) = edit.selection();
            let shown = format!("{}{}{}", &edit.text()[..from], pre, &edit.text()[to..]);
            self.font.draw(&mut inner, &shown, 1, (self.look.m.row_height - 2 - line) / 2, ink);
            let (x0, x1) = (self.font.width(&edit.text()[..from]), self.font.width(&edit.text()[..from]) + self.font.width(&pre));
            inner.fill(Rect::new(x0 + 1, self.look.m.row_height - 3, x1 - x0, 1), ink);
            if self.caret_on() {
                self.part(&mut inner, Part::new(PartKind::Caret, Rect::new(x1, 0, 1, self.look.m.row_height - 2)).ink(ink));
            }
        }
        if !pre.is_empty() {
            // drawn above
        } else if edit.has_selection() {
            let (from, to) = edit.selection();
            let (x0, x1) = (self.font.width(&edit.text()[..from]), self.font.width(&edit.text()[..to]));
            self.part(&mut inner, Part::new(PartKind::Select, Rect::new(x0, 0, x1 - x0 + 1, self.look.m.row_height - 2)).numbers(1, 0, 0));
        } else if self.caret_on() {
            let cx = self.font.width(&edit.text()[..edit.caret()]);
            self.part(&mut inner, Part::new(PartKind::Caret, Rect::new(cx, 0, 1, self.look.m.row_height - 2)).ink(ink));
        }
        clip.blit(&inner, x + 2, y + 1);
    }

    /// Start editing a cell in place. Its text is selected, ready to be
    /// typed over.
    pub fn list_box_edit(&mut self, h: Handle, index: i32, column: i32) -> Result<()> {
        let text = match &self.node(h)?.kind {
            Kind::List { items, columns, .. } => {
                let item = items.get(index.max(0) as usize).filter(|_| index >= 0).ok_or(Error::InvalidArgument)?;
                if column < 0 || column as usize >= columns.len().max(1) {
                    return Err(Error::InvalidArgument);
                }
                if columns.is_empty() { item.clone() } else { cell(item, column as usize).to_string() }
            }
            _ => return Err(Error::WrongType),
        };
        self.list_box_set_selected(h, index)?;
        let mut edit = Editor::new(&text);
        edit.select_all();
        self.list_edit = Some(ListEdit { list: h, item: index as usize, column: column as usize, edit });
        self.caret_epoch = self.now;
        Ok(())
    }

    /// Whether a list has a cell being edited.
    pub(super) fn list_editing(&self, h: Handle) -> bool {
        self.list_edit.as_ref().is_some_and(|e| e.list == h)
    }

    /// Finish editing: keep the new text and report it, or throw it away.
    pub(super) fn list_end_edit(&mut self, keep: bool) -> Vec<Emitted> {
        let Some(done) = self.list_edit.take() else { return Vec::new() };
        self.dirty = true;
        if !keep {
            return Vec::new();
        }
        let text = done.edit.text().to_string();
        let Some(Kind::List { items, columns, .. }) = self.nodes.get_mut(done.list).map(|n| &mut n.kind) else { return Vec::new() };
        let Some(slot) = items.get_mut(done.item) else { return Vec::new() };
        let before = slot.clone();
        if columns.is_empty() {
            *slot = text.clone();
        } else {
            // put the cell back among its neighbours
            let mut cells: Vec<String> = slot.split('\t').map(String::from).collect();
            cells.resize(cells.len().max(done.column + 1), String::new());
            cells[done.column] = text.clone();
            *slot = cells.join("\t");
        }
        if *slot == before {
            return Vec::new(); // nothing was changed
        }
        vec![Emitted { target: done.list, event: "ListBox.edited", args: vec![Value::I32(done.item as i32), Value::I32(done.column as i32), Value::Str(text)] }]
    }

    /// A key while a cell is being edited. Return keeps the edit, Escape
    /// throws it away, and the rest edit the text.
    pub(super) fn list_edit_key(&mut self, key: Key, mods: Mods) -> Vec<Emitted> {
        let (word, extend) = (mods.ctrl, mods.shift);
        match key {
            Key::Return => return self.list_end_edit(true),
            Key::Escape => return self.list_end_edit(false),
            _ => {}
        }
        // cut, copy and paste, as in a text field
        if let (true, Some(e)) = (mods.ctrl, &mut self.list_edit) {
            match key {
                Key::Char('c') | Key::Char('x') if e.edit.has_selection() => {
                    self.copy_out = Some(e.edit.selected().to_string());
                    if key == Key::Char('x') {
                        e.edit.backspace_selection();
                    }
                }
                Key::Char('v') => self.paste_request = Some(e.list),
                _ => {}
            }
        }
        if let Some(e) = &mut self.list_edit {
            match key {
                Key::Left => e.edit.left(word, extend),
                Key::Right => e.edit.right(word, extend),
                Key::Home | Key::Up => e.edit.home(extend),
                Key::End | Key::Down => e.edit.end(extend),
                Key::Backspace => drop(e.edit.backspace(word)),
                Key::Delete => drop(e.edit.delete(word)),
                Key::Char('a') if mods.ctrl => e.edit.select_all(),
                _ => {}
            }
        }
        self.caret_epoch = self.now;
        self.dirty = true;
        Vec::new()
    }

    /// An input method settled on some text for the cell being edited.
    pub(super) fn list_edit_compose(&mut self, text: &str, before: usize, after: usize) {
        if let Some(e) = &mut self.list_edit {
            e.edit.delete_around(before, after);
            if !text.is_empty() {
                e.edit.insert(text);
            }
        }
        self.caret_epoch = self.now;
        self.dirty = true;
    }

    /// Where the caret is in the cell being edited, in window coordinates,
    /// for an input method to put its candidates beside.
    pub(super) fn list_edit_caret(&mut self, h: Handle) -> Option<Rect> {
        let (item, before) = self.list_edit.as_ref().filter(|e| e.list == h).map(|e| (e.item, e.edit.text()[..e.edit.selection().0].to_string()))?;
        let (area, ..) = self.list_shape(h)?;
        let (x, _) = self.edit_span(h, area.w)?;
        let Some(Kind::List { scroll, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        let place = self.list_rows(h).iter().position(|r| r.item == item)? as i32 - scroll;
        let pre = self.preedit.as_ref().filter(|(f, _)| *f == h).map(|(_, t)| t.clone()).unwrap_or_default();
        let along = self.font.width(&before) + self.font.width(&pre);
        if self.list_is(h, ListMode::Icons) {
            let down = self.icon_cell(h, (place + scroll) as usize, item).1 - scroll * CELL.1;
            return Some(Rect::new(area.x + x + 3 + along, area.y + down + 4 + crate::pictures::SIDE, 1, self.look.m.row_height));
        }
        Some(Rect::new(area.x + x + 3 + along, area.y + place * self.look.m.row_height, 1, self.look.m.row_height))
    }

    /// Typed text while a cell is being edited.
    pub(super) fn list_edit_text(&mut self, text: &str) {
        if let Some(e) = &mut self.list_edit {
            e.edit.insert(text);
        }
        self.caret_epoch = self.now;
        self.dirty = true;
    }

    /// A press while a cell is being edited: inside the cell it places the
    /// caret and is used up; anywhere else it ends the edit, keeping it, and
    /// the press goes on to do whatever it would have done.
    pub(super) fn list_edit_press(&mut self, x: i32, y: i32) -> (bool, Vec<Emitted>) {
        let Some(list) = self.list_edit.as_ref().map(|e| e.list) else { return (false, Vec::new()) };
        let Some((area, ..)) = self.list_shape(list) else { return (false, self.list_end_edit(false)) };
        let inside = self.edit_span(list, area.w).zip(self.list_item_at(list, x, y)).filter(|((sx, sw), (item, _))| {
            self.list_edit.as_ref().is_some_and(|e| e.item == *item) && (area.x + sx..area.x + sx + sw).contains(&x)
        });
        let Some(((sx, _), _)) = inside else { return (false, self.list_end_edit(true)) };
        let text = self.list_edit.as_ref().map(|e| e.edit.text().to_string()).unwrap_or_default();
        let (mut pen, mut at) = (area.x + sx + 3, text.len());
        for (i, ch) in text.char_indices() {
            let advance = self.font.advance(ch);
            if x < pen + (advance + 1) / 2 {
                at = i;
                break;
            }
            pen += advance;
        }
        if let Some(e) = &mut self.list_edit {
            e.edit.move_to(at, false);
        }
        self.caret_epoch = self.now;
        self.dirty = true;
        (true, Vec::new())
    }

    // -------------------------------------------------------------- input

    /// The pointer went down in a list.
    pub(super) fn list_press(&mut self, h: Handle, x: i32, y: i32) -> Vec<Emitted> {
        let Some(window) = self.window_of(h) else { return Vec::new() };
        if self.live_focus(window) != Some(h) {
            self.set_focus(window, Some(h), false);
        }
        // the headings: drag an edge to resize, click a heading to sort
        if let Some((column, edge)) = self.heading_at(h, x, y) {
            if let Some(Kind::List { columns, sortable, sort, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
                if edge {
                    self.column_drag = Some((h, column, x, columns[column].1));
                } else if *sortable {
                    // a second click on the same heading turns the order round
                    *sort = Some((column, !matches!(*sort, Some((c, true)) if c == column)));
                    self.list_reveal(h);
                }
            }
            return Vec::new();
        }
        let Some((item, on_mark)) = self.list_item_at(h, x, y) else { return Vec::new() };
        if on_mark {
            // the fold mark folds and unfolds, and does nothing else
            let open = self.tree_view_expanded(h, item as i32).unwrap_or(false);
            self.list_fold(h, item, !open);
            return Vec::new();
        }
        // a second click on the same row, soon after, opens it
        let (t, px, py, n) = self.clicks;
        let again = n > 0 && self.now - t <= DOUBLE_CLICK_MS && (x - px).abs() <= 3 && (y - py).abs() <= 3;
        self.clicks = (self.now, x, y, if again { n + 1 } else { 1 });
        self.list_drag = Some(h);
        // A plain press on one of several selected rows may be the start of
        // dragging them all, so the others are not let go until it proves
        // to be only a click.
        let loose = self.row_comes_loose(h, item);
        if let Some(Kind::List { meta, selected, multiple: true, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind).filter(|_| loose) {
            if !self.mods.ctrl && !self.mods.shift && !again && meta[item].picked && meta.iter().filter(|m| m.picked).count() > 1 {
                *selected = Some(item);
                self.list_collapse = Some((h, item));
                return Vec::new();
            }
        }
        let how = if self.mods.ctrl {
            Pick::Toggle
        } else if self.mods.shift {
            Pick::Extend
        } else {
            Pick::Only
        };
        let mut out = self.list_pick(h, item, how);
        if again && how == Pick::Only {
            out.extend(self.list_activate(h));
        }
        out
    }

    /// The pointer moved with the button down in a list: the selection
    /// follows, or where several can be selected, stretches.
    pub(super) fn list_drag_to(&mut self, h: Handle, x: i32, y: i32) -> Vec<Emitted> {
        // where rows can be moved, a pressed row pulled far enough comes
        // loose and follows the pointer instead
        let pressed = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { selected: Some(item), .. }) => Some(*item).filter(|i| self.row_comes_loose(h, *i)),
            _ => None,
        };
        if let Some(item) = pressed {
            if self.row_drag.is_some() || (y - self.clicks.2).abs() > PULL || (x - self.clicks.1).abs() > 3 * PULL {
                if let Some(window) = self.window_of(h) {
                    // where in its cell an icon was taken hold of, so that it is put down the same way
                    let grip = match (self.row_drag, self.list_shape(h), self.nodes.get(h).map(|n| &n.kind)) {
                        (Some(drag), ..) => drag.grip,
                        (None, Some((area, ..)), Some(Kind::List { scroll, mode: ListMode::Icons, .. })) => {
                            let place = self.list_rows(h).iter().position(|r| r.item == item).unwrap_or(0);
                            let cell = self.icon_cell(h, place, item);
                            (self.clicks.1 - area.x - cell.0, self.clicks.2 - area.y + scroll * CELL.1 - cell.1)
                        }
                        _ => (0, 0),
                    };
                    let icons = self.list_is(h, ListMode::Icons);
                    match self.row_target(window, x, y, Some(h)) {
                        Some((to, gap, onto)) => self.row_drag = Some(RowDrag { from: h, item, to, gap, onto, live: true, point: (x, y), grip }),
                        // loose, though nowhere here would have them: they are on their way out
                        None if icons || !self.list_reorderable(h) => self.row_drag = Some(RowDrag { from: h, item, to: h, gap: 0, onto: None, live: false, point: (x, y), grip }),
                        None => {}
                    }
                    self.dirty = true;
                }
                return Vec::new();
            }
            if self.list_collapse.is_some() {
                return Vec::new(); // waiting to see whether this is a click or a drag
            }
        }
        let stretch = matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::List { multiple: true, .. })) && !self.mods.ctrl;
        match self.list_item_at(h, x, y) {
            Some((item, _)) if stretch => self.list_pick(h, item, Pick::Extend),
            Some((item, _)) if !self.mods.ctrl => self.list_pick(h, item, Pick::Only),
            _ => Vec::new(),
        }
    }

    /// The pointer was let go in a list: put any rows being moved where the
    /// mark showed, in their own list or another, and say what happened.
    pub(super) fn row_drop(&mut self) -> Vec<Emitted> {
        let collapse = self.list_collapse.take();
        let Some(drag) = self.row_drag.take() else {
            // no drag came of it: a click on one of several selected rows selects just that one
            return match collapse {
                Some((h, item)) => self.list_pick(h, item, Pick::Only),
                None => Vec::new(),
            };
        };
        self.dirty = true;
        if !drag.live {
            return Vec::new(); // let go where nothing would have them
        }
        if drag.to == drag.from && self.list_is(drag.from, ListMode::Icons) {
            return self.icons_place(drag);
        }
        let moving = self.moving_rows(drag.from, drag.item);
        if drag.to != drag.from {
            self.rows_transfer(drag.from, drag.to, &moving, drag.gap, drag.onto)
        } else if self.list_is(drag.from, ListMode::Tree) {
            self.tree_move(drag.from, &moving, drag.gap, drag.onto)
        } else {
            self.rows_reorder(drag.from, &moving, drag.gap)
        }
    }

    /// The pointer moved or was let go while a column's edge is held.
    pub(super) fn column_drag_to(&mut self, x: i32, release: bool) {
        let Some((h, column, from_x, from_w)) = self.column_drag else { return };
        if let Some(Kind::List { columns, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
            if let Some(c) = columns.get_mut(column) {
                c.1 = (from_w + x - from_x).clamp(COLUMN_MIN, 4096);
            }
        }
        if release {
            self.column_drag = None;
        }
        self.dirty = true;
    }

    /// A key for a focused list. The arrows move the selection, with Shift
    /// stretching it where several can be selected; in a tree, Right unfolds
    /// or steps in and Left folds or steps out.
    pub(super) fn list_key(&mut self, h: Handle, key: Key, mods: Mods) -> Option<Vec<Emitted>> {
        let rows = self.list_rows(h);
        let (selected, multiple) = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::List { selected, multiple, .. }) => (*selected, *multiple),
            _ => return None,
        };
        if rows.is_empty() {
            return None;
        }
        if let (Key::Char('a'), true, true) = (key, mods.ctrl, multiple) {
            // everything showing, from the first row to the last
            if let Some(Kind::List { anchor, .. }) = self.nodes.get_mut(h).map(|n| &mut n.kind) {
                *anchor = Some(rows[0].item);
            }
            return Some(self.list_pick(h, rows[rows.len() - 1].item, Pick::Extend));
        }
        let at = selected.and_then(|s| rows.iter().position(|r| r.item == s));
        let last = rows.len() - 1;
        if self.list_is(h, ListMode::Icons) {
            // the arrows go about the rows of icons as they would a page of words
            let across = self.icons_across(h) as usize;
            let to = match (key, at) {
                (Key::Right, Some(i)) => (i + 1).min(last),
                (Key::Left, Some(i)) => i.saturating_sub(1),
                (Key::Down, Some(i)) => if i + across <= last { i + across } else { i },
                (Key::Up, Some(i)) => if i >= across { i - across } else { i },
                (Key::Right | Key::Down | Key::Home, _) => 0,
                (Key::Left | Key::Up | Key::End, _) => last,
                _ => return None,
            };
            return Some(self.list_pick(h, rows[to].item, if mods.shift { Pick::Extend } else { Pick::Only }));
        }
        let to = match (key, at) {
            (Key::Down, Some(i)) => i.min(last - 1) + 1,
            (Key::Up, Some(i)) => i.saturating_sub(1),
            (Key::Down | Key::Home, _) => 0,
            (Key::Up | Key::End, _) => last,
            (Key::Right, Some(i)) if rows[i].fold == Some(false) => {
                self.list_fold(h, rows[i].item, true);
                return Some(Vec::new());
            }
            (Key::Right, Some(i)) if rows[i].fold == Some(true) => i + 1, // into it
            (Key::Left, Some(i)) if rows[i].fold == Some(true) => {
                self.list_fold(h, rows[i].item, false);
                return Some(Vec::new());
            }
            // out to what it hangs from: the nearest row above that is less far in
            (Key::Left, Some(i)) if rows[i].depth > 0 => (0..i).rev().find(|j| rows[*j].depth < rows[i].depth).unwrap_or(i),
            _ => return None,
        };
        Some(self.list_pick(h, rows[to].item, if mods.shift { Pick::Extend } else { Pick::Only }))
    }

    /// Typing in a list jumps to the next showing row that starts with the letter.
    pub(super) fn list_type(&mut self, h: Handle, text: &str) -> Option<Vec<Emitted>> {
        let Some(Kind::List { items, selected, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        let typed = text.chars().next().filter(|c| !c.is_whitespace())?.to_lowercase().next()?;
        let rows = self.list_rows(h);
        let from = selected.and_then(|s| rows.iter().position(|r| r.item == s)).map_or(0, |i| i + 1);
        let starts = |row: &Row| items[row.item].chars().next().and_then(|c| c.to_lowercase().next()) == Some(typed);
        let found = (0..rows.len()).map(|i| (from + i) % rows.len()).find(|i| starts(&rows[*i]))?;
        let item = rows[found].item;
        Some(self.list_pick(h, item, Pick::Only))
    }

    /// Return in a list: where items can be edited it starts editing the
    /// current one, as renaming a file does; otherwise it opens it.
    pub(super) fn list_return(&mut self, h: Handle) -> Option<Vec<Emitted>> {
        let Some(Kind::List { editable, selected, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        match (*editable, *selected) {
            (true, Some(item)) => {
                let _ = self.list_box_edit(h, item as i32, 0);
                Some(Vec::new())
            }
            _ => Some(self.list_activate(h)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };

    fn window(ui: &mut Ui, content: Handle) -> Handle {
        let win = ui.window_new("Lists", 320, 220).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        ui.stack_add(col, content).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        win
    }

    fn click(ui: &mut Ui, win: Handle, x: i32, y: i32) -> Vec<Emitted> {
        let mut ev = ui.pointer(win, x, y, PointerKind::Press).unwrap();
        ev.extend(ui.pointer(win, x, y, PointerKind::Release).unwrap());
        ev
    }

    fn numbers(events: &[Emitted], name: &str) -> Vec<i32> {
        events.iter().filter(|e| e.event == name).map(|e| e.args[0].as_i32()).collect()
    }

    #[test]
    fn a_row_selected_before_the_list_is_laid_out_does_not_scroll_it() {
        let mut ui = Ui::natural();
        let list = ui.list_box_new();
        for name in ["a", "b", "c"] {
            ui.list_box_add(list, name).unwrap();
        }
        // not in a window yet, so it has no size
        ui.list_box_set_selected(list, 1).unwrap();
        let win = window(&mut ui, list);
        // the first row is still the first shown: a click on it selects it
        let events = click(&mut ui, win, 20, 10 + 2 + ROW_HEIGHT / 2);
        assert_eq!(numbers(&events, "ListBox.changed"), [0]);
        // a row further down than the list is tall is still brought into view
        let long = ui.list_box_new();
        for i in 0..30 {
            ui.list_box_add(long, &i.to_string()).unwrap();
        }
        ui.list_box_set_selected(long, 20).unwrap();
        let win = window(&mut ui, long);
        let events = click(&mut ui, win, 20, 10 + 2 + ROW_HEIGHT / 2);
        assert_eq!(numbers(&events, "ListBox.changed"), [16], "five rows fit, with the selected one last");
    }

    #[test]
    fn a_table_is_a_list_with_columns() {
        let mut ui = Ui::natural();
        let table = ui.table_new();
        ui.table_add_column(table, "Name", 120).unwrap();
        ui.table_add_column(table, "Size", 60).unwrap();
        ui.table_add_column(table, "Kind", 80).unwrap();
        for row in ["System\t198K\tsuitcase", "Finder\t46K\tapplication", "Note Pad File\t1K\tdocument"] {
            ui.list_box_add(table, row).unwrap();
        }
        let win = window(&mut ui, table);
        let r = ui.nodes.get(table).unwrap().rect;
        // as wide as its columns and scroll bar, with a row of headings on top
        assert_eq!((r.w, r.h), (120 + 60 + 80 + BAR + 4, 5 * ROW_HEIGHT + 4 + ROW_HEIGHT + 1));
        assert_eq!(ui.class_of(table).unwrap(), "Table");
        assert!(ui.is_a(table, "ListBox").unwrap());
        // a click on the headings selects nothing; one below them selects the first row
        assert!(click(&mut ui, win, r.x + 20, r.y + 8).is_empty());
        assert_eq!(numbers(&click(&mut ui, win, r.x + 20, r.y + 2 + ROW_HEIGHT + 1 + 4), "ListBox.changed"), [0]);
        // the keyboard and type-ahead work as in any list
        assert_eq!(numbers(&ui.key(win, Key::Down, NONE).unwrap(), "ListBox.changed"), [1]);
        assert_eq!(numbers(&ui.text(win, "n").unwrap(), "ListBox.changed"), [2]);
        assert_eq!(ui.list_label(table).as_deref(), Some("Note Pad File, 1K, document"));
        // columns belong to tables only
        let plain = ui.list_box_new();
        assert_eq!(ui.table_add_column(plain, "x", 50), Err(Error::WrongType));
        assert_eq!(ui.table_add_column(table, "x", 2), Err(Error::InvalidArgument));
        // a cell too long for its column is cut short, not spilled into the next
        ui.list_box_add(table, "A very long name indeed that goes on\t2K\tfolder").unwrap();
        let canvas = ui.render(win).unwrap();
        assert!(canvas.pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }

    /// A tree: Fruit (Apple, Citrus (Lemon, Lime)), Vegetables (Carrot).
    fn tree(ui: &mut Ui) -> (Handle, [i32; 7]) {
        let t = ui.tree_view_new();
        ui.list_box_add(t, "Fruit").unwrap(); // 0
        let apple = ui.tree_view_add_child(t, 0, "Apple").unwrap();
        let citrus = ui.tree_view_add_child(t, 0, "Citrus").unwrap();
        let lemon = ui.tree_view_add_child(t, citrus, "Lemon").unwrap();
        let lime = ui.tree_view_add_child(t, citrus, "Lime").unwrap();
        let veg = ui.tree_view_add_child(t, -1, "Vegetables").unwrap();
        let carrot = ui.tree_view_add_child(t, veg, "Carrot").unwrap();
        (t, [0, apple, citrus, lemon, lime, veg, carrot])
    }

    fn showing(ui: &Ui, t: Handle) -> Vec<usize> {
        ui.list_rows(t).iter().map(|r| r.item).collect()
    }

    #[test]
    fn a_tree_folds_and_unfolds() {
        let mut ui = Ui::new();
        let (t, [fruit, apple, citrus, lemon, _lime, veg, carrot]) = tree(&mut ui);
        let win = window(&mut ui, t);
        let r = ui.nodes.get(t).unwrap().rect;
        // folded to begin with: only the top level shows
        assert_eq!(showing(&ui, t), [fruit as usize, veg as usize]);
        // a click on the fold mark unfolds, and selects nothing
        assert!(click(&mut ui, win, r.x + 2 + 2 + 5, r.y + 2 + 6).is_empty());
        assert_eq!(showing(&ui, t), [0, 1, 2, 5]);
        assert_eq!(ui.list_box_selected(t).unwrap(), -1);
        // a click on the row's text selects it, by its place in the whole tree
        ui.tick(5_000);
        assert_eq!(numbers(&click(&mut ui, win, r.x + 60, r.y + 2 + 2 * ROW_HEIGHT + 6), "ListBox.changed"), [citrus]);
        // Right unfolds, then steps in; Left steps out, then folds
        ui.key(win, Key::Right, NONE).unwrap();
        assert_eq!(showing(&ui, t), [0, 1, 2, 3, 4, 5]);
        assert_eq!(numbers(&ui.key(win, Key::Right, NONE).unwrap(), "ListBox.changed"), [lemon]);
        assert_eq!(numbers(&ui.key(win, Key::Left, NONE).unwrap(), "ListBox.changed"), [citrus]);
        ui.key(win, Key::Left, NONE).unwrap();
        assert_eq!(showing(&ui, t), [0, 1, 2, 5]);
        // Down walks only what is showing
        assert_eq!(numbers(&ui.key(win, Key::Down, NONE).unwrap(), "ListBox.changed"), [veg]);
        // folding away the selected item's branch selects what it hung from
        ui.list_box_set_selected(t, lemon).unwrap(); // unfolds Citrus to show it
        assert_eq!(showing(&ui, t), [0, 1, 2, 3, 4, 5]);
        ui.tree_view_set_expanded(t, fruit, false).unwrap();
        assert_eq!((showing(&ui, t), ui.list_box_selected(t).unwrap()), (vec![0, 5], fruit));
        // rules
        assert_eq!(ui.tree_view_add_child(t, 99, "x"), Err(Error::InvalidArgument));
        assert_eq!(ui.tree_view_set_expanded(t, -1, true), Err(Error::InvalidArgument));
        let plain = ui.list_box_new();
        assert_eq!(ui.tree_view_add_child(plain, -1, "x"), Err(Error::WrongType));
        assert!(ui.is_a(t, "ListBox").unwrap() && ui.class_of(t).unwrap() == "TreeView");
        // Return opens the selected item
        ui.tree_view_set_expanded(t, veg, true).unwrap();
        ui.list_box_set_selected(t, carrot).unwrap();
        assert_eq!(numbers(&ui.key(win, Key::Return, NONE).unwrap(), "ListBox.activated"), [carrot]);
        let _ = apple;
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }

    const SHIFT: Mods = Mods { shift: true, ctrl: false, alt: false };
    const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false };

    fn fruit(ui: &mut Ui) -> (Handle, Handle, Rect) {
        let list = ui.list_box_new();
        for name in ["Apple", "Banana", "Cherry", "Date", "Elderberry"] {
            ui.list_box_add(list, name).unwrap();
        }
        let win = window(ui, list);
        let r = ui.nodes.get(list).unwrap().rect;
        (list, win, r)
    }

    fn picked(ui: &Ui, list: Handle) -> Vec<i32> {
        (0..5).filter(|i| ui.list_box_is_selected(list, *i).unwrap()).collect()
    }

    /// The middle of the row showing in a given place.
    fn row(r: Rect, place: i32) -> (i32, i32) {
        (r.x + 40, r.y + 2 + place * ROW_HEIGHT + 8)
    }

    #[test]
    fn several_rows_can_be_selected_with_control_and_shift() {
        let mut ui = Ui::new();
        let (list, win, r) = fruit(&mut ui);
        // until it is asked for, Control and Shift change nothing
        ui.set_modifiers(CTRL);
        click(&mut ui, win, row(r, 0).0, row(r, 0).1);
        click(&mut ui, win, row(r, 2).0, row(r, 2).1);
        assert_eq!(picked(&ui, list), [2]);
        ui.list_box_set_multiple(list, true).unwrap();
        // Control adds a row, and takes one away
        ui.tick(5_000);
        assert_eq!(numbers(&click(&mut ui, win, row(r, 0).0, row(r, 0).1), "ListBox.changed"), [0]);
        assert_eq!(picked(&ui, list), [0, 2]);
        ui.tick(10_000);
        click(&mut ui, win, row(r, 2).0, row(r, 2).1);
        assert_eq!(picked(&ui, list), [0]);
        // Shift stretches from where the selection last began
        ui.set_modifiers(SHIFT);
        click(&mut ui, win, row(r, 3).0, row(r, 3).1);
        assert_eq!((picked(&ui, list), ui.list_box_selected(list).unwrap()), (vec![2, 3], 3));
        // a plain click goes back to one
        ui.set_modifiers(NONE);
        ui.tick(15_000);
        click(&mut ui, win, row(r, 1).0, row(r, 1).1);
        assert_eq!(picked(&ui, list), [1]);
        // Shift with the arrows stretches too, and shrinks back
        ui.key(win, Key::Down, SHIFT).unwrap();
        ui.key(win, Key::Down, SHIFT).unwrap();
        assert_eq!(picked(&ui, list), [1, 2, 3]);
        ui.key(win, Key::Up, SHIFT).unwrap();
        assert_eq!(picked(&ui, list), [1, 2]);
        // Control+A takes everything; dragging stretches as well
        ui.key(win, Key::Char('a'), CTRL).unwrap();
        assert_eq!(picked(&ui, list), [0, 1, 2, 3, 4]);
        ui.tick(20_000);
        ui.pointer(win, row(r, 1).0, row(r, 1).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(r, 4).0, row(r, 4).1, PointerKind::Move).unwrap();
        ui.pointer(win, row(r, 4).0, row(r, 4).1, PointerKind::Release).unwrap();
        assert_eq!(picked(&ui, list), [1, 2, 3, 4]);
        // the program can set rows one at a time; turning it off keeps only the current one
        ui.list_box_set_item_selected(list, 0, true).unwrap();
        ui.list_box_set_item_selected(list, 2, false).unwrap();
        assert_eq!(picked(&ui, list), [0, 1, 3, 4]);
        ui.list_box_set_multiple(list, false).unwrap();
        assert_eq!(picked(&ui, list), [0]);
        assert_eq!(ui.list_box_is_selected(list, 9), Err(Error::InvalidArgument));
    }

    fn files(ui: &mut Ui) -> (Handle, Handle, Rect) {
        let table = ui.table_new();
        ui.table_add_column(table, "Name", 110).unwrap();
        ui.table_add_column(table, "Size", 60).unwrap();
        for row in ["System\t198K", "Finder\t46K", "alarm clock\t9K", "Note Pad\t1K"] {
            ui.list_box_add(table, row).unwrap();
        }
        let win = window(ui, table);
        let r = ui.nodes.get(table).unwrap().rect;
        (table, win, r)
    }

    #[test]
    fn a_table_sorts_by_a_heading_without_renumbering_its_rows() {
        let mut ui = Ui::new();
        let (table, win, r) = files(&mut ui);
        let order = |ui: &Ui| ui.list_rows(table).iter().map(|r| r.item).collect::<Vec<_>>();
        // clicking a heading does nothing until sorting is asked for
        click(&mut ui, win, r.x + 30, r.y + 8);
        assert_eq!(order(&ui), [0, 1, 2, 3]);
        ui.table_set_sortable(table, true).unwrap();
        // by name: without regard to case
        click(&mut ui, win, r.x + 30, r.y + 8);
        assert_eq!(order(&ui), [2, 1, 3, 0]);
        // the same heading again turns the order round
        click(&mut ui, win, r.x + 30, r.y + 8);
        assert_eq!(order(&ui), [0, 3, 1, 2]);
        // by size: as numbers, so 9K comes before 46K
        click(&mut ui, win, r.x + 2 + 110 + 20, r.y + 8);
        assert_eq!(order(&ui), [3, 2, 1, 0]);
        // rows keep their numbers: the row shown first is still item 3
        ui.tick(5_000);
        let first = (r.x + 30, r.y + 2 + ROW_HEIGHT + 1 + 8);
        assert_eq!(numbers(&click(&mut ui, win, first.0, first.1), "ListBox.changed"), [3]);
        assert_eq!(numbers(&ui.key(win, Key::Down, NONE).unwrap(), "ListBox.changed"), [2]);
        // from the program, and back to the order rows were added in
        ui.table_sort_by(table, 0, true).unwrap();
        assert_eq!(order(&ui), [2, 1, 3, 0]);
        ui.table_sort_by(table, -1, true).unwrap();
        assert_eq!(order(&ui), [0, 1, 2, 3]);
        assert_eq!(ui.table_sort_by(table, 2, true), Err(Error::InvalidArgument));
        let plain = ui.list_box_new();
        assert_eq!(ui.table_set_sortable(plain, true), Err(Error::WrongType));
    }

    #[test]
    fn a_column_is_resized_by_dragging_its_edge() {
        let mut ui = Ui::natural();
        let (table, win, r) = files(&mut ui);
        ui.table_set_sortable(table, true).unwrap();
        let edge = (r.x + 2 + 110, r.y + 8);
        // over the edge the pointer says so
        assert_eq!(ui.cursor_at(win, edge.0, edge.1), crate::cursor::Cursor::ResizeColumn);
        assert_eq!(ui.cursor_at(win, edge.0 - 40, edge.1), crate::cursor::Cursor::Arrow);
        ui.pointer(win, edge.0 + 1, edge.1, PointerKind::Press).unwrap();
        ui.pointer(win, edge.0 + 31, 400, PointerKind::Move).unwrap(); // the drag need not stay on the headings
        assert_eq!(ui.column_spans(table), [(0, 140), (140, 60)]);
        // it stops at a sensible least width
        ui.pointer(win, 0, 400, PointerKind::Move).unwrap();
        assert_eq!(ui.column_spans(table)[0].1, COLUMN_MIN);
        ui.pointer(win, edge.0 + 11, edge.1, PointerKind::Release).unwrap();
        assert_eq!(ui.column_spans(table), [(0, 120), (120, 60)]);
        // dragging an edge is not a click on a heading: nothing was sorted
        assert_eq!(ui.list_rows(table).iter().map(|r| r.item).collect::<Vec<_>>(), [0, 1, 2, 3]);
        // the table is as wide as its columns now need
        ui.render(win).unwrap();
        assert_eq!(ui.nodes.get(table).unwrap().rect.w, 120 + 60 + BAR + 4);
    }

    #[test]
    fn return_edits_a_row_in_place() {
        let mut ui = Ui::new();
        let (list, win, r) = fruit(&mut ui);
        ui.list_box_set_selected(list, 1).unwrap();
        ui.focus(list).unwrap();
        // not until it is allowed: Return opens the row
        assert_eq!(numbers(&ui.key(win, Key::Return, NONE).unwrap(), "ListBox.activated"), [1]);
        ui.list_box_set_editable(list, true).unwrap();
        assert!(ui.key(win, Key::Return, NONE).unwrap().is_empty());
        assert!(ui.list_editing(list));
        // the text starts selected, so typing replaces it; keys edit, they do not move the selection
        let plain = ui.render(win).unwrap().pixels;
        ui.text(win, "Blueberry").unwrap();
        ui.key(win, Key::Up, NONE).unwrap(); // to the start of the text, not the row above
        ui.key(win, Key::Delete, NONE).unwrap();
        ui.text(win, "b").unwrap();
        assert_ne!(ui.render(win).unwrap().pixels, plain);
        assert_eq!(ui.list_box_selected(list).unwrap(), 1);
        assert_eq!(ui.next_deadline(), Some(CARET_MS)); // a caret blinks in the cell
        // Return keeps it and reports the row, the column and the new text
        let ev = ui.key(win, Key::Return, NONE).unwrap();
        assert_eq!(ev, vec![Emitted { target: list, event: "ListBox.edited", args: vec![Value::I32(1), Value::I32(0), Value::Str("blueberry".into())] }]);
        assert!(!ui.list_editing(list));
        assert_eq!(ui.list_label(list).as_deref(), Some("blueberry"));
        // Escape throws an edit away
        ui.key(win, Key::Return, NONE).unwrap();
        ui.text(win, "gone").unwrap();
        assert!(ui.key(win, Key::Escape, NONE).unwrap().is_empty());
        assert_eq!(ui.list_label(list).as_deref(), Some("blueberry"));
        // an edit that changes nothing reports nothing
        ui.key(win, Key::Return, NONE).unwrap();
        assert!(ui.key(win, Key::Return, NONE).unwrap().is_empty());
        // a click on another row keeps the edit and then selects that row
        ui.key(win, Key::Return, NONE).unwrap();
        ui.text(win, "Bilberry").unwrap();
        let ev = click(&mut ui, win, row(r, 3).0, row(r, 3).1);
        assert_eq!(ev[0].event, "ListBox.edited");
        assert_eq!(numbers(&ev, "ListBox.changed"), [3]);
        // a click in the cell being edited places the caret and is not a click on the row
        ui.key(win, Key::Return, NONE).unwrap();
        assert!(click(&mut ui, win, r.x + 20, row(r, 3).1).is_empty());
        assert!(ui.list_editing(list));
        ui.key(win, Key::Escape, NONE).unwrap();
    }

    #[test]
    fn a_table_cell_is_edited_among_its_neighbours() {
        let mut ui = Ui::new();
        let (table, win, _) = files(&mut ui);
        ui.focus(table).unwrap();
        ui.list_box_edit(table, 1, 1).unwrap(); // Finder's size
        ui.text(win, "52K").unwrap();
        let ev = ui.key(win, Key::Return, NONE).unwrap();
        assert_eq!(ev[0].args, vec![Value::I32(1), Value::I32(1), Value::Str("52K".into())]);
        assert_eq!(ui.list_label(table).as_deref(), Some("Finder, 52K"));
        assert_eq!(ui.list_box_edit(table, 1, 2), Err(Error::InvalidArgument));
        assert_eq!(ui.list_box_edit(table, 7, 0), Err(Error::InvalidArgument));
        // destroying the table while a cell is open is safe
        ui.list_box_edit(table, 0, 0).unwrap();
        ui.destroy(table).unwrap();
        assert!(ui.key(win, Key::Return, NONE).unwrap().is_empty());
    }

    #[test]
    fn icons_sit_before_the_labels_and_line_them_up() {
        let mut ui = Ui::new();
        let (list, win, r) = fruit(&mut ui);
        let plain = ui.render(win).unwrap();
        ui.list_box_set_icon(list, 0, Icon::Folder).unwrap();
        ui.list_box_set_icon(list, 1, Icon::Document).unwrap();
        ui.list_box_set_icon(list, 2, Icon::Application).unwrap();
        let with = ui.render(win).unwrap();
        assert_ne!(with.pixels, plain.pixels);
        // every label moved over, the ones without an icon too, so they still line up
        let ink = |c: &Canvas, place: i32| (r.x + 2..r.x + 100).find(|x| (0..ROW_HEIGHT).any(|dy| c.get(*x, r.y + 2 + place * ROW_HEIGHT + dy) == BLACK));
        assert_eq!(ink(&with, 3).unwrap() - ink(&plain, 3).unwrap(), ICON);
        // each picture is different, and all are black and white
        let strip = |place: i32| (0..ROW_HEIGHT).flat_map(|dy| (0..ICON).map(move |dx| (dx, dy))).map(|(dx, dy)| with.get(r.x + 2 + 4 + dx, r.y + 2 + place * ROW_HEIGHT + dy)).collect::<Vec<_>>();
        assert!(strip(0) != strip(1) && strip(1) != strip(2) && strip(0) != strip(2));
        assert!(with.pixels.iter().all(|p| *p == BLACK || *p == WHITE));
        assert_eq!(ui.list_box_set_icon(list, 5, Icon::Folder), Err(Error::InvalidArgument));
    }

    #[test]
    fn a_program_can_draw_its_own_pictures() {
        let mut ui = Ui::new();
        let (list, win, r) = fruit(&mut ui);
        let plain = ui.render(win).unwrap();
        // a picture is drawn in text: X black, a full stop white, a space see-through
        ui.list_box_set_picture(list, 1, "XXXX\nX..X\nX..X\nXXXX").unwrap();
        let with = ui.render(win).unwrap();
        // it sits before the label, four pixels square, in the middle of the row's height
        let (x0, y0) = (r.x + 2 + 4, r.y + 2 + ROW_HEIGHT + (ROW_HEIGHT - 4) / 2);
        assert_eq!([with.get(x0, y0), with.get(x0 + 1, y0 + 1), with.get(x0 + 3, y0 + 3)], [BLACK, WHITE, BLACK]);
        // and makes room in every row, as a stock icon does
        assert_ne!(with.pixels, plain.pixels);
        // on a selected row the white parts stay white and the clear parts follow the row
        ui.list_box_set_picture(list, 1, "X X\n . ").unwrap();
        ui.list_box_set_selected(list, 1).unwrap();
        let sel = ui.render(win).unwrap();
        let y = r.y + 2 + ROW_HEIGHT + (ROW_HEIGHT - 2) / 2;
        // the whole row is inverted afterwards, picture and all, as the stock icons are
        assert_eq!([sel.get(x0, y), sel.get(x0 + 1, y), sel.get(x0 + 1, y + 1)], [WHITE, BLACK, BLACK]);
        // an empty string takes it away; too big is refused
        ui.list_box_set_picture(list, 1, "").unwrap();
        ui.list_box_set_selected(list, -1).unwrap();
        assert_eq!(ui.render(win).unwrap().pixels, plain.pixels);
        assert_eq!(ui.list_box_set_picture(list, 1, &"X".repeat(15)), Err(Error::InvalidArgument));
        assert_eq!(ui.list_box_set_picture(list, 1, &"X\n".repeat(15)), Err(Error::InvalidArgument));
        assert_eq!(ui.list_box_set_picture(list, 9, "X"), Err(Error::InvalidArgument));
    }

    #[test]
    fn a_cell_being_edited_has_the_clipboard_and_input_methods() {
        let mut ui = Ui::new();
        let (list, win, _) = fruit(&mut ui);
        ui.focus(list).unwrap();
        ui.list_box_edit(list, 0, 0).unwrap();
        // copy what is selected (all of it, to begin with), then cut it
        ui.key(win, Key::Char('c'), CTRL).unwrap();
        assert_eq!(ui.take_copy().as_deref(), Some("Apple"));
        ui.key(win, Key::Char('x'), CTRL).unwrap();
        assert_eq!(ui.take_copy().as_deref(), Some("Apple"));
        // paste is asked of the list, and answered into the cell
        ui.key(win, Key::Char('v'), CTRL).unwrap();
        assert_eq!(ui.take_paste_request(), Some(list));
        assert!(ui.paste_into(list, "Pine").unwrap().is_empty()); // nothing is reported until the edit is kept
        // an input method composes at the cell's caret
        let caret = ui.ime_target(win).expect("the cell can take an input method");
        let before = ui.render(win).unwrap().pixels;
        ui.ime_preedit(win, "app");
        assert_ne!(ui.render(win).unwrap().pixels, before);
        assert!(ui.ime_target(win).unwrap().x > caret.x); // the caret follows the draft
        ui.ime_commit(win, "apple", 0, 0).unwrap();
        ui.ime_commit(win, "", 1, 0).unwrap(); // and it can take a character back
        let ev = ui.key(win, Key::Return, NONE).unwrap();
        assert_eq!(ev[0].args[2], Value::Str("Pineappl".into()));
        // with no cell open, the list is not somewhere to compose or paste
        assert_eq!(ui.ime_target(win), None);
        assert_eq!(ui.paste_into(list, "x"), Err(Error::WrongType));
    }

    #[test]
    fn rows_can_be_dragged_to_a_new_place() {
        let mut ui = Ui::new();
        let (list, win, r) = fruit(&mut ui);
        let order = |ui: &Ui| ui.list_rows(list).iter().map(|r| r.item).collect::<Vec<_>>();
        // until it is allowed, a drag only moves the selection
        ui.pointer(win, row(r, 0).0, row(r, 0).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(r, 2).0, row(r, 2).1, PointerKind::Move).unwrap();
        ui.pointer(win, row(r, 2).0, row(r, 2).1, PointerKind::Release).unwrap();
        assert_eq!((order(&ui), ui.list_box_selected(list).unwrap()), (vec![0, 1, 2, 3, 4], 2));
        ui.list_box_set_reorderable(list, true).unwrap();
        // press Apple and pull it down past Cherry: a mark shows where it would land
        ui.tick(5_000);
        ui.pointer(win, row(r, 0).0, row(r, 0).1, PointerKind::Press).unwrap();
        let still = ui.render(win).unwrap().pixels;
        ui.pointer(win, row(r, 0).0, row(r, 0).1 + 2, PointerKind::Move).unwrap();
        assert_eq!(ui.render(win).unwrap().pixels, still); // a twitch is not a drag
        ui.pointer(win, row(r, 2).0, row(r, 2).1 + 6, PointerKind::Move).unwrap();
        assert_ne!(ui.render(win).unwrap().pixels, still);
        let ev = ui.pointer(win, row(r, 2).0, row(r, 2).1 + 6, PointerKind::Release).unwrap();
        // it reports the item, which keeps its number, and the place it now has
        assert_eq!(ev, vec![Emitted { target: list, event: "ListBox.moved", args: vec![Value::I32(0), Value::I32(2)] }]);
        assert_eq!(order(&ui), [1, 2, 0, 3, 4]);
        assert_eq!((ui.list_box_position_of(list, 0).unwrap(), ui.list_label(list).as_deref()), (2, Some("Apple")));
        // letting go where it came from moves nothing and says nothing
        ui.tick(10_000);
        ui.pointer(win, row(r, 2).0, row(r, 2).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(r, 3).0, row(r, 3).1, PointerKind::Move).unwrap();
        ui.pointer(win, row(r, 2).0, row(r, 2).1, PointerKind::Move).unwrap();
        assert!(ui.pointer(win, row(r, 2).0, row(r, 2).1, PointerKind::Release).unwrap().is_empty());
        // up to the very top, and from the program
        ui.tick(15_000);
        ui.pointer(win, row(r, 4).0, row(r, 4).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(r, 0).0, row(r, 0).1 - 6, PointerKind::Move).unwrap();
        let ev = ui.pointer(win, row(r, 0).0, row(r, 0).1 - 6, PointerKind::Release).unwrap();
        assert_eq!((numbers(&ev, "ListBox.moved"), order(&ui)), (vec![4], vec![4, 1, 2, 0, 3]));
        ui.list_box_move_item(list, 4, 4).unwrap();
        assert_eq!(order(&ui), [1, 2, 0, 3, 4]);
        assert_eq!(ui.list_box_move_item(list, 0, 5), Err(Error::InvalidArgument));
        // a row added later goes at the end
        ui.list_box_add(list, "Fig").unwrap();
        assert_eq!(order(&ui), [1, 2, 0, 3, 4, 5]);
        // moving a row of a sorted table starts from the order it was showing, and unsorts it
        let (table, _, _) = files(&mut ui);
        ui.table_sort_by(table, 0, true).unwrap(); // alarm clock, Finder, Note Pad, System
        ui.list_box_move_item(table, 0, 0).unwrap();
        assert_eq!(ui.list_rows(table).iter().map(|r| r.item).collect::<Vec<_>>(), [0, 2, 1, 3]);
        // a tree's rows can be dragged too, but not put somewhere by number
        let tree = ui.tree_view_new();
        ui.list_box_add(tree, "only").unwrap();
        ui.list_box_set_reorderable(tree, true).unwrap();
        assert_eq!(ui.list_box_move_item(tree, 0, 0), Err(Error::InvalidState));
    }

    #[test]
    fn rows_can_be_removed() {
        let mut ui = Ui::new();
        let (list, _win, _) = fruit(&mut ui);
        ui.list_box_move_item(list, 4, 0).unwrap(); // Elderberry to the top
        ui.list_box_set_selected(list, 3).unwrap(); // Date
        // those after the one removed move down an index; the order shown and the selection follow
        ui.list_box_remove(list, 1).unwrap(); // Banana
        assert_eq!(ui.list_rows(list).iter().map(|r| r.item).collect::<Vec<_>>(), [3, 0, 1, 2]);
        assert_eq!((ui.list_box_selected(list).unwrap(), ui.list_label(list).as_deref()), (2, Some("Date")));
        // removing the selected row leaves nothing selected
        ui.list_box_remove(list, 2).unwrap();
        assert_eq!(ui.list_box_selected(list).unwrap(), -1);
        assert_eq!(ui.list_box_remove(list, 3), Err(Error::InvalidArgument));
        // in a tree, what hangs from it goes too
        let (t, [_, _, citrus, _, _, veg, _]) = tree(&mut ui);
        ui.list_box_remove(t, citrus).unwrap(); // Citrus, Lemon, Lime
        let _ = veg;
        match &ui.nodes.get(t).unwrap().kind {
            Kind::List { items, meta, .. } => {
                assert_eq!(items, &["Fruit", "Apple", "Vegetables", "Carrot"]);
                assert_eq!(meta.iter().map(|m| m.parent).collect::<Vec<_>>(), [None, Some(0), None, Some(2)]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn several_selected_rows_move_together() {
        let mut ui = Ui::new();
        let (list, win, r) = fruit(&mut ui);
        ui.list_box_set_multiple(list, true).unwrap();
        ui.list_box_set_reorderable(list, true).unwrap();
        let order = |ui: &Ui| ui.list_rows(list).iter().map(|r| r.item).collect::<Vec<_>>();
        // select Apple and Cherry (not next to each other)
        ui.list_box_set_item_selected(list, 0, true).unwrap();
        ui.list_box_set_item_selected(list, 2, true).unwrap();
        // press one of them and pull to below Date: both go, keeping their order
        ui.pointer(win, row(r, 2).0, row(r, 2).1, PointerKind::Press).unwrap();
        assert_eq!(picked(&ui, list), [0, 2]); // pressing one of several does not let the others go
        ui.pointer(win, row(r, 3).0, row(r, 3).1 + 6, PointerKind::Move).unwrap();
        let ev = ui.pointer(win, row(r, 3).0, row(r, 3).1 + 6, PointerKind::Release).unwrap();
        assert_eq!(order(&ui), [1, 3, 0, 2, 4]);
        let moved: Vec<(i32, i32)> = ev.iter().filter(|e| e.event == "ListBox.moved").map(|e| (e.args[0].as_i32(), e.args[1].as_i32())).collect();
        assert_eq!(moved, [(0, 2), (2, 3)]);
        assert_eq!(picked(&ui, list), [0, 2]);
        // a plain click on one of them, with no drag, selects just that one
        ui.tick(5_000);
        let ev = click(&mut ui, win, row(r, 2).0, row(r, 2).1);
        assert_eq!((picked(&ui, list), numbers(&ev, "ListBox.changed")), (vec![0], vec![0]));
    }

    #[test]
    fn rows_can_be_dragged_from_one_list_to_another() {
        let mut ui = Ui::new();
        let (left, right) = (ui.list_box_new(), ui.list_box_new());
        for name in ["Apple", "Banana", "Cherry"] {
            ui.list_box_add(left, name).unwrap();
        }
        for name in ["Xigua", "Yuzu"] {
            ui.list_box_add(right, name).unwrap();
        }
        ui.list_box_set_icon(left, 1, Icon::Folder).unwrap();
        let both = ui.stack_new(Orientation::Horizontal);
        ui.stack_add(both, left).unwrap();
        ui.stack_add(both, right).unwrap();
        let win = window(&mut ui, both);
        let (l, r) = (ui.nodes.get(left).unwrap().rect, ui.nodes.get(right).unwrap().rect);
        let labels = |ui: &Ui, h: Handle| match &ui.nodes.get(h).unwrap().kind {
            Kind::List { items, .. } => ui.list_rows(h).iter().map(|row| items[row.item].clone()).collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        ui.list_box_set_reorderable(left, true).unwrap();
        // the other list has to be willing: until it is, the row just moves in its own list
        ui.pointer(win, row(l, 1).0, row(l, 1).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(r, 1).0, row(r, 1).1, PointerKind::Move).unwrap();
        ui.pointer(win, row(r, 1).0, row(r, 1).1, PointerKind::Release).unwrap();
        assert_eq!(labels(&ui, right), ["Xigua", "Yuzu"]);
        ui.list_box_set_accepts_rows(right, true).unwrap();
        ui.list_box_move_item(left, 1, 1).unwrap();
        // drag Banana across, into the gap between Xigua and Yuzu: the mark shows in the other list
        ui.tick(5_000);
        ui.pointer(win, row(l, 1).0, row(l, 1).1, PointerKind::Press).unwrap();
        let before = ui.render(win).unwrap();
        ui.pointer(win, row(r, 1).0, row(r, 1).1 - 6, PointerKind::Move).unwrap();
        let during = ui.render(win).unwrap();
        let changed = |x0: i32, x1: i32| (0..before.height).any(|y| (x0..x1).any(|x| before.get(x, y) != during.get(x, y)));
        assert!(changed(r.x + 2, r.x + 80) && !changed(l.x + 2, l.x + 80));
        let ev = ui.pointer(win, row(r, 1).0, row(r, 1).1 - 6, PointerKind::Release).unwrap();
        assert_eq!((labels(&ui, left), labels(&ui, right)), (vec!["Apple".to_string(), "Cherry".into()], vec!["Xigua".to_string(), "Banana".into(), "Yuzu".into()]));
        // the list it left says which index went; the list it joined says its new index and place
        assert_eq!(ev, vec![
            Emitted { target: left, event: "ListBox.removed", args: vec![Value::I32(1)] },
            Emitted { target: right, event: "ListBox.received", args: vec![Value::I32(2), Value::I32(1)] },
        ]);
        // it arrives selected, with its picture and the keyboard
        assert_eq!((ui.list_box_selected(right).unwrap(), ui.live_focus(win)), (2, Some(right)));
        match &ui.nodes.get(right).unwrap().kind {
            Kind::List { meta, .. } => assert_eq!(meta[2].icon, Icon::Folder),
            _ => unreachable!(),
        }
        // several at once: what is left goes across together, highest index reported first
        ui.list_box_set_multiple(left, true).unwrap();
        ui.list_box_set_item_selected(left, 0, true).unwrap();
        ui.list_box_set_item_selected(left, 1, true).unwrap();
        ui.tick(10_000);
        ui.pointer(win, row(l, 0).0, row(l, 0).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(r, 3).0, row(r, 3).1 + 6, PointerKind::Move).unwrap();
        let ev = ui.pointer(win, row(r, 3).0, row(r, 3).1 + 6, PointerKind::Release).unwrap();
        assert_eq!(numbers(&ev, "ListBox.removed"), [1, 0]);
        assert!(labels(&ui, left).is_empty());
        assert_eq!(labels(&ui, right), ["Xigua", "Banana", "Yuzu", "Apple", "Cherry"]);
        let _ = win;
    }

    #[test]
    fn a_tree_is_rearranged_by_dragging() {
        let mut ui = Ui::new();
        let (t, [fruit, apple, citrus, lemon, lime, veg, carrot]) = tree(&mut ui);
        ui.list_box_set_reorderable(t, true).unwrap();
        for item in [fruit, citrus, veg] {
            ui.tree_view_set_expanded(t, item, true).unwrap();
        }
        ui.list_box_set_rows(t, 8).unwrap();
        let win = window(&mut ui, t);
        let r = ui.nodes.get(t).unwrap().rect;
        // showing: Fruit, Apple, Citrus, Lemon, Lime, Vegetables, Carrot
        assert_eq!(showing(&ui, t), [0, 1, 2, 3, 4, 5, 6]);
        let at = |place: i32, part: i32| (r.x + 80, r.y + 2 + place * ROW_HEIGHT + part);
        let drag = |ui: &mut Ui, from: (i32, i32), to: (i32, i32)| {
            ui.tick(ui.now + 5_000);
            ui.pointer(win, from.0, from.1, PointerKind::Press).unwrap();
            ui.pointer(win, to.0, to.1, PointerKind::Move).unwrap();
            ui.pointer(win, to.0, to.1, PointerKind::Release).unwrap()
        };
        // drop Carrot onto Fruit (the middle of its row): it hangs from Fruit now, as its last
        let ev = drag(&mut ui, at(6, 8), at(0, 8));
        assert_eq!(ui.tree_view_parent_of(t, carrot).unwrap(), fruit);
        assert_eq!(showing(&ui, t), [0, 1, 2, 3, 4, 6, 5]);
        assert_eq!(ev, vec![Emitted { target: t, event: "ListBox.moved", args: vec![Value::I32(carrot), Value::I32(5)] }]);
        // drop Lime into the gap above Apple (the top edge of Apple's row): a sibling of Apple, before it
        drag(&mut ui, at(4, 8), at(1, 1));
        assert_eq!((ui.tree_view_parent_of(t, lime).unwrap(), showing(&ui, t)), (fruit, vec![0, 4, 1, 2, 3, 6, 5]));
        // a branch takes what hangs from it along: Citrus, with Lemon, to the very end at the top level
        drag(&mut ui, at(3, 8), at(7, 6));
        assert_eq!((ui.tree_view_parent_of(t, citrus).unwrap(), ui.tree_view_parent_of(t, lemon).unwrap()), (-1, citrus));
        assert_eq!(showing(&ui, t), [0, 4, 1, 6, 5, 2, 3]);
        // nothing can be dropped into itself or what hangs from it
        let before = showing(&ui, t);
        assert!(drag(&mut ui, at(5, 8), at(6, 8)).is_empty()); // Citrus onto its own Lemon
        assert!(drag(&mut ui, at(5, 8), at(5, 8 + 5)).is_empty());
        assert_eq!(showing(&ui, t), before);
        // dropping onto a folded item unfolds it so the arrival shows
        ui.tree_view_set_expanded(t, veg, false).unwrap();
        drag(&mut ui, at(2, 8), at(4, 8)); // Apple onto Vegetables
        assert_eq!(ui.tree_view_parent_of(t, apple).unwrap(), veg);
        assert!(ui.tree_view_expanded(t, veg).unwrap());
        // while it hovers, a box round the row says "onto" and a line says "between"
        ui.tick(ui.now + 5_000);
        ui.pointer(win, at(1, 8).0, at(1, 8).1, PointerKind::Press).unwrap();
        ui.pointer(win, at(0, 8).0, at(0, 8).1, PointerKind::Move).unwrap();
        let onto = ui.render(win).unwrap();
        ui.pointer(win, at(0, 1).0, at(0, 1).1, PointerKind::Move).unwrap();
        assert_ne!(ui.render(win).unwrap().pixels, onto.pixels);
        ui.row_drag_finish(false); // and a drag can be called off, leaving everything as it was
        assert_eq!(ui.tree_view_parent_of(t, lime).unwrap(), fruit);
        assert_eq!(ui.tree_view_parent_of(t, 99), Err(Error::InvalidArgument));
    }

    #[test]
    fn rows_can_cross_from_one_window_to_another() {
        let mut ui = Ui::new();
        let (left, right, lone) = (ui.list_box_new(), ui.list_box_new(), ui.list_box_new());
        for name in ["Apple", "Banana", "Cherry"] {
            ui.list_box_add(left, name).unwrap();
        }
        ui.list_box_add(right, "Xigua").unwrap();
        ui.list_box_set_reorderable(left, true).unwrap();
        ui.list_box_set_accepts_rows(right, true).unwrap();
        let win1 = window(&mut ui, left);
        let win2 = window(&mut ui, right);
        let win3 = window(&mut ui, lone); // a window whose list does not take rows
        let (l, r) = (ui.nodes.get(left).unwrap().rect, ui.nodes.get(right).unwrap().rect);
        let labels = |ui: &Ui, h: Handle| match &ui.nodes.get(h).unwrap().kind {
            Kind::List { items, .. } => ui.list_rows(h).iter().map(|row| items[row.item].clone()).collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        // pull Banana loose in the first window
        ui.pointer(win1, row(l, 1).0, row(l, 1).1, PointerKind::Press).unwrap();
        ui.pointer(win1, row(l, 2).0, row(l, 2).1, PointerKind::Move).unwrap();
        assert!(ui.row_dragging());
        assert_eq!(ui.row_drag_text(), "Banana"); // what a text field elsewhere would be given
        // the desktop now carries the drag: over a window that will not have it, there is nowhere to land
        assert!(!ui.row_drag_over(win3, Some((30, 30))));
        assert!(!ui.row_drag_over(win1, None));
        // over the second window's list there is, and the mark shows there
        let plain = ui.render(win2).unwrap().pixels;
        assert!(ui.row_drag_over(win2, Some((row(r, 0).0, row(r, 0).1 - 6))));
        assert_ne!(ui.render(win2).unwrap().pixels, plain);
        // dropped: it leaves one window's list and joins the other's, above Xigua
        let ev = ui.row_drag_finish(true);
        assert_eq!((labels(&ui, left), labels(&ui, right)), (vec!["Apple".to_string(), "Cherry".into()], vec!["Banana".to_string(), "Xigua".into()]));
        assert_eq!(ev.iter().map(|e| (e.target, e.event)).collect::<Vec<_>>(), [(left, "ListBox.removed"), (right, "ListBox.received")]);
        assert_eq!(ui.live_focus(win2), Some(right));
        assert!(!ui.row_dragging());
        // let go over nothing that would have it: nothing happens
        ui.tick(5_000);
        ui.pointer(win1, row(l, 0).0, row(l, 0).1, PointerKind::Press).unwrap();
        ui.pointer(win1, row(l, 1).0, row(l, 1).1 + 6, PointerKind::Move).unwrap();
        ui.row_drag_over(win3, Some((30, 30)));
        assert!(ui.row_drag_finish(true).is_empty());
        assert_eq!(labels(&ui, left), ["Apple", "Cherry"]);
        // and the list is ready for the next press: no drag is left hanging
        ui.tick(10_000);
        assert_eq!(numbers(&click(&mut ui, win1, row(l, 1).0, row(l, 1).1), "ListBox.changed"), [1]);
    }

    #[test]
    fn rows_go_into_a_tree_and_branches_come_out() {
        let mut ui = Ui::new();
        let (t, [fruit, _apple, citrus, _lemon, _lime, veg, _carrot]) = tree(&mut ui);
        let list = ui.list_box_new();
        for name in ["Kiwi", "Mango"] {
            ui.list_box_add(list, name).unwrap();
        }
        ui.list_box_set_icon(list, 0, Icon::Document).unwrap();
        for h in [t, list] {
            ui.list_box_set_reorderable(h, true).unwrap();
            ui.list_box_set_accepts_rows(h, true).unwrap();
            ui.list_box_set_rows(h, 8).unwrap();
        }
        ui.tree_view_set_expanded(t, fruit, true).unwrap();
        ui.tree_view_set_expanded(t, citrus, true).unwrap();
        let both = ui.stack_new(Orientation::Horizontal);
        ui.stack_add(both, t).unwrap();
        ui.stack_add(both, list).unwrap();
        let win = window(&mut ui, both);
        let (tr, lr) = (ui.nodes.get(t).unwrap().rect, ui.nodes.get(list).unwrap().rect);
        let drag = |ui: &mut Ui, from: (i32, i32), to: (i32, i32)| {
            ui.tick(ui.now + 5_000);
            ui.pointer(win, from.0, from.1, PointerKind::Press).unwrap();
            ui.pointer(win, to.0, to.1, PointerKind::Move).unwrap();
            ui.pointer(win, to.0, to.1, PointerKind::Release).unwrap()
        };
        // what a list holds, each row with how far in it sits
        let outline = |ui: &Ui, h: Handle| match &ui.nodes.get(h).unwrap().kind {
            Kind::List { items, .. } => ui.list_rows(h).iter().map(|r| format!("{}{}", "-".repeat(r.depth as usize), items[r.item])).collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        // the tree shows: Fruit, -Apple, -Citrus, --Lemon, --Lime, Vegetables
        // Kiwi from the list, dropped onto Vegetables: it hangs from it, which unfolds to show it
        let ev = drag(&mut ui, row(lr, 0), (tr.x + 80, tr.y + 2 + 5 * ROW_HEIGHT + 8));
        assert_eq!(outline(&ui, t), ["Fruit", "-Apple", "-Citrus", "--Lemon", "--Lime", "Vegetables", "-Carrot", "-Kiwi"]);
        assert_eq!(outline(&ui, list), ["Mango"]);
        let kiwi = 7; // the newest item in the tree
        assert_eq!((ui.tree_view_parent_of(t, kiwi).unwrap(), ui.list_box_selected(t).unwrap()), (veg, kiwi));
        assert_eq!(ev.iter().map(|e| e.event).collect::<Vec<_>>(), ["ListBox.removed", "ListBox.received"]);
        assert_eq!(ev[1].args, vec![Value::I32(kiwi), Value::I32(7)]);
        match &ui.nodes.get(t).unwrap().kind {
            Kind::List { meta, .. } => assert_eq!(meta[kiwi as usize].icon, Icon::Document), // its picture came too
            _ => unreachable!(),
        }
        // Mango into the gap above Apple: a sibling of Apple under Fruit
        drag(&mut ui, row(lr, 0), (tr.x + 80, tr.y + 2 + ROW_HEIGHT + 1));
        assert_eq!(outline(&ui, t)[..3], ["Fruit", "-Mango", "-Apple"]);
        assert!(outline(&ui, list).is_empty());
        // a branch dragged out to the plain list arrives whole, laid out flat
        let ev = drag(&mut ui, (tr.x + 80, tr.y + 2 + 3 * ROW_HEIGHT + 8), (lr.x + 40, lr.y + 20)); // Citrus
        assert_eq!(outline(&ui, list), ["Citrus", "Lemon", "Lime"]);
        assert_eq!(outline(&ui, t), ["Fruit", "-Mango", "-Apple", "Vegetables", "-Carrot", "-Kiwi"]);
        // three left the tree, highest index first, and three arrived
        let left: Vec<i32> = ev.iter().filter(|e| e.event == "ListBox.removed").map(|e| e.args[0].as_i32()).collect();
        assert!(left.len() == 3 && left.windows(2).all(|w| w[0] > w[1]), "{left:?}");
        assert_eq!(numbers(&ev, "ListBox.received"), [0, 1, 2]);
        // what is left in the tree still hangs together after the renumbering
        match &ui.nodes.get(t).unwrap().kind {
            Kind::List { items, meta, .. } => {
                for (label, m) in items.iter().zip(meta) {
                    let parent = m.parent.map(|p| items[p].as_str());
                    let expected = match label.as_str() {
                        "Apple" | "Mango" => Some("Fruit"),
                        "Carrot" | "Kiwi" => Some("Vegetables"),
                        _ => None,
                    };
                    assert_eq!(parent, expected, "{label}");
                }
            }
            _ => unreachable!(),
        }
        // from tree to tree a branch keeps its shape
        let other = ui.tree_view_new();
        ui.list_box_add(other, "Basket").unwrap();
        ui.list_box_set_accepts_rows(other, true).unwrap();
        ui.list_box_set_rows(other, 8).unwrap();
        ui.stack_add(both, other).unwrap();
        ui.render(win).unwrap();
        let (tr, or) = (ui.nodes.get(t).unwrap().rect, ui.nodes.get(other).unwrap().rect);
        drag(&mut ui, (tr.x + 80, tr.y + 2 + 8), (or.x + 60, or.y + 2 + 8)); // Fruit onto Basket
        // unfolded as it was when it left, so nothing that was showing is hidden by the move
        assert_eq!(outline(&ui, other), ["Basket", "-Fruit", "--Mango", "--Apple"]);
        let arrived = ui.list_box_selected(other).unwrap();
        assert_eq!(ui.tree_view_parent_of(other, arrived).unwrap(), 0);
        assert_eq!(outline(&ui, t), ["Vegetables", "-Carrot", "-Kiwi"]);
    }

    #[test]
    fn rows_can_be_handed_to_another_program() {
        // two engines, as two programs would have: nothing is shared but the text that passes between them
        let (mut a, mut b) = (Ui::new(), Ui::new());
        let (source, [fruit, _, citrus, ..]) = tree(&mut a);
        a.list_box_set_reorderable(source, true).unwrap();
        a.tree_view_set_expanded(source, fruit, true).unwrap();
        a.tree_view_set_expanded(source, citrus, true).unwrap();
        a.list_box_set_rows(source, 8).unwrap();
        a.list_box_set_icon(source, citrus, Icon::Folder).unwrap();
        a.list_box_set_picture(source, 3, "X.\n.X").unwrap(); // Lemon
        // a label with everything awkward in it: a tab, a line break, a backslash, the separator itself
        match &mut a.nodes.get_mut(source).unwrap().kind {
            Kind::List { items, .. } => items[4] = "Li\tme\\n \n\u{1f}!".to_string(),
            _ => unreachable!(),
        }
        let awkward = "Li\tme\\n \n\u{1f}!";
        let win_a = window(&mut a, source);
        let ra = a.nodes.get(source).unwrap().rect;
        let target = b.tree_view_new();
        b.list_box_add(target, "Basket").unwrap();
        b.list_box_set_accepts_rows(target, true).unwrap();
        b.list_box_set_rows(target, 8).unwrap();
        let win_b = window(&mut b, target);
        let rb = b.nodes.get(target).unwrap().rect;

        // pull Citrus loose in the first program; its branch is what is on offer
        a.pointer(win_a, ra.x + 80, ra.y + 2 + 2 * ROW_HEIGHT + 8, PointerKind::Press).unwrap();
        a.pointer(win_a, ra.x + 80, ra.y + 2 + 4 * ROW_HEIGHT, PointerKind::Move).unwrap();
        let payload = a.row_drag_payload();
        assert!(payload.starts_with("press-rows 1\n") && payload.lines().count() >= 4);
        // the second program knows nothing of it but that rows are over one of its windows
        assert!(!b.foreign_rows_over(win_b, Some((5, 150)))); // nowhere that takes rows
        let plain = b.render(win_b).unwrap().pixels;
        assert!(b.foreign_rows_over(win_b, Some((rb.x + 60, rb.y + 2 + 8)))); // onto Basket
        assert_ne!(b.render(win_b).unwrap().pixels, plain); // the mark shows
        let ev = b.foreign_rows_drop(&payload);
        // the branch arrived with its shape, its folds, its pictures and its awkward label
        match &b.nodes.get(target).unwrap().kind {
            Kind::List { items, meta, .. } => {
                assert_eq!(items, &["Basket", "Citrus", "Lemon", awkward]);
                assert_eq!(meta.iter().map(|m| m.parent).collect::<Vec<_>>(), [None, Some(0), Some(1), Some(1)]);
                assert!(meta[0].open && meta[1].open && meta[1].icon == Icon::Folder);
                assert_eq!(meta[2].art.as_deref(), Some(&vec![vec![1, 2], vec![2, 1]]));
            }
            _ => unreachable!(),
        }
        assert_eq!(numbers(&ev, "ListBox.received"), [1, 2, 3]);
        assert_eq!((b.list_box_selected(target).unwrap(), b.live_focus(win_b)), (1, Some(target)));
        // told it was taken, the first program lets the branch go
        let ev = a.row_drag_taken();
        assert_eq!(numbers(&ev, "ListBox.removed"), [4, 3, 2]);
        assert_eq!(showing(&a, source).len(), 3); // Fruit, Apple, Vegetables
        assert!(!a.row_dragging());

        // what arrives is not trusted: nonsense is left out, and nothing falls over
        for bad in ["", "something else", "press-rows 1\nnot a row", "press-rows 1\n9\u{1f}77\u{1f}x\u{1f}XXXXXXXXXXXXXXXXXXXX\u{1f}deep\\"] {
            assert!(b.foreign_rows_over(win_b, Some((rb.x + 60, rb.y + 2 + 8))));
            b.foreign_rows_drop(bad);
        }
        match &b.nodes.get(target).unwrap().kind {
            // only the last had a row in it: its depth brought down to earth, its picture refused, its label kept
            Kind::List { items, meta, .. } => {
                assert_eq!((items.len(), items[4].as_str()), (5, "deep"));
                assert_eq!((meta[4].parent, meta[4].icon, meta[4].art.is_none()), (Some(0), Icon::None, true));
            }
            _ => unreachable!(),
        }
        // a drop with nowhere marked does nothing, and leaving takes the mark away
        b.foreign_rows_over(win_b, None);
        assert!(b.foreign_rows_drop(&payload).is_empty());
        // a plain list sends and takes rows the same way, flat
        let (flat, flat_win, fr) = fruit_list(&mut b);
        b.list_box_set_accepts_rows(flat, true).unwrap();
        assert!(b.foreign_rows_over(flat_win, Some((fr.x + 40, fr.y + 2 + 4))));
        b.foreign_rows_drop(&payload);
        match &b.nodes.get(flat).unwrap().kind {
            Kind::List { items, .. } => assert_eq!(b.list_rows(flat).iter().map(|r| items[r.item].as_str()).take(3).collect::<Vec<_>>(), ["Citrus", "Lemon", awkward]),
            _ => unreachable!(),
        }
    }

    fn fruit_list(ui: &mut Ui) -> (Handle, Handle, Rect) {
        fruit(ui)
    }

    #[test]
    fn rows_that_stand_for_files_can_be_dragged_out_but_not_about() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Folder", 200, 160).unwrap();
        let list = ui.list_box_new();
        for name in ["a", "b", "c"] {
            ui.list_box_add(list, name).unwrap();
        }
        ui.window_set_content(win, list).unwrap();
        ui.render(win).unwrap();
        let row = |i: i32| (20, 2 + ROW_HEIGHT * i + ROW_HEIGHT / 2);
        // an ordinary row of a list that cannot be rearranged stays put
        ui.pointer(win, row(0).0, row(0).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(0).0, row(2).1, PointerKind::Move).unwrap();
        assert!(!ui.row_dragging());
        ui.pointer(win, row(0).0, row(2).1, PointerKind::Release).unwrap();
        // paths must be whole ones
        assert_eq!(ui.list_box_set_file(list, 0, "relative"), Err(Error::InvalidArgument));
        assert_eq!(ui.list_box_set_file(list, 9, "/x"), Err(Error::InvalidArgument));
        ui.list_box_set_file(list, 0, "/tmp/a file").unwrap();
        ui.list_box_set_file(list, 1, "/tmp/b").unwrap();
        // one that stands for a file comes loose, offering the file
        ui.tick(5_000);
        ui.pointer(win, row(0).0, row(0).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(0).0, row(2).1, PointerKind::Move).unwrap();
        assert!(ui.row_dragging());
        assert_eq!(ui.row_drag_files(), ["/tmp/a file"]);
        // and has a picture to travel under the pointer: one row, clear at the corners
        let picture = ui.row_drag_picture().unwrap();
        assert_eq!(picture.height, ROW_HEIGHT + 2);
        assert_eq!(picture.get(0, 0), crate::canvas::TRANSPARENT);
        assert!(picture.pixels.contains(&BLACK) && picture.pixels.contains(&WHITE));
        // but it has no place among its own rows: let go there, nothing moves
        assert!(!ui.row_drag_over(win, Some(row(2))));
        let events = ui.pointer(win, row(0).0, row(2).1, PointerKind::Release).unwrap();
        assert!(events.iter().all(|e| e.event != "ListBox.moved"), "{events:?}");
        assert_eq!(ui.list_box_position_of(list, 0).unwrap(), 0);
        assert!(!ui.row_dragging());
        // several selected rows go together, but only if every one is a file
        ui.list_box_set_multiple(list, true).unwrap();
        ui.tick(10_000);
        ui.pointer(win, row(0).0, row(0).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(0).0, row(0).1, PointerKind::Release).unwrap();
        ui.set_modifiers(Mods { shift: true, ..Mods::default() });
        ui.pointer(win, row(0).0, row(1).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(0).0, row(1).1, PointerKind::Release).unwrap();
        ui.set_modifiers(Mods::default());
        ui.tick(15_000);
        ui.pointer(win, row(0).0, row(1).1, PointerKind::Press).unwrap();
        ui.pointer(win, row(0).0 + 60, row(1).1, PointerKind::Move).unwrap();
        assert_eq!(ui.row_drag_files(), ["/tmp/a file", "/tmp/b"]);
        assert_eq!(ui.row_drag_picture().unwrap().height, 2 * ROW_HEIGHT + 2);
        ui.row_drag_finish(false);
        assert!(ui.row_drag_picture().is_none());
        ui.list_box_set_file(list, 1, "").unwrap();
        assert_eq!(ui.list_box_set_file(list, 1, ""), Ok(()));
    }

    #[test]
    fn an_icon_view_lays_its_items_out_in_rows_and_is_still_a_list() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Folder", 320, 140).unwrap();
        let view = ui.icon_view_new();
        assert!(ui.is_a(view, "ListBox").unwrap() && ui.is_a(view, "IconView").unwrap() && !ui.is_a(view, "Table").unwrap());
        // three across at its natural width, three rows high
        assert_eq!(ui.measure_list(view), (3 * CELL.0 + BAR + 4, 3 * CELL.1 + 4));
        for (i, name) in ["Letters", "Pictures", "memo\t1K\tdocument", "plan", "tool", "x", "y"].iter().enumerate() {
            ui.list_box_add(view, name).unwrap();
            ui.list_box_set_icon(view, i as i32, [Icon::Folder, Icon::Document, Icon::Application][i % 3]).unwrap();
        }
        ui.list_box_set_editable(view, true).unwrap();
        ui.list_box_set_file(view, 2, "/tmp/memo").unwrap();
        ui.window_set_content(win, view).unwrap();
        let canvas = ui.render(win).unwrap();
        assert!(canvas.pixels.iter().all(|p| *p == BLACK || *p == WHITE));
        // the window is 320 wide: three icons fit across beside the scroll bar, so seven make three rows, of which two fit
        assert_eq!((ui.icons_across(view), ui.list_shape(view).map(|s| (s.1, s.2))), (3, Some((2, 3))));
        let cell = |i: i32| (2 + (i % 3) * CELL.0 + CELL.0 / 2, 2 + (i / 3) * CELL.1 + 20);
        // a click picks the icon under it; past the last in a row, nothing
        assert_eq!(numbers(&click(&mut ui, win, cell(1).0, cell(1).1), "ListBox.changed"), [1]);
        assert_eq!(ui.list_item_at(view, cell(4).0, cell(4).1), Some((4, false)));
        assert_eq!(ui.list_item_at(view, 2 + 3 * CELL.0 + 4, cell(0).1), None);
        // the arrows go across and down, and stop at the ends
        let go = |ui: &mut Ui, key| ui.key(win, key, NONE).map(|_| ui.list_box_selected(view).unwrap()).unwrap();
        assert_eq!([go(&mut ui, Key::Right), go(&mut ui, Key::Down), go(&mut ui, Key::Down), go(&mut ui, Key::Left), go(&mut ui, Key::Up)], [2, 5, 5, 4, 1]);
        // going down past what shows scrolls a row of icons into view
        assert_eq!([go(&mut ui, Key::End), go(&mut ui, Key::Home)], [6, 0]);
        ui.key(win, Key::End, NONE).unwrap();
        assert!(matches!(ui.nodes.get(view).map(|n| &n.kind), Some(Kind::List { scroll: 1, .. })));
        // a name is changed under its icon, and the change reported as for any list
        ui.list_box_set_selected(view, 1).unwrap();
        ui.list_box_edit(view, 1, 0).unwrap();
        ui.text(win, "Drawings").unwrap();
        assert!(ui.list_edit_caret(view).is_some());
        ui.render(win).unwrap();
        let events = ui.key(win, Key::Return, NONE).unwrap();
        assert_eq!(events, vec![Emitted { target: view, event: "ListBox.edited", args: vec![Value::I32(1), Value::I32(0), Value::Str("Drawings".into())] }]);
        // an icon that stands for a file can be dragged out, with only its name on the picture that goes along
        ui.tick(9_000);
        ui.list_box_set_selected(view, 0).unwrap();
        ui.render(win).unwrap();
        ui.pointer(win, cell(2).0, cell(2).1, PointerKind::Press).unwrap();
        ui.pointer(win, cell(2).0 + 40, cell(2).1 + 30, PointerKind::Move).unwrap();
        assert_eq!((ui.row_drag_files(), ui.row_drag_text()), (vec!["/tmp/memo".to_string()], "memo\t1K\tdocument".to_string()));
        assert!(ui.row_drag_picture().is_some_and(|p| p.height == ROW_HEIGHT + 2));
        ui.row_drag_finish(false);
    }

    #[test]
    fn icons_can_be_put_where_the_user_likes() {
        let mut ui = Ui::natural();
        let win = ui.window_new("Folder", 320, 140).unwrap();
        let view = ui.icon_view_new();
        for name in ["a", "b", "c", "d"] {
            ui.list_box_add(view, name).unwrap();
        }
        ui.window_set_content(win, view).unwrap();
        ui.render(win).unwrap();
        let middle = |cell: (i32, i32)| (2 + cell.0 + CELL.0 / 2, 2 + cell.1 + 20);
        let drag = |ui: &mut Ui, from: (i32, i32), to: (i32, i32)| {
            ui.tick(ui.now + 5_000);
            ui.pointer(win, from.0, from.1, PointerKind::Press).unwrap();
            ui.pointer(win, (from.0 + to.0) / 2, (from.1 + to.1) / 2, PointerKind::Move).unwrap();
            ui.pointer(win, to.0, to.1, PointerKind::Move).unwrap();
            ui.pointer(win, to.0, to.1, PointerKind::Release).unwrap()
        };
        // until the view says they may be, icons stay in line
        let from = middle((CELL.0, 0));
        assert!(drag(&mut ui, from, (from.0 + 30, from.1 + 50)).iter().all(|e| e.event != "IconView.placed"));
        assert_eq!(ui.list_item_at(view, from.0, from.1), Some((1, false)));
        // then one dragged goes just as far as the pointer took it, and the program is told where
        ui.list_box_set_reorderable(view, true).unwrap();
        let events = drag(&mut ui, from, (from.0 + 30, from.1 + 50));
        let placed: Vec<_> = events.iter().filter(|e| e.event == "IconView.placed").collect();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].args, vec![Value::I32(1), Value::I32(CELL.0 + 30), Value::I32(50)]);
        assert_eq!(ui.list_item_at(view, from.0, from.1), None, "its old place is empty");
        assert_eq!(ui.list_item_at(view, from.0 + 30, from.1 + 50), Some((1, false)));
        // the others keep their places in line, and the order of items has not changed
        assert_eq!(ui.list_item_at(view, middle((2 * CELL.0, 0)).0, middle((0, 0)).1), Some((2, false)));
        assert_eq!(ui.list_box_position_of(view, 1).unwrap(), 1);
        // the program can put one somewhere itself, lower than the view shows, which makes more to scroll to
        let before = ui.list_shape(view).unwrap().2;
        ui.icon_view_set_position(view, 3, 10, 5 * CELL.1).unwrap();
        assert!(ui.list_shape(view).unwrap().2 > before);
        ui.list_box_set_selected(view, 3).unwrap();
        ui.render(win).unwrap();
        assert!(matches!(ui.nodes.get(view).map(|n| &n.kind), Some(Kind::List { scroll, .. }) if *scroll > 0), "scrolled to show it");
        // and back in line again
        ui.icon_view_set_position(view, 3, -1, -1).unwrap();
        ui.icon_view_set_position(view, 1, -1, -1).unwrap();
        ui.list_box_set_selected(view, 0).unwrap();
        ui.render(win).unwrap();
        assert_eq!(ui.list_item_at(view, from.0, from.1), Some((1, false)));
        // let go outside the view, an icon is not put anywhere
        let events = drag(&mut ui, from, (from.0, 400));
        assert!(events.iter().all(|e| e.event != "IconView.placed"));
        assert_eq!(ui.list_item_at(view, from.0, from.1), Some((1, false)));
        // only an icon view has places to put things
        let list = ui.list_box_new();
        assert_eq!(ui.icon_view_set_position(list, 0, 1, 1), Err(Error::WrongType));
        assert_eq!(ui.icon_view_set_position(view, 9, 1, 1), Err(Error::InvalidArgument));
    }
}
