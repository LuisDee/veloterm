import { execSync, execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { APP_PATH, FOCUS_DELAY_MS, KEY_CODES, SCREENSHOT_PATH } from "../constants.js";

/**
 * Find window ID for a given PID using JXA (JavaScript for Automation).
 * Uses CGWindowListCopyWindowInfo to match PID → window number.
 * Falls back to GetWindowID (Homebrew) if JXA fails.
 */
export function findWindowByPid(pid: number): number | null {
  // Primary: GetWindowID (Homebrew) — most reliable on macOS
  // Requires both app name and window title arguments
  try {
    const result = execSync('GetWindowID "VeloTerm" "VeloTerm" 2>/dev/null || echo ""', {
      timeout: 5000,
      encoding: "utf-8",
    }).trim();

    if (result && result !== "" && result !== "0") {
      return parseInt(result, 10);
    }
  } catch {
    // GetWindowID not installed or failed
  }

  // Fallback: JXA via osascript (requires Screen Recording permission)
  // Note: ObjC.deepUnwrap returns an NSArray which lacks JS Array methods,
  // so we iterate with objectAtIndex instead of using .find()
  try {
    const script = `
      ObjC.import('CoreGraphics');
      var rawList = $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0);
      var count = rawList.count;
      var result = '';
      for (var i = 0; i < count; i++) {
        var w = ObjC.deepUnwrap(rawList.objectAtIndex(i));
        if (w.kCGWindowOwnerPID === ${pid} && w.kCGWindowBounds && w.kCGWindowBounds.Height > 100) {
          result = '' + w.kCGWindowNumber;
          break;
        }
      }
      result;
    `;
    const result = execSync(`osascript -l JavaScript -e '${script.replace(/'/g, "'\\''")}'`, {
      timeout: 5000,
      encoding: "utf-8",
    }).trim();

    if (result && result !== "" && result !== "0") {
      return parseInt(result, 10);
    }
  } catch {
    // JXA failed (likely no Screen Recording permission)
  }

  return null;
}

/**
 * Get the VeloTerm window bounds (x, y, width, height) via CGWindowList.
 * Returns null if window not found.
 */
export function getWindowBounds(pid: number): { x: number; y: number; w: number; h: number } | null {
  try {
    const script = `
      ObjC.import('CoreGraphics');
      var rawList = $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0);
      var count = rawList.count;
      for (var i = 0; i < count; i++) {
        var w = ObjC.deepUnwrap(rawList.objectAtIndex(i));
        if (w.kCGWindowOwnerPID === ${pid} && w.kCGWindowBounds && w.kCGWindowBounds.Height > 100) {
          var b = w.kCGWindowBounds;
          JSON.stringify({x: b.X, y: b.Y, w: b.Width, h: b.Height});
          break;
        }
      }
    `;
    const result = execSync(`osascript -l JavaScript -e '${script.replace(/'/g, "'\\''")}'`, {
      timeout: 5000,
      encoding: "utf-8",
    }).trim();
    if (result) {
      return JSON.parse(result);
    }
  } catch {
    // JXA failed
  }
  return null;
}

/** Track when we last focused the window to avoid disruptive re-clicks */
let lastFocusTime = 0;
const FOCUS_DEBOUNCE_MS = 60000; // Don't re-focus within 60 seconds

/**
 * Focus the VeloTerm window using NSRunningApplication.activateWithOptions
 * via JXA. This brings the window to front without clicking on it,
 * preserving any overlay state (file browser selection, etc.).
 * Debounced to avoid unnecessary activations.
 */
export function focusWindow(pid: number): void {
  const now = Date.now();
  if (now - lastFocusTime < FOCUS_DEBOUNCE_MS) {
    return;
  }

  // Click the VeloTerm window title bar text area to activate without
  // interacting with any UI content. Title bar is at y+28 (below traffic lights).
  const bounds = getWindowBounds(pid);
  if (bounds) {
    try {
      // Click right-of-center in the title bar (avoids traffic lights on left
      // and toolbar icons on right). Title bar ~28px from window top.
      const cx = Math.round(bounds.x + bounds.w * 0.6);
      const cy = Math.round(bounds.y + 14);
      execFileSync("cliclick", [`c:${cx},${cy}`], { timeout: 5000 });
      execSync("sleep 0.25", { timeout: 2000 });
      lastFocusTime = Date.now();
      return;
    } catch {
      // cliclick failed
    }
  }

  // Fallback: `open` the .app bundle
  try {
    execSync(`open "${APP_PATH}"`, { timeout: 5000 });
    execSync("sleep 0.25", { timeout: 2000 });
  } catch {
    // App bundle not found
  }
  lastFocusTime = Date.now();
}

/**
 * Capture screenshot of the window with given ID.
 * Returns the PNG file contents as a Buffer.
 */
export async function captureWindow(windowId: number, _pid: number): Promise<Buffer> {
  // screencapture -l captures by window ID — no need to focus first.
  // Focusing would click the window and change terminal state.
  execFileSync("/usr/sbin/screencapture", ["-o", "-x", "-l", String(windowId), SCREENSHOT_PATH], {
    timeout: 10000,
  });

  return readFileSync(SCREENSHOT_PATH);
}

/**
 * Get image dimensions from a PNG buffer (reads IHDR chunk).
 */
export function pngDimensions(buf: Buffer): { width: number; height: number } {
  // PNG IHDR: bytes 16-19 = width, 20-23 = height (big-endian)
  if (buf.length < 24 || buf[0] !== 0x89 || buf[1] !== 0x50) {
    return { width: 0, height: 0 };
  }
  const width = buf.readUInt32BE(16);
  const height = buf.readUInt32BE(20);
  return { width, height };
}

/**
 * Type text into VeloTerm via cliclick (CGEvent-level injection).
 * Optionally presses Enter afterwards in the same cliclick invocation
 * to avoid re-focusing (which would click and move the cursor).
 */
export function typeText(pid: number, text: string, pressEnter = false): void {
  focusWindow(pid);
  try {
    // -w 150 adds 150ms delay between each event for reliable delivery
    // (30ms dropped chars, 80ms still dropped on longer strings; 150ms is safe)
    const args = ["-w", "150", `t:${text}`];
    if (pressEnter) {
      args.push("kp:return");
    }
    execFileSync("cliclick", args, { timeout: 30000 });
  } catch {
    // Fallback: AppleScript (unreliable with winit windows)
    const escaped = text.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
    execFileSync("osascript", [
      "-e", 'tell application "System Events"',
      "-e", `tell (first process whose unix id is ${pid})`,
      "-e", "delay 0.2",
      "-e", `keystroke "${escaped}"`,
      "-e", "end tell",
      "-e", "end tell",
    ], { timeout: 10000 });
    if (pressEnter) {
      execFileSync("osascript", [
        "-e", 'tell application "System Events"',
        "-e", `tell (first process whose unix id is ${pid})`,
        "-e", "key code 36",
        "-e", "end tell",
        "-e", "end tell",
      ], { timeout: 10000 });
    }
  }
}

/** Map our key names to cliclick kp: key names */
const CLICLICK_KEYS: Record<string, string> = {
  enter: "return",
  return: "return",
  tab: "tab",
  escape: "esc",
  space: "space",
  delete: "delete",
  backspace: "delete",
  up: "arrow-up",
  down: "arrow-down",
  left: "arrow-left",
  right: "arrow-right",
  home: "home",
  end: "end",
  pageup: "page-up",
  pagedown: "page-down",
};

/**
 * Press a special key by name via cliclick (CGEvent-level).
 * For modifier combos (ctrl+e), uses kd:/ku: for modifiers + kp: or t: for the key.
 * Falls back to AppleScript key code if cliclick fails.
 */
export function pressKey(pid: number, keyName: string): void {
  const lower = keyName.toLowerCase();

  // Validate key is supported
  const entry = KEY_CODES[lower];
  if (!entry) {
    throw new Error(
      `Unknown key: "${keyName}". Supported keys: ${Object.keys(KEY_CODES).join(", ")}`
    );
  }

  focusWindow(pid);

  try {
    // Check for modifier combo (e.g., "ctrl+e", "ctrl+c")
    const parts = lower.split("+");
    if (parts.length === 2) {
      const mod = parts[0]; // "ctrl"
      const key = parts[1]; // "e", "c", etc.
      // Map modifier name to cliclick modifier
      const cliMod = mod === "ctrl" ? "ctrl" : mod === "cmd" ? "cmd" : mod === "alt" ? "alt" : mod;
      // Use kd (key down modifier), then type the character, then ku (key up modifier)
      execFileSync("cliclick", [`kd:${cliMod}`, `t:${key}`, `ku:${cliMod}`], { timeout: 10000 });
    } else {
      // Simple key press — map to cliclick name
      const ckKey = CLICLICK_KEYS[lower];
      if (ckKey) {
        execFileSync("cliclick", [`kp:${ckKey}`], { timeout: 10000 });
      } else {
        // Single character key
        execFileSync("cliclick", [`t:${lower}`], { timeout: 10000 });
      }
    }
  } catch {
    // Fallback: AppleScript key code
    const modPart = entry.modifiers
      ? ` using {${entry.modifiers.join(", ")}}`
      : "";
    execFileSync("osascript", [
      "-e", 'tell application "System Events"',
      "-e", `tell (first process whose unix id is ${pid})`,
      "-e", "delay 0.2",
      "-e", `key code ${entry.code}${modPart}`,
      "-e", "end tell",
      "-e", "end tell",
    ], { timeout: 10000 });
  }
}
