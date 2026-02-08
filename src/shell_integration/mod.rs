// Shell integration: OSC sequence parsing, shell state tracking, and prompt navigation.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Maximum number of prompt positions to retain in history.
const MAX_PROMPT_POSITIONS: usize = 1000;

/// Maximum number of command records to retain.
const MAX_COMMAND_HISTORY: usize = 100;

/// Semantic prompt marker types from OSC 133.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PromptMarker {
    /// OSC 133;A — Prompt region start.
    PromptStart,
    /// OSC 133;B — Command start (user pressed Enter).
    CommandStart,
    /// OSC 133;C — Command output start.
    CommandOutputStart,
    /// OSC 133;D — Command finished.
    CommandEnd,
}

/// Events extracted from OSC sequences in the PTY byte stream.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellEvent {
    /// OSC 133 semantic prompt marker with optional exit status (for CommandEnd).
    SemanticPrompt(PromptMarker, Option<i32>),
    /// OSC 7 current working directory change.
    CurrentDirectory(String),
    /// OSC 0 or OSC 2 title change.
    Title(String),
}

/// Record of a completed command with timing information.
#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub start: Instant,
    pub end: Instant,
    pub duration: Duration,
    pub exit_status: Option<i32>,
}

/// Per-pane shell state tracking.
#[derive(Debug)]
pub struct ShellState {
    /// Current working directory reported by the shell (OSC 7).
    pub cwd: Option<String>,
    /// Explicit title set via OSC 0/2, if any.
    pub title: Option<String>,
    /// Whether the title was explicitly set via OSC 0/2 (vs derived from CWD).
    pub title_is_explicit: bool,
    /// Line positions where prompts were detected (OSC 133;A).
    prompt_positions: VecDeque<usize>,
    /// Start time of the currently running command, if any.
    command_start: Option<Instant>,
    /// History of completed commands with timing.
    command_history: VecDeque<CommandRecord>,
    /// Last known exit status.
    pub last_exit_status: Option<i32>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellState {
    /// Create a new ShellState with no tracked state.
    pub fn new() -> Self {
        Self {
            cwd: None,
            title: None,
            title_is_explicit: false,
            prompt_positions: VecDeque::new(),
            command_start: None,
            command_history: VecDeque::new(),
            last_exit_status: None,
        }
    }

    /// Process a shell event and update internal state.
    pub fn handle_event(&mut self, event: &ShellEvent, current_line: usize) {
        match event {
            ShellEvent::SemanticPrompt(marker, exit_status) => match marker {
                PromptMarker::PromptStart => {
                    self.prompt_positions.push_back(current_line);
                    if self.prompt_positions.len() > MAX_PROMPT_POSITIONS {
                        self.prompt_positions.pop_front();
                    }
                }
                PromptMarker::CommandStart => {
                    self.command_start = Some(Instant::now());
                }
                PromptMarker::CommandOutputStart => {
                    // Informational — no state change needed
                }
                PromptMarker::CommandEnd => {
                    if let Some(exit) = exit_status {
                        self.last_exit_status = Some(*exit);
                    }
                    if let Some(start) = self.command_start.take() {
                        let end = Instant::now();
                        let duration = end.duration_since(start);
                        let record = CommandRecord {
                            start,
                            end,
                            duration,
                            exit_status: *exit_status,
                        };
                        self.command_history.push_back(record);
                        if self.command_history.len() > MAX_COMMAND_HISTORY {
                            self.command_history.pop_front();
                        }
                    }
                }
            },
            ShellEvent::CurrentDirectory(path) => {
                self.cwd = Some(path.clone());
            }
            ShellEvent::Title(title) => {
                self.title = Some(title.clone());
                self.title_is_explicit = true;
            }
        }
    }

    /// Get the list of prompt positions (oldest first).
    pub fn prompt_positions(&self) -> &VecDeque<usize> {
        &self.prompt_positions
    }

    /// Get the command history (oldest first).
    pub fn command_history(&self) -> &VecDeque<CommandRecord> {
        &self.command_history
    }

    /// Get the most recent command record, if any.
    pub fn last_command(&self) -> Option<&CommandRecord> {
        self.command_history.back()
    }

    /// Find the prompt position nearest above `current_line`.
    /// Returns None if no prompts are above current_line.
    pub fn previous_prompt(&self, current_line: usize) -> Option<usize> {
        self.prompt_positions
            .iter()
            .rev()
            .find(|&&pos| pos < current_line)
            .copied()
    }

    /// Find the prompt position nearest below `current_line`.
    /// Returns None if no prompts are below current_line.
    pub fn next_prompt(&self, current_line: usize) -> Option<usize> {
        self.prompt_positions
            .iter()
            .find(|&&pos| pos > current_line)
            .copied()
    }
}

/// Parse raw PTY bytes and extract shell events (OSC 7, OSC 133).
/// Returns a list of events found in the byte stream.
/// This is a lightweight pre-scan; the bytes are still passed to alacritty_terminal afterward.
pub fn extract_shell_events(bytes: &[u8]) -> Vec<ShellEvent> {
    let mut events = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Look for ESC ] (OSC start)
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b']' {
            // Find the OSC terminator: BEL (0x07) or ST (ESC \)
            let osc_start = i + 2;
            if let Some((payload, end)) = find_osc_payload(bytes, osc_start) {
                if let Some(event) = parse_osc_payload(payload) {
                    events.push(event);
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    events
}

/// Find the payload and end position of an OSC sequence starting at `start`.
/// Returns (payload_str, position_after_terminator).
fn find_osc_payload(bytes: &[u8], start: usize) -> Option<(&str, usize)> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == 0x07 {
            // BEL terminator
            let payload = std::str::from_utf8(&bytes[start..i]).ok()?;
            return Some((payload, i + 1));
        }
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
            // ST terminator (ESC \)
            let payload = std::str::from_utf8(&bytes[start..i]).ok()?;
            return Some((payload, i + 2));
        }
        i += 1;
    }
    None
}

