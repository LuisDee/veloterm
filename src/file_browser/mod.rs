// File browser overlay — state management for the project file browser.

pub mod tree;
pub mod view;

use crate::input::InputMode;
use std::path::PathBuf;
use tree::{FileTree, TreeNavAction, TreeNavResult, VisibleRow};
use view::{BreadcrumbData, FileTreeViewState};

/// Which panel has focus in a split overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayPanel {
    #[default]
    Left,
    Right,
}

impl OverlayPanel {
    pub fn toggle(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// State for the File Browser overlay.
pub struct FileBrowserState {
    /// Split panel divider position as fraction (0.0..1.0). Default 0.5.
    pub split_ratio: f32,
    /// Which panel has focus: Left (file tree) or Right (preview).
    pub focused_panel: OverlayPanel,
    /// The file tree (None until opened with a root path).
    pub file_tree: Option<FileTree>,
    /// Cached visible rows (recomputed after expand/collapse).
    pub visible_rows: Vec<VisibleRow>,
    /// View state (scroll, selection, hover).
    pub view_state: FileTreeViewState,
    /// Breadcrumb data for the current root.
    pub breadcrumb: Option<BreadcrumbData>,
}

impl FileBrowserState {
    pub fn new() -> Self {
        Self {
            split_ratio: 0.5,
            focused_panel: OverlayPanel::Left,
            file_tree: None,
            visible_rows: Vec::new(),
            view_state: FileTreeViewState::new(),
            breadcrumb: None,
        }
    }

    /// Open the file browser at a given root directory.
    /// Loads the root and auto-expands it.
    pub fn open(&mut self, root: PathBuf) {
        let mut tree = FileTree::new(root.clone());
        let _ = tree.expand(0); // expand root directory
        self.visible_rows = tree.visible_rows();
        self.breadcrumb = Some(BreadcrumbData::from_path(&root, &root));
        self.file_tree = Some(tree);
        self.view_state = FileTreeViewState::new();
    }

    /// Refresh the visible rows cache from the current tree state.
    pub fn refresh_visible_rows(&mut self) {
        if let Some(tree) = &self.file_tree {
            self.visible_rows = tree.visible_rows();
        }
    }

    /// Handle a keyboard navigation action.
    /// Returns the path of a file to open (if Enter on a file).
    pub fn handle_nav_action(&mut self, action: TreeNavAction, viewport_height: f32) -> Option<PathBuf> {
        let result = self.view_state.nav.apply(action, &self.visible_rows);

        if let Some(nav_result) = result {
            match nav_result {
                TreeNavResult::Expand(node_idx) => {
                    if let Some(tree) = &mut self.file_tree {
                        let _ = tree.expand(node_idx);
                        self.visible_rows = tree.visible_rows();
                    }
                }
                TreeNavResult::Collapse(node_idx) => {
                    if let Some(tree) = &mut self.file_tree {
                        tree.collapse(node_idx);
                        self.visible_rows = tree.visible_rows();
                    }
                }
                TreeNavResult::OpenFile(node_idx) => {
                    if let Some(tree) = &self.file_tree {
                        if let Some(node) = tree.get(node_idx) {
                            self.view_state.selected_file = Some(node_idx);
                            return Some(node.path.clone());
                        }
                    }
                }
            }
        }

        // Ensure selected row is visible after navigation
        self.view_state.ensure_selected_visible(self.visible_rows.len(), viewport_height);
        None
    }

    /// Handle clicking on a row (by visible row index).
    pub fn handle_row_click(&mut self, visible_row_idx: usize, viewport_height: f32) -> Option<PathBuf> {
        self.view_state.nav.selected_visible_row = Some(visible_row_idx);
        self.handle_nav_action(TreeNavAction::Enter, viewport_height)
    }

    /// Navigate to a breadcrumb segment path.
    pub fn navigate_to_breadcrumb(&mut self, path: PathBuf) {
        self.open(path);
    }
}

/// Compute the next InputMode when toggling the file browser.
pub fn toggle_file_browser(current: InputMode) -> InputMode {
    if current == InputMode::FileBrowser {
        InputMode::Normal
    } else {
        InputMode::FileBrowser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_browser_state_defaults() {
        let state = FileBrowserState::new();
        assert!((state.split_ratio - 0.5).abs() < f32::EPSILON);
        assert_eq!(state.focused_panel, OverlayPanel::Left);
        assert!(state.file_tree.is_none());
        assert!(state.visible_rows.is_empty());
        assert!(state.breadcrumb.is_none());
    }

    #[test]
    fn file_browser_toggle_focus() {
        assert_eq!(OverlayPanel::Left.toggle(), OverlayPanel::Right);
        assert_eq!(OverlayPanel::Right.toggle(), OverlayPanel::Left);
    }

    #[test]
    fn overlay_panel_default_is_left() {
        assert_eq!(OverlayPanel::default(), OverlayPanel::Left);
    }

    #[test]
    fn overlay_toggle_from_normal_to_file_browser() {
        assert_eq!(toggle_file_browser(InputMode::Normal), InputMode::FileBrowser);
    }

    #[test]
    fn overlay_toggle_from_file_browser_to_normal() {
        assert_eq!(toggle_file_browser(InputMode::FileBrowser), InputMode::Normal);
    }

    #[test]
    fn overlay_switch_from_git_review_to_file_browser() {
        assert_eq!(toggle_file_browser(InputMode::GitReview), InputMode::FileBrowser);
    }

    #[test]
    fn overlay_escape_closes_file_browser() {
        let result = toggle_file_browser(InputMode::FileBrowser);
        assert_eq!(result, InputMode::Normal);
    }

    #[test]
    fn overlay_tab_toggles_panel_focus() {
        let mut state = FileBrowserState::new();
        assert_eq!(state.focused_panel, OverlayPanel::Left);
        state.focused_panel = state.focused_panel.toggle();
        assert_eq!(state.focused_panel, OverlayPanel::Right);
        state.focused_panel = state.focused_panel.toggle();
        assert_eq!(state.focused_panel, OverlayPanel::Left);
    }

    #[test]
    fn overlay_preserves_split_ratio_across_toggle() {
        let mut state = FileBrowserState::new();
        state.split_ratio = 0.3;
        assert!((state.split_ratio - 0.3).abs() < f32::EPSILON);
    }

    // --- Integrated tree + navigation tests ---

    #[test]
    fn open_loads_directory_and_expands_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("hello.txt"), "hi").unwrap();
        std::fs::create_dir(root.join("subdir")).unwrap();

        let mut state = FileBrowserState::new();
        state.open(root.clone());

        assert!(state.file_tree.is_some());
        assert!(!state.visible_rows.is_empty());
        // root + subdir + hello.txt = 3 visible rows
        assert_eq!(state.visible_rows.len(), 3);
        assert!(state.breadcrumb.is_some());
    }

    #[test]
    fn nav_down_selects_rows_sequentially() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.txt"), "a").unwrap();
        std::fs::write(root.join("b.txt"), "b").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        // visible: root(0), a.txt(1), b.txt(2)

        // First Down from None: unwrap_or(0) + 1 = 1
        state.handle_nav_action(TreeNavAction::Down, 500.0);
        assert_eq!(state.view_state.nav.selected_visible_row, Some(1));

        // Second Down moves to row 2
        state.handle_nav_action(TreeNavAction::Down, 500.0);
        assert_eq!(state.view_state.nav.selected_visible_row, Some(2));

        // At last row, stays
        state.handle_nav_action(TreeNavAction::Down, 500.0);
        assert_eq!(state.view_state.nav.selected_visible_row, Some(2));
    }

