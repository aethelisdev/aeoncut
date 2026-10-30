use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app::AeonCutApp;
use crate::app::state::EmptyPathExt;
use crate::i18n::get_strings;
use crate::models::CutMode;

pub fn render(app: &mut AeonCutApp, ui: &mut egui::Ui) {
    let t = get_strings(app.lang);

    // ================= MIDDLE WORKSPACE INNER CARD PANEL =================
    egui::Frame::new()
        .fill(Color32::from_rgb(22, 19, 28))
        .stroke(Stroke::new(1.0, Color32::from_rgb(48, 38, 62)))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

            // 1. ACTIVE CLIP STATUS ROW
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(t.selected_clip)
                        .strong()
                        .color(Color32::from_rgb(220, 215, 235)),
                );
                if let Some(clip) = &app.selected_clip {
                    if clip.has_selection {
                        let name = clip.clip_name.as_deref().unwrap_or("Untitled Clip");
                        let dur = clip.duration_frames.unwrap_or(0);
                        ui.label(
                            egui::RichText::new(name)
                                .strong()
                                .color(Color32::from_rgb(0, 229, 255)),
                        );
                        ui.label(
                            egui::RichText::new(format!("({} frames, {:.2} FPS)", dur, clip.fps))
                                .size(11.5)
                                .color(Color32::from_rgb(150, 145, 165)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(t.no_clip_selected)
                                .color(Color32::from_rgb(255, 171, 0)),
                        );
                    }
                } else {
                    ui.label(
                        egui::RichText::new("No data available")
                            .color(Color32::from_rgb(140, 135, 150)),
                    );
                }
            });

            if let Some(clip) = &app.selected_clip
                && let Some(p) = &clip.file_path
                && !p.empty_path()
            {
                ui.label(
                    egui::RichText::new(format!("{} {}", t.file_label, p))
                        .size(11.0)
                        .color(Color32::from_rgb(140, 135, 155)),
                );
            }

            ui.separator();

            // 2. SETTINGS SLIDERS (Purple Knobs)
            ui.scope(|ui| {
                let widgets = &mut ui.visuals_mut().widgets;
                // Inactive slider handle:
                widgets.inactive.bg_fill = Color32::from_rgb(58, 28, 84);
                widgets.inactive.fg_stroke = Stroke::new(1.5, Color32::from_rgb(170, 52, 224));
                widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(62, 40, 78));

                // Hovered slider handle:
                widgets.hovered.bg_fill = Color32::from_rgb(78, 36, 112);
                widgets.hovered.fg_stroke = Stroke::new(1.8, Color32::from_rgb(215, 75, 255));
                widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(105, 52, 145));

                // Active / Dragging slider handle:
                widgets.active.bg_fill = Color32::from_rgb(98, 44, 140);
                widgets.active.fg_stroke = Stroke::new(2.0, Color32::from_rgb(255, 110, 255));
                widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(180, 60, 240));

                egui::CollapsingHeader::new(
                    egui::RichText::new(t.settings_header)
                        .strong()
                        .color(Color32::from_rgb(225, 218, 240)),
                )
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(t.silence_threshold);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut app.settings.threshold_db,
                                        -60.0..=-10.0,
                                    )
                                    .suffix(" dB"),
                                );
                                ui.label(
                                    egui::RichText::new(t.silence_threshold_hint)
                                        .size(11.0)
                                        .color(Color32::from_rgb(140, 135, 155)),
                                );
                            });
                            ui.end_row();

                            ui.label(t.min_silence_dur);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Slider::new(&mut app.settings.min_silence_ms, 100..=1500)
                                        .suffix(" ms"),
                                );
                                ui.label(
                                    egui::RichText::new(t.min_silence_hint)
                                        .size(11.0)
                                        .color(Color32::from_rgb(140, 135, 155)),
                                );
                            });
                            ui.end_row();

                            ui.label(t.padding_start);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Slider::new(&mut app.settings.padding_start_ms, 0..=400)
                                        .suffix(" ms"),
                                );
                                ui.label(
                                    egui::RichText::new(t.padding_start_hint)
                                        .size(11.0)
                                        .color(Color32::from_rgb(140, 135, 155)),
                                );
                            });
                            ui.end_row();

                            ui.label(t.padding_end);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Slider::new(&mut app.settings.padding_end_ms, 0..=400)
                                        .suffix(" ms"),
                                );
                                ui.label(
                                    egui::RichText::new(t.padding_end_hint)
                                        .size(11.0)
                                        .color(Color32::from_rgb(140, 135, 155)),
                                );
                            });
                            ui.end_row();
                        });
                });
            });

            ui.separator();

            // 3. CUT MODE SELECTOR (Dark Purple Themed)
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(t.cut_mode)
                        .strong()
                        .color(Color32::from_rgb(225, 218, 240)),
                );
                let modes = [
                    (CutMode::CleanTimeline, t.mode_clean_timeline),
                    (CutMode::ActiveTimeline, t.mode_active_timeline),
                    (CutMode::AddMarkers, t.mode_markers),
                ];
                for (mode, label) in modes {
                    let is_selected = app.settings.mode == mode;
                    let (fill, stroke, text_color) = if is_selected {
                        (
                            Color32::from_rgb(54, 26, 78), // karanlık mor aktif buton
                            Stroke::new(1.2, Color32::from_rgb(170, 52, 225)), // mor kontur
                            Color32::from_rgb(245, 238, 255),
                        )
                    } else {
                        (
                            Color32::from_rgb(26, 22, 32),
                            Stroke::new(1.0, Color32::from_rgb(52, 45, 64)),
                            Color32::from_rgb(165, 160, 180),
                        )
                    };

                    let btn =
                        egui::Button::new(egui::RichText::new(label).size(12.5).color(text_color))
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(6.0);

                    if ui.add(btn).clicked() {
                        app.settings.mode = mode;
                    }
                }
            });

            if app.settings.mode == CutMode::CleanTimeline {
                ui.horizontal(|ui| {
                    ui.label(t.suffix_label);
                    ui.text_edit_singleline(&mut app.timeline_suffix);
                });
            }
        });
}
