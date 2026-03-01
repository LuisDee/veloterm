<!-- ARCHITECT CONTEXT | Track: 22_shell_hardening | Wave: 9 | CC: v2 -->

## Cross-Cutting Constraints
- Testing: TDD for signal handling, clear command, bell notification
- Platform Abstraction: shell config paths differ per platform; signal handling differs
- Graceful Shutdown: ensure shell integration doesn't interfere with clean shutdown

## Interfaces

### Owns
- Shell config respect verification (.zshrc, .bashrc, env vars, PATH, aliases)
- Clear command and Cmd+K clear scrollback
- Bell/notification support
- Signal handling verification (SIGINT, SIGTSTP, EOF)
- Tab completion pass-through verification

### Consumes
- `ShellIntegration` (Track 10) — existing shell integration infrastructure
- `URLDetector` (Track 08) — clickable URL verification
- `Terminal` (Track 02) — terminal state for clear operations
- `Config` (Track 03) — bell settings, clear keybinding

## Dependencies
- Track 10_shell_integration: shell integration infrastructure
- Track 08_url_detection: URL click handling

<!-- END ARCHITECT CONTEXT -->

# Track 22: Shell Integration & Usability Hardening

## What This Track Delivers

Hardens the shell integration to production quality by verifying that the terminal correctly respects user shell configuration (.zshrc, .bashrc, environment variables, PATH, aliases), implements clear scrollback (Cmd+K), adds bell/notification support, and verifies correct signal handling (Ctrl+C, Ctrl+Z, Ctrl+D). Ensures the terminal works as a transparent pass-through for all shell features including tab completion.

## Scope

### IN
- Verify shell spawns with user's default shell ($SHELL) and loads rc files (.zshrc, .bashrc)
- Verify environment variables, PATH, and aliases are available in the terminal
- Verify TERM is set correctly (xterm-256color) for full color support
- `clear` command works (clears visible area, not scrollback)
- Cmd+K clears scrollback buffer entirely
- Bell support: terminal bell character (BEL, 0x07) triggers visual flash or system notification
- Configurable bell behavior: visual bell vs system sound vs notification vs disabled
- Signal pass-through: Ctrl+C sends SIGINT, Ctrl+Z sends SIGTSTP, Ctrl+D sends EOF
- Tab completion: shell tab completion works transparently
- Verify clickable URLs (Cmd+click opens in browser) — existing Track 08 functionality
- Verify Ctrl+L clears screen (shell built-in pass-through)

### OUT
- Custom shell integration scripts (beyond what Track 10 provides)
- Remote shell support (SSH)
- Shell-specific features (zsh plugins, bash completion scripts)
- Terminal multiplexer compatibility (tmux, screen)

## Key Design Decisions

1. **Bell behavior default**: Visual bell (brief screen flash) vs system notification vs system sound vs disabled?
   Trade-off: visual bell is least intrusive; system notification is most useful for background alerts; sound is traditional but annoying; disabled is safest default

2. **Cmd+K clear behavior**: Clear scrollback only vs clear scrollback + send clear to shell vs clear everything and reset terminal?
   Trade-off: scrollback-only preserves current prompt; clear+send matches iTerm2; full reset is nuclear

3. **Shell spawn strategy**: Use $SHELL env var vs read /etc/passwd vs hardcode /bin/zsh on macOS + /bin/bash on Linux?
   Trade-off: $SHELL is most correct; /etc/passwd is reliable fallback; hardcoding is simplest but wrong for users with different shells

## Architectural Notes

- Shell integration (Track 10) already provides OSC 7 CWD tracking and semantic prompt detection — this track verifies it works and adds missing features
- The PTY is currently spawned via `portable-pty` which handles fork/exec — verify it sets up the session leader, controlling terminal, and inherits the environment correctly
- Cmd+K clear should call `alacritty_terminal`'s reset or clear method AND clear the scrollback buffer
- Bell support requires intercepting the BEL byte (0x07) during VTE parsing — check if alacritty_terminal exposes a bell callback
- Signal handling (Ctrl+C → SIGINT) should be handled by the PTY layer, not the terminal emulator — verify the PTY correctly forwards these
- URL detection and Cmd+click is already implemented in Track 08 — this track only verifies it works in the current state

## Complexity: M
## Estimated Phases: ~3
