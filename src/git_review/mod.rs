// Git review overlay — state management for the git change review panel.

pub mod diff;
pub mod operations;
pub mod status;
pub mod view;

use std::path::{Path, PathBuf};

use crate::file_browser::OverlayPanel;
use crate::input::InputMode;

use self::status::{GitStatus, SectionState};
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
        }
    }

    /// Initialize by discovering the git repository from a working directory.
    pub fn open_from_cwd(&mut self, cwd: &Path) {
        match git2::Repository::discover(cwd) {
            Ok(repo) => {
                self.repo_path = repo.workdir().map(|p| p.to_path_buf());
                self.error = None;
                self.refresh_status_from_repo(&repo);
            }
            Err(_) => {
                self.repo_path = None;
                self.git_status = None;
                self.error = Some("Not in a git repository".to_string());
            }
        }
    }

    /// Refresh the git status from the stored repo path.
    pub fn refresh_status(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git2::Repository::open(&repo_path) {
            Ok(repo) => self.refresh_status_from_repo(&repo),
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
}
