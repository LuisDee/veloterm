# Track 28 Implementation Plan: Overlay Shell & Split Panel Framework

## Phase 1: State Management + InputMode + Keyboard Shortcuts

### Tests to Write FIRST

**File: `src/input/mod.rs`** (add to existing `#[cfg(test)] mod tests`)

```
test overlay_cmd_ctrl_e_toggles_file_browser
  - match_overlay_command with Ctrl+E returns Some(OverlayCommand::ToggleFileBrowser)

test overlay_cmd_ctrl_g_toggles_git_review
  - match_overlay_command with Ctrl+G returns Some(OverlayCommand::ToggleGitReview)

test overlay_cmd_ctrl_shift_e_does_not_match
  - match_overlay_command with Ctrl+Shift+E returns None (reserved for SplitHorizontal)

test overlay_cmd_ctrl_shift_g_does_not_match
  - match_overlay_command with Ctrl+Shift+G returns None

test overlay_cmd_no_modifier_does_not_match
  - match_overlay_command with bare E/G (no modifiers) returns None

test overlay_cmd_ctrl_only_other_keys_no_match
  - match_overlay_command with Ctrl+X, Ctrl+A, etc. returns None

test input_mode_has_file_browser_variant
  - InputMode::FileBrowser exists and is not equal to Normal

test input_mode_has_git_review_variant
  - InputMode::GitReview exists and is not equal to Normal
```

**File: `src/file_browser/mod.rs`** (new file with `#[cfg(test)] mod tests`)

```
test file_browser_state_defaults
  - FileBrowserState::new() has split_ratio=0.5 and focused_panel=Left

test file_browser_toggle_focus
  - Toggling focused_panel from Left gives Right and vice versa

test overlay_panel_default_is_left
  - OverlayPanel::default() == OverlayPanel::Left
```

**File: `src/git_review/mod.rs`** (new file with `#[cfg(test)] mod tests`)

```
test git_review_state_defaults
  - GitReviewState::new() has split_ratio=0.5 and focused_panel=Left
```

### Implementation Steps

1. **Add `InputMode::FileBrowser` and `InputMode::GitReview`** to the enum in `src/input/mod.rs`

2. **Add `OverlayCommand` enum and `match_overlay_command()` function** in `src/input/mod.rs`:
   - Match Ctrl+E (Ctrl only, NOT Ctrl+Shift) -> `ToggleFileBrowser`
   - Match Ctrl+G (Ctrl only, NOT Ctrl+Shift) -> `ToggleGitReview`
   - Return None for any other key or modifier combination

3. **Create `src/file_browser/mod.rs`**:
   - Define `OverlayPanel` enum (`Left`, `Right`) with `Default` (Left)
   - Define `FileBrowserState` struct with `split_ratio: f32` and `focused_panel: OverlayPanel`
   - Implement `FileBrowserState::new()` -> defaults (0.5, Left)
   - Implement `toggle_focus(&mut self)` method
   - Tests in same file

4. **Create `src/git_review/mod.rs`**:
   - Define `GitReviewState` struct with `split_ratio: f32` and `focused_panel: OverlayPanel`
   - Import `OverlayPanel` from `crate::file_browser`
   - Implement `GitReviewState::new()` -> defaults (0.5, Left)
   - Tests in same file

5. **Register modules in `src/lib.rs`**:
   - Add `pub mod file_browser;`
   - Add `pub mod git_review;`

6. **Run tests** to verify all new tests pass and no regressions

### Files Created/Modified

- `src/input/mod.rs` (modify: add variants + command enum + function + tests)
- `src/file_browser/mod.rs` (create)
- `src/git_review/mod.rs` (create)
- `src/lib.rs` (modify: add module declarations)

### Commit

```
feat(overlay): add InputMode variants and state structs for overlay framework

- Add InputMode::FileBrowser and InputMode::GitReview variants
- Add OverlayCommand enum with match_overlay_command() for Ctrl+E / Ctrl+G
- Create file_browser module with FileBrowserState and OverlayPanel
- Create git_review module with GitReviewState
- 8 new tests covering input matching and state defaults
```

---

## Phase 2: SplitPanel Widget

### Tests to Write FIRST

**File: `src/split_panel.rs`** (new file with `#[cfg(test)] mod tests`)

