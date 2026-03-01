import { execSync, execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { FOCUS_DELAY_MS, KEY_CODES, SCREENSHOT_PATH } from "../constants.js";

/**
 * Find window ID for a given PID using JXA (JavaScript for Automation).
 * Uses CGWindowListCopyWindowInfo to match PID → window number.
 * Falls back to GetWindowID (Homebrew) if JXA fails.
 */
export function findWindowByPid(pid: number): number | null {
  // Primary: JXA via osascript
  try {
    const script = `
      ObjC.import('CoreGraphics');
      var list = ObjC.deepUnwrap(
        $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)
      );
      var win = list.find(function(w) {
        return w.kCGWindowOwnerPID === ${pid} && w.kCGWindowLayer === 0;
      });
      if (win) { win.kCGWindowNumber; } else { ''; }
    `;
    const result = execSync(`osascript -l JavaScript -e '${script.replace(/'/g, "'\\''")}'`, {
      timeout: 5000,
      encoding: "utf-8",
    }).trim();

    if (result && result !== "" && result !== "0") {
      return parseInt(result, 10);
    }
  } catch {
    // JXA failed, try fallback
  }

  // Fallback: GetWindowID (Homebrew)
  try {
    const result = execSync('GetWindowID "VeloTerm" 2>/dev/null || echo ""', {
      timeout: 5000,
      encoding: "utf-8",
    }).trim();

    if (result && result !== "" && result !== "0") {
      return parseInt(result, 10);
    }
  } catch {
    // GetWindowID not installed or failed
  }

  return null;
}

/**
 * Focus the VeloTerm window by PID using System Events.
 */
export function focusWindow(pid: number): void {
  try {
    execSync(
      `osascript -e 'tell application "System Events" to set frontmost of (first process whose unix id is ${pid}) to true'`,
      { timeout: 5000 }
    );
  } catch {
    // Best effort — window may already be focused
  }
}

/**
 * Capture screenshot of the window with given ID.
 * Returns the PNG file contents as a Buffer.
 */
export async function captureWindow(windowId: number, pid: number): Promise<Buffer> {
  focusWindow(pid);

  // Brief delay to ensure window is in front
  await new Promise((r) => setTimeout(r, FOCUS_DELAY_MS));

  execFileSync("screencapture", ["-o", "-x", "-l", String(windowId), SCREENSHOT_PATH], {
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
 * Type text into VeloTerm via osascript keystroke injection.
 * Uses execFileSync with argument array to avoid shell injection.
 */
export function typeText(pid: number, text: string): void {
  focusWindow(pid);
  // Escape backslashes and double quotes for AppleScript string literal
  const escaped = text.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  execFileSync("osascript", [
    "-e", 'tell application "System Events"',
    "-e", "delay 0.2",
    "-e", `keystroke "${escaped}"`,
    "-e", "end tell",
  ], { timeout: 10000 });
}

/**
 * Press a special key by name.
 * Uses execFileSync with argument array to avoid shell injection.
 */
export function pressKey(pid: number, keyName: string): void {
  const entry = KEY_CODES[keyName.toLowerCase()];
  if (!entry) {
    throw new Error(
      `Unknown key: "${keyName}". Supported keys: ${Object.keys(KEY_CODES).join(", ")}`
    );
  }

  focusWindow(pid);

  const modPart = entry.modifiers
    ? ` using {${entry.modifiers.join(", ")}}`
    : "";

  execFileSync("osascript", [
    "-e", 'tell application "System Events"',
    "-e", "delay 0.2",
    "-e", `key code ${entry.code}${modPart}`,
    "-e", "end tell",
  ], { timeout: 10000 });
}
