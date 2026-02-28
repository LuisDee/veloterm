# Track 28 Specification: Overlay Shell & Split Panel Framework

## Overview

This track builds the generic overlay infrastructure that the File Browser and Git Review features will use. It delivers: two new `InputMode` variants, toolbar icon buttons, keyboard shortcuts, a reusable `SplitPanel` widget with draggable divider, overlay lifecycle management, and state scaffolding. At completion, two empty overlay shells can be toggled via toolbar icons or keyboard shortcuts, with a functional resizable split panel.

---

## Data Structures

### InputMode Extensions

**File:** `src/input/mod.rs`

Add two new variants to the existing `InputMode` enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
    CommandPalette,
    Conductor,
    MarkdownPreview,
    FileBrowser,    // NEW
    GitReview,      // NEW
}
```

These variants follow the exact pattern of `Conductor` and `MarkdownPreview` -- they cause the App keyboard handler to intercept keys and the iced view layer to replace the content area.

### Overlay Command Matching

**File:** `src/input/mod.rs`

Add an `OverlayCommand` enum and matcher function:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayCommand {
    ToggleFileBrowser,
    ToggleGitReview,
}

pub fn match_overlay_command(
    logical_key: &Key,
    modifiers: ModifiersState,
) -> Option<OverlayCommand> {
    // Ctrl+E (no shift) -> ToggleFileBrowser
    // Ctrl+G (no shift) -> ToggleGitReview
    // Only fires when Ctrl is held WITHOUT Shift (to avoid conflict with
    // Ctrl+Shift+E = SplitHorizontal pane command).
}
```

**Important shortcut note:** `Ctrl+Shift+E` is already bound to `PaneCommand::SplitHorizontal`. The overlay shortcuts use `Ctrl+E` (Ctrl only, no Shift) and `Ctrl+G` (Ctrl only, no Shift). This means these shortcuts will intercept the terminal control characters Ctrl+E (0x05, readline end-of-line) and Ctrl+G (0x07, bell) when in Normal mode. This is an intentional trade-off -- these overlays are accessed frequently enough to warrant single-modifier shortcuts. Users can still send the raw control characters via the terminal when overlays are active (they get routed differently) or remap in config.

### FileBrowserState

**File:** `src/file_browser/mod.rs` (new module)

```rust
/// State for the File Browser overlay.
/// Phase 1 (this track): minimal shell state for toggle/layout.
/// Future tracks (29-31) add tree data, preview, search, etc.
pub struct FileBrowserState {
    /// Split panel divider position as fraction (0.0..1.0). Default 0.5.
    pub split_ratio: f32,
    /// Which panel has focus: Left (file tree) or Right (preview).
    pub focused_panel: OverlayPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayPanel {
    #[default]
    Left,
    Right,
}

impl FileBrowserState {
    pub fn new() -> Self {
        Self {
            split_ratio: 0.5,
            focused_panel: OverlayPanel::Left,
        }
    }
}
```

### GitReviewState

**File:** `src/git_review/mod.rs` (new module)

```rust
/// State for the Git Review overlay.
/// Phase 1 (this track): minimal shell state for toggle/layout.
/// Future tracks (32-34) add git status, diff rendering, staging, etc.
pub struct GitReviewState {
    /// Split panel divider position as fraction (0.0..1.0). Default 0.5.
    pub split_ratio: f32,
    /// Which panel has focus: Left (changed files) or Right (diff view).
    pub focused_panel: OverlayPanel,
}

impl GitReviewState {
    pub fn new() -> Self {
        Self {
            split_ratio: 0.5,
            focused_panel: OverlayPanel::Left,
        }
    }
}
```

The `OverlayPanel` enum is defined in `src/file_browser/mod.rs` and re-exported so `git_review` can use it too. (Or define it in a shared location -- see module structure below.)

### SplitPanel Widget

**File:** `src/split_panel.rs` (new file)

A custom iced widget that renders two child elements side by side with a draggable vertical divider.

```rust
/// A horizontal split panel with a draggable divider.
///
/// Renders `left` and `right` children separated by a vertical divider.
/// The divider can be dragged to resize. Double-click resets to 50%.
pub struct SplitPanel<'a, Message> {
    left: Element<'a, Message>,
    right: Element<'a, Message>,
    ratio: f32,
    on_resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_reset: Option<Message>,
    divider_width: f32,
    min_ratio: f32,   // 0.2
    max_ratio: f32,   // 0.8
}
```

