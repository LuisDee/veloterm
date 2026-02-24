<!-- ARCHITECT CONTEXT | Track: 27_linux_centos9 | Wave: 14 | CC: v1 -->

## Cross-Cutting Constraints
- Platform Abstraction (CC v1): All platform-specific code isolated behind `#[cfg(target_os)]` blocks or trait boundaries
- No new heavyweight dependencies: no GTK, no Qt -- context menus via iced overlay widgets
- Must work on both X11 and Wayland (CentOS 9 ships both via GNOME)
- CentOS 9 ships glibc 2.34, mesa (Vulkan/OpenGL), wayland-protocols -- all compatible with wgpu
- Bundled Source Code Pro font handles font availability; no system font assumptions
- Testing (CC v1): TDD approach, cargo test --lib, clippy, fmt
- cfg-gating safety: ensure coretext_rasterizer.rs has file-level `#![cfg(target_os = "macos")]` guard
- All 1236 existing tests must continue to pass on macOS after changes

## Interfaces

### Owns
- `src/platform/linux.rs` -- Linux platform module (display scale detection, process info, titlebar stubs)
- Linux foreground process detection via `/proc` filesystem
- Cross-platform PTY environment variables (COLORTERM, TERM_PROGRAM) -- benefits all platforms
- Linux CI pipeline (GitHub Actions CentOS 9 container)
- Linux build documentation

### Consumes
- `Config` (Track 03) -- font_family, font_size, theme settings, hotkey config
- `PtySession` (Track 02) -- PTY spawn with environment variables
- `GlyphAtlas` (Track 01/25) -- cosmic-text/swash rendering path (already exists)
- `HotkeyManager` (Track 14) -- `global-hotkey` crate on X11/Wayland
- `ContextMenuAction` (Track 20) -- existing enum, Linux stubs need implementation via iced
- `arboard` clipboard (Track 17) -- X11 and Wayland clipboard backends

## Dependencies
- All 26 prior tracks are COMPLETE -- no blockers
- Track 24 (iced UI chrome) -- context menu iced implementation builds on iced widget layer
- Track 25 (glyphon) -- font rendering path for Linux

<!-- END ARCHITECT CONTEXT -->

# Track 27: Linux CentOS 9 Port

## What This Track Delivers

Makes VeloTerm compile, run, and achieve feature parity on Linux CentOS 9 (RHEL 9 family). The audit found ZERO compilation blockers -- all macOS-specific code is already cfg-gated and Linux fallback paths exist for every platform-dependent function. This track fills in the stub implementations (foreground process detection, context menus, PTY environment), validates the font rendering pipeline on Linux, adds CI coverage, and documents the Linux build process.

The target environment is CentOS 9 with glibc 2.34, GNOME desktop (Wayland default, X11 fallback), mesa for GPU (Vulkan via RADV/ANV, OpenGL via llvmpipe), and systemd.

## Scope

### IN
- Cross-compile validation: `cargo check --target x86_64-unknown-linux-gnu` passes cleanly
- File-level cfg guard on `src/renderer/coretext_rasterizer.rs` for compile safety
- `src/platform/linux.rs` with real implementations:
  - `foreground_process_name()` via `/proc/<pid>/task/<pid>/children` + `/proc/<child>/comm`
  - `detect_display_scale()` stub (winit handles this on Linux; return winit value)
  - `check_hidpi_status()` stub (no .app bundle concept on Linux; note: this function is dead code on macOS too -- never called -- but stub provided for API symmetry)
  - `set_titlebar_color()` stub (Wayland/X11 do not support programmatic titlebar colors via raw window handle in the same way)
- Update `src/platform/mod.rs` to conditionally include linux module
- Update `src/pty/mod.rs` Linux `foreground_process_name()` to call real implementation
- Fix `MarkdownLinkClicked` handler in `src/window.rs` (macOS-only, needs `xdg-open` fallback for Linux)
- PTY environment enrichment (cross-platform): `COLORTERM=truecolor`, `TERM_PROGRAM=VeloTerm` -- benefits macOS too
- Font rendering validation: cosmic-text handles font fallback internally (bundled Source Code Pro primary, system fonts for missing glyphs)
- Validate cosmic-text/swash rendering path produces correct metrics on Linux
- Context menus via iced overlay widgets (not GTK) for Linux only -- replace the `None` stubs. Keep existing NSMenu on macOS (native feel preserved)
- Validate `global-hotkey` crate on X11 and Wayland
- Validate `arboard` clipboard on X11 and Wayland
- GitHub Actions CI with CentOS 9 container (build + test)
- Linux build documentation (dnf dependencies, build commands, known differences)
- Document known platform differences (macOS vs Linux)

### OUT
- Windows support (separate future track)
- Packaging (RPM, AppImage, Flatpak -- separate future track)
- Wayland-native window decorations (depends on winit/wgpu Wayland maturity)
- Custom Linux installer or desktop entry files
- Performance benchmarking on Linux (separate effort)
- Any changes to the macOS code paths

