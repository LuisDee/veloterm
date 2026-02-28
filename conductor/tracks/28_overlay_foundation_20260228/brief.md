# Track 28: Overlay Shell & Split Panel Framework

## Source Requirements

From `new_feature` spec, Phase 1:
- Implement overlay toggle mechanism following the Conductor pattern
- Build resizable 50/50 split panel layout
- Add toolbar icons for both overlays (File Browser + Git Review)
- Get transitions and keyboard shortcuts working
- Establish theme/design token system shared by both overlays
- Success: Can toggle two empty overlay shells with smooth transitions

From Shared Overlay Behaviors:
- Overlay covers entire window, replacing all terminal panes (preserved in memory)
- Only one overlay active at a time — switching crossfades
- Toolbar remains visible at top for switching/dismissing
- Escape dismisses current overlay
- Ctrl+E / Ctrl+Shift+E toggles File Browser
- Ctrl+G / Ctrl+Shift+G toggles Git Review
- Panel divider: 50/50 default, draggable 20%-80%, 3-4px wide, double-click resets
- Opening overlay focuses left panel; Tab moves between panels

## Cross-Cutting Constraints

- All applicable from v1 (logging, error handling, config, testing, platform abstraction, performance)
- TDD workflow: failing tests first, then implement
- iced 0.14 widget system: MouseArea + container for hover (button hover broken)
- Clear-every-frame pipeline: force_redraw = true always
- Existing InputMode enum pattern for overlay state

## Interface Contracts

Existing overlay pattern (from codebase research):
- `InputMode` enum in `src/input/mod.rs:13-27` — add `FileBrowser` and `GitReview` variants
- `UiState` in `src/renderer/iced_layer.rs:134-196` — snapshot pattern for overlay state
- `UiMessage` in `src/renderer/iced_layer.rs:39-72` — add overlay action messages
- `view()` in `src/renderer/iced_layer.rs:371-510` — content replacement when overlay active
- `chrome_bar()` in `src/renderer/iced_layer.rs:513-600` — add toolbar icons
- `App` struct in `src/window.rs:108-170` — route keyboard shortcuts

## Dependencies

- Track 24 (iced UI Chrome) — complete, provides widget infrastructure
- Track 26 (UI Chrome Redesign) — complete, provides toolbar pattern

## What This Track Delivers

A generic overlay framework that both File Browser and Git Review overlays will use. This includes: two new InputMode variants, toolbar icon buttons, keyboard shortcuts, a resizable split-panel container widget, overlay open/close transitions, and the state management plumbing. At the end, two empty overlay shells can be toggled with full transitions and panel resizing.

## Scope IN

- InputMode::FileBrowser and InputMode::GitReview variants
- UiMessage variants for overlay toggling and panel actions
- Toolbar icons (folder icon for File Browser, git branch icon for Git Review)
- Keyboard shortcuts: Ctrl+E (File Browser), Ctrl+G (Git Review), Escape (dismiss)
- SplitPanel widget: resizable 50/50 vertical split with draggable divider
- Divider: 3-4px, col-resize cursor, double-click to reset, 20%-80% range
- Overlay state management: open/close, only-one-active-at-a-time
- Empty state content for both overlays (placeholder text)
- Theme design tokens for overlay panels (background, borders, text hierarchy)
- Focus management: left panel focused on open, Tab to switch

## Scope OUT

- File tree data model (Track 29)
- File preview/viewer (Track 30)
- Fuzzy search, context menus (Track 31)
- Git status reading (Track 32)
- Diff computation/rendering (Track 33)
- Word-level diff, hunk staging (Track 34)
- Animations/transitions beyond basic show/hide (Track 35)

## Key Design Decisions

1. **Split panel implementation**: Custom widget vs iced_aw::Split? The existing codebase uses custom iced widgets throughout. iced_aw::Split exists but may not match the exact interaction model needed (double-click reset, cursor change, clamped range).

2. **Overlay state storage**: Where to store File Browser and Git Review state? Options: (a) fields on App struct like ConductorState, (b) a new OverlayManager struct, (c) inside UiState. The Conductor pattern uses dedicated state structs stored on App.

3. **Icon rendering**: Text glyphs (Unicode) vs pre-rendered image handles? The existing toolbar uses text-based icons. For folder/git-branch icons, need to decide if Unicode coverage is sufficient or if custom icon assets are needed.

4. **Transition animation**: The spec calls for ~150ms fade/slide. Current overlays have no transition — they snap on/off. Should this track implement transitions or defer to Track 35?

5. **Keyboard shortcut routing**: How to handle Ctrl+E / Ctrl+G when the overlay is already active? Toggle off (spec says so), but also need to handle when a text input inside the overlay might want these keys.

## Architectural Notes

- The existing ConductorState and MarkdownPreviewState patterns are the reference implementations
- The overlay content replaces terminal pane rendering in the iced view() function
- The split panel widget will be reused by both File Browser and Git Review
- This track establishes patterns that all subsequent tracks (29-35) will follow
- The InputMode enum is the central dispatch mechanism for keyboard handling

## Test Strategy

- Unit tests for SplitPanel widget: divider position, clamping, double-click reset
- Unit tests for InputMode transitions: Normal→FileBrowser→Normal, Normal→GitReview→Normal, FileBrowser→GitReview
- Unit tests for keyboard shortcut routing
- Unit tests for overlay state management (only-one-active)
- Integration test: toolbar icon click toggles overlay
- Framework: `cargo test --lib`

## Complexity

M (Medium)

## Estimated Phases

3 phases:
1. InputMode + state management + keyboard shortcuts
2. SplitPanel widget + divider interaction
3. Toolbar icons + overlay view integration + empty states
