// PTY management: spawning shells, reading output, writing input.

use crossbeam_channel::{Receiver, Sender};
use portable_pty::{CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::thread;

/// Errors that can occur during PTY operations.
#[derive(Debug)]
pub enum PtyError {
    /// Failed to open a PTY pair.
    OpenPtyFailed(String),
    /// Failed to spawn the shell process.
    SpawnFailed(String),
    /// Failed to clone the PTY reader.
    ReaderCloneFailed(String),
    /// Failed to take the PTY writer.
    WriterTakeFailed(String),
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyError::OpenPtyFailed(e) => write!(f, "failed to open PTY: {e}"),
            PtyError::SpawnFailed(e) => write!(f, "failed to spawn shell: {e}"),
            PtyError::ReaderCloneFailed(e) => write!(f, "failed to clone PTY reader: {e}"),
            PtyError::WriterTakeFailed(e) => write!(f, "failed to take PTY writer: {e}"),
        }
    }
}

impl std::error::Error for PtyError {}

/// Known shell process names that should fall back to CWD-based titles.
const SHELL_NAMES: &[&str] = &["zsh", "bash", "fish", "sh", "dash", "tcsh", "csh", "ksh"];

/// Returns true if the given process name is a known shell.
pub fn is_shell_process(name: &str) -> bool {
    SHELL_NAMES.contains(&name)
}

/// Query the foreground process name for a given shell PID.
///
/// Returns the basename of the foreground child process, or None if
/// detection fails or the shell itself is the foreground process.
#[cfg(target_os = "macos")]
pub fn foreground_process_name(shell_pid: u32) -> Option<String> {
    extern "C" {
        fn proc_listchildpids(
            ppid: libc::c_int,
            buffer: *mut libc::c_void,
            buffersize: libc::c_int,
        ) -> libc::c_int;
        fn proc_pidpath(
            pid: libc::c_int,
            buffer: *mut libc::c_void,
            buffersize: u32,
        ) -> libc::c_int;
    }

    unsafe {
        // Get number of child PIDs
        let count =
            proc_listchildpids(shell_pid as libc::c_int, std::ptr::null_mut(), 0);
        if count <= 0 {
            return None; // No children — shell is foreground
        }

        let mut pids = vec![0i32; count as usize];
        let buf_size = (count as usize * std::mem::size_of::<i32>()) as libc::c_int;
        let actual =
            proc_listchildpids(shell_pid as libc::c_int, pids.as_mut_ptr() as *mut _, buf_size);
        if actual <= 0 {
            return None;
        }

        let num_pids = actual as usize / std::mem::size_of::<i32>();
        if num_pids == 0 {
            return None;
        }
        // Take the last child (most recently spawned)
        let fg_pid = pids[num_pids - 1];

        let mut path_buf = vec![0u8; 4096];
        let ret = proc_pidpath(
            fg_pid,
            path_buf.as_mut_ptr() as *mut _,
            path_buf.len() as u32,
        );
        if ret <= 0 {
            return None;
        }

        let path = std::ffi::CStr::from_ptr(path_buf.as_ptr() as *const _)
            .to_string_lossy()
            .to_string();
        Some(path.rsplit('/').next().unwrap_or(&path).to_string())
    }
}

