// FileTree — hierarchical file/directory tree with lazy loading for the file browser overlay.

use std::path::{Path, PathBuf};

/// Maximum number of entries to load from a single directory.
pub const MAX_DIR_ENTRIES: usize = 10_000;

/// Type of a file tree node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    File {
        extension: Option<String>,
        size: u64,
    },
    Directory,
    Symlink {
        target: PathBuf,
    },
}

/// A single node in the file tree.
#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub node_type: NodeType,
    pub depth: usize,
    pub parent: Option<usize>,
    /// None = not yet loaded (lazy), Some = loaded (may be empty).
    pub children: Option<Vec<usize>>,
    pub expanded: bool,
}

/// A visible row in the flattened tree, used for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRow {
    pub index: usize,
    pub depth: usize,
    pub name: String,
    pub node_type: NodeType,
    pub expanded: bool,
    pub has_children: bool,
}

/// Flat-storage tree of files and directories with lazy loading.
pub struct FileTree {
    root: PathBuf,
    nodes: Vec<FileNode>,
    pub show_hidden: bool,
}

impl FileTree {
    /// Create a new tree rooted at the given directory.
    /// Populates the root node but does NOT load its children.
    pub fn new(root: PathBuf) -> Self {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string());
        let root_node = FileNode {
            name,
            path: root.clone(),
            node_type: NodeType::Directory,
            depth: 0,
            parent: None,
            children: None,
            expanded: false,
        };
        Self {
            root,
            nodes: vec![root_node],
            show_hidden: false,
        }
    }

    /// Set whether hidden files are shown.
    /// Clears loaded children so they are re-read on next expand.
    pub fn set_show_hidden(&mut self, show: bool) {
        if self.show_hidden != show {
            self.show_hidden = show;
            // Clear all loaded children so expand re-reads
            for node in &mut self.nodes {
                if matches!(node.node_type, NodeType::Directory) {
                    node.children = None;
                    node.expanded = false;
                }
            }
        }
    }

    /// Get the root path of the tree.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree is empty (should never be — always has root).
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get a node by index.
    pub fn get(&self, index: usize) -> Option<&FileNode> {
        self.nodes.get(index)
    }

    /// Get a mutable node by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut FileNode> {
        self.nodes.get_mut(index)
    }

    /// Expand a directory node: read its children from the filesystem.
    /// If already loaded, just set expanded = true.
    /// Hidden files (starting with '.') are excluded.
    pub fn expand(&mut self, index: usize) -> std::io::Result<()> {
        if index >= self.nodes.len() {
            return Ok(());
        }
        if self.nodes[index].node_type != NodeType::Directory {
            return Ok(());
        }

        // If children not yet loaded, read from filesystem
        if self.nodes[index].children.is_none() {
            let path = self.nodes[index].path.clone();
            let depth = self.nodes[index].depth + 1;
            let mut child_indices = Vec::new();
            let show_hidden = self.show_hidden;

            let entries = std::fs::read_dir(&path)?;
            let mut count = 0usize;
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name = entry
                    .file_name()
                    .to_string_lossy()
                    .to_string();

                // Skip hidden files unless show_hidden is enabled
                if !show_hidden && name.starts_with('.') {
                    continue;
                }

                // Large directory guard
                if count >= MAX_DIR_ENTRIES {
                    log::warn!("Directory {} has more than {} entries, truncating", path.display(), MAX_DIR_ENTRIES);
                    break;
                }

                let metadata = entry.metadata();
                let symlink_meta = std::fs::symlink_metadata(&entry_path);

                let node_type = if symlink_meta.as_ref().map(|m| m.is_symlink()).unwrap_or(false) {
                    let target = std::fs::read_link(&entry_path).unwrap_or_default();
                    NodeType::Symlink { target }
                } else if metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                    NodeType::Directory
                } else {
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    NodeType::File {
                        extension: entry_path
                            .extension()
                            .map(|e| e.to_string_lossy().to_string()),
                        size,
                    }
                };

                let child_index = self.nodes.len();
                child_indices.push(child_index);
                self.nodes.push(FileNode {
                    name,
                    path: entry_path,
                    node_type,
                    depth,
                    parent: Some(index),
                    children: None,
                    expanded: false,
                });
                count += 1;
            }

            self.nodes[index].children = Some(child_indices);
            self.sort_children(index);
        }

        self.nodes[index].expanded = true;
        Ok(())
    }

    /// Collapse a directory node (keep children in memory).
    pub fn collapse(&mut self, index: usize) {
        if index < self.nodes.len() {
            self.nodes[index].expanded = false;
        }
    }

    /// Toggle expand/collapse for a directory.
    pub fn toggle(&mut self, index: usize) -> std::io::Result<()> {
        if index >= self.nodes.len() {
            return Ok(());
        }
        if self.nodes[index].expanded {
            self.collapse(index);
            Ok(())
        } else {
            self.expand(index)
        }
    }

    /// Sort children of a node: directories first, then files, case-insensitive alphabetical.
    pub fn sort_children(&mut self, parent: usize) {
        if let Some(children) = self.nodes[parent].children.clone() {
            let mut sorted = children;
            sorted.sort_by(|&a, &b| {
                let node_a = &self.nodes[a];
                let node_b = &self.nodes[b];

                // Directories first
                let a_is_dir = matches!(node_a.node_type, NodeType::Directory);
                let b_is_dir = matches!(node_b.node_type, NodeType::Directory);
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => node_a.name.to_lowercase().cmp(&node_b.name.to_lowercase()),
                }
            });
            self.nodes[parent].children = Some(sorted);
        }
    }

    /// Flatten the expanded tree into visible rows for rendering.
    /// Only includes nodes whose ancestors are all expanded.
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = Vec::new();
        if self.nodes.is_empty() {
            return rows;
        }
        // Start from root's children if root is expanded, or show root itself
        self.collect_visible(0, &mut rows);
        rows
    }

    fn collect_visible(&self, index: usize, rows: &mut Vec<VisibleRow>) {
        let node = &self.nodes[index];
        let has_children = match &node.children {
            Some(c) => !c.is_empty(),
            None => matches!(node.node_type, NodeType::Directory), // unloaded dirs assumed to have children
        };

        rows.push(VisibleRow {
            index,
            depth: node.depth,
            name: node.name.clone(),
            node_type: node.node_type.clone(),
            expanded: node.expanded,
            has_children,
        });

        if node.expanded {
            if let Some(children) = &node.children {
                for &child_idx in children {
                    self.collect_visible(child_idx, rows);
                }
            }
        }
    }

    /// Add a child node to a parent (for testing without filesystem).
    #[cfg(test)]
    pub fn add_child(&mut self, parent: usize, name: &str, node_type: NodeType) -> usize {
        let depth = self.nodes[parent].depth + 1;
        let path = self.nodes[parent].path.join(name);
        let child_index = self.nodes.len();
        self.nodes.push(FileNode {
            name: name.to_string(),
            path,
            node_type,
            depth,
            parent: Some(parent),
            children: None,
            expanded: false,
        });
        if let Some(children) = &mut self.nodes[parent].children {
            children.push(child_index);
        } else {
            self.nodes[parent].children = Some(vec![child_index]);
        }
        child_index
    }
}

