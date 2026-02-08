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
- [ ] Add `CloseTab` variant to `TabCommand`
- [ ] Add Cmd+W (macOS) / Ctrl+Shift+W (Linux) keybinding
- [ ] Modify `TabManager::close_tab()` to allow closing the last tab (return pane IDs always)
- [ ] Wire `CloseTab` in `handle_tab_command()`: close tab, clean up PTY, if last tab exit event loop
- [ ] TDD: tests for close_tab returning pane IDs, last-tab behavior, keybinding matching

### Task 1.3: New Window command (Cmd+N)
- [ ] Add `NewWindow` variant to `AppCommand`
- [ ] Add Cmd+N (macOS) / Ctrl+Shift+N (Linux) keybinding
- [ ] Implement: spawn new process via `std::process::Command` using `std::env::current_exe()`
- [ ] TDD: tests for keybinding matching (process spawn is integration-only)

### Phase 1 Completion
- [ ] Phase completion verification and checkpointing protocol

---

## Phase 2: Tab Close Button & Drag-to-Reorder (FR-1, FR-7)

Visual close button rendering and drag-to-reorder interaction.

### Task 2.1: Tab close button rendering
- [ ] Add close button "×" text cell to `generate_tab_label_text_cells()` — right-aligned within each tab
- [ ] Active tab: always render close button
- [ ] Inactive tabs: render close button only when tab is hovered (track hovered tab index)
- [ ] Add `CloseTab(usize)` variant to `TabBarAction`
- [ ] Update `hit_test_tab_bar()` to detect close button clicks (rightmost ~16px of tab)
- [ ] Wire close button click to `handle_tab_command(CloseTab)` in window.rs
- [ ] TDD: tests for hit_test close button region, close button visibility logic

### Task 2.2: Tab hover tracking
- [ ] Track mouse hover state in tab bar: `hovered_tab: Option<usize>`
- [ ] Update hovered_tab on CursorMoved when y < TAB_BAR_HEIGHT
- [ ] Clear hovered_tab when cursor leaves tab bar
- [ ] Pass hovered_tab to `generate_tab_label_text_cells()` for close button visibility
- [ ] TDD: tests for hover index calculation from cursor position

### Task 2.3: Drag-to-reorder tabs
- [ ] Add tab drag state: `is_dragging_tab: bool`, `drag_tab_index: usize`, `drag_start_x: f32`
- [ ] On mouse press on tab (not close button): begin drag
- [ ] On CursorMoved while dragging: compute target tab index from cursor x; if different from current, call `TabManager::move_tab()`
- [ ] On mouse release: end drag
- [ ] Minimum drag distance before activating (prevent accidental drags on click)
- [ ] TDD: tests for drag target calculation, swap logic

### Phase 2 Completion
- [ ] Phase completion verification and checkpointing protocol

---

## Phase 3: Intelligent Tab Titles & Visual Validation (FR-4, FR-8)

Process name detection, CWD fallback titles, and final integration.

### Task 3.1: Foreground process name detection
- [ ] Add `foreground_process_name()` method to PtySession — queries the PTY's foreground process group
- [ ] On macOS/Linux: use `tcgetpgrp()` to get foreground PID, then read `/proc/{pid}/comm` (Linux) or `proc_pidpath` (macOS)
- [ ] Return `Option<String>` — None if detection fails
- [ ] TDD: tests for process name parsing (mock the syscall results)

### Task 3.2: Intelligent tab title logic
- [ ] Update `process_shell_updates()` to set tab titles with priority: process name > CWD > "Tab N"
- [ ] If foreground process is a shell (zsh, bash, fish, sh): use CWD basename instead
- [ ] Truncate long titles with ellipsis ("…") to fit tab width
- [ ] Update `generate_tab_label_text_cells()` to render tab title text instead of just numbers
- [ ] TDD: tests for title priority logic, shell detection, truncation

### Task 3.3: Final integration and visual validation
- [ ] Build and launch via `./take-screenshot.sh`
- [ ] Verify close button visible on active tab
- [ ] Verify tab titles show process name or CWD
- [ ] Verify Cmd+W, Cmd+T, Cmd+N, Cmd+1-9 work
- [ ] Verify drag-to-reorder works

### Phase 3 Completion
- [ ] Phase completion verification and checkpointing protocol
