use veloterm::window::{App, WindowConfig};

fn main() {
    env_logger::init();
    log::info!("VeloTerm v0.1.0 starting");

    let config = WindowConfig::default();
    let app = App::new(config);
    if let Err(e) = app.run() {
        log::error!("Application error: {e}");
        std::process::exit(1);
    }
}