/// Parse an OSC payload string into a ShellEvent, if recognized.
fn parse_osc_payload(payload: &str) -> Option<ShellEvent> {
    if let Some(rest) = payload.strip_prefix("133;") {
        let (marker, exit_status) = parse_osc133_payload(rest)?;
        Some(ShellEvent::SemanticPrompt(marker, exit_status))
    } else if let Some(uri) = payload.strip_prefix("7;") {
        let path = parse_osc7_uri(uri)?;
        Some(ShellEvent::CurrentDirectory(path))
    } else {
        None
    }
}

/// Parse an OSC 7 URI payload (e.g., "file://hostname/path") and extract the path.
pub fn parse_osc7_uri(payload: &str) -> Option<String> {
    let rest = payload.strip_prefix("file://")?;
    // After "file://", the next component is the hostname (possibly empty),
    // followed by the path starting with '/'.
    let path_start = rest.find('/')?;
    let path = &rest[path_start..];
    Some(percent_decode(path))
}

/// Decode percent-encoded characters in a URI path.
fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(hi), Some(lo)) = (hi, lo) {
                if let Ok(decoded) = u8::from_str_radix(
                    &format!("{}{}", hi as char, lo as char),
                    16,
                ) {
                    result.push(decoded as char);
                    continue;
                }
            }
            result.push('%');
        } else {
            result.push(b as char);
        }
    }
    result
}

