# VeloTerm: MCP Fix, File Browser Overhaul, Git Review Overhaul

## Executive Summary

Four major workstreams in priority order:
1. **Phase 0**: Fix VeloTerm MCP launch — unblock visual testing
2. **Phase 1**: File Browser Overhaul — CWD, click navigation, icons, viewer, selection
3. **Phase 2**: Git Review Overhaul — CWD, validation, staging UI, diff viewer
4. **Phase 3**: Visual review & iteration via MCP screenshots

TDD throughout: write failing tests first, implement until green, then visually verify.

**Rollback plan**: If MCP fix takes longer than expected, Phases 1-2 proceed using `./take-screenshot.sh` + manual osascript for visual testing (proven working method per CLAUDE.md).

---

## Pre-flight: TODO 0.0 — Verify Existing Test Suite
- [ ] Run full test suite: `cargo test` — all 2013 tests must pass before ANY changes
- [ ] Run `git status` — commit any uncommitted work
- [ ] Document baseline: test count, compilation warnings

---

## Phase 0: Fix VeloTerm MCP Launch

### Root Cause Analysis

The MCP server implementation is architecturally correct — it creates the `.app` bundle, sets `VELOTERM_PROJECT_DIR`, copies `Info.plist`, and launches via `open`. The failure occurs in **window detection** during the polling loop.

**Diagnosis**: After `open VeloTerm.app`:
1. macOS spawns VeloTerm via the wrapper script
2. The wrapper `exec`s `veloterm-bin`, which replaces the shell process
3. `findPid()` uses `pgrep -x veloterm-bin` — this should find the process
4. `findWindowByPid(pid)` uses JXA to find a window with matching PID and layer=0

**Suspected issues**:
- **Issue A**: JXA automation permissions — the JXA script uses `ObjC.import('CoreGraphics')` and `CGWindowListCopyWindowInfo` which may require TCC (Transparency, Consent, and Control) permission. If the MCP server's Node.js process hasn't been granted Screen Recording or Accessibility access, the JXA call silently returns an empty window list.
- **Issue B**: `kCGWindowLayer === 0` may not match — wgpu/winit windows on macOS may have a different layer value during GPU surface creation or when the window first appears.
- **Issue C**: The `open` command returns immediately but app launch is async. `spawn("open", ...).unref()` provides no way to know when the app has registered. Early polls fail, and by the time the app is ready, the poll may miss it or the process name may differ.
- **Issue D**: When launched via `open`, macOS may track the process under the bundle executable name (`veloterm`, the wrapper) rather than `veloterm-bin`. Since `pgrep -x veloterm-bin` matches exact process name, it could miss the process.

### Fix Plan

#### TODO 0.1: Add diagnostic logging to MCP server
- [ ] Add `console.error()` logging in `findPid()` to show raw `pgrep` stdout/stderr
- [ ] Add `console.error()` logging in `findWindowByPid()` to show JXA script raw output
- [ ] Log each poll iteration: `poll #N: pid=X|null, windowId=Y|null`
- [ ] Check TCC permissions: test `osascript -l JavaScript -e 'ObjC.import("CoreGraphics"); $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)'` manually
- [ ] Rebuild MCP: `cd mcp-server && npm run build`

#### TODO 0.2: Fix PID detection
- [ ] Test `pgrep -x veloterm-bin` manually while VeloTerm is running via `open`
- [ ] Test `pgrep -f veloterm-bin` (matches command line, not just process name)
- [ ] Test `pgrep -f "VeloTerm.app"` to catch the bundle launcher process
- [ ] After finding PID, verify with `ps -p $PID -o comm=` to confirm process name
- [ ] **Fix**: Change `pgrep -x veloterm-bin` to `pgrep -f veloterm-bin` as primary, with `-x` as secondary
- [ ] **Fix**: Add multiple pgrep strategies in order: `-x veloterm-bin`, `-f veloterm-bin`, `-f VeloTerm`

