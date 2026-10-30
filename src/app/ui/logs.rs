use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app::{AeonCutApp, LogLevel};

pub fn render(app: &mut AeonCutApp, ui: &mut egui::Ui) {
    // ================= STUDIO LOG PANEL =================
    egui::Frame::new()
        .fill(Color32::from_rgb(11, 11, 15)) // dark console background (#0b0b0f)
        .stroke(Stroke::new(1.0, Color32::from_rgb(31, 31, 40))) // subtle border (#1f1f28)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            // 1. Mini Header Bar: Console / Activity Log + Clear Button
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("CONSOLE // ACTIVITY LOG")
                        .size(10.5)
                        .monospace()
                        .color(Color32::from_rgb(145, 140, 160)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let clear_btn = egui::Button::new(
                        egui::RichText::new("Clear")
                            .size(10.0)
                            .color(Color32::from_rgb(160, 155, 175)),
                    )
                    .fill(Color32::from_rgb(22, 20, 28))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(48, 42, 60)))
                    .corner_radius(4.0);

                    if ui.add(clear_btn).clicked() {
                        app.logs.clear();
                    }
                });
            });

            ui.add_space(2.0);
            ui.separator();
            ui.add_space(2.0);

            // 2. Monospace Log Stream with Auto-Scroll
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .min_scrolled_height(80.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);
                    for entry in &app.logs {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(7.0, 0.0);

                            // Timestamp [HH:MM:SS] in matte dark gray (#71717a)
                            ui.label(
                                egui::RichText::new(format!("[{}]", entry.timestamp))
                                    .monospace()
                                    .size(11.0)
                                    .color(Color32::from_rgb(113, 113, 122)),
                            );

                            // Level tag [INFO], [READY], [VAD], [SUCCESS], [WARN], [ERROR]
                            ui.label(
                                egui::RichText::new(format!("[{}]", entry.level.tag()))
                                    .monospace()
                                    .size(11.0)
                                    .strong()
                                    .color(entry.level.color()),
                            );

                            // Message text in soft matching tone
                            let msg_color = match entry.level {
                                LogLevel::Info => Color32::from_rgb(180, 180, 190),
                                LogLevel::Ready => Color32::from_rgb(130, 230, 160),
                                LogLevel::Vad => Color32::from_rgb(215, 175, 255),
                                LogLevel::Success => Color32::from_rgb(130, 230, 160),
                                LogLevel::Warn => Color32::from_rgb(250, 185, 80),
                                LogLevel::Error => Color32::from_rgb(255, 140, 140),
                            };

                            ui.label(
                                egui::RichText::new(&entry.message)
                                    .monospace()
                                    .size(11.0)
                                    .color(msg_color),
                            );
                        });
                    }
                });
        });
}