**Widget behavior:**
- `ratio`: fraction of total width allocated to left panel (0.0..1.0)
- `divider_width`: 4.0 logical pixels (3-4px per spec)
- `min_ratio` / `max_ratio`: clamped to 0.2..0.8 (20%-80% range)
- Drag: emits `on_resize(new_ratio)` during drag (live resize, not on release)
- Double-click: emits `on_reset` message (resets to 0.5)
- Cursor: changes to `CursorIcon::ColResize` when hovering divider
- Visual: divider rendered as a subtle line using theme `border_visible` color

**Widget API:**
```rust
impl<'a, Message> SplitPanel<'a, Message> {
    pub fn new(
        left: impl Into<Element<'a, Message>>,
        right: impl Into<Element<'a, Message>>,
        ratio: f32,
    ) -> Self;

    pub fn on_resize(self, f: impl Fn(f32) -> Message + 'a) -> Self;
    pub fn on_reset(self, msg: Message) -> Self;
    pub fn divider_width(self, width: f32) -> Self;
    pub fn min_ratio(self, min: f32) -> Self;
    pub fn max_ratio(self, max: f32) -> Self;
}
```

The SplitPanel implements `iced_core::Widget<Message, iced_core::Theme, iced_wgpu::Renderer>` with proper `layout()`, `draw()`, `on_event()`, and `mouse_interaction()`.

---

## UiMessage Extensions

**File:** `src/renderer/iced_layer.rs`

Add these variants to the `UiMessage` enum:

```rust
pub enum UiMessage {
    // ... existing variants ...

    // Overlay toggle (from toolbar icons)
    ToggleFileBrowser,
    ToggleGitReview,
    // Overlay icon hover state
    FileBrowserIconEnter,
    FileBrowserIconExit,
    GitReviewIconEnter,
    GitReviewIconExit,
    // Split panel resize
    OverlaySplitResize(f32),
    OverlaySplitReset,
    // Panel focus (Tab key switches panels)
    OverlayFocusPanel(OverlayPanel),
}
```

---

## UiState Extensions

**File:** `src/renderer/iced_layer.rs`

Add to `UiState`:

```rust
pub struct UiState<'a> {
    // ... existing fields ...

    /// Active overlay type (None = no overlay, showing terminal panes).
    pub active_overlay: Option<ActiveOverlay>,
    /// File browser split ratio (only used when overlay is FileBrowser).
    pub file_browser_split_ratio: f32,
    /// Git review split ratio (only used when overlay is GitReview).
    pub git_review_split_ratio: f32,
    /// Which panel is focused in the active overlay.
    pub overlay_focused_panel: OverlayPanel,
    /// Whether the File Browser toolbar icon is hovered.
    pub is_file_browser_icon_hovered: bool,
    /// Whether the Git Review toolbar icon is hovered.
    pub is_git_review_icon_hovered: bool,
}

/// Which overlay is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlay {
    FileBrowser,
    GitReview,
}
```

---

## App Extensions

**File:** `src/window.rs`

Add to `App` struct:

```rust
pub struct App {
    // ... existing fields ...

    /// File browser overlay state. Some = state exists (may or may not be visible).
    file_browser_state: Option<FileBrowserState>,
    /// Git review overlay state. Some = state exists (may or may not be visible).
    git_review_state: Option<GitReviewState>,
    /// Whether the File Browser toolbar icon is hovered.
    hovering_file_browser_icon: bool,
    /// Whether the Git Review toolbar icon is hovered.
    hovering_git_review_icon: bool,
}
```

---

## Module Structure

### New Files

| File | Purpose |
|------|---------|
| `src/file_browser/mod.rs` | `FileBrowserState`, `OverlayPanel` enum |
| `src/git_review/mod.rs` | `GitReviewState` |
| `src/split_panel.rs` | `SplitPanel` widget |

### Modified Files

| File | Changes |
|------|---------|
| `src/lib.rs` | Add `pub mod file_browser;`, `pub mod git_review;`, `pub mod split_panel;` |
| `src/input/mod.rs` | Add `FileBrowser`/`GitReview` to `InputMode`, add `OverlayCommand` enum + `match_overlay_command()` |
| `src/renderer/iced_layer.rs` | Add `UiMessage` variants, `UiState` fields, `ActiveOverlay` enum, toolbar icons in `chrome_bar()`, overlay content in `view()` |
| `src/window.rs` | Add overlay state fields to `App`, wire keyboard shortcuts, handle `UiMessage` variants, populate `UiState` overlay fields |

