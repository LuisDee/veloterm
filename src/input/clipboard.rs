// Clipboard integration: keybinding detection, bracketed paste, and system clipboard access.

use winit::keyboard::{Key, ModifiersState};

/// Detect if a key event is a copy keybinding.
/// macOS: Cmd+C, Linux: Ctrl+Shift+C.
pub fn is_copy_keybinding(key: &Key, modifiers: ModifiersState) -> bool {
    let is_c = matches!(key, Key::Character(s) if s.as_ref() == "c");
    if !is_c {
        return false;
    }
    // macOS: Cmd+C (Super)
    if modifiers.super_key() && !modifiers.control_key() {
        return true;
    }
    // Linux: Ctrl+Shift+C
    modifiers.control_key() && modifiers.shift_key()
}

/// Detect if a key event is a paste keybinding.
/// macOS: Cmd+V, Linux: Ctrl+Shift+V.
pub fn is_paste_keybinding(key: &Key, modifiers: ModifiersState) -> bool {
    let is_v = matches!(key, Key::Character(s) if s.as_ref() == "v");
    if !is_v {
        return false;
    }
    // macOS: Cmd+V (Super)
    if modifiers.super_key() && !modifiers.control_key() {
        return true;
    }
    // Linux: Ctrl+Shift+V
    modifiers.control_key() && modifiers.shift_key()
}

/// Wrap text in bracketed paste mode escape sequences.
pub fn wrap_bracketed_paste(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 12);
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

/// Generate bytes to send to PTY for a paste operation.
/// If bracketed paste mode is enabled, wraps text in escape sequences.
pub fn paste_bytes(text: &str, bracketed_paste_enabled: bool) -> Vec<u8> {
    if bracketed_paste_enabled {
        wrap_bracketed_paste(text)
    } else {
        text.as_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Copy keybinding detection ────────────────────────────────────

    #[test]
    fn copy_cmd_c_is_copy() {
        // macOS: Cmd+C
        assert!(is_copy_keybinding(
            &Key::Character("c".into()),
            ModifiersState::SUPER
        ));
    }

    #[test]
    fn copy_ctrl_shift_c_is_copy() {
        // Linux: Ctrl+Shift+C
        assert!(is_copy_keybinding(
            &Key::Character("c".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn copy_ctrl_c_alone_is_not_copy() {
        // Ctrl+C is terminal interrupt, not copy
        assert!(!is_copy_keybinding(
            &Key::Character("c".into()),
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn copy_wrong_key_is_not_copy() {
        assert!(!is_copy_keybinding(
            &Key::Character("x".into()),
            ModifiersState::SUPER
        ));
    }

    // ── Paste keybinding detection ───────────────────────────────────

    #[test]
    fn paste_cmd_v_is_paste() {
        // macOS: Cmd+V
        assert!(is_paste_keybinding(
            &Key::Character("v".into()),
            ModifiersState::SUPER
        ));
    }

    #[test]
    fn paste_ctrl_shift_v_is_paste() {
        // Linux: Ctrl+Shift+V
        assert!(is_paste_keybinding(
            &Key::Character("v".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ));
    }

    #[test]
    fn paste_ctrl_v_alone_is_not_paste() {
        assert!(!is_paste_keybinding(
            &Key::Character("v".into()),
            ModifiersState::CONTROL
        ));
    }

    // ── Bracketed paste wrapping ─────────────────────────────────────

    #[test]
    fn wrap_bracketed_paste_wraps_text() {
        let result = wrap_bracketed_paste("hello");
        assert_eq!(result, b"\x1b[200~hello\x1b[201~");
    }

    #[test]
    fn wrap_bracketed_paste_empty_string() {
        let result = wrap_bracketed_paste("");
        assert_eq!(result, b"\x1b[200~\x1b[201~");
    }

    // ── Paste byte generation ────────────────────────────────────────

    #[test]
    fn paste_bytes_without_bracketed_mode() {
        let result = paste_bytes("hello", false);
        assert_eq!(result, b"hello");
    }

    #[test]
    fn paste_bytes_with_bracketed_mode() {
        let result = paste_bytes("hello", true);
        assert_eq!(result, b"\x1b[200~hello\x1b[201~");
    }

    #[test]
    fn paste_bytes_utf8_content() {
        let result = paste_bytes("caf\u{00e9}", false);
        assert_eq!(result, "caf\u{00e9}".as_bytes());
    }
}