#### TODO 0.3: Fix window detection
- [ ] Test JXA script manually via `osascript -l JavaScript` with known PID from `pgrep`
- [ ] **Diagnostic**: List ALL windows with JXA — log `kCGWindowLayer`, `kCGWindowOwnerPID`, `kCGWindowOwnerName`, `kCGWindowBounds` for all windows
- [ ] If JXA returns empty list: TCC permissions issue — suggest granting Screen Recording access to Terminal.app / Claude Code
- [ ] **Fix**: Relax window matching — remove `kCGWindowLayer === 0`, match on PID + `kCGWindowBounds` height > 100 (filter out tiny helper windows)
- [ ] **Fix**: Add owner name matching as additional strategy: `kCGWindowOwnerName === 'VeloTerm'` OR `kCGWindowOwnerName === 'veloterm-bin'`
- [ ] Ensure GetWindowID fallback uses correct args: `GetWindowID "VeloTerm" "VeloTerm"`

#### TODO 0.4: Fix launch mechanism
- [ ] **Fix**: Replace `spawn("open", [APP_PATH]).unref()` with `execSync("open --wait-apps " + APP_PATH)` — this blocks until the app registers with Launch Services (PID is guaranteed to exist after)
- [ ] Add 500ms delay after `open --wait-apps` returns before first window poll (window may not exist yet even though process does)
- [ ] Reduce poll interval from 1000ms to 500ms for faster window detection
- [ ] Keep 30s total timeout as safety net

#### TODO 0.5: Rebuild and verify end-to-end
- [ ] `cd mcp-server && npm run build`
- [ ] Test `veloterm_launch` MCP tool — must succeed within 15 seconds
- [ ] Test `veloterm_screenshot` — must return valid PNG showing VeloTerm window
- [ ] Test `veloterm_type("echo hello")` + `veloterm_key("enter")` — must execute command
- [ ] Test `veloterm_key("ctrl+c")` — must send interrupt
- [ ] Test `veloterm_kill` — must terminate cleanly, subsequent `veloterm_launch` works

### Acceptance Criteria (Phase 0)
- `veloterm_launch` succeeds on first attempt within 15 seconds
- `veloterm_screenshot` returns a valid PNG with VeloTerm window content
- `veloterm_type` + `veloterm_key(enter)` executes commands in VeloTerm
- `veloterm_kill` terminates the process cleanly
- Three consecutive launch → screenshot → kill cycles all succeed

---

## Phase 1: File Browser Overhaul

### Current State Analysis

The file browser implementation is **more complete than initially reported**:
- CWD detection via OSC 7 IS implemented (`active_pane_cwd()` at `window.rs:490`)
- Click to expand/collapse IS implemented (`handle_row_click()` at `file_browser/mod.rs:130`)
- File preview with syntax highlighting IS implemented (`preview.rs`)
- File type icons ARE implemented (`tree.rs:309-329`)
- 240+ tests already exist across the module

**Why it appears broken**:
1. OSC 7 depends on the shell emitting it — bash doesn't do this by default. The shell integration script (`shell/bash-integration.sh`) exists and sets up `PROMPT_COMMAND` to emit OSC 7, but it's NOT being injected into the PTY automatically.
2. The PTY sets `TERM_PROGRAM=VeloTerm` but does NOT inject shell integration scripts. The user's shell (bash) would need to source `bash-integration.sh` via `.bashrc` or VeloTerm would need to inject it.
3. Single click = expand OR open file (via Enter action). There's no separate double-click behavior.
4. Text in the preview pane is rendered but not selectable/copyable.

### Implementation Plan

#### TODO 1.0: Research IDE File Explorers
**Tools**: Context7, WebSearch, sequential thinking

Before implementing, conduct thorough research into IDE file explorer features:
- [ ] Query Context7 for VS Code explorer API documentation
- [ ] WebSearch for "VS Code file explorer features 2026"
- [ ] WebSearch for "terminal file manager UX best practices"
- [ ] Study yazi, broot, and ranger feature sets via their docs
- [ ] Build a feature comparison matrix: VS Code vs JetBrains vs Zed vs current VeloTerm
- [ ] Prioritize features by: impact for terminal users, implementation complexity, visual distinctiveness
- [ ] Update TODO 1.3-1.7 based on research findings

#### TODO 1.1: Fix CWD Detection — Shell Integration Injection
**Files**: `src/pty/mod.rs`, `shell/bash-integration.sh`, `shell/zsh-integration.sh`

