// Keyboard input translation: converts winit KeyEvents to terminal byte sequences.

pub mod clipboard;
pub mod mouse;
pub mod selection;

use std::collections::HashMap;
use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::pane::FocusDirection;

/// The current input mode for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal terminal input — keys go to PTY.
    #[default]
    Normal,
    /// Search mode — keys go to the search bar.
    Search,
}

/// A search-mode command resulting from a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchCommand {
    /// Insert a character into the search query.
    InsertChar(char),
    /// Delete the last character from the query.
    DeleteChar,
    /// Navigate to the next match.
    NextMatch,
    /// Navigate to the previous match.
    PrevMatch,
    /// Close the search overlay.
    Close,
    /// Open search (Ctrl+Shift+F from Normal mode).
    Open,
}

/// Check if a key event should open the search overlay (from Normal mode).
/// Returns true if Ctrl+Shift+F is pressed.
pub fn should_open_search(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> bool {
    let ctrl_shift = modifiers.control_key() && modifiers.shift_key();
    if !ctrl_shift {
        return false;
    }
    matches!(logical_key, Key::Character(s) if s.to_lowercase() == "f")
}

/// Process a key event while in Search mode.
/// Returns a SearchCommand describing what action to take.
pub fn match_search_command(
    logical_key: &Key,
    text: Option<&str>,
    modifiers: ModifiersState,
) -> Option<SearchCommand> {
    // Ctrl+Shift+F toggles search off
    if modifiers.control_key() && modifiers.shift_key() {
        if let Key::Character(s) = logical_key {
            if s.to_lowercase() == "f" {
                return Some(SearchCommand::Close);
            }
        }
    }

    match logical_key {
        Key::Named(named) => match named {
            NamedKey::Escape => Some(SearchCommand::Close),
            NamedKey::Backspace => Some(SearchCommand::DeleteChar),
            NamedKey::Enter => {
                if modifiers.shift_key() {
                    Some(SearchCommand::PrevMatch)
                } else {
                    Some(SearchCommand::NextMatch)
                }
            }
            NamedKey::ArrowDown => Some(SearchCommand::NextMatch),
            NamedKey::ArrowUp => Some(SearchCommand::PrevMatch),
            _ => None,
        },
        Key::Character(s) => {
            // Use text field if available, otherwise logical key
            let t = text.unwrap_or(s.as_ref());
            if let Some(ch) = t.chars().next() {
                if !ch.is_control() {
                    return Some(SearchCommand::InsertChar(ch));
                }
            }
            None
        }
        _ => None,
    }
}

/// A pane management command triggered by a keybinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneCommand {
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    FocusDirection(FocusDirection),
    ZoomToggle,
}

/// Check if a key event matches a pane command keybinding.
///
/// Hardcoded defaults (Ctrl+Shift prefix):
/// - Ctrl+Shift+D: split vertical
/// - Ctrl+Shift+E: split horizontal
/// - Ctrl+Shift+W: close pane
/// - Ctrl+Shift+Arrow: focus direction
/// - Ctrl+Shift+Z: zoom toggle
pub fn match_pane_command(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> Option<PaneCommand> {
    let ctrl_shift = modifiers.control_key() && modifiers.shift_key();
    if !ctrl_shift {
        return None;
    }

    match logical_key {
        Key::Character(s) => {
            let lower = s.to_lowercase();
            match lower.as_str() {
                "d" => Some(PaneCommand::SplitVertical),
                "e" => Some(PaneCommand::SplitHorizontal),
                "w" => Some(PaneCommand::ClosePane),
                "z" => Some(PaneCommand::ZoomToggle),
                _ => None,
            }
        }
        Key::Named(named) => match named {
            NamedKey::ArrowLeft => Some(PaneCommand::FocusDirection(FocusDirection::Left)),
            NamedKey::ArrowRight => Some(PaneCommand::FocusDirection(FocusDirection::Right)),
            NamedKey::ArrowUp => Some(PaneCommand::FocusDirection(FocusDirection::Up)),
            NamedKey::ArrowDown => Some(PaneCommand::FocusDirection(FocusDirection::Down)),
            _ => None,
        },
        _ => None,
    }
}

/// A tab management command triggered by a keybinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCommand {
    NewTab,
    NextTab,
    PrevTab,
    SelectTab(usize),
    MoveTabLeft,
    MoveTabRight,
}

