// File browser view — iced widget rendering for the file explorer panel.
//
// Provides functions to build the iced widget tree for the file tree,
// breadcrumb bar, and virtual scrolling.

use crate::file_browser::tree::{
    breadcrumb_segments, file_icon, visible_range, FileTree, NodeType, TreeNavState, VisibleRow,
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

/// Get a display icon for a visible row.
pub fn row_icon(row: &VisibleRow) -> &'static str {
    match &row.node_type {
        NodeType::Directory => {
            if row.expanded {
                "\u{1F4C2}" // open folder
            } else {
                "\u{1F4C1}" // closed folder
            }
        }
        NodeType::File { extension, .. } => file_icon(extension.as_deref()),
    }
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
        };
        assert_eq!(row_icon(&row), "\u{2699}");
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
}
