# Cross-Cutting Concerns

> This file is **append-only**. New versions are added below existing ones.
> Never modify a published version — add a new version section instead.
> Each version is tagged to the wave where it was introduced.

---

## v1 — Initial (Wave 1)

### Structured Logging
- `log` crate with `env_logger` backend; JSON structured output optional via feature flag
- Log levels: error, warn, info, debug, trace. Default level: `info`
- Every log message includes module path (automatic via `log` macros)
- GPU, PTY, config, and input errors logged with actionable context
- Applies to: ALL
- Source: Cross-cutting catalog (always evaluate)

### Error Handling Convention
- `thiserror` for typed error enums per module (`GpuError`, `PtyError`, `ConfigError`)
- `anyhow` at the application boundary (main.rs) for top-level error reporting
- Error messages follow: what went wrong + why it matters + what user can do
- GPU init failures, font loading problems, config parse errors surface visibly on startup
- No panics in library code; `Result` propagation throughout
- Applies to: ALL
- Source: Architecture research (accepted pattern)

### Configuration Management
- Single config file: `~/.config/veloterm/veloterm.toml`
- Parsed with `toml` + `serde` with strict deserialization (unknown keys are errors)
- Secrets: none (desktop app, no credentials)
- Defaults: opinionated and complete — app works without any config file
- Hot-reload: `notify` crate watches config file; invalid changes keep previous state
- Applies to: ALL tracks that read configuration
- Source: Cross-cutting catalog (always evaluate)

### Graceful Shutdown
- On window close or SIGTERM: save session state → SIGHUP child shells → drain PTY channels → release GPU → exit
- Shutdown timeout: 5 seconds before forced exit
- All pane PTY sessions cleaned up; no orphan shell processes
- Applies to: ALL tracks that manage resources or child processes
- Source: Architecture research (accepted pattern)

### Input Validation
- Config file: strict TOML parsing with `serde` — unknown fields rejected, type mismatches reported with field path
- Escape sequences: validated by alacritty_terminal (trusted parser)
- User keyboard input: validated at winit event boundary before translation
- Applies to: Configuration track, input handling
- Source: Cross-cutting catalog (always evaluate)

### Testing
- TDD approach: write failing tests first, then implement
- `cargo test --lib` for all unit tests
- `cargo clippy --all-targets` must pass with no warnings
- `cargo fmt --check` must pass
- Visual features validated via screenshot before user verification
- Applies to: ALL
- Source: Conductor workflow (established practice)

### Platform Abstraction
- All platform-specific code isolated behind trait boundaries or `#[cfg(target_os)]` blocks
- macOS: Metal GPU backend, Cocoa windowing, .app bundle
- Linux: Vulkan/OpenGL GPU backend, X11 windowing, RPM/AppImage
- Keybindings: Cmd on macOS, Ctrl on Linux (configurable via TOML)
- Applies to: ALL tracks with platform-specific behavior
- Source: Product requirements (cross-platform from day one)

### Performance Budget
- Input-to-screen latency: <10ms
- Startup time: <100ms
- Memory per terminal instance: <10MB
- Frame rendering: 2 draw calls per frame (background + foreground)
- Damage tracking: only update vertex buffer for dirty cells
- Applies to: ALL tracks that affect rendering or startup path
- Source: Product requirements (performance targets)

---

### Not Applicable (with justification)

| Concern | Why N/A |
|---------|---------|
| Health Checks (Liveness/Readiness) | Desktop app, not a server — no HTTP endpoints |
| Database Connection Pooling | No database — all state is in-memory or on filesystem |
| Timeout Policies | No external service calls — all IO is local (PTY, filesystem) |
| Distributed Tracing | Single process — structured logging with module paths is sufficient |
| Service Discovery | Single process — no services to discover |
| API Versioning | No APIs — desktop application |
| Event Schema Versioning | No inter-service events |
| Idempotency for Message Handlers | No message queues |
| Authentication + Authorization | Desktop app — OS-level user authentication only |
| CORS Policy | No web endpoints |
| Session Management (web) | Desktop app — session = running process |
| Backup and Recovery | User's scrollback is ephemeral; session persistence (Phase 4) uses local files |
| Data Retention Policy | No persistent user data beyond config file |
| PII Handling | No PII collected — terminal content is user's own data |
| Migration Strategy | No database schema to migrate |
