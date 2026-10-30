use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app::AeonCutApp;
use crate::i18n::get_strings;

pub fn render(app: &mut AeonCutApp, ui: &mut egui::Ui) {
    let t = get_strings(app.lang);
    let is_connected = app.status.as_ref().is_some_and(|s| s.connected);

    // ================= ACTION BUTTONS CARD (50% / 50% Full-Width Split) =================
    egui::Frame::new()
        .fill(Color32::from_rgb(20, 18, 26))
        .stroke(Stroke::new(1.0, Color32::from_rgb(46, 38, 58)))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            let spacing = 12.0;
            let total_w = ui.available_width();
            let btn_w = ((total_w - spacing) / 2.0).max(50.0);
            let btn_h = 42.0;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(spacing, 0.0);

                // 1. Analyze Clip Button (Left 50%)
                let analyze_btn = ui.add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new(t.analyze_btn)
                            .size(14.5)
                            .strong()
                            .color(Color32::from_rgb(245, 240, 255)),
                    )
                    .fill(Color32::from_rgb(54, 26, 78)) // karanlık mor zemin
                    .stroke(Stroke::new(1.2, Color32::from_rgb(170, 52, 225))) // zarif neon mor kontur
                    .corner_radius(8.0),
                );

                if analyze_btn.clicked() {
                    app.run_analysis();
                }

                // 2. Apply Cuts to Resolve Button (Right 50%)
                let can_apply = app.analysis_result.is_some() && is_connected;
                let (apply_fill, apply_stroke, apply_text) = if can_apply {
                    (
                        Color32::from_rgb(20, 85, 52), // zümrüt hazır
                        Stroke::new(1.2, Color32::from_rgb(46, 213, 115)),
                        Color32::from_rgb(240, 255, 245),
                    )
                } else {
                    (
                        Color32::from_rgb(28, 23, 34), // inaktif koyu mor/antrasit
                        Stroke::new(1.0, Color32::from_rgb(54, 44, 68)),
                        Color32::from_rgb(120, 112, 132),
                    )
                };

                let apply_btn = ui.add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new(t.apply_btn)
                            .size(14.5)
                            .strong()
                            .color(apply_text),
                    )
                    .fill(apply_fill)
                    .stroke(apply_stroke)
                    .corner_radius(8.0),
                );

                if apply_btn.clicked() && can_apply {
                    app.apply_cuts_to_resolve();
                }
            });
        });
}
