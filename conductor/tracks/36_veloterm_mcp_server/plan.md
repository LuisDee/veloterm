# Track 36: VeloTerm MCP Server

## Problem

The current `take-screenshot.sh` conflates building, launching, capturing, and killing into one atomic operation. Every screenshot costs 10-20 seconds and destroys the running instance. No interaction capability. No persistent instance.

## Architecture

Three-layer design:

```
┌─────────────────────────────────────┐
│  MCP Server (JSON-RPC over stdio)   │  ← Claude Code talks to this
│  Tools: launch, screenshot, type,   │
│         key, kill                    │
├─────────────────────────────────────┤
│  VeloTerm Daemon Manager            │  ← Launch once, reuse across calls
│  Tracks PID, window ID, readiness   │
│  Module-level state (stdio = 1 proc)│
├─────────────────────────────────────┤
│  macOS Capture Layer                │  ← screencapture -l, osascript
│  JXA for PID-based window discovery │
│  System Events for keystroke inject │
└─────────────────────────────────────┘
```

## Stack

- **Language**: TypeScript (recommended by MCP best practices for SDK quality + type safety)
- **SDK**: `@modelcontextprotocol/sdk` v1.x (stable; v2 pre-alpha not ready)
- **Transport**: stdio (local integration, single-user, subprocess of Claude Code)
- **Validation**: Zod schemas with `.strict()` for all tool inputs
- **API pattern**: `server.registerTool()` (modern API, NOT deprecated `server.tool()`)

## Machine Constraints

- Swift CLI broken (SDK version mismatch) — use JXA via `osascript -l JavaScript` instead
- Python has no Quartz module — don't rely on it
- `GetWindowID` from Homebrew is available as fallback
- Cargo already on PATH: `/Users/luisdeburnay/.cargo/bin/cargo`
- Display is Retina 2x (Apple M3 Pro)
- `.app` bundle required for correct Retina scaling (winit scale_factor)

## Project Structure

```
mcp-server/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts          # McpServer init + stdio transport + tool registration
│   ├── services/
│   │   ├── process.ts    # App build, launch, PID tracking, readiness polling
│   │   └── window.ts     # JXA window discovery, screencapture, osascript keystroke
│   └── constants.ts      # Paths, timeouts, key code map
└── dist/                 # Compiled JS (entry: dist/index.js)
```

## Tools

### `veloterm_launch`
Build VeloTerm, create .app bundle, launch, poll for readiness, cache PID + window ID.
- Input: `rebuild` (bool, default true), `release` (bool, default false)
- Idempotent: returns existing instance if already running
- Readiness: poll via JXA CGWindowList for PID match (not `sleep 5`)
- Annotations: `readOnlyHint: false, destructiveHint: false, idempotentHint: true`

### `veloterm_screenshot`
Capture the already-running VeloTerm window. Sub-second execution.
- Input: none required
- Focus window by PID via System Events, then `screencapture -l <windowId> -o -x`
- Return: ImageContent (base64 PNG inline) + TextContent with dimensions
- Annotations: `readOnlyHint: true, destructiveHint: false, idempotentHint: true`

### `veloterm_type`
Type text into VeloTerm via osascript keystroke injection.
- Input: `text` (string), `press_enter` (bool, default false)
- Focus window, `keystroke` text, optionally `key code 36` for Enter
- Annotations: `readOnlyHint: false, destructiveHint: false, idempotentHint: false`

### `veloterm_key`
Press special keys (Enter, Tab, Escape, arrows, Ctrl+C, etc.)
- Input: `key` (string enum of supported keys)
- Map to macOS key codes + modifier combinations
- Annotations: `readOnlyHint: false, destructiveHint: false, idempotentHint: false`

### `veloterm_kill`
Kill the running VeloTerm instance and clear state.
- Input: none
- `kill -TERM <pid>`, clear module state
- Annotations: `readOnlyHint: false, destructiveHint: true, idempotentHint: true`

## Window Discovery (JXA, not Swift)

```javascript
// osascript -l JavaScript
ObjC.import('CoreGraphics');
const list = ObjC.deepUnwrap(
  $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)
);
const win = list.find(w => w.kCGWindowOwnerPID === PID && w.kCGWindowLayer === 0);
win ? win.kCGWindowNumber : '';
```

Fallback to `GetWindowID "VeloTerm"` if JXA fails.

## Configuration

`.mcp.json` at project root:
```json
{
  "mcpServers": {
    "veloterm": {
      "command": "node",
      "args": ["./mcp-server/dist/index.js"],
      "env": {
        "VELOTERM_PROJECT_DIR": "/Users/luisdeburnay/work/terminal-em",
        "PATH": "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/Users/luisdeburnay/.cargo/bin"
      }
    }
  }
}
```

## Implementation Order

1. Scaffold project (package.json, tsconfig.json, constants.ts)
2. Implement services/window.ts (JXA discovery, screencapture, keystroke)
3. Implement services/process.ts (build, .app bundle, launch, readiness poll)
4. Implement index.ts (McpServer, all 5 tools, stdio transport)
5. Build, test with MCP Inspector
6. Add .mcp.json, verify end-to-end

## Success Criteria

1. MCP server launches VeloTerm once, takes unlimited screenshots at sub-second speed
2. Text input and key press injection work reliably via osascript
3. PID-based window discovery survives title changes and multiple instances
4. `npm run build` succeeds cleanly
5. All existing Rust tests still pass (1872)
