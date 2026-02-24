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

- [ ] Task: Create `src/platform/linux.rs` with platform stubs
  - [ ] Write tests for `foreground_process_name()` with mock /proc data
  - [ ] Write tests for edge cases (no children, invalid pid, missing /proc files)
  - [ ] Implement `foreground_process_name()` via `/proc/<pid>/task/<pid>/children` + `/proc/<child>/comm`
  - [ ] Implement `detect_display_scale()` as winit passthrough
  - [ ] Implement `check_hidpi_status()` as no-op
  - [ ] Implement `set_titlebar_color()` as no-op

- [ ] Task: Update `src/platform/mod.rs` to include Linux module
  - [ ] Add `#[cfg(target_os = "linux")] pub mod linux;`

- [ ] Task: Wire Linux foreground process detection into PTY
  - [ ] Update `src/pty/mod.rs` non-macOS `foreground_process_name()` to call `platform::linux::foreground_process_name()`
  - [ ] Verify with unit tests

- [ ] Task: Verify cross-compile still passes with new platform module
  - [ ] `cargo check --target x86_64-unknown-linux-gnu`
  - [ ] All macOS tests pass

- [ ] Task: Conductor - User Manual Verification 'Phase 2: Platform Module Implementation' (Protocol in workflow.md)

## Phase 3: Terminal Environment & Font Validation

- [ ] Task: Validate cosmic-text/swash font rendering path
  - [ ] Write test verifying `GlyphAtlas::new_swash()` produces valid atlas dimensions
  - [ ] Write test verifying cell width/height consistency with bundled Source Code Pro
  - [ ] Verify atlas minimum 512px constraint holds

- [ ] Task: Verify cosmic-text system font fallback
  - [ ] Document cosmic-text `FontSystem` built-in fallback behavior
  - [ ] Recommend `google-noto-fonts-common` for CentOS 9 Unicode coverage

- [ ] Task: Conductor - User Manual Verification 'Phase 3: Terminal Environment & Font Validation' (Protocol in workflow.md)

## Phase 4: Feature Parity — Context Menus & Integration

- [ ] Task: Implement Linux context menus via iced overlay widgets
  - [ ] Write tests for context menu rendering on non-macOS path
  - [ ] Create iced-based context menu widget (Copy, Paste, Select All, Open Link, Search)
  - [ ] Wire into `src/context_menu.rs` `#[cfg(not(target_os = "macos"))]` path
  - [ ] Verify macOS NSMenu path is unchanged

- [ ] Task: Validate global-hotkey on X11 and document Wayland limitations
  - [ ] Test `global-hotkey` registration on X11
  - [ ] Document Wayland compositor-specific protocol limitations
  - [ ] Add graceful fallback/error message if registration fails on Wayland

- [ ] Task: Validate arboard clipboard on X11 and Wayland
  - [ ] Test copy/paste on X11 backend
  - [ ] Test copy/paste on Wayland backend
  - [ ] Document any clipboard limitations

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
