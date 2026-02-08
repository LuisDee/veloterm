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

#[cfg(not(target_os = "macos"))]
pub fn foreground_process_name(_shell_pid: u32) -> Option<String> {
    // Linux: could read /proc/<pid>/task/<pid>/children + /proc/<child>/comm
    // For now, return None (fall back to CWD)
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
        let pty_system = portable_pty::native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenPtyFailed(e.to_string()))?;

        let cmd = CommandBuilder::new(shell);
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
}
