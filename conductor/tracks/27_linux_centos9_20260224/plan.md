# Track 27: Linux CentOS 9 Port — Implementation Plan

## Phase 1: Build & Compile Validation

- [x] Task: Add file-level `#![cfg(target_os = "macos")]` guard to `src/renderer/coretext_rasterizer.rs` <!-- 253bca0 -->
  - [x] Add inner attribute at top of file
  - [x] Verify macOS tests still pass

- [x] Task: Add cross-platform PTY environment variables <!-- 253bca0 -->
  - [x] Add `cmd.env("COLORTERM", "truecolor")` to `src/pty/mod.rs`
  - [x] Add `cmd.env("TERM_PROGRAM", "VeloTerm")` to `src/pty/mod.rs`
  - [x] Verify tests pass

- [x] Task: Fix MarkdownLinkClicked handler for cross-platform support <!-- 253bca0 -->
  - [x] Add `#[cfg(not(target_os = "macos"))]` block with `xdg-open` to `src/window.rs`
  - [x] Verify existing macOS path unchanged

- [x] Task: Install Linux cross-compile target and validate <!-- 253bca0 -->
  - [x] `rustup target add x86_64-unknown-linux-gnu`
  - [x] Run `cargo check --target x86_64-unknown-linux-gnu`
  - [x] Fix any compilation errors discovered

- [ ] Task: Conductor - User Manual Verification 'Phase 1: Build & Compile Validation' (Protocol in workflow.md)

## Phase 2: Platform Module Implementation

- [x] Task: Create `src/platform/linux.rs` with platform stubs <!-- a20a463 -->
  - [x] Write tests for `foreground_process_name()` with mock /proc data
  - [x] Write tests for edge cases (no children, invalid pid, missing /proc files)
  - [x] Implement `foreground_process_name()` via `/proc/<pid>/task/<pid>/children` + `/proc/<child>/comm`
  - [x] Implement `detect_display_scale()` as winit passthrough
  - [x] Implement `check_hidpi_status()` as no-op
  - [x] Implement `set_titlebar_color()` as no-op

- [x] Task: Update `src/platform/mod.rs` to include Linux module <!-- a20a463 -->
  - [x] Add `#[cfg(target_os = "linux")] pub mod linux;`

- [x] Task: Wire Linux foreground process detection into PTY <!-- a20a463 -->
  - [x] Update `src/pty/mod.rs` non-macOS `foreground_process_name()` to call `platform::linux::foreground_process_name()`
  - [x] Verify with unit tests

- [x] Task: Verify cross-compile still passes with new platform module <!-- a20a463 -->
  - [x] `cargo check --target x86_64-unknown-linux-gnu`
  - [x] All macOS tests pass

- [ ] Task: Conductor - User Manual Verification 'Phase 2: Platform Module Implementation' (Protocol in workflow.md)

## Phase 3: Terminal Environment & Font Validation

- [x] Task: Validate cosmic-text/swash font rendering path <!-- 0d3f1b4 -->
  - [x] Write test verifying atlas produces valid dimensions (atlas_minimum_512px_constraint)
  - [x] Write test verifying cell width/height consistency with bundled Source Code Pro (atlas_bundled_source_code_pro_metrics)
  - [x] Verify atlas minimum 512px constraint holds
  - [x] Write test verifying bytes_per_pixel matches platform (atlas_bytes_per_pixel_matches_platform)
  - [x] Write test verifying scale factors produce different sizes (atlas_scale_factors_produce_different_sizes)

- [x] Task: Verify cosmic-text system font fallback <!-- 0d3f1b4 -->
  - [x] cosmic-text FontSystem built-in fallback documented in brief.md
  - [x] google-noto-fonts-common recommendation in brief.md

- [ ] Task: Conductor - User Manual Verification 'Phase 3: Terminal Environment & Font Validation' (Protocol in workflow.md)

## Phase 4: Feature Parity — Context Menus & Integration

- [x] Task: Implement Linux context menus via iced overlay widgets <!-- ed31709 -->
  - [x] Write tests for context menu rendering on non-macOS path
  - [x] Create iced-based context menu widget (Copy, Paste, Select All, Clear, New Tab, Split, Close Pane)
  - [x] Wire into `src/window.rs` `#[cfg(not(target_os = "macos"))]` right-click handler
  - [x] Verify macOS NSMenu path is unchanged (cfg-gated)

- [x] Task: Validate global-hotkey on X11 and document Wayland limitations <!-- ed31709 -->
  - [x] Test `global-hotkey` registration validation (hotkey parsing tests)
  - [x] Add graceful fallback/error message if registration fails on Wayland
  - [x] Document Wayland compositor-specific protocol limitations in log message

- [x] Task: Validate arboard clipboard on X11 and Wayland <!-- ed31709 -->
  - [x] Test arboard clipboard init and roundtrip
  - [x] Validate Ctrl+Shift+C/V keybindings for Linux
  - [x] arboard auto-detects X11 vs Wayland at runtime

- [ ] Task: Conductor - User Manual Verification 'Phase 4: Feature Parity' (Protocol in workflow.md)

## Phase 5: CI & Documentation

- [ ] Task: Create GitHub Actions CI workflow for Linux
  - [ ] Create `.github/workflows/linux-ci.yml`
  - [ ] Use `quay.io/centos/centos:stream9` container
  - [ ] Install build dependencies via `dnf install`
  - [ ] Run `cargo check`, `cargo test --lib`, `cargo clippy`

- [ ] Task: Document CentOS 9 system dependencies
  - [ ] List required `dnf install` packages (gcc, cmake, pkg-config, mesa-libEGL-devel, etc.)
  - [ ] Document Vulkan driver requirements

- [ ] Task: Document known platform differences
  - [ ] macOS vs Linux feature matrix
  - [ ] Wayland global hotkey limitations
  - [ ] Font fallback recommendations
  - [ ] Context menu differences (NSMenu vs iced overlay)

- [ ] Task: Conductor - User Manual Verification 'Phase 5: CI & Documentation' (Protocol in workflow.md)
