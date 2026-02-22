use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

const VALID_THEMES: &[&str] = &[
    "warm_dark", "midnight", "ember", "dusk", "light",
    // Legacy aliases (backward compat)
    "claude_dark", "claude_light", "claude_warm",
];
const VALID_CURSOR_STYLES: &[&str] = &["block", "beam", "underline"];

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub font: FontConfig,
    pub padding: PaddingConfig,
    pub colors: ColorsConfig,
    pub keys: KeysConfig,
    pub cursor: CursorConfig,
    pub scrollback: ScrollbackConfig,
    pub performance: PerformanceConfig,
    pub links: LinksConfig,
    pub shell: ShellConfig,
    pub vi_mode: ViModeConfig,
    pub quick_terminal: QuickTerminalConfig,
    pub session: SessionConfig,
    pub sidebar: SidebarConfig,
}

/// Font configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct FontConfig {
    pub family: String,
    pub size: f64,
    /// Line-height as a multiplier of font size (e.g., 1.5 = 150%).
    pub line_height: f64,
    /// UI chrome font family (tab bar, menus, status).
    pub ui_family: String,
    /// Display/header font family (welcome screen, about dialog).
    pub display_family: String,
}

/// Terminal content padding in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct PaddingConfig {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

/// Colors/theme configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorsConfig {
    pub theme: String,
}

/// Keybinding configuration — string key combos mapped to action names.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeysConfig {
    pub bindings: HashMap<String, String>,
}

/// Cursor configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorConfig {
    pub style: String,
    pub blink: bool,
    /// Blink rate in milliseconds. 0 = disable blinking.
    pub blink_rate: u64,
}

/// Scrollback configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollbackConfig {
    pub lines: u32,
}

/// Performance configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceConfig {
    pub fps_limit: u32,
}

/// Link detection configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LinksConfig {
    pub enabled: bool,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Shell integration configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellConfig {
    /// Master toggle for shell integration features.
    pub integration_enabled: bool,
    /// Minimum command duration (seconds) to trigger long-running notification.
    pub notification_threshold_secs: u64,
    /// Enable visual bell (brief flash on BEL character).
    pub bell_enabled: bool,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            integration_enabled: true,
            notification_threshold_secs: 10,
            bell_enabled: true,
        }
    }
}

/// Vi-mode configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ViModeConfig {
    /// Whether vi-mode is available.
    pub enabled: bool,
    /// Keybinding to toggle vi-mode (e.g., "ctrl+shift+space").
    pub entry_key: String,
}

impl Default for ViModeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            entry_key: "ctrl+shift+space".to_string(),
        }
    }
}

/// Quick terminal (global hotkey toggle) configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct QuickTerminalConfig {
    /// Enable the quick terminal global hotkey.
    pub enabled: bool,
    /// Global hotkey string (e.g., "Control+`").
    pub hotkey: String,
}

impl Default for QuickTerminalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hotkey: "Control+`".to_string(),
        }
    }
}

/// Session persistence configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    /// Automatically restore the previous session on startup.
    pub auto_restore: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_restore: false,
        }
    }
}

/// Sidebar configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarConfig {
    /// Whether the sidebar is visible by default on startup.
    pub default_visible: bool,
    /// Sidebar width in logical pixels.
    pub width: f32,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            default_visible: true,
            width: 200.0,
        }
    }
}

/// Errors that can occur during config loading and validation.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("validation error: {0}")]
    Validation(String),
}

// ── Serde intermediate structs (allow unknown keys via flatten + deny) ───

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawConfig {
    font: RawFontConfig,
    padding: RawPaddingConfig,
    colors: RawColorsConfig,
    keys: RawKeysConfig,
    cursor: RawCursorConfig,
    scrollback: RawScrollbackConfig,
    performance: RawPerformanceConfig,
    links: RawLinksConfig,
    shell: RawShellConfig,
    vi_mode: RawViModeConfig,
    quick_terminal: RawQuickTerminalConfig,
    session: RawSessionConfig,
    sidebar: RawSidebarConfig,
}

