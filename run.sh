#!/usr/bin/env bash
# VeloTerm — build & run script for macOS and Linux.
#
# Usage:
#   ./run.sh              Build debug, run
#   ./run.sh release      Build release, run
#   ./run.sh --clean      Force clean rebuild (shader changes)
#   ./run.sh --no-log     Don't tail the log after launch
#   ./run.sh --help       Show this help
#
# macOS: Creates a .app bundle and launches via open(1) for Retina HiDPI.
# Linux: Runs the binary directly (no bundle required).
set -euo pipefail

# ─── Defaults ────────────────────────────────────────────────
PROFILE="debug"
CLEAN=false
TAIL_LOG=true
LOG_FILE="/tmp/veloterm.log"
RUST_LOG="${RUST_LOG:-debug,wgpu=warn,naga=warn,cosmic_text=error,iced_wgpu=warn,iced_winit=warn}"

# ─── Arg parsing ─────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        release)    PROFILE="release" ;;
        --clean)    CLEAN=true ;;
        --no-log)   TAIL_LOG=false ;;
        --help|-h)
            sed -n '2,/^set /{ /^#/s/^# \?//p }' "$0"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Run '$0 --help' for usage."
            exit 1
            ;;
    esac
done

# ─── Build ───────────────────────────────────────────────────
if $CLEAN; then
    echo "Cleaning veloterm crate (force shader recompile)..."
    cargo clean -p veloterm 2>/dev/null || true
fi

echo "Building VeloTerm ($PROFILE)..."
if [ "$PROFILE" = "release" ]; then
    cargo build --release
    BINARY="target/release/veloterm"
else
    cargo build
    BINARY="target/debug/veloterm"
fi

if [ ! -f "$BINARY" ]; then
    echo "Build failed: $BINARY not found."
    exit 1
fi

# ─── Platform-specific launch ────────────────────────────────
OS="$(uname -s)"

case "$OS" in
    Darwin)
        # macOS: .app bundle required for correct Retina HiDPI scale factor.
        # Without it, winit reports scale_factor=1.0 on Retina displays.
        APP="target/VeloTerm.app"
        PROJECT_DIR="$(pwd)"

        mkdir -p "$APP/Contents/MacOS"
        cp "$BINARY" "$APP/Contents/MacOS/veloterm-bin"
        cp resources/macos/Info.plist "$APP/Contents/Info.plist"

        # Wrapper script: open(1) doesn't inherit env vars from the shell.
        cat > "$APP/Contents/MacOS/veloterm" << WRAPPER
#!/bin/bash
export RUST_LOG="$RUST_LOG"
export SHELL="\${SHELL:-/bin/zsh}"
export VELOTERM_PROJECT_DIR="$PROJECT_DIR"
exec "\$(dirname "\$0")/veloterm-bin" >> "$LOG_FILE" 2>&1
WRAPPER
        chmod +x "$APP/Contents/MacOS/veloterm"

        # Kill previous instance so open(1) launches the new binary
        pkill -f veloterm-bin 2>/dev/null || true
        sleep 0.3

        > "$LOG_FILE"
        open "$APP"
        echo "VeloTerm launched (.app bundle, Retina HiDPI)."
        ;;

    Linux)
        # Linux: run binary directly. No .app bundle needed.
        > "$LOG_FILE"
        echo "VeloTerm launching..."
        RUST_LOG="$RUST_LOG" "$BINARY" >> "$LOG_FILE" 2>&1 &
        disown
        echo "VeloTerm launched (PID $!)."
        ;;

    *)
        echo "Unsupported platform: $OS"
        echo "VeloTerm supports macOS and Linux."
        exit 1
        ;;
esac

echo "Logs: $LOG_FILE"

if $TAIL_LOG; then
    sleep 1
    tail -f "$LOG_FILE"
fi
