// Text selection: state management, coordinate conversion, and text extraction.

use crate::renderer::grid_renderer::{GridCell, CELL_FLAG_SELECTED};

/// Which half of a cell the selection anchor sits in.
/// Used for sub-cell precision when normalizing selection boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Type of text selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionType {
    /// Click-and-drag character-level selection.
    Range,
    /// Double-click word selection.
    Word,
    /// Triple-click line selection.
    Line,
    /// Rectangular block selection (vi-mode Ctrl+V).
    VisualBlock,
}

/// A text selection region defined by start and end cell coordinates.
/// Row coordinates are absolute (negative = scrollback history, 0+ = screen).
/// `abs_row = viewport_row as i32 - display_offset as i32`
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    /// Start position (absolute_row, col).
    pub start: (i32, usize),
    /// End position (absolute_row, col).
    pub end: (i32, usize),
    /// Type of selection.
    pub selection_type: SelectionType,
    /// Which side of the start cell the anchor sits in.
    pub start_side: Side,
    /// Which side of the end cell the anchor sits in.
    pub end_side: Side,
}

/// Convert pixel coordinates to cell (row, col), clamped to grid bounds.
pub fn pixel_to_cell(
    pixel_x: f64,
    pixel_y: f64,
    cell_width: f32,
    cell_height: f32,
    cols: usize,
    rows: usize,
) -> (usize, usize) {
    let col = (pixel_x.max(0.0) as f32 / cell_width).floor() as usize;
    let row = (pixel_y.max(0.0) as f32 / cell_height).floor() as usize;
    (row.min(rows - 1), col.min(cols - 1))
}

/// Convert pixel coordinates to cell (row, col) plus which Side of the cell,
/// clamped to grid bounds.
pub fn pixel_to_cell_with_side(
    pixel_x: f64,
    pixel_y: f64,
    cell_width: f32,
    cell_height: f32,
    cols: usize,
    rows: usize,
) -> (usize, usize, Side) {
    let (row, col) = pixel_to_cell(pixel_x, pixel_y, cell_width, cell_height, cols, rows);
    let cell_center_x = (col as f64 + 0.5) * cell_width as f64;
    let side = if pixel_x.max(0.0) < cell_center_x {
        Side::Left
    } else {
        Side::Right
    };
    (row, col, side)
}

/// Check if a character is a word character (not whitespace or punctuation).
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Find word boundaries around the given column in a row.
/// Returns (start_col, end_col) inclusive. Words are delimited by whitespace and punctuation.
pub fn find_word_boundaries(
    cells: &[GridCell],
    row: usize,
    col: usize,
    cols: usize,
) -> (usize, usize) {
    let row_start = row * cols;
    let ch = cells[row_start + col].ch;

    if !is_word_char(ch) {
        return (col, col);
    }

    let mut start = col;
    while start > 0 && is_word_char(cells[row_start + start - 1].ch) {
        start -= 1;
    }

    let mut end = col;
    while end + 1 < cols && is_word_char(cells[row_start + end + 1].ch) {
        end += 1;
    }

    (start, end)
}

/// Normalize selection so start is before end in reading order.
/// Returns ((row, col), (row, col)) with Side adjustments applied:
/// - If the first anchor is Side::Right, the column is incremented (selection starts after that cell)
/// - If the last anchor is Side::Left, the column is decremented (selection ends before that cell)
fn normalize(selection: &Selection) -> ((i32, usize), (i32, usize)) {
    let (s, e) = (selection.start, selection.end);
    let (first, first_side, last, last_side) =
        if s.0 < e.0 || (s.0 == e.0 && s.1 <= e.1) {
            (s, selection.start_side, e, selection.end_side)
        } else {
            (e, selection.end_side, s, selection.start_side)
        };

    // For Word and Line selections, Side adjustment is not needed — boundaries
    // are already at word/line edges.
    if selection.selection_type == SelectionType::Word
        || selection.selection_type == SelectionType::Line
    {
        return (first, last);
    }

    // Apply Side adjustments for Range/VisualBlock:
    // Right-side start → selection begins at the next cell
    let first_col = match first_side {
        Side::Right => first.1 + 1,
        Side::Left => first.1,
    };
    // Left-side end → selection ends at the previous cell
    let last_col = match last_side {
        Side::Left => last.1.saturating_sub(1),
        Side::Right => last.1,
    };

    // If Side adjustment causes first_col > last_col on same row, return empty
    if first.0 == last.0 && first_col > last_col {
        return ((first.0, first_col), (last.0, first_col.saturating_sub(1)));
    }

    ((first.0, first_col), (last.0, last_col))
}

