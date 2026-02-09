// Command palette: fuzzy-search access to all VeloTerm actions.

/// An action that can be dispatched from the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    // Pane actions
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    FocusNextPane,
    FocusPrevPane,
    // Tab actions
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    // Font actions
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    // Edit actions
    Copy,
    Paste,
    SelectAll,
    // Terminal actions
    ClearScrollback,
    // Search
    OpenSearch,
    // Vi-mode
    ToggleViMode,
    // Window
    NewWindow,
}

/// A single entry in the command palette.
#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub keybinding: &'static str,
    pub action: PaletteAction,
}

/// Return all available command palette entries.
pub fn command_registry() -> Vec<PaletteEntry> {
    vec![
        PaletteEntry {
            name: "Split Pane Right",
            description: "Split the current pane vertically",
            keybinding: "Cmd+D",
            action: PaletteAction::SplitVertical,
        },
        PaletteEntry {
            name: "Split Pane Down",
            description: "Split the current pane horizontally",
            keybinding: "Cmd+Shift+D",
            action: PaletteAction::SplitHorizontal,
        },
        PaletteEntry {
            name: "Close Pane",
            description: "Close the focused pane",
            keybinding: "Cmd+W",
            action: PaletteAction::ClosePane,
        },
        PaletteEntry {
            name: "Focus Next Pane",
            description: "Move focus to the next pane",
            keybinding: "Cmd+]",
            action: PaletteAction::FocusNextPane,
        },
        PaletteEntry {
            name: "Focus Previous Pane",
            description: "Move focus to the previous pane",
            keybinding: "Cmd+[",
            action: PaletteAction::FocusPrevPane,
        },
        PaletteEntry {
            name: "New Tab",
            description: "Open a new terminal tab",
            keybinding: "Cmd+T",
            action: PaletteAction::NewTab,
        },
        PaletteEntry {
            name: "Close Tab",
            description: "Close the current tab",
            keybinding: "Cmd+W",
            action: PaletteAction::CloseTab,
        },
        PaletteEntry {
            name: "Next Tab",
            description: "Switch to the next tab",
            keybinding: "Ctrl+Tab",
            action: PaletteAction::NextTab,
        },
        PaletteEntry {
            name: "Previous Tab",
            description: "Switch to the previous tab",
            keybinding: "Ctrl+Shift+Tab",
            action: PaletteAction::PrevTab,
        },
        PaletteEntry {
            name: "Increase Font Size",
            description: "Make terminal text larger",
            keybinding: "Cmd++",
            action: PaletteAction::IncreaseFontSize,
        },
        PaletteEntry {
            name: "Decrease Font Size",
            description: "Make terminal text smaller",
            keybinding: "Cmd+-",
            action: PaletteAction::DecreaseFontSize,
        },
        PaletteEntry {
            name: "Reset Font Size",
            description: "Reset terminal text to default size",
            keybinding: "Cmd+0",
            action: PaletteAction::ResetFontSize,
        },
        PaletteEntry {
            name: "Copy",
            description: "Copy selected text to clipboard",
            keybinding: "Cmd+C",
            action: PaletteAction::Copy,
        },
        PaletteEntry {
            name: "Paste",
            description: "Paste from clipboard",
            keybinding: "Cmd+V",
            action: PaletteAction::Paste,
        },
        PaletteEntry {
            name: "Select All",
            description: "Select all terminal content",
            keybinding: "Cmd+A",
            action: PaletteAction::SelectAll,
        },
        PaletteEntry {
            name: "Clear Scrollback",
            description: "Clear terminal scrollback history",
            keybinding: "Cmd+K",
            action: PaletteAction::ClearScrollback,
        },
        PaletteEntry {
            name: "Find",
            description: "Open search bar",
            keybinding: "Ctrl+Shift+F",
            action: PaletteAction::OpenSearch,
        },
        PaletteEntry {
            name: "Toggle Vi Mode",
            description: "Enter or exit vi-mode navigation",
            keybinding: "Ctrl+Shift+Space",
            action: PaletteAction::ToggleViMode,
        },
        PaletteEntry {
            name: "New Window",
            description: "Open a new VeloTerm window",
            keybinding: "Cmd+N",
            action: PaletteAction::NewWindow,
        },
    ]
}

