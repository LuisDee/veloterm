#!/bin/bash
# Script for LLMs to programmatically capture VeloTerm screenshots
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

# Wait for VeloTerm to fully start
echo "⏳ Waiting for VeloTerm to initialize..."
sleep 3

# Find the PID of the actual binary launched via open
VELOTERM_PID=$(pgrep -f "veloterm-bin" | head -1)
if [ -z "$VELOTERM_PID" ]; then
    echo "❌ VeloTerm failed to start. Check logs:"
    tail -20 /tmp/veloterm-screenshot.log
    exit 1
fi

echo "📸 Triggering screenshot (Cmd+Shift+S)..."

# Send Cmd+Shift+S to VeloTerm
osascript <<EOF
tell application "System Events"
    set frontApp to first application process whose frontmost is true
    set frontmost of (first process whose name contains "veloterm") to true
    delay 0.5
    keystroke "s" using {command down, shift down}
    delay 1
    set frontmost of frontApp to true
end tell
EOF

# Wait for screenshot to be written
sleep 2

# Kill VeloTerm
kill $VELOTERM_PID 2>/dev/null || true
sleep 1

# Verify screenshot exists
if [ -f "$SCREENSHOT_FILE" ]; then
    SIZE=$(du -h "$SCREENSHOT_FILE" | cut -f1)
    DIMS=$(sips -g pixelWidth -g pixelHeight "$SCREENSHOT_FILE" 2>/dev/null | grep -E 'pixelWidth|pixelHeight' | awk '{print $2}' | paste -sd 'x' -)
    echo "✅ Screenshot saved: veloterm-latest.png (Retina 2x)"
    echo "   Size: $SIZE"
    echo "   Dimensions: $DIMS (physical pixels at 2x scale)"
    echo "   Path: $SCREENSHOT_FILE"
    echo ""
    echo "📋 LLM: Use this path to view:"
    echo "   $SCREENSHOT_FILE"
else
    echo "❌ Screenshot file not found at: $SCREENSHOT_FILE"
    tail -20 /tmp/veloterm-screenshot.log
    exit 1
fi
