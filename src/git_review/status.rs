// Git status data model — categorized file status for the git review overlay.

use std::path::{Path, PathBuf};

/// Categorized git file status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
    Untracked,
}

impl FileStatus {
    /// Single-character label for display.
    pub fn label(&self) -> &str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed { .. } => "R",
            Self::Untracked => "?",
        }
    }
}

/// A file entry with its git status.
#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: PathBuf,
    pub status: FileStatus,
    pub display_name: String,
    pub display_dir: Option<String>,
}

impl StatusEntry {
    /// Build a StatusEntry from a path and status, extracting display components.
    pub fn from_path(path: &Path, status: FileStatus) -> Self {
        let display_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let display_dir = path.parent().and_then(|p| {
            let s = p.to_string_lossy().into_owned();
            if s.is_empty() { None } else { Some(s) }
        });
        Self {
            path: path.to_path_buf(),
            status,
            display_name,
            display_dir,
        }
    }
}

/// Categorized status sections.
#[derive(Debug, Clone)]
pub struct GitStatus {
    pub staged: Vec<StatusEntry>,
    pub changed: Vec<StatusEntry>,
    pub untracked: Vec<StatusEntry>,
}

impl GitStatus {
    /// Read git status from a repository.
    pub fn from_repo(repo: &git2::Repository) -> Result<Self, git2::Error> {
        let statuses = repo.statuses(None)?;
        let mut staged = Vec::new();
        let mut changed = Vec::new();
        let mut untracked = Vec::new();

        for entry in statuses.iter() {
            let path = match entry.path() {
                Some(p) => PathBuf::from(p),
                None => continue,
            };
            let flags = entry.status();
            let (index_status, workdir_status) = categorize_status(flags);

            if let Some(status) = index_status {
                staged.push(StatusEntry::from_path(&path, status));
            }
            if let Some(status) = workdir_status {
                if status == FileStatus::Untracked {
                    untracked.push(StatusEntry::from_path(&path, status));
                } else {
                    changed.push(StatusEntry::from_path(&path, status));
                }
            }
        }

        // Sort each section by path
        staged.sort_by(|a, b| a.path.cmp(&b.path));
        changed.sort_by(|a, b| a.path.cmp(&b.path));
        untracked.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(Self {
            staged,
            changed,
            untracked,
        })
    }

    /// Total number of entries across all sections.
    pub fn total_count(&self) -> usize {
        self.staged.len() + self.changed.len() + self.untracked.len()
    }

    /// Whether there are no changes at all.
    pub fn is_empty(&self) -> bool {
        self.staged.is_empty() && self.changed.is_empty() && self.untracked.is_empty()
    }
}

/// Categorize git2 status flags into index (staged) and workdir (changed/untracked) statuses.
pub fn categorize_status(
    flags: git2::Status,
) -> (Option<FileStatus>, Option<FileStatus>) {
    let index = if flags.contains(git2::Status::INDEX_NEW) {
        Some(FileStatus::Added)
    } else if flags.contains(git2::Status::INDEX_MODIFIED) {
        Some(FileStatus::Modified)
    } else if flags.contains(git2::Status::INDEX_DELETED) {
        Some(FileStatus::Deleted)
    } else if flags.contains(git2::Status::INDEX_RENAMED) {
        // git2 doesn't expose the old name via status flags alone;
        // rename detection requires a diff. We record it as renamed with empty `from`.
        Some(FileStatus::Renamed {
            from: PathBuf::new(),
        })
    } else {
        None
    };

    let workdir = if flags.contains(git2::Status::WT_NEW) {
        Some(FileStatus::Untracked)
    } else if flags.contains(git2::Status::WT_MODIFIED) {
        Some(FileStatus::Modified)
    } else if flags.contains(git2::Status::WT_DELETED) {
        Some(FileStatus::Deleted)
    } else if flags.contains(git2::Status::WT_RENAMED) {
        Some(FileStatus::Renamed {
            from: PathBuf::new(),
        })
    } else {
        None
    };

    (index, workdir)
}

/// Section collapse state.
#[derive(Debug, Clone)]
pub struct SectionState {
    pub staged_collapsed: bool,
    pub changed_collapsed: bool,
    pub untracked_collapsed: bool,
}

impl Default for SectionState {
    fn default() -> Self {
        Self {
            staged_collapsed: false,
            changed_collapsed: false,
            untracked_collapsed: false,
        }
    }
}

