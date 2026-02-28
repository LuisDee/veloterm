// Diff view rendering helpers for the git review right panel.
// Provides data structures and pure functions for rendering the side-by-side diff.

use crate::git_review::diff::{AlignedRow, ChangeType, DiffHunk, DiffType, FileDiff};

/// Color representation for diff rendering (RGBA 0.0..1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl DiffColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

// Diff color palette (dark theme)
pub const COLOR_ADDED_BG: DiffColor = DiffColor::new(0.0, 0.4, 0.0, 0.15);
pub const COLOR_DELETED_BG: DiffColor = DiffColor::new(0.6, 0.0, 0.0, 0.15);
pub const COLOR_MODIFIED_BG: DiffColor = DiffColor::new(0.6, 0.5, 0.0, 0.15);
pub const COLOR_CONTEXT_BG: DiffColor = DiffColor::new(0.0, 0.0, 0.0, 0.0);

pub const COLOR_ADDED_INDICATOR: DiffColor = DiffColor::new(0.2, 0.8, 0.2, 1.0);
pub const COLOR_DELETED_INDICATOR: DiffColor = DiffColor::new(0.9, 0.2, 0.2, 1.0);
pub const COLOR_MODIFIED_INDICATOR: DiffColor = DiffColor::new(0.9, 0.7, 0.1, 1.0);

pub const COLOR_HUNK_HEADER_BG: DiffColor = DiffColor::new(0.15, 0.15, 0.2, 1.0);
pub const COLOR_LINE_NUMBER: DiffColor = DiffColor::new(0.5, 0.5, 0.5, 1.0);

/// Row height in pixels for the diff view.
pub const ROW_HEIGHT: f32 = 20.0;

/// Line number gutter width in pixels.
pub const GUTTER_WIDTH: f32 = 48.0;

/// Change indicator strip width in pixels.
pub const INDICATOR_WIDTH: f32 = 4.0;

/// Hunk header height in pixels.
pub const HUNK_HEADER_HEIGHT: f32 = 28.0;

/// What to display in the diff view.
#[derive(Debug, Clone)]
pub enum DiffViewContent {
    /// No file selected — show empty state message.
    Empty,
    /// A file diff to display.
    Diff(FileDiff),
    /// Binary file — show message with sizes.
    Binary {
        path: String,
        old_size: Option<u64>,
        new_size: Option<u64>,
    },
}

/// A flattened row for rendering, combining hunks into a single list.
#[derive(Debug, Clone)]
pub enum FlatRow {
    HunkHeader { header: String, index: usize },
    AlignedRow { row: AlignedRow, hunk_index: usize },
}

/// Flatten a FileDiff's hunks into a single list of renderable rows.
pub fn flatten_diff(diff: &FileDiff) -> Vec<FlatRow> {
    let mut flat = Vec::new();
    for (hunk_idx, hunk) in diff.hunks.iter().enumerate() {
        flat.push(FlatRow::HunkHeader {
            header: hunk.header.clone(),
            index: hunk_idx,
        });
        for row in &hunk.rows {
            flat.push(FlatRow::AlignedRow {
                row: row.clone(),
                hunk_index: hunk_idx,
            });
        }
    }
    flat
}

/// Compute the total height of the diff content in pixels.
pub fn total_content_height(diff: &FileDiff) -> f32 {
    let mut height = 0.0;
    for hunk in &diff.hunks {
        height += HUNK_HEADER_HEIGHT;
        height += hunk.rows.len() as f32 * ROW_HEIGHT;
    }
    height
}

/// Compute the visible row range given a scroll offset and viewport height.
/// Returns (first_visible_index, last_visible_index) into the flat row list.
pub fn visible_row_range(
    flat_rows: &[FlatRow],
    scroll_offset: f32,
    viewport_height: f32,
) -> (usize, usize) {
    if flat_rows.is_empty() {
        return (0, 0);
    }

    let mut y = 0.0;
    let mut first = None;
    let mut last = 0;

    for (i, row) in flat_rows.iter().enumerate() {
        let row_height = match row {
            FlatRow::HunkHeader { .. } => HUNK_HEADER_HEIGHT,
            FlatRow::AlignedRow { .. } => ROW_HEIGHT,
        };

        let row_bottom = y + row_height;

        if first.is_none() && row_bottom > scroll_offset {
            first = Some(i);
        }

        if y < scroll_offset + viewport_height {
            last = i;
        } else {
            break;
        }

        y += row_height;
    }

    let first = first.unwrap_or(0);
    // Include one extra row past the end for partial visibility
    let last = (last + 1).min(flat_rows.len());
    (first, last)
}

