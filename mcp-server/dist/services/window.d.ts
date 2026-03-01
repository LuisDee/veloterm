/**
 * Find window ID for a given PID using JXA (JavaScript for Automation).
 * Uses CGWindowListCopyWindowInfo to match PID → window number.
 * Falls back to GetWindowID (Homebrew) if JXA fails.
 */
export declare function findWindowByPid(pid: number): number | null;
/**
 * Get the VeloTerm window bounds (x, y, width, height) via CGWindowList.
 * Returns null if window not found.
 */
export declare function getWindowBounds(pid: number): {
    x: number;
    y: number;
    w: number;
    h: number;
} | null;
/**
 * Focus the VeloTerm window using NSRunningApplication.activateWithOptions
 * via JXA. This brings the window to front without clicking on it,
 * preserving any overlay state (file browser selection, etc.).
 * Debounced to avoid unnecessary activations.
 */
export declare function focusWindow(pid: number): void;
/**
 * Capture screenshot of the window with given ID.
 * Returns the PNG file contents as a Buffer.
 */
export declare function captureWindow(windowId: number, _pid: number): Promise<Buffer>;
/**
 * Get image dimensions from a PNG buffer (reads IHDR chunk).
 */
export declare function pngDimensions(buf: Buffer): {
    width: number;
    height: number;
};
/**
 * Type text into VeloTerm via clipboard paste (Cmd+V).
 * This avoids Karabiner-Elements intercepting individual keystrokes (e.g., 'm').
 * The flow: pbcopy → focus window → Cmd+V → optional Enter.
 * Falls back to cliclick t: if clipboard paste fails.
 */
export declare function typeText(pid: number, text: string, pressEnter?: boolean): void;
/**
 * Press a special key by name via cliclick (CGEvent-level).
 * For modifier combos (ctrl+e), uses kd:/ku: for modifiers + kp: or t: for the key.
 * Falls back to AppleScript key code if cliclick fails.
 */
export declare function pressKey(pid: number, keyName: string): void;
