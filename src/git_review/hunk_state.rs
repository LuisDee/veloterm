// Hunk collapse/expand state management for the diff view.
// Each hunk can be independently collapsed to show only its header.

use std::collections::HashMap;

/// State tracker for hunk collapse/expand in a diff view.
#[derive(Debug, Clone)]
pub struct HunkStates {
    /// Map from hunk index to collapsed state. Default is expanded (not in map).
    collapsed: HashMap<usize, bool>,
}

impl HunkStates {
    pub fn new() -> Self {
        Self {
            collapsed: HashMap::new(),
        }
    }

    /// Whether a hunk at the given index is collapsed.
    pub fn is_collapsed(&self, hunk_index: usize) -> bool {
        self.collapsed.get(&hunk_index).copied().unwrap_or(false)
    }

    /// Toggle the collapsed state of a hunk.
    pub fn toggle(&mut self, hunk_index: usize) {
        let current = self.is_collapsed(hunk_index);
        self.collapsed.insert(hunk_index, !current);
    }

    /// Collapse a specific hunk.
    pub fn collapse(&mut self, hunk_index: usize) {
        self.collapsed.insert(hunk_index, true);
    }

    /// Expand a specific hunk.
    pub fn expand(&mut self, hunk_index: usize) {
        self.collapsed.insert(hunk_index, false);
    }

    /// Collapse all hunks.
    pub fn collapse_all(&mut self, hunk_count: usize) {
        for i in 0..hunk_count {
            self.collapsed.insert(i, true);
        }
    }

    /// Expand all hunks.
    pub fn expand_all(&mut self) {
        self.collapsed.clear();
    }

    /// Reset state (e.g., when switching files).
    pub fn reset(&mut self) {
        self.collapsed.clear();
    }

    /// Count of collapsed hunks.
    pub fn collapsed_count(&self) -> usize {
        self.collapsed.values().filter(|&&v| v).count()
    }
}

/// Summary text for a collapsed hunk header.
pub fn collapsed_hunk_summary(header: &str, row_count: usize) -> String {
    format!("{} ({} lines hidden)", header, row_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_hunks_are_expanded() {
        let states = HunkStates::new();
        assert!(!states.is_collapsed(0));
        assert!(!states.is_collapsed(1));
        assert!(!states.is_collapsed(999));
    }

    #[test]
    fn toggle_collapses_expanded_hunk() {
        let mut states = HunkStates::new();
        states.toggle(0);
        assert!(states.is_collapsed(0));
    }

    #[test]
    fn toggle_expands_collapsed_hunk() {
        let mut states = HunkStates::new();
        states.toggle(0);
        assert!(states.is_collapsed(0));
        states.toggle(0);
        assert!(!states.is_collapsed(0));
    }

    #[test]
    fn collapse_specific_hunk() {
        let mut states = HunkStates::new();
        states.collapse(2);
        assert!(states.is_collapsed(2));
        assert!(!states.is_collapsed(0));
        assert!(!states.is_collapsed(1));
    }

    #[test]
    fn expand_specific_hunk() {
        let mut states = HunkStates::new();
        states.collapse(1);
        assert!(states.is_collapsed(1));
        states.expand(1);
        assert!(!states.is_collapsed(1));
    }

    #[test]
    fn collapse_all_collapses_all() {
        let mut states = HunkStates::new();
        states.collapse_all(3);
        assert!(states.is_collapsed(0));
        assert!(states.is_collapsed(1));
        assert!(states.is_collapsed(2));
    }

    #[test]
    fn expand_all_expands_all() {
        let mut states = HunkStates::new();
        states.collapse_all(3);
        states.expand_all();
        assert!(!states.is_collapsed(0));
        assert!(!states.is_collapsed(1));
        assert!(!states.is_collapsed(2));
    }

    #[test]
    fn reset_clears_state() {
        let mut states = HunkStates::new();
        states.collapse(0);
        states.collapse(2);
        states.reset();
        assert!(!states.is_collapsed(0));
        assert!(!states.is_collapsed(2));
    }

    #[test]
    fn collapsed_count_correct() {
        let mut states = HunkStates::new();
        assert_eq!(states.collapsed_count(), 0);
        states.collapse(0);
        assert_eq!(states.collapsed_count(), 1);
        states.collapse(2);
        assert_eq!(states.collapsed_count(), 2);
        states.expand(0);
        assert_eq!(states.collapsed_count(), 1);
    }

    #[test]
    fn independent_hunks_do_not_interfere() {
        let mut states = HunkStates::new();
        states.collapse(1);
        states.toggle(3);
        assert!(!states.is_collapsed(0));
        assert!(states.is_collapsed(1));
        assert!(!states.is_collapsed(2));
        assert!(states.is_collapsed(3));
    }

    #[test]
    fn collapsed_hunk_summary_format() {
        let summary = collapsed_hunk_summary("@@ -10,5 +12,7 @@", 5);
        assert_eq!(summary, "@@ -10,5 +12,7 @@ (5 lines hidden)");
    }

    #[test]
    fn collapsed_hunk_summary_zero_lines() {
        let summary = collapsed_hunk_summary("@@ -1,0 +1,0 @@", 0);
        assert_eq!(summary, "@@ -1,0 +1,0 @@ (0 lines hidden)");
    }
}