/// Fuzzy match a query against a target string.
/// Returns Some(score) if the query is a subsequence of the target, None otherwise.
/// Higher score = better match.
pub fn fuzzy_match(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    let mut qi = 0;
    let mut score: i32 = 0;
    let mut prev_match = false;
    let mut consecutive = 0;

    for (ti, &tc) in target_lower.iter().enumerate() {
        if qi < query_lower.len() && tc == query_lower[qi] {
            qi += 1;
            // Bonus for consecutive matches
            if prev_match {
                consecutive += 1;
                score += 5 * consecutive;
            } else {
                consecutive = 0;
                score += 1;
            }
            // Bonus for matching at word boundaries
            if ti == 0 || target.as_bytes().get(ti.wrapping_sub(1)) == Some(&b' ') {
                score += 10;
            }
            prev_match = true;
        } else {
            prev_match = false;
            consecutive = 0;
        }
    }

    if qi == query_lower.len() {
        Some(score)
    } else {
        None
    }
}

/// Command palette state.
#[derive(Debug, Clone)]
pub struct PaletteState {
    pub query: String,
    pub filtered: Vec<(usize, i32)>, // (index into registry, score)
    pub selected: usize,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self::new()
    }
}

impl PaletteState {
    pub fn new() -> Self {
        let registry = command_registry();
        let filtered: Vec<(usize, i32)> = (0..registry.len()).map(|i| (i, 0)).collect();
        Self {
            query: String::new(),
            filtered,
            selected: 0,
        }
    }

    /// Update the filtered results based on the current query.
    pub fn update_filter(&mut self) {
        let registry = command_registry();
        let mut scored: Vec<(usize, i32)> = registry
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                // Match against name and description
                let name_score = fuzzy_match(&self.query, entry.name);
                let desc_score = fuzzy_match(&self.query, entry.description);
                let best = match (name_score, desc_score) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };
                best.map(|score| (i, score))
            })
            .collect();

        // Sort by score descending, then by name alphabetically
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| {
            registry[a.0].name.cmp(registry[b.0].name)
        }));

        self.filtered = scored;
        self.selected = 0;
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected < self.filtered.len() - 1 {
            self.selected += 1;
        }
    }

    /// Get the currently selected palette entry's action.
    pub fn selected_action(&self) -> Option<PaletteAction> {
        let registry = command_registry();
        self.filtered
            .get(self.selected)
            .map(|&(idx, _)| registry[idx].action)
    }

    /// Get the selected entry details.
    pub fn selected_entry(&self) -> Option<PaletteEntry> {
        let registry = command_registry();
        self.filtered
            .get(self.selected)
            .map(|&(idx, _)| registry[idx].clone())
    }

    /// Type a character into the query.
    pub fn type_char(&mut self, ch: char) {
        self.query.push(ch);
        self.update_filter();
    }

    /// Delete the last character from the query.
    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    /// Get the number of visible results.
    pub fn result_count(&self) -> usize {
        self.filtered.len()
    }
}

