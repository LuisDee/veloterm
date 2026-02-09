// Global hotkey registration for Quick Terminal toggle.

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use std::thread;
use winit::event_loop::EventLoopProxy;

use crate::config::watcher::UserEvent;

/// Manages a global hotkey that sends QuickTerminalToggle events to the event loop.
pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
    _listener_thread: thread::JoinHandle<()>,
}

impl HotkeyManager {
    /// Register a global hotkey and start listening for presses.
    ///
    /// The `hotkey_str` is parsed by the `global-hotkey` crate (e.g., "Control+`", "Alt+Space").
    /// When pressed, a `QuickTerminalToggle` event is sent via the proxy.
    pub fn new(
        hotkey_str: &str,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("failed to create hotkey manager: {e}"))?;

        let hotkey: HotKey = hotkey_str
            .parse()
            .map_err(|e: global_hotkey::hotkey::HotKeyParseError| {
                format!("failed to parse hotkey '{hotkey_str}': {e}")
            })?;

        manager
            .register(hotkey)
            .map_err(|e| format!("failed to register hotkey: {e}"))?;

        let listener_thread = thread::spawn(move || {
            let receiver = GlobalHotKeyEvent::receiver();
            loop {
                match receiver.recv() {
                    Ok(_event) => {
                        if proxy.send_event(UserEvent::QuickTerminalToggle).is_err() {
                            break; // Event loop closed
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            _manager: manager,
            _listener_thread: listener_thread,
        })
    }
}

/// Parse and validate a hotkey string without registering it.
/// Returns Ok(()) if the string is valid, Err with a message otherwise.
pub fn validate_hotkey_str(hotkey_str: &str) -> Result<(), String> {
    let _hotkey: HotKey = hotkey_str
        .parse()
        .map_err(|e: global_hotkey::hotkey::HotKeyParseError| {
            format!("invalid hotkey '{hotkey_str}': {e}")
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_hotkey_control_backtick() {
        assert!(validate_hotkey_str("Control+`").is_ok());
    }

    #[test]
    fn validate_hotkey_alt_space() {
        assert!(validate_hotkey_str("Alt+Space").is_ok());
    }

    #[test]
    fn validate_hotkey_invalid_string() {
        assert!(validate_hotkey_str("not_a_real_hotkey").is_err());
    }

    #[test]
    fn validate_hotkey_empty_string() {
        assert!(validate_hotkey_str("").is_err());
    }
}
