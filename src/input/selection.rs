// Text selection: state management, coordinate conversion, and text extraction.

use crate::renderer::grid_renderer::{GridCell, CELL_FLAG_SELECTED};

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
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    /// Start position (row, col).
    pub start: (usize, usize),
    /// End position (row, col).
    pub end: (usize, usize),
    /// Type of selection.
    pub selection_type: SelectionType,
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
fn normalize(selection: &Selection) -> ((usize, usize), (usize, usize)) {
    let (s, e) = (selection.start, selection.end);
    if s.0 < e.0 || (s.0 == e.0 && s.1 <= e.1) {
        (s, e)
    } else {
        (e, s)
    }
}

/// Check if a cell at (row, col) is within the selection, in reading order.
pub fn selection_contains(selection: &Selection, row: usize, col: usize) -> bool {
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
pub fn selected_text(cells: &[GridCell], selection: &Selection, cols: usize) -> String {
    let (start, end) = normalize(selection);
    let mut lines = Vec::new();

    for row in start.0..=end.0 {
        let col_start = if row == start.0 { start.1 } else { 0 };
        let col_end = if row == end.0 { end.1 } else { cols - 1 };
        let row_offset = row * cols;

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

/// Set CELL_FLAG_SELECTED on all cells within the selection.
pub fn apply_selection_flags(cells: &mut [GridCell], selection: &Selection, cols: usize) {
    let (start, end) = normalize(selection);
    let rows = cells.len() / cols;

    if selection.selection_type == SelectionType::VisualBlock {
        let col_min = start.1.min(end.1);
        let col_max = start.1.max(end.1);
        for row in start.0..=end.0.min(rows - 1) {
            let row_offset = row * cols;
            for col in col_min..=col_max.min(cols - 1) {
                cells[row_offset + col].flags |= CELL_FLAG_SELECTED;
            }
        }
        return;
    }

    for row in start.0..=end.0.min(rows - 1) {
        let col_start = if row == start.0 { start.1 } else { 0 };
        let col_end = if row == end.0 { end.1 } else { cols - 1 };
        let row_offset = row * cols;

        for col in col_start..=col_end.min(cols - 1) {
            cells[row_offset + col].flags |= CELL_FLAG_SELECTED;
        }
    }
}

/// Extract selected text from a visual-block (rectangular) selection.
/// Each row's selected columns are extracted and joined with newlines.
pub fn selected_text_block(cells: &[GridCell], selection: &Selection, cols: usize) -> String {
    let (start, end) = normalize(selection);
    let col_min = start.1.min(end.1);
    let col_max = start.1.max(end.1);
    let mut lines = Vec::new();

    for row in start.0..=end.0 {
        let row_offset = row * cols;
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
pub fn selected_text_lines(cells: &[GridCell], selection: &Selection, cols: usize) -> String {
    let (start, end) = normalize(selection);
    let mut lines = Vec::new();

    for row in start.0..=end.0 {
        let row_offset = row * cols;
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
    use crate::config::theme::Color;

    /// Helper: create a row of cells from a string, padded to `cols` with spaces.
    fn make_row(text: &str, cols: usize) -> Vec<GridCell> {
        let fg = Color::new(1.0, 1.0, 1.0, 1.0);
        let bg = Color::new(0.0, 0.0, 0.0, 1.0);
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
        };
        assert!(selection_contains(&sel, 0, 5));
    }

    #[test]
    fn selection_contains_end_cell() {
        let sel = Selection {
            start: (0, 5),
            end: (0, 10),
            selection_type: SelectionType::Range,
        };
        assert!(selection_contains(&sel, 0, 10));
    }

    #[test]
    fn selection_does_not_contain_outside() {
        let sel = Selection {
            start: (0, 5),
            end: (0, 10),
            selection_type: SelectionType::Range,
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
        };
        let text = selected_text(&cells, &sel, 20);
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
        };
        let text = selected_text(&cells, &sel, 20);
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
        };
        apply_selection_flags(&mut cells, &sel, 10);
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
        };
        apply_selection_flags(&mut cells, &sel, 20);
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
        };
        apply_selection_flags(&mut cells, &sel, 10);
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
        };
        apply_selection_flags(&mut cells, &sel, 20);
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
        };
        let text = selected_text_block(&cells, &sel, 20);
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
        };
        let text = selected_text_lines(&cells, &sel, 20);
        assert_eq!(text, "first line\nsecond line");
    }
}