/// Check if the key combination should open the command palette.
pub fn should_open_palette(
    logical_key: &winit::keyboard::Key,
    modifiers: winit::keyboard::ModifiersState,
) -> bool {
    #[cfg(target_os = "macos")]
    let trigger = modifiers.super_key() && modifiers.shift_key();
    #[cfg(not(target_os = "macos"))]
    let trigger = modifiers.control_key() && modifiers.shift_key();

    if !trigger {
        return false;
    }

    matches!(
        logical_key,
        winit::keyboard::Key::Character(ref s) if s.as_str() == "p" || s.as_str() == "P"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fuzzy match tests ───────────────────────────────────────

    #[test]
    fn fuzzy_match_empty_query_matches_everything() {
        assert!(fuzzy_match("", "anything").is_some());
    }

    #[test]
    fn fuzzy_match_exact_match() {
        let score = fuzzy_match("Split Pane", "Split Pane");
        assert!(score.is_some());
        assert!(score.unwrap() > 0);
    }

    #[test]
    fn fuzzy_match_case_insensitive() {
        let score = fuzzy_match("split", "Split Pane");
        assert!(score.is_some());
    }

    #[test]
    fn fuzzy_match_subsequence() {
        let score = fuzzy_match("sp", "Split Pane");
        assert!(score.is_some());
    }

    #[test]
    fn fuzzy_match_non_match() {
        assert!(fuzzy_match("xyz", "Split Pane").is_none());
    }

    #[test]
    fn fuzzy_match_partial_no_match() {
        assert!(fuzzy_match("splitz", "Split Pane").is_none());
    }

    #[test]
    fn fuzzy_match_consecutive_bonus() {
        let consec = fuzzy_match("split", "Split Pane").unwrap();
        let spread = fuzzy_match("sltp", "Split Pane").unwrap();
        assert!(consec > spread, "consecutive matches should score higher");
    }

    #[test]
    fn fuzzy_match_word_boundary_bonus() {
        let boundary = fuzzy_match("sp", "Split Pane").unwrap();
        // 's' matches at position 0 (word boundary), 'p' at some later position
        assert!(boundary > 0);
    }

    // ── Command registry tests ──────────────────────────────────

    #[test]
    fn registry_has_entries() {
        let reg = command_registry();
        assert!(!reg.is_empty());
    }

    #[test]
    fn registry_all_names_nonempty() {
        for entry in command_registry() {
            assert!(!entry.name.is_empty(), "Entry has empty name");
        }
    }

    #[test]
    fn registry_all_descriptions_nonempty() {
        for entry in command_registry() {
            assert!(!entry.description.is_empty(), "Entry {} has empty description", entry.name);
        }
    }

    #[test]
    fn registry_contains_split_vertical() {
        let reg = command_registry();
        assert!(reg.iter().any(|e| e.action == PaletteAction::SplitVertical));
    }

    #[test]
    fn registry_contains_new_tab() {
        let reg = command_registry();
        assert!(reg.iter().any(|e| e.action == PaletteAction::NewTab));
    }

    // ── PaletteState tests ──────────────────────────────────────

    #[test]
    fn palette_state_initial_shows_all() {
        let state = PaletteState::new();
        let reg = command_registry();
        assert_eq!(state.result_count(), reg.len());
    }

    #[test]
    fn palette_state_filter_narrows_results() {
        let mut state = PaletteState::new();
        state.type_char('s');
        state.type_char('p');
        state.type_char('l');
        state.type_char('i');
        state.type_char('t');
        // "split" should match fewer entries than all
        let all_count = command_registry().len();
        assert!(state.result_count() < all_count);
        assert!(state.result_count() > 0);
    }

    #[test]
    fn palette_state_no_match_empty_results() {
        let mut state = PaletteState::new();
        for ch in "xyzxyzxyz".chars() {
            state.type_char(ch);
        }
        assert_eq!(state.result_count(), 0);
    }

    #[test]
    fn palette_state_backspace_restores_results() {
        let mut state = PaletteState::new();
        let initial = state.result_count();
        state.type_char('z');
        state.type_char('z');
        state.type_char('z');
        let narrowed = state.result_count();
        state.backspace();
        state.backspace();
        state.backspace();
        assert_eq!(state.result_count(), initial);
        assert!(narrowed <= initial);
    }

    #[test]
    fn palette_state_select_next() {
        let mut state = PaletteState::new();
        assert_eq!(state.selected, 0);
        state.select_next();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn palette_state_select_prev_at_zero() {
        let mut state = PaletteState::new();
        state.select_prev(); // Should stay at 0
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn palette_state_selected_action() {
        let state = PaletteState::new();
        let action = state.selected_action();
        assert!(action.is_some());
    }

    #[test]
    fn palette_state_selected_entry_has_name() {
        let state = PaletteState::new();
        let entry = state.selected_entry();
        assert!(entry.is_some());
        assert!(!entry.unwrap().name.is_empty());
    }

    // ── Keybinding detection tests ──────────────────────────────

    #[test]
    fn should_open_palette_cmd_shift_p() {
        use winit::keyboard::{Key, ModifiersState};
        let mods = ModifiersState::SUPER | ModifiersState::SHIFT;
        assert!(should_open_palette(&Key::Character("P".into()), mods));
    }

    #[test]
    fn should_open_palette_cmd_shift_p_lowercase() {
        use winit::keyboard::{Key, ModifiersState};
        let mods = ModifiersState::SUPER | ModifiersState::SHIFT;
        assert!(should_open_palette(&Key::Character("p".into()), mods));
    }

    #[test]
    fn should_not_open_palette_without_shift() {
        use winit::keyboard::{Key, ModifiersState};
        let mods = ModifiersState::SUPER;
        assert!(!should_open_palette(&Key::Character("p".into()), mods));
    }

    #[test]
    fn should_not_open_palette_without_super() {
        use winit::keyboard::{Key, ModifiersState};
        let mods = ModifiersState::SHIFT;
        assert!(!should_open_palette(&Key::Character("p".into()), mods));
    }

    #[test]
    fn should_not_open_palette_wrong_key() {
        use winit::keyboard::{Key, ModifiersState};
        let mods = ModifiersState::SUPER | ModifiersState::SHIFT;
        assert!(!should_open_palette(&Key::Character("x".into()), mods));
    }
}
