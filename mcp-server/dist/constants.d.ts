export declare const PROJECT_DIR: string;
export declare const APP_PATH: string;
export declare const BINARY_PATH: string;
export declare const INFO_PLIST_PATH: string;
export declare const SCREENSHOT_PATH: string;
export declare const BUILD_LOG_PATH = "/tmp/veloterm-mcp-build.log";
export declare const RUNTIME_LOG_PATH = "/tmp/veloterm-mcp-runtime.log";
export declare const LAUNCH_TIMEOUT_MS = 30000;
export declare const LAUNCH_POLL_INTERVAL_MS = 1000;
export declare const FOCUS_DELAY_MS = 300;
/** macOS key codes for special keys */
export declare const KEY_CODES: Record<string, {
    code: number;
    modifiers?: string[];
}>;
export declare const SUPPORTED_KEYS: string[];