/// Check if a key event matches a tab command keybinding.
///
/// Hardcoded defaults (Ctrl+Shift prefix):
/// - Ctrl+Shift+T: new tab
/// - Ctrl+Shift+Tab / Ctrl+Shift+PageDown: next tab
/// - Ctrl+Shift+PageUp: previous tab
/// - Ctrl+Shift+1..9: select tab by number
/// - Ctrl+Shift+{ / }: move tab left/right
pub fn match_tab_command(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> Option<TabCommand> {
    let ctrl_shift = modifiers.control_key() && modifiers.shift_key();
    if !ctrl_shift {
        return None;
    }

    match logical_key {
        Key::Character(s) => {
            let lower = s.to_lowercase();
            match lower.as_str() {
                "t" => Some(TabCommand::NewTab),
                "{" => Some(TabCommand::MoveTabLeft),
                "}" => Some(TabCommand::MoveTabRight),
                "1" => Some(TabCommand::SelectTab(0)),
                "2" => Some(TabCommand::SelectTab(1)),
                "3" => Some(TabCommand::SelectTab(2)),
                "4" => Some(TabCommand::SelectTab(3)),
                "5" => Some(TabCommand::SelectTab(4)),
                "6" => Some(TabCommand::SelectTab(5)),
                "7" => Some(TabCommand::SelectTab(6)),
                "8" => Some(TabCommand::SelectTab(7)),
                "9" => Some(TabCommand::SelectTab(8)),
                _ => None,
            }
        }
        Key::Named(named) => match named {
            NamedKey::Tab => Some(TabCommand::NextTab),
            NamedKey::PageUp => Some(TabCommand::PrevTab),
            NamedKey::PageDown => Some(TabCommand::NextTab),
            _ => None,
        },
        _ => None,
    }
}

/// A shell integration command triggered by a keybinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    PreviousPrompt,
    NextPrompt,
}