impl SectionState {
    pub fn toggle_staged(&mut self) {
        self.staged_collapsed = !self.staged_collapsed;
    }

    pub fn toggle_changed(&mut self) {
        self.changed_collapsed = !self.changed_collapsed;
    }

    pub fn toggle_untracked(&mut self) {
        self.untracked_collapsed = !self.untracked_collapsed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // -- FileStatus label tests --

    #[test]
    fn file_status_label_added() {
        assert_eq!(FileStatus::Added.label(), "A");
    }

    #[test]
    fn file_status_label_modified() {
        assert_eq!(FileStatus::Modified.label(), "M");
    }

    #[test]
    fn file_status_label_deleted() {
        assert_eq!(FileStatus::Deleted.label(), "D");
    }

    #[test]
    fn file_status_label_renamed() {
        let r = FileStatus::Renamed {
            from: PathBuf::from("old.rs"),
        };
        assert_eq!(r.label(), "R");
    }

    #[test]
    fn file_status_label_untracked() {
        assert_eq!(FileStatus::Untracked.label(), "?");
    }

    // -- StatusEntry::from_path tests --

    #[test]
    fn status_entry_root_file() {
        let entry = StatusEntry::from_path(Path::new("README.md"), FileStatus::Modified);
        assert_eq!(entry.display_name, "README.md");
        assert_eq!(entry.display_dir, None);
        assert_eq!(entry.path, PathBuf::from("README.md"));
    }

    #[test]
    fn status_entry_nested_file() {
        let entry = StatusEntry::from_path(Path::new("src/main.rs"), FileStatus::Added);
        assert_eq!(entry.display_name, "main.rs");
        assert_eq!(entry.display_dir, Some("src".to_string()));
    }

    #[test]
    fn status_entry_deeply_nested() {
        let entry =
            StatusEntry::from_path(Path::new("src/git_review/status.rs"), FileStatus::Modified);
        assert_eq!(entry.display_name, "status.rs");
        assert_eq!(entry.display_dir, Some("src/git_review".to_string()));
    }

    // -- categorize_status tests --

    #[test]
    fn categorize_index_new() {
        let (idx, wd) = categorize_status(git2::Status::INDEX_NEW);
        assert_eq!(idx, Some(FileStatus::Added));
        assert_eq!(wd, None);
    }

    #[test]
    fn categorize_index_modified() {
        let (idx, wd) = categorize_status(git2::Status::INDEX_MODIFIED);
        assert_eq!(idx, Some(FileStatus::Modified));
        assert_eq!(wd, None);
    }

    #[test]
    fn categorize_index_deleted() {
        let (idx, wd) = categorize_status(git2::Status::INDEX_DELETED);
        assert_eq!(idx, Some(FileStatus::Deleted));
        assert_eq!(wd, None);
    }

    #[test]
    fn categorize_index_renamed() {
        let (idx, wd) = categorize_status(git2::Status::INDEX_RENAMED);
        assert!(matches!(idx, Some(FileStatus::Renamed { .. })));
        assert_eq!(wd, None);
    }

    #[test]
    fn categorize_wt_new() {
        let (idx, wd) = categorize_status(git2::Status::WT_NEW);
        assert_eq!(idx, None);
        assert_eq!(wd, Some(FileStatus::Untracked));
    }

    #[test]
    fn categorize_wt_modified() {
        let (idx, wd) = categorize_status(git2::Status::WT_MODIFIED);
        assert_eq!(idx, None);
        assert_eq!(wd, Some(FileStatus::Modified));
    }

    #[test]
    fn categorize_wt_deleted() {
        let (idx, wd) = categorize_status(git2::Status::WT_DELETED);
        assert_eq!(idx, None);
        assert_eq!(wd, Some(FileStatus::Deleted));
    }

    #[test]
    fn categorize_wt_renamed() {
        let (idx, wd) = categorize_status(git2::Status::WT_RENAMED);
        assert_eq!(idx, None);
        assert!(matches!(wd, Some(FileStatus::Renamed { .. })));
    }

    #[test]
    fn categorize_both_index_and_workdir() {
        let flags = git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED;
        let (idx, wd) = categorize_status(flags);
        assert_eq!(idx, Some(FileStatus::Modified));
        assert_eq!(wd, Some(FileStatus::Modified));
    }

    #[test]
    fn categorize_ignored_returns_none() {
        let (idx, wd) = categorize_status(git2::Status::IGNORED);
        assert_eq!(idx, None);
        assert_eq!(wd, None);
    }

    #[test]
    fn categorize_empty_returns_none() {
        let (idx, wd) = categorize_status(git2::Status::CURRENT);
        assert_eq!(idx, None);
        assert_eq!(wd, None);
    }

    // -- SectionState tests --

    #[test]
    fn section_state_default_all_expanded() {
        let state = SectionState::default();
        assert!(!state.staged_collapsed);
        assert!(!state.changed_collapsed);
        assert!(!state.untracked_collapsed);
    }

    #[test]
    fn section_state_toggle_staged() {
        let mut state = SectionState::default();
        state.toggle_staged();
        assert!(state.staged_collapsed);
        state.toggle_staged();
        assert!(!state.staged_collapsed);
    }

    #[test]
    fn section_state_toggle_changed() {
        let mut state = SectionState::default();
        state.toggle_changed();
        assert!(state.changed_collapsed);
    }

    #[test]
    fn section_state_toggle_untracked() {
        let mut state = SectionState::default();
        state.toggle_untracked();
        assert!(state.untracked_collapsed);
    }

    // -- GitStatus helper tests --

    #[test]
    fn git_status_empty() {
        let status = GitStatus {
            staged: vec![],
            changed: vec![],
            untracked: vec![],
        };
        assert!(status.is_empty());
        assert_eq!(status.total_count(), 0);
    }

    #[test]
    fn git_status_total_count() {
        let status = GitStatus {
            staged: vec![StatusEntry::from_path(Path::new("a.rs"), FileStatus::Added)],
            changed: vec![
                StatusEntry::from_path(Path::new("b.rs"), FileStatus::Modified),
                StatusEntry::from_path(Path::new("c.rs"), FileStatus::Deleted),
            ],
            untracked: vec![StatusEntry::from_path(
                Path::new("d.rs"),
                FileStatus::Untracked,
            )],
        };
        assert!(!status.is_empty());
        assert_eq!(status.total_count(), 4);
    }

    // -- Integration test with real git2 repo --

    fn setup_test_repo() -> (tempfile::TempDir, git2::Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    #[test]
    fn from_repo_empty_repo() {
        let (_dir, repo) = setup_test_repo();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert!(status.is_empty());
    }

    #[test]
    fn from_repo_untracked_file() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(status.untracked[0].display_name, "new.txt");
        assert_eq!(status.untracked[0].status, FileStatus::Untracked);
        assert!(status.staged.is_empty());
        assert!(status.changed.is_empty());
    }

    #[test]
    fn from_repo_staged_new_file() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].status, FileStatus::Added);
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn from_repo_modified_tracked_file() {
        let (dir, repo) = setup_test_repo();
        // Create initial commit
        std::fs::write(dir.path().join("file.txt"), "v1").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Modify file
        std::fs::write(dir.path().join("file.txt"), "v2").unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.changed.len(), 1);
        assert_eq!(status.changed[0].status, FileStatus::Modified);
    }

    #[test]
    fn from_repo_entries_sorted_by_path() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("z.txt"), "z").unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("m.txt"), "m").unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        let names: Vec<_> = status.untracked.iter().map(|e| &e.display_name).collect();
        assert_eq!(names, vec!["a.txt", "m.txt", "z.txt"]);
    }

    #[test]
    fn from_repo_staged_and_modified() {
        let (dir, repo) = setup_test_repo();
        // Create initial commit
        std::fs::write(dir.path().join("file.txt"), "v1").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Stage a modification, then modify again
        std::fs::write(dir.path().join("file.txt"), "v2").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        std::fs::write(dir.path().join("file.txt"), "v3").unwrap();

        let status = GitStatus::from_repo(&repo).unwrap();
        // File appears in both staged and changed
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].status, FileStatus::Modified);
        assert_eq!(status.changed.len(), 1);
        assert_eq!(status.changed[0].status, FileStatus::Modified);
    }

    #[test]
    fn from_repo_deleted_tracked_file() {
        let (dir, repo) = setup_test_repo();
        // Create initial commit
        std::fs::write(dir.path().join("file.txt"), "v1").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Delete the file
        std::fs::remove_file(dir.path().join("file.txt")).unwrap();
        let status = GitStatus::from_repo(&repo).unwrap();
        assert_eq!(status.changed.len(), 1);
        assert_eq!(status.changed[0].status, FileStatus::Deleted);
    }
}
