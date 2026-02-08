# Track 10: Shell Integration — Implementation Plan

## Phase 1: Shell State & OSC Interception

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
- [ ] Test default shell config values (enabled=true, threshold=10)
- [ ] Test TOML parsing of `[shell]` section
- [ ] Test hot-reload of shell config values

### 1.6 Implement shell configuration
- [ ] Add `ShellConfig` to config types
- [ ] Add TOML deserialization support
- [ ] Wire into existing config hot-reload system

### Phase 1 Completion: Verification and Checkpointing

---

## Phase 2: Prompt Navigation & Command Timing

Build on shell state to enable prompt jumping and command duration tracking.

### 2.1 Write tests for prompt navigation
- [ ] Test jump to previous prompt from middle of scrollback
- [ ] Test jump to next prompt from middle of scrollback
- [ ] Test jump to previous prompt when already at first prompt (no-op)
- [ ] Test jump to next prompt when already at last prompt (no-op)
- [ ] Test prompt navigation with no prompts recorded (no-op)
- [ ] Test prompt navigation respects current viewport position

### 2.2 Implement prompt navigation
- [ ] Add `previous_prompt()` and `next_prompt()` methods to ShellState
- [ ] Integrate prompt navigation with Terminal's scroll position
- [ ] Add keybinding actions for prompt navigation (Ctrl+Shift+Up/Down)
- [ ] Wire keybindings through existing input handling in window.rs

### 2.3 Write tests for command timing
- [ ] Test command duration calculation (end - start)
- [ ] Test command history stores last N commands with timing
- [ ] Test timing data accessible per-pane
- [ ] Test multiple sequential commands each tracked independently

### 2.4 Implement command timing
- [ ] Add `CommandRecord` struct (start, end, duration, exit_status)
- [ ] Store command history in ShellState (bounded VecDeque)
- [ ] Update timing on OSC 133;B (start) and OSC 133;D (end) events

### Phase 2 Completion: Verification and Checkpointing

---

## Phase 3: Notifications, Tab Titles & Shell Scripts

In-app notification badges, CWD-driven tab titles, and shell integration scripts.

### 3.1 Write tests for long-running command notification
- [ ] Test notification triggered when command > threshold in non-focused pane
- [ ] Test no notification for focused pane
- [ ] Test no notification when duration < threshold
- [ ] Test notification badge set on correct tab
- [ ] Test badge clears when tab/pane receives focus
- [ ] Test notification respects config enable/disable toggle

### 3.2 Implement long-running command notification
- [ ] Add `has_notification` badge flag to Tab struct
- [ ] On command completion in non-focused pane, check duration vs threshold
- [ ] Set badge flag on owning tab
- [ ] Clear badge on tab focus/switch
- [ ] Render badge indicator in tab bar UI
- [ ] Mark damage on tab bar when badge changes

### 3.3 Write tests for tab title from CWD
- [ ] Test CWD change updates tab title to directory name
- [ ] Test explicit OSC 0/2 title takes priority over CWD
- [ ] Test CWD title only updates for active pane in tab
- [ ] Test title reverts to CWD when OSC title is cleared

### 3.4 Implement tab title from CWD
- [ ] On OSC 7 CWD change, extract last path component
- [ ] Update tab title via TabManager if pane is active and no explicit title set
- [ ] Track whether title was set explicitly (OSC 0/2) vs derived from CWD

### 3.5 Write tests for shell integration scripts
- [ ] Test bash script emits correct OSC 133 sequences (parse expected output)
- [ ] Test zsh script emits correct OSC 133 sequences
- [ ] Test fish script emits correct OSC 133 sequences
- [ ] Test all scripts emit OSC 7 with CWD
- [ ] Test scripts don't interfere with existing PROMPT_COMMAND/precmd

### 3.6 Implement shell integration scripts
- [ ] Create `shell/bash-integration.sh` with OSC 133 + OSC 7 emission
- [ ] Create `shell/zsh-integration.sh` with precmd/preexec hooks
- [ ] Create `shell/fish-integration.fish` with fish event handlers
- [ ] Ensure scripts are non-destructive (append to existing hooks, don't replace)

### Phase 3 Completion: Verification and Checkpointing
