// Context menu: native OS right-click menu with terminal actions.

/// Actions that can be triggered from the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMenuAction {
    Copy,
    Paste,
    SelectAll,
    ClearScrollback,
    NewTab,
    NewWindow,
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    CloseTab,
    CloseOtherTabs,
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
            7 => Some(Self::ClearScrollback),
            8 => Some(Self::NewTab),
            9 => Some(Self::NewWindow),
            10 => Some(Self::CloseTab),
            11 => Some(Self::CloseOtherTabs),
            _ => None,
        }
    }
}

/// Show a native terminal area context menu.
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

/// Show a native tab bar context menu.
/// Returns the selected action, or None if dismissed.
#[cfg(target_os = "macos")]
pub fn show_tab_context_menu(
    window: &winit::window::Window,
) -> Option<ContextMenuAction> {
    macos::show_tab_context_menu(window)
}

#[cfg(not(target_os = "macos"))]
pub fn show_tab_context_menu(
    _window: &winit::window::Window,
) -> Option<ContextMenuAction> {
    None
}

#[cfg(target_os = "macos")]
mod macos {
    use super::ContextMenuAction;
    use objc2::sel;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSMenu, NSMenuItem, NSView};
    use objc2_foundation::NSString;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    fn make_item(
        mtm: MainThreadMarker,
        title: &str,
        tag: isize,
        enabled: bool,
        key_equiv: &str,
    ) -> objc2::rc::Retained<NSMenuItem> {
        let ns_title = NSString::from_str(title);
        let ns_key = NSString::from_str(key_equiv);
        // Provide a dummy action selector so macOS doesn't grey out the item.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &ns_title,
                Some(sel!(performClick:)),
                &ns_key,
            )
        };
        item.setTag(tag);
        if !enabled {
            item.setEnabled(false);
        }
        item
    }

    /// Get the NSView and show a popup menu, returning the selected action.
    fn show_popup(
        menu: &NSMenu,
        window: &winit::window::Window,
    ) -> Option<ContextMenuAction> {
        let handle = window.window_handle().ok()?;
        let ns_view: &NSView = match handle.as_raw() {
            RawWindowHandle::AppKit(h) => unsafe { &*(h.ns_view.as_ptr() as *const NSView) },
            _ => return None,
        };

        let mouse_location = NSEvent::mouseLocation();
        let ns_window = ns_view.window()?;
        let window_point = ns_window.convertPointFromScreen(mouse_location);
        let view_point = ns_view.convertPoint_fromView(window_point, None);

        let _result =
            menu.popUpMenuPositioningItem_atLocation_inView(None, view_point, Some(ns_view));

        let highlighted = menu.highlightedItem();
        let selected = highlighted.as_deref()?;
        let tag = selected.tag();

        ContextMenuAction::from_tag(tag)
    }

    pub fn show_context_menu(
        has_selection: bool,
        window: &winit::window::Window,
    ) -> Option<ContextMenuAction> {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);

        menu.addItem(&make_item(mtm, "Copy", 1, has_selection, "c"));
        menu.addItem(&make_item(mtm, "Paste", 2, true, "v"));
        menu.addItem(&make_item(mtm, "Select All", 3, true, "a"));
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&make_item(mtm, "Clear", 7, true, "k"));
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&make_item(mtm, "New Tab", 8, true, "t"));
        menu.addItem(&make_item(mtm, "New Window", 9, true, "n"));
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&make_item(mtm, "Split Pane Right", 4, true, ""));
        menu.addItem(&make_item(mtm, "Split Pane Down", 5, true, ""));
        menu.addItem(&make_item(mtm, "Close Pane", 6, true, ""));

        show_popup(&menu, window)
    }

    pub fn show_tab_context_menu(
        window: &winit::window::Window,
    ) -> Option<ContextMenuAction> {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);

        menu.addItem(&make_item(mtm, "New Tab", 8, true, "t"));
        menu.addItem(&make_item(mtm, "Close Tab", 10, true, "w"));
        menu.addItem(&make_item(mtm, "Close Other Tabs", 11, true, ""));

        show_popup(&menu, window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_menu_action_from_tag_original() {
        assert_eq!(ContextMenuAction::from_tag(1), Some(ContextMenuAction::Copy));
        assert_eq!(ContextMenuAction::from_tag(2), Some(ContextMenuAction::Paste));
        assert_eq!(ContextMenuAction::from_tag(3), Some(ContextMenuAction::SelectAll));
        assert_eq!(ContextMenuAction::from_tag(4), Some(ContextMenuAction::SplitVertical));
        assert_eq!(ContextMenuAction::from_tag(5), Some(ContextMenuAction::SplitHorizontal));
        assert_eq!(ContextMenuAction::from_tag(6), Some(ContextMenuAction::ClosePane));
    }

    #[test]
    fn context_menu_action_from_tag_new_items() {
        assert_eq!(ContextMenuAction::from_tag(7), Some(ContextMenuAction::ClearScrollback));
        assert_eq!(ContextMenuAction::from_tag(8), Some(ContextMenuAction::NewTab));
        assert_eq!(ContextMenuAction::from_tag(9), Some(ContextMenuAction::NewWindow));
        assert_eq!(ContextMenuAction::from_tag(10), Some(ContextMenuAction::CloseTab));
        assert_eq!(ContextMenuAction::from_tag(11), Some(ContextMenuAction::CloseOtherTabs));
    }

    #[test]
    fn context_menu_action_from_tag_invalid() {
        assert_eq!(ContextMenuAction::from_tag(0), None);
        assert_eq!(ContextMenuAction::from_tag(99), None);
        assert_eq!(ContextMenuAction::from_tag(-1), None);
    }

    #[test]
    fn context_menu_action_equality() {
        assert_eq!(ContextMenuAction::Copy, ContextMenuAction::Copy);
        assert_ne!(ContextMenuAction::Copy, ContextMenuAction::Paste);
        assert_ne!(ContextMenuAction::CloseTab, ContextMenuAction::CloseOtherTabs);
    }

    #[test]
    fn all_tags_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for tag in 1..=11 {
            let action = ContextMenuAction::from_tag(tag);
            assert!(action.is_some(), "Tag {tag} should map to an action");
            assert!(seen.insert(action), "Tag {tag} produced duplicate action");
        }
    }
}
