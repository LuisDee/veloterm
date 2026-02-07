# Execution Sequence

> Wave-based ordering derived from the dependency graph.
> Tracks within the same wave are independent and can run in parallel.
> All tracks in a wave must complete before the next wave starts.

---

## Wave 0 — Foundation (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 01 | 01_window_gpu | XL | Window creation, GPU rendering pipeline, glyph atlas |
| 02 | 02_core_terminal | XL | PTY, terminal emulation, input, cursor, colors, scrollback, selection, clipboard, resize |

### Wave 0 Status: COMPLETE
- 290 tests passing
- All quality gates met

---

## Wave 1 — Configuration & Optimization

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 03 | 03_config | M | TOML config system with hot-reload, theme switching, keybinding definitions |
| 07 | 07_perf_damage | M | Dirty cell tracking, selective buffer updates, frame budget optimization |

### Wave 1 Completion Criteria
- [ ] Config file loads and applies all settings
- [ ] Hot-reload detects changes and applies without restart
- [ ] Invalid config keeps previous state with logged warning
- [ ] Damage tracking reduces per-frame GPU buffer updates to dirty cells only

---

## Wave 2 — Pane Layout Engine

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 04 | 04_pane_layout | L | Binary tree pane layout, split/close operations, focus management, layout calculation |

### Wave 2 Completion Criteria
- [ ] Vertical and horizontal splits create new panes with independent terminals
- [ ] Closing a pane collapses the tree correctly
- [ ] Focus switching works via keyboard shortcuts
- [ ] Layout calculation correctly distributes pixel space

---

## Wave 3 — Pane UI & Tabs

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 05 | 05_pane_ui | M | Divider bar rendering, drag-to-resize, mouse click focus, pane zoom |
| 06 | 06_tabs | M | Tab bar rendering, tab management, independent pane tree per tab |

### Wave 3 Completion Criteria
- [ ] Divider bars render between panes and are draggable
- [ ] Pane zoom temporarily maximizes a pane
- [ ] Tab bar renders with tab titles
- [ ] New/close/switch tabs work via keyboard and mouse

---

## Wave 4 — Developer Workflow Features

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 08 | 08_url_detection | S | Clickable URLs and file paths, $EDITOR integration |
| 09 | 09_scrollback_search | M | Regex search in scrollback, search UI overlay, match highlighting |
| 10 | 10_shell_integration | M | OSC semantic prompts, CWD tracking, command timing, notifications |
| 11 | 11_vi_mode | M | Modal selection, vi motion commands, visual line/block modes |
| 14 | 14_quick_terminal | S | Global hotkey registration, window summon/dismiss |

### Wave 4 Completion Criteria
- [ ] URLs in terminal output are clickable and open in browser
- [ ] File paths open in $EDITOR on click
- [ ] Search overlay finds regex matches in scrollback with highlighting
- [ ] Shell integration detects prompts and tracks command timing
- [ ] Vi-mode enables keyboard-driven selection with motions
- [ ] Global hotkey summons/dismisses VeloTerm window

---

## Wave 5 — Advanced Features

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 12 | 12_session_persistence | M | Save/restore session layout, pane positions, scrollback |
| 13 | 13_command_palette | M | Fuzzy command search overlay, action dispatch |

### Wave 5 Completion Criteria
- [ ] Session layout saves on exit and restores on startup
- [ ] Command palette opens with hotkey, fuzzy-matches actions, dispatches

---

## Progress Summary

| Wave | Tracks | Total Complexity | Status |
|------|--------|-----------------|--------|
| 0 | 2 | 8 (XL+XL) | COMPLETE |
| 1 | 2 | 4 (M+M) | NOT_STARTED |
| 2 | 1 | 3 (L) | NOT_STARTED |
| 3 | 2 | 4 (M+M) | NOT_STARTED |
| 4 | 5 | 8 (S+M+M+M+S) | NOT_STARTED |
| 5 | 2 | 4 (M+M) | NOT_STARTED |

Overall: 8/31 complexity-weighted units complete (26%)
