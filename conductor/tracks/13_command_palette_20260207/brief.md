<!-- ARCHITECT CONTEXT | Track: 13_command_palette | Wave: 5 | CC: v1 -->

## Cross-Cutting Constraints
- Testing: TDD, fuzzy match algorithm tests
- Performance Budget: palette must open instantly (<50ms)

## Interfaces

### Owns
- Command registry (all available actions)
- Fuzzy search/filter engine
- Command palette UI overlay

### Consumes
- `Config` (Track 03) — palette keybinding, action list
- `TabManager` (Track 06) — tab-related actions

## Dependencies
- Track 03_config: keybindings and action registry
- Track 06_tabs: tab management actions

<!-- END ARCHITECT CONTEXT -->

# Track 13: Command Palette

## What This Track Delivers

A command palette overlay (similar to VS Code's Cmd+Shift+P) that provides fuzzy-search access to all VeloTerm actions — splitting panes, switching tabs, changing themes, opening settings, and any other registered command. The palette opens with a keyboard shortcut, shows a searchable list of actions, and dispatches the selected action.

## Scope

### IN
- Command palette overlay UI (text input + filtered action list)
- Fuzzy search/filter algorithm for action matching
- Action registry: all VeloTerm commands registered with name, description, keybinding hint
- Dispatch: executing the selected action
- Recently-used actions sorting (most used appear first)
- Keybinding hint display next to each action

### OUT
- File picker or project navigation (not a file manager)
- Custom user commands or scripting
- Search within terminal content (Track 09 — scrollback_search)

## Key Design Decisions

1. **Fuzzy match algorithm**: Simple substring vs Sublime-style fuzzy (score character positions) vs external crate?
   Trade-off: substring is simplest; Sublime-style feels better for partial matches; crate avoids reinventing

2. **UI rendering**: egui overlay vs custom GPU overlay vs dedicated popup window?
   Trade-off: egui handles text input; custom matches aesthetic; popup is OS-native but heavy

3. **Action registry**: Static compile-time list vs dynamic registration at startup vs plugin-extensible?
   Trade-off: static is simplest and fastest; dynamic allows runtime changes; plugins are over-engineering for now

4. **Category grouping**: Flat list vs categorized (Pane, Tab, Config, etc.) vs both with toggle?
   Trade-off: flat is simpler and faster to search; categories help discovery; both adds UI complexity

## Architectural Notes

- The command palette captures keyboard input modally — same pattern as search overlay (Track 09) and vi-mode (Track 11)
- Consider sharing the modal input infrastructure across these three features
- The action registry should be extensible: new tracks register their actions (e.g., Track 10 registers "Toggle Shell Integration")
- If egui is used for other overlays, use it here too for consistency

## Complexity: M
## Estimated Phases: ~3
