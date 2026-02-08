# Track 21: Anthropic Dark Theme & Color Rendering — Implementation Plan

## Phase 1: Theme Struct & Token Update (FR-1, FR-2)

Restructure the Theme struct and update claude_dark to reference values.

### Task 1.1: Extend Theme struct with reference fields
- [x] Add new fields: `surface`, `surface_raised`, `terminal_bg`, `text_secondary`, `text_dim`, `border_subtle`, `blue` <!-- 3a1042a -->
- [x] Rename `text_primary` → `text`, `pane_background` → `terminal_bg`, `text_muted` → `text_secondary` <!-- 3a1042a -->
- [x] Keep `accent_hover`, `prompt`, `selection`, `search_match`, `search_match_active` (still needed) <!-- 3a1042a -->
- [x] Update all three theme constructors with values for new fields <!-- 3a1042a -->
- [x] TDD: tests for new field values on claude_dark matching exact reference hex <!-- 3a1042a -->

### Task 1.2: Update claude_dark to reference token values
- [x] Replace all claude_dark hex values with exact reference tokens <!-- 3a1042a -->
- [x] Update existing tests to assert new reference values <!-- 3a1042a -->
- [x] Verify claude_light and claude_warm have sensible values for new fields <!-- 3a1042a -->
- [x] TDD: tests asserting each claude_dark field matches reference hex exactly <!-- 3a1042a -->

### Task 1.3: Update all theme consumers for renamed fields
- [x] Update `grid_bridge.rs`: DEFAULT_FG → #FAF9F5, DEFAULT_BG → #181715 (FR-7) <!-- 3a1042a -->
- [x] Update `tab/bar.rs`: `pane_background` → `surface`, `text_muted` → `text_secondary` <!-- 3a1042a -->
- [x] Update `renderer/grid_renderer.rs`: `text_primary` → `text`, `text_muted` → `text_secondary` <!-- 3a1042a -->
- [x] Update `window.rs`: all `text_primary` → `text` references <!-- 3a1042a -->
- [x] Update all test assertions that reference old field names or old color values <!-- 3a1042a -->
- [x] TDD: verify compilation and all tests pass <!-- 3a1042a -->

### Phase 1 Completion
- [x] Phase completion verification and checkpointing protocol

---

## Phase 2: UI Element Styling (FR-3, FR-4, FR-5, FR-9)

Apply theme tokens to tab bar, terminal area, pane borders, and cursor.

### Task 2.1: Tab bar styling with reference tokens
- [x] Tab bar background: use `surface` instead of `pane_background` <!-- 3a1042a -->
- [x] Active tab: use `accent` for indicator stripe (2px at bottom) <!-- f204b22 -->
- [x] Active tab text: use `text` color <!-- f204b22 -->
- [x] Inactive tab text: use `text_secondary` color <!-- 3a1042a -->
- [x] Tab separators: use `border` color <!-- 3a1042a -->
- [x] TDD: tests for tab bar quad colors matching theme tokens <!-- f204b22 -->

### Task 2.2: Terminal content area and window background
- [x] Clear color (window background): use `background` (#141413) <!-- f204b22 -->
- [x] Terminal content area: use `terminal_bg` (#181715) <!-- 3a1042a -->
- [x] Ensure visual distinction between chrome and content areas <!-- f204b22 -->
- [x] TDD: verify background colors in renderer <!-- f204b22 -->

### Task 2.3: Cursor rendering in accent color
- [x] Block cursor fill: use `accent` (#D97757) <!-- f204b22 -->
- [x] Character under cursor: use `text` or `background` for contrast <!-- f204b22 -->
- [x] Verify cursor blink still works with new colors <!-- f204b22 -->
- [x] TDD: tests for cursor color values <!-- f204b22 -->

### Phase 2 Completion
- [x] Phase completion verification and checkpointing protocol

---

## Phase 3: macOS Integration & Visual Validation (FR-6, FR-8)

Platform integration and final verification.

### Task 3.1: macOS title bar color integration
- [x] Set `NSWindow.backgroundColor` to match theme `surface` color on window creation <!-- 86e70e9 -->
- [x] Apply via platform-specific code in window setup (winit window attributes or raw NSWindow access) <!-- 86e70e9 -->
- [x] Verify title bar blends with tab bar chrome <!-- 86e70e9 -->
- [x] TDD: test that platform code compiles (integration verified visually) <!-- 86e70e9 -->

### Task 3.2: ANSI color rendering verification
- [x] Verify 16 named ANSI colors render with standard values <!-- 86e70e9 -->
- [x] Verify 256-color cube calculation is correct <!-- 86e70e9 -->
- [x] Verify TrueColor pass-through works <!-- 86e70e9 -->
- [x] TDD: tests for ANSI color conversion correctness (spot-check key values) <!-- 86e70e9 -->

### Task 3.3: Visual validation and integration test
- [x] Build and launch via `./take-screenshot.sh` <!-- 86e70e9 -->
- [x] Verify claude_dark theme matches reference tokens visually <!-- 86e70e9 -->
- [x] Verify tab bar uses surface/accent colors <!-- 86e70e9 -->
- [x] Verify terminal content area uses terminal_bg <!-- 86e70e9 -->
- [x] Verify cursor renders in accent color <!-- 86e70e9 -->
- [x] Verify colored terminal output (prompt, etc.) renders correctly <!-- 86e70e9 -->

### Phase 3 Completion
- [x] Phase completion verification and checkpointing protocol
