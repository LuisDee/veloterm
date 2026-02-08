// Context menu: native OS right-click menu with terminal actions.

/// Actions that can be triggered from the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    Copy,
    Paste,
    SelectAll,
    SplitVertical,
    SplitHorizontal,
    ClosePane,
}

impl ContextMenuAction {
    fn from_tag(tag: isize) -> Option<Self> {
        match tag {
            1 => Some(Self::Copy),
            2 => Some(Self::Paste),
            3 => Some(Self::SelectAll),
            4 => Some(Self::SplitVertical),
            5 => Some(Self::SplitHorizontal),
            6 => Some(Self::ClosePane),
            _ => None,
        }
    }
}

/// Show a native context menu at the given screen position.
/// Returns the selected action, or None if dismissed.
#[cfg(target_os = "macos")]
pub fn show_context_menu(
    has_selection: bool,
    window: &winit::window::Window,
) -> Option<ContextMenuAction> {
    macos::show_context_menu(has_selection, window)
}

#[cfg(not(target_os = "macos"))]
pub fn show_context_menu(
    _has_selection: bool,
    _window: &winit::window::Window,
) -> Option<ContextMenuAction> {
    None
}

#[cfg(target_os = "macos")]
mod macos {
    use super::ContextMenuAction;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSMenu, NSMenuItem, NSView};
    use objc2_foundation::NSString;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    fn make_item(
        mtm: MainThreadMarker,
        title: &str,
        tag: isize,
        enabled: bool,
    ) -> objc2::rc::Retained<NSMenuItem> {
        let ns_title = NSString::from_str(title);
        let key_equiv = NSString::from_str("");
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &ns_title,
                None,
                &key_equiv,
            )
        };
        item.setTag(tag);
        if !enabled {
            item.setEnabled(false);
        }
        item
    }

    pub fn show_context_menu(
        has_selection: bool,
        window: &winit::window::Window,
    ) -> Option<ContextMenuAction> {
        // We're in the winit event loop, so we're on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        // Get the NSView from winit window
        let handle = window.window_handle().ok()?;
        let ns_view: &NSView = match handle.as_raw() {
            RawWindowHandle::AppKit(h) => unsafe { &*(h.ns_view.as_ptr() as *const NSView) },
            _ => return None,
        };

        // Build the menu
        let menu = NSMenu::new(mtm);

        let copy_item = make_item(mtm, "Copy", 1, has_selection);
        let paste_item = make_item(mtm, "Paste", 2, true);
        let select_all_item = make_item(mtm, "Select All", 3, true);
        let split_v_item = make_item(mtm, "Split Vertical", 4, true);
        let split_h_item = make_item(mtm, "Split Horizontal", 5, true);
        let close_item = make_item(mtm, "Close Pane", 6, true);

        menu.addItem(&copy_item);
        menu.addItem(&paste_item);
        menu.addItem(&select_all_item);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&split_v_item);
        menu.addItem(&split_h_item);
        menu.addItem(&close_item);

        // Get mouse location and convert to view coords
        let mouse_location = unsafe { NSEvent::mouseLocation() };
        let ns_window = ns_view.window()?;
        let window_point = unsafe { ns_window.convertPointFromScreen(mouse_location) };
        let view_point = unsafe { ns_view.convertPoint_fromView(window_point, None) };

        // Show popup menu — blocks until user selects or dismisses
        let _result = unsafe {
            menu.popUpMenuPositioningItem_atLocation_inView(None, view_point, Some(ns_view))
        };

        // Check which item was selected via highlightedItem
        let highlighted = unsafe { menu.highlightedItem() };
        let selected = highlighted.as_deref()?;
        let tag = selected.tag();

        ContextMenuAction::from_tag(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_menu_action_from_tag() {
        assert_eq!(ContextMenuAction::from_tag(1), Some(ContextMenuAction::Copy));
        assert_eq!(ContextMenuAction::from_tag(2), Some(ContextMenuAction::Paste));
        assert_eq!(ContextMenuAction::from_tag(3), Some(ContextMenuAction::SelectAll));
        assert_eq!(ContextMenuAction::from_tag(4), Some(ContextMenuAction::SplitVertical));
        assert_eq!(ContextMenuAction::from_tag(5), Some(ContextMenuAction::SplitHorizontal));
        assert_eq!(ContextMenuAction::from_tag(6), Some(ContextMenuAction::ClosePane));
        assert_eq!(ContextMenuAction::from_tag(0), None);
        assert_eq!(ContextMenuAction::from_tag(99), None);
    }

    #[test]
    fn context_menu_action_equality() {
        assert_eq!(ContextMenuAction::Copy, ContextMenuAction::Copy);
        assert_ne!(ContextMenuAction::Copy, ContextMenuAction::Paste);
    }
}