The PTY must inject shell integration automatically. After spawning the shell:
1. Write the shell integration script to the PTY master
2. This sets up `PROMPT_COMMAND` for bash / `precmd` for zsh / `fish_prompt` for fish to emit OSC 7
3. The scripts already exist: `shell/bash-integration.sh`, `shell/zsh-integration.sh`

**Tests**:
- [ ] `test_pty_injects_shell_integration_bash` — spawn bash PTY, read output, verify OSC 7 emitted after first prompt
- [ ] `test_pty_injects_shell_integration_zsh` — same for zsh
- [ ] `test_pty_injects_shell_integration_fish` — same for fish (uses `--on-variable PWD` hook)
- [ ] `test_active_pane_cwd_updates_after_cd` — feed OSC 7 bytes, verify `shell_state().cwd` updates
- [ ] `test_file_browser_opens_at_pane_cwd` — open file browser after CWD update, verify root matches
- [ ] `test_git_review_discovers_repo_from_pane_cwd` — open git review after CWD update
- [ ] `test_cwd_fallback_when_osc7_not_emitted` — if shell doesn't emit OSC 7, verify fallback to HOME (not crash)
- [ ] `test_shell_integration_doesnt_break_existing_prompt_command` — inject into bash that already has PROMPT_COMMAND set
- [ ] `test_unknown_shell_graceful_fallback` — if shell is nushell/elvish/unknown, don't crash, just skip injection

**Implementation**:
- After `pair.slave.spawn_command(cmd)`, detect shell from command path basename
- For bash: write `source /dev/stdin <<'VELOTERM_INIT'\n<bash-integration.sh contents>\nVELOTERM_INIT\n` to PTY master
- For zsh: similar with zsh hooks
- For fish: use `function __veloterm_cwd --on-variable PWD` pattern
- For unknown shells: log warning, skip injection (graceful degradation)
- Wait a brief delay (50ms) before writing to let the shell initialize

#### TODO 1.2: Mouse Click Improvements — Single vs Double Click
**Files**: `src/window.rs`, `src/file_browser/mod.rs`

Current: single click on a row calls `handle_row_click()` which calls `TreeNavAction::Enter` — this both selects AND performs the action (expand dir / open file). This needs to be split.

**Breaking change audit**: `handle_row_click` is called from:
1. `window.rs` mouse click handler (line ~3568) — primary caller, needs splitting
2. Tests in `file_browser/mod.rs` — need updating to match new behavior

**Tests**:
- [ ] `test_single_click_selects_row` — click on any row, verify selected_visible_row updates
- [ ] `test_single_click_does_not_expand_directory` — click collapsed dir, verify still collapsed
- [ ] `test_single_click_does_not_open_preview` — click file, verify preview NOT loaded
- [ ] `test_double_click_expands_directory` — two clicks within 500ms on collapsed dir → expanded
- [ ] `test_double_click_collapses_expanded_directory` — two clicks on expanded dir → collapsed
- [ ] `test_double_click_opens_file_preview` — two clicks on file → preview loaded in right pane
- [ ] `test_double_click_timing_threshold` — clicks at exactly 500ms apart = double-click
- [ ] `test_single_click_after_timeout` — click, wait 600ms, click = two separate single clicks
- [ ] `test_click_on_chevron_always_toggles` — clicking in chevron area (first 20px) toggles expand regardless of single/double
- [ ] `test_double_click_state_reset_on_different_row` — click row 1, then row 3 within 500ms = NOT double-click

**Implementation**:
- Add to `FileTreeViewState`: `last_click_time: Option<Instant>`, `last_click_row: Option<usize>`
- New method: `handle_click(visible_row_idx, click_x)` — returns `ClickAction` enum:
  - `Select(row)` — single click, just highlight
  - `Toggle(row)` — double-click on directory or click on chevron
  - `Open(path)` — double-click on file
- `handle_row_click()` is replaced by `handle_single_click()` and `handle_double_click()`
- `window.rs` tracks double-click timing and calls the appropriate method

#### TODO 1.3: Enhanced File Type Icons
**Files**: `src/file_browser/tree.rs`

