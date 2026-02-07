<!-- ARCHITECT CONTEXT | Track: 03_config | Wave: 1 | CC: v1 -->

## Cross-Cutting Constraints
- Configuration Management: TOML at ~/.config/veloterm/veloterm.toml, hot-reload via notify, strict serde parsing
- Error Handling: thiserror for ConfigError enum, invalid config keeps previous state with logged warning
- Testing: TDD, cargo test --lib, clippy clean, fmt clean

## Interfaces

### Owns
- `Config::load(path)` — parse TOML config file into typed struct
- `Config::watch(path, callback)` — file watcher for hot-reload
- `Config::diff(old, new)` — determine changed sections for selective apply
- `Config::default()` — complete defaults so app works without config file
- Theme system: claude_dark, claude_light, claude_warm built-in themes

### Consumes
- Existing `src/config/theme.rs` (Color, Theme types from Track 1)

## Dependencies
- None (Wave 1 — no new track dependencies)
- Builds on existing `src/config/` module from completed tracks

<!-- END ARCHITECT CONTEXT -->

# Track 03: Configuration & Theming

## What This Track Delivers

A complete TOML-based configuration system with hot-reload that controls all user-facing settings — fonts, colors, keybindings, scrollback size, cursor style, and performance tuning. Changes to the config file are detected and applied automatically without restarting the terminal. The existing hardcoded defaults in the codebase become the fallback values when no config file exists.

## Scope

### IN
- TOML config file parsing with serde deserialization
- Config file structure: font settings, keybindings, colors/theme, scrollback, cursor, performance
- Hot-reload via `notify` crate file watcher
- Config diffing to determine what changed (font change → rebuild atlas, color change → update uniforms)
- Three built-in Claude-themed color schemes (Light, Dark, Warm)
- Custom keybinding definitions
- Default config generation (`veloterm --print-default-config`)
- Error handling for malformed config with actionable messages

### OUT
- Custom user themes beyond the 3 built-ins (Phase 4 — Track 14 Visual Polish)
- Per-pane configuration overrides (not planned)
- GUI settings editor (not planned — config file is the interface)
- Shell integration settings (Track 10 — shell_integration)

## Key Design Decisions

1. **Config file structure**: Flat sections (`[font]`, `[colors]`, `[keys]`) vs nested hierarchy (`[pane.default.font]`)?
   Trade-off: simplicity and discoverability vs future extensibility for per-pane overrides

2. **Hot-reload scope**: Which settings can be hot-reloaded vs which require restart?
   Trade-off: font/atlas changes cause brief flicker; GPU backend cannot change at runtime. Where is the line?

3. **Keybinding format**: Simple string mapping (`"ctrl+shift+d" = "split_vertical"`) vs structured keybinding objects with contexts?
   Trade-off: ease of configuration vs supporting modal keybindings (normal mode vs vi-mode)

4. **Theme system**: Inline color definitions in config vs named theme references (`theme = "claude_dark"`)?
   Trade-off: per-color customization flexibility vs clean theme switching

5. **Config validation timing**: Validate on load only vs continuous validation with schema?
   Trade-off: simplicity vs catching partial writes during hot-reload

6. **Default config location**: `~/.config/veloterm/` (XDG) vs platform-native (`~/Library/Preferences/` on macOS)?
   Trade-off: cross-platform consistency vs platform convention compliance

## Architectural Notes

- The existing `src/config/theme.rs` has `Color` and `Theme` types — extend rather than replace
- Font changes require glyph atlas rebuild (`GlyphAtlas::new()` in renderer) — design the reload callback to handle this
- Keybinding definitions here will be consumed by every future input-handling track (pane splits, tabs, search, vi-mode)
- The `notify` crate file watcher runs on its own thread — integrate with the winit event loop via `EventLoopProxy::send_event()`

## Complexity: M
## Estimated Phases: ~3
