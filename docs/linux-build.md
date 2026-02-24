# Building VeloTerm on Linux (CentOS Stream 9 / RHEL 9)

## System Dependencies

Install build dependencies via `dnf`:

```bash
sudo dnf install -y \
  gcc gcc-c++ cmake make pkg-config \
  mesa-libEGL-devel mesa-libGL-devel mesa-vulkan-drivers \
  libX11-devel libXcursor-devel libXrandr-devel libXi-devel \
  wayland-devel libxkbcommon-devel \
  fontconfig-devel freetype-devel \
  git curl
```

### Package breakdown

| Package | Purpose |
|---------|---------|
| `gcc`, `gcc-c++`, `cmake`, `make` | C/C++ toolchain for native dependencies |
| `pkg-config` | Locating system libraries during build |
| `mesa-libEGL-devel`, `mesa-libGL-devel` | OpenGL/EGL headers for wgpu |
| `mesa-vulkan-drivers` | Vulkan ICD (Intel/AMD) for wgpu Vulkan backend |
| `libX11-devel`, `libXcursor-devel`, `libXrandr-devel`, `libXi-devel` | X11 windowing support |
| `wayland-devel`, `libxkbcommon-devel` | Wayland windowing support |
| `fontconfig-devel`, `freetype-devel` | Font discovery and rasterization (cosmic-text) |

### Vulkan drivers

VeloTerm uses wgpu which auto-selects the best GPU backend:

- **Intel**: `mesa-vulkan-drivers` provides the Intel ANV driver
- **AMD**: `mesa-vulkan-drivers` provides the AMD RADV driver
- **NVIDIA**: Install the proprietary NVIDIA driver package from RPM Fusion

Verify Vulkan support:
```bash
vulkaninfo --summary 2>/dev/null | head -5
```

### Optional: font packages

VeloTerm bundles Source Code Pro, but for system font fallback (emoji, CJK, etc.):

```bash
sudo dnf install -y google-noto-fonts-common google-noto-sans-cjk-fonts
```

## Building

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build --release

# Run
./target/release/veloterm
```

## Running

Unlike macOS (which requires a `.app` bundle for proper Retina scaling), on Linux the
bare binary runs correctly. winit auto-detects the display scale from X11/Wayland.

```bash
cargo run --release
```

## Troubleshooting

### "Failed to create wgpu adapter"

Ensure Vulkan drivers are installed. On headless servers or containers without GPU
access, VeloTerm will not run (GPU-accelerated rendering is required).

### "Failed to register quick terminal hotkey"

Global hotkeys use X11 XGrabKey. On Wayland compositors that don't support the
`wlr-foreign-toplevel-management` protocol, global hotkey registration will fail
gracefully with a log warning. The terminal itself works fine; only the global
hotkey toggle for Quick Terminal is affected.

### Font rendering looks different from macOS

macOS uses CoreText with subpixel antialiasing (RGBA atlas). Linux uses
cosmic-text/swash with grayscale antialiasing (R8 atlas). Text is legible on
both platforms but may appear slightly different.
