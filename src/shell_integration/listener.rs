// Custom EventListener for capturing alacritty_terminal events.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::vte::ansi::Rgb;

/// Terminal event captured by VeloTermListener.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Window title changed via OSC 0/2.
    TitleChanged(String),
    /// Title reset to default.
    TitleReset,
    /// Terminal bell.
    Bell,
}

/// A query response that needs to be written back to the PTY.
pub enum QueryResponse {
    /// Direct response string (DA1, DA2, DSR, DECRPM, etc.)
    Direct(String),
    /// Color query — caller must provide RGB, formatter produces response.
    Color(usize, Arc<dyn Fn(Rgb) -> String + Sync + Send>),
    /// Text area size query — caller must provide WindowSize, formatter produces response.
    TextAreaSize(Arc<dyn Fn(WindowSize) -> String + Sync + Send>),
}

impl std::fmt::Debug for QueryResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryResponse::Direct(s) => write!(f, "Direct({:?})", s),
            QueryResponse::Color(idx, _) => write!(f, "Color({})", idx),
            QueryResponse::TextAreaSize(_) => write!(f, "TextAreaSize"),
        }
    }
}

/// Shared event queue between VeloTermListener and the Terminal wrapper.
pub type EventQueue = Rc<RefCell<Vec<TerminalEvent>>>;

/// Shared queue for PTY write-back responses (query answers).
pub type ResponseQueue = Rc<RefCell<Vec<QueryResponse>>>;

/// Create a new listener and its associated event queue.
/// The listener is moved into alacritty_terminal's Term, and the queue
/// is retained by our Terminal wrapper to drain events after each feed().
pub fn create_listener() -> (VeloTermListener, EventQueue, ResponseQueue) {
    let queue = Rc::new(RefCell::new(Vec::new()));
    let responses = Rc::new(RefCell::new(Vec::new()));
    let listener = VeloTermListener {
        events: queue.clone(),
        responses: responses.clone(),
    };
    (listener, queue, responses)
}

/// Custom event listener that captures terminal events for shell integration
/// and query responses for PTY write-back.
/// Uses shared Rc<RefCell> since the listener is moved into Term<T> and
/// we need to read events from outside.
#[derive(Debug, Clone)]
pub struct VeloTermListener {
    events: EventQueue,
    responses: ResponseQueue,
}

impl EventListener for VeloTermListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::Title(title) => {
                self.events
                    .borrow_mut()
                    .push(TerminalEvent::TitleChanged(title));
            }
            Event::ResetTitle => {
                self.events.borrow_mut().push(TerminalEvent::TitleReset);
            }
            Event::Bell => {
                self.events.borrow_mut().push(TerminalEvent::Bell);
            }
            Event::PtyWrite(response) => {
                self.responses
                    .borrow_mut()
                    .push(QueryResponse::Direct(response));
            }
            Event::ColorRequest(index, formatter) => {
                self.responses
                    .borrow_mut()
                    .push(QueryResponse::Color(index, formatter));
            }
            Event::TextAreaSizeRequest(formatter) => {
                self.responses
                    .borrow_mut()
                    .push(QueryResponse::TextAreaSize(formatter));
            }
            _ => {}
        }
    }
}

/// Drain all pending events from a shared event queue.
pub fn drain_events(queue: &EventQueue) -> Vec<TerminalEvent> {
    queue.borrow_mut().drain(..).collect()
}

/// Drain all pending query responses from the response queue.
pub fn drain_responses(queue: &ResponseQueue) -> Vec<QueryResponse> {
    queue.borrow_mut().drain(..).collect()
}

