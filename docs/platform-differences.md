# Platform Differences: macOS vs Linux

## Feature Matrix

| Feature | macOS | Linux (X11) | Linux (Wayland) |
|---------|-------|-------------|-----------------|
| GPU rendering (wgpu) | Metal | Vulkan | Vulkan |
| Font rasterization | CoreText (RGBA subpixel) | cosmic-text/swash (grayscale) | cosmic-text/swash (grayscale) |
| Context menus | Native NSMenu (blocking) | iced overlay widget | iced overlay widget |
| Global hotkey (Quick Terminal) | Works | Works (XGrabKey) | May fail (no standard protocol) |
| Clipboard | macOS pasteboard | X11 selections | wl-clipboard protocol |
| Display scaling | CoreGraphics + winit | winit auto-detect | winit auto-detect |
| Titlebar color | NSWindow API | No-op | No-op |
| URL opening | `open` command | `xdg-open` command | `xdg-open` command |
| Process detection | `proc_listchildpids` | `/proc/<pid>/task/<pid>/children` | `/proc/<pid>/task/<pid>/children` |
| .app bundle required | Yes (for HiDPI) | No | No |

## Context Menus

**macOS**: Uses native `NSMenu` via objc2 bindings. The menu is synchronous/blocking
-- `show_context_menu()` does not return until the user selects an item or dismisses.
Provides native macOS look and feel with keyboard shortcut hints.

**Linux**: Uses an iced overlay widget rendered on top of the terminal content. The menu
is event-driven -- right-click sets `context_menu_visible = true`, user clicks dispatch
`UiMessage::ContextMenuAction`, and clicking outside or pressing any key dismisses the
menu. Visual style matches the terminal theme.

## Global Hotkeys (Quick Terminal)

**macOS**: `global-hotkey` crate registers via the macOS Carbon Events API. Works
reliably with any hotkey combination.

**Linux X11**: Uses `XGrabKey` to register global hotkeys. Works on all X11 window
managers and desktop environments.

**Linux Wayland**: Wayland does not have a standard protocol for global hotkey
registration. The `global-hotkey` crate may use X11 compatibility (XWayland) or fail
gracefully. When registration fails, VeloTerm logs a warning:

```
WARN: Failed to register quick terminal hotkey: ...
Note: global hotkeys may not work on Wayland compositors that don't support
the X11 XGrabKey protocol.
```

The terminal itself works fine; only the global toggle hotkey is affected.

## Clipboard

**macOS**: Uses the macOS system pasteboard via `arboard`. Copy is `Cmd+C`, paste is
`Cmd+V`.

**Linux**: `arboard` auto-detects X11 vs Wayland at runtime. On X11, it uses the
X11 selection protocol. On Wayland, it uses the `wl-clipboard` data device protocol.
Copy is `Ctrl+Shift+C`, paste is `Ctrl+Shift+V` (standard terminal emulator convention
-- `Ctrl+C` sends SIGINT).

## Font Rendering

**macOS**: CoreText rasterizer produces RGBA bitmaps with subpixel antialiasing.
Each color channel carries independent coverage for subpixel AA blending. The glyph
atlas uses 4 bytes per pixel.

**Linux**: cosmic-text/swash rasterizer produces grayscale (alpha-only) bitmaps. The
glyph atlas uses 1 byte per pixel. Text is clear and legible but without subpixel
positioning.

Both platforms bundle Source Code Pro as the default font. cosmic-text's `FontSystem`
provides automatic system font fallback for missing glyphs (emoji, CJK, etc.). For
best coverage on CentOS 9, install `google-noto-fonts-common`.

## Display Scaling

**macOS**: Requires a `.app` bundle with `NSHighResolutionCapable=true` in Info.plist
for winit to report the correct 2x scale factor on Retina displays. Without the bundle,
`src/platform/macos.rs` detects the actual scale via CoreGraphics.

**Linux**: winit auto-detects the display scale from X11 (Xft.dpi resource) or Wayland
(wl_output.scale). No special configuration needed.
