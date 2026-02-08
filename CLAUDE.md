# VeloTerm — Development Guide

## Building & Running

**macOS requires a .app bundle for proper rendering.** Running the bare binary
(`cargo run`) results in scale_factor=1.0 on Retina displays — fonts render at
half size and look blurry. Always use:

```bash
./run.sh          # debug build, launches as .app bundle
./run.sh release  # release build
```

This wraps the binary in `target/VeloTerm.app/` with an `Info.plist` that sets
`NSHighResolutionCapable=true`, giving winit the correct 2x scale factor.

As a fallback, `src/platform/macos.rs` detects the actual display scale via
CoreGraphics even without a bundle, but the `.app` path is preferred.

## Screenshots

**Always use `./take-screenshot.sh`** — it is the only reliable method:

```bash
./take-screenshot.sh
# Then view with:
Read("/Users/luisdeburnay/work/terminal-em/veloterm-latest.png")
```

How it works:
1. Builds VeloTerm and creates the `.app` bundle (with `VELOTERM_PROJECT_DIR` set)
2. Launches via `open` for proper Retina scaling
3. Sends Cmd+Shift+S to trigger GPU buffer capture
4. Saves to `veloterm-latest.png` (overwrites previous)

**DO NOT manually build the .app wrapper in bash commands.** The script sets
`VELOTERM_PROJECT_DIR` in the wrapper so the GPU capture knows where to write
the PNG. If you construct the wrapper yourself and forget this env var, the
capture fails with "Read-only file system" because it tries to write inside
the `.app` bundle. Just run the script.

## Testing

Use the `test-runner` skill (Bash subagent, haiku model) for all test execution.
Never run `cargo test` directly in the main context — it pollutes context with
verbose output.

## Architecture

- **lib.rs + main.rs pattern** for testability
- **Renderer**: wgpu-based GPU pipeline, CoreText glyph rasterization on macOS
  (swash/cosmic-text on other platforms)
- **Font**: Bundled JetBrains Mono (system fonts may be Nerd Font variants with
  wrong metrics)
- **Config**: TOML at `~/.config/veloterm/config.toml`, hot-reloads on change
- **PTY**: alacritty_terminal for VT parsing, portable-pty for shell spawning

## Key Constraints

- Atlas minimum size: 512px (smaller causes UV sampling issues)
- wgpu surface dimensions MUST be clamped to `device.limits().max_texture_dimension_2d`
  before `Surface::configure` — macOS windows can exceed GPU limits
- Rust env: prefix shell commands with `source "$HOME/.cargo/env" &&`
- `ls` is aliased to neofetch on this machine — use Glob tool instead