/// Check if there are any pending events in a shared event queue.
pub fn has_events(queue: &EventQueue) -> bool {
    !queue.borrow().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Listener creation ──────────────────────────────────────────

    #[test]
    fn listener_starts_with_no_events() {
        let (_listener, queue, _responses) = create_listener();
        assert!(!has_events(&queue));
        assert!(drain_events(&queue).is_empty());
    }

    // ── Event capture ──────────────────────────────────────────────

    #[test]
    fn listener_captures_title_change() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::Title("my-project".to_string()));
        assert!(has_events(&queue));
        let events = drain_events(&queue);
        assert_eq!(events.len(), 1);
        match &events[0] {
            TerminalEvent::TitleChanged(title) => assert_eq!(title, "my-project"),
            other => panic!("expected TitleChanged, got {:?}", other),
        }
    }

    #[test]
    fn listener_captures_title_reset() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::ResetTitle);
        let events = drain_events(&queue);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TerminalEvent::TitleReset));
    }

    #[test]
    fn listener_captures_bell() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::Bell);
        let events = drain_events(&queue);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TerminalEvent::Bell));
    }

    #[test]
    fn listener_ignores_wakeup() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::Wakeup);
        assert!(!has_events(&queue));
    }

    #[test]
    fn listener_ignores_cursor_blinking() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::CursorBlinkingChange);
        assert!(!has_events(&queue));
    }

    // ── Drain behavior ─────────────────────────────────────────────

    #[test]
    fn drain_clears_events() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::Title("test".to_string()));
        let _ = drain_events(&queue);
        assert!(!has_events(&queue));
        assert!(drain_events(&queue).is_empty());
    }

    #[test]
    fn listener_stores_multiple_events_in_order() {
        let (listener, queue, _responses) = create_listener();
        listener.send_event(Event::Title("first".to_string()));
        listener.send_event(Event::Bell);
        listener.send_event(Event::Title("second".to_string()));
        let events = drain_events(&queue);
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], TerminalEvent::TitleChanged(t) if t == "first"));
        assert!(matches!(events[1], TerminalEvent::Bell));
        assert!(matches!(&events[2], TerminalEvent::TitleChanged(t) if t == "second"));
    }

    // ── Integration with alacritty_terminal Term ───────────────────

    #[test]
    fn listener_works_with_term() {
        use alacritty_terminal::grid::Dimensions;
        use alacritty_terminal::term::Config;
        use alacritty_terminal::vte::ansi;

        struct TestSize;
        impl Dimensions for TestSize {
            fn total_lines(&self) -> usize {
                24
            }
            fn screen_lines(&self) -> usize {
                24
            }
            fn columns(&self) -> usize {
                80
            }
        }

        let (listener, queue, _responses) = create_listener();
        let config = Config::default();
        let mut term = alacritty_terminal::term::Term::new(config, &TestSize, listener);
        let mut processor: ansi::Processor = ansi::Processor::new();

        // Feed an OSC 2 title sequence: ESC ] 2 ; my-title BEL
        processor.advance(&mut term, b"\x1b]2;my-title\x07");

        // The shared queue should have captured the title event
        let events = drain_events(&queue);
        assert_eq!(events.len(), 1);
        match &events[0] {
            TerminalEvent::TitleChanged(title) => assert_eq!(title, "my-title"),
            other => panic!("expected TitleChanged, got {:?}", other),
        }
    }

    // ── Query response capture ──────────────────────────────────

    #[test]
    fn listener_captures_pty_write() {
        let (listener, _queue, responses) = create_listener();
        listener.send_event(Event::PtyWrite("\x1b[?62;22c".to_string()));
        let resps = drain_responses(&responses);
        assert_eq!(resps.len(), 1);
        match &resps[0] {
            QueryResponse::Direct(s) => assert_eq!(s, "\x1b[?62;22c"),
            other => panic!("expected Direct, got {:?}", other),
        }
    }

    #[test]
    fn listener_captures_color_request() {
        let (listener, _queue, responses) = create_listener();
        let formatter = Arc::new(|rgb: Rgb| {
            format!(
                "\x1b]11;rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}\x1b\\",
                rgb.r, rgb.r, rgb.g, rgb.g, rgb.b, rgb.b
            )
        });
        listener.send_event(Event::ColorRequest(11, formatter));
        let resps = drain_responses(&responses);
        assert_eq!(resps.len(), 1);
        match &resps[0] {
            QueryResponse::Color(idx, f) => {
                assert_eq!(*idx, 11);
                let result = f(Rgb { r: 0x1a, g: 0x1a, b: 0x1a });
                assert!(result.contains("rgb:1a1a/1a1a/1a1a"));
            }
            other => panic!("expected Color, got {:?}", other),
        }
    }

    #[test]
    fn da1_query_produces_pty_write() {
        use alacritty_terminal::grid::Dimensions;
        use alacritty_terminal::term::Config;
        use alacritty_terminal::vte::ansi;

        struct TestSize;
        impl Dimensions for TestSize {
            fn total_lines(&self) -> usize { 24 }
            fn screen_lines(&self) -> usize { 24 }
            fn columns(&self) -> usize { 80 }
        }

        let (listener, _queue, responses) = create_listener();
        let config = Config::default();
        let mut term = alacritty_terminal::term::Term::new(config, &TestSize, listener);
        let mut processor: ansi::Processor = ansi::Processor::new();

        // Send DA1 query: ESC [ c
        processor.advance(&mut term, b"\x1b[c");

        let resps = drain_responses(&responses);
        assert!(!resps.is_empty(), "DA1 should produce a PtyWrite response");
        match &resps[0] {
            QueryResponse::Direct(s) => {
                assert!(s.starts_with("\x1b[?"), "DA1 response should start with CSI ?");
                assert!(s.ends_with("c"), "DA1 response should end with 'c'");
            }
            other => panic!("expected Direct PtyWrite for DA1, got {:?}", other),
        }
    }

    #[test]
    fn dsr_cursor_position_produces_response() {
        use alacritty_terminal::grid::Dimensions;
        use alacritty_terminal::term::Config;
        use alacritty_terminal::vte::ansi;

        struct TestSize;
        impl Dimensions for TestSize {
            fn total_lines(&self) -> usize { 24 }
            fn screen_lines(&self) -> usize { 24 }
            fn columns(&self) -> usize { 80 }
        }

        let (listener, _queue, responses) = create_listener();
        let config = Config::default();
        let mut term = alacritty_terminal::term::Term::new(config, &TestSize, listener);
        let mut processor: ansi::Processor = ansi::Processor::new();

        // Send DSR 6n (cursor position report): ESC [ 6 n
        processor.advance(&mut term, b"\x1b[6n");

        let resps = drain_responses(&responses);
        assert!(!resps.is_empty(), "DSR 6n should produce a response");
        match &resps[0] {
            QueryResponse::Direct(s) => {
                // Response should be CSI row ; col R
                assert!(s.starts_with("\x1b["), "DSR response should start with CSI");
                assert!(s.ends_with("R"), "DSR response should end with 'R'");
                assert!(s.contains(";"), "DSR response should contain ';' separator");
            }
            other => panic!("expected Direct PtyWrite for DSR, got {:?}", other),
        }
    }

    #[test]
    fn osc11_background_color_query_produces_color_request() {
        use alacritty_terminal::grid::Dimensions;
        use alacritty_terminal::term::Config;
        use alacritty_terminal::vte::ansi;

        struct TestSize;
        impl Dimensions for TestSize {
            fn total_lines(&self) -> usize { 24 }
            fn screen_lines(&self) -> usize { 24 }
            fn columns(&self) -> usize { 80 }
        }

        let (listener, _queue, responses) = create_listener();
        let config = Config::default();
        let mut term = alacritty_terminal::term::Term::new(config, &TestSize, listener);
        let mut processor: ansi::Processor = ansi::Processor::new();

        // Send OSC 11 query (background color): ESC ] 11 ; ? BEL
        processor.advance(&mut term, b"\x1b]11;?\x07");

        let resps = drain_responses(&responses);
        assert!(!resps.is_empty(), "OSC 11 should produce a ColorRequest response");
        match &resps[0] {
            QueryResponse::Color(idx, formatter) => {
                // alacritty_terminal maps OSC 11 to internal index 257
                assert_eq!(*idx, 257);
                // Test the formatter with a known color
                let result = formatter(Rgb { r: 0x1d, g: 0x1f, b: 0x21 });
                assert!(result.contains("1d1d"), "Should contain duplicated red bytes");
                assert!(result.contains("1f1f"), "Should contain duplicated green bytes");
                assert!(result.contains("2121"), "Should contain duplicated blue bytes");
            }
            other => panic!("expected Color for OSC 11, got {:?}", other),
        }
    }
}
