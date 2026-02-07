// Keyboard input translation: converts winit KeyEvents to terminal byte sequences.

pub mod clipboard;
pub mod selection;

use std::collections::HashMap;
use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::pane::{FocusDirection, SplitDirection};

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
}