```
test split_panel_clamp_ratio_below_minimum
  - clamp_ratio(0.1, 0.2, 0.8) returns 0.2

test split_panel_clamp_ratio_above_maximum
  - clamp_ratio(0.9, 0.2, 0.8) returns 0.8

test split_panel_clamp_ratio_within_range
  - clamp_ratio(0.4, 0.2, 0.8) returns 0.4

test split_panel_clamp_ratio_at_boundaries
  - clamp_ratio(0.2, 0.2, 0.8) returns 0.2
  - clamp_ratio(0.8, 0.2, 0.8) returns 0.8

test split_panel_default_ratio_is_valid
  - 0.5 is within 0.2..0.8

test split_panel_reset_ratio
  - The reset value is 0.5

test split_panel_divider_hit_test
  - Given a total width and ratio, compute the divider x position
  - A point inside the divider hit zone returns true
  - A point outside returns false

test split_panel_left_width_calculation
  - Given total_width=1000.0, ratio=0.5, divider=4.0:
    left_width = (1000.0 - 4.0) * 0.5 = 498.0

test split_panel_right_width_calculation
  - Given total_width=1000.0, ratio=0.3, divider=4.0:
    left_width = (1000.0 - 4.0) * 0.3 = 298.8
    right_width = 1000.0 - 298.8 - 4.0 = 697.2

test split_panel_ratio_from_cursor_position
  - Given cursor_x=500.0, total_width=1000.0, divider=4.0:
    ratio = 500.0 / 1000.0 = 0.5
  - Given cursor_x=200.0, total_width=1000.0:
    ratio = 0.2 (clamped at minimum)
```

### Implementation Steps

1. **Create `src/split_panel.rs`**:
   - Define `SplitPanel<'a, Message>` struct (see spec for fields)
   - Implement builder API: `new()`, `on_resize()`, `on_reset()`, `divider_width()`, `min_ratio()`, `max_ratio()`
   - Implement helper functions (extract as standalone fns for testability):
     - `clamp_ratio(ratio, min, max) -> f32`
     - `divider_hit_test(cursor_x, total_width, ratio, divider_width) -> bool`
     - `left_panel_width(total_width, ratio, divider_width) -> f32`
     - `ratio_from_cursor(cursor_x, total_width) -> f32`
   - Implement `Widget` trait for `SplitPanel`:
     - `layout()`: Create a node with three children (left, divider, right)
     - `draw()`: Draw left child, divider line, right child
     - `on_event()`: Handle mouse press/move/release on divider, double-click detection
     - `mouse_interaction()`: Return `CursorIcon::ColResize` when hovering divider
   - Implement `From<SplitPanel<'a, Message>> for Element<'a, Message>`
   - Tests for helper functions in same file

2. **Register module in `src/lib.rs`**:
   - Add `pub mod split_panel;`

3. **Run tests** to verify all new tests pass and no regressions

### Files Created/Modified

- `src/split_panel.rs` (create)
- `src/lib.rs` (modify: add module declaration)

### Commit

```
feat(overlay): add SplitPanel widget with draggable divider

- Custom iced widget for horizontal split with vertical divider
- Divider drag with live resize, clamped to 20%-80% range
- Double-click divider resets to 50%
- col-resize cursor on divider hover
- 10 new tests covering ratio clamping, hit testing, layout math
```

---

## Phase 3: Integration + Toolbar + View

### Tests to Write FIRST

**File: `src/file_browser/mod.rs`** (add to existing tests)

```
test overlay_toggle_from_normal_to_file_browser
  - Starting from InputMode::Normal, toggling file browser sets InputMode::FileBrowser
  - FileBrowserState is created if None

test overlay_toggle_from_file_browser_to_normal
  - Starting from InputMode::FileBrowser, toggling file browser sets InputMode::Normal

test overlay_switch_from_git_review_to_file_browser
  - Starting from InputMode::GitReview, toggling file browser sets InputMode::FileBrowser directly

test overlay_escape_closes_file_browser
  - Starting from InputMode::FileBrowser, Escape sets InputMode::Normal

test overlay_tab_toggles_panel_focus
  - In FileBrowser mode with focused_panel=Left, Tab sets focused_panel=Right
  - In FileBrowser mode with focused_panel=Right, Tab sets focused_panel=Left

test overlay_preserves_split_ratio_across_toggle
  - Open overlay, set split_ratio to 0.3, close, reopen -> ratio is still 0.3
```

**File: `src/git_review/mod.rs`** (add to existing tests)

```
test overlay_toggle_from_normal_to_git_review
  - Starting from InputMode::Normal, toggling git review sets InputMode::GitReview

test overlay_toggle_from_git_review_to_normal
  - Starting from InputMode::GitReview, toggling git review sets InputMode::Normal

test overlay_switch_from_file_browser_to_git_review
  - Starting from InputMode::FileBrowser, toggling git review sets InputMode::GitReview directly
```

