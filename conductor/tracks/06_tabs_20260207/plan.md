# Track 06: Tab System — Plan

## Phase 1: Tab Data Model & Lifecycle [checkpoint: 69318c3]

### Task 1.1: TabId and Tab struct <!-- 73c0b42 -->
- [x] Write tests for TabId uniqueness (atomic counter, like PaneId)
- [x] Write tests for Tab creation with default title and PaneTree
- [x] Implement `TabId`, `Tab` in `src/tab/mod.rs`
- [x] Add `pub mod tab;` to `src/lib.rs`

### Task 1.2: TabManager core operations <!-- 73c0b42 -->
- [x] Write tests for TabManager::new() creates single tab
- [x] Write tests for new_tab() appends and returns TabId
- [x] Write tests for close_tab() removes tab, returns PaneIds for cleanup
- [x] Write tests for close_tab() on last tab returns None (can't close last)
- [x] Write tests for active_tab() / active_tab_mut() access
- [x] Implement TabManager in `src/tab/mod.rs`

### Task 1.3: Tab navigation <!-- 73c0b42 -->
- [x] Write tests for select_tab(index) switches active tab, clamps to range
- [x] Write tests for next_tab() / prev_tab() cycle behavior (wraps around)
- [x] Write tests for move_tab(from, to) reorder
- [x] Write tests for set_title() updates tab title
- [x] Implement navigation methods on TabManager

### Phase 1 Completion
- [x] Phase completion verification and checkpointing

## Phase 2: Tab Commands & Tab Bar Rendering [checkpoint: 86fe2ec]

### Task 2.1: TabCommand keybinding matching <!-- 5436b8b -->
- [x] Write tests for Ctrl+Shift+T → NewTab
- [x] Write tests for Ctrl+Shift+Tab → NextTab
- [x] Write tests for Ctrl+Shift+PageUp → PrevTab, PageDown → NextTab
- [x] Write tests for Ctrl+Shift+1..9 → SelectTab(n)
- [x] Write tests for Ctrl+Shift+{ → MoveTabLeft, } → MoveTabRight
- [x] Implement `TabCommand` enum and `match_tab_command()` in `src/input/mod.rs`

### Task 2.2: Tab bar quad generation <!-- 73c0b42 -->
- [x] Write tests for generating tab background quads (active vs inactive colors)
- [x] Write tests for tab width calculation (shrink with many tabs, min/max clamping)
- [x] Write tests for new-tab button "+" quad at right end
- [x] Write tests for tab separator quads
- [x] Implement `generate_tab_bar_quads()` in `src/tab/bar.rs`

### Task 2.3: Tab bar hit testing <!-- 73c0b42 -->
- [x] Write tests for click on tab → returns tab index
- [x] Write tests for click on "+" button → returns NewTab action
- [x] Write tests for click outside tab bar (y > 28) → returns None
- [x] Implement `hit_test_tab_bar()` in `src/tab/bar.rs`

### Phase 2 Completion
- [x] Phase completion verification and checkpointing

## Phase 3: App Integration & Visual Validation

### Task 3.1: Wire TabManager into App <!-- aa7df7b -->
- [x] Replace `App.pane_tree: PaneTree` with `App.tab_manager: TabManager`
- [x] Update all pane_tree references to go through `tab_manager.active_tab().pane_tree`
- [x] Update pane state cleanup: on close_tab(), remove all tab's PaneIds from pane_states
- [x] Add tab bar quad generation to RedrawRequested (before pane rendering)
- [x] Offset pane layout bounds by TAB_BAR_HEIGHT (28px)
- [x] Write integration tests for tab creation, switching, closing

### Task 3.2: Wire tab keyboard shortcuts <!-- aa7df7b -->
- [x] Add match_tab_command() check in KeyboardInput handler (before pane commands)
- [x] Update Ctrl+Shift+W: close pane if multiple panes, close tab if single pane
- [x] On NewTab: create tab + spawn pane with PTY
- [x] On tab switch: update interaction layout for new active tab's PaneTree
- [x] Write integration tests for keyboard-driven tab operations

### Task 3.3: Wire tab bar mouse interaction <!-- aa7df7b -->
- [x] Route CursorMoved/MouseInput to tab bar hit test when y < TAB_BAR_HEIGHT
- [x] Offset y-coordinate for pane interaction when y >= TAB_BAR_HEIGHT
- [x] Handle tab click → select_tab, "+" click → new_tab
- [x] Write integration tests for mouse-driven tab operations

### Task 3.4: Tab title rendering (deferred)
- [ ] ~~Render tab titles using glyph atlas text (reuse GridRenderer text capabilities)~~
- [x] Default title: "Shell" stored in Tab struct
- [ ] ~~Truncate long titles with ellipsis to fit tab width~~
- Note: Overlay pipeline only supports colored rectangles. Text rendering in tab bar requires extending the overlay shader to support textured quads. Deferred to a future track. Tab bar is functional via colored backgrounds + click interaction.

### Task 3.5: Visual Validation <!-- aa7df7b -->
- [x] Run application and validate via screenshots:
  - [x] Tab bar visible at top of window
  - [x] Active tab visually distinct from inactive tabs
  - [x] New tab creation via Ctrl+Shift+T
  - [x] Tab switching via Ctrl+Shift+1 keyboard shortcut
  - [x] Tab close (Ctrl+Shift+W on single-pane tab closes tab)
  - [x] Multiple tabs with pane splits inside

### Phase 3 Completion
- [x] Phase completion verification and checkpointing
