use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

const VALID_THEMES: &[&str] = &["claude_dark", "claude_light", "claude_warm"];
const VALID_CURSOR_STYLES: &[&str] = &["block", "beam", "underline"];

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub font: FontConfig,
    pub colors: ColorsConfig,
    pub keys: KeysConfig,
    pub cursor: CursorConfig,
    pub scrollback: ScrollbackConfig,
    pub performance: PerformanceConfig,
}

/// Font configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct FontConfig {
    pub family: String,
    pub size: f64,
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
    colors: RawColorsConfig,
    keys: RawKeysConfig,
    cursor: RawCursorConfig,
    scrollback: RawScrollbackConfig,
    performance: RawPerformanceConfig,
}

#[derive(Deserialize)]
#[serde(default)]
struct RawFontConfig {
    family: String,
    size: f64,
}

impl Default for RawFontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 14.0,
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
            theme: "claude_dark".to_string(),
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
}

impl Default for RawCursorConfig {
    fn default() -> Self {
        Self {
            style: "block".to_string(),
            blink: true,
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

// ── Default impls ───────────────────────────────────────────────────────

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 14.0,
        }
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            theme: "claude_dark".to_string(),
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: "block".to_string(),
            blink: true,
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
            },
            scrollback: ScrollbackConfig {
                lines: raw.scrollback.lines,
            },
            performance: PerformanceConfig {
                fps_limit: raw.performance.fps_limit,
            },
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate the config, returning an error if any values are out of range.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.font.size <= 0.0 {
            return Err(ConfigError::Validation("font size must be > 0".to_string()));
        }

        if !VALID_THEMES.contains(&self.colors.theme.as_str()) {
            return Err(ConfigError::Validation(format!(
                "unknown theme '{}', valid themes: {}",
                self.colors.theme,
                VALID_THEMES.join(", ")
            )));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ── Default tests ───────────────────────────────────────────────

    #[test]
    fn default_font_size() {
        let config = Config::default();
        assert_eq!(config.font.size, 14.0);
    }

    #[test]
    fn default_font_family() {
        let config = Config::default();
        assert_eq!(config.font.family, "monospace");
    }

    #[test]
    fn default_theme() {
        let config = Config::default();
        assert_eq!(config.colors.theme, "claude_dark");
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

    // ── TOML parsing tests ──────────────────────────────────────────

    #[test]
    fn parse_complete_toml() {
        let toml = r#"
[font]
family = "JetBrains Mono"
size = 16.0

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
        assert_eq!(config.font.size, 16.0);
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
size = 18.0
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.font.size, 18.0);
        assert_eq!(config.font.family, "monospace");
        assert_eq!(config.colors.theme, "claude_dark");
        assert_eq!(config.cursor.style, "block");
        assert!(config.cursor.blink);
        assert_eq!(config.scrollback.lines, 10_000);
        assert_eq!(config.performance.fps_limit, 60);
    }

    #[test]
    fn parse_empty_toml_uses_all_defaults() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.font.family, "monospace");
        assert_eq!(config.colors.theme, "claude_dark");
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
    fn invalid_zero_font_size() {
        let toml = r#"
[font]
size = 0.0
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
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
        assert_eq!(config.colors.theme, "claude_dark");
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let path = Path::new("/tmp/nonexistent_veloterm_config_test.toml");
        let config = Config::load(path).unwrap();
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.colors.theme, "claude_dark");
    }

    // ── ConfigError display test ────────────────────────────────────

    #[test]
    fn config_error_display() {
        let err = ConfigError::Validation("font size must be > 0".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("font size must be > 0"));
    }
}
