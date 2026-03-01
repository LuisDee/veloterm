# Spec: Configuration & Theming

## Overview

A complete TOML-based configuration system with hot-reload that controls all user-facing settings — fonts, colors, keybindings, scrollback size, cursor style, and performance tuning. Changes to the config file are detected and applied automatically without restarting the terminal. The existing hardcoded defaults in the codebase become the fallback values when no config file exists.

## Design Decisions

1. **Config file structure**: Flat top-level sections (`[font]`, `[colors]`, `[keys]`, `[cursor]`, `[scrollback]`, `[performance]`). A future `[overrides.<pane-name>]` layer will be added when Track 04 (Pane Layout) lands — no nested hierarchy now.

2. **Hot-reload scope**: Everything hot-reloads without restart. Font changes trigger glyph atlas rebuild (brief flicker is acceptable). Color changes update GPU uniforms. Keybinding, cursor, scrollback, and performance changes apply immediately.

3. **Keybinding format**: Simple string-mapping pairs (`"ctrl+shift+d" = "split_vertical"`). Vi-mode (Track 11) will add a `[keys.vi_mode]` sub-table later.

4. **Theme system**: Named themes only — `theme = "claude_dark"` (or `claude_light`, `claude_warm`). No individual color overrides. Clean theme switching.

5. **Config location**: `~/.config/veloterm/veloterm.toml` on all platforms (XDG convention). Respects `$XDG_CONFIG_HOME` if set.

6. **Config validation**: Validate on load only. If hot-reload detects a malformed file, keep the previous valid config and log a warning. No continuous schema validation.

## Functional Requirements

### FR-1: Config File Parsing
- Parse `~/.config/veloterm/veloterm.toml` using `toml` + `serde` deserialization into a typed `Config` struct.
- All fields are optional with sensible defaults — the app must work without a config file.
- Respect `$XDG_CONFIG_HOME` environment variable (default `~/.config`).
- Unknown keys are ignored with a logged warning (forward-compatible).

### FR-2: Config Sections
- **`[font]`**: `family` (String), `size` (f64, default 14.0)
- **`[colors]`**: `theme` (String enum: `"claude_dark"` | `"claude_light"` | `"claude_warm"`, default `"claude_dark"`)
- **`[keys]`**: String key-action pairs (e.g., `"ctrl+shift+c" = "copy"`)
- **`[cursor]`**: `style` (String enum: `"block"` | `"beam"` | `"underline"`, default `"block"`), `blink` (bool, default true)
- **`[scrollback]`**: `lines` (u32, default 10000)
- **`[performance]`**: `fps_limit` (u32, default 60)

### FR-3: Hot-Reload
- Watch the config file for changes using the `notify` crate on a background thread.
- On file change: re-parse, validate, diff against current config.
- If valid: apply changes selectively (font change → rebuild atlas, color change → update theme, etc.).
- If invalid: keep previous config, log warning with error details.
- Integrate file watcher notifications with the winit event loop via `EventLoopProxy::send_event()`.

### FR-4: Config Diffing
- `Config::diff(old, new)` returns a `ConfigDelta` indicating which sections changed.
- Consumers (renderer, input handler, terminal) react only to their relevant section changes.

### FR-5: Default Config Generation
- `veloterm --print-default-config` writes a fully commented default config to stdout.
- All default values and available options are documented in the generated output.

### FR-6: Integration Points
- `WindowConfig` in `src/window.rs` derives its values from the `Config` struct.
- Renderer reads theme from `Config` instead of hardcoded `claude_dark()`.
- Input handler reads keybindings from `Config`.
- Cursor renderer reads cursor style/blink from `Config`.
- Terminal module reads scrollback size from `Config`.

## Non-Functional Requirements

- Config load must complete in < 10ms.
- Hot-reload must not drop frames — apply changes between frames.
- Invalid config must never crash the application.

## Acceptance Criteria

1. App launches with no config file — uses all defaults, no errors.
2. App launches with a valid config file — all settings applied.
3. App launches with a partially filled config file — missing fields use defaults.
4. App launches with an invalid config file — logs warning, uses all defaults.
5. Editing the config file while app is running triggers hot-reload within 1 second.
6. Font change hot-reload rebuilds glyph atlas and re-renders.
7. Theme change hot-reload updates all rendered colors.
8. Keybinding change hot-reload updates input handling.
9. `veloterm --print-default-config` outputs valid, parseable TOML.
10. All 3 built-in themes render correctly.

## Out of Scope

- Custom user themes beyond the 3 built-ins (future track)
- Per-pane configuration overrides (future — Track 04 override layer)
- GUI settings editor (config file is the interface)
- Shell integration settings (Track 10)
- Individual color overrides within a theme
