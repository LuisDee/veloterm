<!-- ARCHITECT CONTEXT | Track: 14_quick_terminal | Wave: 4 | CC: v1 -->

## Cross-Cutting Constraints
- Platform Abstraction: global hotkey registration differs per platform (macOS: Carbon/Cocoa APIs, Linux: X11 keybindings)
- Testing: TDD for toggle logic, platform-specific integration testing

## Interfaces

### Owns
- Global hotkey registration
- Window summon/dismiss toggle
- Drop-down animation (optional)

### Consumes
- `Config` (Track 03) — global hotkey keybinding, animation settings

## Dependencies
- Track 03_config: global hotkey configuration

<!-- END ARCHITECT CONTEXT -->

# Track 14: Quick Terminal

## What This Track Delivers

A global hotkey that summons or dismisses the VeloTerm window from anywhere in the OS, similar to iTerm2's "hotkey window" or Guake. Pressing the configured hotkey (e.g., Ctrl+`) toggles the terminal window's visibility, optionally with a slide-down animation.

## Scope

### IN
- Global hotkey registration (works even when VeloTerm is not focused)
- Window show/hide toggle on hotkey press
- Optional slide-down animation from top of screen
- Window positioning: configurable (center, top-drop-down, bottom)
- Remember window state (size, position) across show/hide cycles

### OUT
- Multiple quick terminal profiles
- Per-monitor quick terminal
- Quake-style always-on-top mode (beyond basic toggle)

## Key Design Decisions

1. **Global hotkey mechanism**: Platform-native APIs (Carbon on macOS, X11 XGrabKey on Linux) vs background daemon listening?
   Trade-off: native APIs are reliable but platform-specific; daemon adds process management complexity

2. **Window behavior on summon**: Raise existing window vs always create centered vs restore last position?
   Trade-off: raise is simplest; centered is consistent; last position remembers user preference

3. **Animation**: Slide-down from top vs fade-in vs instant show vs configurable?
   Trade-off: slide-down is Guake-style and polished; instant is fastest; configurable adds complexity

4. **Focus behavior**: Steal focus on summon vs raise without focus vs configurable?
   Trade-off: steal focus is expected; raise-only prevents input interruption

## Architectural Notes

- macOS global hotkeys use the Carbon `RegisterEventHotKey` API or newer Cocoa APIs — this is inherently platform-specific
- Linux X11 global hotkeys use `XGrabKey` — works but only on X11 (Wayland needs a different approach)
- winit does not provide global hotkey APIs — this requires platform-specific code behind `#[cfg(target_os)]`
- Consider the `global-hotkey` crate which abstracts platform differences

## Complexity: S
## Estimated Phases: ~2