/// Compute visible row range for virtual scrolling.
/// Returns (start_row, end_row) — exclusive end.
pub fn visible_range(scroll_offset: f32, viewport_height: f32, row_height: f32, total_rows: usize) -> (usize, usize) {
    if total_rows == 0 || row_height <= 0.0 || viewport_height <= 0.0 {
        return (0, 0);
    }
    let start = (scroll_offset / row_height).floor() as usize;
    let visible_count = (viewport_height / row_height).ceil() as usize + 1; // +1 for partial row
    let end = (start + visible_count).min(total_rows);
    let start = start.min(total_rows);
    (start, end)
}

/// Map a file extension to a display icon character.
pub fn file_icon(extension: Option<&str>) -> &'static str {
    match extension {
        Some("rs") => "\u{2699}",     // gear for Rust
        Some("py") => "\u{1F40D}",    // snake for Python — but we use text for safety
        Some("js") | Some("jsx") => "JS",
        Some("ts") | Some("tsx") => "TS",
        Some("json") => "{}",
        Some("toml") => "\u{2699}",   // gear
        Some("yaml") | Some("yml") => "Y",
        Some("md") => "\u{2193}",     // markdown arrow
        Some("txt") => "\u{2261}",    // text lines
        Some("sh") | Some("bash") | Some("zsh") => "$",
        Some("css") | Some("scss") => "#",
        Some("html") | Some("htm") => "<>",
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") => "\u{25A3}",
        Some("lock") => "\u{1F512}",
        Some("wgsl") | Some("glsl") | Some("hlsl") => "\u{25B3}", // shader triangle
        _ => "\u{25A1}",             // generic file
    }
}

