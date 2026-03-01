// File browser view — iced widget rendering for the file explorer panel.
//
// Provides functions to build the iced widget tree for the file tree,
// breadcrumb bar, and virtual scrolling.

use crate::file_browser::tree::{
    breadcrumb_segments, file_icon_info, visible_range, IconInfo, NodeType, TreeNavState, VisibleRow,
};
use std::path::Path;

/// Fixed row height in logical pixels for virtual scrolling.
pub const ROW_HEIGHT: f32 = 28.0;

/// Indentation per depth level in logical pixels.
pub const INDENT_PER_DEPTH: f32 = 18.0;

/// Compute the total content height for the file tree.
pub fn total_content_height(total_rows: usize) -> f32 {
    total_rows as f32 * ROW_HEIGHT
}

/// Compute indentation width for a given depth level (in logical pixels).
pub fn indent_width(depth: usize) -> f32 {
    depth as f32 * INDENT_PER_DEPTH
}

/// Chevron character for directory expand/collapse state.
pub fn chevron(expanded: bool) -> &'static str {
    if expanded {
        "\u{25BE}" // ▾ down-pointing small triangle
    } else {
        "\u{25B8}" // ▸ right-pointing small triangle
    }
}

/// Get a display icon for a visible row (legacy, returns just the icon string).
pub fn row_icon(row: &VisibleRow) -> &'static str {
    row_icon_info(row).icon
}

/// Get full icon info (icon + color hint) for a visible row.
pub fn row_icon_info(row: &VisibleRow) -> IconInfo {
    match &row.node_type {
        NodeType::Directory => {
            if row.expanded {
                IconInfo { icon: "\u{1F4C2}", color_hint: "default" } // open folder
            } else {
                IconInfo { icon: "\u{1F4C1}", color_hint: "default" } // closed folder
            }
        }
        NodeType::File { extension, .. } => {
            file_icon_info(extension.as_deref(), Some(&row.name))
        }
        NodeType::Symlink { .. } => IconInfo { icon: "\u{1F517}", color_hint: "default" }, // link symbol
    }
}

/// Build the indent guide prefix string for a tree row.
/// Returns empty string for depth-0 rows. For deeper rows, uses box-drawing characters.
pub fn indent_guide_prefix(row: &VisibleRow) -> String {
    if row.depth == 0 {
        return String::new();
    }

    let mut prefix = String::new();

    // For each ancestor level (skip level 0 — root never has guides)
    for d in 1..row.depth {
        if d < row.ancestor_has_next_sibling.len() && row.ancestor_has_next_sibling[d] {
            prefix.push_str("\u{2502} "); // │ + space (continuing line)
        } else {
            prefix.push_str("  "); // blank (no continuing line)
        }
    }

    // Current item connector
    if row.is_last_child {
        prefix.push_str("\u{2514}\u{2500}"); // └─
    } else {
        prefix.push_str("\u{251C}\u{2500}"); // ├─
    }

    prefix
}

