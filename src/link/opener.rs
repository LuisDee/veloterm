use std::process::Command;

use super::{DetectedLink, LinkKind};

/// Build the command to open a detected link.
/// Returns (program, args) tuple for the platform-appropriate open command.
pub fn build_open_command(link: &DetectedLink) -> (String, Vec<String>) {
    match link.kind {
        LinkKind::Url => {
            let program = platform_open_command();
            (program, vec![link.text.clone()])
        }
        LinkKind::FilePath => {
            // Try $EDITOR first for file paths, fall back to system open
            if let Ok(editor) = std::env::var("EDITOR") {
                (editor, vec![link.text.clone()])
            } else {
                let program = platform_open_command();
                (program, vec![link.text.clone()])
            }
        }
    }
}

/// Open a detected link using the appropriate system command.
/// Spawns the process in the background (non-blocking).
pub fn open_link(link: &DetectedLink) {
    let (program, args) = build_open_command(link);
    log::info!("Opening link: {} with {}", link.text, program);

    match Command::new(&program).args(&args).spawn() {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to open link '{}' with {}: {}", link.text, program, e);
        }
    }
}

/// Returns the platform-appropriate "open" command.
fn platform_open_command() -> String {
    if cfg!(target_os = "macos") {
        "open".to_string()
    } else {
        "xdg-open".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url_link(text: &str) -> DetectedLink {
        DetectedLink {
            kind: LinkKind::Url,
            start: (0, 0),
            end: (0, text.len().saturating_sub(1)),
            text: text.to_string(),
        }
    }

    fn path_link(text: &str) -> DetectedLink {
        DetectedLink {
            kind: LinkKind::FilePath,
            start: (0, 0),
            end: (0, text.len().saturating_sub(1)),
            text: text.to_string(),
        }
    }

    #[test]
    fn url_opens_with_system_command() {
        let link = url_link("https://example.com");
        let (program, args) = build_open_command(&link);
        if cfg!(target_os = "macos") {
            assert_eq!(program, "open");
        } else {
            assert_eq!(program, "xdg-open");
        }
        assert_eq!(args, vec!["https://example.com"]);
    }

    #[test]
    fn filepath_uses_editor_env() {
        // Temporarily set EDITOR
        let original = std::env::var("EDITOR").ok();
        std::env::set_var("EDITOR", "vim");

        let link = path_link("/usr/bin/cargo");
        let (program, args) = build_open_command(&link);
        assert_eq!(program, "vim");
        assert_eq!(args, vec!["/usr/bin/cargo"]);

        // Restore
        match original {
            Some(val) => std::env::set_var("EDITOR", val),
            None => std::env::remove_var("EDITOR"),
        }
    }

    #[test]
    fn filepath_falls_back_to_system_open() {
        let original = std::env::var("EDITOR").ok();
        std::env::remove_var("EDITOR");

        let link = path_link("/usr/bin/cargo");
        let (program, args) = build_open_command(&link);
        if cfg!(target_os = "macos") {
            assert_eq!(program, "open");
        } else {
            assert_eq!(program, "xdg-open");
        }
        assert_eq!(args, vec!["/usr/bin/cargo"]);

        // Restore
        if let Some(val) = original {
            std::env::set_var("EDITOR", val);
        }
    }

    #[test]
    fn platform_open_command_is_known() {
        let cmd = platform_open_command();
        assert!(cmd == "open" || cmd == "xdg-open");
    }
}