/// Map a file extension to a short label for icons (ASCII-safe fallback).
pub fn file_icon_label(extension: Option<&str>) -> &'static str {
    match extension {
        Some("rs") => "rs",
        Some("py") => "py",
        Some("js") | Some("jsx") => "js",
        Some("ts") | Some("tsx") => "ts",
        Some("json") => "{}",
        Some("toml") => "tm",
        Some("yaml") | Some("yml") => "ym",
        Some("md") => "md",
        Some("sh") | Some("bash") | Some("zsh") => "sh",
        Some("css") | Some("scss") => "cs",
        Some("html") | Some("htm") => "ht",
        _ => "--",
    }
}

/// Parse a path into breadcrumb segments.
/// Returns (display_segments, corresponding_paths).
pub fn breadcrumb_segments(path: &Path, root: &Path) -> Vec<(String, PathBuf)> {
    let mut segments = Vec::new();

    // Build segments from root to path
    if let Ok(relative) = path.strip_prefix(root) {
        // Start with root
        let root_name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string());
        segments.push((root_name, root.to_path_buf()));

        // Add each component
        let mut current = root.to_path_buf();
        for component in relative.components() {
            current = current.join(component);
            segments.push((component.as_os_str().to_string_lossy().to_string(), current.clone()));
        }
    } else {
        // path is not under root — just show the path itself
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        segments.push((name, path.to_path_buf()));
    }

    segments
}

/// Keyboard navigation actions for the file tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNavAction {
    /// Move selection up one row.
    Up,
    /// Move selection down one row.
    Down,
    /// Collapse directory or move to parent.
    Left,
    /// Expand directory or move into first child.
    Right,
    /// Open file / toggle directory.
    Enter,
    /// Jump to first row.
    Home,
    /// Jump to last row.
    End,
}

/// Keyboard navigation state for the tree.
#[derive(Debug, Clone)]
pub struct TreeNavState {
    /// Index of the selected row in the visible_rows list.
    pub selected_visible_row: Option<usize>,
}

impl TreeNavState {
    pub fn new() -> Self {
        Self {
            selected_visible_row: None,
        }
    }