#[derive(Deserialize)]
#[serde(default)]
struct RawFontConfig {
    family: String,
    size: f64,
    line_height: f64,
    ui_family: String,
    display_family: String,
}

impl Default for RawFontConfig {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            size: 18.0,
            line_height: 1.6,
            ui_family: "Inter".to_string(),
            display_family: "Georgia".to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawPaddingConfig {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
}

impl Default for RawPaddingConfig {
    fn default() -> Self {
        Self {
            top: 16.0,
            bottom: 16.0,
            left: 22.0,
            right: 22.0,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawColorsConfig {
    theme: String,
}

impl Default for RawColorsConfig {
    fn default() -> Self {
        Self {
            theme: "warm_dark".to_string(),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawKeysConfig {
    #[serde(flatten)]
    bindings: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(default)]
struct RawCursorConfig {
    style: String,
    blink: bool,
    blink_rate: u64,
}

impl Default for RawCursorConfig {
    fn default() -> Self {
        Self {
            style: "block".to_string(),
            blink: true,
            blink_rate: 500,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawScrollbackConfig {
    lines: u32,
}

impl Default for RawScrollbackConfig {
    fn default() -> Self {
        Self { lines: 10_000 }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawPerformanceConfig {
    fps_limit: u32,
}

impl Default for RawPerformanceConfig {
    fn default() -> Self {
        Self { fps_limit: 60 }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawLinksConfig {
    enabled: bool,
}

impl Default for RawLinksConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawShellConfig {
    integration_enabled: bool,
    notification_threshold_secs: u64,
    bell_enabled: bool,
}

impl Default for RawShellConfig {
    fn default() -> Self {
        Self {
            integration_enabled: true,
            notification_threshold_secs: 10,
            bell_enabled: true,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawViModeConfig {
    enabled: bool,
    entry_key: String,
}

impl Default for RawViModeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            entry_key: "ctrl+shift+space".to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawQuickTerminalConfig {
    enabled: bool,
    hotkey: String,
}

impl Default for RawQuickTerminalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hotkey: "Control+`".to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawSessionConfig {
    auto_restore: bool,
}

impl Default for RawSessionConfig {
    fn default() -> Self {
        Self {
            auto_restore: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct RawSidebarConfig {
    default_visible: bool,
    width: f32,
}

impl Default for RawSidebarConfig {
    fn default() -> Self {
        Self {
            default_visible: true,
            width: 200.0,
        }
    }
}

// ── Default impls ───────────────────────────────────────────────────────

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            size: 18.0,
            line_height: 1.6,
            ui_family: "Inter".to_string(),
            display_family: "Georgia".to_string(),
        }
    }
}

impl Default for PaddingConfig {
    fn default() -> Self {
        Self {
            top: 16.0,
            bottom: 16.0,
            left: 22.0,
            right: 22.0,
        }
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            theme: "warm_dark".to_string(),
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: "block".to_string(),
            blink: true,
            blink_rate: 500,
        }
    }
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self { lines: 10_000 }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self { fps_limit: 60 }
    }
}

// ── Config implementation ───────────────────────────────────────────────

impl Config {
    /// Load config from a TOML file path. Returns defaults if file does not exist.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Self::from_toml(&contents),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("No config file at {}, using defaults", path.display());
                Ok(Self::default())
            }
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    /// Parse a TOML string into a Config.
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig =
            toml::from_str(toml_str).map_err(|e| ConfigError::Parse(e.to_string()))?;

        let config = Self {
            font: FontConfig {
                family: raw.font.family,
                size: raw.font.size,
                line_height: raw.font.line_height,
                ui_family: raw.font.ui_family,
                display_family: raw.font.display_family,
            },
            padding: PaddingConfig {
                top: raw.padding.top,
                bottom: raw.padding.bottom,
                left: raw.padding.left,
                right: raw.padding.right,
            },
            colors: ColorsConfig {
                theme: raw.colors.theme,
            },
            keys: KeysConfig {
                bindings: raw.keys.bindings,
            },
            cursor: CursorConfig {
                style: raw.cursor.style,
                blink: raw.cursor.blink,
                blink_rate: raw.cursor.blink_rate,
            },
            scrollback: ScrollbackConfig {
                lines: raw.scrollback.lines,
            },
            performance: PerformanceConfig {
                fps_limit: raw.performance.fps_limit,
            },
            links: LinksConfig {
                enabled: raw.links.enabled,
            },
            shell: ShellConfig {
                integration_enabled: raw.shell.integration_enabled,
                notification_threshold_secs: raw.shell.notification_threshold_secs,
                bell_enabled: raw.shell.bell_enabled,
            },
            vi_mode: ViModeConfig {
                enabled: raw.vi_mode.enabled,
                entry_key: raw.vi_mode.entry_key,
            },
            quick_terminal: QuickTerminalConfig {
                enabled: raw.quick_terminal.enabled,
                hotkey: raw.quick_terminal.hotkey,
            },
            session: SessionConfig {
                auto_restore: raw.session.auto_restore,
            },
            sidebar: SidebarConfig {
                default_visible: raw.sidebar.default_visible,
                width: raw.sidebar.width,
            },
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate the config, returning an error if any values are out of range.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.font.size < 8.0 {
            return Err(ConfigError::Validation(
                "font size must be >= 8".to_string(),
            ));
        }

        if self.font.line_height < 0.5 || self.font.line_height > 3.0 {
            return Err(ConfigError::Validation(
                "font line_height must be between 0.5 and 3.0".to_string(),
            ));
        }

        if self.padding.top < 0.0
            || self.padding.bottom < 0.0
            || self.padding.left < 0.0
            || self.padding.right < 0.0
        {
            return Err(ConfigError::Validation(
                "padding values must be >= 0".to_string(),
            ));
        }

        if !VALID_THEMES.contains(&self.colors.theme.as_str()) {
            return Err(ConfigError::Validation(format!(
                "unknown theme '{}', valid themes: {}",
                self.colors.theme,
                VALID_THEMES.join(", ")
            )));
        }

        if self.cursor.blink_rate != 0
            && (self.cursor.blink_rate < 100 || self.cursor.blink_rate > 2000)
        {
            return Err(ConfigError::Validation(
                "cursor blink_rate must be 0 (disabled) or between 100 and 2000 ms".to_string(),
            ));
        }

        if !VALID_CURSOR_STYLES.contains(&self.cursor.style.as_str()) {
            return Err(ConfigError::Validation(format!(
                "unknown cursor style '{}', valid styles: {}",
                self.cursor.style,
                VALID_CURSOR_STYLES.join(", ")
            )));
        }

        if self.scrollback.lines == 0 {
            return Err(ConfigError::Validation(
                "scrollback lines must be > 0".to_string(),
            ));
        }

        if self.performance.fps_limit == 0 {
            return Err(ConfigError::Validation("fps_limit must be > 0".to_string()));
        }

        Ok(())
    }

    /// Compare two configs and return a delta indicating which sections changed.
    pub fn diff(&self, other: &Config) -> ConfigDelta {
        ConfigDelta {
            font_changed: self.font != other.font,
            padding_changed: self.padding != other.padding,
            colors_changed: self.colors != other.colors,
            keys_changed: self.keys != other.keys,
            cursor_changed: self.cursor != other.cursor,
            scrollback_changed: self.scrollback != other.scrollback,
            performance_changed: self.performance != other.performance,
            links_changed: self.links != other.links,
            shell_changed: self.shell != other.shell,
            vi_mode_changed: self.vi_mode != other.vi_mode,
            quick_terminal_changed: self.quick_terminal != other.quick_terminal,
            session_changed: self.session != other.session,
            sidebar_changed: self.sidebar != other.sidebar,
        }
    }

    /// Generate a fully commented default config as a TOML string.
    pub fn print_default() -> String {
        r#"# VeloTerm Configuration
# Place this file at ~/.config/veloterm/veloterm.toml

[font]
# Terminal content font family (with fallback: JetBrains Mono -> SF Mono -> Menlo -> system)
family = "JetBrains Mono"
# Font size in points
size = 18.0
# Line-height multiplier (1.6 = 160% of font size)
line_height = 1.6
# UI chrome font (tab bar, menus, status)
ui_family = "Inter"
# Display/header font (welcome screen, about)
display_family = "Georgia"

[padding]
# Terminal content padding in pixels
top = 16.0
bottom = 16.0
left = 22.0
right = 22.0

[colors]
# Theme: "warm_dark", "midnight", "ember", "dusk", or "light"
theme = "warm_dark"

[cursor]
# Cursor style: "block", "beam", or "underline"
style = "block"
# Enable cursor blinking
blink = true
# Blink rate in milliseconds (0 = disable, 100-2000)
blink_rate = 500

[scrollback]
# Number of lines to keep in scrollback history
lines = 10000

[performance]
# Maximum frames per second
fps_limit = 60

[shell]
# Enable shell integration features (prompt detection, CWD tracking, command timing)
integration_enabled = true
# Minimum command duration (seconds) to trigger long-running notification
notification_threshold_secs = 10
# Enable visual bell (brief flash on BEL character)
bell_enabled = true

[vi_mode]
# Enable vi-mode for keyboard-driven scrollback navigation
enabled = true
# Keybinding to toggle vi-mode
entry_key = "ctrl+shift+space"

[quick_terminal]
# Enable global hotkey to toggle window visibility
enabled = false
# Global hotkey (e.g., "Control+`", "Alt+Space")
hotkey = "Control+`"

[session]
# Automatically restore the previous session on startup
auto_restore = false

[sidebar]
# Show sidebar by default on startup
default_visible = true
# Sidebar width in logical pixels
width = 200.0

# [keys]
# Keybindings as "key_combo" = "action" pairs
# Example:
# "ctrl+shift+c" = "copy"
# "ctrl+shift+v" = "paste"
"#
        .to_string()
    }
}

/// Indicates which config sections changed between two Config instances.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDelta {
    pub font_changed: bool,
    pub padding_changed: bool,
    pub colors_changed: bool,
    pub keys_changed: bool,
    pub cursor_changed: bool,
    pub scrollback_changed: bool,
    pub performance_changed: bool,
    pub links_changed: bool,
    pub shell_changed: bool,
    pub vi_mode_changed: bool,
    pub quick_terminal_changed: bool,
    pub session_changed: bool,
    pub sidebar_changed: bool,
}

impl ConfigDelta {
    /// Returns true if no sections changed.
    pub fn is_empty(&self) -> bool {
        !self.font_changed
            && !self.padding_changed
            && !self.colors_changed
            && !self.keys_changed
            && !self.cursor_changed
            && !self.scrollback_changed
            && !self.performance_changed
            && !self.links_changed
            && !self.shell_changed
            && !self.vi_mode_changed
            && !self.quick_terminal_changed
            && !self.session_changed
            && !self.sidebar_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ── Default tests ───────────────────────────────────────────────

    #[test]
    fn default_font_size() {
        let config = Config::default();
        assert_eq!(config.font.size, 18.0);
    }

    #[test]
    fn default_font_family() {
        let config = Config::default();
        assert_eq!(config.font.family, "JetBrains Mono");
    }

    #[test]
    fn default_font_line_height() {
        let config = Config::default();
        assert_eq!(config.font.line_height, 1.6);
    }

    #[test]
    fn default_font_ui_family() {
        let config = Config::default();
        assert_eq!(config.font.ui_family, "Inter");
    }

    #[test]
    fn default_font_display_family() {
        let config = Config::default();
        assert_eq!(config.font.display_family, "Georgia");
    }

    #[test]
    fn default_padding() {
        let config = Config::default();
        assert_eq!(config.padding.top, 16.0);
        assert_eq!(config.padding.bottom, 16.0);
        assert_eq!(config.padding.left, 22.0);
        assert_eq!(config.padding.right, 22.0);
    }

    #[test]
    fn default_theme() {
        let config = Config::default();
        assert_eq!(config.colors.theme, "warm_dark");
    }

    #[test]
    fn default_cursor_style() {
        let config = Config::default();
        assert_eq!(config.cursor.style, "block");
    }

    #[test]
    fn default_cursor_blink() {
        let config = Config::default();
        assert!(config.cursor.blink);
    }

    #[test]
    fn default_scrollback_lines() {
        let config = Config::default();
        assert_eq!(config.scrollback.lines, 10_000);
    }

    #[test]
    fn default_fps_limit() {
        let config = Config::default();
        assert_eq!(config.performance.fps_limit, 60);
    }

    #[test]
    fn default_keys_empty() {
        let config = Config::default();
        assert!(config.keys.bindings.is_empty());
    }

    #[test]
    fn default_links_enabled() {
        let config = Config::default();
        assert!(config.links.enabled);
    }

    // ── TOML parsing tests ──────────────────────────────────────────

    #[test]
    fn parse_complete_toml() {
        let toml = r#"
[font]
family = "JetBrains Mono"
size = 14.0

[colors]
theme = "claude_light"

[keys]
"ctrl+shift+c" = "copy"
"ctrl+shift+v" = "paste"

[cursor]
style = "beam"
blink = false

[scrollback]
lines = 5000

[performance]
fps_limit = 120
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.colors.theme, "claude_light");
        assert_eq!(config.cursor.style, "beam");
        assert!(!config.cursor.blink);
        assert_eq!(config.scrollback.lines, 5000);
        assert_eq!(config.performance.fps_limit, 120);
        assert_eq!(config.keys.bindings.get("ctrl+shift+c").unwrap(), "copy");
        assert_eq!(config.keys.bindings.get("ctrl+shift+v").unwrap(), "paste");
    }

    #[test]
    fn parse_partial_toml_uses_defaults() {
        let toml = r#"
[font]
size = 14.0
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.line_height, 1.6);
        assert_eq!(config.colors.theme, "warm_dark");
        assert_eq!(config.cursor.style, "block");
        assert!(config.cursor.blink);
        assert_eq!(config.scrollback.lines, 10_000);
        assert_eq!(config.performance.fps_limit, 60);
        assert_eq!(config.padding.top, 16.0);
    }

    #[test]
    fn parse_empty_toml_uses_all_defaults() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config.font.size, 18.0);
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.line_height, 1.6);
        assert_eq!(config.colors.theme, "warm_dark");
        assert_eq!(config.padding.top, 16.0);
    }

    #[test]
    fn parse_links_config() {
        let toml = r#"
[links]
enabled = false
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.links.enabled);
    }

    #[test]
    fn parse_links_default_enabled() {
        let config = Config::from_toml("").unwrap();
        assert!(config.links.enabled);
    }

    // ── Shell config tests ────────────────────────────────────────

    #[test]
    fn default_shell_integration_enabled() {
        let config = Config::default();
        assert!(config.shell.integration_enabled);
    }

    #[test]
    fn default_shell_notification_threshold() {
        let config = Config::default();
        assert_eq!(config.shell.notification_threshold_secs, 10);
    }

    #[test]
    fn parse_shell_config() {
        let toml = r#"
[shell]
integration_enabled = false
notification_threshold_secs = 30
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.shell.integration_enabled);
        assert_eq!(config.shell.notification_threshold_secs, 30);
    }

    #[test]
    fn parse_shell_config_defaults() {
        let config = Config::from_toml("").unwrap();
        assert!(config.shell.integration_enabled);
        assert_eq!(config.shell.notification_threshold_secs, 10);
    }

    #[test]
    fn default_bell_enabled() {
        let config = Config::default();
        assert!(config.shell.bell_enabled);
    }

    #[test]
    fn parse_bell_disabled() {
        let toml = r#"
[shell]
bell_enabled = false
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.shell.bell_enabled);
    }

    #[test]
    fn diff_detects_shell_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.shell.notification_threshold_secs = 30;
        let delta = a.diff(&b);
        assert!(delta.shell_changed);
    }

    // ── Vi-mode config tests ──────────────────────────────────────

    #[test]
    fn default_vi_mode_enabled() {
        let config = Config::default();
        assert!(config.vi_mode.enabled);
    }

    #[test]
    fn default_vi_mode_entry_key() {
        let config = Config::default();
        assert_eq!(config.vi_mode.entry_key, "ctrl+shift+space");
    }

    #[test]
    fn parse_vi_mode_config() {
        let toml = r#"
[vi_mode]
enabled = false
entry_key = "ctrl+space"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.vi_mode.enabled);
        assert_eq!(config.vi_mode.entry_key, "ctrl+space");
    }

    #[test]
    fn parse_vi_mode_config_defaults() {
        let config = Config::from_toml("").unwrap();
        assert!(config.vi_mode.enabled);
        assert_eq!(config.vi_mode.entry_key, "ctrl+shift+space");
    }

    #[test]
    fn diff_detects_vi_mode_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.vi_mode.enabled = false;
        let delta = a.diff(&b);
        assert!(delta.vi_mode_changed);
    }

    // ── Padding & line_height config tests ────────────────────────

    #[test]
    fn parse_padding_config() {
        let toml = r#"
[padding]
top = 20.0
bottom = 12.0
left = 24.0
right = 24.0
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.padding.top, 20.0);
        assert_eq!(config.padding.bottom, 12.0);
        assert_eq!(config.padding.left, 24.0);
        assert_eq!(config.padding.right, 24.0);
    }

    #[test]
    fn parse_font_line_height() {
        let toml = r#"
[font]
line_height = 1.8
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.font.line_height, 1.8);
    }