/// Check if a cell at (absolute_row, col) is within the selection, in reading order.
pub fn selection_contains(selection: &Selection, row: i32, col: usize) -> bool {
    let (start, end) = normalize(selection);

    if row < start.0 || row > end.0 {
        return false;
    }

    if selection.selection_type == SelectionType::VisualBlock {
        // Block selection: only cells within the column range on each row
        let col_min = start.1.min(end.1);
        let col_max = start.1.max(end.1);
        return col >= col_min && col <= col_max;
    }

    if start.0 == end.0 {
        // Single-line selection
        col >= start.1 && col <= end.1
    } else if row == start.0 {
        col >= start.1
    } else if row == end.0 {
        col <= end.1
    } else {
        // Middle rows are fully selected
        true
    }
}

/// Extract the selected text from grid cells as a UTF-8 string.
/// Trailing spaces per line are trimmed; lines are joined with '\n'.
/// `display_offset` converts absolute rows to viewport rows for cell lookup.
pub fn selected_text(cells: &[GridCell], selection: &Selection, cols: usize, display_offset: usize) -> String {
    let (start, end) = normalize(selection);
    let rows = cells.len() / cols;
    let mut lines = Vec::new();

    for abs_row in start.0..=end.0 {
        let vp_row = abs_row + display_offset as i32;
        if vp_row < 0 || vp_row >= rows as i32 {
            continue;
        }
        let vp_row = vp_row as usize;
        let col_start = if abs_row == start.0 { start.1 } else { 0 };
        let col_end = if abs_row == end.0 { end.1 } else { cols - 1 };
        let row_offset = vp_row * cols;

        let mut line = String::new();
        for col in col_start..=col_end {
            if row_offset + col < cells.len() {
                line.push(cells[row_offset + col].ch);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}

/// Return the column index of the rightmost non-space cell in a grid row.
/// Returns 0 if the row is entirely empty (spaces).
fn last_occupied_column(cells: &[GridCell], row_offset: usize, cols: usize) -> usize {
    for col in (0..cols).rev() {
        if row_offset + col < cells.len() && cells[row_offset + col].ch != ' ' {
            return col;
        }
    }
    0
}

/// Set CELL_FLAG_SELECTED on all cells within the selection.
/// `display_offset` converts absolute rows to viewport rows for cell lookup.
/// For multi-line selections, middle rows and trailing portions of the start row
/// are clamped to the last occupied (non-space) column to avoid highlighting
/// empty regions beyond text content.
pub fn apply_selection_flags(cells: &mut [GridCell], selection: &Selection, cols: usize, display_offset: usize) {
    let (start, end) = normalize(selection);
    let rows = cells.len() / cols;

    if selection.selection_type == SelectionType::VisualBlock {
        let col_min = start.1.min(end.1);
        let col_max = start.1.max(end.1);
        for abs_row in start.0..=end.0 {
            let vp_row = abs_row + display_offset as i32;
            if vp_row < 0 || vp_row >= rows as i32 {
                continue;
            }
            let row_offset = vp_row as usize * cols;
            for col in col_min..=col_max.min(cols - 1) {
                cells[row_offset + col].flags |= CELL_FLAG_SELECTED;
            }
        }
        return;
    }

    // Line-type selections highlight full rows — don't clamp
    let is_line_selection = selection.selection_type == SelectionType::Line;

    for abs_row in start.0..=end.0 {
        let vp_row = abs_row + display_offset as i32;
        if vp_row < 0 || vp_row >= rows as i32 {
            continue;
        }
        let vp_row = vp_row as usize;
        let row_offset = vp_row * cols;
        let col_start = if abs_row == start.0 { start.1 } else { 0 };

        let col_end = if abs_row == end.0 {
            // Last row of selection: always use the selection end column
            end.1
        } else if is_line_selection {
            // Line selections highlight full rows
            cols - 1
        } else {
            // Middle/start rows: clamp to last occupied cell
            let last_occ = last_occupied_column(cells, row_offset, cols);
            // If the row is entirely empty, don't highlight anything beyond col 0
            // But if start is on this row, at least go to the start column
            if abs_row == start.0 {
                last_occ.max(start.1)
            } else {
                last_occ
            }
        };

        for col in col_start..=col_end.min(cols - 1) {
            cells[row_offset + col].flags |= CELL_FLAG_SELECTED;
        }
    }
}

/// Extract selected text from a visual-block (rectangular) selection.
/// Each row's selected columns are extracted and joined with newlines.
pub fn selected_text_block(cells: &[GridCell], selection: &Selection, cols: usize, display_offset: usize) -> String {
    let (start, end) = normalize(selection);
    let col_min = start.1.min(end.1);
    let col_max = start.1.max(end.1);
    let rows = cells.len() / cols;
    let mut lines = Vec::new();

    for abs_row in start.0..=end.0 {
        let vp_row = abs_row + display_offset as i32;
        if vp_row < 0 || vp_row >= rows as i32 {
            continue;
        }
        let row_offset = vp_row as usize * cols;
        let mut line = String::new();
        for col in col_min..=col_max {
            if row_offset + col < cells.len() {
                line.push(cells[row_offset + col].ch);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}

/// Extract selected text from a visual-line selection (full rows).
pub fn selected_text_lines(cells: &[GridCell], selection: &Selection, cols: usize, display_offset: usize) -> String {
    let (start, end) = normalize(selection);
    let rows = cells.len() / cols;
    let mut lines = Vec::new();

    for abs_row in start.0..=end.0 {
        let vp_row = abs_row + display_offset as i32;
        if vp_row < 0 || vp_row >= rows as i32 {
            continue;
        }
        let row_offset = vp_row as usize * cols;
        let mut line = String::new();
        for col in 0..cols {
            if row_offset + col < cells.len() {
                line.push(cells[row_offset + col].ch);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::{Color, color_new};

    /// Helper: create a row of cells from a string, padded to `cols` with spaces.
    fn make_row(text: &str, cols: usize) -> Vec<GridCell> {
        let fg = color_new(1.0, 1.0, 1.0, 1.0);
        let bg = color_new(0.0, 0.0, 0.0, 1.0);
        let mut cells = Vec::with_capacity(cols);
        for ch in text.chars().take(cols) {
            cells.push(GridCell::new(ch, fg, bg));
        }
        while cells.len() < cols {
            cells.push(GridCell::new(' ', fg, bg));
        }
        cells
    }

    // ── Pixel to cell conversion ─────────────────────────────────────

    #[test]
    fn pixel_to_cell_origin() {
        let (row, col) = pixel_to_cell(0.0, 0.0, 10.0, 20.0, 80, 24);
        assert_eq!((row, col), (0, 0));
    }

    #[test]
    fn pixel_to_cell_mid_cell() {
        // pixel (15, 25) with cell_width=10, cell_height=20 → col=1, row=1
        let (row, col) = pixel_to_cell(15.0, 25.0, 10.0, 20.0, 80, 24);
        assert_eq!((row, col), (1, 1));
    }

    #[test]
    fn pixel_to_cell_clamped_to_bounds() {
        let (row, col) = pixel_to_cell(9999.0, 9999.0, 10.0, 20.0, 80, 24);
        assert_eq!((row, col), (23, 79));
    }

    #[test]
    fn pixel_to_cell_negative_clamped_to_zero() {
        let (row, col) = pixel_to_cell(-5.0, -10.0, 10.0, 20.0, 80, 24);
        assert_eq!((row, col), (0, 0));
    }

    // ── Selection contains ─────────────────────────────────────────

    #[test]
    fn selection_contains_start_cell() {
        let sel = Selection {
            start: (0, 5),
            end: (0, 10),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        assert!(selection_contains(&sel, 0, 5));
    }

    #[test]
    fn selection_contains_end_cell() {
        let sel = Selection {
            start: (0, 5),
            end: (0, 10),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        assert!(selection_contains(&sel, 0, 10));
    }

    #[test]
    fn selection_does_not_contain_outside() {
        let sel = Selection {
            start: (0, 5),
            end: (0, 10),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        assert!(!selection_contains(&sel, 0, 4));
        assert!(!selection_contains(&sel, 0, 11));
    }

    #[test]
    fn selection_multiline_contains_middle_row() {
        let sel = Selection {
            start: (0, 5),
            end: (2, 10),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        // Middle row (row 1) should be fully selected
        assert!(selection_contains(&sel, 1, 0));
        assert!(selection_contains(&sel, 1, 79));
    }

    #[test]
    fn selection_reversed_start_end() {
        // User drags from right to left: end < start in same row
        let sel = Selection {
            start: (0, 10),
            end: (0, 5),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        assert!(selection_contains(&sel, 0, 7));
    }

    // ── Word boundary detection ────────────────────────────────────

    #[test]
    fn word_boundaries_single_word() {
        let cells = make_row("hello world", 20);
        let (start, end) = find_word_boundaries(&cells, 0, 2, 20);
        assert_eq!((start, end), (0, 4));
    }

    #[test]
    fn word_boundaries_at_space() {
        let cells = make_row("hello world", 20);
        let (start, end) = find_word_boundaries(&cells, 0, 5, 20);
        assert_eq!((start, end), (5, 5));
    }

    #[test]
    fn word_boundaries_with_punctuation() {
        let cells = make_row("foo.bar baz", 20);
        let (start, end) = find_word_boundaries(&cells, 0, 0, 20);
        assert_eq!((start, end), (0, 2));
    }

    // ── Line selection ─────────────────────────────────────────────

    #[test]
    fn line_selection_spans_full_row() {
        let sel = Selection {
            start: (1, 0),
            end: (1, 19),
            selection_type: SelectionType::Line,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        for col in 0..20 {
            assert!(selection_contains(&sel, 1, col));
        }
    }

    // ── Selected text extraction ───────────────────────────────────

    #[test]
    fn selected_text_single_line() {
        let cells = make_row("hello world", 20);
        let sel = Selection {
            start: (0, 0),
            end: (0, 4),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        let text = selected_text(&cells, &sel, 20, 0);
        assert_eq!(text, "hello");
    }

    #[test]
    fn selected_text_multiline_trims_trailing_spaces() {
        let mut cells = make_row("hello world", 20);
        cells.extend(make_row("second line!", 20));
        let sel = Selection {
            start: (0, 6),
            end: (1, 5),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        let text = selected_text(&cells, &sel, 20, 0);
        assert_eq!(text, "world\nsecond");
    }

    // ── Apply selection flags ──────────────────────────────────────

    #[test]
    fn apply_selection_flags_sets_selected_flag() {
        let mut cells = make_row("hello", 10);
        let sel = Selection {
            start: (0, 0),
            end: (0, 4),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 10, 0);
        assert_ne!(cells[0].flags & CELL_FLAG_SELECTED, 0);
        assert_ne!(cells[4].flags & CELL_FLAG_SELECTED, 0);
    }

    #[test]
    fn apply_selection_flags_does_not_set_outside() {
        let mut cells = make_row("hello world", 20);
        let sel = Selection {
            start: (0, 0),
            end: (0, 4),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 20, 0);
        assert_eq!(cells[5].flags & CELL_FLAG_SELECTED, 0);
    }

    // ── Selection cleared ──────────────────────────────────────────

    #[test]
    fn cleared_selection_flags_reset_to_zero() {
        let mut cells = make_row("hello", 10);
        let sel = Selection {
            start: (0, 0),
            end: (0, 4),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 10, 0);
        assert_ne!(cells[0].flags & CELL_FLAG_SELECTED, 0);
        // Clear by applying empty-range or resetting flags
        for cell in cells.iter_mut() {
            cell.flags &= !CELL_FLAG_SELECTED;
        }
        assert_eq!(cells[0].flags & CELL_FLAG_SELECTED, 0);
    }

    // ── Visual-block selection ──────────────────────────────────────

    #[test]
    fn visual_block_contains_only_rectangle() {
        let sel = Selection {
            start: (0, 2),
            end: (2, 5),
            selection_type: SelectionType::VisualBlock,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        // Inside the rectangle
        assert!(selection_contains(&sel, 0, 3));
        assert!(selection_contains(&sel, 1, 2));
        assert!(selection_contains(&sel, 1, 5));
        assert!(selection_contains(&sel, 2, 4));
        // Outside the column range
        assert!(!selection_contains(&sel, 1, 1));
        assert!(!selection_contains(&sel, 1, 6));
        // Outside the row range
        assert!(!selection_contains(&sel, 3, 3));
    }

    #[test]
    fn visual_block_apply_flags_rectangle() {
        let mut cells = make_row("hello world", 20);
        cells.extend(make_row("second line!", 20));
        cells.extend(make_row("third  line!", 20));
        let sel = Selection {
            start: (0, 2),
            end: (2, 5),
            selection_type: SelectionType::VisualBlock,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 20, 0);
        // Row 0: cols 2-5 selected
        assert_ne!(cells[2].flags & CELL_FLAG_SELECTED, 0);
        assert_ne!(cells[5].flags & CELL_FLAG_SELECTED, 0);
        assert_eq!(cells[1].flags & CELL_FLAG_SELECTED, 0);
        assert_eq!(cells[6].flags & CELL_FLAG_SELECTED, 0);
        // Row 1: cols 2-5 selected
        assert_ne!(cells[22].flags & CELL_FLAG_SELECTED, 0);
        assert_ne!(cells[25].flags & CELL_FLAG_SELECTED, 0);
        assert_eq!(cells[21].flags & CELL_FLAG_SELECTED, 0);
    }

    #[test]
    fn selected_text_block_extracts_rectangle() {
        let mut cells = make_row("hello world", 20);
        cells.extend(make_row("abcde fghij", 20));
        let sel = Selection {
            start: (0, 0),
            end: (1, 4),
            selection_type: SelectionType::VisualBlock,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        let text = selected_text_block(&cells, &sel, 20, 0);
        assert_eq!(text, "hello\nabcde");
    }

    #[test]
    fn selected_text_lines_full_rows() {
        let mut cells = make_row("first line", 20);
        cells.extend(make_row("second line", 20));
        cells.extend(make_row("third line", 20));
        let sel = Selection {
            start: (0, 0),
            end: (1, 19),
            selection_type: SelectionType::Line,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        let text = selected_text_lines(&cells, &sel, 20, 0);
        assert_eq!(text, "first line\nsecond line");
    }

    // ── Absolute coordinate with display_offset ────────────────────

    #[test]
    fn selection_with_scroll_offset_maps_correctly() {
        // 3 viewport rows, selection at absolute rows -2..-1 (scrollback)
        // display_offset=2 means viewport row 0 = abs -2, row 1 = abs -1, row 2 = abs 0
        let mut cells = make_row("scrollback1", 20);
        cells.extend(make_row("scrollback2", 20));
        cells.extend(make_row("screen line", 20));
        let sel = Selection {
            start: (-2, 0),
            end: (-2, 10),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        // display_offset=2: abs -2 + 2 = viewport row 0
        let text = selected_text(&cells, &sel, 20, 2);
        assert_eq!(text, "scrollback1");
    }

    #[test]
    fn apply_flags_with_scroll_offset() {
        let mut cells = make_row("line0", 10);
        cells.extend(make_row("line1", 10));
        // Selection at absolute row -1 with display_offset=1 → viewport row 0
        let sel = Selection {
            start: (-1, 0),
            end: (-1, 4),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 10, 1);
        assert_ne!(cells[0].flags & CELL_FLAG_SELECTED, 0); // viewport row 0
        assert_eq!(cells[10].flags & CELL_FLAG_SELECTED, 0); // viewport row 1 unaffected
    }

    #[test]
    fn selection_offscreen_skipped() {
        let mut cells = make_row("visible", 10);
        // Selection at absolute row -5, display_offset=1 → viewport row -4 (off-screen)
        let sel = Selection {
            start: (-5, 0),
            end: (-5, 6),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 10, 1);
        // No cell should be flagged
        for cell in &cells {
            assert_eq!(cell.flags & CELL_FLAG_SELECTED, 0);
        }
    }

    // ── Selection clamping to occupied cells (Bug 1) ─────────────

    #[test]
    fn selection_clamps_to_last_occupied_column() {
        // Row 0: "hello" (5 chars) in 80-column grid
        // Row 1: "world!" (6 chars) in 80-column grid
        let mut cells = make_row("hello", 80);
        cells.extend(make_row("world!", 80));
        let sel = Selection {
            start: (0, 0),
            end: (1, 5),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 80, 0);
        // Row 0: "hello" occupies cols 0-4, so cols 0-4 should be selected
        for col in 0..5 {
            assert_ne!(cells[col].flags & CELL_FLAG_SELECTED, 0,
                "row 0, col {} should be selected", col);
        }
        // Row 0: cols beyond "hello" (5-79) should NOT be selected
        for col in 5..80 {
            assert_eq!(cells[col].flags & CELL_FLAG_SELECTED, 0,
                "row 0, col {} should NOT be selected (empty)", col);
        }
        // Row 1: cols 0-5 should be selected (end of selection)
        for col in 0..6 {
            assert_ne!(cells[80 + col].flags & CELL_FLAG_SELECTED, 0,
                "row 1, col {} should be selected", col);
        }
    }

    #[test]
    fn selection_middle_rows_clamped_to_content() {
        // 3 rows in 40-column grid
        let mut cells = make_row("start line", 40);     // 10 chars
        cells.extend(make_row("mid", 40));               // 3 chars
        cells.extend(make_row("end line here", 40));     // 13 chars
        let sel = Selection {
            start: (0, 0),
            end: (2, 12),
            selection_type: SelectionType::Range,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 40, 0);
        // Row 0 (start): clamped to last occupied = col 9 ("start line")
        assert_ne!(cells[9].flags & CELL_FLAG_SELECTED, 0);
        assert_eq!(cells[10].flags & CELL_FLAG_SELECTED, 0);
        // Row 1 (middle): clamped to last occupied = col 2 ("mid")
        assert_ne!(cells[40].flags & CELL_FLAG_SELECTED, 0);  // col 0
        assert_ne!(cells[42].flags & CELL_FLAG_SELECTED, 0);  // col 2
        assert_eq!(cells[43].flags & CELL_FLAG_SELECTED, 0);  // col 3 empty
        assert_eq!(cells[79].flags & CELL_FLAG_SELECTED, 0);  // col 39 empty
        // Row 2 (end): uses selection end col = 12
        assert_ne!(cells[80 + 12].flags & CELL_FLAG_SELECTED, 0);
        assert_eq!(cells[80 + 13].flags & CELL_FLAG_SELECTED, 0);
    }

    #[test]
    fn line_selection_highlights_full_row_width() {
        // Line selection should NOT clamp — it selects full rows
        let mut cells = make_row("short", 20);
        let sel = Selection {
            start: (0, 0),
            end: (0, 19),
            selection_type: SelectionType::Line,
            start_side: Side::Left,
            end_side: Side::Right,
        };
        apply_selection_flags(&mut cells, &sel, 20, 0);
        // All 20 columns should be selected for Line type
        for col in 0..20 {
            assert_ne!(cells[col].flags & CELL_FLAG_SELECTED, 0,
                "line selection: col {} should be selected", col);
        }
    }
}
