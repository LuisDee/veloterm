# Plan: Configuration & Theming

## Phase 1: Config Struct and TOML Parsing [checkpoint: a9a4326]

- [x] Task: Write tests for config struct and TOML parsing <!-- a194c04 -->
  - Write tests for `Config::default()` producing valid defaults (font size 14.0, theme "claude_dark", cursor block+blink, scrollback 10000, fps 60)
  - Write tests for parsing a complete valid TOML string into `Config`
  - Write tests for parsing a partial TOML (missing sections use defaults)
  - Write tests for parsing an empty TOML file (all defaults)
  - Write tests for unknown keys being ignored without error
  - Write tests for invalid values (e.g., negative font size, unknown theme name) returning `ConfigError`
  - Write tests for `Config::load(path)` reading from disk
  - Write tests for `Config::load(path)` when file does not exist (returns defaults)
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement config struct and TOML parsing <!-- a194c04 -->
  - Add `toml` and `serde` dependencies to `Cargo.toml`
  - Create `src/config/types.rs` with typed `Config`, `FontConfig`, `ColorsConfig`, `KeysConfig`, `CursorConfig`, `ScrollbackConfig`, `PerformanceConfig` structs
  - Implement `serde::Deserialize` with `#[serde(default)]` for all optional fields
  - Implement `Config::default()` with all documented defaults
  - Implement `Config::load(path)` — read file, parse TOML, return typed config or defaults on missing file
  - Create `ConfigError` enum with `thiserror` for parse errors, I/O errors, validation errors
  - Implement validation: font size > 0, theme name in allowed set, scrollback > 0
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'Config Struct and TOML Parsing' (Protocol in workflow.md) <!-- a9a4326 -->

## Phase 2: Config Diffing and Default Generation [checkpoint: 993b50b]

- [x] Task: Write tests for config diffing and default generation <!-- f5bcc50 -->
  - Write tests for `Config::diff(old, new)` detecting font changes
  - Write tests for `Config::diff(old, new)` detecting theme changes
  - Write tests for `Config::diff(old, new)` detecting keybinding changes
  - Write tests for `Config::diff(old, new)` detecting cursor changes
  - Write tests for `Config::diff(old, new)` returning empty delta when configs are identical
  - Write tests for `--print-default-config` output being valid parseable TOML
  - Write tests for the generated default config round-tripping (serialize → parse → equals default)
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement config diffing and default generation <!-- f5bcc50 -->
  - Create `ConfigDelta` struct with boolean flags for each changed section (`font_changed`, `colors_changed`, `keys_changed`, `cursor_changed`, `scrollback_changed`, `performance_changed`)
  - Implement `Config::diff(old, new) -> ConfigDelta` comparing each section
  - Derive `PartialEq` on all config sub-structs for diffing
  - Implement `Config::print_default()` generating fully commented TOML output
  - Wire `--print-default-config` CLI flag via `std::env::args()` in `main.rs`
  - Run tests and confirm they pass (Green phase)

- [x] Task: Conductor - User Manual Verification 'Config Diffing and Default Generation' (Protocol in workflow.md) <!-- 993b50b -->

## Phase 3: Hot-Reload via File Watcher

- [x] Task: Write tests for config file watching and hot-reload <!-- 47e31ac -->
  - Write tests for `ConfigWatcher::new(path)` initialization
  - Write tests for file modification triggering a reload callback
  - Write tests for reload callback receiving the new valid `Config`
  - Write tests for malformed file change keeping previous config and producing a warning
  - Write tests for `ConfigDelta` being sent through the reload callback
  - Write tests for watcher integration with `EventLoopProxy` custom event
  - Run tests and confirm they fail (Red phase)

- [x] Task: Implement config file watching and hot-reload <!-- 47e31ac -->
  - Add `notify` dependency to `Cargo.toml`
  - Create `src/config/watcher.rs` with `ConfigWatcher` struct
  - Spawn a background thread running `notify::recommended_watcher` on the config file
  - On file change: re-parse config, validate, diff against current, send `ConfigDelta` via `EventLoopProxy::send_event()`
  - On parse error: log warning with error details, keep previous config
  - Define a custom `UserEvent::ConfigReloaded(Config, ConfigDelta)` enum variant for the winit event loop
  - Ensure watcher thread shuts down cleanly on drop
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Hot-Reload via File Watcher' (Protocol in workflow.md)

## Phase 4: Integration — Wire Config into Renderer and Input

- [ ] Task: Write tests for config integration with renderer and input
  - Write tests for renderer using theme from config instead of hardcoded `claude_dark()`
  - Write tests for cursor style and blink from config being applied
  - Write tests for scrollback size from config being passed to terminal
  - Write tests for keybinding lookup from config
  - Write tests for font size from config affecting cell dimensions
  - Run tests and confirm they fail (Red phase)

- [ ] Task: Implement config integration with renderer and input
  - Update `main.rs` to load `Config` at startup and pass it through the application
  - Update `WindowConfig` to derive values from `Config`
  - Update renderer initialization to use `Config.colors.theme` for theme selection
  - Update cursor renderer to use `Config.cursor.style` and `Config.cursor.blink`
  - Update terminal initialization to use `Config.scrollback.lines`
  - Update input handler to read keybindings from `Config.keys`
  - Wire `UserEvent::ConfigReloaded` handler in the event loop to apply deltas:
    - Font change → rebuild glyph atlas
    - Color change → update theme/uniforms
    - Cursor change → update cursor style
    - Keys change → update keybinding map
    - Scrollback change → log info (applies on next terminal creation)
  - Run tests and confirm they pass (Green phase)

- [ ] Task: Conductor - User Manual Verification 'Integration — Wire Config into Renderer and Input' (Protocol in workflow.md)
