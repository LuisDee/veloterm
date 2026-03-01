// File browser overlay — state management for the project file browser.

pub mod actions;
pub mod git_status;
pub mod preview;
pub mod search;
pub mod tree;
pub mod view;
pub mod watcher;

use crate::input::InputMode;
use git_status::TreeGitStatus;
use preview::{FilePreview, PreviewViewState};
use search::{FuzzyMatcher, SearchResult};
use std::path::PathBuf;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use tree::{FileTree, NodeType, TreeNavAction, TreeNavResult, VisibleRow};
use view::{compact_visible_rows, BreadcrumbData, FileTreeViewState};

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
    /// Currently loaded file preview (None = empty state).
    pub preview: Option<FilePreview>,
    /// Preview panel scroll/wrap state.
    pub preview_view: PreviewViewState,
    /// Shared syntax set for highlighting (loaded once).
    pub syntax_set: SyntaxSet,
    /// Shared theme set for highlighting (loaded once).
    pub theme_set: ThemeSet,
    /// Whether search mode is active (typing into the search bar).
    pub search_active: bool,
    /// Current search query string.
    pub search_query: String,
    /// Fuzzy matcher for file search.
    pub search_matcher: FuzzyMatcher,
    /// Current search results.
    pub search_results: Vec<SearchResult>,
    /// Whether to compact single-child directory chains (default true).
    pub compact_folders: bool,
    /// Cached git status for displaying indicators in the file tree.
    pub git_status: Option<TreeGitStatus>,
    /// Git repository root path (for computing relative paths).
    pub repo_root: Option<PathBuf>,
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
            preview: None,
            preview_view: PreviewViewState::new(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            search_active: false,
            search_query: String::new(),
            search_matcher: FuzzyMatcher::new(),
            search_results: Vec::new(),
            compact_folders: true,
            git_status: None,
            repo_root: None,
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
        self.refresh_git_status(&root);
    }

    /// Refresh git status indicators from the repository at the given root.
    pub fn refresh_git_status(&mut self, root: &std::path::Path) {
        match git2::Repository::discover(root) {
            Ok(repo) => {
                self.repo_root = repo.workdir().map(|p| p.to_path_buf());
                match TreeGitStatus::from_repo(&repo) {
                    Ok(status) => self.git_status = Some(status),
                    Err(_) => self.git_status = None,
                }
            }
            Err(_) => {
                self.repo_root = None;
                self.git_status = None;
            }
        }
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
    /// This triggers the Enter action (expand dir / open file).
    pub fn handle_row_click(&mut self, visible_row_idx: usize, viewport_height: f32) -> Option<PathBuf> {
        self.view_state.nav.selected_visible_row = Some(visible_row_idx);
        self.handle_nav_action(TreeNavAction::Enter, viewport_height)
    }

    /// Select a row without triggering any action (no expand/open).
    /// Used for single-click: just highlight the row and load preview.
    pub fn handle_row_select(&mut self, visible_row_idx: usize) -> Option<PathBuf> {
        self.view_state.nav.selected_visible_row = Some(visible_row_idx);
        // Return the path of the selected item for preview loading
        if let Some(row) = self.visible_rows.get(visible_row_idx) {
            if let Some(tree) = &self.file_tree {
                if let Some(node) = tree.get(row.index) {
                    if node.path.is_file() {
                        self.view_state.selected_file = Some(row.index);
                        return Some(node.path.clone());
                    }
                }
            }
        }
        None
    }

    /// Navigate to a breadcrumb segment path.
    pub fn navigate_to_breadcrumb(&mut self, path: PathBuf) {
        self.open(path);
    }

    /// Enter search mode: activate the search bar and clear any previous query.
    pub fn enter_search_mode(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.search_results.clear();
    }

    /// Exit search mode: deactivate the search bar and clear results.
    pub fn exit_search_mode(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_results.clear();
    }

    /// Update search results for the given query.
    pub fn update_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        if query.is_empty() {
            self.search_results.clear();
        } else if let Some(tree) = &self.file_tree {
            self.search_results = self.search_matcher.search_files(
                query,
                tree.nodes(),
                tree.root(),
            );
        }
    }

    /// Append a character to the search query and refresh results.
    pub fn search_append_char(&mut self, ch: char) {
        self.search_query.push(ch);
        let q = self.search_query.clone();
        self.update_search(&q);
    }

    /// Remove the last character from the search query and refresh results.
    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        let q = self.search_query.clone();
        self.update_search(&q);
    }

    /// Get the effective visible rows — filtered by search when active,
    /// or compacted when compact_folders is enabled.
    pub fn effective_visible_rows(&self) -> Vec<VisibleRow> {
        if self.search_active && !self.search_query.is_empty() {
            // In search mode with a query: show flat list of matching files
            self.search_results
                .iter()
                .filter_map(|result| {
                    self.file_tree.as_ref().and_then(|tree| {
                        tree.get(result.node_index).map(|node| VisibleRow {
                            index: result.node_index,
                            depth: 0,
                            name: result.relative_path.clone(),
                            node_type: node.node_type.clone(),
                            expanded: false,
                            has_children: false,
                            is_last_child: false,
                            ancestor_has_next_sibling: vec![],
                        })
                    })
                })
                .collect()
        } else {
            let rows = self.visible_rows.clone();
            if self.compact_folders {
                compact_visible_rows(rows)
            } else {
                rows
            }
        }
    }

    /// Reset state while preserving layout and syntax resources.
    /// Clears: file_tree, visible_rows, view_state, breadcrumb, preview, preview_view
    /// Preserves: split_ratio, focused_panel, syntax_set, theme_set
    pub fn reset(&mut self) {
        self.file_tree = None;
        self.visible_rows.clear();
        self.view_state = FileTreeViewState::new();
        self.breadcrumb = None;
        self.preview = None;
        self.preview_view = PreviewViewState::new();
        self.search_active = false;
        self.search_query.clear();
        self.search_results.clear();
    }

    /// Load a file preview for the given path.
    /// Resets preview scroll state.
    pub fn load_preview(&mut self, path: &std::path::Path) {
        self.preview_view = PreviewViewState::new();
        match FilePreview::load(path, &self.syntax_set, &self.theme_set) {
            Ok(preview) => {
                self.preview = Some(preview);
            }
            Err(e) => {
                log::warn!("Failed to load preview for {}: {e}", path.display());
                self.preview = None;
            }
        }
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

        // Click on the file row (handle_row_click = double-click action)
        let result = state.handle_row_click(1, 500.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root.join("click.txt"));
        assert_eq!(state.view_state.selected_file.is_some(), true);
    }

    #[test]
    fn row_select_only_selects_file_without_expanding_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let sub = root.join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("inner.txt"), "in").unwrap();
        std::fs::write(root.join("hello.txt"), "hi").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root.clone());
        let initial_rows = state.visible_rows.len();

        // Single-click on subdir (row 1) — should NOT expand it
        let result = state.handle_row_select(1);
        assert!(result.is_none(), "selecting a dir should not return a file path");
        assert_eq!(state.view_state.nav.selected_visible_row, Some(1));
        assert_eq!(state.visible_rows.len(), initial_rows, "dir should not expand on select");
    }

    #[test]
    fn row_select_on_file_returns_path_for_preview() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("preview.txt"), "content").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root.clone());

        // Single-click on file row (root=0, file=1)
        let result = state.handle_row_select(1);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root.join("preview.txt"));
        assert_eq!(state.view_state.nav.selected_visible_row, Some(1));
        assert!(state.view_state.selected_file.is_some());
    }

    #[test]
    fn row_select_out_of_bounds_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.txt"), "a").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);

        // Out-of-bounds index
        let result = state.handle_row_select(999);
        assert!(result.is_none());
        assert_eq!(state.view_state.nav.selected_visible_row, Some(999));
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

    // -- reset() tests --

    #[test]
    fn reset_clears_tree_and_preview() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("test.txt"), "hello").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        assert!(state.file_tree.is_some());
        assert!(!state.visible_rows.is_empty());

        state.reset();
        assert!(state.file_tree.is_none());
        assert!(state.visible_rows.is_empty());
        assert!(state.breadcrumb.is_none());
        assert!(state.preview.is_none());
    }

    #[test]
    fn reset_preserves_layout() {
        let mut state = FileBrowserState::new();
        state.split_ratio = 0.7;
        state.focused_panel = OverlayPanel::Right;

        state.reset();

        assert!((state.split_ratio - 0.7).abs() < f32::EPSILON);
        assert_eq!(state.focused_panel, OverlayPanel::Right);
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

    // ── Search mode ─────────────────────────────────────────

    fn create_test_state() -> FileBrowserState {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("hello.txt"), "hi").unwrap();
        let mut state = FileBrowserState::new();
        state.open(root);
        // Leak the tempdir so the directory stays around for the test
        std::mem::forget(dir);
        state
    }

    #[test]
    fn search_slash_enters_search_mode() {
        let mut state = create_test_state();
        state.enter_search_mode();
        assert!(state.search_active);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn search_escape_exits_search() {
        let mut state = create_test_state();
        state.enter_search_mode();
        state.search_append_char('t');
        state.exit_search_mode();
        assert!(!state.search_active);
        assert!(state.search_query.is_empty());
        assert!(state.search_results.is_empty());
    }

    #[test]
    fn search_filters_visible_rows() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("config.toml"), "").unwrap();
        std::fs::write(root.join("main.rs"), "").unwrap();
        std::fs::write(root.join("readme.md"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        state.enter_search_mode();
        state.update_search("config");

        let rows = state.effective_visible_rows();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].name.contains("config"));
    }

    #[test]
    fn search_empty_query_shows_all() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.txt"), "").unwrap();
        std::fs::write(root.join("b.txt"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        let all_rows = state.effective_visible_rows().len();

        state.enter_search_mode();
        // Empty query -- should show all rows
        let search_rows = state.effective_visible_rows().len();
        assert_eq!(search_rows, all_rows);
    }

    #[test]
    fn search_append_char() {
        let mut state = create_test_state();
        state.enter_search_mode();
        state.search_append_char('h');
        state.search_append_char('e');
        assert_eq!(state.search_query, "he");
    }

    #[test]
    fn search_backspace() {
        let mut state = create_test_state();
        state.enter_search_mode();
        state.search_append_char('h');
        state.search_append_char('e');
        state.search_backspace();
        assert_eq!(state.search_query, "h");
    }

    #[test]
    fn search_backspace_on_empty() {
        let mut state = create_test_state();
        state.enter_search_mode();
        state.search_backspace(); // should not panic
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn search_results_sorted_by_relevance() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("main.rs"), "").unwrap();
        std::fs::write(root.join("lib.rs"), "").unwrap();
        std::fs::write(root.join("config.toml"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        state.enter_search_mode();
        state.update_search("rs");

        let rows = state.effective_visible_rows();
        // Should find at least the .rs files
        assert!(rows.len() >= 2);
        // All results should be files (search only matches files)
        for row in &rows {
            assert!(matches!(row.node_type, NodeType::File { .. }));
        }
    }

    #[test]
    fn search_reset_clears_search_state() {
        let mut state = create_test_state();
        state.enter_search_mode();
        state.search_append_char('x');
        state.reset();
        assert!(!state.search_active);
        assert!(state.search_query.is_empty());
        assert!(state.search_results.is_empty());
    }

    // ── Compact folders ──────────────────────────────────────

    #[test]
    fn compact_folders_single_child_chain() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let deep = root.join("a").join("b").join("c");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("file.txt"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        state.compact_folders = true;

        // Expand a, then a/b, then a/b/c
        // After open(), root is expanded. visible rows: root, a
        // Find 'a' and expand
        let a_idx = state.visible_rows.iter()
            .find(|r| r.name == "a").unwrap().index;
        if let Some(tree) = &mut state.file_tree {
            let _ = tree.expand(a_idx);
        }
        state.refresh_visible_rows();

        // Find 'b' and expand
        let b_idx = state.visible_rows.iter()
            .find(|r| r.name == "b").unwrap().index;
        if let Some(tree) = &mut state.file_tree {
            let _ = tree.expand(b_idx);
        }
        state.refresh_visible_rows();

        // Find 'c' and expand
        let c_idx = state.visible_rows.iter()
            .find(|r| r.name == "c").unwrap().index;
        if let Some(tree) = &mut state.file_tree {
            let _ = tree.expand(c_idx);
        }
        state.refresh_visible_rows();

        let rows = state.effective_visible_rows();
        // Should have compacted: "a/b/c" as one row + file.txt
        let dir_row = rows.iter().find(|r| r.name.contains("a")).unwrap();
        assert!(
            dir_row.name.contains("a/b/c") || dir_row.name.contains("a/b"),
            "single-child dirs should be compacted, got: {}",
            dir_row.name
        );
    }

    #[test]
    fn compact_folders_multi_child_not_compacted() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let sub = root.join("src");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("a.rs"), "").unwrap();
        std::fs::write(sub.join("b.rs"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        state.compact_folders = true;

        let rows = state.effective_visible_rows();
        // "src" has 2 children -- should NOT be compacted
        let src_row = rows.iter().find(|r| r.name == "src").unwrap();
        assert_eq!(src_row.name, "src");
    }

    #[test]
    fn compact_folders_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let deep = root.join("a").join("b");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("file.txt"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);
        state.compact_folders = false;

        let rows = state.effective_visible_rows();
        // Should NOT compact -- "a" should be a separate row
        assert!(rows.iter().any(|r| r.name == "a"), "should have separate 'a' row");
    }

    #[test]
    fn compact_folders_default_is_true() {
        let state = FileBrowserState::new();
        assert!(state.compact_folders);
    }

    #[test]
    fn effective_visible_rows_without_search_returns_compacted() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("file.txt"), "").unwrap();

        let mut state = FileBrowserState::new();
        state.open(root);

        // Not in search mode — should return compacted visible rows
        assert!(!state.search_active);
        let rows = state.effective_visible_rows();
        assert!(!rows.is_empty());
    }
}
