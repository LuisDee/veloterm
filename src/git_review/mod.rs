// Git review overlay — state management for the git change review panel.

pub mod diff;
pub mod diff_view;
pub mod hunk_staging;
pub mod hunk_state;
pub mod inline_diff;
pub mod operations;
pub mod status;
pub mod syntax_highlight;
pub mod view;

use std::path::{Path, PathBuf};

use crate::file_browser::OverlayPanel;
use crate::input::InputMode;

use self::diff::FileDiff;
use self::diff_view::DiffScrollState;
use self::status::{FileStatus, GitStatus, SectionState};
use self::syntax_highlight::DiffHighlighter;
use self::view::Section;

/// State for the Git Review overlay.
pub struct GitReviewState {
    /// Split panel divider position as fraction (0.0..1.0). Default 0.5.
    pub split_ratio: f32,
    /// Which panel has focus: Left (changed files) or Right (diff view).
    pub focused_panel: OverlayPanel,
    /// Current git status (None if not yet loaded or not in a repo).
    pub git_status: Option<GitStatus>,
    /// Section collapse state.
    pub section_state: SectionState,
    /// Currently selected file entry.
    pub selected: Option<(Section, usize)>,
    /// Commit message text.
    pub commit_message: String,
    /// Repository root path (None if not in a repo).
    pub repo_path: Option<PathBuf>,
    /// Error message to display (e.g. "Not in a git repository").
    pub error: Option<String>,
    /// Whether a discard confirmation is pending, and for which path.
    pub discard_confirm: Option<PathBuf>,
    /// Cached diff for the currently selected file.
    pub cached_diff: Option<CachedDiff>,
    /// Scroll state for the diff view.
    pub diff_scroll: DiffScrollState,
    /// Current hunk index in the diff view (for hunk-level staging).
    pub current_hunk_index: usize,
    /// Current branch name (e.g. "main", "feature/foo").
    pub branch_name: Option<String>,
    /// Cached syntax highlighter (syntect loading is expensive).
    pub highlighter: DiffHighlighter,
}

/// A cached diff tied to a specific file path and section.
#[derive(Debug, Clone)]
pub struct CachedDiff {
    /// The path this diff was computed for.
    pub path: PathBuf,
    /// Which section the file was in when the diff was computed.
    pub section: Section,
    /// The computed diff result.
    pub diff: FileDiff,
}

impl GitReviewState {
    pub fn new() -> Self {
        Self {
            split_ratio: 0.5,
            focused_panel: OverlayPanel::Left,
            git_status: None,
            section_state: SectionState::default(),
            selected: None,
            commit_message: String::new(),
            repo_path: None,
            error: None,
            discard_confirm: None,
            cached_diff: None,
            diff_scroll: DiffScrollState::new(),
            current_hunk_index: 0,
            branch_name: None,
            highlighter: DiffHighlighter::new(),
        }
    }

    /// Initialize by discovering the git repository from a working directory.
    pub fn open_from_cwd(&mut self, cwd: &Path) {
        match git2::Repository::discover(cwd) {
            Ok(repo) => {
                self.repo_path = repo.workdir().map(|p| p.to_path_buf());
                self.error = None;
                self.branch_name = Self::detect_branch(&repo);
                self.refresh_status_from_repo(&repo);
            }
            Err(_) => {
                self.repo_path = None;
                self.git_status = None;
                self.branch_name = None;
                self.error = Some("Not in a git repository".to_string());
            }
        }
    }

    /// Detect the current branch name from a repository.
    fn detect_branch(repo: &git2::Repository) -> Option<String> {
        if let Ok(head) = repo.head() {
            if head.is_branch() {
                return head.shorthand().map(|s| s.to_string());
            }
            // Detached HEAD — show short hash
            if let Ok(commit) = head.peel_to_commit() {
                let id = commit.id();
                return Some(format!("{:.7}", id));
            }
        }
        None
    }