    #[test]
    fn parse_font_ui_and_display_families() {
        let toml = r#"
[font]
ui_family = "SF Pro"
display_family = "Galaxie Copernicus"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.font.ui_family, "SF Pro");
        assert_eq!(config.font.display_family, "Galaxie Copernicus");
    }

    #[test]
    fn diff_detects_padding_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.padding.left = 20.0;
        let delta = a.diff(&b);
        assert!(delta.padding_changed);
        assert!(!delta.font_changed);
    }

    #[test]
    fn diff_detects_line_height_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.font.line_height = 2.0;
        let delta = a.diff(&b);
        assert!(delta.font_changed);
    }

    #[test]
    fn parse_unknown_keys_ignored() {
        let toml = r#"
[font]
size = 14.0
unknown_key = "value"

[unknown_section]
foo = "bar"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.font.size, 14.0);
    }

    // ── Validation tests ────────────────────────────────────────────

    #[test]
    fn invalid_negative_font_size() {
        let toml = r#"
[font]
size = -1.0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_font_size_below_minimum() {
        let toml = r#"
[font]
size = 7.0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn valid_font_size_at_minimum() {
        let toml = r#"
[font]
size = 8.0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_line_height_too_low() {
        let toml = r#"
[font]
line_height = 0.3
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_line_height_too_high() {
        let toml = r#"
[font]
line_height = 3.5
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn valid_line_height_boundary() {
        let toml = r#"
[font]
line_height = 0.5
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_negative_padding() {
        let toml = r#"
[padding]
left = -5.0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn valid_zero_padding() {
        let toml = r#"
[padding]
top = 0.0
bottom = 0.0
left = 0.0
right = 0.0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_theme_name() {
        let toml = r#"
[colors]
theme = "nonexistent_theme"
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_cursor_style() {
        let toml = r#"
[cursor]
style = "crosshair"
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_zero_scrollback() {
        let toml = r#"
[scrollback]
lines = 0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_zero_fps() {
        let toml = r#"
[performance]
fps_limit = 0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    // ── File loading tests ──────────────────────────────────────────

    #[test]
    fn load_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("veloterm.toml");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"[font]\nsize = 20.0\n").unwrap();
        }
        let config = Config::load(&path).unwrap();
        assert_eq!(config.font.size, 20.0);
        assert_eq!(config.colors.theme, "warm_dark");
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let path = Path::new("/tmp/nonexistent_veloterm_config_test.toml");
        let config = Config::load(path).unwrap();
        assert_eq!(config.font.size, 18.0);
        assert_eq!(config.colors.theme, "warm_dark");
    }

    // ── ConfigError display test ────────────────────────────────────

    #[test]
    fn config_error_display() {
        let err = ConfigError::Validation("font size must be > 0".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("font size must be > 0"));
    }

    // ── Config diffing tests ────────────────────────────────────────

    #[test]
    fn diff_identical_configs_is_empty() {
        let a = Config::default();
        let b = Config::default();
        let delta = a.diff(&b);
        assert!(delta.is_empty());
    }

    #[test]
    fn diff_detects_font_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.font.size = 20.0;
        let delta = a.diff(&b);
        assert!(delta.font_changed);
        assert!(!delta.colors_changed);
    }

    #[test]
    fn diff_detects_theme_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.colors.theme = "light".to_string();
        let delta = a.diff(&b);
        assert!(delta.colors_changed);
        assert!(!delta.font_changed);
    }

    #[test]
    fn diff_detects_keybinding_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.keys
            .bindings
            .insert("ctrl+c".to_string(), "copy".to_string());
        let delta = a.diff(&b);
        assert!(delta.keys_changed);
    }

    #[test]
    fn diff_detects_cursor_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.cursor.style = "beam".to_string();
        let delta = a.diff(&b);
        assert!(delta.cursor_changed);
    }

    #[test]
    fn diff_detects_scrollback_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.scrollback.lines = 5000;
        let delta = a.diff(&b);
        assert!(delta.scrollback_changed);
    }

    #[test]
    fn diff_detects_performance_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.performance.fps_limit = 120;
        let delta = a.diff(&b);
        assert!(delta.performance_changed);
    }

    // ── Default config generation tests ─────────────────────────────

    #[test]
    fn print_default_is_valid_toml() {
        let toml_str = Config::print_default();
        // Must parse without error
        let config = Config::from_toml(&toml_str).unwrap();
        assert_eq!(config, Config::default());
    }

    // ── Cursor blink_rate config tests ─────────────────────────────

    #[test]
    fn default_cursor_blink_rate() {
        let config = Config::default();
        assert_eq!(config.cursor.blink_rate, 500);
    }

    #[test]
    fn parse_cursor_blink_rate() {
        let toml = r#"
[cursor]
style = "block"
blink = true
blink_rate = 750
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.cursor.blink_rate, 750);
    }

    #[test]
    fn parse_cursor_blink_rate_default_when_missing() {
        let toml = r#"
[cursor]
style = "block"
blink = true
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.cursor.blink_rate, 500);
    }

    #[test]
    fn invalid_cursor_blink_rate_too_low() {
        let toml = r#"
[cursor]
blink_rate = 50
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_cursor_blink_rate_too_high() {
        let toml = r#"
[cursor]
blink_rate = 3000
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn valid_cursor_blink_rate_zero_disables() {
        let toml = r#"
[cursor]
blink_rate = 0
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.cursor.blink_rate, 0);
    }

    #[test]
    fn valid_cursor_blink_rate_at_minimum() {
        let toml = r#"
[cursor]
blink_rate = 100
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.cursor.blink_rate, 100);
    }

    #[test]
    fn valid_cursor_blink_rate_at_maximum() {
        let toml = r#"
[cursor]
blink_rate = 2000
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.cursor.blink_rate, 2000);
    }

    #[test]
    fn diff_detects_cursor_blink_rate_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.cursor.blink_rate = 750;
        let delta = a.diff(&b);
        assert!(delta.cursor_changed);
    }

    #[test]
    fn print_default_includes_blink_rate() {
        let toml_str = Config::print_default();
        assert!(toml_str.contains("blink_rate"));
    }

    #[test]
    fn print_default_round_trips() {
        let toml_str = Config::print_default();
        let parsed = Config::from_toml(&toml_str).unwrap();
        let default = Config::default();
        assert_eq!(parsed.font, default.font);
        assert_eq!(parsed.colors, default.colors);
        assert_eq!(parsed.cursor, default.cursor);
        assert_eq!(parsed.scrollback, default.scrollback);
        assert_eq!(parsed.performance, default.performance);
    }

    // ── Quick terminal config tests ──────────────────────────────

    #[test]
    fn default_quick_terminal_disabled() {
        let config = Config::default();
        assert!(!config.quick_terminal.enabled);
    }

    #[test]
    fn default_quick_terminal_hotkey() {
        let config = Config::default();
        assert_eq!(config.quick_terminal.hotkey, "Control+`");
    }

    #[test]
    fn parse_quick_terminal_config() {
        let toml = r#"
[quick_terminal]
enabled = true
hotkey = "Alt+Space"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.quick_terminal.enabled);
        assert_eq!(config.quick_terminal.hotkey, "Alt+Space");
    }

    #[test]
    fn parse_quick_terminal_defaults() {
        let config = Config::from_toml("").unwrap();
        assert!(!config.quick_terminal.enabled);
        assert_eq!(config.quick_terminal.hotkey, "Control+`");
    }

    #[test]
    fn diff_detects_quick_terminal_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.quick_terminal.enabled = true;
        let delta = a.diff(&b);
        assert!(delta.quick_terminal_changed);
    }

    // ── Session config tests ──────────────────────────────────────

    #[test]
    fn default_session_auto_restore_disabled() {
        let config = Config::default();
        assert!(!config.session.auto_restore);
    }

    #[test]
    fn parse_session_auto_restore() {
        let toml = r#"
[session]
auto_restore = true
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.session.auto_restore);
    }

    #[test]
    fn diff_detects_session_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.session.auto_restore = true;
        let delta = a.diff(&b);
        assert!(delta.session_changed);
    }

    // ── Sidebar config tests ─────────────────────────────────────

    #[test]
    fn default_sidebar_config() {
        let config = Config::default();
        assert!(config.sidebar.default_visible);
        assert_eq!(config.sidebar.width, 200.0);
    }

    #[test]
    fn parse_sidebar_config_from_toml() {
        let toml = r#"
[sidebar]
default_visible = true
width = 250.0
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.sidebar.default_visible);
        assert_eq!(config.sidebar.width, 250.0);
    }

    #[test]
    fn parse_sidebar_config_defaults() {
        let config = Config::from_toml("").unwrap();
        assert!(config.sidebar.default_visible);
        assert_eq!(config.sidebar.width, 200.0);
    }

    #[test]
    fn diff_detects_sidebar_width_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.sidebar.width = 250.0;
        let delta = a.diff(&b);
        assert!(delta.sidebar_changed);
    }

    #[test]
    fn diff_detects_sidebar_visible_change() {
        let a = Config::default();
        let mut b = Config::default();
        b.sidebar.default_visible = false;
        let delta = a.diff(&b);
        assert!(delta.sidebar_changed);
    }
}