/// Check if a key event matches a shell integration command keybinding.
///
/// Hardcoded defaults:
/// - Ctrl+Shift+P: jump to previous prompt
/// - Ctrl+Shift+N: jump to next prompt
pub fn match_shell_command(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> Option<ShellCommand> {
    let ctrl_shift = modifiers.control_key() && modifiers.shift_key();
    if !ctrl_shift {
        return None;
    }

    match logical_key {
        Key::Character(s) => {
            let lower = s.to_lowercase();
            match lower.as_str() {
                "p" => Some(ShellCommand::PreviousPrompt),
                "n" => Some(ShellCommand::NextPrompt),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Check if a key event should toggle vi-mode (default: Ctrl+Shift+Space).
pub fn should_toggle_vi_mode(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> bool {
    let ctrl_shift = modifiers.control_key() && modifiers.shift_key();
    if !ctrl_shift {
        return false;
    }
    matches!(logical_key, Key::Named(NamedKey::Space))
}

/// An application-level command triggered by a keybinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
}

/// Return the platform-appropriate "primary" modifier.
/// macOS uses Super (Cmd), others use Ctrl.
fn is_primary_modifier(modifiers: ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.super_key() && !modifiers.control_key() && !modifiers.shift_key()
    } else {
        modifiers.control_key() && !modifiers.super_key() && !modifiers.shift_key()
    }
}

/// Check if a key event matches an application command keybinding.
///
/// Platform-aware shortcuts:
/// - Cmd+= / Cmd+Plus (macOS) or Ctrl+= / Ctrl+Plus (Linux): increase font size
/// - Cmd+Minus (macOS) or Ctrl+Minus (Linux): decrease font size
/// - Cmd+0 (macOS) or Ctrl+0 (Linux): reset font size
pub fn match_app_command(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> Option<AppCommand> {
    if !is_primary_modifier(modifiers) {
        return None;
    }

    match logical_key {
        Key::Character(s) => {
            match s.as_ref() {
                "=" | "+" => Some(AppCommand::IncreaseFontSize),
                "-" => Some(AppCommand::DecreaseFontSize),
                "0" => Some(AppCommand::ResetFontSize),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Translate a winit key event into terminal byte sequences to send to the PTY.
///
/// Returns `None` if the key event should not produce any output (e.g. modifier-only
/// keys, key releases, or unhandled keys).
pub fn translate_key(
    logical_key: &Key,
    text: Option<&str>,
    state: ElementState,
    modifiers: ModifiersState,
) -> Option<Vec<u8>> {
    // Only handle key presses, not releases
    if state == ElementState::Released {
        return None;
    }

    match logical_key {
        Key::Character(s) => {
            // Ctrl+letter → control byte
            if modifiers.control_key() {
                if let Some(ch) = s.chars().next() {
                    if let Some(byte) = ctrl_key_byte(ch) {
                        return Some(vec![byte]);
                    }
                }
            }
            // Use the text field if available, otherwise the logical key string
            let t = text.unwrap_or(s.as_ref());
            Some(t.as_bytes().to_vec())
        }
        Key::Named(named) => named_key_bytes(*named, modifiers),
        _ => None,
    }
}

/// Look up a keybinding action from config bindings.
///
/// Given a key combo string (e.g., "ctrl+shift+c") and the config keybinding map,
/// returns the action name if a binding is defined, or None.
pub fn lookup_binding(key_combo: &str, bindings: &HashMap<String, String>) -> Option<String> {
    bindings.get(key_combo).cloned()
}

/// Translate a Ctrl+key combination to a control byte (0x01..=0x1A).
/// Returns `None` if the character is not a letter a-z/A-Z.
fn ctrl_key_byte(ch: char) -> Option<u8> {
    let lower = ch.to_ascii_lowercase();
    if lower.is_ascii_lowercase() {
        Some(lower as u8 - b'a' + 1)
    } else {
        None
    }
}

/// Translate a named key (Enter, Backspace, etc.) to terminal bytes.
fn named_key_bytes(key: NamedKey, _modifiers: ModifiersState) -> Option<Vec<u8>> {
    match key {
        // Basic control keys
        NamedKey::Enter => Some(b"\r".to_vec()),
        NamedKey::Backspace => Some(vec![0x7f]),
        NamedKey::Tab => Some(b"\t".to_vec()),
        NamedKey::Escape => Some(b"\x1b".to_vec()),
        NamedKey::Space => Some(b" ".to_vec()),

        // Arrow keys (normal mode CSI sequences)
        NamedKey::ArrowUp => Some(b"\x1b[A".to_vec()),
        NamedKey::ArrowDown => Some(b"\x1b[B".to_vec()),
        NamedKey::ArrowRight => Some(b"\x1b[C".to_vec()),
        NamedKey::ArrowLeft => Some(b"\x1b[D".to_vec()),

        // Navigation keys
        NamedKey::Home => Some(b"\x1b[H".to_vec()),
        NamedKey::End => Some(b"\x1b[F".to_vec()),
        NamedKey::Insert => Some(b"\x1b[2~".to_vec()),
        NamedKey::Delete => Some(b"\x1b[3~".to_vec()),
        NamedKey::PageUp => Some(b"\x1b[5~".to_vec()),
        NamedKey::PageDown => Some(b"\x1b[6~".to_vec()),

        // Function keys (F1-F4: SS3 format, F5-F12: CSI ~ format)
        NamedKey::F1 => Some(b"\x1bOP".to_vec()),
        NamedKey::F2 => Some(b"\x1bOQ".to_vec()),
        NamedKey::F3 => Some(b"\x1bOR".to_vec()),
        NamedKey::F4 => Some(b"\x1bOS".to_vec()),
        NamedKey::F5 => Some(b"\x1b[15~".to_vec()),
        NamedKey::F6 => Some(b"\x1b[17~".to_vec()),
        NamedKey::F7 => Some(b"\x1b[18~".to_vec()),
        NamedKey::F8 => Some(b"\x1b[19~".to_vec()),
        NamedKey::F9 => Some(b"\x1b[20~".to_vec()),
        NamedKey::F10 => Some(b"\x1b[21~".to_vec()),
        NamedKey::F11 => Some(b"\x1b[23~".to_vec()),
        NamedKey::F12 => Some(b"\x1b[24~".to_vec()),

        // Modifier-only keys produce no output
        NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Super => None,

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a pressed key event translation
    fn press(key: Key, text: Option<&str>, mods: ModifiersState) -> Option<Vec<u8>> {
        translate_key(&key, text, ElementState::Pressed, mods)
    }

    fn no_mods() -> ModifiersState {
        ModifiersState::empty()
    }

    // ── Printable character encoding ────────────────────────────────

    #[test]
    fn ascii_letter_a() {
        let result = press(Key::Character("a".into()), Some("a"), no_mods());
        assert_eq!(result, Some(b"a".to_vec()));
    }

    #[test]
    fn ascii_letter_uppercase_a() {
        let result = press(Key::Character("A".into()), Some("A"), ModifiersState::SHIFT);
        assert_eq!(result, Some(b"A".to_vec()));
    }

    #[test]
    fn ascii_digit() {
        let result = press(Key::Character("5".into()), Some("5"), no_mods());
        assert_eq!(result, Some(b"5".to_vec()));
    }

    #[test]
    fn ascii_symbol() {
        let result = press(Key::Character("@".into()), Some("@"), no_mods());
        assert_eq!(result, Some(b"@".to_vec()));
    }

    #[test]
    fn utf8_multibyte_char() {
        let result = press(
            Key::Character("\u{00e9}".into()),
            Some("\u{00e9}"),
            no_mods(),
        );
        assert_eq!(result, Some("\u{00e9}".as_bytes().to_vec()));
    }

    #[test]
    fn space_key() {
        let result = press(Key::Character(" ".into()), Some(" "), no_mods());
        assert_eq!(result, Some(b" ".to_vec()));
    }

    // ── Key release produces no output ──────────────────────────────

    #[test]
    fn key_release_produces_none() {
        let result = translate_key(
            &Key::Character("a".into()),
            Some("a"),
            ElementState::Released,
            no_mods(),
        );
        assert_eq!(result, None);
    }

    // ── Special key translation ─────────────────────────────────────

    #[test]
    fn enter_key() {
        let result = press(Key::Named(NamedKey::Enter), None, no_mods());
        assert_eq!(result, Some(b"\r".to_vec()));
    }

    #[test]
    fn backspace_key() {
        let result = press(Key::Named(NamedKey::Backspace), None, no_mods());
        assert_eq!(result, Some(vec![0x7f]));
    }

    #[test]
    fn tab_key() {
        let result = press(Key::Named(NamedKey::Tab), None, no_mods());
        assert_eq!(result, Some(b"\t".to_vec()));
    }

    #[test]
    fn escape_key() {
        let result = press(Key::Named(NamedKey::Escape), None, no_mods());
        assert_eq!(result, Some(b"\x1b".to_vec()));
    }

    // ── Arrow keys → ANSI escape sequences ──────────────────────────

    #[test]
    fn arrow_up() {
        let result = press(Key::Named(NamedKey::ArrowUp), None, no_mods());
        assert_eq!(result, Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn arrow_down() {
        let result = press(Key::Named(NamedKey::ArrowDown), None, no_mods());
        assert_eq!(result, Some(b"\x1b[B".to_vec()));
    }

    #[test]
    fn arrow_right() {
        let result = press(Key::Named(NamedKey::ArrowRight), None, no_mods());
        assert_eq!(result, Some(b"\x1b[C".to_vec()));
    }

    #[test]
    fn arrow_left() {
        let result = press(Key::Named(NamedKey::ArrowLeft), None, no_mods());
        assert_eq!(result, Some(b"\x1b[D".to_vec()));
    }

    // ── Control key combinations ────────────────────────────────────

    #[test]
    fn ctrl_c() {
        let result = press(Key::Character("c".into()), None, ModifiersState::CONTROL);
        assert_eq!(result, Some(vec![0x03]));
    }

    #[test]
    fn ctrl_d() {
        let result = press(Key::Character("d".into()), None, ModifiersState::CONTROL);
        assert_eq!(result, Some(vec![0x04]));
    }

    #[test]
    fn ctrl_z() {
        let result = press(Key::Character("z".into()), None, ModifiersState::CONTROL);
        assert_eq!(result, Some(vec![0x1a]));
    }

    #[test]
    fn ctrl_a() {
        let result = press(Key::Character("a".into()), None, ModifiersState::CONTROL);
        assert_eq!(result, Some(vec![0x01]));
    }

    #[test]
    fn ctrl_l() {
        let result = press(Key::Character("l".into()), None, ModifiersState::CONTROL);
        assert_eq!(result, Some(vec![0x0c]));
    }

    #[test]
    fn ctrl_e() {
        let result = press(Key::Character("e".into()), None, ModifiersState::CONTROL);
        assert_eq!(result, Some(vec![0x05]));
    }

    // ── Function keys ───────────────────────────────────────────────

    #[test]
    fn f1_key() {
        let result = press(Key::Named(NamedKey::F1), None, no_mods());
        assert_eq!(result, Some(b"\x1bOP".to_vec()));
    }

    #[test]
    fn f2_key() {
        let result = press(Key::Named(NamedKey::F2), None, no_mods());
        assert_eq!(result, Some(b"\x1bOQ".to_vec()));
    }

    #[test]
    fn f3_key() {
        let result = press(Key::Named(NamedKey::F3), None, no_mods());
        assert_eq!(result, Some(b"\x1bOR".to_vec()));
    }

    #[test]
    fn f4_key() {
        let result = press(Key::Named(NamedKey::F4), None, no_mods());
        assert_eq!(result, Some(b"\x1bOS".to_vec()));
    }

    #[test]
    fn f5_key() {
        let result = press(Key::Named(NamedKey::F5), None, no_mods());
        assert_eq!(result, Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn f12_key() {
        let result = press(Key::Named(NamedKey::F12), None, no_mods());
        assert_eq!(result, Some(b"\x1b[24~".to_vec()));
    }

    // ── Navigation keys ─────────────────────────────────────────────

    #[test]
    fn home_key() {
        let result = press(Key::Named(NamedKey::Home), None, no_mods());
        assert_eq!(result, Some(b"\x1b[H".to_vec()));
    }

    #[test]
    fn end_key() {
        let result = press(Key::Named(NamedKey::End), None, no_mods());
        assert_eq!(result, Some(b"\x1b[F".to_vec()));
    }

    #[test]
    fn delete_key() {
        let result = press(Key::Named(NamedKey::Delete), None, no_mods());
        assert_eq!(result, Some(b"\x1b[3~".to_vec()));
    }

    #[test]
    fn insert_key() {
        let result = press(Key::Named(NamedKey::Insert), None, no_mods());
        assert_eq!(result, Some(b"\x1b[2~".to_vec()));
    }

    #[test]
    fn page_up_key() {
        let result = press(Key::Named(NamedKey::PageUp), None, no_mods());
        assert_eq!(result, Some(b"\x1b[5~".to_vec()));
    }

    #[test]
    fn page_down_key() {
        let result = press(Key::Named(NamedKey::PageDown), None, no_mods());
        assert_eq!(result, Some(b"\x1b[6~".to_vec()));
    }

    // ── Modifier-only keys produce no output ────────────────────────

    #[test]
    fn space_named_key() {
        let result = press(Key::Named(NamedKey::Space), None, no_mods());
        assert_eq!(result, Some(b" ".to_vec()));
    }

    #[test]
    fn shift_alone_produces_none() {
        let result = press(Key::Named(NamedKey::Shift), None, ModifiersState::SHIFT);
        assert_eq!(result, None);
    }

    #[test]
    fn control_alone_produces_none() {
        let result = press(Key::Named(NamedKey::Control), None, ModifiersState::CONTROL);
        assert_eq!(result, None);
    }

    #[test]
    fn alt_alone_produces_none() {
        let result = press(Key::Named(NamedKey::Alt), None, ModifiersState::ALT);
        assert_eq!(result, None);
    }

    // ── ctrl_key_byte helper ────────────────────────────────────────

    #[test]
    fn ctrl_byte_lowercase_a() {
        assert_eq!(ctrl_key_byte('a'), Some(0x01));
    }

    #[test]
    fn ctrl_byte_uppercase_a() {
        assert_eq!(ctrl_key_byte('A'), Some(0x01));
    }

    #[test]
    fn ctrl_byte_z() {
        assert_eq!(ctrl_key_byte('z'), Some(0x1a));
    }

    #[test]
    fn ctrl_byte_non_letter() {
        assert_eq!(ctrl_key_byte('5'), None);
    }

    // ── Keybinding lookup tests ───────────────────────────────────

    #[test]
    fn lookup_binding_found() {
        let mut bindings = HashMap::new();
        bindings.insert("ctrl+shift+c".to_string(), "copy".to_string());
        assert_eq!(
            lookup_binding("ctrl+shift+c", &bindings),
            Some("copy".to_string())
        );
    }

    #[test]
    fn lookup_binding_not_found() {
        let bindings = HashMap::new();
        assert_eq!(lookup_binding("ctrl+shift+c", &bindings), None);
    }

    #[test]
    fn lookup_binding_multiple() {
        let mut bindings = HashMap::new();
        bindings.insert("ctrl+shift+c".to_string(), "copy".to_string());
        bindings.insert("ctrl+shift+v".to_string(), "paste".to_string());
        assert_eq!(
            lookup_binding("ctrl+shift+v", &bindings),
            Some("paste".to_string())
        );
    }

    // ── Pane command matching ──────────────────────────────────────

    fn ctrl_shift() -> ModifiersState {
        ModifiersState::CONTROL | ModifiersState::SHIFT
    }

    #[test]
    fn pane_cmd_split_vertical() {
        let result = match_pane_command(&Key::Character("D".into()), ctrl_shift());
        assert_eq!(result, Some(PaneCommand::SplitVertical));
    }

    #[test]
    fn pane_cmd_split_horizontal() {
        let result = match_pane_command(&Key::Character("E".into()), ctrl_shift());
        assert_eq!(result, Some(PaneCommand::SplitHorizontal));
    }

    #[test]
    fn pane_cmd_close_pane() {
        let result = match_pane_command(&Key::Character("W".into()), ctrl_shift());
        assert_eq!(result, Some(PaneCommand::ClosePane));
    }

    #[test]
    fn pane_cmd_zoom_toggle() {
        let result = match_pane_command(&Key::Character("Z".into()), ctrl_shift());
        assert_eq!(result, Some(PaneCommand::ZoomToggle));
    }

    #[test]
    fn pane_cmd_focus_left() {
        let result = match_pane_command(&Key::Named(NamedKey::ArrowLeft), ctrl_shift());
        assert_eq!(
            result,
            Some(PaneCommand::FocusDirection(FocusDirection::Left))
        );
    }

    #[test]
    fn pane_cmd_focus_right() {
        let result = match_pane_command(&Key::Named(NamedKey::ArrowRight), ctrl_shift());
        assert_eq!(
            result,
            Some(PaneCommand::FocusDirection(FocusDirection::Right))
        );
    }

    #[test]
    fn pane_cmd_focus_up() {
        let result = match_pane_command(&Key::Named(NamedKey::ArrowUp), ctrl_shift());
        assert_eq!(
            result,
            Some(PaneCommand::FocusDirection(FocusDirection::Up))
        );
    }

    #[test]
    fn pane_cmd_focus_down() {
        let result = match_pane_command(&Key::Named(NamedKey::ArrowDown), ctrl_shift());
        assert_eq!(
            result,
            Some(PaneCommand::FocusDirection(FocusDirection::Down))
        );
    }

    #[test]
    fn pane_cmd_normal_key_no_match() {
        let result = match_pane_command(&Key::Character("a".into()), no_mods());
        assert_eq!(result, None);
    }

    #[test]
    fn pane_cmd_ctrl_only_no_match() {
        let result = match_pane_command(&Key::Character("d".into()), ModifiersState::CONTROL);
        assert_eq!(result, None);
    }

    #[test]
    fn pane_cmd_unbound_key_with_ctrl_shift_no_match() {
        let result = match_pane_command(&Key::Character("x".into()), ctrl_shift());
        assert_eq!(result, None);
    }

    // ── Tab command matching ─────────────────────────────────────

    #[test]
    fn tab_cmd_new_tab() {
        let result = match_tab_command(&Key::Character("T".into()), ctrl_shift());
        assert_eq!(result, Some(TabCommand::NewTab));
    }

    #[test]
    fn tab_cmd_new_tab_lowercase() {
        let result = match_tab_command(&Key::Character("t".into()), ctrl_shift());
        assert_eq!(result, Some(TabCommand::NewTab));
    }

    #[test]
    fn tab_cmd_next_tab_via_tab_key() {
        let result = match_tab_command(&Key::Named(NamedKey::Tab), ctrl_shift());
        assert_eq!(result, Some(TabCommand::NextTab));
    }

    #[test]
    fn tab_cmd_next_tab_via_pagedown() {
        let result = match_tab_command(&Key::Named(NamedKey::PageDown), ctrl_shift());
        assert_eq!(result, Some(TabCommand::NextTab));
    }

    #[test]
    fn tab_cmd_prev_tab_via_pageup() {
        let result = match_tab_command(&Key::Named(NamedKey::PageUp), ctrl_shift());
        assert_eq!(result, Some(TabCommand::PrevTab));
    }

    #[test]
    fn tab_cmd_select_tab_1() {
        let result = match_tab_command(&Key::Character("1".into()), ctrl_shift());
        assert_eq!(result, Some(TabCommand::SelectTab(0)));
    }

    #[test]
    fn tab_cmd_select_tab_9() {
        let result = match_tab_command(&Key::Character("9".into()), ctrl_shift());
        assert_eq!(result, Some(TabCommand::SelectTab(8)));
    }

    #[test]
    fn tab_cmd_move_tab_left() {
        let result = match_tab_command(&Key::Character("{".into()), ctrl_shift());
        assert_eq!(result, Some(TabCommand::MoveTabLeft));
    }

    #[test]
    fn tab_cmd_move_tab_right() {
        let result = match_tab_command(&Key::Character("}".into()), ctrl_shift());
        assert_eq!(result, Some(TabCommand::MoveTabRight));
    }

    #[test]
    fn tab_cmd_no_match_without_ctrl_shift() {
        let result = match_tab_command(&Key::Character("t".into()), no_mods());
        assert_eq!(result, None);
    }

    #[test]
    fn tab_cmd_no_match_unbound_key() {
        let result = match_tab_command(&Key::Character("x".into()), ctrl_shift());
        assert_eq!(result, None);
    }

    // ── Search command matching (2.3) ───────────────────────────────

    #[test]
    fn search_ctrl_shift_f_opens_search() {
        assert!(should_open_search(&Key::Character("F".into()), ctrl_shift()));
    }

    #[test]
    fn search_ctrl_shift_f_lowercase_opens_search() {
        assert!(should_open_search(&Key::Character("f".into()), ctrl_shift()));
    }

    #[test]
    fn search_normal_f_does_not_open_search() {
        assert!(!should_open_search(&Key::Character("f".into()), no_mods()));
    }

    #[test]
    fn search_ctrl_only_f_does_not_open_search() {
        assert!(!should_open_search(
            &Key::Character("f".into()),
            ModifiersState::CONTROL
        ));
    }

    // ── 2.3.1 Printable chars → InsertChar ─────────────────────────

    #[test]
    fn search_mode_printable_char() {
        let result = match_search_command(
            &Key::Character("a".into()),
            Some("a"),
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::InsertChar('a')));
    }

    #[test]
    fn search_mode_uppercase_char() {
        let result = match_search_command(
            &Key::Character("A".into()),
            Some("A"),
            ModifiersState::SHIFT,
        );
        assert_eq!(result, Some(SearchCommand::InsertChar('A')));
    }

    #[test]
    fn search_mode_digit_char() {
        let result = match_search_command(
            &Key::Character("5".into()),
            Some("5"),
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::InsertChar('5')));
    }

    #[test]
    fn search_mode_special_char() {
        let result = match_search_command(
            &Key::Character(".".into()),
            Some("."),
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::InsertChar('.')));
    }

    // ── 2.3.2 Backspace → DeleteChar ───────────────────────────────

    #[test]
    fn search_mode_backspace() {
        let result = match_search_command(
            &Key::Named(NamedKey::Backspace),
            None,
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::DeleteChar));
    }

    // ── 2.3.3 Escape → Close ──────────────────────────────────────

    #[test]
    fn search_mode_escape_closes() {
        let result = match_search_command(
            &Key::Named(NamedKey::Escape),
            None,
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::Close));
    }

    // ── 2.3.4 Enter → NextMatch ───────────────────────────────────

    #[test]
    fn search_mode_enter_next_match() {
        let result = match_search_command(
            &Key::Named(NamedKey::Enter),
            None,
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::NextMatch));
    }

    // ── 2.3.5 Shift+Enter → PrevMatch ─────────────────────────────

    #[test]
    fn search_mode_shift_enter_prev_match() {
        let result = match_search_command(
            &Key::Named(NamedKey::Enter),
            None,
            ModifiersState::SHIFT,
        );
        assert_eq!(result, Some(SearchCommand::PrevMatch));
    }

    // ── 2.3.6 Ctrl+Shift+F in search mode → Close (toggle) ───────

    #[test]
    fn search_mode_ctrl_shift_f_closes() {
        let result = match_search_command(
            &Key::Character("F".into()),
            None,
            ctrl_shift(),
        );
        assert_eq!(result, Some(SearchCommand::Close));
    }

    // ── 2.3.7 Arrow up/down → prev/next match ─────────────────────

    #[test]
    fn search_mode_arrow_down_next_match() {
        let result = match_search_command(
            &Key::Named(NamedKey::ArrowDown),
            None,
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::NextMatch));
    }

    #[test]
    fn search_mode_arrow_up_prev_match() {
        let result = match_search_command(
            &Key::Named(NamedKey::ArrowUp),
            None,
            no_mods(),
        );
        assert_eq!(result, Some(SearchCommand::PrevMatch));
    }

    // ── Shell command matching ────────────────────────────────────

    #[test]
    fn shell_cmd_previous_prompt() {
        let result = match_shell_command(&Key::Character("P".into()), ctrl_shift());
        assert_eq!(result, Some(ShellCommand::PreviousPrompt));
    }

    #[test]
    fn shell_cmd_previous_prompt_lowercase() {
        let result = match_shell_command(&Key::Character("p".into()), ctrl_shift());
        assert_eq!(result, Some(ShellCommand::PreviousPrompt));
    }

    #[test]
    fn shell_cmd_next_prompt() {
        let result = match_shell_command(&Key::Character("N".into()), ctrl_shift());
        assert_eq!(result, Some(ShellCommand::NextPrompt));
    }

    #[test]
    fn shell_cmd_next_prompt_lowercase() {
        let result = match_shell_command(&Key::Character("n".into()), ctrl_shift());
        assert_eq!(result, Some(ShellCommand::NextPrompt));
    }

    #[test]
    fn shell_cmd_no_match_without_ctrl_shift() {
        let result = match_shell_command(&Key::Character("p".into()), no_mods());
        assert_eq!(result, None);
    }

    #[test]
    fn shell_cmd_no_match_unbound_key() {
        let result = match_shell_command(&Key::Character("x".into()), ctrl_shift());
        assert_eq!(result, None);
    }

    // ── Vi-mode toggle ──────────────────────────────────────────

    #[test]
    fn vi_mode_toggle_ctrl_shift_space() {
        assert!(should_toggle_vi_mode(
            &Key::Named(NamedKey::Space),
            ctrl_shift()
        ));
    }

    #[test]
    fn vi_mode_toggle_space_alone_no_match() {
        assert!(!should_toggle_vi_mode(
            &Key::Named(NamedKey::Space),
            no_mods()
        ));
    }

    #[test]
    fn vi_mode_toggle_ctrl_only_no_match() {
        assert!(!should_toggle_vi_mode(
            &Key::Named(NamedKey::Space),
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn vi_mode_toggle_other_key_no_match() {
        assert!(!should_toggle_vi_mode(
            &Key::Character("a".into()),
            ctrl_shift()
        ));
    }

    // ── InputMode default ──────────────────────────────────────────

    #[test]
    fn input_mode_defaults_to_normal() {
        assert_eq!(InputMode::default(), InputMode::Normal);
    }

    // ── App command matching (font size) ─────────────────────────

    /// Helper: platform primary modifier (Super on macOS, Ctrl on Linux).
    fn primary_mod() -> ModifiersState {
        if cfg!(target_os = "macos") {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        }
    }

    #[test]
    fn app_cmd_increase_font_equals() {
        let result = match_app_command(&Key::Character("=".into()), primary_mod());
        assert_eq!(result, Some(AppCommand::IncreaseFontSize));
    }

    #[test]
    fn app_cmd_increase_font_plus() {
        let result = match_app_command(&Key::Character("+".into()), primary_mod());
        assert_eq!(result, Some(AppCommand::IncreaseFontSize));
    }

    #[test]
    fn app_cmd_decrease_font() {
        let result = match_app_command(&Key::Character("-".into()), primary_mod());
        assert_eq!(result, Some(AppCommand::DecreaseFontSize));
    }

    #[test]
    fn app_cmd_reset_font() {
        let result = match_app_command(&Key::Character("0".into()), primary_mod());
        assert_eq!(result, Some(AppCommand::ResetFontSize));
    }

    #[test]
    fn app_cmd_no_match_without_modifier() {
        let result = match_app_command(&Key::Character("=".into()), no_mods());
        assert_eq!(result, None);
    }

    #[test]
    fn app_cmd_no_match_wrong_modifier() {
        // Ctrl+Shift should not trigger app commands
        let result = match_app_command(&Key::Character("=".into()), ctrl_shift());
        assert_eq!(result, None);
    }

    #[test]
    fn app_cmd_no_match_unbound_key() {
        let result = match_app_command(&Key::Character("a".into()), primary_mod());
        assert_eq!(result, None);
    }
}