    /// Reset cached state while preserving layout settings.
    /// Clears: cached_diff, commit_message, selected, error, discard_confirm, diff_scroll
    /// Preserves: split_ratio, focused_panel, repo_path
    pub fn reset(&mut self) {
        self.cached_diff = None;
        self.commit_message.clear();
        self.selected = None;
        self.error = None;
        self.discard_confirm = None;
        self.diff_scroll = DiffScrollState::new();
        self.current_hunk_index = 0;
    }

    /// Refresh the git status from the stored repo path.
    pub fn refresh_status(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git2::Repository::open(&repo_path) {
            Ok(repo) => {
                self.branch_name = Self::detect_branch(&repo);
                self.refresh_status_from_repo(&repo);
            }
            Err(e) => self.error = Some(format!("Failed to open repo: {}", e)),
        }
    }

    fn refresh_status_from_repo(&mut self, repo: &git2::Repository) {
        match GitStatus::from_repo(repo) {
            Ok(status) => {
                self.git_status = Some(status);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Failed to read git status: {}", e));
            }
        }
    }

    /// Stage a file and refresh status.
    pub fn stage_file(&mut self, path: &Path) {
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = operations::stage_file(&repo, path) {
                    self.error = Some(format!("Stage failed: {}", e));
                    return;
                }
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Unstage a file and refresh status.
    pub fn unstage_file(&mut self, path: &Path) {
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = operations::unstage_file(&repo, path) {
                    self.error = Some(format!("Unstage failed: {}", e));
                    return;
                }
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Discard changes to a file (requires confirmation first).
    pub fn discard_file(&mut self, path: &Path) {
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = operations::discard_file(&repo, path) {
                    self.error = Some(format!("Discard failed: {}", e));
                    return;
                }
                self.discard_confirm = None;
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Stage all unstaged + untracked files.
    pub fn stage_all(&mut self) {
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = operations::stage_all(&repo) {
                    self.error = Some(format!("Stage all failed: {}", e));
                    return;
                }
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Unstage all staged files.
    pub fn unstage_all(&mut self) {
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = operations::unstage_all(&repo) {
                    self.error = Some(format!("Unstage all failed: {}", e));
                    return;
                }
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Commit staged changes with the current commit message.
    /// Returns true on success, false on failure.
    pub fn commit(&mut self) -> bool {
        let message = self.commit_message.trim().to_string();
        if message.is_empty() {
            self.error = Some("Commit message cannot be empty".to_string());
            return false;
        }
        let staged_count = self
            .git_status
            .as_ref()
            .map(|s| s.staged.len())
            .unwrap_or(0);
        if staged_count == 0 {
            self.error = Some("No staged changes to commit".to_string());
            return false;
        }

        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                match operations::commit(&repo, &message) {
                    Ok(_) => {
                        self.commit_message.clear();
                        self.error = None;
                        self.invalidate_diff_cache();
                        self.refresh_status_from_repo(&repo);
                        return true;
                    }
                    Err(e) => {
                        self.error = Some(format!("Commit failed: {}", e));
                    }
                }
            }
        }
        false
    }

    /// Whether the commit button should be enabled.
    pub fn can_commit(&self) -> bool {
        let staged_count = self
            .git_status
            .as_ref()
            .map(|s| s.staged.len())
            .unwrap_or(0);
        view::commit_button_enabled(staged_count, &self.commit_message)
    }

    /// Navigate selection up or down.
    pub fn navigate(&mut self, direction: i32) {
        if let Some(status) = &self.git_status {
            self.selected =
                view::navigate_selection(status, &self.section_state, self.selected, direction);
        }
    }

    /// Compute and cache the diff for the currently selected file.
    /// Returns a reference to the cached diff, or None if no file is selected.
    pub fn compute_selected_diff(&mut self) -> Option<&FileDiff> {
        let (section, index) = self.selected?;
        let status = self.git_status.as_ref()?;
        let entry = match section {
            Section::Staged => status.staged.get(index),
            Section::Changed => status.changed.get(index),
            Section::Untracked => status.untracked.get(index),
        }?;

        let path = entry.path.clone();
        let file_status = entry.status.clone();

        // Check if cache is still valid
        if let Some(cached) = &self.cached_diff {
            if cached.path == path && cached.section == section {
                return self.cached_diff.as_ref().map(|c| &c.diff);
            }
        }

        // Compute new diff
        let repo_path = self.repo_path.as_ref()?;
        let repo = git2::Repository::open(repo_path).ok()?;

        let staged = section == Section::Staged;
        let untracked = file_status == FileStatus::Untracked;

        match diff::compute_diff(&repo, &path, staged, untracked) {
            Ok(file_diff) => {
                self.cached_diff = Some(CachedDiff {
                    path,
                    section,
                    diff: file_diff,
                });
                self.diff_scroll.reset();
                self.current_hunk_index = 0;
                self.cached_diff.as_ref().map(|c| &c.diff)
            }
            Err(e) => {
                self.error = Some(format!("Diff failed: {}", e));
                self.cached_diff = None;
                None
            }
        }
    }

    /// Get the cached diff if available and still valid for the current selection.
    pub fn current_diff(&self) -> Option<&FileDiff> {
        let (section, index) = self.selected?;
        let status = self.git_status.as_ref()?;
        let entry = match section {
            Section::Staged => status.staged.get(index),
            Section::Changed => status.changed.get(index),
            Section::Untracked => status.untracked.get(index),
        }?;

        let cached = self.cached_diff.as_ref()?;
        if cached.path == entry.path && cached.section == section {
            Some(&cached.diff)
        } else {
            None
        }
    }

    /// Invalidate the diff cache (e.g. after staging/unstaging).
    pub fn invalidate_diff_cache(&mut self) {
        self.cached_diff = None;
    }

    /// Navigate to the first entry.
    pub fn navigate_home(&mut self) {
        if let Some(status) = &self.git_status {
            // Find the first entry in any non-empty section
            if !status.staged.is_empty() {
                self.selected = Some((Section::Staged, 0));
            } else if !status.changed.is_empty() {
                self.selected = Some((Section::Changed, 0));
            } else if !status.untracked.is_empty() {
                self.selected = Some((Section::Untracked, 0));
            }
        }
    }

    /// Navigate to the last entry.
    pub fn navigate_end(&mut self) {
        if let Some(status) = &self.git_status {
            if !status.untracked.is_empty() {
                self.selected = Some((Section::Untracked, status.untracked.len() - 1));
            } else if !status.changed.is_empty() {
                self.selected = Some((Section::Changed, status.changed.len() - 1));
            } else if !status.staged.is_empty() {
                self.selected = Some((Section::Staged, status.staged.len() - 1));
            }
        }
    }

    /// Stage the hunk at `current_hunk_index` for the selected file.
    /// Only works when a Changed/Untracked file is selected.
    pub fn stage_current_hunk(&mut self) {
        let (section, index) = match self.selected {
            Some(s) => s,
            None => return,
        };
        // Hunk staging only makes sense for unstaged (Changed) files
        if section != Section::Changed {
            return;
        }
        let path = {
            let status = match &self.git_status {
                Some(s) => s,
                None => return,
            };
            match status.changed.get(index) {
                Some(e) => e.path.clone(),
                None => return,
            }
        };
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = hunk_staging::stage_hunk(&repo, &path, self.current_hunk_index) {
                    self.error = Some(format!("Stage hunk failed: {}", e));
                    return;
                }
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Unstage the hunk at `current_hunk_index` for the selected file.
    /// Only works when a Staged file is selected.
    pub fn unstage_current_hunk(&mut self) {
        let (section, index) = match self.selected {
            Some(s) => s,
            None => return,
        };
        if section != Section::Staged {
            return;
        }
        let path = {
            let status = match &self.git_status {
                Some(s) => s,
                None => return,
            };
            match status.staged.get(index) {
                Some(e) => e.path.clone(),
                None => return,
            }
        };
        if let Some(repo_path) = &self.repo_path {
            if let Ok(repo) = git2::Repository::open(repo_path) {
                if let Err(e) = hunk_staging::unstage_hunk(&repo, &path, self.current_hunk_index) {
                    self.error = Some(format!("Unstage hunk failed: {}", e));
                    return;
                }
                self.invalidate_diff_cache();
                self.refresh_status_from_repo(&repo);
            }
        }
    }

    /// Navigate to the next hunk in the current diff.
    /// Returns true if the hunk index changed.
    pub fn next_hunk(&mut self) -> bool {
        let hunk_count = self.current_diff().map(|d| d.hunks.len()).unwrap_or(0);
        if hunk_count == 0 {
            return false;
        }
        if self.current_hunk_index + 1 < hunk_count {
            self.current_hunk_index += 1;
            true
        } else {
            false
        }
    }

    /// Navigate to the previous hunk in the current diff.
    /// Returns true if the hunk index changed.
    pub fn prev_hunk(&mut self) -> bool {
        if self.current_hunk_index > 0 {
            self.current_hunk_index -= 1;
            true
        } else {
            false
        }
    }

    /// Get the total number of hunks in the current diff.
    pub fn hunk_count(&self) -> usize {
        self.current_diff().map(|d| d.hunks.len()).unwrap_or(0)
    }

    /// Get the path of the currently selected file, if any.
    pub fn selected_path(&self) -> Option<&Path> {
        let (section, index) = self.selected?;
        let status = self.git_status.as_ref()?;
        let entries = match section {
            Section::Staged => &status.staged,
            Section::Changed => &status.changed,
            Section::Untracked => &status.untracked,
        };
        entries.get(index).map(|e| e.path.as_path())
    }
}

/// Compute the next InputMode when toggling the git review.
pub fn toggle_git_review(current: InputMode) -> InputMode {
    if current == InputMode::GitReview {
        InputMode::Normal
    } else {
        InputMode::GitReview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn git_review_state_defaults() {
        let state = GitReviewState::new();
        assert!((state.split_ratio - 0.5).abs() < f32::EPSILON);
        assert_eq!(state.focused_panel, OverlayPanel::Left);
        assert!(state.git_status.is_none());
        assert!(state.commit_message.is_empty());
        assert!(state.repo_path.is_none());
        assert!(state.error.is_none());
        assert!(state.selected.is_none());
        assert!(state.discard_confirm.is_none());
        assert!(state.cached_diff.is_none());
    }

    #[test]
    fn overlay_toggle_from_normal_to_git_review() {
        assert_eq!(toggle_git_review(InputMode::Normal), InputMode::GitReview);
    }

    #[test]
    fn overlay_toggle_from_git_review_to_normal() {
        assert_eq!(toggle_git_review(InputMode::GitReview), InputMode::Normal);
    }

    #[test]
    fn overlay_switch_from_file_browser_to_git_review() {
        assert_eq!(toggle_git_review(InputMode::FileBrowser), InputMode::GitReview);
    }

    fn setup_test_repo() -> (tempfile::TempDir, git2::Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    fn make_initial_commit(dir: &Path, repo: &git2::Repository) {
        std::fs::write(dir.join("initial.txt"), "initial").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("initial.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    #[test]
    fn open_from_cwd_in_repo() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());
        assert!(state.repo_path.is_some());
        assert!(state.error.is_none());
        assert!(state.git_status.is_some());
    }

    #[test]
    fn open_from_cwd_not_in_repo() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());
        assert!(state.repo_path.is_none());
        assert!(state.error.is_some());
        assert_eq!(state.error.as_deref(), Some("Not in a git repository"));
    }

    #[test]
    fn can_commit_requires_staged_and_message() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        // No staged, no message
        assert!(!state.can_commit());

        // Add message but no staged
        state.commit_message = "test".to_string();
        assert!(!state.can_commit());

        // Stage a file
        std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
        state.stage_file(Path::new("new.txt"));
        assert!(state.can_commit());

        // Clear message
        state.commit_message.clear();
        assert!(!state.can_commit());
    }

    #[test]
    fn commit_clears_message_and_staged() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("file.txt"), "content").unwrap();
        state.stage_file(Path::new("file.txt"));
        state.commit_message = "test commit".to_string();

        let success = state.commit();
        assert!(success);
        assert!(state.commit_message.is_empty());
        assert!(state.git_status.as_ref().unwrap().staged.is_empty());
    }

    #[test]
    fn stage_and_unstage_workflow() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        // Modify file
        std::fs::write(dir.path().join("initial.txt"), "changed").unwrap();
        state.refresh_status();
        assert_eq!(state.git_status.as_ref().unwrap().changed.len(), 1);

        // Stage
        state.stage_file(Path::new("initial.txt"));
        assert_eq!(state.git_status.as_ref().unwrap().staged.len(), 1);
        assert!(state.git_status.as_ref().unwrap().changed.is_empty());

        // Unstage
        state.unstage_file(Path::new("initial.txt"));
        assert!(state.git_status.as_ref().unwrap().staged.is_empty());
        assert_eq!(state.git_status.as_ref().unwrap().changed.len(), 1);
    }