Current icons use basic Unicode. Enhance with better Unicode symbols and add color hints.

**Tests**:
- [ ] `test_icon_for_rust_file` — `.rs` → Rust-specific icon
- [ ] `test_icon_for_javascript` — `.js` → JS icon
- [ ] `test_icon_for_typescript` — `.ts` → TS icon
- [ ] `test_icon_for_python` — `.py` → Python icon
- [ ] `test_icon_for_markdown` — `.md` → Markdown icon
- [ ] `test_icon_for_json` — `.json` → JSON icon
- [ ] `test_icon_for_toml` — `.toml` → config icon
- [ ] `test_icon_for_yaml` — `.yml`/`.yaml` → config icon
- [ ] `test_icon_for_image` — `.png`/`.jpg` → image icon
- [ ] `test_icon_for_binary` — `.exe`/`.dll` → binary icon
- [ ] `test_icon_for_directory_collapsed` — closed folder icon
- [ ] `test_icon_for_directory_expanded` — open folder icon
- [ ] `test_icon_for_gitignore` — `.gitignore` → git icon
- [ ] `test_icon_for_dockerfile` — `Dockerfile` → Docker icon
- [ ] `test_icon_for_lock_file` — `Cargo.lock`/`package-lock.json` → lock icon
- [ ] `test_icon_for_unknown_extension` — fallback generic file icon
- [ ] `test_icon_color_hint_source_code` — source files get blue-ish color hint
- [ ] `test_icon_color_hint_config` — config files get yellow color hint
- [ ] `test_icon_color_hint_image` — image files get magenta color hint

**Implementation**:
- Expand icon mapping in `file_icon()` function
- Return `IconInfo { char: char, color_hint: Option<Color> }` instead of just `char`
- Group: source code (blue), config (yellow), data (cyan), image (magenta), binary (red), archive (orange), git (orange), docker (blue), docs (green)
- Use Unicode symbols from Mathematical and Technical blocks that render well in JetBrains Mono

#### TODO 1.4: Indent Guides
**Files**: `src/file_browser/view.rs`, renderer integration

**Tests**:
- [ ] `test_indent_guide_not_rendered_at_depth_0` — root items have no guide
- [ ] `test_indent_guide_at_depth_1` — one vertical connector
- [ ] `test_indent_guide_at_depth_3` — three vertical connectors
- [ ] `test_indent_guide_last_child_uses_corner` — `└─` for last sibling
- [ ] `test_indent_guide_middle_child_uses_tee` — `├─` for non-last sibling
- [ ] `test_indent_guide_continuation_when_parent_has_more_children` — `│` continues down
- [ ] `test_indent_guides_for_mixed_tree` — complex tree with varying depths renders correctly

**Implementation**:
- Add `is_last_child: bool` field to `VisibleRow`
- Add `ancestor_has_next_sibling: Vec<bool>` to `VisibleRow` — one bool per depth level
- In rendering, draw: `│ ` for ancestors with next siblings, `  ` for ancestors without, `├─` for current non-last, `└─` for current last
- Use Unicode box-drawing characters: `│` (U+2502), `├` (U+251C), `└` (U+2514), `─` (U+2500)

#### TODO 1.5: Text Selection & Copy in Preview Pane
**Files**: `src/file_browser/preview.rs`, `src/window.rs`

Current: text in preview is rendered but not interactive.

**TextSelection struct definition**:
```rust
pub struct TextSelection {
    pub start_line: usize,      // 0-indexed line number
    pub start_char: usize,      // character offset within line (not pixel)
    pub end_line: usize,
    pub end_char: usize,
    pub active: bool,           // true while mouse is being dragged
}

impl TextSelection {
    pub fn extract_text(&self, lines: &[String]) -> String { ... }
    pub fn contains_line(&self, line: usize) -> bool { ... }
    pub fn line_selection_range(&self, line: usize) -> Option<(usize, usize)> { ... }
}
```

**Pixel-to-character mapping**: Since the preview uses monospace font (JetBrains Mono), character offset = `(click_x - gutter_width) / cell_width`. Line = `(click_y + scroll_offset) / row_height`. This is exact for monospace rendering.

