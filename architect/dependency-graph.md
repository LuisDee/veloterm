# Dependency Graph

> Directed Acyclic Graph (DAG) of track dependencies.
> An edge A -> B means "A depends on B" (B must complete before A starts).

---

## Track Dependencies

| Track | Depends On | Interfaces Consumed |
|-------|-----------|---------------------|
| 01_window_gpu (COMPLETE) | -- | -- |
| 02_core_terminal (COMPLETE) | 01_window_gpu | Renderer, Window |
| 03_config (COMPLETE) | -- | -- |
| 04_pane_layout (COMPLETE) | 03_config | Config (keybindings, defaults) |
| 05_pane_ui (COMPLETE) | 04_pane_layout, 03_config | PaneTree, Config (theme) |
| 06_tabs (COMPLETE) | 04_pane_layout, 03_config | PaneTree, Config (keybindings) |
| 07_perf_damage (COMPLETE) | -- | -- |
| 08_url_detection (COMPLETE) | 03_config | Config (URL styling, click action) |
| 09_scrollback_search (COMPLETE) | 03_config | Config (keybindings, search UI theme) |
| 10_shell_integration (COMPLETE) | 03_config | Config (shell integration settings) |
| 11_vi_mode (COMPLETE) | 03_config | Config (vi keybindings) |
| 12_session_persistence (COMPLETE) | 04_pane_layout, 06_tabs | PaneTree, TabManager |
| 13_command_palette (COMPLETE) | 03_config, 06_tabs | Config, TabManager |
| 14_quick_terminal (COMPLETE) | 03_config | Config (global hotkey) |
| 15_font_padding (COMPLETE) | -- | -- |
| 16_cursor_input (COMPLETE) | -- | -- |
| 17_selection_clipboard (COMPLETE) | -- | -- |
| 18_scrollback_scrollbar (COMPLETE) | -- | -- |
| 19_tab_management (COMPLETE) | -- | -- |
| 20_context_menus (COMPLETE) | -- | -- |
| 21_theme_colors (COMPLETE) | -- | -- |
| 22_shell_hardening (COMPLETE) | -- | -- |
| 23_iced_foundation (COMPLETE) | 01_window_gpu | wgpu Device/Queue/Surface, Window, winit events |
| 24_iced_ui_chrome (COMPLETE) | 23_iced_foundation | iced Engine/Renderer/UserInterface |
| 25_glyphon_text (COMPLETE) | -- | -- (independent, replaces glyph atlas) |
| 26_ui_chrome_redesign | 24_iced_ui_chrome | iced UI layer, Config |
| 27_linux_centos9 | -- | Config, PtySession, GlyphAtlas, HotkeyManager, ContextMenuAction, arboard (all complete) |

---

## DAG Visualization

```
COMPLETE:    [01_window_gpu]    [02_core_terminal]
                   |                   |
Wave 1:      [03_config]           [07_perf_damage]
                  |
Wave 2:      [04_pane_layout]
               |        |
Wave 3:   [05_pane_ui] [06_tabs]
               |           |
Wave 4:   [08_url]  [09_search]  [10_shell]  [11_vi_mode]  [14_quick_term]
               |           |
Wave 5:        [12_session]    [13_cmd_palette]

--- iced Renderer Migration ---

Wave 10:  [23_iced_foundation]    [25_glyphon_text]
                   |
Wave 11:  [24_iced_ui_chrome]

--- UI Redesign ---

Wave 13:  [26_ui_chrome_redesign]

--- Platform Portability ---

Wave 14:  [27_linux_centos9]  (no dependencies -- all prior tracks complete)
```
