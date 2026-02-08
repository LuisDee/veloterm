use crate::renderer::grid_renderer::GridCell;
use std::time::Duration;

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

/// Manages damage detection by caching the previous frame's grid cells
/// and diffing against the current frame.
pub struct DamageState {
    prev_cells: Option<Vec<GridCell>>,
    pub(crate) cols: usize,
    force_full: bool,
}

impl DamageState {
    /// Create a new DamageState with the given column count.
    pub fn new(cols: usize) -> Self {
        Self {
            prev_cells: None,
            cols,
            force_full: false,
        }
    }

    /// Process a new frame's cells and return per-row dirty flags.
    ///
    /// On the first frame (no cache) or after `force_full_damage()`, returns all-dirty.
    /// Otherwise, diffs against the previous frame row-by-row.
    /// Updates the cache with the current cells after diffing.
    pub fn process_frame(&mut self, cells: &[GridCell]) -> Vec<bool> {
        let rows = if self.cols > 0 {
            cells.len() / self.cols
        } else {
            0
        };

        let dirty = match self.prev_cells.as_ref() {
            Some(prev) if !self.force_full => diff_grid_rows(prev, cells, self.cols),
            _ => {
                self.force_full = false;
                vec![true; rows]
            }
        };

        self.prev_cells = Some(cells.to_vec());
        dirty
    }

    /// Force the next frame to be fully dirty (e.g., on resize, theme change).
    pub fn force_full_damage(&mut self) {
        self.force_full = true;
    }

    /// Resize the state for a new column count. Clears the cache.
    pub fn resize(&mut self, cols: usize) {
        self.cols = cols;
        self.prev_cells = None;
    }
}

/// Manages per-pane damage states for multi-pane rendering.
///
/// Each pane gets its own independent DamageState, identified by PaneId.
/// Panes can be added/removed dynamically as splits/closes occur.
pub struct PaneDamageMap {
    states: std::collections::HashMap<crate::pane::PaneId, DamageState>,
}

impl Default for PaneDamageMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneDamageMap {
    /// Create a new empty PaneDamageMap.
    pub fn new() -> Self {
        Self {
            states: std::collections::HashMap::new(),
        }
    }

    /// Get or create the DamageState for a pane. New panes start with full damage.
    pub fn get_or_create(&mut self, pane_id: crate::pane::PaneId, cols: usize) -> &mut DamageState {
        self.states
            .entry(pane_id)
            .or_insert_with(|| DamageState::new(cols))
    }

    /// Remove the DamageState for a closed pane.
    pub fn remove(&mut self, pane_id: crate::pane::PaneId) {
        self.states.remove(&pane_id);
    }

    /// Force full damage on all panes (e.g., after window resize).
    pub fn force_full_damage_all(&mut self) {
        for state in self.states.values_mut() {
            state.force_full_damage();
        }
    }

    /// Number of tracked panes.
    pub fn pane_count(&self) -> usize {
        self.states.len()
    }
}

/// Tracks per-frame timing metrics for the render loop.
///
/// Records diff time (damage detection), update time (GPU buffer writes),
/// and total frame time. Computes rolling averages over a configurable
/// summary interval and logs them periodically.
pub struct FrameMetrics {
    diff_times: Vec<Duration>,
    update_times: Vec<Duration>,
    total_times: Vec<Duration>,
    summary_interval: usize,
    frame_count: usize,
}

impl FrameMetrics {
    /// Create a new FrameMetrics that logs a summary every `summary_interval` frames.
    pub fn new(summary_interval: usize) -> Self {
        Self {
            diff_times: Vec::new(),
            update_times: Vec::new(),
            total_times: Vec::new(),
            summary_interval,
            frame_count: 0,
        }
    }

    /// Record timing for one frame. Logs at debug level per-frame
    /// and info level summary every `summary_interval` frames.
    pub fn record(&mut self, diff_time: Duration, update_time: Duration, total_time: Duration) {
        self.diff_times.push(diff_time);
        self.update_times.push(update_time);
        self.total_times.push(total_time);
        self.frame_count += 1;

        log::debug!(
            "Frame {}: diff={:?} update={:?} total={:?}",
            self.frame_count,
            diff_time,
            update_time,
            total_time,
        );

        if self.frame_count.is_multiple_of(self.summary_interval) {
            let (avg_diff, avg_update, avg_total) = self.averages();
            log::info!(
                "Frame metrics ({} frames): avg diff={:?} update={:?} total={:?}",
                self.diff_times.len(),
                avg_diff,
                avg_update,
                avg_total,
            );
            self.diff_times.clear();
            self.update_times.clear();
            self.total_times.clear();
        }
    }

    /// Returns the number of frames recorded since the last summary (or since creation).
    pub fn frame_count(&self) -> usize {
        self.diff_times.len()
    }