## Key Design Decisions

1. **Linux foreground process detection: /proc parsing vs. external crate?**
   - **Option A (Direct /proc):** Read `/proc/<pid>/task/<pid>/children` for child PIDs, then `/proc/<child>/comm` for process name. Zero dependencies, works on all Linux kernels >= 3.5, but requires parsing procfs text format.
   - **Option B (procfs crate):** Use the `procfs` crate for typed access to /proc. Cleaner API but adds a dependency.
   - **Option C (sysinfo crate):** Use `sysinfo` for process tree queries. Heavy dependency, overkill for one function.
   - Trade-off: Direct /proc reading is simple enough for this use case (two file reads) and avoids new dependencies, consistent with the "no heavyweight deps" constraint.

2. **Context menus on Linux: iced overlay vs. GTK dependency vs. no menus?**
   - **Option A (iced overlay):** Implement context menus as iced widgets rendered in the compositor overlay. Works on both X11 and Wayland without external dependencies. Consistent with the existing iced UI chrome.
   - **Option B (GTK):** Link against GTK for native context menus. Heavyweight dependency, breaks the "no GTK" constraint, and creates a mixed toolkit situation.
   - **Option C (Stub/None):** Keep returning None on Linux. Functional but feature-incomplete.
   - Trade-off: iced overlay is the right approach for Linux since the project already uses iced for all UI chrome (Track 24). **On macOS, keep the existing NSMenu implementation** -- it provides native system integration (services menu, dictation, etc.) that an iced overlay cannot replicate. The Linux iced menus will match the VeloTerm UI style, which is acceptable since Linux desktop menus are less standardized.

3. **Display scale detection: Linux-specific detection vs. trust winit?**
   - On macOS, `detect_display_scale()` queries CoreGraphics because bare binaries outside .app bundles get incorrect scale. On Linux, winit gets the correct scale from X11/Wayland natively.
   - **Decision:** The Linux `detect_display_scale()` should simply return the winit-reported value. No CoreGraphics equivalent needed.

4. **PTY environment variables: which to set?**
   - `TERM=xterm-256color` is already set (existing code).
   - `COLORTERM=truecolor` advertises 24-bit color support to CLI tools (bat, delta, ls --color).
   - `TERM_PROGRAM=VeloTerm` identifies the terminal to shell integration scripts.
   - `TERM_PROGRAM_VERSION` could be added for completeness.
   - Trade-off: Setting COLORTERM and TERM_PROGRAM is low-risk and high-value for CLI tool integration on ALL platforms (bat, delta, starship, etc. check these on macOS too). These should be added cross-platform, not Linux-only.

5. **Font fallback on Linux: bundled font sufficient vs. system font chain?**
   - VeloTerm bundles Source Code Pro Medium via `include_bytes!()`. cosmic-text loads this directly, no system font needed for ASCII/Latin.
   - For characters outside Source Code Pro's coverage (CJK, emoji, box-drawing), cosmic-text's built-in system font fallback automatically queries system fonts. There is no custom fallback chain to implement -- cosmic-text handles this internally via its `FontSystem`.
   - **Decision:** The bundled font is primary. cosmic-text's built-in fallback is sufficient. Document that CentOS 9 users should install `google-noto-fonts-common` for broad Unicode coverage.

6. **CI container: CentOS 9 base vs. Fedora vs. Ubuntu?**
   - The target is CentOS 9, so CI should use CentOS 9 (or RHEL 9 UBI) to catch glibc/library compatibility issues.
   - **Decision:** Use `quay.io/centos/centos:stream9` as the CI base image. Install build deps via dnf.

## Architectural Notes

- **All macOS code is already isolated.** `src/platform/mod.rs` only includes `macos` module under `#[cfg(target_os = "macos")]`. `src/context_menu.rs` has `#[cfg(target_os = "macos")]` on the macOS impl and `#[cfg(not(target_os = "macos"))]` on the stub. `src/pty/mod.rs` has cfg-gated foreground process detection. `Cargo.toml` has macOS-only dependencies under `[target.'cfg(target_os = "macos")'.dependencies]`.
- **coretext_rasterizer.rs is the one file without a file-level cfg guard.** It uses macOS-only crate imports (`core_text`, `core_graphics`, `core_foundation`) at the top. While `glyph_atlas.rs` conditionally imports it via `#[cfg(target_os = "macos")]`, adding a file-level guard is defense-in-depth.
- **wgpu on CentOS 9:** wgpu selects Vulkan (via mesa/RADV on AMD, ANV on Intel) or OpenGL as fallback. CentOS 9 ships mesa 22.3+ which supports Vulkan 1.2. The `wgpu` crate handles backend selection automatically.
- **global-hotkey on Linux:** The `global-hotkey` crate uses X11 XGrabKey on X11 and has experimental Wayland support. On Wayland, global hotkeys may require compositor-specific protocols (not universally supported). This is a known limitation to document.
- **arboard on Linux:** The `arboard` crate supports both X11 (via x11-clipboard) and Wayland (via wl-clipboard). It auto-detects the display server at runtime. CentOS 9's GNOME runs on Wayland by default with X11 fallback.

