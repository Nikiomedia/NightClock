#![windows_subsystem = "windows"]
//! NightClock – eine elegante Nachtuhr mit Wecker für den Zweitbildschirm.

mod alarm;
mod ambient;
mod app;
mod audio;
mod clock;
#[cfg(debug_assertions)]
mod devshot;
mod fonts;
mod i18n;
mod icon;
mod platform;
mod settings;
mod ui;
mod vfont;

fn main() -> eframe::Result<()> {
    let settings = settings::Settings::load();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let force_fullscreen = args
        .iter()
        .any(|a| a == "--fullscreen" || a == "-f" || a == "/f");

    const ICON: u32 = 64;
    let icon = egui::IconData {
        rgba: icon::render_icon(ICON),
        width: ICON,
        height: ICON,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NightClock")
            .with_inner_size([980.0, 560.0])
            .with_min_inner_size([360.0, 220.0])
            .with_icon(icon),
        centered: true,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "NightClock",
        options,
        Box::new(move |cc| Ok(Box::new(app::NightClockApp::new(cc, settings, force_fullscreen)))),
    )
}
