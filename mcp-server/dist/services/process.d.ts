/** Module-level state — persists across tool calls (stdio = single process). */
export interface VeloTermState {
    pid: number | null;
    windowId: number | null;
    launchedAt: number | null;
}
export declare const state: VeloTermState;
/** Check if a process is alive. */
export declare function isAlive(pid: number): boolean;
/** Build VeloTerm via cargo. */
export declare function buildVeloTerm(release: boolean): void;
/** Create .app bundle with Info.plist and wrapper script. */
export declare function createAppBundle(release: boolean): void;
/** Launch VeloTerm .app bundle via macOS `open` command. */
export declare function launchApp(): void;
/** Find VeloTerm PID via pgrep. */
export declare function findPid(): number | null;
/**
 * Poll for VeloTerm readiness: PID + window ID.
 * Much better than `sleep 5` — adapts to actual startup time.
 */
export declare function waitForReady(): Promise<{
    pid: number;
    windowId: number;
    elapsedMs: number;
}>;
/** Kill VeloTerm and clear state. */
export declare function killVeloTerm(): boolean;
/** Ensure state is consistent — re-discover if PID died. */
export declare function refreshState(): void;
