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

# Shell Integration Specification

## Overview

Shell integration enables VeloTerm to understand shell semantics — where prompts are, the current working directory, when commands start and finish, and how long they took. This powers semantic prompt navigation, accurate CWD display in tab titles, command timing, and in-app notifications when long-running background commands complete.

Integration is opt-in: VeloTerm ships shell integration scripts that users source from their shell rc files. Without setup, the terminal gracefully degrades to basic behavior.

## Functional Requirements

### FR-1: OSC Sequence Interception (Performer Integration)

Hook into alacritty_terminal's event system to intercept OSC sequences at parse time.

- **FR-1.1:** Replace `VoidListener` with a custom `EventListener` implementation that captures terminal events including title changes and color requests.
- **FR-1.2:** Create a `ShellState` struct per pane that tracks: current working directory, prompt regions, command start/end timestamps, last exit status.
- **FR-1.3:** Intercept OSC 133 (semantic prompt) markers:
  - `\e]133;A\a` — Prompt start
  - `\e]133;B\a` — Command start (user pressed Enter)
  - `\e]133;C\a` — Command output start
  - `\e]133;D;{exit_status}\a` — Command finished with exit status
- **FR-1.4:** Intercept OSC 7 (CWD reporting):
  - Parse `\e]7;file://hostname/path\a` to extract the current working directory
  - Store in `ShellState.cwd`
- **FR-1.5:** Intercept OSC 0/2 (window/tab title):
  - Capture title set by the shell and update the pane's title field
  - Feed into tab title display

### FR-2: Shell State Tracking

- **FR-2.1:** Maintain a `ShellState` per pane in `PaneState`, alongside the existing `Terminal` and `PtySession`.
- **FR-2.2:** On OSC 133;B (command start), record `Instant::now()` as command start time.
- **FR-2.3:** On OSC 133;D (command end), record end time, compute duration, store exit status.
- **FR-2.4:** On OSC 7, update `ShellState.cwd` and trigger tab title update if the pane is the active pane in the active tab.
- **FR-2.5:** Maintain a list of prompt line positions in scrollback to support prompt navigation.

### FR-3: Prompt Navigation

- **FR-3.1:** Implement "jump to previous prompt" — scroll to the nearest OSC 133;A marker above the current viewport.
- **FR-3.2:** Implement "jump to next prompt" — scroll to the nearest OSC 133;A marker below the current viewport.
- **FR-3.3:** Add configurable keybindings for prompt navigation (default: `Ctrl+Shift+Up` / `Ctrl+Shift+Down`).

### FR-4: Command Timing Display

- **FR-4.1:** After a command completes (OSC 133;D), store the duration in `ShellState`.
- **FR-4.2:** Expose command timing data so it can be queried (e.g., for future status bar display or tooltip). The actual UI rendering of timing is deferred — this track only tracks and stores the data.

### FR-5: Long-Running Command Notification (In-App)

- **FR-5.1:** When a command completes in a **non-focused** pane and its duration exceeds a configurable threshold (default: 10 seconds), trigger an in-app notification.
- **FR-5.2:** In-app notification: display a visual badge/indicator on the tab containing the pane (e.g., a dot or highlight on the tab label).
- **FR-5.3:** The badge clears when the user switches to that tab/pane.
- **FR-5.4:** Native OS notifications are out of scope — deferred to a future track.

### FR-6: Tab Title from CWD

- **FR-6.1:** When a pane's CWD changes via OSC 7, update the owning tab's title to the last path component (directory name) if no explicit OSC 0/2 title has been set.
- **FR-6.2:** Explicit OSC 0/2 title takes priority over CWD-derived title.

### FR-7: Shell Integration Scripts

- **FR-7.1:** Ship shell integration scripts for bash, zsh, and fish at a well-known location (e.g., bundled in the binary or installed to `~/.config/veloterm/shell/`).
- **FR-7.2:** Each script emits:
  - OSC 133 markers around prompt and command execution
  - OSC 7 with the current working directory after each command
- **FR-7.3:** Provide a one-liner for each shell that users add to their rc file:
  - bash: `source ~/.config/veloterm/shell/bash-integration.sh`
  - zsh: `source ~/.config/veloterm/shell/zsh-integration.sh`
  - fish: `source ~/.config/veloterm/shell/fish-integration.fish`

### FR-8: Configuration

- **FR-8.1:** Add `[shell]` section to config with:
  - `integration_enabled: bool` (default: true) — master toggle
  - `notification_threshold_secs: u64` (default: 10) — minimum command duration for notification
- **FR-8.2:** Hot-reload support: config changes take effect without restart.

## Non-Functional Requirements

- **NFR-1:** OSC interception must add zero measurable latency to terminal throughput. The performer integration should be a lightweight check on each event.
- **NFR-2:** Shell state tracking must not leak memory — prompt positions should be bounded (e.g., keep last 1000 prompt positions).
- **NFR-3:** Graceful degradation: if no shell integration scripts are sourced, all features silently degrade. No errors, no warnings to the user.
- **NFR-4:** Shell scripts must be POSIX-compatible where possible and must not interfere with existing user prompt customizations.

## Acceptance Criteria

1. OSC 133 sequences are correctly parsed and prompt regions tracked
2. OSC 7 sequences update CWD in ShellState and tab title
3. Prompt navigation (up/down) works in scrollback
4. Command timing is recorded for each command
5. Long-running command notification badge appears on non-focused tabs
6. Shell integration scripts emit correct sequences for bash, zsh, and fish
7. Config controls notification threshold and enable/disable
8. All features degrade gracefully without shell setup
9. >80% test coverage on new modules

## Out of Scope

- Native OS notifications (macOS notification center) — deferred
- Shell-specific command completion or suggestion
- Remote shell integration (SSH sessions)
- Command output capture or semantic parsing
- Auto-injection into shell rc files
- CWD tracking fallback (/proc polling, regex parsing)
- Status bar UI for command timing display
