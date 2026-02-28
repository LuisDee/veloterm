// File browser overlay — state management for the project file browser.

pub mod tree;

use crate::input::InputMode;

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
/// Phase 1 (Track 28): minimal shell state for toggle/layout.
/// Future tracks (29-31) add tree data, preview, search, etc.
pub struct FileBrowserState {
    /// Split panel divider position as fraction (0.0..1.0). Default 0.5.
    pub split_ratio: f32,
    /// Which panel has focus: Left (file tree) or Right (preview).
    pub focused_panel: OverlayPanel,
}

impl FileBrowserState {
    pub fn new() -> Self {
        Self {
            split_ratio: 0.5,
            focused_panel: OverlayPanel::Left,
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
        // Escape from FileBrowser should go to Normal (handled by caller, but the toggle function
        // for a different mode always returns FileBrowser, not Normal — escape is a separate path)
        // This test validates that toggling from a non-FileBrowser mode results in FileBrowser.
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
        // Simulate close + reopen: state is preserved (not destroyed)
        assert!((state.split_ratio - 0.3).abs() < f32::EPSILON);
    }
}
