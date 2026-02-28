// Git status indicators for the file browser tree — shows M/U/S badges on files
// and propagates status indicators up to parent directories.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Git status indicator for display in the file browser tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitIndicator {
    /// File has been modified in the working directory.
    Modified,
    /// File is untracked (new, not yet staged).
    Untracked,
    /// File has been staged for commit.
    Staged,
    /// File is ignored by .gitignore.
    Ignored,
    /// Directory contains children with status changes (propagated).
    HasChanges,
}

impl GitIndicator {
    /// Single-character label for display.
    pub fn label(&self) -> &str {
        match self {
            Self::Modified => "M",
            Self::Untracked => "U",
            Self::Staged => "S",
            Self::Ignored => "I",
            Self::HasChanges => "*",
        }
    }

    /// Whether this indicator should be shown with a dimmed style.
    pub fn is_dimmed(&self) -> bool {
        matches!(self, Self::Ignored)
    }
}

/// Cached git status for all files in a repository.
pub struct TreeGitStatus {
    /// Map from relative file path to its git indicator.
    file_statuses: HashMap<PathBuf, GitIndicator>,
    /// Map from directory path to propagated indicator (if any child has changes).
    dir_statuses: HashMap<PathBuf, GitIndicator>,
}

impl TreeGitStatus {
    /// Build a TreeGitStatus by reading the git repository status.
    ///
    /// `repo_root` is the working directory root of the repository.
    /// Paths in the returned maps are relative to `repo_root`.
    pub fn from_repo(repo: &git2::Repository) -> Result<Self, git2::Error> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(true)
            .recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts))?;
        let mut file_statuses = HashMap::new();

        for entry in statuses.iter() {
            let path = match entry.path() {
                Some(p) => PathBuf::from(p),
                None => continue,
            };
            let flags = entry.status();
            if let Some(indicator) = map_status_flags(flags) {
                file_statuses.insert(path, indicator);
            }
        }

        let dir_statuses = propagate_to_parents(&file_statuses);

        Ok(Self {
            file_statuses,
            dir_statuses,
        })
    }

    /// Build from a pre-built map (for testing).
    pub fn from_map(file_statuses: HashMap<PathBuf, GitIndicator>) -> Self {
        let dir_statuses = propagate_to_parents(&file_statuses);
        Self {
            file_statuses,
            dir_statuses,
        }
    }

    /// Get the indicator for a file path (relative to repo root).
    pub fn file_status(&self, relative_path: &Path) -> Option<GitIndicator> {
        self.file_statuses.get(relative_path).copied()
    }

    /// Get the indicator for a directory path (relative to repo root).
    /// Returns Some(HasChanges) if any descendant has a status.
    pub fn dir_status(&self, relative_path: &Path) -> Option<GitIndicator> {
        self.dir_statuses.get(relative_path).copied()
    }

    /// Get the indicator for any path (file or directory).
    pub fn status_for(&self, relative_path: &Path) -> Option<GitIndicator> {
        self.file_statuses
            .get(relative_path)
            .or_else(|| self.dir_statuses.get(relative_path))
            .copied()
    }

    /// Whether the cache is empty (no status entries).
    pub fn is_empty(&self) -> bool {
        self.file_statuses.is_empty()
    }

    /// Total number of file entries with status.
    pub fn file_count(&self) -> usize {
        self.file_statuses.len()
    }

    /// Total number of directories with propagated status.
    pub fn dir_count(&self) -> usize {
        self.dir_statuses.len()
    }
}

/// Map git2 status flags to our display indicator.
/// Priority: Staged > Modified > Untracked > Ignored.
fn map_status_flags(flags: git2::Status) -> Option<GitIndicator> {
    // Staged takes priority (index changes)
    if flags.intersects(
        git2::Status::INDEX_NEW
            | git2::Status::INDEX_MODIFIED
            | git2::Status::INDEX_DELETED
            | git2::Status::INDEX_RENAMED,
    ) {
        return Some(GitIndicator::Staged);
    }

    // Working tree modifications
    if flags.intersects(git2::Status::WT_MODIFIED | git2::Status::WT_DELETED | git2::Status::WT_RENAMED) {
        return Some(GitIndicator::Modified);
    }

    // Untracked
    if flags.contains(git2::Status::WT_NEW) {
        return Some(GitIndicator::Untracked);
    }

    // Ignored
    if flags.contains(git2::Status::IGNORED) {
        return Some(GitIndicator::Ignored);
    }

    None
}