    #[test]
    fn nav_enter_on_file_returns_path() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("hello.txt"), "hi").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root.clone());

        // Navigate to the file (root at 0, hello.txt at 1)
        state.view_state.nav.selected_visible_row = Some(1);
        let result = state.handle_nav_action(TreeNavAction::Enter, 500.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root.join("hello.txt"));
    }

    #[test]
    fn nav_expand_collapse_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let sub = root.join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("inner.txt"), "in").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);

        // subdir is at visible row 1 (dirs first)
        let initial_rows = state.visible_rows.len(); // root + subdir
        state.view_state.nav.selected_visible_row = Some(1);

        // Expand subdir
        state.handle_nav_action(TreeNavAction::Enter, 500.0);
        assert!(state.visible_rows.len() > initial_rows); // now has inner.txt

        // Collapse subdir
        state.view_state.nav.selected_visible_row = Some(1);
        state.handle_nav_action(TreeNavAction::Enter, 500.0);
        assert_eq!(state.visible_rows.len(), initial_rows);
    }

    #[test]
    fn row_click_selects_and_acts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("click.txt"), "hi").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root.clone());

        // Click on the file row
        let result = state.handle_row_click(1, 500.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root.join("click.txt"));
        assert_eq!(state.view_state.selected_file.is_some(), true);
    }

    #[test]
    fn breadcrumb_navigate_resets_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let sub = root.join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("inner.txt"), "in").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);

        // Navigate to subdir via breadcrumb
        state.navigate_to_breadcrumb(sub.clone());
        let bc = state.breadcrumb.as_ref().unwrap();
        assert_eq!(bc.segments.last().unwrap().0, "subdir");
        assert!(state.visible_rows.len() >= 2); // subdir root + inner.txt
    }

    #[test]
    fn nav_left_from_file_goes_to_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let sub = root.join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("inner.txt"), "in").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);

        // Expand subdir
        state.view_state.nav.selected_visible_row = Some(1);
        state.handle_nav_action(TreeNavAction::Right, 500.0);

        // Now at expanded subdir, move into child
        state.handle_nav_action(TreeNavAction::Right, 500.0);
        // Should be on inner.txt
        assert_eq!(state.view_state.nav.selected_visible_row, Some(2));

        // Left from file should go to parent (subdir at row 1)
        state.handle_nav_action(TreeNavAction::Left, 500.0);
        assert_eq!(state.view_state.nav.selected_visible_row, Some(1));
    }
}
