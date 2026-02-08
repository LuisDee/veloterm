# Track 08: URL & Path Detection — Plan

## Phase 1: Link Detection Engine [checkpoint: a30aa07]

### Task 1.1: DetectedLink data model and LinkKind <!-- 307fc6d -->
- [x] Write tests for DetectedLink creation with Url and FilePath kinds
- [x] Write tests for link_contains(row, col) hit testing
- [x] Implement `DetectedLink`, `LinkKind` in `src/link/mod.rs`
- [x] Add `pub mod link;` to `src/lib.rs`

### Task 1.2: URL detection with linkify <!-- 307fc6d -->
- [x] Add `linkify` dependency to Cargo.toml
- [x] Write tests for detecting http:// URLs in single-line text
- [x] Write tests for detecting https:// URLs with paths and query strings
- [x] Write tests for URL boundary detection (stops at ), ], >, quotes, whitespace)
- [x] Write tests for no false positives on plain text
- [x] Implement `detect_urls(lines: &[String]) -> Vec<DetectedLink>` in `src/link/detector.rs`

### Task 1.3: File path detection <!-- 307fc6d -->
- [x] Write tests for detecting absolute Unix paths (/usr/bin/foo)
- [x] Write tests for detecting home-relative paths (~/Documents/file.txt)
- [x] Write tests for path boundary detection (stops at whitespace, quotes, parens)
- [x] Write tests for ignoring false positives (/dev/null, single /)
- [x] Write tests for paths with dots and extensions (/foo/bar.rs, /tmp/file.log)
- [x] Implement `detect_paths(lines: &[String]) -> Vec<DetectedLink>` in `src/link/detector.rs`

### Task 1.4: LinkDetector with caching <!-- 307fc6d -->
- [x] Write tests for LinkDetector::scan() combining URL and path detection
- [x] Write tests for link_at(row, col) returning correct link
- [x] Write tests for link_at() returning None for non-link positions
- [x] Write tests for generation counter incrementing on re-scan
- [x] Implement `LinkDetector` in `src/link/mod.rs`

### Phase 1 Completion
- [x] Phase completion verification and checkpointing

## Phase 2: Grid Shader Underline & App Integration [checkpoint: d449be4]

### Task 2.1: Grid shader underline rendering <!-- 911e52a -->
- [x] Write tests for CELL_FLAG_UNDERLINE being passed to shader correctly (existing tests verify flag propagation)
- [x] Extend grid.wgsl vertex shader to pass underline flag to fragment shader
- [x] Extend grid.wgsl fragment shader: draw 1px underline at cell bottom when flag is set
- [x] Write visual validation test: render cells with underline flag, verify via screenshot

### Task 2.2: LinksConfig and opener <!-- 911e52a -->
- [x] Write tests for LinksConfig defaults (enabled: true, auto-detect modifier)
- [x] Write tests for LinksConfig parsing from TOML
- [x] Write tests for open_link() dispatching Url vs FilePath
- [x] Write tests for platform command construction (open/xdg-open, $EDITOR)
- [x] Implement `LinksConfig` in `src/config/types.rs`
- [x] Implement `open_link()` in `src/link/opener.rs`

### Task 2.3: Wire link detection into App <!-- 911e52a -->
- [x] Add `link_detector: LinkDetector` to App struct
- [x] Trigger re-scan when terminal content changes (on PTY read, not per-frame)
- [x] Track modifier key state via ModifiersChanged event
- [x] On CursorMoved with modifier held: check link_at(), set cursor to Pointer, mark cells for underline highlight
- [x] On CursorMoved without modifier: clear any link highlight
- [x] On MouseInput with modifier: check link_at(), call open_link(), consume event
- [x] Request redraw when link highlight changes
- [x] Write integration tests for hover highlight and click dispatch

### Task 2.4: Visual Validation <!-- 911e52a -->
- [x] Build and run application
- [x] Validate via screenshots:
  - [x] URL underline highlight visible on modifier+hover
  - [x] File path underline highlight visible on modifier+hover
  - [x] Cursor changes to pointer hand on link hover
  - [x] Modifier+click opens URL in browser
  - [x] Modifier+click opens file path in editor

### Phase 2 Completion
- [x] Phase completion verification and checkpointing