    /// Apply a navigation action, returning the node index to act on (if any).
    /// The caller must provide the current visible rows and tree for context.
    pub fn apply(
        &mut self,
        action: TreeNavAction,
        visible_rows: &[VisibleRow],
    ) -> Option<TreeNavResult> {
        if visible_rows.is_empty() {
            return None;
        }

        match action {
            TreeNavAction::Up => {
                let current = self.selected_visible_row.unwrap_or(0);
                if current > 0 {
                    self.selected_visible_row = Some(current - 1);
                }
                None
            }
            TreeNavAction::Down => {
                let current = self.selected_visible_row.unwrap_or(0);
                if current + 1 < visible_rows.len() {
                    self.selected_visible_row = Some(current + 1);
                } else if self.selected_visible_row.is_none() {
                    self.selected_visible_row = Some(0);
                }
                None
            }
            TreeNavAction::Left => {
                if let Some(row_idx) = self.selected_visible_row {
                    if row_idx < visible_rows.len() {
                        let row = &visible_rows[row_idx];
                        if row.node_type == NodeType::Directory && row.expanded {
                            return Some(TreeNavResult::Collapse(row.index));
                        }
                        // Move to parent: find the row with the parent's depth
                        if row.depth > 0 {
                            for (i, r) in visible_rows[..row_idx].iter().enumerate().rev() {
                                if r.depth == row.depth - 1 {
                                    self.selected_visible_row = Some(i);
                                    break;
                                }
                            }
                        }
                    }
                }
                None
            }
            TreeNavAction::Right => {
                if let Some(row_idx) = self.selected_visible_row {
                    if row_idx < visible_rows.len() {
                        let row = &visible_rows[row_idx];
                        if row.node_type == NodeType::Directory {
                            if !row.expanded {
                                return Some(TreeNavResult::Expand(row.index));
                            }
                            // Already expanded — move into first child
                            if row_idx + 1 < visible_rows.len() {
                                let next = &visible_rows[row_idx + 1];
                                if next.depth > row.depth {
                                    self.selected_visible_row = Some(row_idx + 1);
                                }
                            }
                        }
                    }
                }
                None
            }
            TreeNavAction::Enter => {
                if let Some(row_idx) = self.selected_visible_row {
                    if row_idx < visible_rows.len() {
                        let row = &visible_rows[row_idx];
                        match &row.node_type {
                            NodeType::Directory => {
                                if row.expanded {
                                    Some(TreeNavResult::Collapse(row.index))
                                } else {
                                    Some(TreeNavResult::Expand(row.index))
                                }
                            }
                            NodeType::File { .. } | NodeType::Symlink { .. } => {
                                Some(TreeNavResult::OpenFile(row.index))
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            TreeNavAction::Home => {
                self.selected_visible_row = Some(0);
                None
            }
            TreeNavAction::End => {
                if !visible_rows.is_empty() {
                    self.selected_visible_row = Some(visible_rows.len() - 1);
                }
                None
            }
        }
    }
}

/// Result of a tree navigation action that requires the caller to mutate the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNavResult {
    Expand(usize),
    Collapse(usize),
    OpenFile(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- FileTree data model tests ---

    #[test]
    fn tree_new_has_root_node() {
        let tree = FileTree::new(PathBuf::from("/tmp/project"));
        assert_eq!(tree.len(), 1);
        assert!(!tree.is_empty());
        let root = tree.get(0).unwrap();
        assert_eq!(root.name, "project");
        assert_eq!(root.node_type, NodeType::Directory);
        assert_eq!(root.depth, 0);
        assert!(root.parent.is_none());
        assert!(root.children.is_none()); // not loaded yet
        assert!(!root.expanded);
    }

    #[test]
    fn tree_root_path() {
        let tree = FileTree::new(PathBuf::from("/tmp/project"));
        assert_eq!(tree.root(), Path::new("/tmp/project"));
    }

    #[test]
    fn tree_add_children_and_sort() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        // Add children: files and dirs in random order
        tree.add_child(0, "zebra.rs", NodeType::File { extension: Some("rs".into()), size: 100 });
        tree.add_child(0, "src", NodeType::Directory);
        tree.add_child(0, "apple.txt", NodeType::File { extension: Some("txt".into()), size: 50 });
        tree.add_child(0, "docs", NodeType::Directory);

        assert_eq!(tree.len(), 5); // root + 4 children

        // Sort: dirs first (docs, src), then files (apple.txt, zebra.rs)
        tree.sort_children(0);
        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(tree.get(children[0]).unwrap().name, "docs");
        assert_eq!(tree.get(children[1]).unwrap().name, "src");
        assert_eq!(tree.get(children[2]).unwrap().name, "apple.txt");
        assert_eq!(tree.get(children[3]).unwrap().name, "zebra.rs");
    }

    #[test]
    fn tree_sort_case_insensitive() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        tree.add_child(0, "Zebra", NodeType::Directory);
        tree.add_child(0, "alpha", NodeType::Directory);
        tree.add_child(0, "Beta", NodeType::Directory);

        tree.sort_children(0);
        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(tree.get(children[0]).unwrap().name, "alpha");
        assert_eq!(tree.get(children[1]).unwrap().name, "Beta");
        assert_eq!(tree.get(children[2]).unwrap().name, "Zebra");
    }

    #[test]
    fn tree_depth_calculation() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        let src = tree.add_child(0, "src", NodeType::Directory);
        let main = tree.add_child(src, "main.rs", NodeType::File { extension: Some("rs".into()), size: 200 });

        assert_eq!(tree.get(0).unwrap().depth, 0);
        assert_eq!(tree.get(src).unwrap().depth, 1);
        assert_eq!(tree.get(main).unwrap().depth, 2);
    }

    #[test]
    fn tree_parent_tracking() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        let src = tree.add_child(0, "src", NodeType::Directory);
        let main = tree.add_child(src, "main.rs", NodeType::File { extension: Some("rs".into()), size: 200 });

        assert!(tree.get(0).unwrap().parent.is_none());
        assert_eq!(tree.get(src).unwrap().parent, Some(0));
        assert_eq!(tree.get(main).unwrap().parent, Some(src));
    }

    // --- Visible rows / flattening tests ---

    #[test]
    fn tree_visible_rows_unexpanded_root() {
        let tree = FileTree::new(PathBuf::from("/tmp/project"));
        let rows = tree.visible_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "project");
        assert!(!rows[0].expanded);
        assert!(rows[0].has_children); // directory assumed to have children
    }