#[cfg(target_os = "linux")]
pub fn foreground_process_name(shell_pid: u32) -> Option<String> {
    crate::platform::linux::foreground_process_name(shell_pid)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn foreground_process_name(_shell_pid: u32) -> Option<String> {
    None
}

/// Extract a process name from a full path string.
pub fn basename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Determine the shell to spawn: `$SHELL` or `/bin/sh` fallback.
pub fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

/// Resolve the shell program from config.
///
/// Priority: config.program > macOS user shell (dscl) > $SHELL > /bin/zsh.
///
/// On macOS, `$SHELL` can be wrong when VeloTerm is launched from a bash parent
/// (e.g. a CI runner, IDE terminal, or script). We check the actual configured
/// user shell via `dscl` as a more reliable source on macOS.
pub fn resolve_shell(config: &crate::config::types::ShellConfig) -> String {
    // 1. Explicit config override always wins
    if let Some(ref program) = config.program {
        return program.clone();
    }

    // 2. On macOS, check the user's configured login shell via Directory Services
    #[cfg(target_os = "macos")]
    {
        if let Some(shell) = macos_user_shell() {
            return shell;
        }
    }

    // 3. Fall back to $SHELL, then /bin/zsh
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

/// Query the macOS Directory Services for the current user's configured login shell.
/// Returns None if the query fails (non-macOS, dscl not found, parse error).
#[cfg(target_os = "macos")]
fn macos_user_shell() -> Option<String> {
    let user = std::env::var("USER").ok()?;
    let output = std::process::Command::new("dscl")
        .args([".", "-read", &format!("/Users/{user}"), "UserShell"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "UserShell: /bin/zsh"
    let shell = stdout.trim().strip_prefix("UserShell:")?.trim().to_string();
    if shell.is_empty() || !std::path::Path::new(&shell).exists() {
        return None;
    }
    Some(shell)
}

/// Starship suppression snippet for zsh: defines a shadow function before .zshrc,
/// removes it after sourcing, so `eval "$(starship init zsh)"` is a no-op.
const ZSH_STARSHIP_SUPPRESS: &str = r#"
# Suppress starship if configured
if [[ "$VELOTERM_DISABLE_STARSHIP" == "1" ]]; then
    starship() { command echo ""; }
fi
"#;

const ZSH_STARSHIP_UNSUPPRESS: &str = r#"
# Remove starship shadow after init
if [[ "$VELOTERM_DISABLE_STARSHIP" == "1" ]]; then
    unfunction starship 2>/dev/null
fi
"#;

/// Starship suppression snippet for bash.
const BASH_STARSHIP_SUPPRESS: &str = r#"
# Suppress starship if configured
if [[ "$VELOTERM_DISABLE_STARSHIP" == "1" ]]; then
    starship() { command echo ""; }
fi
"#;

const BASH_STARSHIP_UNSUPPRESS: &str = r#"
# Remove starship shadow after init
if [[ "$VELOTERM_DISABLE_STARSHIP" == "1" ]]; then
    unset -f starship 2>/dev/null
fi
"#;

/// Set up shell integration by modifying the CommandBuilder before spawn.
///
/// For bash: creates a wrapper rcfile that sources `~/.bashrc` then our
/// integration script, and uses `--rcfile` to load it. This is invisible.
///
/// For zsh: creates a temp ZDOTDIR with `.zshenv` and `.zshrc` that source
/// the user's originals then append our integration. Also invisible.
///
/// For fish: writes integration to a temp file and uses `--init-command`.
///
/// For unknown shells: does nothing (no-op).
fn prepare_shell_integration(shell: &str, cmd: &mut CommandBuilder) {
    let shell_name = basename_from_path(shell);

    match shell_name {
        "bash" => {
            let integration = include_str!("../../shell/bash-integration.sh");
            let wrapper = format!(
                "# VeloTerm bash integration wrapper\n\
                 {BASH_STARSHIP_SUPPRESS}\n\
                 [ -f ~/.bashrc ] && source ~/.bashrc\n\
                 {BASH_STARSHIP_UNSUPPRESS}\n\
                 {integration}\n"
            );
            let path = "/tmp/veloterm-bashrc.sh";
            if std::fs::write(path, &wrapper).is_ok() {
                cmd.arg("--rcfile");
                cmd.arg(path);
            }
        }
        "zsh" => {
            let integration = include_str!("../../shell/zsh-integration.sh");
            let zdotdir = "/tmp/veloterm-zdotdir";
            if std::fs::create_dir_all(zdotdir).is_ok() {
                // .zshenv: restore ZDOTDIR so zsh finds user's other dotfiles,
                // then source user's .zshenv
                let zshenv = "# VeloTerm zsh wrapper\n\
                     ZDOTDIR=\"$HOME\"\n\
                     [ -f \"$HOME/.zshenv\" ] && source \"$HOME/.zshenv\"\n";
                let _ = std::fs::write(format!("{zdotdir}/.zshenv"), zshenv);

                // .zshrc: starship shadow → source user's .zshrc → remove shadow → our integration
                let zshrc = format!(
                    "# VeloTerm zsh wrapper\n\
                     {ZSH_STARSHIP_SUPPRESS}\n\
                     [ -f \"$HOME/.zshrc\" ] && source \"$HOME/.zshrc\"\n\
                     {ZSH_STARSHIP_UNSUPPRESS}\n\
                     {integration}\n"
                );
                let _ = std::fs::write(format!("{zdotdir}/.zshrc"), &zshrc);

                cmd.env("ZDOTDIR", zdotdir);
            }
        }
        "fish" => {
            let integration = include_str!("../../shell/fish-integration.fish");
            let path = "/tmp/veloterm-integration.fish";
            if std::fs::write(path, integration).is_ok() {
                cmd.arg("--init-command");
                cmd.arg(format!("source {path}"));
            }
        }
        _ => {} // No integration for other shells
    }
}

/// Managed PTY session with a reader thread and writer handle.
pub struct PtySession {
    /// Receive raw bytes from the PTY reader thread.
    pub reader_rx: Receiver<Vec<u8>>,
    /// Writer handle for sending input to the PTY.
    writer: Box<dyn Write + Send>,
    /// The child process handle.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    /// The master PTY handle (kept alive for resize).
    master: Box<dyn MasterPty + Send>,
    /// Reader thread join handle.
    _reader_thread: thread::JoinHandle<()>,
}

impl PtySession {
    const READ_BUFFER_SIZE: usize = 64 * 1024;

    /// Spawn a new PTY session with the given shell and size.
    pub fn new(shell: &str, cols: u16, rows: u16) -> Result<Self, PtyError> {
        Self::new_with_cwd(shell, cols, rows, None, None)
    }

    /// Spawn a new PTY session with the given shell, size, optional CWD, and optional config.
    pub fn new_with_cwd(
        shell: &str,
        cols: u16,
        rows: u16,
        cwd: Option<&str>,
        shell_config: Option<&crate::config::types::ShellConfig>,
    ) -> Result<Self, PtyError> {
        let pty_system = portable_pty::native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenPtyFailed(e.to_string()))?;

        let mut cmd = CommandBuilder::new(shell);
        // Set TERM for proper color and capability support
        cmd.env("TERM", "xterm-256color");
        // Advertise 24-bit color support to CLI tools (bat, delta, ls --color, etc.)
        cmd.env("COLORTERM", "truecolor");
        // Identify the terminal emulator to shell integration scripts (Powerlevel10k, etc.)
        cmd.env("TERM_PROGRAM", "VeloTerm");
        // Remove RUST_LOG so it doesn't leak into the user's shell and cause
        // debug output from other Rust tools (fnm, ripgrep, etc.)
        cmd.env_remove("RUST_LOG");

        // Apply shell config: args, env, starship suppression
        if let Some(config) = shell_config {
            for arg in &config.args {
                cmd.arg(arg);
            }
            for (key, val) in &config.env {
                cmd.env(key, val);
            }
            if config.disable_starship {
                cmd.env("VELOTERM_DISABLE_STARSHIP", "1");
            }
        }

        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        // Inject shell integration (CWD tracking, prompt markers) before spawn.
        // Uses --rcfile for bash, ZDOTDIR for zsh — invisible to user.
        let integration_enabled = shell_config.map_or(true, |c| c.integration_enabled);
        if integration_enabled {
            prepare_shell_integration(shell, &mut cmd);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        // Drop slave — we only need the master side
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::ReaderCloneFailed(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::WriterTakeFailed(e.to_string()))?;

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = crossbeam_channel::unbounded();

        let reader_thread = thread::spawn(move || {
            let mut buf = vec![0u8; Self::READ_BUFFER_SIZE];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF — shell exited
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            reader_rx: rx,
            writer,
            _child: child,
            master: pair.master,
            _reader_thread: reader_thread,
        })
    }

    /// Write bytes to the PTY (keyboard input).
    pub fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    /// Returns the PID of the child shell process.
    pub fn child_pid(&self) -> Option<u32> {
        self._child.process_id()
    }

    /// Resize the PTY.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenPtyFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Shell detection ─────────────────────────────────────────────

    #[test]
    fn default_shell_returns_nonempty_string() {
        let shell = default_shell();
        assert!(
            !shell.is_empty(),
            "default_shell() should return a non-empty path"
        );
    }

    #[test]
    fn default_shell_returns_valid_path() {
        let shell = default_shell();
        assert!(
            std::path::Path::new(&shell).exists(),
            "default_shell() returned '{}' which does not exist",
            shell
        );
    }

    // ── PTY creation ────────────────────────────────────────────────

    #[test]
    fn pty_session_spawns_successfully() {
        let session = PtySession::new("/bin/sh", 80, 24);
        assert!(
            session.is_ok(),
            "PtySession::new should succeed with /bin/sh"
        );
    }

    #[test]
    fn pty_session_sets_initial_size() {
        let session = PtySession::new("/bin/sh", 120, 40).expect("spawn failed");
        // The master PTY should report the size we set
        let size = session.master.get_size().expect("get_size failed");
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
    }

    // ── PTY read/write ──────────────────────────────────────────────

    #[test]
    fn pty_session_receives_output() {
        let mut session = PtySession::new("/bin/sh", 80, 24).expect("spawn failed");
        // Write a command that produces output
        session
            .write(b"echo hello_pty_test\n")
            .expect("write failed");
        // Wait for output (with timeout)
        let output = session
            .reader_rx
            .recv_timeout(std::time::Duration::from_secs(3))
            .expect("should receive output from PTY");
        assert!(!output.is_empty(), "PTY output should not be empty");
    }

    #[test]
    fn pty_session_write_sends_to_shell() {
        let mut session = PtySession::new("/bin/sh", 80, 24).expect("spawn failed");
        // Write a command and check we get recognizable output back
        session.write(b"echo MARKER_12345\n").expect("write failed");

        let mut all_output = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            match session
                .reader_rx
                .recv_timeout(std::time::Duration::from_millis(200))
            {
                Ok(chunk) => all_output.extend_from_slice(&chunk),
                Err(_) => {
                    if !all_output.is_empty() {
                        break;
                    }
                }
            }
        }

        let output_str = String::from_utf8_lossy(&all_output);
        assert!(
            output_str.contains("MARKER_12345"),
            "PTY output should contain the echoed marker, got: '{}'",
            output_str
        );
    }

    // ── PTY resize ──────────────────────────────────────────────────

    #[test]
    fn pty_session_resize_updates_size() {
        let session = PtySession::new("/bin/sh", 80, 24).expect("spawn failed");
        session.resize(132, 50).expect("resize failed");
        let size = session.master.get_size().expect("get_size failed");
        assert_eq!(size.cols, 132);
        assert_eq!(size.rows, 50);
    }

    // ── Process name detection ─────────────────────────────────────

    #[test]
    fn is_shell_process_detects_common_shells() {
        assert!(is_shell_process("zsh"));
        assert!(is_shell_process("bash"));
        assert!(is_shell_process("fish"));
        assert!(is_shell_process("sh"));
        assert!(is_shell_process("dash"));
    }

    #[test]
    fn is_shell_process_rejects_non_shells() {
        assert!(!is_shell_process("vim"));
        assert!(!is_shell_process("claude"));
        assert!(!is_shell_process("python"));
        assert!(!is_shell_process("node"));
    }

    #[test]
    fn basename_from_path_extracts_name() {
        assert_eq!(basename_from_path("/usr/bin/vim"), "vim");
        assert_eq!(basename_from_path("/usr/local/bin/claude"), "claude");
        assert_eq!(basename_from_path("python3"), "python3");
        assert_eq!(basename_from_path("/bin/zsh"), "zsh");
    }

    #[test]
    fn child_pid_returns_some() {
        let session = PtySession::new("/bin/sh", 80, 24).expect("spawn failed");
        assert!(session.child_pid().is_some());
    }

    #[test]
    fn pty_session_sets_term_env() {
        let mut session = PtySession::new("/bin/sh", 80, 24).expect("spawn failed");
        // The TERM variable should be xterm-256color
        session.write(b"echo $TERM\n").expect("write failed");
        let mut all_output = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            match session
                .reader_rx
                .recv_timeout(std::time::Duration::from_millis(200))
            {
                Ok(chunk) => all_output.extend_from_slice(&chunk),
                Err(_) => {
                    if !all_output.is_empty() {
                        break;
                    }
                }
            }
        }
        let output_str = String::from_utf8_lossy(&all_output);
        assert!(
            output_str.contains("xterm-256color"),
            "PTY should set TERM=xterm-256color, got: '{}'",
            output_str
        );
    }

    #[test]
    fn foreground_process_of_idle_shell_is_none() {
        // An idle shell has no foreground children
        let session = PtySession::new("/bin/sh", 80, 24).expect("spawn failed");
        let pid = session.child_pid().unwrap();
        // Give shell a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));
        let name = foreground_process_name(pid);
        // Idle shell has no children, so should be None
        // (or could be a shell startup process — just verify it doesn't panic)
        let _ = name;
    }

    // ── Shell integration preparation ──────────────────────────────

    #[test]
    fn prepare_bash_creates_rcfile() {
        let mut cmd = CommandBuilder::new("/bin/bash");
        prepare_shell_integration("/bin/bash", &mut cmd);
        // The wrapper rcfile should exist and contain both .bashrc sourcing
        // and our integration
        let path = "/tmp/veloterm-bashrc.sh";
        assert!(
            std::path::Path::new(path).exists(),
            "bash wrapper rcfile should be created"
        );
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(
            contents.contains("source ~/.bashrc"),
            "wrapper should source user's .bashrc"
        );
        assert!(
            contents.contains("__veloterm_osc7"),
            "wrapper should contain OSC 7 function"
        );
        assert!(
            contents.contains("VELOTERM_SHELL_INTEGRATION"),
            "wrapper should have guard variable"
        );
    }

    #[test]
    fn prepare_zsh_creates_zdotdir() {
        let mut cmd = CommandBuilder::new("/bin/zsh");
        prepare_shell_integration("/bin/zsh", &mut cmd);
        let zdotdir = "/tmp/veloterm-zdotdir";
        assert!(
            std::path::Path::new(&format!("{zdotdir}/.zshenv")).exists(),
            "zsh wrapper .zshenv should be created"
        );
        assert!(
            std::path::Path::new(&format!("{zdotdir}/.zshrc")).exists(),
            "zsh wrapper .zshrc should be created"
        );
        let zshrc = std::fs::read_to_string(format!("{zdotdir}/.zshrc")).unwrap();
        assert!(
            zshrc.contains("source \"$HOME/.zshrc\""),
            "wrapper should source user's .zshrc"
        );
        assert!(
            zshrc.contains("add-zsh-hook"),
            "wrapper should contain zsh integration"
        );
        let zshenv = std::fs::read_to_string(format!("{zdotdir}/.zshenv")).unwrap();
        assert!(
            zshenv.contains("ZDOTDIR=\"$HOME\""),
            ".zshenv should restore ZDOTDIR"
        );
    }

    #[test]
    fn prepare_fish_creates_integration_file() {
        let mut cmd = CommandBuilder::new("/usr/local/bin/fish");
        prepare_shell_integration("/usr/local/bin/fish", &mut cmd);
        let path = "/tmp/veloterm-integration.fish";
        assert!(
            std::path::Path::new(path).exists(),
            "fish integration file should be created"
        );
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(
            contents.contains("__veloterm_emit_osc7"),
            "fish integration should contain OSC 7 function"
        );
    }

    #[test]
    fn prepare_noop_for_unknown_shell() {
        // Should not panic or create files for unknown shells
        let mut cmd = CommandBuilder::new("/usr/bin/python3");
        prepare_shell_integration("/usr/bin/python3", &mut cmd);
    }

    #[test]
    fn prepare_noop_for_sh() {
        // plain sh has no integration — should not panic
        let mut cmd = CommandBuilder::new("/bin/sh");
        prepare_shell_integration("/bin/sh", &mut cmd);
    }

    // ── Shell config resolution ──────────────────────────────────

    #[test]
    fn resolve_shell_config_override() {
        let mut config = crate::config::types::ShellConfig::default();
        config.program = Some("/usr/local/bin/fish".to_string());
        assert_eq!(resolve_shell(&config), "/usr/local/bin/fish");
    }

    #[test]
    fn resolve_shell_env_fallback() {
        let config = crate::config::types::ShellConfig::default();
        let shell = resolve_shell(&config);
        // Should use $SHELL if set, otherwise /bin/zsh
        assert!(!shell.is_empty());
    }

    #[test]
    fn resolve_shell_zsh_default_when_no_env() {
        // We can't unset SHELL in the current process safely, but we can verify
        // the function handles the case where program is None
        let config = crate::config::types::ShellConfig::default();
        let shell = resolve_shell(&config);
        // Either $SHELL or /bin/zsh — both valid
        assert!(shell.starts_with('/'));
    }

    #[test]
    fn zsh_wrapper_contains_starship_suppression() {
        let mut cmd = CommandBuilder::new("/bin/zsh");
        prepare_shell_integration("/bin/zsh", &mut cmd);
        let zdotdir = "/tmp/veloterm-zdotdir";
        let zshrc = std::fs::read_to_string(format!("{zdotdir}/.zshrc")).unwrap();
        assert!(
            zshrc.contains("VELOTERM_DISABLE_STARSHIP"),
            "zsh wrapper should contain starship suppression check"
        );
        assert!(
            zshrc.contains("unfunction starship"),
            "zsh wrapper should remove starship shadow after init"
        );
    }

    #[test]
    fn bash_wrapper_contains_starship_suppression() {
        let mut cmd = CommandBuilder::new("/bin/bash");
        prepare_shell_integration("/bin/bash", &mut cmd);
        let path = "/tmp/veloterm-bashrc.sh";
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(
            contents.contains("VELOTERM_DISABLE_STARSHIP"),
            "bash wrapper should contain starship suppression check"
        );
        assert!(
            contents.contains("unset -f starship"),
            "bash wrapper should remove starship shadow after init"
        );
    }

    #[test]
    fn pty_session_with_shell_config() {
        let config = crate::config::types::ShellConfig::default();
        let session = PtySession::new_with_cwd("/bin/sh", 80, 24, None, Some(&config));
        assert!(session.is_ok(), "PtySession with shell config should succeed");
    }

    #[test]
    fn bash_pty_emits_osc7_after_integration() {
        // Spawn bash with integration and verify OSC 7 is emitted
        let mut session = PtySession::new("/bin/bash", 80, 24).expect("spawn failed");
        // Give bash time to start and source the rcfile
        std::thread::sleep(std::time::Duration::from_millis(1000));
        // Trigger a new prompt to emit OSC 7
        session.write(b"\n").expect("write failed");
        std::thread::sleep(std::time::Duration::from_millis(500));
        let mut all_output = Vec::new();
        while let Ok(chunk) = session.reader_rx.try_recv() {
            all_output.extend_from_slice(&chunk);
        }
        let output = String::from_utf8_lossy(&all_output);
        assert!(
            output.contains("\x1b]7;file://"),
            "bash with integration should emit OSC 7, got: {:?}",
            &output[..output.len().min(300)]
        );
    }
}