/// Get the y-offset of a specific flat row index.
pub fn row_y_offset(flat_rows: &[FlatRow], index: usize) -> f32 {
    let mut y = 0.0;
    for (i, row) in flat_rows.iter().enumerate() {
        if i == index {
            return y;
        }
        y += match row {
            FlatRow::HunkHeader { .. } => HUNK_HEADER_HEIGHT,
            FlatRow::AlignedRow { .. } => ROW_HEIGHT,
        };
    }
    y
}

/// Get the background color for a change type.
pub fn bg_color_for_change(change_type: ChangeType) -> DiffColor {
    match change_type {
        ChangeType::Context => COLOR_CONTEXT_BG,
        ChangeType::Added => COLOR_ADDED_BG,
        ChangeType::Deleted => COLOR_DELETED_BG,
        ChangeType::Modified => COLOR_MODIFIED_BG,
    }
}

/// Get the indicator strip color for a change type.
pub fn indicator_color_for_change(change_type: ChangeType) -> DiffColor {
    match change_type {
        ChangeType::Context => COLOR_CONTEXT_BG,
        ChangeType::Added => COLOR_ADDED_INDICATOR,
        ChangeType::Deleted => COLOR_DELETED_INDICATOR,
        ChangeType::Modified => COLOR_MODIFIED_INDICATOR,
    }
}

/// Format a line number for display in the gutter.
/// Returns empty string for None (spacer lines).
pub fn format_line_number(line_number: Option<usize>, width: usize) -> String {
    match line_number {
        Some(n) => format!("{:>width$}", n, width = width),
        None => " ".repeat(width),
    }
}

/// Compute the gutter width needed for a given max line number.
/// Minimum 3 characters wide, plus padding.
pub fn gutter_char_width(max_line: usize) -> usize {
    let digits = if max_line == 0 {
        1
    } else {
        (max_line as f64).log10().floor() as usize + 1
    };
    digits.max(3)
}

/// Get the header text for a diff type.
pub fn diff_type_header(diff_type: &DiffType, path: &str) -> String {
    match diff_type {
        DiffType::Modified => format!("{} (Modified)", path),
        DiffType::Added => format!("{} (Added)", path),
        DiffType::Deleted => format!("{} (Deleted)", path),
        DiffType::Renamed { from } => format!("{} → {} (Renamed)", from, path),
        DiffType::Binary => format!("{} (Binary)", path),
    }
}

/// Compute the maximum line number across all hunks (for gutter sizing).
pub fn max_line_number(diff: &FileDiff) -> usize {
    let mut max = 0usize;
    for hunk in &diff.hunks {
        for row in &hunk.rows {
            if let Some(left) = &row.left {
                if let Some(n) = left.line_number {
                    max = max.max(n);
                }
            }
            if let Some(right) = &row.right {
                if let Some(n) = right.line_number {
                    max = max.max(n);
                }
            }
        }
    }
    max
}