**Tests**:
- [ ] `test_text_selection_struct_extract_single_line` — select part of one line, verify text
- [ ] `test_text_selection_struct_extract_multi_line` — select across lines, verify text with newlines
- [ ] `test_text_selection_struct_contains_line` — boundary checks
- [ ] `test_text_selection_struct_line_range` — partial line selection ranges
- [ ] `test_preview_click_sets_cursor` — click in preview text, verify cursor position
- [ ] `test_preview_click_drag_creates_selection` — click+drag selects text range
- [ ] `test_preview_selection_range_correct` — verify start/end positions match
- [ ] `test_preview_copy_selection_to_clipboard` — Cmd+C copies selected text
- [ ] `test_preview_select_all` — Cmd+A selects all preview text
- [ ] `test_preview_selection_cleared_on_new_file` — opening new file clears selection
- [ ] `test_preview_selection_on_empty_file` — no crash on empty file
- [ ] `test_preview_selection_on_binary_file` — no selection possible on binary files
- [ ] `test_preview_copy_single_line` — select one line, copy, verify clipboard content
- [ ] `test_preview_copy_multiline` — select across lines, verify clipboard has newlines

**Implementation**:
- Add `selection: Option<TextSelection>` to `PreviewViewState`
- On mouse down in preview: set selection start (compute line/char from pixel position using cell_width)
- On mouse drag: update selection end
- On mouse up: finalize selection
- Render selection highlight as colored rectangle overlay on selected character ranges
- Cmd+C / Ctrl+C (when preview focused) → `selection.extract_text()` → clipboard via `arboard`
- Cmd+A / Ctrl+A → select all lines

#### TODO 1.6: Page Up/Down and Scroll Improvements
**Files**: `src/file_browser/mod.rs`, `src/file_browser/view.rs`

**Tests**:
- [ ] `test_page_down_moves_by_viewport_height` — PageDown advances by visible rows count
- [ ] `test_page_up_moves_by_viewport_height` — PageUp goes back by visible rows count
- [ ] `test_page_down_at_bottom_stops` — doesn't scroll past last row
- [ ] `test_page_up_at_top_stops` — doesn't scroll above first row
- [ ] `test_half_page_scroll_ctrl_d` — Ctrl+D moves half viewport
- [ ] `test_half_page_scroll_ctrl_u` — Ctrl+U moves half viewport
- [ ] `test_mouse_scroll_in_tree` — mouse wheel scrolls file tree
- [ ] `test_mouse_scroll_in_preview` — mouse wheel scrolls preview independently

**Implementation**:
- Add `PageUp`, `PageDown` key handling in `handle_nav_action()`
- Calculate viewport row count from panel height / row height
- Add `Ctrl+D` / `Ctrl+U` for half-page scroll
- Wire mouse wheel events to scroll the focused panel

#### TODO 1.7: Additional IDE Features
**Files**: various file_browser modules

**Tests**:
- [ ] `test_search_slash_enters_search_mode` — pressing `/` in tree activates search
- [ ] `test_search_filters_visible_rows` — typing narrows the tree to matching entries
- [ ] `test_search_escape_exits_search` — Escape clears search, restores full tree
- [ ] `test_compact_folders` — single-child directory chains shown as `src/utils/` one row
- [ ] `test_compact_folder_expand_shows_children` — expanding compact folder shows real children
- [ ] `test_reveal_file_in_tree` — after opening a file, tree scrolls to show it
- [ ] `test_file_size_in_metadata` — file size shown in preview metadata (ALREADY DONE — verify)
- [ ] `test_modification_time_in_metadata` — mod time shown in preview metadata (ALREADY DONE — verify)
- [ ] `test_git_status_indicators_in_tree` — modified files show M indicator, untracked show U (ALREADY DONE — verify)
- [ ] `test_git_status_color_coding` — modified=yellow, untracked=green, ignored=dim

#### TODO 1.8: Conductor Track Dashboard CWD
**Files**: `src/window.rs`, relevant conductor integration code

The conductor track dashboard should also respect the active pane's CWD when displaying tracks.

