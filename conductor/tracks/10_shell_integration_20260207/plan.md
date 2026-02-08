# Track 10: Shell Integration — Implementation Plan

## Phase 1: Shell State & OSC Interception [checkpoint: 4b6b8ca]

Core infrastructure: custom event listener, shell state struct, OSC 133/7/0 parsing.

### 1.1 Write tests for ShellState struct and OSC sequence parsing
- [x] Test ShellState creation with default values
- [x] Test OSC 133;A (prompt start) updates prompt positions
- [x] Test OSC 133;B (command start) records start timestamp
- [x] Test OSC 133;C (command output start) tracking
- [x] Test OSC 133;D (command end) records end time, duration, exit status
- [x] Test OSC 7 CWD parsing from `file://hostname/path` format
- [x] Test OSC 0/2 title capture
- [x] Test prompt position list is bounded (max 1000 entries)
- [x] Test graceful handling of malformed OSC sequences

### 1.2 Implement ShellState and OSC parsing <!-- b0e2ba1 -->
- [x] Create `src/shell_integration/mod.rs` module
- [x] Implement `ShellState` struct with CWD, prompt positions, command timing, title fields
- [x] Implement `OscParser` that extracts shell events from OSC sequence data
- [x] Implement bounded prompt position storage (VecDeque, max 1000)

### 1.3 Write tests for custom EventListener
- [x] Test custom listener captures title change events
- [x] Test listener stores events in a queue for later processing
- [x] Test listener integrates with alacritty_terminal Term

### 1.4 Implement custom EventListener and Terminal integration <!-- 2bd23ab -->
- [x] Create `VeloTermListener` implementing alacritty_terminal's `EventListener` trait
- [x] Replace `VoidListener` with `VeloTermListener` in Terminal struct
- [x] Wire event listener to drain events during `feed()` and update ShellState
- [x] Add ShellState to PaneState in window.rs

### 1.5 Write tests for config shell section
- [x] Test default shell config values (enabled=true, threshold=10)
- [x] Test TOML parsing of `[shell]` section
- [x] Test hot-reload of shell config values

### 1.6 Implement shell configuration <!-- 1714958 -->
- [x] Add `ShellConfig` to config types
- [x] Add TOML deserialization support
- [x] Wire into existing config hot-reload system

### Phase 1 Completion: Verification and Checkpointing

---

## Phase 2: Prompt Navigation & Command Timing [checkpoint: 4087fa9]

Build on shell state to enable prompt jumping and command duration tracking.

### 2.1 Write tests for prompt navigation <!-- 3164e36 -->
- [x] Test jump to previous prompt from middle of scrollback
- [x] Test jump to next prompt from middle of scrollback
- [x] Test jump to previous prompt when already at first prompt (no-op)
- [x] Test jump to next prompt when already at last prompt (no-op)
- [x] Test prompt navigation with no prompts recorded (no-op)
- [x] Test prompt navigation respects current viewport position

### 2.2 Implement prompt navigation <!-- 3164e36 -->
- [x] Add `previous_prompt()` and `next_prompt()` methods to ShellState
- [x] Integrate prompt navigation with Terminal's scroll position
- [x] Add keybinding actions for prompt navigation (Ctrl+Shift+P/N)
- [x] Wire keybindings through existing input handling in window.rs

### 2.3 Write tests for command timing <!-- 3164e36 -->
- [x] Test command duration calculation (end - start)
- [x] Test command history stores last N commands with timing
- [x] Test timing data accessible per-pane
- [x] Test multiple sequential commands each tracked independently

### 2.4 Implement command timing <!-- 3164e36 -->
- [x] Add `CommandRecord` struct (start, end, duration, exit_status)
- [x] Store command history in ShellState (bounded VecDeque)
- [x] Update timing on OSC 133;B (start) and OSC 133;D (end) events

### Phase 2 Completion: Verification and Checkpointing

---

## Phase 3: Notifications, Tab Titles & Shell Scripts [checkpoint: PENDING]

In-app notification badges, CWD-driven tab titles, and shell integration scripts.

### 3.1 Write tests for long-running command notification <!-- e731fec -->
- [x] Test notification triggered when command > threshold in non-focused pane
- [x] Test no notification for focused pane
- [x] Test no notification when duration < threshold
- [x] Test notification badge set on correct tab
- [x] Test badge clears when tab/pane receives focus
- [x] Test notification respects config enable/disable toggle

### 3.2 Implement long-running command notification <!-- e731fec -->
- [x] Add `has_notification` badge flag to Tab struct
- [x] On command completion in non-focused pane, check duration vs threshold
- [x] Set badge flag on owning tab
- [x] Clear badge on tab focus/switch
- [x] Render badge indicator in tab bar UI
- [x] Mark damage on tab bar when badge changes

### 3.3 Write tests for tab title from CWD <!-- e731fec -->
- [x] Test CWD change updates tab title to directory name
- [x] Test explicit OSC 0/2 title takes priority over CWD
- [x] Test CWD title only updates for active pane in tab
- [x] Test title reverts to CWD when OSC title is cleared

### 3.4 Implement tab title from CWD <!-- e731fec -->
- [x] On OSC 7 CWD change, extract last path component
- [x] Update tab title via TabManager if pane is active and no explicit title set
- [x] Track whether title was set explicitly (OSC 0/2) vs derived from CWD

### 3.5 Write tests for shell integration scripts <!-- 53d6fd5 -->
- [x] Test bash script emits correct OSC 133 sequences (parse expected output)
- [x] Test zsh script emits correct OSC 133 sequences
- [x] Test fish script emits correct OSC 133 sequences
- [x] Test all scripts emit OSC 7 with CWD
- [x] Test scripts don't interfere with existing PROMPT_COMMAND/precmd

### 3.6 Implement shell integration scripts <!-- 53d6fd5 -->
- [x] Create `shell/bash-integration.sh` with OSC 133 + OSC 7 emission
- [x] Create `shell/zsh-integration.sh` with precmd/preexec hooks
- [x] Create `shell/fish-integration.fish` with fish event handlers
- [x] Ensure scripts are non-destructive (append to existing hooks, don't replace)

### Phase 3 Completion: Verification and Checkpointing
