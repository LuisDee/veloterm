// Git review overlay — state management for the git change review panel.

pub mod status;

use crate::file_browser::OverlayPanel;
use crate::input::InputMode;

/// State for the Git Review overlay.
/// Phase 1 (Track 28): minimal shell state for toggle/layout.
/// Future tracks (32-34) add git status, diff rendering, staging, etc.
pub struct GitReviewState {
    /// Split panel divider position as fraction (0.0..1.0). Default 0.5.
    pub split_ratio: f32,
    /// Which panel has focus: Left (changed files) or Right (diff view).
    pub focused_panel: OverlayPanel,
}

impl GitReviewState {
    pub fn new() -> Self {
        Self {
            split_ratio: 0.5,
            focused_panel: OverlayPanel::Left,
        }
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

    #[test]
    fn git_review_state_defaults() {
        let state = GitReviewState::new();
        assert!((state.split_ratio - 0.5).abs() < f32::EPSILON);
        assert_eq!(state.focused_panel, OverlayPanel::Left);
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
}