**Tests**:
- [ ] `test_conductor_dashboard_uses_active_pane_cwd` — verify dashboard reads from active pane CWD
- [ ] `test_conductor_dashboard_fallback_to_home` — if CWD not available, use HOME

**Implementation**:
- Audit the conductor dashboard opening code in `window.rs`
- Ensure it calls `active_pane_cwd()` like file browser and git review do
- If not, wire it up following the same pattern

### Acceptance Criteria (Phase 1)
- [ ] File browser opens at the active pane's CWD (not root or HOME)
- [ ] Single click selects, double click opens/expands
- [ ] Click on chevron always toggles expand/collapse
- [ ] All file types have distinct, recognizable icons with color hints
- [ ] Indent guides show parent-child relationships clearly
- [ ] Text in preview pane is selectable and copyable (Cmd+C)
- [ ] Page Up/Down work in both tree and preview panels
- [ ] `/` activates search filter in tree
- [ ] Git status indicators visible on modified/untracked files
- [ ] Conductor dashboard respects CWD
- [ ] All new tests pass (target: 80+ new tests)

---

## Phase 2: Git Review Overhaul

### Current State Analysis

The git review implementation is **substantially complete** with 252 existing tests:
- CWD-based repo discovery via `git2::Repository::discover()` IS implemented
- Three-section file listing (staged/changed/untracked) IS implemented
- Click to show diff IS implemented with side-by-side alignment
- Syntax highlighting and inline word-level diffs ARE implemented
- Stage/unstage/discard operations ARE implemented

**Why it appears broken**:
1. Same CWD issue as file browser — without shell integration injection, `active_pane_cwd()` returns HOME, and if HOME isn't a git repo, git review shows "Not in a git repository"
2. The user needs to navigate to a git repo directory in their shell first
3. Some features (hunk-level staging, mouse click in diff) are coded but not wired to UI

### Implementation Plan

#### TODO 2.1: Fix CWD Detection (shared with Phase 1)
This is the SAME fix as TODO 1.1 — shell integration injection. Once OSC 7 works, both file browser and git review will get correct CWD.

**Tests** (in addition to TODO 1.1):
- [ ] `test_git_review_opens_from_pane_cwd` — cd to git repo, open git review, verify repo found
- [ ] `test_git_review_error_when_not_in_repo` — cd to non-repo dir, verify error message shown
- [ ] `test_git_review_follows_cd` — cd to different repo, reopen, verify new repo
- [ ] `test_git_review_fallback_when_cwd_not_available` — if no OSC 7, show clear error message

#### TODO 2.2: Validate File List Sections
**Files**: `src/git_review/status.rs`, `src/git_review/view.rs`

Most of this is already implemented and tested (252 tests). Focus on verifying and filling gaps.

**Tests**:
- [ ] `test_staged_section_shows_added_files` — `git add` a file, verify in staged section
- [ ] `test_changed_section_shows_modified_files` — modify tracked file, verify in changed section
- [ ] `test_untracked_section_shows_new_files` — create new file, verify in untracked
- [ ] `test_section_headers_show_counts` — "Staged (3)" header shows correct count
- [ ] `test_section_collapse_hides_files` — collapse staged section, verify files hidden
- [ ] `test_section_expand_shows_files` — expand collapsed section, verify files visible
- [ ] `test_deleted_file_shows_in_changed` — delete tracked file, verify shows with D status
- [ ] `test_renamed_file_shows_in_staged` — `git mv`, verify shows with R status
- [ ] `test_empty_sections_hidden` — if no staged files, staged section header not shown

#### TODO 2.3: Click-to-Diff in File List
**Files**: `src/window.rs`, `src/git_review/mod.rs`

**Tests**:
- [ ] `test_click_staged_file_shows_diff` — click file in staged, right pane shows HEAD→index diff
- [ ] `test_click_changed_file_shows_diff` — click file in changed, shows index→workdir diff
- [ ] `test_click_untracked_file_shows_content` — click untracked file, right pane shows all-green
- [ ] `test_diff_shows_line_numbers` — verify line numbers in diff gutter
- [ ] `test_diff_shows_add_delete_colors` — added=green, deleted=red
- [ ] `test_diff_context_lines_shown` — unchanged context lines visible around changes
- [ ] `test_diff_hunk_headers_shown` — `@@ -1,3 +1,5 @@` headers visible
- [ ] `test_diff_word_level_highlighting` — inline word diffs highlighted within changed lines
- [ ] `test_diff_binary_file_message` — binary files show "Binary file changed" message
- [ ] `test_click_different_file_updates_diff` — clicking another file replaces diff content

