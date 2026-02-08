// Tab management: tab lifecycle, ordering, and multi-tab state.

pub mod bar;

use std::sync::atomic::{AtomicU32, Ordering};

use crate::pane::{PaneId, PaneTree};

static NEXT_TAB_ID: AtomicU32 = AtomicU32::new(1);

/// Unique identifier for a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u32);

impl TabId {
    pub fn new() -> Self {
        Self(NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed))
    }

    #[cfg(test)]
    pub(crate) fn reset_counter() {
        NEXT_TAB_ID.store(1, Ordering::Relaxed);
    }
}

/// A single tab containing an independent pane tree.
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub pane_tree: PaneTree,
}

impl Tab {
    pub fn new() -> Self {
        let pane_tree = PaneTree::new();
        Self {
            id: TabId::new(),
            title: "Shell".to_string(),
            pane_tree,
        }
    }

    /// Returns all PaneIds owned by this tab's pane tree.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        self.pane_tree.pane_ids()
    }
}

/// Manages an ordered list of tabs with one active tab.
pub struct TabManager {
    tabs: Vec<Tab>,
    active_index: usize,
}

impl TabManager {
    /// Creates a new TabManager with a single default tab.
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::new()],
            active_index: 0,
        }
    }

    /// Returns the number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Returns a slice of all tabs (for rendering the tab bar).
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Returns the active tab index.
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Returns a reference to the active tab.
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    /// Returns a mutable reference to the active tab.
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    /// Creates a new tab, appends it after the active tab, makes it active, and returns its TabId.
    pub fn new_tab(&mut self) -> TabId {
        let tab = Tab::new();
        let id = tab.id;
        let insert_pos = self.active_index + 1;
        self.tabs.insert(insert_pos, tab);
        self.active_index = insert_pos;
        id
    }

    /// Closes the tab at the given index.
    /// Returns the PaneIds that need cleanup, or None if it's the last tab (can't close).
    pub fn close_tab(&mut self, index: usize) -> Option<Vec<PaneId>> {
        if self.tabs.len() <= 1 {
            return None;
        }
        if index >= self.tabs.len() {
            return None;
        }
        let removed = self.tabs.remove(index);
        let pane_ids = removed.pane_ids();
        // Adjust active index
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        } else if self.active_index > index {
            self.active_index -= 1;
        }
        Some(pane_ids)
    }

    /// Switches to the tab at the given index (clamped to valid range).
    pub fn select_tab(&mut self, index: usize) {
        if !self.tabs.is_empty() {
            self.active_index = index.min(self.tabs.len() - 1);
        }
    }

    /// Switches to the next tab, wrapping around.
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /// Switches to the previous tab, wrapping around.
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_index == 0 {
                self.active_index = self.tabs.len() - 1;
            } else {
                self.active_index -= 1;
            }
        }
    }

    /// Moves the tab from one index to another.
    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        // Track the active tab through the move
        if self.active_index == from {
            self.active_index = to;
        } else if from < self.active_index && to >= self.active_index {
            self.active_index -= 1;
        } else if from > self.active_index && to <= self.active_index {
            self.active_index += 1;
        }
    }

    /// Sets the title of the tab at the given index.
    pub fn set_title(&mut self, tab_index: usize, title: &str) {
        if let Some(tab) = self.tabs.get_mut(tab_index) {
            tab.title = title.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        TabId::reset_counter();
        PaneId::reset_counter();
    }

    // ── TabId ─────────────────────────────────────────────────────

    #[test]
    fn tab_id_unique() {
        setup();
        let a = TabId::new();
        let b = TabId::new();
        let c = TabId::new();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_eq!(a.0 + 1, b.0);
        assert_eq!(b.0 + 1, c.0);
    }

    #[test]
    fn tab_id_reset_counter() {
        setup();
        let a = TabId::new();
        assert_eq!(a.0, 1);
        TabId::reset_counter();
        let b = TabId::new();
        assert_eq!(b.0, 1);
    }

    // ── Tab ───────────────────────────────────────────────────────

    #[test]
    fn tab_new_has_default_title() {
        setup();
        let tab = Tab::new();
        assert_eq!(tab.title, "Shell");
    }

    #[test]
    fn tab_new_has_pane_tree_with_one_pane() {
        setup();
        let tab = Tab::new();
        assert_eq!(tab.pane_tree.pane_count(), 1);
    }

    #[test]
    fn tab_pane_ids_returns_tree_panes() {
        setup();
        let tab = Tab::new();
        let ids = tab.pane_ids();
        assert_eq!(ids.len(), 1);
    }

    // ── TabManager::new ──────────────────────────────────────────

    #[test]
    fn manager_new_has_one_tab() {
        setup();
        let mgr = TabManager::new();
        assert_eq!(mgr.tab_count(), 1);
        assert_eq!(mgr.active_index(), 0);
    }

    #[test]
    fn manager_active_tab_is_first() {
        setup();
        let mgr = TabManager::new();
        let tab = mgr.active_tab();
        assert_eq!(tab.title, "Shell");
    }

    // ── TabManager::new_tab ──────────────────────────────────────

    #[test]
    fn new_tab_appends_and_activates() {
        setup();
        let mut mgr = TabManager::new();
        let first_id = mgr.active_tab().id;
        let new_id = mgr.new_tab();
        assert_ne!(first_id, new_id);
        assert_eq!(mgr.tab_count(), 2);
        assert_eq!(mgr.active_index(), 1);
        assert_eq!(mgr.active_tab().id, new_id);
    }

    #[test]
    fn new_tab_inserts_after_active() {
        setup();
        let mut mgr = TabManager::new();
        let _t1 = mgr.active_tab().id;
        let t2 = mgr.new_tab(); // active=1
        mgr.select_tab(0); // go back to tab 0
        let t3 = mgr.new_tab(); // should insert at index 1
        assert_eq!(mgr.tabs[0].id, _t1);
        assert_eq!(mgr.tabs[1].id, t3);
        assert_eq!(mgr.tabs[2].id, t2);
        assert_eq!(mgr.active_index(), 1);
    }

    // ── TabManager::close_tab ────────────────────────────────────

    #[test]
    fn close_tab_removes_and_returns_pane_ids() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        let result = mgr.close_tab(1);
        assert!(result.is_some());
        let ids = result.unwrap();
        assert_eq!(ids.len(), 1); // one pane per new tab
        assert_eq!(mgr.tab_count(), 1);
    }

    #[test]
    fn close_last_tab_returns_none() {
        setup();
        let mut mgr = TabManager::new();
        let result = mgr.close_tab(0);
        assert!(result.is_none());
        assert_eq!(mgr.tab_count(), 1);
    }

    #[test]
    fn close_tab_out_of_range_returns_none() {
        setup();
        let mut mgr = TabManager::new();
        let result = mgr.close_tab(5);
        assert!(result.is_none());
    }

    #[test]
    fn close_active_tab_adjusts_index() {
        setup();
        let mut mgr = TabManager::new();
        let _t1 = mgr.active_tab().id;
        mgr.new_tab(); // t2 at index 1, active
        mgr.new_tab(); // t3 at index 2, active
        // Active is 2, close it
        mgr.close_tab(2);
        assert_eq!(mgr.active_index(), 1); // clamped
        assert_eq!(mgr.tab_count(), 2);
    }

    #[test]
    fn close_tab_before_active_adjusts_index() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.new_tab(); // active = 2
        mgr.close_tab(0);
        assert_eq!(mgr.active_index(), 1); // shifted down
    }

    // ── TabManager::select_tab ───────────────────────────────────

    #[test]
    fn select_tab_switches_active() {
        setup();
        let mut mgr = TabManager::new();
        let t1 = mgr.active_tab().id;
        mgr.new_tab();
        mgr.select_tab(0);
        assert_eq!(mgr.active_index(), 0);
        assert_eq!(mgr.active_tab().id, t1);
    }

    #[test]
    fn select_tab_clamps_to_range() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.select_tab(100);
        assert_eq!(mgr.active_index(), 1); // clamped to last
    }

    // ── TabManager::next_tab / prev_tab ──────────────────────────

    #[test]
    fn next_tab_cycles_forward() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.new_tab();
        mgr.select_tab(0);
        mgr.next_tab();
        assert_eq!(mgr.active_index(), 1);
        mgr.next_tab();
        assert_eq!(mgr.active_index(), 2);
        mgr.next_tab();
        assert_eq!(mgr.active_index(), 0); // wraps
    }

    #[test]
    fn prev_tab_cycles_backward() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.new_tab();
        mgr.select_tab(0);
        mgr.prev_tab();
        assert_eq!(mgr.active_index(), 2); // wraps to end
        mgr.prev_tab();
        assert_eq!(mgr.active_index(), 1);
    }

    // ── TabManager::move_tab ─────────────────────────────────────

    #[test]
    fn move_tab_reorders() {
        setup();
        let mut mgr = TabManager::new();
        let t1 = mgr.active_tab().id;
        let t2 = mgr.new_tab();
        let t3 = mgr.new_tab();
        // Order: [t1, t2, t3], active=2 (t3)
        mgr.move_tab(2, 0);
        // Order: [t3, t1, t2], active should follow t3 to 0
        assert_eq!(mgr.tabs[0].id, t3);
        assert_eq!(mgr.tabs[1].id, t1);
        assert_eq!(mgr.tabs[2].id, t2);
        assert_eq!(mgr.active_index(), 0);
    }

    #[test]
    fn move_tab_same_index_noop() {
        setup();
        let mut mgr = TabManager::new();
        mgr.new_tab();
        mgr.move_tab(0, 0);
        assert_eq!(mgr.tab_count(), 2);
    }

    #[test]
    fn move_tab_out_of_range_noop() {
        setup();
        let mut mgr = TabManager::new();
        mgr.move_tab(0, 5);
        assert_eq!(mgr.tab_count(), 1);
    }

    // ── TabManager::set_title ────────────────────────────────────

    #[test]
    fn set_title_updates_tab() {
        setup();
        let mut mgr = TabManager::new();
        mgr.set_title(0, "~/projects");
        assert_eq!(mgr.active_tab().title, "~/projects");
    }

    #[test]
    fn set_title_out_of_range_noop() {
        setup();
        let mut mgr = TabManager::new();
        mgr.set_title(5, "nope");
        assert_eq!(mgr.active_tab().title, "Shell");
    }
}