/// Propagate status indicators from files up to their parent directories.
/// Any directory that contains a file with Modified, Untracked, or Staged status
/// gets a HasChanges indicator.
fn propagate_to_parents(
    file_statuses: &HashMap<PathBuf, GitIndicator>,
) -> HashMap<PathBuf, GitIndicator> {
    let mut dir_statuses = HashMap::new();

    for (path, indicator) in file_statuses {
        // Skip ignored files for propagation
        if *indicator == GitIndicator::Ignored {
            continue;
        }

        // Walk up to each parent directory
        let mut current = path.as_path();
        while let Some(parent) = current.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            dir_statuses
                .entry(parent.to_path_buf())
                .or_insert(GitIndicator::HasChanges);
            current = parent;
        }
    }

    dir_statuses
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- GitIndicator tests --

    #[test]
    fn indicator_label_modified() {
        assert_eq!(GitIndicator::Modified.label(), "M");
    }

    #[test]
    fn indicator_label_untracked() {
        assert_eq!(GitIndicator::Untracked.label(), "U");
    }

    #[test]
    fn indicator_label_staged() {
        assert_eq!(GitIndicator::Staged.label(), "S");
    }

    #[test]
    fn indicator_label_ignored() {
        assert_eq!(GitIndicator::Ignored.label(), "I");
    }

    #[test]
    fn indicator_label_has_changes() {
        assert_eq!(GitIndicator::HasChanges.label(), "*");
    }

    #[test]
    fn indicator_dimmed_only_for_ignored() {
        assert!(!GitIndicator::Modified.is_dimmed());
        assert!(!GitIndicator::Untracked.is_dimmed());
        assert!(!GitIndicator::Staged.is_dimmed());
        assert!(GitIndicator::Ignored.is_dimmed());
        assert!(!GitIndicator::HasChanges.is_dimmed());
    }

    // -- map_status_flags tests --

    #[test]
    fn map_flags_index_new_is_staged() {
        assert_eq!(map_status_flags(git2::Status::INDEX_NEW), Some(GitIndicator::Staged));
    }

    #[test]
    fn map_flags_index_modified_is_staged() {
        assert_eq!(
            map_status_flags(git2::Status::INDEX_MODIFIED),
            Some(GitIndicator::Staged)
        );
    }

    #[test]
    fn map_flags_index_deleted_is_staged() {
        assert_eq!(
            map_status_flags(git2::Status::INDEX_DELETED),
            Some(GitIndicator::Staged)
        );
    }

    #[test]
    fn map_flags_index_renamed_is_staged() {
        assert_eq!(
            map_status_flags(git2::Status::INDEX_RENAMED),
            Some(GitIndicator::Staged)
        );
    }

    #[test]
    fn map_flags_wt_modified_is_modified() {
        assert_eq!(
            map_status_flags(git2::Status::WT_MODIFIED),
            Some(GitIndicator::Modified)
        );
    }

    #[test]
    fn map_flags_wt_deleted_is_modified() {
        assert_eq!(
            map_status_flags(git2::Status::WT_DELETED),
            Some(GitIndicator::Modified)
        );
    }

    #[test]
    fn map_flags_wt_new_is_untracked() {
        assert_eq!(
            map_status_flags(git2::Status::WT_NEW),
            Some(GitIndicator::Untracked)
        );
    }

    #[test]
    fn map_flags_ignored_is_ignored() {
        assert_eq!(
            map_status_flags(git2::Status::IGNORED),
            Some(GitIndicator::Ignored)
        );
    }

    #[test]
    fn map_flags_current_is_none() {
        assert_eq!(map_status_flags(git2::Status::CURRENT), None);
    }

    #[test]
    fn map_flags_staged_takes_priority_over_wt() {
        // File is both staged and modified in workdir
        let flags = git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED;
        assert_eq!(map_status_flags(flags), Some(GitIndicator::Staged));
    }

    // -- propagate_to_parents tests --

    #[test]
    fn propagate_empty_map_produces_empty() {
        let file_statuses = HashMap::new();
        let dirs = propagate_to_parents(&file_statuses);
        assert!(dirs.is_empty());
    }

    #[test]
    fn propagate_single_file_marks_parent() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(PathBuf::from("src/main.rs"), GitIndicator::Modified);
        let dirs = propagate_to_parents(&file_statuses);
        assert_eq!(dirs.get(Path::new("src")), Some(&GitIndicator::HasChanges));
    }

    #[test]
    fn propagate_nested_file_marks_all_ancestors() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(
            PathBuf::from("src/git_review/status.rs"),
            GitIndicator::Modified,
        );
        let dirs = propagate_to_parents(&file_statuses);
        assert_eq!(
            dirs.get(Path::new("src/git_review")),
            Some(&GitIndicator::HasChanges)
        );
        assert_eq!(dirs.get(Path::new("src")), Some(&GitIndicator::HasChanges));
    }

    #[test]
    fn propagate_ignored_files_not_propagated() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(PathBuf::from("target/debug/out"), GitIndicator::Ignored);
        let dirs = propagate_to_parents(&file_statuses);
        assert!(dirs.is_empty(), "ignored files should not propagate to parents");
    }

    #[test]
    fn propagate_multiple_files_same_dir() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(PathBuf::from("src/a.rs"), GitIndicator::Modified);
        file_statuses.insert(PathBuf::from("src/b.rs"), GitIndicator::Untracked);
        let dirs = propagate_to_parents(&file_statuses);
        assert_eq!(dirs.get(Path::new("src")), Some(&GitIndicator::HasChanges));
        // Only one entry for the directory
        assert_eq!(dirs.len(), 1);
    }

    #[test]
    fn propagate_staged_and_modified_both_propagate() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(PathBuf::from("a/one.rs"), GitIndicator::Staged);
        file_statuses.insert(PathBuf::from("b/two.rs"), GitIndicator::Modified);
        let dirs = propagate_to_parents(&file_statuses);
        assert_eq!(dirs.get(Path::new("a")), Some(&GitIndicator::HasChanges));
        assert_eq!(dirs.get(Path::new("b")), Some(&GitIndicator::HasChanges));
    }

    // -- TreeGitStatus tests --

    #[test]
    fn tree_git_status_from_map_empty() {
        let status = TreeGitStatus::from_map(HashMap::new());
        assert!(status.is_empty());
        assert_eq!(status.file_count(), 0);
        assert_eq!(status.dir_count(), 0);
    }

    #[test]
    fn tree_git_status_file_lookup() {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("src/main.rs"), GitIndicator::Modified);
        let status = TreeGitStatus::from_map(map);
        assert_eq!(
            status.file_status(Path::new("src/main.rs")),
            Some(GitIndicator::Modified)
        );
        assert_eq!(status.file_status(Path::new("other.rs")), None);
    }

    #[test]
    fn tree_git_status_dir_lookup() {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("src/main.rs"), GitIndicator::Modified);
        let status = TreeGitStatus::from_map(map);
        assert_eq!(
            status.dir_status(Path::new("src")),
            Some(GitIndicator::HasChanges)
        );
    }

    #[test]
    fn tree_git_status_status_for_file() {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("src/main.rs"), GitIndicator::Staged);
        let status = TreeGitStatus::from_map(map);
        assert_eq!(
            status.status_for(Path::new("src/main.rs")),
            Some(GitIndicator::Staged)
        );
    }

    #[test]
    fn tree_git_status_status_for_dir() {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("src/main.rs"), GitIndicator::Untracked);
        let status = TreeGitStatus::from_map(map);
        assert_eq!(
            status.status_for(Path::new("src")),
            Some(GitIndicator::HasChanges)
        );
    }

    #[test]
    fn tree_git_status_status_for_unknown() {
        let status = TreeGitStatus::from_map(HashMap::new());
        assert_eq!(status.status_for(Path::new("nonexistent")), None);
    }

    #[test]
    fn tree_git_status_counts() {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("src/a.rs"), GitIndicator::Modified);
        map.insert(PathBuf::from("src/b.rs"), GitIndicator::Untracked);
        map.insert(PathBuf::from("tests/t.rs"), GitIndicator::Staged);
        let status = TreeGitStatus::from_map(map);
        assert_eq!(status.file_count(), 3);
        // dirs: src, tests
        assert_eq!(status.dir_count(), 2);
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
    fn from_repo_empty_has_no_status() {
        let (_dir, repo) = setup_test_repo();
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        assert!(status.is_empty());
    }

    #[test]
    fn from_repo_untracked_file_shows_untracked() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        assert_eq!(
            status.file_status(Path::new("new.txt")),
            Some(GitIndicator::Untracked)
        );
    }

    #[test]
    fn from_repo_staged_file_shows_staged() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        assert_eq!(
            status.file_status(Path::new("new.txt")),
            Some(GitIndicator::Staged)
        );
    }

    #[test]
    fn from_repo_modified_file_shows_modified() {
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
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        assert_eq!(
            status.file_status(Path::new("file.txt")),
            Some(GitIndicator::Modified)
        );
    }

    #[test]
    fn from_repo_nested_file_propagates_to_dir() {
        let (dir, repo) = setup_test_repo();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main(){}").unwrap();
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        assert_eq!(
            status.file_status(Path::new("src/main.rs")),
            Some(GitIndicator::Untracked)
        );
        assert_eq!(
            status.dir_status(Path::new("src")),
            Some(GitIndicator::HasChanges)
        );
    }

    #[test]
    fn from_repo_gitignored_file_shows_ignored() {
        let (dir, repo) = setup_test_repo();
        std::fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        std::fs::write(dir.path().join("debug.log"), "log data").unwrap();
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        assert_eq!(
            status.file_status(Path::new("debug.log")),
            Some(GitIndicator::Ignored)
        );
    }

    #[test]
    fn from_repo_ignored_does_not_propagate() {
        let (dir, repo) = setup_test_repo();
        // Use file-level ignore pattern (not directory) for predictable git2 behavior
        std::fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        std::fs::create_dir(dir.path().join("logs")).unwrap();
        std::fs::write(dir.path().join("logs/debug.log"), "log data").unwrap();
        let status = TreeGitStatus::from_repo(&repo).unwrap();
        // debug.log should be ignored
        assert_eq!(
            status.file_status(Path::new("logs/debug.log")),
            Some(GitIndicator::Ignored)
        );
        // logs directory should NOT have HasChanges (only ignored children)
        assert_eq!(status.dir_status(Path::new("logs")), None);
    }
}
