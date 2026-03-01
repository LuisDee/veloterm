import { homedir } from "node:os";
import { join } from "node:path";
export const PROJECT_DIR = process.env.VELOTERM_PROJECT_DIR ?? join(homedir(), "work/terminal-em");
export const APP_PATH = join(PROJECT_DIR, "target/VeloTerm.app");
export const BINARY_PATH = join(PROJECT_DIR, "target/debug/veloterm");
export const INFO_PLIST_PATH = join(PROJECT_DIR, "resources/macos/Info.plist");
export const SCREENSHOT_PATH = join(PROJECT_DIR, "veloterm-latest.png");
export const BUILD_LOG_PATH = "/tmp/veloterm-mcp-build.log";
export const RUNTIME_LOG_PATH = "/tmp/veloterm-mcp-runtime.log";
export const LAUNCH_TIMEOUT_MS = 30_000;
export const LAUNCH_POLL_INTERVAL_MS = 1_000;
export const FOCUS_DELAY_MS = 300;
/** macOS key codes for special keys */
export const KEY_CODES = {
    enter: { code: 36 },
    return: { code: 36 },
    tab: { code: 48 },
    escape: { code: 53 },
    space: { code: 49 },
    delete: { code: 51 },
    backspace: { code: 51 },
    up: { code: 126 },
    down: { code: 125 },
    left: { code: 123 },
    right: { code: 124 },
    home: { code: 115 },
    end: { code: 119 },
    pageup: { code: 116 },
    pagedown: { code: 121 },
    "ctrl+c": { code: 8, modifiers: ["control down"] },
    "ctrl+d": { code: 2, modifiers: ["control down"] },
    "ctrl+z": { code: 6, modifiers: ["control down"] },
    "ctrl+l": { code: 37, modifiers: ["control down"] },
    "ctrl+a": { code: 0, modifiers: ["control down"] },
    "ctrl+e": { code: 14, modifiers: ["control down"] },
    "ctrl+r": { code: 15, modifiers: ["control down"] },
    "ctrl+w": { code: 13, modifiers: ["control down"] },
    "ctrl+u": { code: 32, modifiers: ["control down"] },
    "ctrl+k": { code: 40, modifiers: ["control down"] },
    "ctrl+g": { code: 5, modifiers: ["control down"] },
    "ctrl+b": { code: 11, modifiers: ["control down"] },
    "ctrl+t": { code: 17, modifiers: ["control down"] },
    "ctrl+n": { code: 45, modifiers: ["control down"] },
};
export const SUPPORTED_KEYS = Object.keys(KEY_CODES);
//# sourceMappingURL=constants.js.map