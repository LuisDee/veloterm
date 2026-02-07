use crate::renderer::grid_renderer::GridCell;

/// Compare two grid cell buffers row-by-row and return per-row dirty flags.
///
/// Returns a `Vec<bool>` with one entry per row. A row is dirty if any cell
/// in that row differs between `prev` and `curr`. If the slices have different
/// lengths, all rows of the larger grid are marked dirty (full damage).
pub fn diff_grid_rows(prev: &[GridCell], curr: &[GridCell], cols: usize) -> Vec<bool> {
    if cols == 0 {
        return Vec::new();
    }

    let prev_rows = prev.len() / cols;
    let curr_rows = curr.len() / cols;

    // Different grid sizes → full damage on the larger grid
    if prev.len() != curr.len() {
        let max_rows = prev_rows.max(curr_rows);
        return vec![true; max_rows];
    }

    (0..curr_rows)
        .map(|row| {
            let start = row * cols;
            let end = start + cols;
            prev[start..end] != curr[start..end]
        })
        .collect()
}

/// Tracks per-row dirty state for the terminal grid.
///
/// Enables selective GPU buffer updates by identifying which rows changed
/// since the last frame, instead of rebuilding the entire instance buffer.
pub struct DamageTracker {
    dirty: Vec<bool>,
}

impl DamageTracker {
    /// Create a new DamageTracker for the given number of rows.
    /// All rows start clean.
    pub fn new(rows: usize) -> Self {
        Self {
            dirty: vec![false; rows],
        }
    }

    /// Mark a specific row as dirty. Out-of-bounds indices are ignored.
    pub fn mark_row_dirty(&mut self, row: usize) {
        if let Some(flag) = self.dirty.get_mut(row) {
            *flag = true;
        }
    }

    /// Mark all rows as dirty (full damage).
    pub fn mark_all_dirty(&mut self) {
        self.dirty.fill(true);
    }