## Phases

### Phase 1: Build & Compile Validation (~1 day)
- **Prerequisite:** `rustup target add x86_64-unknown-linux-gnu` (cross-compile target). Note: full `cargo build` cross-compilation requires a Linux sysroot and linker -- `cargo check` (type-checking only) works without a linker and is sufficient for Phase 1 validation
- Cross-compile check: `cargo check --target x86_64-unknown-linux-gnu`
- Add file-level `#![cfg(target_os = "macos")]` to `src/renderer/coretext_rasterizer.rs`
- Fix `MarkdownLinkClicked` handler in `src/window.rs` -- add `xdg-open` fallback for non-macOS
- Add `COLORTERM=truecolor` and `TERM_PROGRAM=VeloTerm` to PTY environment (cross-platform, in `src/pty/mod.rs`)
- Verify all cfg gates compile cleanly
- Document CentOS 9 system dependencies (`dnf install` command)
- **Gate:** `cargo check` passes for linux-gnu target, all existing tests still pass on macOS

### Phase 2: Platform Module Implementation (~2 days)
- Create `src/platform/linux.rs` with:
  - `foreground_process_name()` via `/proc` filesystem
  - `detect_display_scale()` passthrough
  - `check_hidpi_status()` no-op
  - `set_titlebar_color()` no-op
- Update `src/platform/mod.rs` to conditionally include linux module
- Update `src/pty/mod.rs` to call linux platform impl for foreground process
- Unit tests for /proc parsing (mock /proc paths for testing)
- **Gate:** All platform functions have Linux implementations, tests pass

### Phase 3: Terminal Environment & Font Validation (~1 day)
- Validate cosmic-text/swash font rendering path (the existing non-macOS path in glyph_atlas.rs)
- Test font metric consistency: bundled Source Code Pro at various DPI values
- Verify cosmic-text built-in system font fallback works for CJK/emoji on Linux
- Add font fallback documentation for Linux (recommend `google-noto-fonts-common`)
- **Gate:** Font rendering produces valid atlas, PTY environment verified (COLORTERM/TERM_PROGRAM added in Phase 1)

### Phase 4: Feature Parity — Context Menus & Integration (~2 days)
- Implement context menus as iced overlay widgets for Linux only (replacing the `None` stubs in the `#[cfg(not(target_os = "macos"))]` path)
- **Keep existing NSMenu on macOS** -- native system integration (services, dictation) is worth preserving
- Validate `global-hotkey` on X11 (document Wayland limitations)
- Validate `arboard` clipboard on X11 and Wayland
- **Gate:** Right-click context menus work on Linux, clipboard works on both display servers, macOS NSMenu unchanged

### Phase 5: CI & Documentation (~1 day)
- GitHub Actions workflow: CentOS 9 container, build + test
- Linux build instructions in project documentation
- Document known macOS vs Linux differences
- Document Wayland global hotkey limitations
- **Gate:** CI green on CentOS 9, documentation complete

## Acceptance Criteria

1. `cargo check --target x86_64-unknown-linux-gnu` passes with zero errors
2. `cargo build --target x86_64-unknown-linux-gnu` produces a working binary
3. All 1236+ existing tests pass on macOS (no regressions)
4. New Linux-specific tests pass (foreground process detection, /proc parsing, PTY environment)
5. `foreground_process_name()` returns correct process names on Linux via /proc
6. PTY sessions set COLORTERM=truecolor and TERM_PROGRAM=VeloTerm on ALL platforms
7. Context menus render via iced on Linux (not returning None); macOS NSMenu preserved
8. Clipboard copy/paste works on X11 and Wayland
9. `MarkdownLinkClicked` handler works cross-platform (xdg-open on Linux)
10. GitHub Actions CI builds and tests on CentOS 9 container
11. Linux build documentation is complete and accurate

## Test Strategy

- **Framework:** `cargo test` (consistent with all 1236 existing tests)
- **Unit tests:**
  - `/proc/<pid>/children` parsing with mock data
  - `/proc/<pid>/comm` reading with mock data
  - `foreground_process_name()` returns None for process with no children
  - PTY environment variables are set correctly (COLORTERM, TERM_PROGRAM)
  - Font metric validation: bundled Source Code Pro produces consistent cell dimensions
- **Integration tests (Linux CI only):**
  - Full PTY session spawn and output on Linux
  - Cross-compile check as CI step
  - wgpu device creation on CentOS 9 (may need software rendering in CI)
- **Regression tests:**
  - All 1236 existing tests must pass without modification on macOS
- **Quality threshold:** 80% line coverage (advisory), 100% pass rate

## Complexity: L
## Estimated Phases: 5
