# Initial Concept

VeloTerm — A cross-platform (macOS + Linux), GPU-accelerated terminal emulator with native split panes, built in Rust. The goal is to be fast enough to match Alacritty's performance class while being more featureful and more pleasant to use, with built-in multiplexing (split panes + tabs) that makes tmux unnecessary.

# Product Guide — VeloTerm

## Vision

VeloTerm is a super lightweight, blazing-fast terminal emulator that combines the raw speed of Alacritty with the productivity features developers actually need — without the bloat. It eliminates the need for external multiplexers like tmux by providing native, GPU-rendered split panes and tabs with an intuitive, polished GUI.

## Target Users

VeloTerm is built for **software developers** — engineers who spend significant time in the terminal for coding, building, debugging, and version control. These users demand low-latency input response, reliable rendering of complex TUI applications (vim, htop, etc.), and efficient multi-session workflows without context switching.

## Core Goals

1. **Replace tmux/screen entirely** — Provide built-in split panes and tabs so developers no longer need external terminal multiplexers. Pane management should be native, fast, and intuitive with clickable dividers, drag-to-resize, and keyboard shortcuts.

2. **Blazing performance** — Match or beat Alacritty's input-to-screen latency (<10ms target) with GPU-accelerated rendering, glyph atlas caching, and damage-tracked rendering. Startup time must be under 100ms. Memory footprint must remain minimal (<10MB per terminal instance).

## Key Differentiators

- **Native split panes with a polished GUI** — Clickable dividers, drag-to-resize, and intuitive mouse-driven pane management that feels native rather than a bolted-on overlay. Pane zoom, focus indicators, and keyboard navigation included.

- **Developer-centric workflow integration** — Shell integration with semantic prompt detection, clickable file paths that open in `$EDITOR`, command timing, smart notifications when long-running commands complete in background panes, and scrollback search with regex support.

- **Performance + usability balance** — Fast enough to compete with Alacritty on benchmarks, featureful enough to replace WezTerm for daily use, but simpler and more focused than either. Super lightweight with minimal resource consumption.

## Target Platforms

First-class, equal support for both platforms from day one:

- **macOS** — Apple Silicon only (aarch64-apple-darwin). GPU rendering via Metal. Full Retina/HiDPI support. Native `.app` bundle distribution.

- **Linux** — X11 on CentOS 9 only. GPU rendering via Vulkan (primary) with OpenGL fallback. RPM and/or AppImage packaging.

## Configuration

VeloTerm uses a **TOML configuration file** located at `~/.config/veloterm/veloterm.toml`. The configuration supports **hot-reload** — changes to the file are detected and applied automatically without restarting the terminal. The config covers font settings, keybindings, scrollback size, cursor style, performance tuning, and theming.

## Visual Identity

VeloTerm ships with three built-in Claude-themed color schemes (Light, Dark, Warm) designed to WCAG AAA contrast standards. The visual design emphasizes warm, approachable aesthetics with generous spacing, soft edges (8px border radius), and a peachy-orange accent color. Full specifications are in `conductor/ui-guide.md`.

## Feature Priorities

### Phase 1 — Core Terminal (MVP)
- Window creation and GPU-accelerated rendering (Metal on macOS, Vulkan/OpenGL on Linux)
- Terminal emulation with full VT100/VT220/xterm escape sequence support
- PTY management with dedicated IO threads for minimal latency
- Glyph atlas rendering with damage tracking
- True Color (24-bit) support
- Basic scrollback with configurable history
- Text selection and clipboard integration
- Cursor rendering (block, beam, underline)
- TOML configuration with hot-reload
- DPI/HiDPI awareness

### Phase 2 — Split Panes & Tabs
- Binary tree pane layout engine
- Vertical and horizontal splits with keyboard shortcuts
- Clickable divider bars with drag-to-resize
- Focus switching (click or keyboard)
- Pane zoom (temporarily maximize a pane)
- Tab support with tab bar rendering
- Independent terminal instance per pane/tab

### Phase 3 — Developer Workflow Features
- Clickable URLs and file paths
- Scrollback search with regex support
- Vi-mode for keyboard-driven text selection
- Shell integration (semantic prompts, CWD tracking, command timing)
- Notification on long-running command completion
- Quick terminal (global hotkey summon/dismiss)

### Phase 4 — Advanced Features
- Kitty Graphics Protocol (inline images)
- Kitty Keyboard Protocol
- Session persistence
- Command palette
- Multiple color schemes
- Background opacity/blur
- Broadcast input across panes
- Custom keybindings
- Ligature support
