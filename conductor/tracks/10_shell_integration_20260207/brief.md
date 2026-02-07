<!-- ARCHITECT CONTEXT | Track: 10_shell_integration | Wave: 4 | CC: v1 -->

## Cross-Cutting Constraints
- Testing: TDD, OSC sequence parsing tests
- Error Handling: gracefully degrade if shell doesn't support integration

## Interfaces

### Owns
- OSC semantic prompt detection
- CWD tracking from shell reports
- Command timing (start/end timestamps)
- Long-running command notification

### Consumes
- `Config` (Track 03) — notification settings, shell integration enable/disable
- Terminal ANSI parser output (existing alacritty_terminal)

## Dependencies
- Track 03_config: shell integration settings

<!-- END ARCHITECT CONTEXT -->

# Track 10: Shell Integration

## What This Track Delivers

Shell integration that enables the terminal to understand shell semantics — where prompts are, what the current working directory is, when commands start and finish, and how long they took. This powers features like: semantic prompt navigation (jump between prompts), accurate CWD display in tab titles, command timing display, and notifications when long-running background commands complete.

## Scope

### IN
- OSC 133 semantic prompt markers (prompt start, command start, command end, exit status)
- OSC 7 CWD reporting (shell reports its CWD to the terminal)
- Command timing: record start/end timestamps for each command
- Long-running command notification: alert when a command in a non-focused pane finishes (configurable threshold, e.g., >10s)
- Prompt navigation: jump to previous/next prompt in scrollback
- Shell setup scripts for bash, zsh, fish (to emit the OSC sequences)

### OUT
- Shell-specific command completion or suggestion
- Remote shell integration (SSH sessions)
- Command output capture or semantic parsing

## Key Design Decisions

1. **OSC parsing location**: Intercept OSC sequences in the alacritty_terminal performer vs post-process terminal output vs custom pre-parser?
   Trade-off: performer integration is cleanest but requires alacritty_terminal modification; post-processing is decoupled but may miss sequences; pre-parser adds latency

2. **Notification mechanism**: macOS native notifications (NSUserNotification) vs in-app visual notification vs sound only?
   Trade-off: native notifications work when app is in background; in-app is cross-platform; sound is subtle

3. **Shell setup distribution**: Auto-inject shell integration into shell rc files vs provide manual setup instructions vs detect and configure on first run?
   Trade-off: auto-inject is seamless but modifies user files; manual is safest; auto-detect is complex

4. **CWD tracking fallback**: Trust OSC 7 reports only vs poll /proc/PID/cwd (Linux) vs parse prompt regex?
   Trade-off: OSC 7 is reliable when shells support it; /proc polling works without shell changes; regex is fragile

## Architectural Notes

- alacritty_terminal's `Perform` trait handles OSC sequences — check if it exposes hooks for custom OSC handling
- Shell integration is opt-in: without shell setup, everything degrades gracefully to basic terminal behavior
- CWD tracking feeds into Tab titles (Track 06) and URL path detection (Track 08) — design the CWD notification interface for reuse
- Consider the "Event Bus" deferred pattern here: if CWD changes, command completion, and prompt detection all need to notify multiple consumers, this might trigger the pattern adoption

## Complexity: M
## Estimated Phases: ~3
