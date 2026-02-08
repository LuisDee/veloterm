# Track 15: Font Rendering & Terminal Padding — Implementation Plan

## Phase 1: Config & Font Loading Foundation

### Task 1.1: Add padding and font config fields [x] <!-- 87a9e0d -->
- [ ] Add `PaddingConfig` struct to `config/types.rs` with `top`, `bottom`, `left`, `right` (all `f64`, default 12.0)
- [ ] Add `line_height` field to `FontConfig` (f64, default 1.5)
- [ ] Add `ui_family` and `display_family` fields to `FontConfig` (String, with defaults)
- [ ] Add `padding` field to top-level `Config`
- [ ] Update `RawFontConfig` and `RawConfig` for TOML deserialization
- [ ] Update `Config::from_toml()` conversion, `Config::validate()` bounds checks, `Config::diff()` delta detection
- [ ] Add `padding_changed` flag to `ConfigDelta`
- [ ] Write tests: parsing, defaults, validation (min 8px size, padding >= 0, line_height 0.5–3.0)

### Task 1.2: Font family resolution with fallback chain [x] <!-- b986645 -->
- [ ] Modify `GlyphAtlas::new()` to accept font family name and line_height multiplier parameters
- [ ] Implement font family resolution: try configured family → fallback chain → system monospace
- [ ] Bundle JetBrains Mono as `include_bytes!()` compiled-in resource
- [ ] Load bundled font into `FontSystem` via `font_system.db_mut().load_font_data()`
- [ ] Update line_height calculation to use configurable multiplier (default 1.5x, was hardcoded 1.6x)
- [ ] Write tests: fallback chain resolution, bundled font loads, cell dimensions with custom line_height

### Task 1.3: Wire font config through renderer [x] <!-- b986645 -->
- [ ] Update `Renderer::new()` to accept `&Config` (or font family + size + line_height) instead of just `font_size`
- [ ] Pass font family and line_height from config to `GlyphAtlas::new()`
- [ ] Update `window.rs` renderer creation to pass config
- [ ] Ensure existing tests pass with new API (update test helpers)
- [ ] Write integration test: config font.family change triggers atlas rebuild

## Phase 2: Terminal Padding & Scissor Rect

### Task 2.1: Padding-aware grid dimensions [x] <!-- a58f7b8 -->
- [ ] Modify `GridDimensions::from_pane_rect()` to accept padding parameters
- [ ] Subtract padding from available area before calculating columns/rows: `cols = floor((width - left - right) / cell_width)`
- [ ] Store padding in `GridDimensions` for downstream use
- [ ] Update `grid_dims_for_rect()` in `window.rs` to pass padding from config
- [ ] Write tests: padding reduces cols/rows, zero padding = no change, large padding clamps to min 1 col/1 row

### Task 2.2: Scissor rect rendering with padding offset [x] <!-- a58f7b8 -->
- [ ] Update pane rendering in `Renderer::render_frame()` to apply padding offset to scissor rects
- [ ] Each pane's scissor rect is inset by padding: `(x + left, y + top, width - left - right, height - top - bottom)`
- [ ] Padding area fills with terminal background color (clear color handles this)
- [ ] Update cursor rendering to operate in content-area coordinates (no padding knowledge needed)
- [ ] Write tests: scissor rect calculation with various padding values, cursor position unaffected by padding

### Task 2.3: Mouse coordinate translation with padding [x] <!-- a58f7b8 -->
- [ ] Update mouse-to-cell mapping in `window.rs` to subtract padding offset before dividing by cell size
- [ ] Clicks in the padding area are ignored (no cell hit)
- [ ] Selection drag respects padding offset
- [ ] Write tests: click at padding edge → no cell, click just inside padding → cell (0,0)

## Phase 3: Runtime Font Size Adjustment & Integration

### Task 3.1: Font size keyboard shortcuts
- [ ] Add `AppCommand` enum to `input/mod.rs`: `IncreaseFontSize`, `DecreaseFontSize`, `ResetFontSize`
- [ ] Add `match_app_command()` matcher: Cmd+= / Cmd+Plus → increase, Cmd+Minus → decrease, Cmd+0 → reset
- [ ] Platform-aware: use `super_key()` on macOS (Cmd), `control_key()` on Linux (Ctrl)
- [ ] Write tests: all three shortcuts detected correctly, no false positives with other modifiers

### Task 3.2: Font size change pipeline
- [ ] In `window.rs` event handler, match `AppCommand` and execute font size change
- [ ] Implement ~10% step: `new_size = (current * 1.1).round()` for increase, `(current / 1.1).round()` for decrease
- [ ] Clamp to min 8px, max 72px
- [ ] Store default size from config for Cmd+0 reset
- [ ] On size change: rebuild atlas → recalculate all pane grid dimensions → resize all PTYs → trigger full redraw
- [ ] Write tests: size increase/decrease math, boundary clamping, reset to default

### Task 3.3: Config hot-reload for font and padding
- [ ] Wire config watcher `font_changed` delta to trigger atlas rebuild + grid recalculation
- [ ] Wire config watcher `padding_changed` delta to trigger grid recalculation + redraw
- [ ] Invalid font names: log warning, fall back to next in chain
- [ ] Write tests: config delta detection for font and padding changes

### Task 3.4: Final integration and validation
- [ ] Run full test suite — all existing + new tests pass
- [ ] Build and run application — verify JetBrains Mono renders at 13px with 1.5x line-height
- [ ] Verify 12px padding visible on all sides (text not flush against pane edges)
- [ ] Verify Cmd+Plus/Minus changes font size visually
- [ ] Verify Cmd+0 resets to configured default
- [ ] Screenshot validation with Playwright MCP