/// Parse an OSC 133 payload (e.g., "A", "B", "D;0") into a PromptMarker and optional exit status.
pub fn parse_osc133_payload(payload: &str) -> Option<(PromptMarker, Option<i32>)> {
    if payload.is_empty() {
        return None;
    }
    let first = payload.as_bytes()[0];
    match first {
        b'A' => Some((PromptMarker::PromptStart, None)),
        b'B' => Some((PromptMarker::CommandStart, None)),
        b'C' => Some((PromptMarker::CommandOutputStart, None)),
        b'D' => {
            let exit_status = payload
                .strip_prefix("D;")
                .and_then(|s| s.parse::<i32>().ok());
            Some((PromptMarker::CommandEnd, exit_status))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ShellState creation ─────────────────────────────────────────

    #[test]
    fn shell_state_default_cwd_is_none() {
        let state = ShellState::new();
        assert!(state.cwd.is_none());
    }

    #[test]
    fn shell_state_default_title_is_none() {
        let state = ShellState::new();
        assert!(state.title.is_none());
    }

    #[test]
    fn shell_state_default_no_prompt_positions() {
        let state = ShellState::new();
        assert!(state.prompt_positions().is_empty());
    }

    #[test]
    fn shell_state_default_no_command_history() {
        let state = ShellState::new();
        assert!(state.command_history().is_empty());
    }

    #[test]
    fn shell_state_default_no_exit_status() {
        let state = ShellState::new();
        assert!(state.last_exit_status.is_none());
    }

    #[test]
    fn shell_state_default_title_not_explicit() {
        let state = ShellState::new();
        assert!(!state.title_is_explicit);
    }

    // ── OSC 133 prompt start ────────────────────────────────────────

    #[test]
    fn osc133a_adds_prompt_position() {
        let mut state = ShellState::new();
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::PromptStart, None),
            42,
        );
        assert_eq!(state.prompt_positions().len(), 1);
        assert_eq!(state.prompt_positions()[0], 42);
    }

    #[test]
    fn osc133a_multiple_prompts_in_order() {
        let mut state = ShellState::new();
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::PromptStart, None),
            10,
        );
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::PromptStart, None),
            20,
        );
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::PromptStart, None),
            30,
        );
        assert_eq!(state.prompt_positions().len(), 3);
        assert_eq!(state.prompt_positions()[0], 10);
        assert_eq!(state.prompt_positions()[2], 30);
    }

    // ── OSC 133;B command start ─────────────────────────────────────

    #[test]
    fn osc133b_records_command_start_time() {
        let mut state = ShellState::new();
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandStart, None),
            10,
        );
        assert!(state.command_start.is_some());
    }

    // ── OSC 133;C command output start ──────────────────────────────

    #[test]
    fn osc133c_is_tracked() {
        // CommandOutputStart should not panic or alter core state unexpectedly
        let mut state = ShellState::new();
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandOutputStart, None),
            10,
        );
        // No command record yet — output start is informational
        assert!(state.command_history().is_empty());
    }

    // ── OSC 133;D command end ───────────────────────────────────────

    #[test]
    fn osc133d_records_command_with_duration() {
        let mut state = ShellState::new();
        // Simulate command start
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandStart, None),
            10,
        );
        // Small delay to have measurable duration
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Command end with exit status 0
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandEnd, Some(0)),
            11,
        );
        assert_eq!(state.command_history().len(), 1);
        let record = state.last_command().unwrap();
        assert!(record.duration >= Duration::from_millis(1));
        assert_eq!(record.exit_status, Some(0));
    }

    #[test]
    fn osc133d_stores_exit_status() {
        let mut state = ShellState::new();
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandStart, None),
            10,
        );
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandEnd, Some(127)),
            11,
        );
        assert_eq!(state.last_exit_status, Some(127));
        assert_eq!(state.last_command().unwrap().exit_status, Some(127));
    }

    #[test]
    fn osc133d_without_start_is_ignored() {
        let mut state = ShellState::new();
        // Command end without a prior command start — should not panic
        state.handle_event(
            &ShellEvent::SemanticPrompt(PromptMarker::CommandEnd, Some(0)),
            10,
        );
        assert!(state.command_history().is_empty());
    }

    // ── OSC 7 CWD parsing ──────────────────────────────────────────

    #[test]
    fn osc7_sets_cwd() {
        let mut state = ShellState::new();
        state.handle_event(
            &ShellEvent::CurrentDirectory("/home/user/projects".to_string()),
            0,
        );
        assert_eq!(state.cwd.as_deref(), Some("/home/user/projects"));
    }

    #[test]
    fn parse_osc7_uri_extracts_path() {
        let path = parse_osc7_uri("file://hostname/home/user/projects");
        assert_eq!(path.as_deref(), Some("/home/user/projects"));
    }

    #[test]
    fn parse_osc7_uri_localhost() {
        let path = parse_osc7_uri("file://localhost/tmp");
        assert_eq!(path.as_deref(), Some("/tmp"));
    }

    #[test]
    fn parse_osc7_uri_empty_host() {
        let path = parse_osc7_uri("file:///home/user");
        assert_eq!(path.as_deref(), Some("/home/user"));
    }

    #[test]
    fn parse_osc7_uri_invalid_scheme() {
        let path = parse_osc7_uri("http://example.com/foo");
        assert!(path.is_none());
    }

    #[test]
    fn parse_osc7_uri_percent_encoded() {
        let path = parse_osc7_uri("file://host/home/user/my%20folder");
        assert_eq!(path.as_deref(), Some("/home/user/my folder"));
    }

    // ── OSC 0/2 title capture ──────────────────────────────────────

    #[test]
    fn title_event_sets_title() {
        let mut state = ShellState::new();
        state.handle_event(&ShellEvent::Title("my-project".to_string()), 0);
        assert_eq!(state.title.as_deref(), Some("my-project"));
        assert!(state.title_is_explicit);
    }

    // ── Prompt position bounding ───────────────────────────────────

    #[test]
    fn prompt_positions_bounded_at_max() {
        let mut state = ShellState::new();
        for i in 0..MAX_PROMPT_POSITIONS + 100 {
            state.handle_event(
                &ShellEvent::SemanticPrompt(PromptMarker::PromptStart, None),
                i,
            );
        }
        assert_eq!(state.prompt_positions().len(), MAX_PROMPT_POSITIONS);
        // Oldest positions should have been evicted
        assert_eq!(state.prompt_positions()[0], 100);
    }

    // ── Malformed OSC sequence handling ─────────────────────────────

    #[test]
    fn parse_osc133_valid_a() {
        let result = parse_osc133_payload("A");
        assert_eq!(result, Some((PromptMarker::PromptStart, None)));
    }

    #[test]
    fn parse_osc133_valid_b() {
        let result = parse_osc133_payload("B");
        assert_eq!(result, Some((PromptMarker::CommandStart, None)));
    }

    #[test]
    fn parse_osc133_valid_c() {
        let result = parse_osc133_payload("C");
        assert_eq!(result, Some((PromptMarker::CommandOutputStart, None)));
    }

    #[test]
    fn parse_osc133_valid_d_with_status() {
        let result = parse_osc133_payload("D;0");
        assert_eq!(result, Some((PromptMarker::CommandEnd, Some(0))));
    }

    #[test]
    fn parse_osc133_valid_d_nonzero_status() {
        let result = parse_osc133_payload("D;1");
        assert_eq!(result, Some((PromptMarker::CommandEnd, Some(1))));
    }

    #[test]
    fn parse_osc133_valid_d_no_status() {
        let result = parse_osc133_payload("D");
        assert_eq!(result, Some((PromptMarker::CommandEnd, None)));
    }

    #[test]
    fn parse_osc133_unknown_marker() {
        let result = parse_osc133_payload("Z");
        assert!(result.is_none());
    }

    #[test]
    fn parse_osc133_empty_payload() {
        let result = parse_osc133_payload("");
        assert!(result.is_none());
    }

    // ── Byte stream extraction ─────────────────────────────────────

    #[test]
    fn extract_osc7_from_bytes() {
        // OSC 7 with ST terminator: ESC ] 7 ; file://host/path ESC backslash
        let bytes = b"\x1b]7;file://host/home/user\x1b\\";
        let events = extract_shell_events(bytes);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ShellEvent::CurrentDirectory(path) => assert_eq!(path, "/home/user"),
            other => panic!("expected CurrentDirectory, got {:?}", other),
        }
    }

    #[test]
    fn extract_osc133_from_bytes() {
        // OSC 133 with BEL terminator: ESC ] 133 ; A BEL
        let bytes = b"\x1b]133;A\x07";
        let events = extract_shell_events(bytes);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ShellEvent::SemanticPrompt(marker, status) => {
                assert_eq!(*marker, PromptMarker::PromptStart);
                assert!(status.is_none());
            }
            other => panic!("expected SemanticPrompt, got {:?}", other),
        }
    }

    #[test]
    fn extract_osc133_d_with_status_from_bytes() {
        let bytes = b"\x1b]133;D;0\x07";
        let events = extract_shell_events(bytes);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ShellEvent::SemanticPrompt(marker, status) => {
                assert_eq!(*marker, PromptMarker::CommandEnd);
                assert_eq!(*status, Some(0));
            }
            other => panic!("expected SemanticPrompt, got {:?}", other),
        }
    }

    #[test]
    fn extract_multiple_events_from_mixed_bytes() {
        // Mixed: some text, then OSC 133;A, then more text, then OSC 7
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"Hello world");
        bytes.extend_from_slice(b"\x1b]133;A\x07");
        bytes.extend_from_slice(b"some output");
        bytes.extend_from_slice(b"\x1b]7;file:///tmp/foo\x1b\\");
        let events = extract_shell_events(&bytes);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn extract_no_events_from_plain_text() {
        let bytes = b"Hello, this is just plain text\r\n";
        let events = extract_shell_events(bytes);
        assert!(events.is_empty());
    }

    #[test]
    fn extract_osc7_with_bel_terminator() {
        let bytes = b"\x1b]7;file://host/path\x07";
        let events = extract_shell_events(bytes);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ShellEvent::CurrentDirectory(path) => assert_eq!(path, "/path"),
            other => panic!("expected CurrentDirectory, got {:?}", other),
        }
    }

    #[test]
    fn extract_osc133_with_st_terminator() {
        let bytes = b"\x1b]133;B\x1b\\";
        let events = extract_shell_events(bytes);
        assert_eq!(events.len(), 1);
        match &events[0] {
            ShellEvent::SemanticPrompt(marker, _) => {
                assert_eq!(*marker, PromptMarker::CommandStart);
            }
            other => panic!("expected SemanticPrompt, got {:?}", other),
        }
    }
}
