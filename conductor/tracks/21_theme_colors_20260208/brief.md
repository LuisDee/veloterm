<!-- ARCHITECT CONTEXT | Track: 21_theme_colors | Wave: 8 | CC: v2 -->

## Cross-Cutting Constraints
- UI Reference Compliance: theme MUST match the reference implementation EXACTLY — all colors, spacing, border radii
- Testing: TDD for color conversion, ANSI color mapping, theme application
- Configuration Management: theme selectable via config with hot-reload

## Interfaces

### Owns
- Anthropic dark theme token system (all colors, spacing, radii from reference)
- ANSI 256-color and TrueColor rendering verification
- Colored prompt, ls, git status rendering verification
- macOS window chrome integration (title bar color matching)
- Cursor style and blink rate visual configuration

### Consumes
- `Config` (Track 03) — theme selection, cursor config
- `Theme` (Track 03) — existing theme infrastructure
- `GridBridge` (Track 02) — ANSI color to RGBA conversion
- `GlyphAtlas` (Track 15) — font metrics for consistent rendering

## Dependencies
- Track 03_config: theme configuration and hot-reload
- Track 15_font_padding: font metrics and padding system

<!-- END ARCHITECT CONTEXT -->

# Track 21: Anthropic Dark Theme & Color Rendering

## UI Reference — CRITICAL

This track's PRIMARY requirement is pixel-perfect adherence to the reference implementation. Every color, spacing value, and visual element must match:

- **Reference Cargo.toml:** `/Users/luisdeburnay/Downloads/Cargo.toml`
- **Reference main.rs:** `/Users/luisdeburnay/Downloads/src/main.rs`

### Reference Design Tokens (MUST match exactly)

```
Background:       #141413  (bg — window and deepest background)
Surface:          #1E1D1B  (elevated surface — chrome bars, headers)
Surface Raised:   #282724  (hover states, subtle cards)
Terminal BG:      #181715  (terminal content area background)
Text:             #FAF9F5  (primary text — cream white)
Text Secondary:   #B0AEA5  (supporting text)
Text Dim:         #6B6662  (timestamps, labels)
Border:           #33312E  (borders and dividers)
Border Subtle:    #262522  (less emphasis borders)
Accent:           #D97757  (terracotta orange — brand accent)
Success:          #788C5D  (sage green)
Blue:             #6A9BCC  (info — muted blue)
Error:            #C44242  (danger)
```

### Reference Layout Values
- Title bar padding: 14px vertical, 24px horizontal
- Content area padding: 12px
- Pane content padding: 16px
- Border radius: 8px on panes
- Tab accent stripe: 2px height
- Status bar padding: 10px vertical, 24px horizontal
- Font: 13px monospace, line-height 1.5x

## What This Track Delivers

A complete visual overhaul to match the Anthropic dark theme reference mockup exactly. Verifies and corrects ANSI 256-color and TrueColor rendering so that colored terminal output (prompts, ls, git status, etc.) renders correctly. Integrates the window chrome (title bar) with the macOS traffic light buttons. Makes all visual elements (tab bar, pane headers, borders, status indicators) use the reference design tokens.

## Scope

### IN
- Complete Anthropic dark theme implementation matching ALL reference design tokens
- ANSI 16-color palette mapped to theme-appropriate colors
- ANSI 256-color (6x6x6 cube + grayscale) rendering verified
- TrueColor (24-bit RGB) pass-through rendering verified
- Colored shell prompt rendering (PS1 with ANSI escapes)
- Colored `ls` output rendering
- Colored `git status` / `git diff` rendering
- Tab bar styling matching reference (accent stripe, numbered tabs, surface colors)
- Pane border styling (8px border radius, accent color for active pane)
- Status bar rendering matching reference layout
- macOS title bar color integration (match bg color to avoid chrome mismatch)
- Cursor rendering in theme colors

### OUT
- Additional theme creation (focus on getting one theme perfect)
- Theme editor UI
- Per-pane theming
- Background opacity/blur/transparency

## Key Design Decisions

1. **Theme architecture**: Update existing `claude_dark` theme in-place vs create new `anthropic_dark` theme vs replace all three themes with reference values?
   Trade-off: updating in-place is simplest; new theme preserves backward compat; replacing all ensures consistency

2. **macOS title bar**: Custom-drawn title bar (full control) vs native title bar with background color hint vs transparent title bar with content extending into it?
   Trade-off: custom gives full control but loses native feel; native with color hint is safest; transparent is modern but tricky

3. **ANSI color mapping strategy**: Map ANSI colors to closest theme-appropriate colors vs use standard ANSI colors (ignore theme) vs provide both options in config?
   Trade-off: theme-mapped looks cohesive; standard ANSI is expected behavior; configurable satisfies both

## Architectural Notes

- The existing theme system in `src/config/theme.rs` has 3 themes (claude_dark, claude_light, claude_warm) — the reference values should replace the `claude_dark` defaults
- The reference uses specific RGB values (#141413, #D97757, etc.) — these must be applied to ALL rendering passes (background, text, borders, tab bar, cursor)
- ANSI color rendering goes through `grid_bridge.rs` which maps `alacritty_terminal::term::color::Rgb` to wgpu RGBA — verify the conversion is correct for all 256 colors
- macOS title bar integration may require `NSWindow.titlebarAppearsTransparent` or `backgroundColor` — this is platform-specific winit configuration
- The reference has pane headers with badge numbers, accent stripes, and status dots — VeloTerm's tab bar should match this style
- ALL existing test assertions that check specific color values may need updating when theme tokens change

## Complexity: M
## Estimated Phases: ~3