/// Compact single-child directory chains in the visible rows.
///
/// When a directory has exactly one child that is also a directory, merge them
/// into a single row with a combined name (e.g. "a/b/c"). The merged row uses
/// the depth of the first directory in the chain and inherits other fields from
/// the last directory in the chain.
pub fn compact_visible_rows(rows: Vec<VisibleRow>) -> Vec<VisibleRow> {
    if rows.is_empty() {
        return rows;
    }

    let mut result: Vec<VisibleRow> = Vec::with_capacity(rows.len());
    let mut i = 0;

    while i < rows.len() {
        let row = &rows[i];

        // Only try to compact directories (skip root at depth 0)
        if !matches!(row.node_type, NodeType::Directory) || !row.expanded || row.depth == 0 {
            result.push(row.clone());
            i += 1;
            continue;
        }

        // Check if this expanded dir has exactly one child that is also an expanded dir
        // The child would be the next row at depth + 1
        let start_depth = row.depth;
        let mut chain_name = row.name.clone();
        let mut chain_end = i;

        let mut j = i + 1;
        while j < rows.len() {
            let next = &rows[j];
            // Must be exactly one level deeper and a directory
            if next.depth != rows[chain_end].depth + 1 {
                break;
            }
            if !matches!(next.node_type, NodeType::Directory) || !next.expanded {
                break;
            }
            // Check: the previous dir at chain_end must have exactly one child
            // in the visible rows at depth chain_end.depth + 1. Count children
            // of the current chain end: they are consecutive rows at depth+1.
            let parent_depth = rows[chain_end].depth;
            let mut child_count = 0;
            for k in (chain_end + 1)..rows.len() {
                if rows[k].depth <= parent_depth {
                    break;
                }
                if rows[k].depth == parent_depth + 1 {
                    child_count += 1;
                }
            }
            if child_count != 1 {
                break;
            }

            chain_name = format!("{}/{}", chain_name, next.name);
            chain_end = j;
            j += 1;
        }

        if chain_end > i {
            // Create a compacted row
            let last = &rows[chain_end];
            result.push(VisibleRow {
                index: last.index,
                depth: start_depth,
                name: chain_name,
                node_type: last.node_type.clone(),
                expanded: last.expanded,
                has_children: last.has_children,
                is_last_child: row.is_last_child,
                ancestor_has_next_sibling: row.ancestor_has_next_sibling.clone(),
            });
            // Skip the compacted rows (the chain interior dirs), continue from
            // the row after the last chain dir
            i = chain_end + 1;
        } else {
            result.push(row.clone());
            i += 1;
        }
    }

    result
}

/// Compute which visible rows should be rendered given scroll state.
pub fn compute_visible_slice<'a>(
    all_rows: &'a [VisibleRow],
    scroll_offset: f32,
    viewport_height: f32,
) -> &'a [VisibleRow] {
    let (start, end) = visible_range(scroll_offset, viewport_height, ROW_HEIGHT, all_rows.len());
    &all_rows[start..end]
}

/// State for the file tree view (scroll offset, selection, etc.)
#[derive(Debug, Clone)]
pub struct FileTreeViewState {
    pub scroll_offset: f32,
    pub nav: TreeNavState,
    pub hovered_row: Option<usize>,
    pub selected_file: Option<usize>, // node index of the file being previewed
}

impl FileTreeViewState {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0.0,
            nav: TreeNavState::new(),
            hovered_row: None,
            selected_file: None,
        }
    }

    /// Scroll by delta pixels, clamped to valid range.
    pub fn scroll_by(&mut self, delta: f32, total_rows: usize, viewport_height: f32) {
        let max_offset = (total_content_height(total_rows) - viewport_height).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_offset);
    }

    /// Ensure the selected row is visible by adjusting scroll offset.
    pub fn ensure_selected_visible(&mut self, total_rows: usize, viewport_height: f32) {
        if let Some(row_idx) = self.nav.selected_visible_row {
            let row_top = row_idx as f32 * ROW_HEIGHT;
            let row_bottom = row_top + ROW_HEIGHT;

            if row_top < self.scroll_offset {
                self.scroll_offset = row_top;
            } else if row_bottom > self.scroll_offset + viewport_height {
                self.scroll_offset = row_bottom - viewport_height;
            }
        }
        let max_offset = (total_content_height(total_rows) - viewport_height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_offset);
    }
}

/// Breadcrumb data for rendering.
#[derive(Debug, Clone)]
pub struct BreadcrumbData {
    pub segments: Vec<(String, std::path::PathBuf)>,
}

impl BreadcrumbData {
    pub fn from_path(path: &Path, root: &Path) -> Self {
        Self {
            segments: breadcrumb_segments(path, root),
        }
    }

