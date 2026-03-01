<!-- ARCHITECT CONTEXT | Track: 15_font_padding | Wave: 6 | CC: v2 -->

## Cross-Cutting Constraints
- Performance Budget: glyph atlas rebuild on font change must complete in <200ms; no frame drops during normal rendering
- Configuration Management: font family, font size, line-height, padding all configurable via `veloterm.toml` with hot-reload
- Testing: TDD for padding calculations, font metrics extraction, atlas rebuild logic
- UI Reference Compliance: all visual output must match the reference mockup aesthetic

## Interfaces

### Owns
- Font loading and fallback chain resolution
- Glyph atlas rebuild on font/size change
- Terminal content padding/margin system
- Runtime font size adjustment (Cmd+Plus/Cmd+Minus)

### Consumes
- `Config` (Track 03) — font.family, font.size, font.line_height, padding settings
- `GlyphAtlas` (Track 01) — existing atlas infrastructure to extend
- `GridRenderer` (Track 01) — cell size and grid positioning

## Dependencies
- Track 01_window_gpu_pipeline: GlyphAtlas and GridRenderer infrastructure
- Track 03_config: font and padding configuration fields

<!-- END ARCHITECT CONTEXT -->

# Track 15: Font Rendering & Terminal Padding — Specification

## Overview

This track delivers professional-grade font rendering with a multi-font strategy matching the Anthropic/Claude brand aesthetic, configurable terminal padding/margins, and runtime font size adjustment. The terminal content uses JetBrains Mono (bundled) with SF Mono fallback. UI chrome (tab bar, title bar, menus) uses Styrene B → Inter → SF Pro. Display/header text uses Galaxie Copernicus → Georgia.

Terminal content is rendered inside a padded container inset using GPU scissor rects, so text is never flush against window edges. Font size is adjustable at runtime via Cmd+Plus/Cmd+Minus with ~10% percentage steps.

## UI Reference

All visual output MUST match the reference implementation:
- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

Key reference values: 13px monospace font, 1.5x line-height, 16px content padding, warm dark background `#181715`.

## Functional Requirements

### FR-1: Multi-Font Strategy

Three font categories with fallback chains:

| Context | Primary | Fallback 1 | Fallback 2 | Fallback 3 |
|---------|---------|-----------|-----------|-----------|
| Terminal content | JetBrains Mono (bundled) | SF Mono | Menlo | System monospace |
| UI chrome (tab bar, menus, status) | Styrene B | Inter | SF Pro | System sans-serif |
| Display/headers (welcome, about) | Galaxie Copernicus | Georgia | System serif | — |

- JetBrains Mono MUST be embedded in the binary as a compiled-in resource (~300KB)
- UI and display fonts are resolved via cosmic-text's system font database at startup
- If no font in a fallback chain is found, use the system default for that category
- The terminal content font family is configurable via `font.family` in `veloterm.toml`
- The UI chrome and display fonts are configurable via `font.ui_family` and `font.display_family`

### FR-2: Font Metrics & Rendering

- Default font size: 13px (matching reference)
- Default line-height: 1.5x font size (matching reference)
- Line-height configurable via `font.line_height` (float multiplier, e.g., 1.5)
- Letter-spacing: determined by monospace cell width (advance width of 'M' glyph)
- Anti-aliased rendering via cosmic-text's SwashCache (existing infrastructure)
- Cell dimensions (width, height) recalculated when font or size changes
- Glyph atlas uses the terminal content font; UI text rendered separately

### FR-3: Terminal Padding

- Configurable padding on all four sides via `padding.top`, `padding.bottom`, `padding.left`, `padding.right` in config
- Default: 12px on all sides (minimum recommended)
- Implementation: container inset with GPU scissor rect
  - The terminal grid renders into a sub-region of the pane area, inset by the padding values
  - A scissor rect clips rendering to the content area, preventing overflow into padding
  - The padding area fills with the terminal background color
  - Renderers (grid, cursor, selection, scrollbar) operate in content-area coordinates — they do NOT need to know about padding
- Mouse coordinate translation: window coordinates → subtract padding offset → content-area cell coordinates
- Padding applies per-pane (each pane has its own padded content area)

### FR-4: Runtime Font Size Adjustment

- Cmd+Plus (or Cmd+=): increase font size by ~10% (round to nearest integer pixel)
- Cmd+Minus: decrease font size by ~10% (round to nearest integer pixel, minimum 8px)
- Cmd+0: reset to configured default size
- On size change:
  1. Recalculate cell dimensions from new font metrics
  2. Invalidate and rebuild glyph atlas (clear all cached glyphs, re-rasterize on demand)
  3. Recalculate terminal grid dimensions (cols/rows) from new cell size and window size
  4. Resize terminal and PTY to new dimensions
  5. Trigger full redraw
- Atlas rebuild must complete in <200ms
- Ctrl+Plus/Minus on Linux (platform-aware keybinding)

### FR-5: Config Hot-Reload

- All font and padding settings are hot-reloadable via the existing config watcher (Track 03)
- On config change affecting font: rebuild glyph atlas, recalculate grid, resize terminal
- On config change affecting padding: recalculate content area inset, trigger redraw
- Invalid font names fall back to next font in the chain with a logged warning

## Non-Functional Requirements

- Glyph atlas rebuild on font/size change: <200ms
- No frame drops during normal rendering with new font system
- Startup time impact from font loading: <50ms additional
- Binary size increase from bundled JetBrains Mono: ~300KB
- Memory: glyph atlas size proportional to font size (512px minimum atlas, scale with DPI)

## Acceptance Criteria

1. VeloTerm renders terminal content in JetBrains Mono at 13px with 1.5x line-height by default
2. Terminal text has 12px padding on all sides — text is NOT flush against pane edges
3. Cmd+Plus increases font size by ~10%, Cmd+Minus decreases, Cmd+0 resets
4. Font size change triggers atlas rebuild and terminal resize in <200ms
5. UI chrome text (tab bar, status indicators) renders in Inter/SF Pro (or system sans-serif)
6. Padding values are configurable via `veloterm.toml` and hot-reloadable
7. Font family is configurable via `veloterm.toml` and hot-reloadable
8. Mouse click-to-cell mapping accounts for padding offsets correctly
9. All existing tests continue to pass after font/padding changes

## Out of Scope

- Ligature support (future track)
- Custom font loading from arbitrary file paths (use system fonts + bundled)
- Per-pane font settings (all panes share the same font)
- Font rendering for non-Latin scripts beyond basic Unicode (future enhancement)
- Bold/italic font variants (use synthetic bold/italic from cosmic-text)