---

## Overlay Lifecycle

### Opening

1. User presses `Ctrl+E` (keyboard) or clicks File Browser icon (toolbar)
2. App checks current `input_mode`:
   - If `InputMode::FileBrowser` -> close overlay (toggle off), set `InputMode::Normal`
   - If any other mode -> set `InputMode::FileBrowser`, create `FileBrowserState` if None
3. Same pattern for `Ctrl+G` / Git Review icon -> `InputMode::GitReview`
4. If switching between overlays (e.g., File Browser open, user presses Ctrl+G):
   - Directly set `input_mode = InputMode::GitReview` (no intermediate Normal state)
   - The previous overlay state is preserved (not destroyed)

### Closing

1. User presses `Escape` while in FileBrowser or GitReview mode
2. App sets `input_mode = InputMode::Normal`
3. Overlay state is preserved (not destroyed) so re-opening restores position

### View Rendering

In `IcedLayer::view()`, after the existing conductor dashboard check:

```rust
let content: IcedElement<'a> = if state.conductor.is_some() {
    Self::conductor_dashboard(state, scale)
} else if let Some(overlay) = state.active_overlay {
    Self::overlay_content(state, scale, overlay)  // NEW
} else {
    base_content
};
```

`overlay_content()` renders a `SplitPanel` with:
- **Left panel:** Empty placeholder ("File Browser" or "Changed Files" centered text)
- **Right panel:** Empty placeholder ("File Preview" or "Diff View" centered text)

### Focus Management

- Opening an overlay sets `focused_panel = OverlayPanel::Left`
- `Tab` key (when in overlay mode) toggles between Left and Right panels
- The focused panel gets a subtle visual indicator (e.g., slightly different background or accent border)

---

## Chrome Bar Toolbar Icons

**File:** `src/renderer/iced_layer.rs`, in `chrome_bar()`

Add two new icon buttons between the center brand text and the tracks icon:

```
[hamburger] .......... [sparkle VeloTerm] .......... [folder] [git] [tracks]
```

- **File Browser icon:** Unicode folder `\u{1F4C1}` (or simpler `\u{2636}` / text "FB"). Rendered as text with DM Sans font, size 14px. Color: accent when FileBrowser active, text_secondary otherwise.
- **Git Review icon:** Unicode branch symbol -- use text glyph. Color: accent when GitReview active, text_secondary otherwise.
- Both use the same MouseArea + container pattern as the existing tracks icon (hover bg change).

---

## Keyboard Shortcut Routing

**File:** `src/window.rs`, in the keyboard event handler

Shortcuts are checked at the App level, BEFORE overlay-specific key handling:

1. `Ctrl+E` (Ctrl only, no Shift): Toggle File Browser overlay
2. `Ctrl+G` (Ctrl only, no Shift): Toggle Git Review overlay
3. `Escape` (when in FileBrowser or GitReview mode): Close overlay

These are checked after command palette toggle but before pane/tab/search commands, following the existing priority order.

When in `InputMode::FileBrowser` or `InputMode::GitReview`:
- `Tab` key: switch focused panel (Left <-> Right)
- `Escape`: close overlay
- All other keys: consumed (no pass-through to PTY)

---

## Acceptance Criteria

1. **InputMode variants exist:** `InputMode::FileBrowser` and `InputMode::GitReview` are defined and testable
2. **Overlay toggle:** Pressing Ctrl+E opens File Browser overlay, pressing again closes it. Same for Ctrl+G and Git Review.
3. **Only-one-active:** Opening File Browser while Git Review is open switches directly (Git Review closes, File Browser opens)
4. **Escape dismisses:** Pressing Escape while any overlay is active returns to Normal mode
5. **Toolbar icons:** File Browser and Git Review icons are visible in the chrome bar, with correct hover states
6. **Toolbar icon toggle:** Clicking a toolbar icon toggles its corresponding overlay
7. **SplitPanel widget:** Renders two panels side by side with correct ratio
8. **SplitPanel divider drag:** Dragging the divider resizes panels in real-time
9. **SplitPanel clamping:** Divider cannot be dragged below 20% or above 80%
10. **SplitPanel double-click reset:** Double-clicking the divider resets ratio to 0.5
11. **Focus management:** Opening overlay focuses left panel; Tab switches panels
12. **Empty state content:** Both overlays show placeholder text in their panels
13. **State preservation:** Closing and reopening an overlay preserves the split ratio
14. **All existing tests pass:** No regressions in the 1236 existing tests