    /// Get the display text with separators.
    pub fn display_text(&self) -> String {
        self.segments
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_browser::tree::{FileTree, NodeType, VisibleRow};
    use std::path::PathBuf;

    // --- Row height / content height ---

    #[test]
    fn total_content_height_basic() {
        assert!((total_content_height(10) - 280.0).abs() < f32::EPSILON);
        assert!((total_content_height(0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn indent_width_calculation() {
        assert!((indent_width(0) - 0.0).abs() < f32::EPSILON);
        assert!((indent_width(1) - 18.0).abs() < f32::EPSILON);
        assert!((indent_width(3) - 54.0).abs() < f32::EPSILON);
    }

    // --- Chevron ---

    #[test]
    fn chevron_expanded() {
        assert_eq!(chevron(true), "\u{25BE}");
    }

    #[test]
    fn chevron_collapsed() {
        assert_eq!(chevron(false), "\u{25B8}");
    }

    // --- Row icon ---

    #[test]
    fn row_icon_directory_expanded() {
        let row = VisibleRow {
            index: 0,
            depth: 0,
            name: "src".into(),
            node_type: NodeType::Directory,
            expanded: true,
            has_children: true,
            is_last_child: false,
            ancestor_has_next_sibling: vec![],
        };
        assert_eq!(row_icon(&row), "\u{1F4C2}");
    }

    #[test]
    fn row_icon_directory_collapsed() {
        let row = VisibleRow {
            index: 0,
            depth: 0,
            name: "src".into(),
            node_type: NodeType::Directory,
            expanded: false,
            has_children: true,
            is_last_child: false,
            ancestor_has_next_sibling: vec![],
        };
        assert_eq!(row_icon(&row), "\u{1F4C1}");
    }

    #[test]
    fn row_icon_file_with_extension() {
        let row = VisibleRow {
            index: 0,
            depth: 1,
            name: "main.rs".into(),
            node_type: NodeType::File {
                extension: Some("rs".into()),
                size: 100,
            },
            expanded: false,
            has_children: false,
            is_last_child: false,
            ancestor_has_next_sibling: vec![],
        };
        assert_eq!(row_icon(&row), "R");
    }

    // --- Virtual scrolling slice ---

    #[test]
    fn compute_visible_slice_all_visible() {
        let rows: Vec<VisibleRow> = (0..5)
            .map(|i| VisibleRow {
                index: i,
                depth: 0,
                name: format!("item_{i}"),
                node_type: NodeType::File {
                    extension: None,
                    size: 0,
                },
                expanded: false,
                has_children: false,
                is_last_child: false,
                ancestor_has_next_sibling: vec![],
            })
            .collect();

        let slice = compute_visible_slice(&rows, 0.0, 500.0);
        assert_eq!(slice.len(), 5); // all fit
    }

    #[test]
    fn compute_visible_slice_scrolled() {
        let rows: Vec<VisibleRow> = (0..100)
            .map(|i| VisibleRow {
                index: i,
                depth: 0,
                name: format!("item_{i}"),
                node_type: NodeType::File {
                    extension: None,
                    size: 0,
                },
                expanded: false,
                has_children: false,
                is_last_child: false,
                ancestor_has_next_sibling: vec![],
            })
            .collect();

        // Scroll down 2 rows (56px), viewport shows ~10 rows (280px)
        let slice = compute_visible_slice(&rows, 56.0, 280.0);
        assert_eq!(slice[0].index, 2);
        assert!(slice.len() <= 12); // about 10-11 rows visible
    }

    // --- Scroll state ---

    #[test]
    fn scroll_by_clamps_to_zero() {
        let mut state = FileTreeViewState::new();
        state.scroll_by(-100.0, 10, 280.0);
        assert!((state.scroll_offset - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_by_clamps_to_max() {
        let mut state = FileTreeViewState::new();
        // 10 rows * 28px = 280px total, viewport = 200px, max_offset = 80px
        state.scroll_by(1000.0, 10, 200.0);
        assert!((state.scroll_offset - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_by_when_content_fits() {
        let mut state = FileTreeViewState::new();
        // 5 rows * 28px = 140px total, viewport = 500px, max_offset = 0
        state.scroll_by(100.0, 5, 500.0);
        assert!((state.scroll_offset - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ensure_selected_visible_scrolls_down() {
        let mut state = FileTreeViewState::new();
        state.scroll_offset = 0.0;
        state.nav.selected_visible_row = Some(20); // row 20 at 560px
        state.ensure_selected_visible(50, 280.0);
        // Should scroll so row 20's bottom (588px) is at viewport bottom
        assert!((state.scroll_offset - 308.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ensure_selected_visible_scrolls_up() {
        let mut state = FileTreeViewState::new();
        state.scroll_offset = 500.0;
        state.nav.selected_visible_row = Some(5); // row 5 at 140px
        state.ensure_selected_visible(50, 280.0);
        assert!((state.scroll_offset - 140.0).abs() < f32::EPSILON);
    }

    // --- Breadcrumb ---

    #[test]
    fn breadcrumb_display_text() {
        let bc = BreadcrumbData::from_path(
            &PathBuf::from("/home/user/project/src"),
            &PathBuf::from("/home/user/project"),
        );
        assert_eq!(bc.display_text(), "project / src");
    }

    #[test]
    fn breadcrumb_segments_count() {
        let bc = BreadcrumbData::from_path(
            &PathBuf::from("/home/user/project/src/lib.rs"),
            &PathBuf::from("/home/user/project"),
        );
        assert_eq!(bc.segments.len(), 3); // project, src, lib.rs
    }

    // --- FileTreeViewState ---

    #[test]
    fn view_state_defaults() {
        let state = FileTreeViewState::new();
        assert!((state.scroll_offset - 0.0).abs() < f32::EPSILON);
        assert!(state.hovered_row.is_none());
        assert!(state.selected_file.is_none());
        assert!(state.nav.selected_visible_row.is_none());
    }

    // --- Compact visible rows ---

    fn make_dir_row(index: usize, depth: usize, name: &str, expanded: bool) -> VisibleRow {
        VisibleRow {
            index,
            depth,
            name: name.into(),
            node_type: NodeType::Directory,
            expanded,
            has_children: true,
            is_last_child: false,
            ancestor_has_next_sibling: vec![],
        }
    }

    fn make_file_row(index: usize, depth: usize, name: &str) -> VisibleRow {
        VisibleRow {
            index,
            depth,
            name: name.into(),
            node_type: NodeType::File { extension: None, size: 0 },
            expanded: false,
            has_children: false,
            is_last_child: false,
            ancestor_has_next_sibling: vec![],
        }
    }

    #[test]
    fn compact_empty_rows() {
        let result = compact_visible_rows(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn compact_single_child_dir_chain() {
        // root > a (expanded, 1 child) > b (expanded, 1 child) > file.txt
        let rows = vec![
            make_dir_row(0, 0, "root", true),
            make_dir_row(1, 1, "a", true),
            make_dir_row(2, 2, "b", true),
            make_file_row(3, 3, "file.txt"),
        ];
        let result = compact_visible_rows(rows);
        // root stays, a/b compacted into one row, file.txt remains
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "root");
        assert_eq!(result[1].name, "a/b");
        assert_eq!(result[1].depth, 1); // depth of first in chain
        assert_eq!(result[2].name, "file.txt");
    }

    #[test]
    fn compact_multi_child_dir_not_compacted() {
        // root > src (expanded, 2 children: a.rs, b.rs)
        let rows = vec![
            make_dir_row(0, 0, "root", true),
            make_dir_row(1, 1, "src", true),
            make_file_row(2, 2, "a.rs"),
            make_file_row(3, 2, "b.rs"),
        ];
        let result = compact_visible_rows(rows);
        assert_eq!(result.len(), 4); // no compaction
        assert_eq!(result[1].name, "src");
    }

    #[test]
    fn compact_collapsed_dir_not_compacted() {
        // root > a (collapsed) — not expanded, so no compaction
        let rows = vec![
            make_dir_row(0, 0, "root", true),
            make_dir_row(1, 1, "a", false),
        ];
        let result = compact_visible_rows(rows);
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].name, "a");
    }

    #[test]
    fn compact_deep_chain() {
        // root > a > b > c > file.txt (all single-child expanded dirs)
        let rows = vec![
            make_dir_row(0, 0, "root", true),
            make_dir_row(1, 1, "a", true),
            make_dir_row(2, 2, "b", true),
            make_dir_row(3, 3, "c", true),
            make_file_row(4, 4, "file.txt"),
        ];
        let result = compact_visible_rows(rows);
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].name, "a/b/c");
        assert_eq!(result[1].depth, 1);
        assert_eq!(result[2].name, "file.txt");
    }

    #[test]
    fn compact_preserves_files_between_dirs() {
        // root > file1.txt, dir_a (collapsed)
        let rows = vec![
            make_dir_row(0, 0, "root", true),
            make_file_row(1, 1, "file1.txt"),
            make_dir_row(2, 1, "dir_a", false),
        ];
        let result = compact_visible_rows(rows);
        assert_eq!(result.len(), 3); // no compaction possible
    }
}