Note: The toggle logic tests above are unit tests on helper functions. The actual App-level wiring (keyboard events, UiMessage handling) is integration-level and verified by manual testing + compile-time correctness. The tests validate the state transition logic that the App calls into.

### Implementation Steps

1. **Add overlay toggle helper to `src/file_browser/mod.rs`**:
   ```rust
   /// Compute the next InputMode when toggling the file browser.
   pub fn toggle_file_browser(current: InputMode) -> InputMode {
       if current == InputMode::FileBrowser {
           InputMode::Normal
       } else {
           InputMode::FileBrowser
       }
   }
   ```
   Same pattern in `src/git_review/mod.rs` for `toggle_git_review()`.

2. **Add UiMessage variants** to `src/renderer/iced_layer.rs`:
   - `ToggleFileBrowser`, `ToggleGitReview`
   - `FileBrowserIconEnter`, `FileBrowserIconExit`
   - `GitReviewIconEnter`, `GitReviewIconExit`
   - `OverlaySplitResize(f32)`, `OverlaySplitReset`

3. **Add UiState fields** to `src/renderer/iced_layer.rs`:
   - `active_overlay: Option<ActiveOverlay>`
   - `file_browser_split_ratio: f32`
   - `git_review_split_ratio: f32`
   - `overlay_focused_panel: OverlayPanel`
   - `is_file_browser_icon_hovered: bool`
   - `is_git_review_icon_hovered: bool`
   - Define `ActiveOverlay` enum

4. **Add toolbar icons in `chrome_bar()`**:
   - File Browser icon: folder glyph `\u{1F4C1}` (or a simpler text glyph that renders well in DM Sans)
   - Git Review icon: a suitable text glyph for version control
   - Both use MouseArea + container pattern matching the tracks icon
   - Position: in the right-side icon group, before the tracks icon
   - Layout: `[sidebar_btn] ... [VeloTerm] ... [file_browser] [git_review] [tracks]`

5. **Add overlay content rendering in `view()`**:
   - After the conductor dashboard check, add overlay check:
     ```rust
     } else if let Some(overlay) = state.active_overlay {
         Self::overlay_content(state, scale, overlay)
     }
     ```
   - Implement `overlay_content()` that creates a `SplitPanel` with:
     - Left: placeholder container with centered "File Browser" / "Changed Files" text
     - Right: placeholder container with centered "Select a file to preview" / "Select a file to view changes" text
     - Appropriate split ratio from state
     - `on_resize` -> `UiMessage::OverlaySplitResize`
     - `on_reset` -> `UiMessage::OverlaySplitReset`

6. **Wire App fields in `src/window.rs`**:
   - Add `file_browser_state`, `git_review_state`, hover fields to App struct
   - Initialize in `App::new()`: all None/false
   - In keyboard handler (after command palette, before pane commands):
     - Import and call `match_overlay_command()`
     - If `ToggleFileBrowser`: call toggle logic, create state if None
     - If `ToggleGitReview`: call toggle logic, create state if None
   - In overlay mode keyboard handler (similar to conductor mode):
     - `Escape` -> close overlay
     - `Tab` -> toggle panel focus
     - Consume all other keys
   - In UiMessage handler:
     - `ToggleFileBrowser` -> same toggle logic as keyboard
     - `ToggleGitReview` -> same toggle logic as keyboard
     - `OverlaySplitResize(ratio)` -> update appropriate state's split_ratio
     - `OverlaySplitReset` -> set split_ratio to 0.5
     - Icon hover enter/exit -> set hover bools
   - In UiState construction (the big state snapshot):
     - Set `active_overlay` based on `input_mode`
     - Set split ratios from state
     - Set hover bools

7. **Run full test suite** to verify no regressions

### Files Created/Modified

- `src/file_browser/mod.rs` (modify: add toggle function + tests)
- `src/git_review/mod.rs` (modify: add toggle function + tests)
- `src/renderer/iced_layer.rs` (modify: UiMessage, UiState, chrome_bar, view, overlay_content)
- `src/window.rs` (modify: App fields, keyboard handler, UiMessage handler, UiState construction)

### Commit

```
feat(overlay): integrate overlay framework with toolbar icons and keyboard shortcuts

- Add toolbar icons for File Browser and Git Review in chrome bar
- Wire Ctrl+E / Ctrl+G keyboard shortcuts for overlay toggle
- Render SplitPanel with empty placeholder content for both overlays
- Handle overlay UiMessages (toggle, resize, reset, focus)
- Tab key toggles panel focus within overlay
- Escape dismisses active overlay
- 9 new tests covering toggle logic and state transitions
```
