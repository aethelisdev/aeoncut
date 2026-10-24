mod app;
mod bridge;
mod engine;
mod i18n;
mod models;

use app::AeonCutApp;
use eframe::egui;

fn load_app_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!("../assets/Aeoncut.png");
    let img = image::load_from_memory(png_bytes).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut builder = egui::ViewportBuilder::default()
        .with_app_id("aeon-cut")
        .with_inner_size([760.0, 560.0])
        .with_min_inner_size([600.0, 440.0])
        .with_title("AEON-CUT // DaVinci Resolve 21 VAD Studio");

    if let Some(icon) = load_app_icon() {
        builder = builder.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport: builder,
        ..Default::default()
    };

    eframe::run_native(
        "Aeon-Cut",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(18, 18, 22);
            visuals.window_fill = egui::Color32::from_rgb(22, 22, 28);
            visuals.extreme_bg_color = egui::Color32::from_rgb(14, 14, 18);
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(AeonCutApp::default()))
        }),
    )
}
