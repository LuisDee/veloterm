# Track 19: Tab Management Polish — Implementation Plan

## Phase 1: Platform Keybindings & Close Tab (FR-2, FR-3, FR-5, FR-6, FR-9)

Refactor keybindings to be platform-aware and add CloseTab command.

### Task 1.1: Platform-aware keybinding refactor
- [x] Refactor `match_tab_command()` to accept both Cmd (macOS) and Ctrl+Shift (Linux) modifiers <!-- 30f78ea -->
- [x] Add Cmd+T for NewTab on macOS (alongside existing Ctrl+Shift+T) <!-- 30f78ea -->
- [x] Add Cmd+1-9 for SelectTab on macOS (alongside existing Ctrl+Shift+1-9) <!-- 30f78ea -->
- [x] Add Cmd+Shift+[ for PrevTab, Cmd+Shift+] for NextTab on macOS <!-- 30f78ea -->
- [x] TDD: tests for platform-aware modifier matching on both macOS and Linux <!-- 30f78ea -->

### Task 1.2: CloseTab command and last-tab-closes-window
- [x] Add `CloseTab` variant to `TabCommand` <!-- a30504d -->
- [x] Add Cmd+W (macOS) keybinding <!-- a30504d -->
- [x] Wire `CloseTab` in `handle_tab_command()` via existing `handle_close_active_tab()` <!-- a30504d -->
- [x] TDD: tests for CloseTab keybinding matching, Ctrl+Shift+W stays as ClosePane <!-- a30504d -->

### Task 1.3: New Window command (Cmd+N)
- [x] Add `NewWindow` variant to `AppCommand` <!-- fb27eac -->
- [x] Add Cmd+N (macOS) / Ctrl+N (Linux) keybinding <!-- fb27eac -->
- [x] Implement: spawn new process via `std::process::Command` using `std::env::current_exe()` <!-- fb27eac -->
- [x] TDD: tests for keybinding matching (process spawn is integration-only) <!-- fb27eac -->

### Phase 1 Completion
- [x] Phase completion verification and checkpointing protocol

---

## Phase 2: Tab Close Button & Drag-to-Reorder (FR-1, FR-7)

Visual close button rendering and drag-to-reorder interaction.

### Task 2.1: Tab close button rendering
- [x] Add close button "×" text cell to `generate_tab_label_text_cells()` — right-aligned within each tab <!-- 0277018 -->
- [x] Active tab: always render close button <!-- 0277018 -->
- [x] Inactive tabs: render close button only when tab is hovered <!-- 0277018 -->
- [x] Add `CloseTab(usize)` variant to `TabBarAction` <!-- 0277018 -->
- [x] Update `hit_test_tab_bar()` to detect close button clicks (rightmost ~16px of tab) <!-- 0277018 -->
- [x] Wire close button click to close tab in window.rs <!-- 0277018 -->
- [x] TDD: tests for hit_test close button region, close button visibility logic <!-- 0277018 -->

### Task 2.2: Tab hover tracking
- [x] Track mouse hover state in tab bar: `hovered_tab: Option<usize>` <!-- 0277018 -->
- [x] Update hovered_tab on CursorMoved when y < TAB_BAR_HEIGHT <!-- 0277018 -->
- [x] Clear hovered_tab when cursor leaves tab bar <!-- 0277018 -->
- [x] Pass hovered_tab to `generate_tab_label_text_cells()` for close button visibility <!-- 0277018 -->

### Task 2.3: Drag-to-reorder tabs
- [x] Add tab drag state: `tab_drag_index`, `tab_drag_start_x`, `tab_drag_active` <!-- 3c5c95a -->
- [x] On mouse press on tab (not close button): begin drag tracking <!-- 3c5c95a -->
- [x] On CursorMoved while dragging: compute target tab index, call `move_tab()` <!-- 3c5c95a -->
- [x] On mouse release: end drag <!-- 3c5c95a -->
- [x] Minimum drag distance (5px) before activating <!-- 3c5c95a -->
- [x] TDD: tests for drag target calculation, swap logic <!-- 3c5c95a -->

### Phase 2 Completion
- [x] Phase completion verification and checkpointing protocol

---

## Phase 3: Intelligent Tab Titles & Visual Validation (FR-4, FR-8)

Process name detection, CWD fallback titles, and final integration.

### Task 3.1: Foreground process name detection
- [x] Add `foreground_process_name()` using macOS `proc_listchildpids` + `proc_pidpath` FFI <!-- 758fdb9 -->
- [x] Add `is_shell_process()` and `basename_from_path()` helpers <!-- 758fdb9 -->
- [x] Return `Option<String>` — None if detection fails <!-- 758fdb9 -->
- [x] Guard against subtraction overflow when num_pids == 0 <!-- 0c8da07 -->
- [x] Throttle FFI calls to 1Hz to avoid per-frame syscall overhead <!-- 0c8da07 -->
- [x] TDD: tests for shell detection, basename parsing, child_pid <!-- 758fdb9 -->

### Task 3.2: Intelligent tab title logic
- [x] Update `process_shell_updates()` with title priority: explicit (OSC) > process name > CWD > "Shell" <!-- 64a67d1 -->
- [x] If foreground process is a shell (zsh, bash, fish, sh): use CWD basename instead <!-- 64a67d1 -->
- [x] Truncate long titles with ellipsis ("…") to fit tab width <!-- 0277018 -->
- [x] Tab labels render title text instead of just numbers <!-- 0277018 -->

### Task 3.3: Final integration and visual validation
- [x] Build and launch via `./take-screenshot.sh` — no panic, app runs <!-- 0c8da07 -->
- [x] Verify close button visible on active tab <!-- 0c8da07 -->
- [x] Verify tab titles show process name or CWD <!-- 0c8da07 -->

### Phase 3 Completion
- [x] Phase completion verification and checkpointing protocol