    /// Compute average durations over accumulated frames.
    /// Returns (avg_diff, avg_update, avg_total). Returns zero durations if no frames recorded.
    pub fn averages(&self) -> (Duration, Duration, Duration) {
        let n = self.diff_times.len();
        if n == 0 {
            return (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        }
        let avg = |times: &[Duration]| times.iter().sum::<Duration>() / n as u32;
        (
            avg(&self.diff_times),
            avg(&self.update_times),
            avg(&self.total_times),
        )
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

    // ── DamageState tests ───────────────────────────────────────────

    #[test]
    fn damage_state_first_frame_all_dirty() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let dirty = state.process_frame(&cells);
        assert_eq!(dirty.len(), 3);
        assert!(dirty.iter().all(|&d| d), "first frame should be all dirty");
    }

    #[test]
    fn damage_state_second_frame_no_changes_no_dirty() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells); // first frame
        let dirty = state.process_frame(&cells); // identical second frame
        assert_eq!(dirty, vec![false, false, false]);
    }

    #[test]
    fn damage_state_second_frame_one_row_changed() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        let mut changed = cells.clone();
        changed[5] = GridCell::new('B', white(), black()); // row 1
        let dirty = state.process_frame(&changed);
        assert_eq!(dirty, vec![false, true, false]);
    }

    #[test]
    fn damage_state_cache_updates_after_each_diff() {
        let mut state = DamageState::new(4);
        let cells_a = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells_a); // first frame
        let mut cells_b = cells_a.clone();
        cells_b[0] = GridCell::new('B', white(), black());
        let _ = state.process_frame(&cells_b); // second frame: row 0 dirty
                                               // Third frame same as cells_b → no dirty
        let dirty = state.process_frame(&cells_b);
        assert_eq!(dirty, vec![false, false, false]);
    }

    #[test]
    fn damage_state_resize_clears_cache_returns_all_dirty() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        state.resize(5); // new column count
        let cells_new = make_grid(5, 2, 'A');
        let dirty = state.process_frame(&cells_new);
        assert_eq!(dirty.len(), 2);
        assert!(dirty.iter().all(|&d| d), "after resize should be all dirty");
    }

    #[test]
    fn damage_state_force_full_damage() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        state.force_full_damage();
        let dirty = state.process_frame(&cells); // same cells but forced
        assert_eq!(dirty.len(), 3);
        assert!(
            dirty.iter().all(|&d| d),
            "forced full damage should be all dirty"
        );
    }

    // ── Full-damage event trigger tests ──────────────────────────────

    #[test]
    fn resize_triggers_full_damage_and_cache_clear() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        // Simulate resize event
        state.resize(6);
        state.force_full_damage();
        let new_cells = make_grid(6, 2, 'A');
        let dirty = state.process_frame(&new_cells);
        assert_eq!(dirty.len(), 2);
        assert!(
            dirty.iter().all(|&d| d),
            "resize should trigger full damage"
        );
    }

    #[test]
    fn theme_change_triggers_full_damage() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        // Simulate theme change: same grid but force full repaint
        state.force_full_damage();
        let dirty = state.process_frame(&cells);
        assert!(
            dirty.iter().all(|&d| d),
            "theme change should trigger full damage"
        );
    }

    #[test]
    fn font_size_change_triggers_full_damage() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        // Simulate font size change: resize + force full damage
        state.resize(5);
        state.force_full_damage();
        let new_cells = make_grid(5, 4, 'A');
        let dirty = state.process_frame(&new_cells);
        assert_eq!(dirty.len(), 4);
        assert!(
            dirty.iter().all(|&d| d),
            "font size change should trigger full damage"
        );
    }

    #[test]
    fn scroll_triggers_full_damage() {
        let mut state = DamageState::new(4);
        let cells_a = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells_a);
        // Simulate scroll: content changes + force full damage
        state.force_full_damage();
        let cells_b = make_grid(4, 3, 'B');
        let dirty = state.process_frame(&cells_b);
        assert!(
            dirty.iter().all(|&d| d),
            "scroll should trigger full damage"
        );
    }

    #[test]
    fn force_full_damage_resets_after_one_frame() {
        let mut state = DamageState::new(4);
        let cells = make_grid(4, 3, 'A');
        let _ = state.process_frame(&cells);
        state.force_full_damage();
        let _ = state.process_frame(&cells); // consumes the force flag
                                             // Next frame with same data should be clean
        let dirty = state.process_frame(&cells);
        assert!(
            dirty.iter().all(|&d| !d),
            "force flag should reset after one frame"
        );
    }

    // ── FrameMetrics tests ───────────────────────────────────────────

    use std::time::Duration;

    #[test]
    fn frame_metrics_starts_at_zero_frames() {
        let metrics = FrameMetrics::new(60);
        assert_eq!(metrics.frame_count(), 0);
    }

    #[test]
    fn frame_metrics_records_frame_count() {
        let mut metrics = FrameMetrics::new(60);
        metrics.record(
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(400),
        );
        metrics.record(
            Duration::from_micros(150),
            Duration::from_micros(250),
            Duration::from_micros(500),
        );
        assert_eq!(metrics.frame_count(), 2);
    }

    #[test]
    fn frame_metrics_averages_single_frame() {
        let mut metrics = FrameMetrics::new(60);
        metrics.record(
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(400),
        );
        let (avg_diff, avg_update, avg_total) = metrics.averages();
        assert_eq!(avg_diff, Duration::from_micros(100));
        assert_eq!(avg_update, Duration::from_micros(200));
        assert_eq!(avg_total, Duration::from_micros(400));
    }

    #[test]
    fn frame_metrics_averages_multiple_frames() {
        let mut metrics = FrameMetrics::new(60);
        metrics.record(
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(400),
        );
        metrics.record(
            Duration::from_micros(300),
            Duration::from_micros(400),
            Duration::from_micros(800),
        );
        let (avg_diff, avg_update, avg_total) = metrics.averages();
        assert_eq!(avg_diff, Duration::from_micros(200));
        assert_eq!(avg_update, Duration::from_micros(300));
        assert_eq!(avg_total, Duration::from_micros(600));
    }

    #[test]
    fn frame_metrics_averages_zero_when_empty() {
        let metrics = FrameMetrics::new(60);
        let (avg_diff, avg_update, avg_total) = metrics.averages();
        assert_eq!(avg_diff, Duration::ZERO);
        assert_eq!(avg_update, Duration::ZERO);
        assert_eq!(avg_total, Duration::ZERO);
    }

    // ── PaneDamageMap tests ─────────────────────────────────────────

    use crate::pane::PaneId;

    #[test]
    fn pane_damage_each_pane_has_independent_state() {
        let mut map = PaneDamageMap::new();
        let id_a = PaneId(100);
        let id_b = PaneId(200);

        let cells_a = make_grid(4, 3, 'A');
        let cells_b = make_grid(4, 3, 'B');

        // First frame for both: all dirty
        let dirty_a = map.get_or_create(id_a, 4).process_frame(&cells_a);
        let dirty_b = map.get_or_create(id_b, 4).process_frame(&cells_b);
        assert!(dirty_a.iter().all(|&d| d));
        assert!(dirty_b.iter().all(|&d| d));

        // Second frame: same data → no dirty
        let dirty_a = map.get_or_create(id_a, 4).process_frame(&cells_a);
        let dirty_b = map.get_or_create(id_b, 4).process_frame(&cells_b);
        assert!(dirty_a.iter().all(|&d| !d));
        assert!(dirty_b.iter().all(|&d| !d));
    }

    #[test]
    fn pane_damage_change_in_pane_a_does_not_mark_pane_b_dirty() {
        let mut map = PaneDamageMap::new();
        let id_a = PaneId(100);
        let id_b = PaneId(200);

        let cells = make_grid(4, 3, 'A');
        let _ = map.get_or_create(id_a, 4).process_frame(&cells);
        let _ = map.get_or_create(id_b, 4).process_frame(&cells);

        // Change pane A only
        let mut changed = cells.clone();
        changed[0] = GridCell::new('X', white(), black());
        let dirty_a = map.get_or_create(id_a, 4).process_frame(&changed);
        let dirty_b = map.get_or_create(id_b, 4).process_frame(&cells);

        assert!(dirty_a[0]); // Pane A row 0 is dirty
        assert!(dirty_b.iter().all(|&d| !d)); // Pane B is clean
    }

    #[test]
    fn pane_damage_force_full_damage_all() {
        let mut map = PaneDamageMap::new();
        let id_a = PaneId(100);
        let id_b = PaneId(200);

        let cells = make_grid(4, 3, 'A');
        let _ = map.get_or_create(id_a, 4).process_frame(&cells);
        let _ = map.get_or_create(id_b, 4).process_frame(&cells);

        // Force all dirty (simulates resize)
        map.force_full_damage_all();

        let dirty_a = map.get_or_create(id_a, 4).process_frame(&cells);
        let dirty_b = map.get_or_create(id_b, 4).process_frame(&cells);
        assert!(dirty_a.iter().all(|&d| d));
        assert!(dirty_b.iter().all(|&d| d));
    }

    #[test]
    fn pane_damage_new_pane_starts_with_full_damage() {
        let mut map = PaneDamageMap::new();
        let id = PaneId(100);
        let cells = make_grid(4, 3, 'A');

        // First frame for a new pane is always fully dirty
        let dirty = map.get_or_create(id, 4).process_frame(&cells);
        assert!(dirty.iter().all(|&d| d));
    }

    #[test]
    fn pane_damage_remove_cleans_up() {
        let mut map = PaneDamageMap::new();
        let id = PaneId(100);
        let cells = make_grid(4, 3, 'A');
        let _ = map.get_or_create(id, 4).process_frame(&cells);
        assert_eq!(map.pane_count(), 1);

        map.remove(id);
        assert_eq!(map.pane_count(), 0);

        // Re-adding should start fresh (full damage)
        let dirty = map.get_or_create(id, 4).process_frame(&cells);
        assert!(dirty.iter().all(|&d| d));
    }
}
