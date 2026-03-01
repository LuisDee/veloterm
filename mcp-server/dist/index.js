#!/usr/bin/env node
/**
 * VeloTerm MCP Server
 *
 * Enables Claude Code to launch, screenshot, type into, and interact with
 * a running VeloTerm instance. Separates build/launch from capture so
 * screenshots are sub-second after initial launch.
 *
 * Transport: stdio (local integration, single-user)
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { SUPPORTED_KEYS } from "./constants.js";
import { state, refreshState, isAlive, buildVeloTerm, createAppBundle, launchApp, waitForReady, killVeloTerm, } from "./services/process.js";
import { captureWindow, pngDimensions, typeText, pressKey, } from "./services/window.js";
// ── Server setup ─────────────────────────────────────────────────────
const server = new McpServer({
    name: "veloterm-mcp-server",
    version: "1.0.0",
});
// ── veloterm_launch ──────────────────────────────────────────────────
const LaunchInputSchema = z
    .object({
    rebuild: z
        .boolean()
        .default(true)
        .describe("Whether to run `cargo build` before launching (default: true)"),
    release: z
        .boolean()
        .default(false)
        .describe("Use release build instead of debug (default: false)"),
})
    .strict();
server.registerTool("veloterm_launch", {
    title: "Launch VeloTerm",
    description: `Build and launch a VeloTerm instance, or return the existing one if already running.

Creates the .app bundle required for proper Retina scaling on macOS, launches via \`open\`,
and polls for window readiness (no fixed sleep). State persists across tool calls.

Args:
  - rebuild (boolean): Run cargo build first (default: true)
  - release (boolean): Use release build (default: false)

Returns:
  Status text with PID, window ID, and launch time.

Errors:
  - Build failure: check /tmp/veloterm-mcp-build.log
  - Window timeout: check /tmp/veloterm-mcp-runtime.log`,
    inputSchema: LaunchInputSchema,
    annotations: {
        readOnlyHint: false,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
    },
}, async (params) => {
    refreshState();
    // If already running, return current state
    if (state.pid && isAlive(state.pid) && state.windowId) {
        const uptimeSec = state.launchedAt
            ? Math.round((Date.now() - state.launchedAt) / 1000)
            : 0;
        return {
            content: [
                {
                    type: "text",
                    text: `VeloTerm already running (PID ${state.pid}, Window ${state.windowId}, uptime ${uptimeSec}s)`,
                },
            ],
        };
    }
    try {
        // Build
        if (params.rebuild) {
            buildVeloTerm(params.release);
        }
        // Create .app bundle
        createAppBundle(params.release);
        // Launch
        launchApp();
        // Poll for readiness
        const { pid, windowId, elapsedMs } = await waitForReady();
        state.pid = pid;
        state.windowId = windowId;
        state.launchedAt = Date.now();
        return {
            content: [
                {
                    type: "text",
                    text: `VeloTerm launched (PID ${pid}, Window ${windowId}, ready in ${Math.round(elapsedMs / 1000)}s)`,
                },
            ],
        };
    }
    catch (error) {
        return {
            content: [
                {
                    type: "text",
                    text: `Error launching VeloTerm: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
            isError: true,
        };
    }
});
// ── veloterm_screenshot ──────────────────────────────────────────────
server.registerTool("veloterm_screenshot", {
    title: "Screenshot VeloTerm",
    description: `Capture the current VeloTerm window as a PNG screenshot.

Returns the image inline (base64) so Claude can see it directly. Sub-second execution
when VeloTerm is already running. Call veloterm_launch first if not running.

Returns:
  - Inline PNG image
  - Text with file path and dimensions

Errors:
  - "VeloTerm not running" if no instance is active`,
    inputSchema: z.object({}).strict(),
    annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
    },
}, async () => {
    refreshState();
    if (!state.pid || !state.windowId || !isAlive(state.pid)) {
        return {
            content: [
                {
                    type: "text",
                    text: "VeloTerm not running. Call veloterm_launch first.",
                },
            ],
            isError: true,
        };
    }
    try {
        const pngBuf = await captureWindow(state.windowId, state.pid);
        const { width, height } = pngDimensions(pngBuf);
        const base64 = pngBuf.toString("base64");
        return {
            content: [
                {
                    type: "image",
                    data: base64,
                    mimeType: "image/png",
                },
                {
                    type: "text",
                    text: `Screenshot captured (${width}x${height} pixels)`,
                },
            ],
        };
    }
    catch (error) {
        return {
            content: [
                {
                    type: "text",
                    text: `Error capturing screenshot: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
            isError: true,
        };
    }
});
// ── veloterm_type ────────────────────────────────────────────────────
const TypeInputSchema = z
    .object({
    text: z.string().min(1).describe("Text to type into VeloTerm"),
    press_enter: z
        .boolean()
        .default(false)
        .describe("Press Enter after typing the text (default: false)"),
})
    .strict();
server.registerTool("veloterm_type", {
    title: "Type into VeloTerm",
    description: `Type text into the VeloTerm window via macOS keystroke injection.

Focuses the VeloTerm window, types the given text, and optionally presses Enter.
Useful for running commands in the terminal.

Args:
  - text (string): Text to type
  - press_enter (boolean): Press Enter after typing (default: false)

Example:
  - Type a command: text="echo hello", press_enter=true
  - Type partial input: text="git com", press_enter=false`,
    inputSchema: TypeInputSchema,
    annotations: {
        readOnlyHint: false,
        destructiveHint: false,
        idempotentHint: false,
        openWorldHint: false,
    },
}, async (params) => {
    refreshState();
    if (!state.pid || !isAlive(state.pid)) {
        return {
            content: [
                {
                    type: "text",
                    text: "VeloTerm not running. Call veloterm_launch first.",
                },
            ],
            isError: true,
        };
    }
    try {
        typeText(state.pid, params.text, params.press_enter);
        return {
            content: [
                {
                    type: "text",
                    text: `Typed: "${params.text}"${params.press_enter ? " + Enter" : ""}`,
                },
            ],
        };
    }
    catch (error) {
        return {
            content: [
                {
                    type: "text",
                    text: `Error typing: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
            isError: true,
        };
    }
});
// ── veloterm_key ─────────────────────────────────────────────────────
const KeyInputSchema = z
    .object({
    key: z
        .enum(SUPPORTED_KEYS)
        .describe("Key to press"),
})
    .strict();
server.registerTool("veloterm_key", {
    title: "Press Key in VeloTerm",
    description: `Press a special key or key combination in VeloTerm.

Focuses the window and sends the key via macOS System Events.

Args:
  - key (string): Key name. Supported: ${SUPPORTED_KEYS.join(", ")}

Examples:
  - Press Enter: key="enter"
  - Press Ctrl+C: key="ctrl+c"
  - Press arrow down: key="down"`,
    inputSchema: KeyInputSchema,
    annotations: {
        readOnlyHint: false,
        destructiveHint: false,
        idempotentHint: false,
        openWorldHint: false,
    },
}, async (params) => {
    refreshState();
    if (!state.pid || !isAlive(state.pid)) {
        return {
            content: [
                {
                    type: "text",
                    text: "VeloTerm not running. Call veloterm_launch first.",
                },
            ],
            isError: true,
        };
    }
    try {
        pressKey(state.pid, params.key);
        return {
            content: [
                {
                    type: "text",
                    text: `Pressed: ${params.key}`,
                },
            ],
        };
    }
    catch (error) {
        return {
            content: [
                {
                    type: "text",
                    text: `Error pressing key: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
            isError: true,
        };
    }
});
// ── veloterm_kill ────────────────────────────────────────────────────
server.registerTool("veloterm_kill", {
    title: "Kill VeloTerm",
    description: `Kill the running VeloTerm instance and clear all state.

Sends SIGTERM to the process. Safe to call even if not running.

Returns:
  Confirmation of kill or "not running" status.`,
    inputSchema: z.object({}).strict(),
    annotations: {
        readOnlyHint: false,
        destructiveHint: true,
        idempotentHint: true,
        openWorldHint: false,
    },
}, async () => {
    const killed = killVeloTerm();
    return {
        content: [
            {
                type: "text",
                text: killed
                    ? "VeloTerm killed and state cleared."
                    : "VeloTerm was not running.",
            },
        ],
    };
});
// ── Graceful cleanup on exit ─────────────────────────────────────────
function cleanup() {
    if (state.pid && isAlive(state.pid)) {
        try {
            process.kill(state.pid, "SIGTERM");
        }
        catch {
            // Best effort
        }
    }
}
process.on("SIGINT", () => {
    cleanup();
    process.exit(0);
});
process.on("SIGTERM", () => {
    cleanup();
    process.exit(0);
});
// ── Start server ─────────────────────────────────────────────────────
async function main() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("VeloTerm MCP server running via stdio");
}
main().catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
});
//# sourceMappingURL=index.js.map