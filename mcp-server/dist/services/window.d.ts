/**
 * Find window ID for a given PID using JXA (JavaScript for Automation).
 * Uses CGWindowListCopyWindowInfo to match PID → window number.
 * Falls back to GetWindowID (Homebrew) if JXA fails.
 */
export declare function findWindowByPid(pid: number): number | null;
/**
 * Focus the VeloTerm window by PID using multiple strategies.
 * Strategy 1: `open` the .app bundle (most reliable for macOS activation)
 * Strategy 2: System Events frontmost (fallback)
 */
export declare function focusWindow(pid: number): void;
/**
 * Capture screenshot of the window with given ID.
 * Returns the PNG file contents as a Buffer.
 */
export declare function captureWindow(windowId: number, pid: number): Promise<Buffer>;
/**
 * Get image dimensions from a PNG buffer (reads IHDR chunk).
 */
export declare function pngDimensions(buf: Buffer): {
    width: number;
    height: number;
};
/**
 * Type text into VeloTerm via osascript keystroke injection.
 * Uses execFileSync with argument array to avoid shell injection.
 */
export declare function typeText(pid: number, text: string): void;
/**
 * Press a special key by name.
 * Uses execFileSync with argument array to avoid shell injection.
 */
export declare function pressKey(pid: number, keyName: string): void;
