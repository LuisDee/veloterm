# Track 27: Linux CentOS 9 Port — Specification

## Overview

Make VeloTerm compile, run, and achieve feature parity on Linux CentOS 9 (RHEL 9 family). The portability audit found zero compilation blockers — all macOS-specific code is already cfg-gated and Linux fallback paths exist. This track fills in stub implementations, validates the font rendering pipeline, adds CI coverage, and documents the Linux build process.

**Target environment:** CentOS 9 Stream, glibc 2.34, GNOME (Wayland default / X11 fallback), mesa (Vulkan 1.2 via RADV/ANV), systemd.

## Functional Requirements

### FR-1: Cross-Compile Validation
- `cargo check --target x86_64-unknown-linux-gnu` passes with zero errors
- File-level `#![cfg(target_os = "macos")]` guard added to `src/renderer/coretext_rasterizer.rs`
- All existing 1236+ tests continue to pass on macOS

### FR-2: Linux Platform Module
- `src/platform/linux.rs` created with:
  - `foreground_process_name(pid)` — reads `/proc/<pid>/task/<pid>/children` and `/proc/<child>/comm`
  - `detect_display_scale()` — returns winit-reported value (no CoreGraphics equivalent needed)
  - `check_hidpi_status()` — no-op stub (dead code on macOS too, provided for API symmetry)
  - `set_titlebar_color()` — no-op stub (X11/Wayland don't support programmatic titlebar colors)
- `src/platform/mod.rs` updated with `#[cfg(target_os = "linux")] pub mod linux;`
- `src/pty/mod.rs` Linux path calls real `foreground_process_name()` instead of returning `None`

### FR-3: Cross-Platform PTY Environment
- `COLORTERM=truecolor` added to PTY spawn environment (all platforms)
- `TERM_PROGRAM=VeloTerm` added to PTY spawn environment (all platforms)
- Existing `TERM=xterm-256color` unchanged

### FR-4: MarkdownLinkClicked Cross-Platform Fix
- `src/window.rs` `MarkdownLinkClicked` handler gains `#[cfg(not(target_os = "macos"))]` block using `xdg-open`

### FR-5: Linux Context Menus (iced Overlay)
- Context menus implemented as iced overlay widgets for Linux (replacing `None` stubs)
- Existing macOS NSMenu implementation preserved unchanged
- Menus support: Copy, Paste, Select All, Open Link, Search

### FR-6: Cross-Platform Integration Validation
- `global-hotkey` validated on X11; Wayland limitations documented
- `arboard` clipboard validated on X11 and Wayland
- cosmic-text/swash font rendering validated with correct metrics

### FR-7: CI Pipeline
- GitHub Actions workflow with CentOS 9 Stream container (`quay.io/centos/centos:stream9`)
- Runs `cargo check`, `cargo test`, `cargo clippy`
- Documents required system dependencies (`dnf install` command)

### FR-8: Documentation
- Linux build instructions (system deps, build commands)
- Known macOS vs Linux differences
- Wayland global hotkey limitations
- Font fallback recommendations (`google-noto-fonts-common`)

## Non-Functional Requirements

- No new heavyweight dependencies (no GTK, no Qt)
- Must work on both X11 and Wayland
- All platform-specific code behind `#[cfg(target_os)]` blocks
- 80% line coverage target for new code
- 100% test pass rate

## Acceptance Criteria

1. `cargo check --target x86_64-unknown-linux-gnu` passes with zero errors
2. `cargo build --target x86_64-unknown-linux-gnu` produces a working binary
3. All 1236+ existing tests pass on macOS (no regressions)
4. New Linux-specific tests pass (foreground process detection, /proc parsing, PTY environment)
5. `foreground_process_name()` returns correct process names on Linux via /proc
6. PTY sessions set COLORTERM=truecolor and TERM_PROGRAM=VeloTerm on ALL platforms
7. Context menus render via iced on Linux; macOS NSMenu preserved
8. Clipboard copy/paste works on X11 and Wayland
9. `MarkdownLinkClicked` handler works cross-platform (xdg-open on Linux)
10. GitHub Actions CI builds and tests on CentOS 9 container
11. Linux build documentation is complete and accurate

## Out of Scope

- Windows support
- Linux packaging (RPM, AppImage, Flatpak)
- Wayland-native window decorations
- Custom Linux installer or desktop entry files
- Performance benchmarking on Linux
- Changes to macOS code paths (except cross-platform env var additions)
