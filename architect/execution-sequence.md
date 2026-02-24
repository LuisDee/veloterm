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

## Wave 1 — Configuration & Optimization (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 03 | 03_config | M | TOML config system with hot-reload, theme switching, keybinding definitions |
| 07 | 07_perf_damage | M | Dirty cell tracking, selective buffer updates, frame budget optimization |

### Wave 1 Status: COMPLETE

---

## Wave 2 — Pane Layout Engine (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 04 | 04_pane_layout | L | Binary tree pane layout, split/close operations, focus management, layout calculation |

### Wave 2 Status: COMPLETE

---

## Wave 3 — Pane UI & Tabs (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 05 | 05_pane_ui | M | Divider bar rendering, drag-to-resize, mouse click focus, pane zoom |
| 06 | 06_tabs | M | Tab bar rendering, tab management, independent pane tree per tab |

### Wave 3 Status: COMPLETE

---

## Wave 4 — Developer Workflow Features (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 08 | 08_url_detection | S | Clickable URLs and file paths, $EDITOR integration |
| 09 | 09_scrollback_search | M | Regex search in scrollback, search UI overlay, match highlighting |
| 10 | 10_shell_integration | M | OSC semantic prompts, CWD tracking, command timing, notifications |
| 11 | 11_vi_mode | M | Modal selection, vi motion commands, visual line/block modes |
| 14 | 14_quick_terminal | S | Global hotkey registration, window summon/dismiss |

### Wave 4 Status: COMPLETE

---

## Wave 5 — Advanced Features (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 12 | 12_session_persistence | M | Save/restore session layout, pane positions, scrollback |
| 13 | 13_command_palette | M | Fuzzy command search overlay, action dispatch |

### Wave 5 Status: COMPLETE

---

## Wave 6 — Core Terminal Quality (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 15 | 15_font_padding | M | Font rendering refinement, terminal padding |
| 16 | 16_cursor_input | S | Cursor shapes, input polish |
| 17 | 17_selection_clipboard | M | Text selection & clipboard |
| 18 | 18_scrollback_scrollbar | M | Scrollback & scrollbar |

### Wave 6 Status: COMPLETE

---

## Wave 7 — Tab & Window Management (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 19 | 19_tab_management | M | Tab management polish |
| 20 | 20_context_menus | M | Context menus |

### Wave 7 Status: COMPLETE

---

## Wave 8 — Visual Polish & Theming (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 21 | 21_theme_colors | M | Anthropic Dark theme, color rendering |

### Wave 8 Status: COMPLETE

---

## Wave 9 — Shell Integration Hardening (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 22 | 22_shell_hardening | M | Shell integration & usability hardening |

### Wave 9 Status: COMPLETE

---

## Wave 10 — iced Renderer Migration (Foundation) (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 23 | 23_iced_foundation | L | iced_wgpu Engine/Renderer setup, winit event wiring, compositor proof-of-concept |
| 25 | 25_glyphon_text | M | Replace hand-rolled glyph atlas with glyphon 0.8 (cosmic-text backend) for HiDPI text |

### Wave 10 Status: COMPLETE

---

## Wave 11 — iced Renderer Migration (UI Chrome) (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 24 | 24_iced_ui_chrome | L | Replace overlay.wgsl pipeline with iced widgets for all UI chrome |

### Wave 11 Status: COMPLETE

---

## Wave 12+ — Deferred Features (COMPLETE)

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 14 | 14_quick_terminal | S | Global hotkey registration, window summon/dismiss |
| 12 | 12_session_persistence | M | Save/restore session layout |
| 13 | 13_command_palette | M | Fuzzy command search overlay |

### Wave 12+ Status: COMPLETE

---

## Wave 13 — UI Chrome Redesign

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 26 | 26_ui_chrome_redesign | L | UI Chrome redesign per Claude Terminal design spec |

### Wave 13 Status: IN_PROGRESS

---

## Wave 14 — Platform Portability

| # | Track ID | Complexity | Description |
|---|----------|------------|-------------|
| 27 | 27_linux_centos9 | L | Linux CentOS 9 port: platform module, process detection, context menus, CI |

### Wave 14 Completion Criteria
- [ ] `cargo check --target x86_64-unknown-linux-gnu` passes
- [ ] `cargo build --target x86_64-unknown-linux-gnu` produces working binary
- [ ] Linux platform module with real foreground process detection
- [ ] PTY environment includes COLORTERM and TERM_PROGRAM
- [ ] Context menus work via iced on Linux
- [ ] GitHub Actions CI green on CentOS 9 container
- [ ] All macOS tests pass without regression

---

## Progress Summary

| Wave | Tracks | Total Complexity | Status |
|------|--------|-----------------|--------|
| 0 | 2 | 8 (XL+XL) | COMPLETE |
| 1 | 2 | 4 (M+M) | COMPLETE |
| 2 | 1 | 3 (L) | COMPLETE |
| 3 | 2 | 4 (M+M) | COMPLETE |
| 4 | 5 | 7 (S+M+M+M+S) | COMPLETE |
| 5 | 2 | 4 (M+M) | COMPLETE |
| 6 | 4 | 7 (M+S+M+M) | COMPLETE |
| 7 | 2 | 4 (M+M) | COMPLETE |
| 8 | 1 | 2 (M) | COMPLETE |
| 9 | 1 | 2 (M) | COMPLETE |
| 10 | 2 | 5 (L+M) | COMPLETE |
| 11 | 1 | 3 (L) | COMPLETE |
| 12+ | 3 | 5 (S+M+M) | COMPLETE |
| 13 | 1 | 3 (L) | IN_PROGRESS |
| 14 | 1 | 3 (L) | NOT_STARTED |

Overall: 25/27 tracks complete (93%), 58/64 complexity-weighted units (91%)
