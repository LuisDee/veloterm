<!-- ARCHITECT CONTEXT | Track: 08_url_detection | Wave: 4 | CC: v1 -->

## Cross-Cutting Constraints
- Testing: TDD, URL regex matching tests
- Performance Budget: detection must not add per-frame cost — run on terminal content change only

## Interfaces

### Owns
- URL/path detection engine
- Click-to-open handler
- Visual highlight for detected links

### Consumes
- `Config` (Track 03) — URL styling, click modifier key, $EDITOR setting
- Terminal grid content (existing)

## Dependencies
- Track 03_config: URL styling and $EDITOR configuration

<!-- END ARCHITECT CONTEXT -->

# Track 08: URL & Path Detection — Specification

## Overview

Automatic detection of URLs and absolute file paths in terminal output. Detected links are visually highlighted (underline + accent color) when the user hovers with the modifier key held. Modifier+click opens URLs in the default browser and file paths in the configured editor.

## Design Decisions

1. **Click activation**: Modifier+click (Cmd on macOS, Ctrl on Linux/Windows)
2. **Path detection scope**: Absolute paths only (`/foo/bar`, `~/foo`). Relative paths deferred to Track 10 (Shell Integration with CWD tracking).
3. **Detection library**: `linkify` crate for URL detection + custom regex for absolute file paths
4. **Highlight rendering**: Extend grid shader to render underline decorations using existing `CELL_FLAG_UNDERLINE`

## Data Model

### DetectedLink
```rust
pub struct DetectedLink {
    pub kind: LinkKind,
    pub start: (usize, usize),  // (row, col) — grid coordinates
    pub end: (usize, usize),    // (row, col) — inclusive end
    pub text: String,           // The detected URL or path text
}

pub enum LinkKind {
    Url,
    FilePath,
}
```

### LinkDetector
```rust
pub struct LinkDetector {
    links: Vec<DetectedLink>,
    generation: u64,  // incremented on each re-scan
}
```

- `scan(grid: &TerminalGrid) -> Vec<DetectedLink>` — scans visible terminal content for URLs and paths
- `link_at(row: usize, col: usize) -> Option<&DetectedLink>` — returns link at grid position (for hover/click)

## Detection Rules

### URLs
- Use `linkify` crate with default `LinkFinder` configuration
- Detect: `http://`, `https://`, `ftp://` URLs
- Stop at common terminal delimiters: `)`, `]`, `>`, `'`, `"`, whitespace

### File Paths
- Custom regex for absolute paths: `/[a-zA-Z0-9._/-]+` (Unix-style)
- Home directory paths: `~/[a-zA-Z0-9._/-]+`
- Must contain at least one `/` separator after the prefix
- Ignore common false positives (e.g., `/dev/null`, `/proc/` entries)
- Optional: validate path exists on filesystem for click-to-open (best-effort, no blocking I/O in render path)

## Hover & Highlight

### Modifier Detection
- Track modifier key state (Cmd on macOS, Ctrl on Linux) via winit `ModifiersChanged` event
- When modifier is held + cursor moves over a link:
  - Set cursor to `CursorIcon::Pointer` (hand)
  - Mark link cells with `CELL_FLAG_UNDERLINE` and accent foreground color
  - Request redraw

### Grid Shader Extension
- Extend `shaders/grid.wgsl` fragment shader to render underline decoration:
  - Extract bit 4 (`CELL_FLAG_UNDERLINE`) from flags
  - When set: draw a 1px line at the bottom of the cell (last ~2 rows of pixels)
  - Use foreground color for the underline
- This also enables underline rendering for terminal escape sequences (bonus)

## Click Handling

### Modifier+Click Flow
1. On `MouseInput::Pressed` with modifier key held:
2. Convert pixel position to grid coordinates via `pixel_to_cell()`
3. Query `link_detector.link_at(row, col)`
4. If link found:
   - `LinkKind::Url` → open with system browser (`open` on macOS, `xdg-open` on Linux)
   - `LinkKind::FilePath` → open with `$EDITOR` or configured editor, falling back to system open
5. Consume the click event (don't pass to selection system)

### Platform Commands
- macOS: `open <url>` / `$EDITOR <path>` (fallback: `open <path>`)
- Linux: `xdg-open <url>` / `$EDITOR <path>` (fallback: `xdg-open <path>`)

## Performance

- Detection runs **only** on terminal content change (dirty flag from damage tracking)
- No per-frame scanning
- Link cache invalidated when terminal content changes
- Hover hit-testing is O(n) over detected links (n is small, typically <50 visible links)

## Configuration

Add to `Config`:
```rust
pub struct LinksConfig {
    pub enabled: bool,          // default: true
    pub modifier: String,       // "cmd" (macOS) / "ctrl" (Linux), auto-detected
}
```

Theme uses existing `accent` color for link highlight. No new theme fields needed.

## Integration Points

- `src/link/mod.rs` — LinkDetector, DetectedLink, LinkKind
- `src/link/detector.rs` — URL and path detection logic
- `src/link/opener.rs` — Platform-specific open commands
- `shaders/grid.wgsl` — Underline rendering
- `src/window.rs` — Wire hover detection, modifier tracking, click handling
- `src/config/types.rs` — LinksConfig

## Acceptance Criteria

1. URLs (http/https/ftp) detected and highlighted on modifier+hover
2. Absolute file paths detected and highlighted on modifier+hover
3. Modifier+click opens URL in default browser
4. Modifier+click opens file path in editor (or system default)
5. Cursor changes to pointer hand when hovering over link with modifier held
6. Underline decoration renders correctly in grid shader
7. Detection only runs on content change, not per-frame
8. All detection logic tested with TDD (>80% coverage)

## Out of Scope

- Relative path detection (requires Track 10 Shell Integration for CWD)
- Custom URL handlers or protocol handlers
- URL preview/tooltip
- OSC 8 hyperlink escape sequence support
- Right-click context menu for links
