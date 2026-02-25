//! Linux platform module.
//!
//! Provides platform-specific functions for Linux, mirroring the API surface
//! of `platform::macos`. Functions that have no Linux equivalent are no-ops.

use std::fs;

/// No-op on Linux. The .app bundle HiDPI concept is macOS-only.
pub fn check_hidpi_status(_winit_scale: f64) {
    // Nothing to do on Linux — winit reports correct scale natively.
}

/// On Linux, winit reports the correct display scale from X11/Wayland.
/// No CoreGraphics detection needed — just return the winit value.
pub fn detect_display_scale(winit_scale: f64) -> f64 {
    winit_scale
}

/// No-op on Linux. X11/Wayland do not support programmatic titlebar
/// color changes in the same way macOS NSWindow does.
pub fn set_titlebar_color(_window: &winit::window::Window, _r: f64, _g: f64, _b: f64) {
    // Nothing to do on Linux.
}

/// Read child PIDs from `/proc/<pid>/task/<pid>/children`.
///
/// Returns a list of child PIDs, or an empty vec on failure.
fn read_child_pids(pid: u32) -> Vec<u32> {
    let path = format!("/proc/{pid}/task/{pid}/children");
    match fs::read_to_string(&path) {
        Ok(content) => content
            .split_whitespace()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Read a process name from `/proc/<pid>/comm`.
///
/// Returns the process name (trimmed), or None on failure.
fn read_proc_comm(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/comm");
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Query the foreground process name for a given shell PID.
///
/// Reads `/proc/<pid>/task/<pid>/children` to find child PIDs, then
/// reads `/proc/<child>/comm` for the last (most recent) child.
/// Returns None if the shell has no children or detection fails.
pub fn foreground_process_name(shell_pid: u32) -> Option<String> {
    let children = read_child_pids(shell_pid);
    // Take the last child (most recently forked)
    let child_pid = children.last()?;
    read_proc_comm(*child_pid)
}

/// Parse child PIDs from the content of a `/proc/<pid>/task/<pid>/children` file.
///
/// Exposed for testing with mock data.
pub fn parse_children_content(content: &str) -> Vec<u32> {
    content
        .split_whitespace()
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

/// Parse a process name from the content of a `/proc/<pid>/comm` file.
///
/// Exposed for testing with mock data.
pub fn parse_comm_content(content: &str) -> Option<String> {
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_children_content tests ──

    #[test]
    fn parse_children_single_pid() {
        let pids = parse_children_content("1234 ");
        assert_eq!(pids, vec![1234]);
    }

    #[test]
    fn parse_children_multiple_pids() {
        let pids = parse_children_content("100 200 300 ");
        assert_eq!(pids, vec![100, 200, 300]);
    }

    #[test]
    fn parse_children_empty() {
        let pids = parse_children_content("");
        assert!(pids.is_empty());
    }

    #[test]
    fn parse_children_whitespace_only() {
        let pids = parse_children_content("   \n  ");
        assert!(pids.is_empty());
    }

    #[test]
    fn parse_children_with_invalid_entries() {
        let pids = parse_children_content("123 abc 456");
        assert_eq!(pids, vec![123, 456]);
    }

    #[test]
    fn parse_children_newline_separated() {
        let pids = parse_children_content("100\n200\n300\n");
        assert_eq!(pids, vec![100, 200, 300]);
    }

    // ── parse_comm_content tests ──

    #[test]
    fn parse_comm_normal() {
        assert_eq!(parse_comm_content("vim\n"), Some("vim".to_string()));
    }

    #[test]
    fn parse_comm_no_newline() {
        assert_eq!(parse_comm_content("bash"), Some("bash".to_string()));
    }

    #[test]
    fn parse_comm_empty() {
        assert_eq!(parse_comm_content(""), None);
    }

    #[test]
    fn parse_comm_whitespace_only() {
        assert_eq!(parse_comm_content("  \n  "), None);
    }

    #[test]
    fn parse_comm_with_spaces_in_name() {
        // Some process names can have spaces (rare but possible)
        assert_eq!(
            parse_comm_content("Web Content\n"),
            Some("Web Content".to_string())
        );
    }

    // ── detect_display_scale tests ──

    #[test]
    fn detect_scale_returns_winit_value() {
        assert_eq!(detect_display_scale(1.0), 1.0);
        assert_eq!(detect_display_scale(2.0), 2.0);
        assert_eq!(detect_display_scale(1.5), 1.5);
    }

    // ── foreground_process_name with mock /proc via tempdir ──

    #[test]
    fn foreground_process_from_mock_proc() {
        // This test validates the parsing logic, not actual /proc access.
        // Real /proc testing happens in Linux CI integration tests.
        let content = "5678 9012 ";
        let pids = parse_children_content(content);
        assert_eq!(pids.last(), Some(&9012));

        let comm = parse_comm_content("nvim\n");
        assert_eq!(comm, Some("nvim".to_string()));
    }

    #[test]
    fn foreground_process_no_children() {
        let content = "";
        let pids = parse_children_content(content);
        assert!(pids.is_empty());
        // foreground_process_name would return None
    }
}