#### TODO 2.4: Mouse Support in Diff View
**Files**: `src/window.rs`, `src/git_review/diff_view.rs`

**Tests**:
- [ ] `test_diff_view_mouse_scroll` — mouse wheel scrolls diff vertically
- [ ] `test_diff_view_click_does_not_crash` — click in diff area doesn't panic
- [ ] `test_diff_view_horizontal_scroll` — horizontal scroll for long lines

#### TODO 2.5: Wire Hunk-Level Staging UI
**Files**: `src/window.rs`, `src/git_review/mod.rs`, `src/git_review/hunk_staging.rs`

The hunk staging logic is already implemented and tested in `hunk_staging.rs`. Wire it to the keyboard/UI.

**Tests**:
- [ ] `test_hunk_header_shows_stage_button` — hunk headers have (s)tage / (u)nstage action hint
- [ ] `test_press_s_on_hunk_stages_it` — navigate to hunk, press `s`, hunk moves to staged
- [ ] `test_press_u_on_hunk_unstages_it` — navigate to hunk in staged, press `u`, hunk removed
- [ ] `test_hunk_collapse_expand` — press Enter on hunk header toggles collapse

#### TODO 2.6: Git Status Auto-Refresh
**Files**: `src/git_review/mod.rs`, `src/window.rs`

**Tests**:
- [ ] `test_status_refreshes_after_stage` — stage a file, verify list updates immediately
- [ ] `test_status_refreshes_after_unstage` — unstage, verify update
- [ ] `test_status_refreshes_after_discard` — discard, verify update
- [ ] `test_status_refreshes_after_commit` — commit, verify staged section clears

### Acceptance Criteria (Phase 2)
- [ ] Git review opens correctly when CWD is in a git repository
- [ ] Shows clear error message when CWD is not in a git repo
- [ ] Three sections (staged/changed/untracked) with correct file counts
- [ ] Clicking a file shows its diff in the right pane
- [ ] Diff has proper coloring (green=add, red=delete)
- [ ] Keyboard shortcuts (s/u/d/S/U/c) all work
- [ ] Hunk-level staging accessible via keyboard
- [ ] Mouse scroll works in diff view
- [ ] All new tests pass (target: 30+ new tests)

---

## Phase 3: Visual Review & Iteration

### Process

After Phases 0-2 are implemented, use the MCP tools for systematic visual review:

#### TODO 3.1: File Browser Visual Review
- [ ] `veloterm_launch` → `veloterm_type("cd ~/work/terminal-em")` → `veloterm_key("enter")`
- [ ] Open file browser with Ctrl+E (or configured binding)
- [ ] `veloterm_screenshot` — verify tree shows project directory, not root/HOME
- [ ] Navigate to `src/main.rs`, double-click to open
- [ ] `veloterm_screenshot` — verify syntax-highlighted preview with line numbers
- [ ] Check: indent guides visible, icons distinct, selection highlighting works
- [ ] Check: text is selectable in preview
- [ ] Compare against VS Code feature matrix from research
- [ ] Document issues → create fix plan

#### TODO 3.2: Git Review Visual Review
- [ ] `veloterm_type("cd ~/work/terminal-em")` → modify a file → `git add` another
- [ ] Open git review with Ctrl+G
- [ ] `veloterm_screenshot` — verify three sections with correct files
- [ ] Navigate to a modified file, select it
- [ ] `veloterm_screenshot` — verify diff colors, line numbers, hunk headers
- [ ] Test stage (s) / unstage (u) keyboard shortcuts
- [ ] Test commit flow
- [ ] Document issues → create fix plan

#### TODO 3.3: Iteration Cycle
- [ ] Fix all issues found in visual review
- [ ] Re-run full test suite — all must pass
- [ ] Take final screenshots comparing before/after
- [ ] Verify against VS Code feature checklist from research
- [ ] Judge: "has this been implemented to its fullest potential?"
- [ ] If not: create another fix plan and iterate

