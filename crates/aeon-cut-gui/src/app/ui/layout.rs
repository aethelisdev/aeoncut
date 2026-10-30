use std::time::Instant;

use eframe::egui::{self, Color32, Vec2};

use crate::app::AeonCutApp;
use crate::app::ui::{actions, header, logs, settings, timeline};

pub fn render_central_panel(app: &mut AeonCutApp, ui: &mut egui::Ui) {
    app.ensure_textures(ui.ctx());

    // Auto refresh status every 4 seconds if disconnected
    if app.last_status_check.elapsed().as_secs() > 4 {
        if app.status.as_ref().is_none_or(|s| !s.connected)
            && let Ok(stat) = app.bridge.get_status()
        {
            app.status = Some(stat);
        }
        app.last_status_check = Instant::now();
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_rgb(17, 16, 22)))
        .show(ui, |ui| {
            // 1. Top Header with logo, title, and controls
            header::render(app, ui);

            // 2. Body content below header
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 18,
                    right: 18,
                    top: 18,
                    bottom: 14,
                })
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 12.0);

                    // Workspace Settings Panel (Clip Info, Sliders, Cut Mode)
                    settings::render(app, ui);

                    // Action Buttons (Analyze Clip & Apply Cuts to Resolve)
                    actions::render(app, ui);

                    // Analysis Stats & Colored Audio Timeline Preview
                    timeline::render(app, ui);

                    // Studio Activity Console Log Panel
                    logs::render(app, ui);
                });
        });
}
