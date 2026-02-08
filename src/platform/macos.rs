use core_graphics::display::CGDisplay;

/// Check HiDPI status and log a warning if running without a .app bundle on Retina.
///
/// macOS only reports 2x scale_factor to apps inside a .app bundle with
/// NSHighResolutionCapable=true in Info.plist. Use `./run.sh` to wrap the
/// debug binary in a minimal bundle for proper Retina support.
pub fn check_hidpi_status(winit_scale: f64) {
    let exe = std::env::current_exe().unwrap_or_default();
    let in_bundle = exe
        .parent() // MacOS/
        .and_then(|p| p.parent()) // Contents/
        .and_then(|p| p.parent()) // .app/
        .map(|p| p.extension().map_or(false, |e| e == "app"))
        .unwrap_or(false);

    if winit_scale < 1.5 && !in_bundle {
        log::warn!(
            "Not running as .app bundle — Retina/HiDPI disabled (scale={winit_scale:.1}). \
             Use ./run.sh for 2x rendering on Retina displays."
        );
    }
}

/// Detect the actual display scale factor using CoreGraphics.
///
/// When a bare binary (not .app bundle) runs on Retina, winit reports
/// scale_factor=1.0 but the display is actually 2x. This function queries
/// the main display's native pixel dimensions vs its logical (point) dimensions
/// to determine the true backing scale factor.
///
/// Returns the detected scale, or falls back to the winit-reported value.
pub fn detect_display_scale(winit_scale: f64) -> f64 {
    let display = CGDisplay::main();
    let pixel_width = display.pixels_wide() as f64;
    let bounds = display.bounds();
    let point_width = bounds.size.width;

    if point_width > 0.0 {
        let detected = pixel_width / point_width;
        if (detected - winit_scale).abs() > 0.1 {
            log::info!(
                "Display scale: detected {detected:.1}x from CoreGraphics \
                 (winit reported {winit_scale:.1}x)"
            );
        }
        detected
    } else {
        winit_scale
    }
}
