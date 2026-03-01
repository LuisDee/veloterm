// File browser actions — hidden files filtering, gitignore detection,
// and context menu action dispatch.

use std::path::{Path, PathBuf};

use crate::file_browser::tree::{FileNode, NodeType, VisibleRow};

/// Context menu action that can be performed on a file or directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuAction {
    /// Copy the absolute path to clipboard.
    CopyAbsolutePath,
    /// Copy the path relative to the tree root.
    CopyRelativePath,
    /// Copy just the filename.
    CopyFilename,
    /// Open the containing directory in the system file manager.
    RevealInFileManager,
    /// Move the file or directory to the system trash.
    MoveToTrash,
}

/// Result of executing a context menu action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    /// Text to copy to clipboard.
    CopyText(String),
    /// File manager was opened (or would be opened).
    FileManagerOpened,
    /// Item was moved to trash.
    MovedToTrash,
    /// Action failed with an error message.
    Error(String),
}

/// Execute a context menu action on a file path.
///
/// `absolute_path` is the full path to the file/directory.
/// `root` is the tree root for computing relative paths.
pub fn execute_action(
    action: &ContextMenuAction,
    absolute_path: &Path,
    root: &Path,
) -> ActionResult {
    match action {
        ContextMenuAction::CopyAbsolutePath => {
            ActionResult::CopyText(absolute_path.to_string_lossy().to_string())
        }
        ContextMenuAction::CopyRelativePath => {
            let relative = absolute_path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| absolute_path.to_string_lossy().to_string());
            ActionResult::CopyText(relative)
        }
        ContextMenuAction::CopyFilename => {
            let name = absolute_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ActionResult::CopyText(name)
        }
        ContextMenuAction::RevealInFileManager => {
            let dir = if absolute_path.is_dir() {
                absolute_path
            } else {
                absolute_path.parent().unwrap_or(absolute_path)
            };
            match open::that(dir) {
                Ok(()) => ActionResult::FileManagerOpened,
                Err(e) => ActionResult::Error(format!("Failed to open file manager: {e}")),
            }
        }
        ContextMenuAction::MoveToTrash => match trash::delete(absolute_path) {
            Ok(()) => ActionResult::MovedToTrash,
            Err(e) => ActionResult::Error(format!("Failed to move to trash: {e}")),
        },
    }
}