    #[test]
    fn tree_visible_rows_expanded() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        tree.add_child(0, "src", NodeType::Directory);
        tree.add_child(0, "README.md", NodeType::File { extension: Some("md".into()), size: 50 });
        tree.nodes[0].expanded = true;
        tree.sort_children(0);

        let rows = tree.visible_rows();
        assert_eq!(rows.len(), 3); // root + src + README.md
        assert_eq!(rows[0].name, "project");
        assert_eq!(rows[1].name, "src"); // dir first
        assert_eq!(rows[2].name, "README.md");
    }

    #[test]
    fn tree_visible_rows_collapsed_hides_children() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        let src = tree.add_child(0, "src", NodeType::Directory);
        tree.add_child(src, "main.rs", NodeType::File { extension: Some("rs".into()), size: 100 });
        tree.nodes[0].expanded = true;
        tree.nodes[src].expanded = false; // src is collapsed

        let rows = tree.visible_rows();
        assert_eq!(rows.len(), 2); // root + src (main.rs hidden)
    }

    #[test]
    fn tree_visible_rows_nested_expanded() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        let src = tree.add_child(0, "src", NodeType::Directory);
        tree.add_child(src, "main.rs", NodeType::File { extension: Some("rs".into()), size: 100 });
        tree.add_child(src, "lib.rs", NodeType::File { extension: Some("rs".into()), size: 200 });
        tree.nodes[0].expanded = true;
        tree.nodes[src].expanded = true;
        tree.sort_children(src);

        let rows = tree.visible_rows();
        assert_eq!(rows.len(), 4); // root + src + lib.rs + main.rs
        assert_eq!(rows[0].name, "project");
        assert_eq!(rows[1].name, "src");
        assert_eq!(rows[2].name, "lib.rs");
        assert_eq!(rows[3].name, "main.rs");
    }

    // --- Expand/collapse with real filesystem ---

    #[test]
    fn tree_expand_real_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir(root.join("subdir")).unwrap();
        std::fs::write(root.join("file.txt"), "hello").unwrap();

        let mut tree = FileTree::new(root);
        tree.expand(0).unwrap();

        assert!(tree.get(0).unwrap().expanded);
        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(children.len(), 2); // subdir + file.txt

        // Check sorting: dir first
        let first = tree.get(children[0]).unwrap();
        assert_eq!(first.node_type, NodeType::Directory);
        assert_eq!(first.name, "subdir");
    }

    #[test]
    fn tree_expand_hides_hidden_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join(".hidden"), "secret").unwrap();
        std::fs::write(root.join("visible.txt"), "hello").unwrap();

        let mut tree = FileTree::new(root);
        tree.expand(0).unwrap();

        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(children.len(), 1); // only visible.txt
        assert_eq!(tree.get(children[0]).unwrap().name, "visible.txt");
    }

    #[test]
    fn tree_collapse_preserves_children() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("file.txt"), "hello").unwrap();

        let mut tree = FileTree::new(root);
        tree.expand(0).unwrap();
        assert!(tree.get(0).unwrap().expanded);

        tree.collapse(0);
        assert!(!tree.get(0).unwrap().expanded);
        // Children still in memory
        assert!(tree.get(0).unwrap().children.is_some());
    }

    #[test]
    fn tree_toggle_expand_collapse() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("file.txt"), "hello").unwrap();

        let mut tree = FileTree::new(root);
        tree.toggle(0).unwrap(); // expand
        assert!(tree.get(0).unwrap().expanded);
        tree.toggle(0).unwrap(); // collapse
        assert!(!tree.get(0).unwrap().expanded);
    }

    #[test]
    fn tree_expand_file_is_noop() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        tree.add_child(0, "file.rs", NodeType::File { extension: Some("rs".into()), size: 100 });
        tree.nodes[0].expanded = true;

        let child_idx = tree.get(0).unwrap().children.as_ref().unwrap()[0];
        tree.expand(child_idx).unwrap(); // should be no-op
        assert!(!tree.get(child_idx).unwrap().expanded);
    }

    #[test]
    fn tree_expand_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir(root.join("empty")).unwrap();

        let mut tree = FileTree::new(root);
        tree.expand(0).unwrap();

        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        let empty_idx = children[0];
        tree.expand(empty_idx).unwrap();

        let empty_children = tree.get(empty_idx).unwrap().children.as_ref().unwrap();
        assert!(empty_children.is_empty());
    }

    // --- Virtual scrolling ---

    #[test]
    fn visible_range_basic() {
        let (start, end) = visible_range(0.0, 280.0, 28.0, 100);
        assert_eq!(start, 0);
        assert_eq!(end, 11); // ceil(280/28) + 1 = 11
    }

    #[test]
    fn visible_range_scrolled() {
        let (start, end) = visible_range(56.0, 280.0, 28.0, 100);
        assert_eq!(start, 2); // floor(56/28) = 2
        assert_eq!(end, 13); // 2 + 11
    }

    #[test]
    fn visible_range_clamped_to_total() {
        let (start, end) = visible_range(0.0, 1000.0, 28.0, 5);
        assert_eq!(start, 0);
        assert_eq!(end, 5); // clamped to total_rows
    }

    #[test]
    fn visible_range_empty() {
        let (start, end) = visible_range(0.0, 280.0, 28.0, 0);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn visible_range_zero_height() {
        let (start, end) = visible_range(0.0, 0.0, 28.0, 100);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    // --- File icon mapping ---

    #[test]
    fn file_icon_rust() {
        assert_eq!(file_icon(Some("rs")), "\u{2699}");
    }

    #[test]
    fn file_icon_unknown() {
        assert_eq!(file_icon(Some("xyz")), "\u{25A1}");
        assert_eq!(file_icon(None), "\u{25A1}");
    }

    #[test]
    fn file_icon_javascript() {
        assert_eq!(file_icon(Some("js")), "JS");
        assert_eq!(file_icon(Some("jsx")), "JS");
    }

    #[test]
    fn file_icon_label_known() {
        assert_eq!(file_icon_label(Some("rs")), "rs");
        assert_eq!(file_icon_label(Some("py")), "py");
    }

    #[test]
    fn file_icon_label_unknown() {
        assert_eq!(file_icon_label(Some("xyz")), "--");
        assert_eq!(file_icon_label(None), "--");
    }

    // --- Breadcrumb tests ---

    #[test]
    fn breadcrumb_from_root() {
        let root = PathBuf::from("/home/user/project");
        let segments = breadcrumb_segments(&root, &root);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "project");
        assert_eq!(segments[0].1, root);
    }

    #[test]
    fn breadcrumb_with_subpath() {
        let root = PathBuf::from("/home/user/project");
        let path = PathBuf::from("/home/user/project/src/main.rs");
        let segments = breadcrumb_segments(&path, &root);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].0, "project");
        assert_eq!(segments[1].0, "src");
        assert_eq!(segments[2].0, "main.rs");
        assert_eq!(segments[2].1, path);
    }

    #[test]
    fn breadcrumb_unrelated_path() {
        let root = PathBuf::from("/home/user/project");
        let path = PathBuf::from("/other/path");
        let segments = breadcrumb_segments(&path, &root);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "path");
    }

    // --- Keyboard navigation tests ---

    #[test]
    fn nav_down_from_none_selects_first() {
        let mut nav = TreeNavState::new();
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
        ];
        nav.apply(TreeNavAction::Down, &rows);
        assert_eq!(nav.selected_visible_row, Some(0));
    }

    #[test]
    fn nav_down_advances() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
            VisibleRow { index: 1, depth: 1, name: "src".into(), node_type: NodeType::Directory, expanded: false, has_children: true },
        ];
        nav.apply(TreeNavAction::Down, &rows);
        assert_eq!(nav.selected_visible_row, Some(1));
    }

    #[test]
    fn nav_up_at_top_stays() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
        ];
        nav.apply(TreeNavAction::Up, &rows);
        assert_eq!(nav.selected_visible_row, Some(0));
    }

    #[test]
    fn nav_down_at_bottom_stays() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
        ];
        nav.apply(TreeNavAction::Down, &rows);
        assert_eq!(nav.selected_visible_row, Some(0)); // stays, already at last
    }

    #[test]
    fn nav_left_collapses_expanded_dir() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
        ];
        let result = nav.apply(TreeNavAction::Left, &rows);
        assert_eq!(result, Some(TreeNavResult::Collapse(0)));
    }

    #[test]
    fn nav_left_on_file_goes_to_parent() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(1);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
            VisibleRow { index: 1, depth: 1, name: "file.rs".into(), node_type: NodeType::File { extension: Some("rs".into()), size: 100 }, expanded: false, has_children: false },
        ];
        nav.apply(TreeNavAction::Left, &rows);
        assert_eq!(nav.selected_visible_row, Some(0)); // moved to parent
    }

    #[test]
    fn nav_right_expands_collapsed_dir() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 5, depth: 1, name: "src".into(), node_type: NodeType::Directory, expanded: false, has_children: true },
        ];
        let result = nav.apply(TreeNavAction::Right, &rows);
        assert_eq!(result, Some(TreeNavResult::Expand(5)));
    }

    #[test]
    fn nav_right_moves_into_expanded_dir() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "src".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
            VisibleRow { index: 1, depth: 1, name: "main.rs".into(), node_type: NodeType::File { extension: Some("rs".into()), size: 100 }, expanded: false, has_children: false },
        ];
        nav.apply(TreeNavAction::Right, &rows);
        assert_eq!(nav.selected_visible_row, Some(1));
    }

    #[test]
    fn nav_enter_on_file_opens() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 3, depth: 1, name: "main.rs".into(), node_type: NodeType::File { extension: Some("rs".into()), size: 100 }, expanded: false, has_children: false },
        ];
        let result = nav.apply(TreeNavAction::Enter, &rows);
        assert_eq!(result, Some(TreeNavResult::OpenFile(3)));
    }

    #[test]
    fn nav_enter_on_collapsed_dir_expands() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 2, depth: 0, name: "docs".into(), node_type: NodeType::Directory, expanded: false, has_children: true },
        ];
        let result = nav.apply(TreeNavAction::Enter, &rows);
        assert_eq!(result, Some(TreeNavResult::Expand(2)));
    }

    #[test]
    fn nav_enter_on_expanded_dir_collapses() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 2, depth: 0, name: "docs".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
        ];
        let result = nav.apply(TreeNavAction::Enter, &rows);
        assert_eq!(result, Some(TreeNavResult::Collapse(2)));
    }

    #[test]
    fn nav_empty_rows_returns_none() {
        let mut nav = TreeNavState::new();
        let rows: Vec<VisibleRow> = vec![];
        assert!(nav.apply(TreeNavAction::Down, &rows).is_none());
        assert!(nav.apply(TreeNavAction::Enter, &rows).is_none());
    }

    #[test]
    fn visible_row_has_children_false_for_empty_loaded_dir() {
        let mut tree = FileTree::new(PathBuf::from("/tmp/project"));
        let empty_dir = tree.add_child(0, "empty", NodeType::Directory);
        tree.nodes[empty_dir].children = Some(vec![]); // loaded but empty
        tree.nodes[0].expanded = true;

        let rows = tree.visible_rows();
        let empty_row = &rows[1];
        assert_eq!(empty_row.name, "empty");
        assert!(!empty_row.has_children); // loaded and empty
    }

    // --- Symlink NodeType ---

    #[test]
    fn symlink_node_type_exists() {
        let s = NodeType::Symlink { target: PathBuf::from("/tmp/target") };
        assert_ne!(s, NodeType::File { extension: None, size: 0 });
        assert_ne!(s, NodeType::Directory);
    }

    // --- show_hidden ---

    #[test]
    fn show_hidden_default_false() {
        let tree = FileTree::new(PathBuf::from("/tmp/project"));
        assert!(!tree.show_hidden);
    }

    #[test]
    fn show_hidden_includes_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join(".hidden"), "secret").unwrap();
        std::fs::write(root.join("visible.txt"), "hello").unwrap();

        // show_hidden = false: only visible.txt
        let mut tree = FileTree::new(root.clone());
        tree.expand(0).unwrap();
        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(tree.get(children[0]).unwrap().name, "visible.txt");

        // show_hidden = true: both files
        let mut tree2 = FileTree::new(root);
        tree2.show_hidden = true;
        tree2.expand(0).unwrap();
        let children2 = tree2.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(children2.len(), 2);
    }

    // --- Large directory guard ---

    #[test]
    fn large_dir_truncation() {
        // We can't easily create 10001 files quickly, so test the constant exists
        // and the logic by creating a dir with a few files and verifying it works
        assert_eq!(MAX_DIR_ENTRIES, 10_000);

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        for i in 0..5 {
            std::fs::write(root.join(format!("file{}.txt", i)), "x").unwrap();
        }
        let mut tree = FileTree::new(root);
        tree.expand(0).unwrap();
        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        assert_eq!(children.len(), 5); // well under limit
    }

    // --- Home/End navigation ---

    #[test]
    fn nav_home_jumps_to_first() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(3);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
            VisibleRow { index: 1, depth: 1, name: "a.txt".into(), node_type: NodeType::File { extension: None, size: 0 }, expanded: false, has_children: false },
            VisibleRow { index: 2, depth: 1, name: "b.txt".into(), node_type: NodeType::File { extension: None, size: 0 }, expanded: false, has_children: false },
            VisibleRow { index: 3, depth: 1, name: "c.txt".into(), node_type: NodeType::File { extension: None, size: 0 }, expanded: false, has_children: false },
        ];
        nav.apply(TreeNavAction::Home, &rows);
        assert_eq!(nav.selected_visible_row, Some(0));
    }

    #[test]
    fn nav_end_jumps_to_last() {
        let mut nav = TreeNavState::new();
        nav.selected_visible_row = Some(0);
        let rows = vec![
            VisibleRow { index: 0, depth: 0, name: "root".into(), node_type: NodeType::Directory, expanded: true, has_children: true },
            VisibleRow { index: 1, depth: 1, name: "a.txt".into(), node_type: NodeType::File { extension: None, size: 0 }, expanded: false, has_children: false },
            VisibleRow { index: 2, depth: 1, name: "b.txt".into(), node_type: NodeType::File { extension: None, size: 0 }, expanded: false, has_children: false },
        ];
        nav.apply(TreeNavAction::End, &rows);
        assert_eq!(nav.selected_visible_row, Some(2));
    }

    // --- Symlink in real filesystem ---

    #[test]
    fn tree_expand_detects_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("real.txt"), "content").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("real.txt"), root.join("link.txt")).unwrap();
            let mut tree = FileTree::new(root);
            tree.expand(0).unwrap();
            let children = tree.get(0).unwrap().children.as_ref().unwrap();
            let names: Vec<_> = children.iter().map(|&i| tree.get(i).unwrap().name.as_str()).collect();
            assert!(names.contains(&"link.txt"));
            // Find the symlink node
            let link_node = children.iter().find(|&&i| tree.get(i).unwrap().name == "link.txt").unwrap();
            assert!(matches!(tree.get(*link_node).unwrap().node_type, NodeType::Symlink { .. }));
        }
    }

    #[test]
    fn tree_file_extension_captured() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("main.rs"), "fn main() {}").unwrap();

        let mut tree = FileTree::new(root);
        tree.expand(0).unwrap();

        let children = tree.get(0).unwrap().children.as_ref().unwrap();
        let file = tree.get(children[0]).unwrap();
        match &file.node_type {
            NodeType::File { extension, size } => {
                assert_eq!(extension.as_deref(), Some("rs"));
                assert!(*size > 0);
            }
            _ => panic!("expected file node"),
        }
    }
}
