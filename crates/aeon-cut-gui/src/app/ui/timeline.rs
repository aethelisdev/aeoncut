use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app::AeonCutApp;
use crate::i18n::get_strings;

pub fn render(app: &mut AeonCutApp, ui: &mut egui::Ui) {
    let t = get_strings(app.lang);

    // ================= ANALYSIS & VISUAL TIMELINE PREVIEW =================
    if let Some(result) = &app.analysis_result {
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(24, 24, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(45, 45, 60)))
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(t.summary_header)
                            .strong()
                            .color(Color32::from_rgb(0, 229, 255)),
                    );
                    ui.label(format!(
                        "{}: {:.1}s | {}: {:.1}s | {}: {:.1}s | {}: {}",
                        t.total_label,
                        result.total_duration_seconds,
                        t.speech_label,
                        result.speech_duration_seconds,
                        t.silence_label,
                        result.silence_duration_seconds,
                        t.cuts_label,
                        result.segments.len()
                    ));
                });

                // Draw visual timeline preview
                let (rect, _response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 28.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter();

                // Background (silence = dark red)
                painter.rect_filled(rect, 4.0, Color32::from_rgb(40, 20, 25));

                let total_dur = result.total_duration_seconds.max(0.001);
                for seg in &result.segments {
                    let x_start =
                        rect.left() + (seg.start_seconds / total_dur) as f32 * rect.width();
                    let x_end = rect.left() + (seg.end_seconds / total_dur) as f32 * rect.width();
                    let seg_rect = Rect::from_min_max(
                        Pos2::new(x_start, rect.top() + 2.0),
                        Pos2::new(x_end.max(x_start + 1.0), rect.bottom() - 2.0),
                    );
                    // Speech blocks = vibrant emerald green
                    painter.rect_filled(seg_rect, 2.0, Color32::from_rgb(16, 185, 129));
                }
            });

        ui.add_space(6.0);
    }
}