---

## Agent Team Structure

### Team of 7 Agents

| Agent | Role | Phase | After Phase Completion |
|-------|------|-------|----------------------|
| **Lead** | Orchestrator | All | Coordinates, tracks progress |
| **MCP-Fixer** | Phase 0 specialist | 0 | Reassigned to Phase 3 visual testing |
| **Shell-Integrator** | CWD fix | 1.1/2.1 | Reassigned to help Phase 1 or 2 |
| **File-Browser-Dev** | File browser features | 1.2-1.4, 1.6-1.8 | Phase 3 file browser review |
| **Preview-Dev** | Preview pane | 1.5 | Help with git review diff view |
| **Git-Review-Dev** | Git review features | 2.2-2.6 | Phase 3 git review |
| **Reviewer** | Quality gate | All | Reviews every PR/commit |

### Workflow Per Agent
1. Read assigned TODO items and existing code
2. Write failing tests (TDD red phase)
3. Run tests to confirm they fail (`test-runner` skill)
4. Implement minimum code to pass
5. Run full test suite (all 2013+ tests)
6. Commit and push
7. Request review from Reviewer agent
8. Fix review feedback
9. Visual verification via MCP (Phase 3)

---

## Dependencies

```
TODO 0.0 (Pre-flight: verify tests)
  ↓
Phase 0 (MCP Fix) ← can proceed to Phase 1 using take-screenshot.sh if stuck
  ↓
TODO 1.0 (Research) ← informs feature list for Phase 1
  ↓
TODO 1.1 (Shell Integration) ← CRITICAL PATH: blocks ALL CWD features
  ↓
┌─────────────────────────┬─────────────────┐
│ TODO 1.2-1.8            │ TODO 2.2-2.6    │
│ (File Browser features) │ (Git Review)    │
│ Can run in parallel     │ Can run parallel│
└─────────────────────────┴─────────────────┘
  ↓                          ↓
Phase 3 (Visual Review & Iteration)
```

## Test Count Targets

| Phase | New Tests | Running Total |
|-------|-----------|---------------|
| Current | — | 2013 |
| Phase 0 | 0 (manual verification) | 2013 |
| Phase 1 | ~90 | ~2103 |
| Phase 2 | ~30 | ~2133 |
| **Target** | **~120** | **~2133+** |

## Key Files Modified

| File | Phase | Changes |
|------|-------|---------|
| `mcp-server/src/services/process.ts` | 0 | Fix PID detection, use `open --wait-apps`, add logging |
| `mcp-server/src/services/window.ts` | 0 | Relax window matching, add diagnostics |
| `src/pty/mod.rs` | 1.1 | Inject shell integration scripts after shell spawn |
| `src/file_browser/mod.rs` | 1.2 | Split single/double click, new click handling |
| `src/file_browser/tree.rs` | 1.3 | Enhanced icon mapping with color hints |
| `src/file_browser/view.rs` | 1.4, 1.6 | Indent guides, Page Up/Down, VisibleRow changes |
| `src/file_browser/preview.rs` | 1.5 | TextSelection struct, click/drag/copy |
| `src/window.rs` | 1.2, 1.8, 2.3, 2.4 | Double-click timing, conductor CWD, mouse in diff |
| `src/git_review/mod.rs` | 2.5 | Wire hunk staging to keyboard handler |
| `src/git_review/diff_view.rs` | 2.4 | Mouse click/scroll support |

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| MCP fix takes too long | Use `take-screenshot.sh` + osascript as fallback for visual testing |
| Shell integration injection breaks user shells | Detect shell type, wrap in guard (`if [ -z "$VELOTERM_SHELL_INTEGRATION" ]`), test with bash/zsh/fish |
| `handle_row_click` breaking change | Audit all callers, update tests, keep backward-compatible API with deprecation |
| Preview text selection needs pixel-to-char mapping | Monospace font means simple `x / cell_width` math — no complex glyph measurement needed |
| TCC permissions block JXA window detection | Add `GetWindowID` as primary fallback, document permission requirements |
