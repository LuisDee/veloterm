use super::types::{Config, ConfigDelta};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Custom user event for the winit event loop.
#[derive(Debug, Clone)]
pub enum UserEvent {
    /// A valid config reload was detected.
    ConfigReloaded(Config, ConfigDelta),
    /// Quick terminal global hotkey was pressed.
    QuickTerminalToggle,
}

/// Watches a config file for changes and invokes a callback on valid reloads.
///
/// On parse/validation error, the previous config is kept and a warning is logged.
/// The watcher thread shuts down cleanly when dropped (notify handles this on drop).
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    _path: PathBuf,
}

impl ConfigWatcher {
    /// Create a new config watcher.
    ///
    /// - `path` — the config file to watch
    /// - `current_config` — baseline for diffing
    /// - `on_reload` — called with (new_config, delta) when a valid change is detected
    pub fn new<F>(path: &Path, current_config: Config, on_reload: F) -> Result<Self, notify::Error>
    where
        F: Fn(Config, ConfigDelta) + Send + 'static,
    {
        let config_path = path.to_path_buf();
        let current = Arc::new(Mutex::new(current_config));

        let watch_path = config_path.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                let event = match res {
                    Ok(ev) => ev,
                    Err(e) => {
                        log::warn!("File watcher error: {e}");
                        return;
                    }
                };

                // Only react to modify/create events (covers writes and atomic saves)
                use notify::EventKind;
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {}
                    _ => return,
                }

                // Re-read and parse the config file
                let contents = match std::fs::read_to_string(&watch_path) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to read config file: {e}");
                        return;
                    }
                };

                let new_config = match Config::from_toml(&contents) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Config reload failed (keeping previous): {e}");
                        return;
                    }
                };

                let mut prev = current.lock().unwrap();
                let delta = prev.diff(&new_config);
                if !delta.is_empty() {
                    on_reload(new_config.clone(), delta);
                    *prev = new_config;
                }
            })?;

        // Watch the parent directory (some editors do atomic saves via rename)
        let watch_dir = path.parent().unwrap_or(path);
        watcher.watch(watch_dir, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            _path: config_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn watcher_new_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("veloterm.toml");
        std::fs::write(&path, "[font]\nsize = 14.0\n").unwrap();

        let (tx, _rx) = mpsc::channel::<(Config, ConfigDelta)>();
        let watcher = ConfigWatcher::new(&path, Config::default(), move |cfg, delta| {
            let _ = tx.send((cfg, delta));
        });
        assert!(watcher.is_ok());
    }

    #[test]
    fn watcher_detects_file_modification() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("veloterm.toml");
        std::fs::write(&path, "[font]\nsize = 14.0\n").unwrap();

        let (tx, rx) = mpsc::channel::<(Config, ConfigDelta)>();
        let _watcher = ConfigWatcher::new(&path, Config::default(), move |cfg, delta| {
            let _ = tx.send((cfg, delta));
        })
        .unwrap();

        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(&path, "[font]\nsize = 20.0\n").unwrap();

        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok(), "Expected config reload callback");
    }

    #[test]
    fn watcher_callback_receives_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("veloterm.toml");
        std::fs::write(&path, "[font]\nsize = 14.0\n").unwrap();

        let (tx, rx) = mpsc::channel::<(Config, ConfigDelta)>();
        let _watcher = ConfigWatcher::new(&path, Config::default(), move |cfg, delta| {
            let _ = tx.send((cfg, delta));
        })
        .unwrap();

        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(&path, "[font]\nsize = 20.0\n").unwrap();

        let (config, _delta) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(config.font.size, 20.0);
    }

    #[test]
    fn watcher_malformed_change_keeps_previous() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("veloterm.toml");
        std::fs::write(&path, "[font]\nsize = 14.0\n").unwrap();

        let (tx, rx) = mpsc::channel::<(Config, ConfigDelta)>();
        let _watcher = ConfigWatcher::new(&path, Config::default(), move |cfg, delta| {
            let _ = tx.send((cfg, delta));
        })
        .unwrap();

        // First: valid change to prove watcher works
        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(&path, "[font]\nsize = 20.0\n").unwrap();
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok(), "Valid change should trigger callback");

        // Now: invalid change (font size 0 fails validation)
        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(&path, "[font]\nsize = 0.0\n").unwrap();

        // Should NOT receive callback
        let result = rx.recv_timeout(Duration::from_secs(2));
        assert!(
            result.is_err(),
            "Malformed config should not trigger callback"
        );
    }

    #[test]
    fn watcher_sends_config_delta() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("veloterm.toml");
        std::fs::write(&path, "[font]\nsize = 14.0\n").unwrap();

        let (tx, rx) = mpsc::channel::<(Config, ConfigDelta)>();
        let _watcher = ConfigWatcher::new(&path, Config::default(), move |cfg, delta| {
            let _ = tx.send((cfg, delta));
        })
        .unwrap();

        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(&path, "[font]\nsize = 20.0\n").unwrap();

        let (_config, delta) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(delta.font_changed);
        assert!(!delta.colors_changed);
    }

    #[test]
    fn user_event_can_carry_config_and_delta() {
        let config = Config::default();
        let delta = config.diff(&config);
        let event = UserEvent::ConfigReloaded(config.clone(), delta);
        match event {
            UserEvent::ConfigReloaded(c, d) => {
                assert_eq!(c, config);
                assert!(d.is_empty());
            }
            UserEvent::QuickTerminalToggle => {
                panic!("Expected ConfigReloaded, got QuickTerminalToggle");
            }
        }
    }
}
