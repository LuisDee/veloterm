use crate::config::theme::Color;
use crate::renderer::grid_renderer::GridCell;
use crate::search::SearchMatch;

/// Apply search match highlighting to grid cells.
///
/// Modifies cell background colors in-place for cells that fall within
/// search matches. The current match uses `active_color`, others use `match_color`.
///
/// `matches` should be pre-filtered to only include visible matches.
/// `columns` is the grid width used to compute cell indices.
pub fn apply_search_highlights(
    cells: &mut [GridCell],
    matches: &[SearchMatch],
    current_index: usize,
    columns: usize,
    match_color: Color,
    active_color: Color,
) {
    let total_rows = cells.len() / columns.max(1);
    for (i, m) in matches.iter().enumerate() {
        if m.row < 0 || m.row as usize >= total_rows {
            continue;
        }
        let color = if i == current_index {
            active_color
        } else {
            match_color
        };
        let row_start = m.row as usize * columns;
        let col_end = m.end_col.min(columns);
        for col in m.start_col..col_end {
            cells[row_start + col].bg = color;
        }
    }
}

/// Clear search highlights by restoring original background colors.
/// This is equivalent to not calling `apply_search_highlights` on the next frame,
/// since highlights are applied per-frame to extracted cells.
/// This function exists for explicit clearing + damage marking.
pub fn clear_search_highlights(
    cells: &mut [GridCell],
    matches: &[SearchMatch],
    columns: usize,
    original_bg: Color,
) {
    let total_rows = cells.len() / columns.max(1);
    for m in matches {
        if m.row < 0 || m.row as usize >= total_rows {
            continue;
        }
        let row_start = m.row as usize * columns;
        let col_end = m.end_col.min(columns);
        for col in m.start_col..col_end {
            cells[row_start + col].bg = original_bg;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::color_new;

    const MATCH_COLOR: Color = color_new(0.36, 0.29, 0.12, 1.0); // #5C4A1E
    const ACTIVE_COLOR: Color = color_new(0.55, 0.41, 0.08, 1.0); // #8B6914
    const DEFAULT_BG: Color = color_new(0.10, 0.09, 0.09, 1.0);
    const DEFAULT_FG: Color = color_new(0.91, 0.90, 0.87, 1.0);

    fn make_cells(cols: usize, rows: usize, text_rows: &[&str]) -> Vec<GridCell> {
        let mut cells = vec![GridCell::new(' ', DEFAULT_FG, DEFAULT_BG); cols * rows];
        for (row_idx, text) in text_rows.iter().enumerate() {
            for (col_idx, ch) in text.chars().enumerate() {
                if row_idx < rows && col_idx < cols {
                    cells[row_idx * cols + col_idx] = GridCell::new(ch, DEFAULT_FG, DEFAULT_BG);
                }
            }
        }
        cells
    }

    // ── 3.1.1 apply_search_highlights modifies background ────────────

    #[test]
    fn highlight_modifies_matched_cell_backgrounds() {
        let cols = 20;
        let mut cells = make_cells(cols, 3, &["hello world", "foo bar", "baz"]);
        let matches = vec![SearchMatch {
            row: 0,
            start_col: 0,
            end_col: 5,
        }];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        // Cells 0-4 of row 0 should have active_color (since current_index=0 matches index 0)
        for col in 0..5 {
            assert_eq!(
                cells[col].bg, ACTIVE_COLOR,
                "cell at col {col} should have active highlight"
            );
        }
        // Cell 5 should NOT be highlighted
        assert_eq!(cells[5].bg, DEFAULT_BG);
    }

    #[test]
    fn highlight_does_not_modify_non_matched_cells() {
        let cols = 10;
        let mut cells = make_cells(cols, 2, &["hello", "world"]);
        let matches = vec![SearchMatch {
            row: 0,
            start_col: 0,
            end_col: 5,
        }];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        // Row 1 cells should be untouched
        for col in 0..5 {
            assert_eq!(
                cells[cols + col].bg, DEFAULT_BG,
                "row 1 cell at col {col} should be unchanged"
            );
        }
    }

    // ── 3.1.2 current match vs other matches ────────────────────────

    #[test]
    fn current_match_uses_active_color() {
        let cols = 20;
        let mut cells = make_cells(cols, 2, &["aaa bbb aaa", "ccc"]);
        let matches = vec![
            SearchMatch { row: 0, start_col: 0, end_col: 3 },
            SearchMatch { row: 0, start_col: 8, end_col: 11 },
        ];
        // current_index=1 means second match is active
        apply_search_highlights(&mut cells, &matches, 1, cols, MATCH_COLOR, ACTIVE_COLOR);
        // First match (index 0) should use match_color
        assert_eq!(cells[0].bg, MATCH_COLOR);
        assert_eq!(cells[1].bg, MATCH_COLOR);
        assert_eq!(cells[2].bg, MATCH_COLOR);
        // Second match (index 1) should use active_color
        assert_eq!(cells[8].bg, ACTIVE_COLOR);
        assert_eq!(cells[9].bg, ACTIVE_COLOR);
        assert_eq!(cells[10].bg, ACTIVE_COLOR);
    }

    #[test]
    fn non_current_matches_use_match_color() {
        let cols = 20;
        let mut cells = make_cells(cols, 1, &["abc abc abc"]);
        let matches = vec![
            SearchMatch { row: 0, start_col: 0, end_col: 3 },
            SearchMatch { row: 0, start_col: 4, end_col: 7 },
            SearchMatch { row: 0, start_col: 8, end_col: 11 },
        ];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        // Match 0 (current) → active
        assert_eq!(cells[0].bg, ACTIVE_COLOR);
        // Match 1 → match_color
        assert_eq!(cells[4].bg, MATCH_COLOR);
        // Match 2 → match_color
        assert_eq!(cells[8].bg, MATCH_COLOR);
    }

    // ── 3.1.3 only visible viewport cells get highlighted ───────────

    #[test]
    fn highlight_only_affects_rows_within_cell_bounds() {
        let cols = 10;
        let rows = 3;
        let mut cells = make_cells(cols, rows, &["aaa", "bbb", "ccc"]);
        // Match on row 5 which is outside visible range (only 3 rows)
        let matches = vec![SearchMatch {
            row: 5,
            start_col: 0,
            end_col: 3,
        }];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        // No cells should be modified
        for cell in &cells {
            assert_eq!(cell.bg, DEFAULT_BG);
        }
    }

    #[test]
    fn highlight_skips_negative_rows() {
        let cols = 10;
        let rows = 3;
        let mut cells = make_cells(cols, rows, &["aaa", "bbb", "ccc"]);
        let matches = vec![SearchMatch {
            row: -1,
            start_col: 0,
            end_col: 3,
        }];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        for cell in &cells {
            assert_eq!(cell.bg, DEFAULT_BG);
        }
    }

    // ── 3.1.4 clearing search removes highlights ─────────────────────

    #[test]
    fn clear_highlights_restores_original_bg() {
        let cols = 10;
        let mut cells = make_cells(cols, 2, &["hello", "world"]);
        let matches = vec![SearchMatch {
            row: 0,
            start_col: 0,
            end_col: 5,
        }];
        // Apply then clear
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        clear_search_highlights(&mut cells, &matches, cols, DEFAULT_BG);
        for col in 0..5 {
            assert_eq!(
                cells[col].bg, DEFAULT_BG,
                "cell at col {col} should be restored"
            );
        }
    }

    #[test]
    fn empty_matches_is_noop() {
        let cols = 10;
        let mut cells = make_cells(cols, 1, &["hello"]);
        let original: Vec<GridCell> = cells.clone();
        apply_search_highlights(&mut cells, &[], 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        assert_eq!(cells, original);
    }

    // ── 3.1 additional: multi-row matches ────────────────────────────

    #[test]
    fn highlight_multiple_rows() {
        let cols = 10;
        let mut cells = make_cells(cols, 3, &["abc", "def", "abc"]);
        let matches = vec![
            SearchMatch { row: 0, start_col: 0, end_col: 3 },
            SearchMatch { row: 2, start_col: 0, end_col: 3 },
        ];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        // Row 0 = active match
        assert_eq!(cells[0].bg, ACTIVE_COLOR);
        // Row 1 = not matched
        assert_eq!(cells[cols].bg, DEFAULT_BG);
        // Row 2 = non-active match
        assert_eq!(cells[2 * cols].bg, MATCH_COLOR);
    }

    #[test]
    fn highlight_clamps_column_to_grid_width() {
        let cols = 5;
        let mut cells = make_cells(cols, 1, &["abcde"]);
        // Match end_col extends beyond grid width
        let matches = vec![SearchMatch {
            row: 0,
            start_col: 3,
            end_col: 10,
        }];
        apply_search_highlights(&mut cells, &matches, 0, cols, MATCH_COLOR, ACTIVE_COLOR);
        // Only cols 3 and 4 should be highlighted (clamped to cols)
        assert_eq!(cells[3].bg, ACTIVE_COLOR);
        assert_eq!(cells[4].bg, ACTIVE_COLOR);
        // Should not panic accessing out of bounds
    }
}
