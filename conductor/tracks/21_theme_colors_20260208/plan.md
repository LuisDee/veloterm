# Track 21: Anthropic Dark Theme & Color Rendering — Implementation Plan

## Phase 1: Theme Struct & Token Update (FR-1, FR-2)

Restructure the Theme struct and update claude_dark to reference values.

### Task 1.1: Extend Theme struct with reference fields
- [ ] Add new fields: `surface`, `surface_raised`, `terminal_bg`, `text_secondary`, `text_dim`, `border_subtle`, `blue`
- [ ] Rename `text_primary` → `text`, `pane_background` → `terminal_bg`, `text_muted` → `text_secondary`
- [ ] Keep `accent_hover`, `prompt`, `selection`, `search_match`, `search_match_active` (still needed)
- [ ] Update all three theme constructors with values for new fields
- [ ] TDD: tests for new field values on claude_dark matching exact reference hex

### Task 1.2: Update claude_dark to reference token values
- [ ] Replace all claude_dark hex values with exact reference tokens
- [ ] Update existing tests to assert new reference values
- [ ] Verify claude_light and claude_warm have sensible values for new fields
- [ ] TDD: tests asserting each claude_dark field matches reference hex exactly

### Task 1.3: Update all theme consumers for renamed fields
- [ ] Update `grid_bridge.rs`: DEFAULT_FG → #FAF9F5, DEFAULT_BG → #181715 (FR-7)
- [ ] Update `tab/bar.rs`: `pane_background` → `surface`, `text_muted` → `text_secondary`
- [ ] Update `renderer/grid_renderer.rs`: `text_primary` → `text`, `text_muted` → `text_secondary`
- [ ] Update `window.rs`: all `text_primary` → `text` references
- [ ] Update all test assertions that reference old field names or old color values
- [ ] TDD: verify compilation and all tests pass

### Phase 1 Completion
- [ ] Phase completion verification and checkpointing protocol

---

## Phase 2: UI Element Styling (FR-3, FR-4, FR-5, FR-9)

Apply theme tokens to tab bar, terminal area, pane borders, and cursor.

### Task 2.1: Tab bar styling with reference tokens
- [ ] Tab bar background: use `surface` instead of `pane_background`
- [ ] Active tab: use `accent` for indicator stripe (2px at bottom)
- [ ] Active tab text: use `text` color
- [ ] Inactive tab text: use `text_secondary` color
- [ ] Tab separators: use `border` color
- [ ] TDD: tests for tab bar quad colors matching theme tokens

### Task 2.2: Terminal content area and window background
- [ ] Clear color (window background): use `background` (#141413)
- [ ] Terminal content area: use `terminal_bg` (#181715)
- [ ] Ensure visual distinction between chrome and content areas
- [ ] TDD: verify background colors in renderer

### Task 2.3: Cursor rendering in accent color
- [ ] Block cursor fill: use `accent` (#D97757)
- [ ] Character under cursor: use `text` or `background` for contrast
- [ ] Verify cursor blink still works with new colors
- [ ] TDD: tests for cursor color values

### Phase 2 Completion
- [ ] Phase completion verification and checkpointing protocol

---

## Phase 3: macOS Integration & Visual Validation (FR-6, FR-8)

Platform integration and final verification.

### Task 3.1: macOS title bar color integration
- [ ] Set `NSWindow.backgroundColor` to match theme `surface` color on window creation
- [ ] Apply via platform-specific code in window setup (winit window attributes or raw NSWindow access)
- [ ] Verify title bar blends with tab bar chrome
- [ ] TDD: test that platform code compiles (integration verified visually)

### Task 3.2: ANSI color rendering verification
- [ ] Verify 16 named ANSI colors render with standard values
- [ ] Verify 256-color cube calculation is correct
- [ ] Verify TrueColor pass-through works
- [ ] TDD: tests for ANSI color conversion correctness (spot-check key values)

### Task 3.3: Visual validation and integration test
- [ ] Build and launch via `./take-screenshot.sh`
- [ ] Verify claude_dark theme matches reference tokens visually
- [ ] Verify tab bar uses surface/accent colors
- [ ] Verify terminal content area uses terminal_bg
- [ ] Verify cursor renders in accent color
- [ ] Verify colored terminal output (prompt, etc.) renders correctly

### Phase 3 Completion
- [ ] Phase completion verification and checkpointing protocol
