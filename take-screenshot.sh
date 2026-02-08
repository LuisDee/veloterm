#!/bin/bash
# Script for LLMs to programmatically capture VeloTerm screenshots
# Uses macOS screencapture + GetWindowID for color-accurate window capture.
# Always saves to: veloterm-latest.png (overwrites previous)
# Usage: ./take-screenshot.sh

set -e

PROJECT_DIR="/Users/luisdeburnay/work/terminal-em"
SCREENSHOT_FILE="$PROJECT_DIR/veloterm-latest.png"

cd "$PROJECT_DIR"

# Clean up any old screenshots (keep only latest)
echo "🧹 Cleaning up old screenshots..."
rm -f veloterm-*.png 2>/dev/null || true

echo "🚀 Building and launching VeloTerm (.app bundle for Retina)..."
source "$HOME/.cargo/env"
cargo build > /tmp/veloterm-screenshot.log 2>&1
APP="$PROJECT_DIR/target/VeloTerm.app"
mkdir -p "$APP/Contents/MacOS"
cp "$PROJECT_DIR/target/debug/veloterm" "$APP/Contents/MacOS/veloterm-bin"
cp "$PROJECT_DIR/resources/macos/Info.plist" "$APP/Contents/Info.plist"

# Create wrapper script so open(1) launches with RUST_LOG and log redirection
cat > "$APP/Contents/MacOS/veloterm" << WRAPPER
#!/bin/bash
DIR="\$(dirname "\$0")"
export RUST_LOG=info
export VELOTERM_PROJECT_DIR="$PROJECT_DIR"
exec "\$DIR/veloterm-bin" >> /tmp/veloterm-screenshot.log 2>&1
WRAPPER
chmod +x "$APP/Contents/MacOS/veloterm"

# Launch via 'open' for proper .app bundle behavior (Retina scaling)
open "$APP"

# Wait for VeloTerm to fully start and shell to produce output
echo "⏳ Waiting for VeloTerm to initialize..."
sleep 5

# Find the PID of the actual binary launched via open
VELOTERM_PID=$(pgrep -f "veloterm-bin" | head -1)
if [ -z "$VELOTERM_PID" ]; then
    echo "❌ VeloTerm failed to start. Check logs:"
    tail -20 /tmp/veloterm-screenshot.log
    exit 1
fi

echo "📸 Capturing screenshot..."

# Bring VeloTerm to front
osascript -e 'tell application "System Events" to set frontmost of (first process whose name contains "veloterm") to true' 2>/dev/null
sleep 1

# Use GetWindowID to get the CGWindowID, then screencapture
# Window title is set by our code to "Claude Terminal — Anthropic"
WINDOW_ID=$(GetWindowID "VeloTerm" "Claude Terminal — Anthropic" 2>/dev/null || echo "")
# If that fails, try listing all windows and grabbing the main one
if [ -z "$WINDOW_ID" ] || [ "$WINDOW_ID" = "0" ]; then
    WINDOW_ID=$(GetWindowID "VeloTerm" --list 2>/dev/null | grep -v "size=500x500" | head -1 | grep -oE 'id=[0-9]+' | cut -d= -f2)
fi
echo "Window ID: $WINDOW_ID"

if [ -n "$WINDOW_ID" ] && [ "$WINDOW_ID" != "0" ]; then
    screencapture -o -x -l "$WINDOW_ID" "$SCREENSHOT_FILE" 2>/dev/null
fi

# Fallback: if window capture failed, use Cmd+Shift+S GPU capture
if [ ! -f "$SCREENSHOT_FILE" ] || [ ! -s "$SCREENSHOT_FILE" ]; then
    echo "OS capture failed, falling back to GPU capture via Cmd+Shift+S..."
    osascript -e 'tell application "System Events"' \
              -e 'set frontmost of (first process whose name contains "veloterm") to true' \
              -e 'delay 0.5' \
              -e 'keystroke "s" using {command down, shift down}' \
              -e 'delay 1.5' \
              -e 'end tell' 2>/dev/null
    sleep 2
fi

sleep 1

# Kill VeloTerm
kill $VELOTERM_PID 2>/dev/null || true
sleep 1

# Verify screenshot exists
if [ -f "$SCREENSHOT_FILE" ] && [ -s "$SCREENSHOT_FILE" ]; then
    SIZE=$(du -h "$SCREENSHOT_FILE" | cut -f1)
    DIMS=$(sips -g pixelWidth -g pixelHeight "$SCREENSHOT_FILE" 2>/dev/null | grep -E 'pixelWidth|pixelHeight' | awk '{print $2}' | paste -sd 'x' -)
    echo "✅ Screenshot saved: veloterm-latest.png"
    echo "   Size: $SIZE"
    echo "   Dimensions: $DIMS"
    echo "   Path: $SCREENSHOT_FILE"
    echo ""
    echo "📋 LLM: Use this path to view:"
    echo "   $SCREENSHOT_FILE"
else
    echo "❌ Screenshot failed"
    tail -20 /tmp/veloterm-screenshot.log
    exit 1
fi