    /// Iterate over the indices of dirty rows.
    pub fn dirty_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.dirty
            .iter()
            .enumerate()
            .filter_map(|(i, &dirty)| if dirty { Some(i) } else { None })
    }

    /// Reset all rows to clean.
    pub fn clear(&mut self) {
        self.dirty.fill(false);
    }

    /// Resize the tracker to a new row count. All rows become clean after resize.
    pub fn resize(&mut self, rows: usize) {
        self.dirty = vec![false; rows];
    }

    /// Returns the number of rows being tracked.
    pub fn row_count(&self) -> usize {
        self.dirty.len()
    }

    /// Returns true if any row is dirty.
    pub fn has_damage(&self) -> bool {
        self.dirty.iter().any(|&d| d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_has_correct_row_count() {
        let tracker = DamageTracker::new(50);
        assert_eq!(tracker.row_count(), 50);
    }

    #[test]
    fn new_tracker_has_no_dirty_rows() {
        let tracker = DamageTracker::new(50);
        assert_eq!(tracker.dirty_rows().count(), 0);
        assert!(!tracker.has_damage());
    }

    #[test]
    fn mark_row_dirty_flags_single_row() {
        let mut tracker = DamageTracker::new(50);
        tracker.mark_row_dirty(10);
        let dirty: Vec<usize> = tracker.dirty_rows().collect();
        assert_eq!(dirty, vec![10]);
        assert!(tracker.has_damage());
    }

    #[test]
    fn mark_multiple_rows_dirty() {
        let mut tracker = DamageTracker::new(50);
        tracker.mark_row_dirty(5);
        tracker.mark_row_dirty(20);
        tracker.mark_row_dirty(49);
        let dirty: Vec<usize> = tracker.dirty_rows().collect();
        assert_eq!(dirty, vec![5, 20, 49]);
    }

    #[test]
    fn mark_all_dirty_flags_every_row() {
        let mut tracker = DamageTracker::new(10);
        tracker.mark_all_dirty();
        let dirty: Vec<usize> = tracker.dirty_rows().collect();
        assert_eq!(dirty, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn clear_resets_all_flags() {
        let mut tracker = DamageTracker::new(50);
        tracker.mark_all_dirty();
        assert!(tracker.has_damage());
        tracker.clear();
        assert_eq!(tracker.dirty_rows().count(), 0);
        assert!(!tracker.has_damage());
    }

    #[test]
    fn resize_adjusts_row_count_and_clears() {
        let mut tracker = DamageTracker::new(50);
        tracker.mark_row_dirty(10);
        tracker.resize(30);
        assert_eq!(tracker.row_count(), 30);
        assert_eq!(tracker.dirty_rows().count(), 0);
    }

    #[test]
    fn out_of_bounds_row_is_ignored() {
        let mut tracker = DamageTracker::new(10);
        tracker.mark_row_dirty(10); // exactly out of bounds
        tracker.mark_row_dirty(100); // way out of bounds
        assert_eq!(tracker.dirty_rows().count(), 0);
        assert!(!tracker.has_damage());
    }

    #[test]
    fn marking_same_row_twice_is_idempotent() {
        let mut tracker = DamageTracker::new(50);
        tracker.mark_row_dirty(5);
        tracker.mark_row_dirty(5);
        let dirty: Vec<usize> = tracker.dirty_rows().collect();
        assert_eq!(dirty, vec![5]);
    }

    #[test]
    fn zero_rows_tracker_works() {
        let tracker = DamageTracker::new(0);
        assert_eq!(tracker.row_count(), 0);
        assert_eq!(tracker.dirty_rows().count(), 0);
    }

    // ── GridCell PartialEq tests ────────────────────────────────────

    use crate::config::theme::Color;
    use crate::renderer::grid_renderer::GridCell;

    fn white() -> Color {
        Color::new(1.0, 1.0, 1.0, 1.0)
    }

    fn black() -> Color {
        Color::new(0.0, 0.0, 0.0, 1.0)
    }

    fn red() -> Color {
        Color::new(1.0, 0.0, 0.0, 1.0)
    }

    #[test]
    fn gridcell_equal_when_all_fields_match() {
        let a = GridCell::new('A', white(), black());
        let b = GridCell::new('A', white(), black());
        assert_eq!(a, b);
    }

    #[test]
    fn gridcell_not_equal_when_char_differs() {
        let a = GridCell::new('A', white(), black());
        let b = GridCell::new('B', white(), black());
        assert_ne!(a, b);
    }

    #[test]
    fn gridcell_not_equal_when_fg_differs() {
        let a = GridCell::new('A', white(), black());
        let b = GridCell::new('A', red(), black());
        assert_ne!(a, b);
    }

    #[test]
    fn gridcell_not_equal_when_bg_differs() {
        let a = GridCell::new('A', white(), black());
        let b = GridCell::new('A', white(), red());
        assert_ne!(a, b);
    }

    #[test]
    fn gridcell_not_equal_when_flags_differ() {
        let mut a = GridCell::new('A', white(), black());
        let mut b = GridCell::new('A', white(), black());
        a.flags = 0;
        b.flags = crate::renderer::grid_renderer::CELL_FLAG_UNDERLINE;
        assert_ne!(a, b);
    }

    // ── Grid diff tests ─────────────────────────────────────────────

    fn make_grid(cols: usize, rows: usize, ch: char) -> Vec<GridCell> {
        vec![GridCell::new(ch, white(), black()); cols * rows]
    }

    #[test]
    fn diff_identical_grids_no_dirty_rows() {
        let grid = make_grid(4, 3, 'A');
        let result = diff_grid_rows(&grid, &grid, 4);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn diff_single_cell_change_marks_only_that_row() {
        let prev = make_grid(4, 3, 'A');
        let mut curr = prev.clone();
        curr[5] = GridCell::new('B', white(), black()); // row 1, col 1
        let result = diff_grid_rows(&prev, &curr, 4);
        assert_eq!(result, vec![false, true, false]);
    }

    #[test]
    fn diff_changes_in_multiple_rows() {
        let prev = make_grid(4, 3, 'A');
        let mut curr = prev.clone();
        curr[0] = GridCell::new('X', white(), black()); // row 0
        curr[9] = GridCell::new('Y', white(), black()); // row 2
        let result = diff_grid_rows(&prev, &curr, 4);
        assert_eq!(result, vec![true, false, true]);
    }

    #[test]
    fn diff_empty_grids_no_dirty_rows() {
        let result = diff_grid_rows(&[], &[], 0);
        assert!(result.is_empty());
    }

    #[test]
    fn diff_different_sizes_all_dirty() {
        let small = make_grid(4, 2, 'A');
        let large = make_grid(4, 3, 'A');
        let result = diff_grid_rows(&small, &large, 4);
        // When sizes differ, all rows of the larger grid should be dirty
        assert!(result.iter().all(|&d| d));
        assert_eq!(result.len(), 3);
    }
}
