# Dependency Graph

> Directed Acyclic Graph (DAG) of track dependencies.
> An edge A → B means "A depends on B" (B must complete before A starts).

---

## Track Dependencies

| Track | Depends On | Interfaces Consumed |
|-------|-----------|---------------------|
| 01_window_gpu (COMPLETE) | — | — |
| 02_core_terminal (COMPLETE) | 01_window_gpu | Renderer, Window |
| 03_config | — | — |
| 04_pane_layout | 03_config | Config (keybindings, defaults) |
| 05_pane_ui | 04_pane_layout, 03_config | PaneTree, Config (theme) |
| 06_tabs | 04_pane_layout, 03_config | PaneTree, Config (keybindings) |
| 07_perf_damage | — | — |
| 08_url_detection | 03_config | Config (URL styling, click action) |
| 09_scrollback_search | 03_config | Config (keybindings, search UI theme) |
| 10_shell_integration | 03_config | Config (shell integration settings) |
| 11_vi_mode | 03_config | Config (vi keybindings) |
| 12_session_persistence | 04_pane_layout, 06_tabs | PaneTree, TabManager |
| 13_command_palette | 03_config, 06_tabs | Config, TabManager |
| 14_quick_terminal | 03_config | Config (global hotkey) |
| 23_iced_foundation | 01_window_gpu | wgpu Device/Queue/Surface, Window, winit events |
| 24_iced_ui_chrome | 23_iced_foundation | iced Engine/Renderer/UserInterface |
| 25_glyphon_text | — | — (independent, replaces glyph atlas) |

---

## DAG Visualization

```
COMPLETE:    [01_window_gpu]    [02_core_terminal]
                   │                   │
Wave 1:      [03_config]           [07_perf_damage]
                  │
Wave 2:      [04_pane_layout]
               │        │
Wave 3:   [05_pane_ui] [06_tabs]
               │           │
Wave 4:   [08_url]  [09_search]  [10_shell]  [11_vi_mode]  [14_quick_term]
               │           │
Wave 5:        [12_session]    [13_cmd_palette]

--- iced Renderer Migration ---

Wave 10:  [23_iced_foundation]    [25_glyphon_text]
                   │
Wave 11:  [24_iced_ui_chrome]
```
