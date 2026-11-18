// SPDX-License-Identifier: GPL-3.0-or-later
//! The progress bar and the tab view.
//!
//! A progress bar fills from the left as work is done, or, when there is no
//! telling how long the work will take, shows the slanted stripes of a
//! barber's pole on the move. A tab view holds several pages of widgets in
//! the space of one, with a row of tabs along the top to choose between them.

use super::*;

const STRIPE_MS: u64 = 60;
const TAB_HEIGHT: i32 = crate::style::TAB_HEIGHT;
const TAB_PAD: i32 = crate::style::TAB_PAD;
const TAB_INDENT: i32 = 6;
const PAGE_PAD: i32 = 8;

impl Ui {
    // ---------------------------------------------------------- progress bars

    pub fn progress_bar_new(&mut self) -> Handle {
        self.add(Kind::Progress { value: 0, max: 100, busy: false, length: 160 })
    }

    pub fn progress_bar_value(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Progress { value, .. } => Ok(*value),
            _ => Err(Error::WrongType),
        }
    }

    pub fn progress_bar_max(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Progress { max, .. } => Ok(*max),
            _ => Err(Error::WrongType),
        }
    }

    /// Set how much is done; it is brought into range.
    pub fn progress_bar_set_value(&mut self, h: Handle, to: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Progress { value, max, .. } => {
                *value = to.clamp(0, *max);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn progress_bar_set_max(&mut self, h: Handle, to: i32) -> Result<()> {
        if to < 1 {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Progress { value, max, .. } => {
                *max = to;
                *value = (*value).min(to);
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// Show moving stripes in place of an amount, for work of unknown length.
    pub fn progress_bar_set_busy(&mut self, h: Handle, on: bool) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Progress { busy, .. } => {
                *busy = on;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub fn progress_bar_set_length(&mut self, h: Handle, to: i32) -> Result<()> {
        if !(16..=16384).contains(&to) {
            return Err(Error::InvalidArgument);
        }
        match &mut self.node_mut(h)?.kind {
            Kind::Progress { length, .. } => {
                *length = to;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    pub(super) fn measure_progress(&self, h: Handle) -> (i32, i32) {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Progress { length, .. }) => (*length, self.look.m.progress_height),
            _ => (0, 0),
        }
    }

    /// How far the stripes have moved.
    fn stripe_phase(&self) -> i32 {
        ((self.now / STRIPE_MS) % 8) as i32
    }

    pub(super) fn paint_progress(&mut self, h: Handle, rect: Rect, canvas: &mut Canvas) {
        let Some(Kind::Progress { value, max, busy, .. }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (value, max, busy) = (*value, (*max).max(1), *busy);
        let part = Part::new(PartKind::Progress, rect).numbers(value, max, 0).phase(self.stripe_phase());
        self.part(canvas, part.when(state::BUSY, busy));
    }

    /// Whether any progress bar is showing moving stripes.
    fn any_busy(&self) -> bool {
        self.nodes.handles().into_iter().any(|h| matches!(self.nodes.get(h).map(|n| &n.kind), Some(Kind::Progress { busy: true, .. })))
    }

    /// Keep the stripes moving.
    pub(super) fn progress_tick(&mut self) {
        if self.any_busy() {
            let phase = self.stripe_phase();
            if phase != self.stripes {
                self.stripes = phase;
                self.dirty = true;
            }
        }
    }

    pub(super) fn progress_deadline(&self) -> Option<u64> {
        self.any_busy().then(|| (self.now / STRIPE_MS + 1) * STRIPE_MS)
    }

    // ---------------------------------------------------------------- tab views

    pub fn tab_view_new(&mut self) -> Handle {
        self.add(Kind::Tabs { pages: Vec::new(), selected: 0 })
    }

    /// Add a page: the word on its tab, and the widget shown when it is chosen.
    pub fn tab_view_add(&mut self, tabs: Handle, title: &str, content: Handle) -> Result<()> {
        if !matches!(self.node(tabs)?.kind, Kind::Tabs { .. }) {
            return Err(Error::WrongType);
        }
        self.adoptable(tabs, content)?;
        if let Kind::Tabs { pages, .. } = &mut self.node_mut(tabs)?.kind {
            pages.push((title.to_string(), content));
        }
        self.node_mut(content)?.parent = Some(tabs);
        Ok(())
    }

    /// The index of the page showing, or -1 if there are no pages.
    pub fn tab_view_selected(&self, h: Handle) -> Result<i32> {
        match &self.node(h)?.kind {
            Kind::Tabs { pages, selected } => Ok(if pages.is_empty() { -1 } else { *selected as i32 }),
            _ => Err(Error::WrongType),
        }
    }

    /// Show a page from the program. Nothing is emitted.
    pub fn tab_view_set_selected(&mut self, h: Handle, index: i32) -> Result<()> {
        match &mut self.node_mut(h)?.kind {
            Kind::Tabs { pages, selected } => {
                if index < 0 || index as usize >= pages.len() {
                    return Err(Error::InvalidArgument);
                }
                *selected = index as usize;
                Ok(())
            }
            _ => Err(Error::WrongType),
        }
    }

    /// The page that is showing.
    pub(super) fn tab_page(&self, h: Handle) -> Option<Handle> {
        match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Tabs { pages, selected }) => pages.get(*selected).map(|p| p.1),
            _ => None,
        }
    }

    /// Whether a widget is on a page of some tab view that is not showing.
    pub(super) fn on_hidden_page(&self, h: Handle) -> bool {
        let mut child = h;
        while let Some(parent) = self.nodes.get(child).and_then(|n| n.parent) {
            if matches!(self.nodes.get(parent).map(|n| &n.kind), Some(Kind::Tabs { .. })) && self.tab_page(parent) != Some(child) {
                return true;
            }
            child = parent;
        }
        false
    }

    /// Each tab's area along the top.
    fn tab_rects(&mut self, h: Handle) -> Vec<Rect> {
        let Some(node) = self.nodes.get(h) else { return Vec::new() };
        let Kind::Tabs { pages, .. } = &node.kind else { return Vec::new() };
        let (rect, titles) = (node.rect, pages.iter().map(|p| p.0.clone()).collect::<Vec<_>>());
        let mut x = rect.x + TAB_INDENT;
        titles
            .iter()
            .map(|title| {
                let w = self.font.width(title) + 2 * TAB_PAD;
                let r = Rect::new(x, rect.y, w, TAB_HEIGHT);
                x += w - 1; // neighbours share an edge
                r
            })
            .collect()
    }

    /// The framed area the pages show in.
    fn tab_pane(rect: Rect) -> Rect {
        Rect::new(rect.x, rect.y + TAB_HEIGHT - 1, rect.w, (rect.h - TAB_HEIGHT + 1).max(0))
    }

    pub(super) fn measure_tabs(&mut self, h: Handle) -> (i32, i32) {
        let pages: Vec<Handle> = match self.nodes.get(h).map(|n| &n.kind) {
            Some(Kind::Tabs { pages, .. }) => pages.iter().map(|p| p.1).collect(),
            _ => Vec::new(),
        };
        // big enough for the largest page, so the view does not change size as pages turn
        let (mut w, mut tall) = (0, 0);
        for page in pages {
            let (pw, ph) = self.measure(page);
            w = w.max(pw);
            tall = tall.max(ph);
        }
        let tabs = self.tab_rects(h).last().map_or(0, |r| r.x + r.w) - self.nodes.get(h).map_or(0, |n| n.rect.x);
        ((w + 2 * PAGE_PAD + 2).max(tabs + TAB_INDENT), tall + 2 * PAGE_PAD + 1 + TAB_HEIGHT)
    }

    pub(super) fn arrange_tabs(&mut self, h: Handle, rect: Rect) {
        if let Some(page) = self.tab_page(h) {
            self.arrange(page, Self::tab_pane(rect).inset(1 + PAGE_PAD));
        }
    }

    pub(super) fn paint_tabs(&mut self, h: Handle, rect: Rect, live: bool, enabled: bool, ctx: Paint, canvas: &mut Canvas) {
        let Some(Kind::Tabs { pages, selected }) = self.nodes.get(h).map(|n| &n.kind) else { return };
        let (titles, selected) = (pages.iter().map(|p| p.0.clone()).collect::<Vec<_>>(), *selected);
        let (tabs, pane, phase) = (self.tab_rects(h), Self::tab_pane(rect), self.ants_step_public());
        // the tabs behind first, then the pane over their feet, then the chosen
        // tab, which the style joins to the pane
        for i in (0..tabs.len()).filter(|i| *i != selected) {
            self.part(canvas, Part::new(PartKind::Tab, tabs[i]).text(&titles[i]));
        }
        self.part(canvas, Part::new(PartKind::TabPane, pane));
        if let Some(r) = tabs.get(selected).copied() {
            let part = Part::new(PartKind::Tab, r).text(&titles[selected]).phase(phase);
            self.part(canvas, part.when(state::FRONT, true).when(state::FOCUSED, live));
        }
        if let Some(page) = self.tab_page(h) {
            self.paint(page, canvas, enabled, ctx);
        }
    }

    /// What is under a point in a tab view: the view itself along its row of
    /// tabs, otherwise whatever is there on the page that is showing.
    pub(super) fn hit_tabs(&self, h: Handle, x: i32, y: i32) -> Option<Handle> {
        let rect = self.nodes.get(h)?.rect;
        if y < rect.y + TAB_HEIGHT {
            return Some(h);
        }
        self.tab_page(h).and_then(|page| self.hit(page, x, y))
    }

    /// Turn to a page because the user chose it.
    fn tab_turn(&mut self, h: Handle, index: usize) -> Vec<Emitted> {
        match self.nodes.get_mut(h).map(|n| &mut n.kind) {
            Some(Kind::Tabs { pages, selected }) if index < pages.len() && *selected != index => {
                *selected = index;
                self.dirty = true;
                vec![Emitted { target: h, event: "TabView.changed", args: vec![Value::I32(index as i32)] }]
            }
            _ => Vec::new(),
        }
    }

    /// The pointer went down on a tab view's row of tabs.
    pub(super) fn tabs_press(&mut self, h: Handle, x: i32, y: i32) -> Vec<Emitted> {
        match self.tab_rects(h).iter().position(|r| r.contains(x, y)) {
            Some(index) => self.tab_turn(h, index),
            None => Vec::new(),
        }
    }

    /// Left and Right turn the pages of a focused tab view.
    pub(super) fn tabs_key(&mut self, h: Handle, key: Key) -> Option<Vec<Emitted>> {
        let Some(Kind::Tabs { pages, selected }) = self.nodes.get(h).map(|n| &n.kind) else { return None };
        let (count, selected) = (pages.len(), *selected);
        let to = match key {
            Key::Right if count > 0 => (selected + 1) % count,
            Key::Left if count > 0 => (selected + count - 1) % count,
            _ => return None,
        };
        Some(self.tab_turn(h, to))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods { shift: false, ctrl: false, alt: false };

    fn window(ui: &mut Ui, content: Handle) -> Handle {
        let win = ui.window_new("More", 280, 200).unwrap();
        let col = ui.stack_new(Orientation::Vertical);
        ui.stack_set_padding(col, 10).unwrap();
        ui.stack_add(col, content).unwrap();
        ui.window_set_content(win, col).unwrap();
        ui.render(win).unwrap();
        win
    }

    fn black(canvas: &Canvas, r: Rect) -> i32 {
        (r.y..r.y + r.h).map(|y| (r.x..r.x + r.w).filter(|x| canvas.get(*x, y) == BLACK).count() as i32).sum()
    }

    #[test]
    fn a_progress_bar_fills_and_a_busy_one_moves() {
        let mut ui = Ui::natural();
        let bar = ui.progress_bar_new();
        ui.progress_bar_set_length(bar, 102).unwrap(); // 100 pixels inside the frame
        let win = window(&mut ui, bar);
        let inner = ui.nodes.get(bar).unwrap().rect.inset(1);
        assert_eq!(black(&ui.render(win).unwrap(), inner), 0);
        ui.progress_bar_set_value(bar, 25).unwrap();
        assert_eq!(black(&ui.render(win).unwrap(), inner), 25 * inner.h);
        ui.progress_bar_set_value(bar, 400).unwrap();
        assert_eq!((ui.progress_bar_value(bar).unwrap(), black(&ui.render(win).unwrap(), inner)), (100, 100 * inner.h));
        // nothing to animate, so the clock is not wanted
        assert_eq!(ui.next_deadline(), None);
        assert_eq!(ui.focus(bar), Err(Error::WrongType)); // it is only for looking at
        // busy: half black in stripes, and it asks to be redrawn as they move
        ui.progress_bar_set_busy(bar, true).unwrap();
        let first = ui.render(win).unwrap();
        assert!((black(&first, inner) - inner.w * inner.h / 2).abs() <= inner.h); // near enough half
        assert_eq!(ui.next_deadline(), Some(STRIPE_MS));
        ui.take_dirty();
        ui.tick(STRIPE_MS);
        assert!(ui.take_dirty());
        assert_ne!(ui.render(win).unwrap().pixels, first.pixels);
        ui.progress_bar_set_busy(bar, false).unwrap();
        assert_eq!(ui.next_deadline(), None);
        assert_eq!(ui.progress_bar_set_max(bar, 0), Err(Error::InvalidArgument));
    }

    #[test]
    fn a_tab_view_shows_one_page_at_a_time() {
        let mut ui = Ui::new();
        let tabs = ui.tab_view_new();
        let (first, second) = (ui.checkbox_new("On the first page"), ui.button_new("On the second"));
        ui.tab_view_add(tabs, "General", first).unwrap();
        ui.tab_view_add(tabs, "Advanced", second).unwrap();
        let win = window(&mut ui, tabs);
        let r = ui.nodes.get(tabs).unwrap().rect;
        assert_eq!(ui.tab_view_selected(tabs).unwrap(), 0);
        // the first page's checkbox can be clicked; the second page's button is not there
        let at = ui.nodes.get(first).unwrap().rect;
        ui.pointer(win, at.x + 4, at.y + 4, PointerKind::Press).unwrap();
        assert_eq!(ui.pointer(win, at.x + 4, at.y + 4, PointerKind::Release).unwrap()[0].event, "Checkbox.toggled");
        assert_eq!(ui.focus(second), Err(Error::InvalidState)); // hidden: it cannot take the keyboard
        // a click on the second tab turns the page and says so
        let tab2 = ui.tab_rects(tabs)[1];
        let ev = ui.pointer(win, tab2.x + 8, tab2.y + 8, PointerKind::Press).unwrap();
        ui.pointer(win, tab2.x + 8, tab2.y + 8, PointerKind::Release).unwrap();
        assert_eq!(ev, vec![Emitted { target: tabs, event: "TabView.changed", args: vec![Value::I32(1)] }]);
        ui.render(win).unwrap();
        let at = ui.nodes.get(second).unwrap().rect;
        assert!(at.y > r.y + TAB_HEIGHT && at.x > r.x); // laid out inside the pane
        ui.pointer(win, at.x + 4, at.y + 4, PointerKind::Press).unwrap();
        assert_eq!(ui.pointer(win, at.x + 4, at.y + 4, PointerKind::Release).unwrap()[0].event, "Button.clicked");
        // Tab reaches the row of tabs, then what is on the page showing, and only that
        ui.focus(tabs).unwrap();
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(second));
        ui.key(win, Key::Tab, NONE).unwrap();
        assert_eq!(ui.live_focus(win), Some(tabs));
        // Left and Right turn the pages, and wrap
        assert_eq!(ui.key(win, Key::Right, NONE).unwrap()[0].args, vec![Value::I32(0)]);
        assert_eq!(ui.key(win, Key::Left, NONE).unwrap()[0].args, vec![Value::I32(1)]);
        // it is as big as its biggest page, whichever is showing
        let size = ui.measure_tabs(tabs);
        ui.tab_view_set_selected(tabs, 0).unwrap();
        assert_eq!(ui.measure_tabs(tabs), size);
        assert_eq!(ui.tab_view_set_selected(tabs, 2), Err(Error::InvalidArgument));
        // assistive technology is told the page showing, and sees only that page
        let tree = ui.accessible(win).unwrap();
        let node = tree.nodes.iter().find(|n| n.handle == tabs).unwrap();
        assert_eq!((node.value.as_deref(), node.children.clone()), (Some("General"), vec![first]));
        // destroying it takes every page with it, showing or not
        ui.destroy(tabs).unwrap();
        assert_eq!(ui.class_of(second), Err(Error::InvalidHandle));
        assert!(ui.render(win).unwrap().pixels.iter().all(|p| *p == BLACK || *p == WHITE));
    }
}