/// Check if a filename is a hidden file (starts with '.').
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Check if a file path is ignored by .gitignore rules in a git repository.
///
/// Uses git2 to check if the path would be ignored.
/// Returns false if not in a git repo or on error.
pub fn is_gitignored(repo_path: &Path, file_path: &Path) -> bool {
    let repo = match git2::Repository::discover(repo_path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let workdir = match repo.workdir() {
        Some(w) => w,
        None => return false,
    };
    // Canonicalize to handle macOS /var -> /private/var symlinks
    let canonical_workdir = workdir.canonicalize().unwrap_or_else(|_| workdir.to_path_buf());
    let canonical_file = file_path.canonicalize().unwrap_or_else(|_| file_path.to_path_buf());
    let relative = match canonical_file.strip_prefix(&canonical_workdir) {
        Ok(r) => r,
        Err(_) => return false,
    };
    repo.is_path_ignored(relative).unwrap_or(false)
}

/// Filter visible rows based on hidden files toggle.
///
/// When `show_hidden` is false, rows whose names start with '.' are excluded.
/// Returns a new Vec with only visible rows.
pub fn filter_hidden(rows: &[VisibleRow], show_hidden: bool) -> Vec<VisibleRow> {
    if show_hidden {
        return rows.to_vec();
    }
    rows.iter()
        .filter(|row| !is_hidden(&row.name))
        .cloned()
        .collect()
}

/// Expand the tree including hidden files when show_hidden is true.
/// This is an alternative expand that doesn't skip dot-files.
pub fn expand_with_hidden(
    tree_nodes: &mut Vec<FileNode>,
    index: usize,
    show_hidden: bool,
) -> std::io::Result<()> {
    if index >= tree_nodes.len() {
        return Ok(());
    }
    if tree_nodes[index].node_type != NodeType::Directory {
        return Ok(());
    }
    // If children already loaded, just set expanded
    if tree_nodes[index].children.is_some() {
        tree_nodes[index].expanded = true;
        return Ok(());
    }

    let path = tree_nodes[index].path.clone();
    let depth = tree_nodes[index].depth + 1;
    let mut child_indices = Vec::new();

    let entries = std::fs::read_dir(&path)?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files unless show_hidden is true
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata()?;
        let node_type = if metadata.is_dir() {
            NodeType::Directory
        } else {
            NodeType::File {
                extension: entry_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string()),
                size: metadata.len(),
            }
        };

        let child_index = tree_nodes.len();
        child_indices.push(child_index);
        tree_nodes.push(FileNode {
            name,
            path: entry_path,
            node_type,
            depth,
            parent: Some(index),
            children: None,
            expanded: false,
        });
    }

    tree_nodes[index].children = Some(child_indices);
    tree_nodes[index].expanded = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_browser::tree::NodeType;
    use std::path::PathBuf;

    // -- is_hidden tests --

    #[test]
    fn hidden_dot_file() {
        assert!(is_hidden(".gitignore"));
        assert!(is_hidden(".env"));
        assert!(is_hidden(".hidden"));
    }

    #[test]
    fn visible_file_not_hidden() {
        assert!(!is_hidden("main.rs"));
        assert!(!is_hidden("README.md"));
        assert!(!is_hidden("Cargo.toml"));
    }

    #[test]
    fn empty_string_not_hidden() {
        assert!(!is_hidden(""));
    }

    #[test]
    fn dot_dot_is_hidden() {
        assert!(is_hidden(".."));
        assert!(is_hidden("."));
    }

    // -- filter_hidden tests --

    fn make_rows() -> Vec<VisibleRow> {
        vec![
            VisibleRow {
                index: 0,
                depth: 0,
                name: "project".into(),
                node_type: NodeType::Directory,
                expanded: true,
                has_children: true,
                is_last_child: false,
                ancestor_has_next_sibling: vec![],
            },
            VisibleRow {
                index: 1,
                depth: 1,
                name: ".git".into(),
                node_type: NodeType::Directory,
                expanded: false,
                has_children: true,
                is_last_child: false,
                ancestor_has_next_sibling: vec![],
            },
            VisibleRow {
                index: 2,
                depth: 1,
                name: ".gitignore".into(),
                node_type: NodeType::File {
                    extension: None,
                    size: 50,
                },
                expanded: false,
                has_children: false,
                is_last_child: false,
                ancestor_has_next_sibling: vec![],
            },
            VisibleRow {
                index: 3,
                depth: 1,
                name: "src".into(),
                node_type: NodeType::Directory,
                expanded: false,
                has_children: true,
                is_last_child: false,
                ancestor_has_next_sibling: vec![],
            },
            VisibleRow {
                index: 4,
                depth: 1,
                name: "README.md".into(),
                node_type: NodeType::File {
                    extension: Some("md".into()),
                    size: 100,
                },
                expanded: false,
                has_children: false,
                is_last_child: true,
                ancestor_has_next_sibling: vec![],
            },
        ]
    }

    #[test]
    fn filter_hidden_hides_dot_files() {
        let rows = make_rows();
        let filtered = filter_hidden(&rows, false);
        // Should have: project, src, README.md (3 items, .git and .gitignore filtered)
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().all(|r| !r.name.starts_with('.')));
    }

    #[test]
    fn filter_hidden_show_all_when_enabled() {
        let rows = make_rows();
        let filtered = filter_hidden(&rows, true);
        assert_eq!(filtered.len(), 5); // all rows
    }

    #[test]
    fn filter_hidden_empty_input() {
        let filtered = filter_hidden(&[], false);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_hidden_preserves_order() {
        let rows = make_rows();
        let filtered = filter_hidden(&rows, false);
        assert_eq!(filtered[0].name, "project");
        assert_eq!(filtered[1].name, "src");
        assert_eq!(filtered[2].name, "README.md");
    }

    // -- Context menu action tests --

    #[test]
    fn action_copy_absolute_path() {
        let path = PathBuf::from("/home/user/project/src/main.rs");
        let root = PathBuf::from("/home/user/project");
        let result = execute_action(&ContextMenuAction::CopyAbsolutePath, &path, &root);
        assert_eq!(
            result,
            ActionResult::CopyText("/home/user/project/src/main.rs".to_string())
        );
    }

    #[test]
    fn action_copy_relative_path() {
        let path = PathBuf::from("/home/user/project/src/main.rs");
        let root = PathBuf::from("/home/user/project");
        let result = execute_action(&ContextMenuAction::CopyRelativePath, &path, &root);
        assert_eq!(
            result,
            ActionResult::CopyText("src/main.rs".to_string())
        );
    }

    #[test]
    fn action_copy_relative_path_at_root() {
        let path = PathBuf::from("/home/user/project/README.md");
        let root = PathBuf::from("/home/user/project");
        let result = execute_action(&ContextMenuAction::CopyRelativePath, &path, &root);
        assert_eq!(
            result,
            ActionResult::CopyText("README.md".to_string())
        );
    }

    #[test]
    fn action_copy_filename() {
        let path = PathBuf::from("/home/user/project/src/main.rs");
        let root = PathBuf::from("/home/user/project");
        let result = execute_action(&ContextMenuAction::CopyFilename, &path, &root);
        assert_eq!(
            result,
            ActionResult::CopyText("main.rs".to_string())
        );
    }

    #[test]
    fn action_copy_filename_directory() {
        let path = PathBuf::from("/home/user/project/src");
        let root = PathBuf::from("/home/user/project");
        let result = execute_action(&ContextMenuAction::CopyFilename, &path, &root);
        assert_eq!(result, ActionResult::CopyText("src".to_string()));
    }

    #[test]
    fn action_move_to_trash_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("trash_me.txt");
        std::fs::write(&file, "delete this").unwrap();
        assert!(file.exists());

        let result = execute_action(
            &ContextMenuAction::MoveToTrash,
            &file,
            dir.path(),
        );
        assert_eq!(result, ActionResult::MovedToTrash);
        assert!(!file.exists());
    }

    #[test]
    fn action_move_to_trash_nonexistent_file() {
        let path = PathBuf::from("/tmp/nonexistent_file_for_trash_test_xyz");
        let result = execute_action(
            &ContextMenuAction::MoveToTrash,
            &path,
            Path::new("/tmp"),
        );
        // Should return Error since file doesn't exist
        assert!(matches!(result, ActionResult::Error(_)));
    }

    // -- gitignore detection tests --

    #[test]
    fn gitignored_file_detected() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        let log_file = dir.path().join("debug.log");
        std::fs::write(&log_file, "log").unwrap();

        assert!(is_gitignored(dir.path(), &log_file));
    }

    #[test]
    fn non_ignored_file_not_detected() {
        let dir = tempfile::tempdir().unwrap();
        let _repo = git2::Repository::init(dir.path()).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        let rs_file = dir.path().join("main.rs");
        std::fs::write(&rs_file, "fn main(){}").unwrap();

        assert!(!is_gitignored(dir.path(), &rs_file));
    }

    #[test]
    fn gitignored_outside_repo_returns_false() {
        let path = PathBuf::from("/tmp/definitely_not_a_repo_xyz/file.txt");
        assert!(!is_gitignored(Path::new("/tmp/definitely_not_a_repo_xyz"), &path));
    }

    // -- expand_with_hidden tests --

    #[test]
    fn expand_with_hidden_false_skips_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join(".hidden"), "secret").unwrap();
        std::fs::write(root.join("visible.txt"), "hello").unwrap();

        let mut nodes = vec![FileNode {
            name: "root".to_string(),
            path: root,
            node_type: NodeType::Directory,
            depth: 0,
            parent: None,
            children: None,
            expanded: false,
        }];

        expand_with_hidden(&mut nodes, 0, false).unwrap();
        let children = nodes[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(nodes[children[0]].name, "visible.txt");
    }

    #[test]
    fn expand_with_hidden_true_includes_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join(".hidden"), "secret").unwrap();
        std::fs::write(root.join("visible.txt"), "hello").unwrap();

        let mut nodes = vec![FileNode {
            name: "root".to_string(),
            path: root,
            node_type: NodeType::Directory,
            depth: 0,
            parent: None,
            children: None,
            expanded: false,
        }];

        expand_with_hidden(&mut nodes, 0, true).unwrap();
        let children = nodes[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        let names: Vec<&str> = children.iter().map(|&i| nodes[i].name.as_str()).collect();
        assert!(names.contains(&".hidden"));
        assert!(names.contains(&"visible.txt"));
    }

    #[test]
    fn expand_with_hidden_on_file_is_noop() {
        let mut nodes = vec![FileNode {
            name: "file.rs".to_string(),
            path: PathBuf::from("/tmp/file.rs"),
            node_type: NodeType::File {
                extension: Some("rs".into()),
                size: 100,
            },
            depth: 0,
            parent: None,
            children: None,
            expanded: false,
        }];

        expand_with_hidden(&mut nodes, 0, true).unwrap();
        assert!(!nodes[0].expanded);
    }

    #[test]
    fn expand_with_hidden_already_loaded_just_sets_expanded() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.txt"), "a").unwrap();

        let mut nodes = vec![FileNode {
            name: "root".to_string(),
            path: root,
            node_type: NodeType::Directory,
            depth: 0,
            parent: None,
            children: Some(vec![]), // already loaded (empty)
            expanded: false,
        }];

        expand_with_hidden(&mut nodes, 0, true).unwrap();
        assert!(nodes[0].expanded);
        // Children not reloaded (still empty from initial load)
        assert_eq!(nodes[0].children.as_ref().unwrap().len(), 0);
    }

    // -- ContextMenuAction equality --

    #[test]
    fn context_menu_actions_are_comparable() {
        assert_eq!(ContextMenuAction::CopyAbsolutePath, ContextMenuAction::CopyAbsolutePath);
        assert_ne!(ContextMenuAction::CopyAbsolutePath, ContextMenuAction::CopyFilename);
    }
}