/// Count total rows across all hunks.
pub fn total_row_count(diff: &FileDiff) -> usize {
    diff.hunks.iter().map(|h| h.rows.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_review::diff::{DiffLine, DiffHunk};

    fn make_context_row(old_num: usize, new_num: usize, content: &str) -> AlignedRow {
        AlignedRow {
            left: Some(DiffLine {
                content: content.to_string(),
                line_number: Some(old_num),
                change_type: ChangeType::Context,
            }),
            right: Some(DiffLine {
                content: content.to_string(),
                line_number: Some(new_num),
                change_type: ChangeType::Context,
            }),
        }
    }

    fn make_add_row(new_num: usize, content: &str) -> AlignedRow {
        AlignedRow {
            left: None,
            right: Some(DiffLine {
                content: content.to_string(),
                line_number: Some(new_num),
                change_type: ChangeType::Added,
            }),
        }
    }

    fn make_delete_row(old_num: usize, content: &str) -> AlignedRow {
        AlignedRow {
            left: Some(DiffLine {
                content: content.to_string(),
                line_number: Some(old_num),
                change_type: ChangeType::Deleted,
            }),
            right: None,
        }
    }

    fn make_modified_row(
        old_num: usize,
        new_num: usize,
        old_content: &str,
        new_content: &str,
    ) -> AlignedRow {
        AlignedRow {
            left: Some(DiffLine {
                content: old_content.to_string(),
                line_number: Some(old_num),
                change_type: ChangeType::Modified,
            }),
            right: Some(DiffLine {
                content: new_content.to_string(),
                line_number: Some(new_num),
                change_type: ChangeType::Modified,
            }),
        }
    }

    fn make_test_diff() -> FileDiff {
        FileDiff {
            path: "test.rs".to_string(),
            hunks: vec![
                DiffHunk {
                    header: "@@ -1,3 +1,4 @@".to_string(),
                    old_start: 1,
                    new_start: 1,
                    rows: vec![
                        make_context_row(1, 1, "line1"),
                        make_modified_row(2, 2, "old", "new"),
                        make_add_row(3, "inserted"),
                        make_context_row(3, 4, "line3"),
                    ],
                },
                DiffHunk {
                    header: "@@ -10,2 +11,1 @@".to_string(),
                    old_start: 10,
                    new_start: 11,
                    rows: vec![
                        make_delete_row(10, "removed"),
                        make_context_row(11, 11, "kept"),
                    ],
                },
            ],
            diff_type: DiffType::Modified,
        }
    }

    // -- bg_color_for_change --

    #[test]
    fn bg_color_context_is_transparent() {
        let c = bg_color_for_change(ChangeType::Context);
        assert!((c.a - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn bg_color_added_is_green() {
        let c = bg_color_for_change(ChangeType::Added);
        assert!(c.g > c.r);
        assert!(c.a > 0.0);
    }

    #[test]
    fn bg_color_deleted_is_red() {
        let c = bg_color_for_change(ChangeType::Deleted);
        assert!(c.r > c.g);
        assert!(c.a > 0.0);
    }

    #[test]
    fn bg_color_modified_is_yellow() {
        let c = bg_color_for_change(ChangeType::Modified);
        assert!(c.r > 0.0 && c.g > 0.0);
        assert!(c.a > 0.0);
    }

    // -- indicator_color_for_change --

    #[test]
    fn indicator_added_green() {
        let c = indicator_color_for_change(ChangeType::Added);
        assert!(c.g > c.r);
        assert!((c.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn indicator_deleted_red() {
        let c = indicator_color_for_change(ChangeType::Deleted);
        assert!(c.r > c.g);
        assert!((c.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn indicator_modified_yellow() {
        let c = indicator_color_for_change(ChangeType::Modified);
        assert!(c.r > 0.0 && c.g > 0.0);
        assert!((c.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn indicator_context_transparent() {
        let c = indicator_color_for_change(ChangeType::Context);
        assert!((c.a - 0.0).abs() < f32::EPSILON);
    }

    // -- format_line_number --

    #[test]
    fn format_line_number_some() {
        assert_eq!(format_line_number(Some(42), 4), "  42");
    }

    #[test]
    fn format_line_number_none() {
        assert_eq!(format_line_number(None, 4), "    ");
    }

    #[test]
    fn format_line_number_single_digit() {
        assert_eq!(format_line_number(Some(5), 3), "  5");
    }

    #[test]
    fn format_line_number_large() {
        assert_eq!(format_line_number(Some(12345), 5), "12345");
    }

    // -- gutter_char_width --

    #[test]
    fn gutter_width_small() {
        assert_eq!(gutter_char_width(1), 3);
        assert_eq!(gutter_char_width(9), 3);
        assert_eq!(gutter_char_width(99), 3);
        assert_eq!(gutter_char_width(999), 3);
    }

    #[test]
    fn gutter_width_four_digits() {
        assert_eq!(gutter_char_width(1000), 4);
        assert_eq!(gutter_char_width(9999), 4);
    }

    #[test]
    fn gutter_width_five_digits() {
        assert_eq!(gutter_char_width(10000), 5);
    }

    #[test]
    fn gutter_width_zero() {
        assert_eq!(gutter_char_width(0), 3);
    }

    // -- diff_type_header --

    #[test]
    fn header_modified() {
        assert_eq!(
            diff_type_header(&DiffType::Modified, "src/main.rs"),
            "src/main.rs (Modified)"
        );
    }

    #[test]
    fn header_added() {
        assert_eq!(
            diff_type_header(&DiffType::Added, "new.rs"),
            "new.rs (Added)"
        );
    }

    #[test]
    fn header_deleted() {
        assert_eq!(
            diff_type_header(&DiffType::Deleted, "old.rs"),
            "old.rs (Deleted)"
        );
    }

    #[test]
    fn header_renamed() {
        assert_eq!(
            diff_type_header(
                &DiffType::Renamed {
                    from: "old.rs".into()
                },
                "new.rs"
            ),
            "old.rs → new.rs (Renamed)"
        );
    }

    #[test]
    fn header_binary() {
        assert_eq!(
            diff_type_header(&DiffType::Binary, "image.png"),
            "image.png (Binary)"
        );
    }

    // -- max_line_number --

    #[test]
    fn max_line_number_finds_highest() {
        let diff = make_test_diff();
        assert_eq!(max_line_number(&diff), 11);
    }

    #[test]
    fn max_line_number_empty_diff() {
        let diff = FileDiff {
            path: "empty.rs".to_string(),
            hunks: vec![],
            diff_type: DiffType::Modified,
        };
        assert_eq!(max_line_number(&diff), 0);
    }

    // -- total_row_count --

    #[test]
    fn total_row_count_sums_hunks() {
        let diff = make_test_diff();
        assert_eq!(total_row_count(&diff), 6); // 4 + 2
    }

    #[test]
    fn total_row_count_empty() {
        let diff = FileDiff {
            path: "x".to_string(),
            hunks: vec![],
            diff_type: DiffType::Modified,
        };
        assert_eq!(total_row_count(&diff), 0);
    }

    // -- flatten_diff --

    #[test]
    fn flatten_produces_correct_count() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // 2 hunk headers + 6 aligned rows = 8
        assert_eq!(flat.len(), 8);
    }

    #[test]
    fn flatten_starts_with_hunk_header() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        assert!(matches!(flat[0], FlatRow::HunkHeader { index: 0, .. }));
    }

    #[test]
    fn flatten_hunk_header_has_correct_text() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        if let FlatRow::HunkHeader { header, .. } = &flat[0] {
            assert_eq!(header, "@@ -1,3 +1,4 @@");
        } else {
            panic!("Expected hunk header");
        }
    }

    #[test]
    fn flatten_second_hunk_at_correct_position() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // First hunk: header + 4 rows = 5 items. Second hunk header at index 5.
        assert!(matches!(flat[5], FlatRow::HunkHeader { index: 1, .. }));
    }

    // -- total_content_height --

    #[test]
    fn content_height_correct() {
        let diff = make_test_diff();
        let expected = 2.0 * HUNK_HEADER_HEIGHT + 6.0 * ROW_HEIGHT;
        assert!((total_content_height(&diff) - expected).abs() < f32::EPSILON);
    }

    // -- visible_row_range --

    #[test]
    fn visible_range_no_scroll() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // Viewport large enough for everything
        let (first, last) = visible_row_range(&flat, 0.0, 10000.0);
        assert_eq!(first, 0);
        assert_eq!(last, flat.len());
    }

    #[test]
    fn visible_range_scrolled_past_first_hunk() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // Scroll past first hunk header + 4 rows
        let scroll = HUNK_HEADER_HEIGHT + 4.0 * ROW_HEIGHT;
        let (first, last) = visible_row_range(&flat, scroll, ROW_HEIGHT * 2.0);
        // Should start at second hunk header (index 5)
        assert_eq!(first, 5);
        assert!(last <= flat.len());
    }

    #[test]
    fn visible_range_empty() {
        let (first, last) = visible_row_range(&[], 0.0, 100.0);
        assert_eq!(first, 0);
        assert_eq!(last, 0);
    }

    #[test]
    fn visible_range_small_viewport() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // Very small viewport — only shows first hunk header
        let (first, last) = visible_row_range(&flat, 0.0, HUNK_HEADER_HEIGHT);
        assert_eq!(first, 0);
        // Should include at least the first row
        assert!(last >= 1);
    }

    // -- row_y_offset --

    #[test]
    fn row_y_offset_first_row() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        assert!((row_y_offset(&flat, 0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn row_y_offset_after_header() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // Row 1 is after the first hunk header
        assert!((row_y_offset(&flat, 1) - HUNK_HEADER_HEIGHT).abs() < f32::EPSILON);
    }

    #[test]
    fn row_y_offset_second_hunk() {
        let diff = make_test_diff();
        let flat = flatten_diff(&diff);
        // Second hunk header at index 5: HUNK_HEADER_HEIGHT + 4 * ROW_HEIGHT
        let expected = HUNK_HEADER_HEIGHT + 4.0 * ROW_HEIGHT;
        assert!((row_y_offset(&flat, 5) - expected).abs() < f32::EPSILON);
    }

    // -- DiffViewContent --

    #[test]
    fn diff_view_content_empty_variant() {
        let content = DiffViewContent::Empty;
        assert!(matches!(content, DiffViewContent::Empty));
    }

    #[test]
    fn diff_view_content_binary_variant() {
        let content = DiffViewContent::Binary {
            path: "img.png".to_string(),
            old_size: Some(1024),
            new_size: Some(2048),
        };
        if let DiffViewContent::Binary {
            path,
            old_size,
            new_size,
        } = content
        {
            assert_eq!(path, "img.png");
            assert_eq!(old_size, Some(1024));
            assert_eq!(new_size, Some(2048));
        }
    }
}
