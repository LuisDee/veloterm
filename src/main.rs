use veloterm::config::types::Config;
use veloterm::window::{App, WindowConfig};

fn main() {
    // Handle --print-default-config before any other initialization
    if std::env::args().any(|a| a == "--print-default-config") {
        print!("{}", Config::print_default());
        return;
    }

    env_logger::init();
    log::info!("VeloTerm v0.1.0 starting");

    // Load config from XDG path or use defaults
    let config_path = dirs_config_path();
    let app_config = match Config::load(&config_path) {
        Ok(cfg) => {
            log::info!("Config loaded from {}", config_path.display());
            cfg
        }
        Err(e) => {
            log::warn!("Config load error ({}), using defaults", e);
            Config::default()
        }
    };
    log::info!(
        "Theme: {}, Font size: {}, Scrollback: {}",
        app_config.colors.theme,
        app_config.font.size,
        app_config.scrollback.lines
    );

    let window_config = WindowConfig::default();
    let app = App::new(window_config, app_config);
    if let Err(e) = app.run() {
        log::error!("Application error: {e}");
        std::process::exit(1);
    }
}

/// Get the config file path (~/.config/veloterm/config.toml).
fn dirs_config_path() -> std::path::PathBuf {
    let mut path = dirs_home().join(".config").join("veloterm");
    std::fs::create_dir_all(&path).ok();
    path.push("config.toml");
    path
}

/// Get the user's home directory.
fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}