    #[test]
    fn discard_restores_file() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "changed").unwrap();
        state.refresh_status();
        assert_eq!(state.git_status.as_ref().unwrap().changed.len(), 1);

        state.discard_file(Path::new("initial.txt"));
        assert!(state.git_status.as_ref().unwrap().changed.is_empty());
        let content = std::fs::read_to_string(dir.path().join("initial.txt")).unwrap();
        assert_eq!(content, "initial");
    }

    #[test]
    fn stage_all_and_unstage_all() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "changed").unwrap();
        std::fs::write(dir.path().join("new.txt"), "new").unwrap();
        state.refresh_status();

        state.stage_all();
        let status = state.git_status.as_ref().unwrap();
        assert_eq!(status.staged.len(), 2);
        assert!(status.changed.is_empty());
        assert!(status.untracked.is_empty());

        state.unstage_all();
        let status = state.git_status.as_ref().unwrap();
        assert!(status.staged.is_empty());
    }

    #[test]
    fn navigate_selects_entries() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        state.refresh_status();

        assert!(state.selected.is_none());
        state.navigate(1); // down
        assert_eq!(state.selected, Some((Section::Untracked, 0)));
        state.navigate(1); // down
        assert_eq!(state.selected, Some((Section::Untracked, 1)));
        state.navigate(-1); // up
        assert_eq!(state.selected, Some((Section::Untracked, 0)));
    }

    #[test]
    fn selected_path_returns_correct_file() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        state.refresh_status();
        state.navigate(1);

        let path = state.selected_path();
        assert_eq!(path, Some(Path::new("test.txt")));
    }

    #[test]
    fn commit_fails_with_empty_message() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("file.txt"), "content").unwrap();
        state.stage_file(Path::new("file.txt"));

        let success = state.commit();
        assert!(!success);
        assert!(state.error.is_some());
    }

    #[test]
    fn commit_fails_with_no_staged() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());
        state.commit_message = "test".to_string();

        let success = state.commit();
        assert!(!success);
        assert!(state.error.is_some());
    }

    #[test]
    fn discard_confirm_flow() {
        let mut state = GitReviewState::new();
        assert!(state.discard_confirm.is_none());

        state.discard_confirm = Some(PathBuf::from("test.rs"));
        assert_eq!(
            state.discard_confirm.as_deref(),
            Some(Path::new("test.rs"))
        );

        // Cancel
        state.discard_confirm = None;
        assert!(state.discard_confirm.is_none());
    }

    // -- Diff integration tests --

    #[test]
    fn compute_diff_for_untracked_file() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("new.txt"), "hello\nworld").unwrap();
        state.refresh_status();

        // Select the untracked file
        state.navigate(1);
        assert_eq!(state.selected, Some((Section::Untracked, 0)));

        // Compute diff
        let diff = state.compute_selected_diff();
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert_eq!(diff.diff_type, diff::DiffType::Added);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn compute_diff_for_modified_file() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "changed content").unwrap();
        state.refresh_status();

        state.navigate(1);
        assert_eq!(state.selected, Some((Section::Changed, 0)));

        let diff = state.compute_selected_diff();
        assert!(diff.is_some());
        assert_eq!(diff.unwrap().diff_type, diff::DiffType::Modified);
    }

    #[test]
    fn compute_diff_for_staged_file() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "staged change").unwrap();
        state.stage_file(Path::new("initial.txt"));

        state.navigate(1);
        assert_eq!(state.selected, Some((Section::Staged, 0)));

        let diff = state.compute_selected_diff();
        assert!(diff.is_some());
        assert_eq!(diff.unwrap().diff_type, diff::DiffType::Modified);
    }

    #[test]
    fn diff_cache_hit() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("file.txt"), "content").unwrap();
        state.refresh_status();
        state.navigate(1);

        // First compute
        state.compute_selected_diff();
        assert!(state.cached_diff.is_some());

        // Second call should use cache (same path and section)
        let diff = state.current_diff();
        assert!(diff.is_some());
    }

    #[test]
    fn diff_cache_invalidated_on_stage() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        state.refresh_status();
        state.navigate(1);
        state.compute_selected_diff();
        assert!(state.cached_diff.is_some());

        // Stage invalidates cache
        state.stage_file(Path::new("initial.txt"));
        assert!(state.cached_diff.is_none());
    }

    #[test]
    fn diff_cache_invalidated_on_unstage() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        state.stage_file(Path::new("initial.txt"));
        state.navigate(1);
        state.compute_selected_diff();
        assert!(state.cached_diff.is_some());

        state.unstage_file(Path::new("initial.txt"));
        assert!(state.cached_diff.is_none());
    }

    #[test]
    fn diff_cache_invalidated_on_discard() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        state.refresh_status();
        state.navigate(1);
        state.compute_selected_diff();
        assert!(state.cached_diff.is_some());

        state.discard_file(Path::new("initial.txt"));
        assert!(state.cached_diff.is_none());
    }

    #[test]
    fn diff_cache_invalidated_on_commit() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        // Modify and stage a tracked file
        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        state.stage_file(Path::new("initial.txt"));

        // Select the staged file and compute its diff
        state.navigate(1);
        assert_eq!(state.selected, Some((Section::Staged, 0)));
        state.compute_selected_diff();
        assert!(state.cached_diff.is_some());

        // Commit should invalidate cache
        state.commit_message = "test commit".to_string();
        state.commit();
        assert!(state.cached_diff.is_none());
    }

    #[test]
    fn diff_cache_miss_on_selection_change() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(dir.path().join("b.txt"), "bbb").unwrap();
        state.refresh_status();

        // Select first file, compute diff
        state.navigate(1);
        state.compute_selected_diff();
        assert!(state.cached_diff.is_some());
        assert_eq!(
            state.cached_diff.as_ref().unwrap().path,
            PathBuf::from("a.txt")
        );

        // Select second file — cache should miss
        state.navigate(1);
        let diff = state.current_diff();
        assert!(diff.is_none()); // Cache miss

        // Compute for new file
        state.compute_selected_diff();
        assert_eq!(
            state.cached_diff.as_ref().unwrap().path,
            PathBuf::from("b.txt")
        );
    }

    #[test]
    fn current_diff_none_when_no_selection() {
        let state = GitReviewState::new();
        assert!(state.current_diff().is_none());
    }

    // -- Empty repo handling --

    #[test]
    fn empty_repo_shows_message_not_error() {
        // A git init repo with no commits should not produce an error
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());
        assert!(state.repo_path.is_some());
        assert!(state.error.is_none());
        let status = state.git_status.as_ref().unwrap();
        assert!(status.is_empty());
    }

    // -- reset() tests --

    #[test]
    fn reset_clears_cached_data() {
        let mut state = GitReviewState::new();
        state.cached_diff = Some(CachedDiff {
            path: PathBuf::from("test.rs"),
            section: Section::Changed,
            diff: diff::FileDiff {
                path: "test.rs".to_string(),
                hunks: vec![],
                diff_type: diff::DiffType::Modified,
            },
        });
        state.commit_message = "wip".to_string();
        state.selected = Some((Section::Changed, 0));
        state.error = Some("something".to_string());

        state.reset();

        assert!(state.cached_diff.is_none());
        assert!(state.commit_message.is_empty());
        assert!(state.selected.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn reset_preserves_layout() {
        let mut state = GitReviewState::new();
        state.split_ratio = 0.3;
        state.focused_panel = OverlayPanel::Right;
        state.repo_path = Some(PathBuf::from("/tmp/repo"));

        state.reset();

        assert!((state.split_ratio - 0.3).abs() < f32::EPSILON);
        assert_eq!(state.focused_panel, OverlayPanel::Right);
        assert!(state.repo_path.is_some());
    }

    // -- navigate_home / navigate_end --

    #[test]
    fn navigate_home_selects_first() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("z.txt"), "z").unwrap();
        state.refresh_status();

        // Navigate to some entry first
        state.navigate(1);
        state.navigate(1);

        state.navigate_home();
        assert_eq!(state.selected, Some((Section::Untracked, 0)));
    }

    #[test]
    fn navigate_end_selects_last() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("z.txt"), "z").unwrap();
        state.refresh_status();

        state.navigate_end();
        assert_eq!(state.selected, Some((Section::Untracked, 1)));
    }

    #[test]
    fn diff_scroll_resets_on_new_diff() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("file.txt"), "line1\nline2\nline3").unwrap();
        state.refresh_status();
        state.navigate(1);

        // Scroll down, then compute diff
        state.diff_scroll.scroll_vertical(100.0);
        state.compute_selected_diff();
        // After computing a new diff, scroll should reset
        assert!((state.diff_scroll.vertical_offset - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn invalidate_diff_cache_explicit() {
        let mut state = GitReviewState::new();
        state.cached_diff = Some(CachedDiff {
            path: PathBuf::from("test.rs"),
            section: Section::Changed,
            diff: diff::FileDiff {
                path: "test.rs".to_string(),
                hunks: vec![],
                diff_type: diff::DiffType::Modified,
            },
        });
        assert!(state.cached_diff.is_some());
        state.invalidate_diff_cache();
        assert!(state.cached_diff.is_none());
    }

    // -- current_hunk_index / hunk navigation --

    #[test]
    fn hunk_index_defaults_to_zero() {
        let state = GitReviewState::new();
        assert_eq!(state.current_hunk_index, 0);
    }

    #[test]
    fn hunk_index_resets_on_new_diff() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("file.txt"), "line1\nline2").unwrap();
        state.refresh_status();
        state.navigate(1);

        state.current_hunk_index = 5;
        state.compute_selected_diff();
        assert_eq!(state.current_hunk_index, 0);
    }

    #[test]
    fn hunk_index_resets_on_reset() {
        let mut state = GitReviewState::new();
        state.current_hunk_index = 3;
        state.reset();
        assert_eq!(state.current_hunk_index, 0);
    }

    #[test]
    fn hunk_count_zero_when_no_diff() {
        let state = GitReviewState::new();
        assert_eq!(state.hunk_count(), 0);
    }

    #[test]
    fn next_hunk_returns_false_when_no_diff() {
        let mut state = GitReviewState::new();
        assert!(!state.next_hunk());
    }

    #[test]
    fn prev_hunk_returns_false_at_zero() {
        let mut state = GitReviewState::new();
        assert!(!state.prev_hunk());
    }

    #[test]
    fn hunk_navigation_with_diff() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        // Create a file with content and commit it
        std::fs::write(dir.path().join("initial.txt"), "changed content\nmore changes").unwrap();
        state.refresh_status();

        state.navigate(1); // select the changed file
        state.compute_selected_diff();

        let count = state.hunk_count();
        assert!(count > 0);
        assert_eq!(state.current_hunk_index, 0);

        // If there's only 1 hunk, next should not advance
        if count == 1 {
            assert!(!state.next_hunk());
            assert_eq!(state.current_hunk_index, 0);
        }
    }

    // -- click-to-diff tests --

    #[test]
    fn click_staged_file_loads_diff() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        // Stage a modification
        std::fs::write(dir.path().join("initial.txt"), "staged change").unwrap();
        state.stage_file(Path::new("initial.txt"));

        // Select the staged file
        state.selected = Some((Section::Staged, 0));
        let diff = state.compute_selected_diff();
        assert!(diff.is_some());
        assert_eq!(diff.unwrap().diff_type, diff::DiffType::Modified);
    }

    #[test]
    fn click_changed_file_loads_diff() {
        let (dir, repo) = setup_test_repo();
        make_initial_commit(dir.path(), &repo);

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("initial.txt"), "modified").unwrap();
        state.refresh_status();

        state.selected = Some((Section::Changed, 0));
        let diff = state.compute_selected_diff();
        assert!(diff.is_some());
        assert_eq!(diff.unwrap().diff_type, diff::DiffType::Modified);
    }

    #[test]
    fn click_different_file_updates_diff() {
        let (dir, _repo) = setup_test_repo();
        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        std::fs::write(dir.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(dir.path().join("b.txt"), "bbb").unwrap();
        state.refresh_status();

        // Select first file
        state.selected = Some((Section::Untracked, 0));
        state.compute_selected_diff();
        assert_eq!(
            state.cached_diff.as_ref().unwrap().path,
            PathBuf::from("a.txt")
        );

        // Select second file — should load different diff
        state.selected = Some((Section::Untracked, 1));
        state.compute_selected_diff();
        assert_eq!(
            state.cached_diff.as_ref().unwrap().path,
            PathBuf::from("b.txt")
        );
    }

    // -- hunk staging integration tests --

    #[test]
    fn stage_current_hunk_stages_from_changed() {
        let (dir, repo) = setup_test_repo();
        // Create a file with many lines
        let mut content = String::new();
        for i in 1..=30 {
            content.push_str(&format!("line{}\n", i));
        }
        let mut index = repo.index().unwrap();
        std::fs::write(dir.path().join("file.txt"), &content).unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Modify line 3 and line 28 (far apart = separate hunks)
        let mut modified = String::new();
        for i in 1..=30 {
            if i == 3 {
                modified.push_str("CHANGED_3\n");
            } else if i == 28 {
                modified.push_str("CHANGED_28\n");
            } else {
                modified.push_str(&format!("line{}\n", i));
            }
        }
        std::fs::write(dir.path().join("file.txt"), &modified).unwrap();

        let mut state = GitReviewState::new();
        state.open_from_cwd(dir.path());

        // Select the changed file
        state.selected = Some((Section::Changed, 0));
        state.current_hunk_index = 0;

        // Stage hunk 0
        state.stage_current_hunk();

        // Should now have staged and unstaged entries
        let status = state.git_status.as_ref().unwrap();
        assert!(!status.staged.is_empty());
    }

    #[test]
    fn stage_hunk_noop_for_staged_section() {
        let mut state = GitReviewState::new();
        state.selected = Some((Section::Staged, 0));
        // Should not panic or error — just a no-op
        state.stage_current_hunk();
        assert!(state.error.is_none());
    }

    #[test]
    fn unstage_hunk_noop_for_changed_section() {
        let mut state = GitReviewState::new();
        state.selected = Some((Section::Changed, 0));
        // Should not panic or error — just a no-op
        state.unstage_current_hunk();
        assert!(state.error.is_none());
    }
}
