import { execSync, spawn } from "node:child_process";
import { mkdirSync, copyFileSync, writeFileSync, chmodSync, existsSync, } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { PROJECT_DIR, APP_PATH, BINARY_PATH, INFO_PLIST_PATH, BUILD_LOG_PATH, RUNTIME_LOG_PATH, LAUNCH_TIMEOUT_MS, LAUNCH_POLL_INTERVAL_MS, } from "../constants.js";
import { findWindowByPid } from "./window.js";
export const state = {
    pid: null,
    windowId: null,
    launchedAt: null,
};
/** Check if a process is alive. */
export function isAlive(pid) {
    try {
        process.kill(pid, 0); // Signal 0 = check existence
        return true;
    }
    catch {
        return false;
    }
}
/** Build VeloTerm via cargo. */
export function buildVeloTerm(release) {
    const args = release ? ["build", "--release"] : ["build"];
    execSync(`cargo ${args.join(" ")} > "${BUILD_LOG_PATH}" 2>&1`, {
        cwd: PROJECT_DIR,
        timeout: 300_000, // 5 min for build
        env: {
            ...process.env,
            PATH: `${join(homedir(), ".cargo/bin")}:${process.env.PATH}`,
        },
    });
    // For release builds, binary is in a different location
    if (release) {
        const relBinary = join(PROJECT_DIR, "target/release/veloterm");
        if (!existsSync(relBinary)) {
            throw new Error("Release build succeeded but binary not found");
        }
    }
}
/** Create .app bundle with Info.plist and wrapper script. */
export function createAppBundle(release) {
    const macosDir = join(APP_PATH, "Contents/MacOS");
    mkdirSync(macosDir, { recursive: true });
    const srcBinary = release
        ? join(PROJECT_DIR, "target/release/veloterm")
        : BINARY_PATH;
    copyFileSync(srcBinary, join(macosDir, "veloterm-bin"));
    const contentsDir = join(APP_PATH, "Contents");
    copyFileSync(INFO_PLIST_PATH, join(contentsDir, "Info.plist"));
    // Create wrapper script with env vars
    const wrapper = `#!/bin/bash
DIR="$(dirname "$0")"
export RUST_LOG=info
export VELOTERM_PROJECT_DIR="${PROJECT_DIR}"
cd "${PROJECT_DIR}"
exec "$DIR/veloterm-bin" >> "${RUNTIME_LOG_PATH}" 2>&1
`;
    const wrapperPath = join(macosDir, "veloterm");
    writeFileSync(wrapperPath, wrapper);
    chmodSync(wrapperPath, 0o755);
}
/** Launch VeloTerm .app bundle via macOS `open` command. */
export function launchApp() {
    spawn("open", [APP_PATH], {
        detached: true,
        stdio: "ignore",
    }).unref();
}
/** Find VeloTerm PID via pgrep. */
export function findPid() {
    try {
        const result = execSync("pgrep -x veloterm-bin", {
            timeout: 5000,
            encoding: "utf-8",
        }).trim();
        const pids = result.split("\n").filter(Boolean);
        if (pids.length > 0) {
            return parseInt(pids[0], 10);
        }
    }
    catch {
        // pgrep returns non-zero when no match
    }
    return null;
}
/**
 * Poll for VeloTerm readiness: PID + window ID.
 * Much better than `sleep 5` — adapts to actual startup time.
 */
export async function waitForReady() {
    const start = Date.now();
    while (Date.now() - start < LAUNCH_TIMEOUT_MS) {
        const pid = findPid();
        if (pid) {
            const windowId = findWindowByPid(pid);
            if (windowId) {
                return { pid, windowId, elapsedMs: Date.now() - start };
            }
        }
        await new Promise((r) => setTimeout(r, LAUNCH_POLL_INTERVAL_MS));
    }
    throw new Error(`VeloTerm window did not appear within ${LAUNCH_TIMEOUT_MS / 1000}s. ` +
        `Check ${RUNTIME_LOG_PATH} for errors.`);
}
/** Kill VeloTerm and clear state. */
export function killVeloTerm() {
    if (state.pid && isAlive(state.pid)) {
        try {
            process.kill(state.pid, "SIGTERM");
        }
        catch {
            // Already dead
        }
        state.pid = null;
        state.windowId = null;
        state.launchedAt = null;
        return true;
    }
    // Also try pgrep in case state is stale
    const pid = findPid();
    if (pid) {
        try {
            process.kill(pid, "SIGTERM");
        }
        catch {
            // Already dead
        }
    }
    state.pid = null;
    state.windowId = null;
    state.launchedAt = null;
    return pid !== null;
}
/** Ensure state is consistent — re-discover if PID died. */
export function refreshState() {
    if (state.pid && !isAlive(state.pid)) {
        state.pid = null;
        state.windowId = null;
        state.launchedAt = null;
    }
}
//# sourceMappingURL=process.js.map