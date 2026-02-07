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

# Track 08: URL & Path Detection

## What This Track Delivers

Automatic detection of URLs and file paths in terminal output, rendering them as clickable links. URLs open in the default browser; file paths open in the user's configured `$EDITOR`. Detected links are visually highlighted (underline + color change) on hover.

## Scope

### IN
- URL detection in terminal cell content (http://, https://, ftp://)
- File path detection (absolute paths, relative paths with context)
- Visual highlight: underline + accent color on hover (Cmd/Ctrl+click or plain click, configurable)
- Click handler: URLs → default browser, file paths → $EDITOR
- Detection runs on terminal content change, not per frame
- linkify crate integration for URL detection

### OUT
- Semantic file path detection from shell integration (Track 10 — needs CWD context)
- Custom URL handlers or protocol handlers
- URL preview/tooltip

## Key Design Decisions

1. **Click activation**: Plain click vs modifier+click (Cmd/Ctrl+click) to open links?
   Trade-off: plain click conflicts with text selection; modifier+click is standard in terminals but less discoverable

2. **Path detection scope**: Absolute paths only vs relative paths (relative to what CWD?) vs regex-configurable?
   Trade-off: absolute paths are unambiguous; relative paths need CWD context (shell integration); regex is flexible but error-prone

3. **Detection library**: `linkify` crate vs custom regex vs alacritty_terminal's built-in URL detection?
   Trade-off: linkify is battle-tested for URLs; alacritty_terminal may already detect URLs; custom regex handles file paths

4. **Highlight rendering**: Modify cell attributes (underline flag) vs overlay layer vs shader-based hover detection?
   Trade-off: cell attributes integrate naturally; overlay is cleaner but adds rendering pass; shader is GPU-efficient but complex

## Architectural Notes

- alacritty_terminal may already have URL detection support — check before building from scratch
- File path detection with relative paths becomes much more useful when combined with Track 10 (shell integration) for CWD tracking
- The click handler needs to distinguish between text selection drags and link clicks — coordinate with existing selection logic in `src/input/selection.rs`

## Complexity: S
## Estimated Phases: ~2
