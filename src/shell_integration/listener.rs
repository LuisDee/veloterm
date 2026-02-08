// Custom EventListener for capturing alacritty_terminal events.

use std::cell::RefCell;
use std::rc::Rc;

use alacritty_terminal::event::{Event, EventListener};

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

/// Shared event queue between VeloTermListener and the Terminal wrapper.
pub type EventQueue = Rc<RefCell<Vec<TerminalEvent>>>;

/// Create a new listener and its associated event queue.
/// The listener is moved into alacritty_terminal's Term, and the queue
/// is retained by our Terminal wrapper to drain events after each feed().
pub fn create_listener() -> (VeloTermListener, EventQueue) {
    let queue = Rc::new(RefCell::new(Vec::new()));
    let listener = VeloTermListener {
        events: queue.clone(),
    };
    (listener, queue)
}

/// Custom event listener that captures terminal events for shell integration.
/// Uses a shared Rc<RefCell> since the listener is moved into Term<T> and
/// we need to read events from outside.
#[derive(Debug, Clone)]
pub struct VeloTermListener {
    events: EventQueue,
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
            _ => {}
        }
    }
}

/// Drain all pending events from a shared event queue.
pub fn drain_events(queue: &EventQueue) -> Vec<TerminalEvent> {
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
        let (_listener, queue) = create_listener();
        assert!(!has_events(&queue));
        assert!(drain_events(&queue).is_empty());
    }

    // ── Event capture ──────────────────────────────────────────────

    #[test]
    fn listener_captures_title_change() {
        let (listener, queue) = create_listener();
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
        let (listener, queue) = create_listener();
        listener.send_event(Event::ResetTitle);
        let events = drain_events(&queue);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TerminalEvent::TitleReset));
    }

    #[test]
    fn listener_captures_bell() {
        let (listener, queue) = create_listener();
        listener.send_event(Event::Bell);
        let events = drain_events(&queue);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TerminalEvent::Bell));
    }

    #[test]
    fn listener_ignores_wakeup() {
        let (listener, queue) = create_listener();
        listener.send_event(Event::Wakeup);
        assert!(!has_events(&queue));
    }

    #[test]
    fn listener_ignores_cursor_blinking() {
        let (listener, queue) = create_listener();
        listener.send_event(Event::CursorBlinkingChange);
        assert!(!has_events(&queue));
    }

    // ── Drain behavior ─────────────────────────────────────────────

    #[test]
    fn drain_clears_events() {
        let (listener, queue) = create_listener();
        listener.send_event(Event::Title("test".to_string()));
        let _ = drain_events(&queue);
        assert!(!has_events(&queue));
        assert!(drain_events(&queue).is_empty());
    }

    #[test]
    fn listener_stores_multiple_events_in_order() {
        let (listener, queue) = create_listener();
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

        let (listener, queue) = create_listener();
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
}
