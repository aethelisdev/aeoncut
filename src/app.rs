use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, TextureHandle, Vec2};

use crate::bridge::ResolveBridge;
use crate::engine::{load_audio_pcm_16k, VadDetector};
use crate::i18n::{get_strings, Language};
use crate::models::{AnalysisResult, CutMode, CutSettings, SelectedClipInfo, TimelineStatus};

pub struct AeonCutApp {
    bridge: ResolveBridge,
    status: Option<TimelineStatus>,
    selected_clip: Option<SelectedClipInfo>,
    settings: CutSettings,
    analysis_result: Option<AnalysisResult>,
    status_message: String,
    status_is_error: bool,
    timeline_suffix: String,
    last_status_check: Instant,
    pub lang: Language,
    logo_texture: Option<TextureHandle>,
}

impl Default for AeonCutApp {
    fn default() -> Self {
        let bridge = ResolveBridge::new();
        let status = bridge.get_status().ok();
        let selected_clip = bridge.get_selected_clip().ok();
        let lang = Language::English;
        let t = get_strings(lang);

        Self {
            bridge,
            status,
            selected_clip,
            settings: CutSettings::default(),
            analysis_result: None,
            status_message: t.ready_status.to_string(),
            status_is_error: false,
            timeline_suffix: "AeonCut".to_string(),
            last_status_check: Instant::now(),
            lang,
            logo_texture: None,
        }
    }
}

impl AeonCutApp {
    pub fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_none() {
            let img_bytes = include_bytes!("../assets/Aeoncut.png");
            if let Ok(loaded) = image::load_from_memory(img_bytes) {
                let rgba = loaded.to_rgba8();
                let (w, h) = rgba.dimensions();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &rgba,
                );
                self.logo_texture = Some(ctx.load_texture(
                    "aeoncut_logo",
                    color_image,
                    Default::default(),
                ));
            }
        }
    }

    pub fn refresh_resolve_status(&mut self) {
        match self.bridge.get_status() {
            Ok(stat) => {
                self.status = Some(stat);
                if self.status.as_ref().map_or(false, |s| s.connected) {
                    if let Ok(clip) = self.bridge.get_selected_clip() {
                        self.selected_clip = Some(clip);
                    }
                    self.status_message = "Synchronized with DaVinci Resolve.".to_string();
                    self.status_is_error = false;
                } else {
                    self.status_message = "DaVinci Resolve is not running or scripting API is disabled.".to_string();
                    self.status_is_error = true;
                }
            }
            Err(e) => {
                self.status_message = format!("Resolve error: {}", e);
                self.status_is_error = true;
            }
        }
    }

    pub fn run_analysis(&mut self) {
        if let Ok(clip) = self.bridge.get_selected_clip() {
            self.selected_clip = Some(clip);
        }

        let clip = match &self.selected_clip {
            Some(c) if c.has_selection => c,
            _ => {
                self.status_message = "Error: No clip selected in timeline. Please select a clip.".to_string();
                self.status_is_error = true;
                return;
            }
        };

        let file_path = match &clip.file_path {
            Some(p) if !p.empty_path() => PathBuf::from(p),
            _ => {
                self.status_message = "Error: Clip media file path could not be resolved.".to_string();
                self.status_is_error = true;
                return;
            }
        };

        self.status_message = format!(
            "Loading and analyzing audio: {}...",
            file_path.file_name().unwrap_or_default().to_string_lossy()
        );

        let start_time = Instant::now();
        match load_audio_pcm_16k(&file_path) {
            Ok(audio) => {
                let result = VadDetector::analyze(&audio, &self.settings, clip);
                let elapsed = start_time.elapsed().as_millis();
                self.status_message = format!(
                    "✓ Analysis completed in {} ms! Detected {} speech segments ({:.1}% speech, {:.1}% silence).",
                    elapsed,
                    result.segments.len(),
                    result.speech_percentage,
                    100.0 - result.speech_percentage
                );
                self.status_is_error = false;
                self.analysis_result = Some(result);
            }
            Err(e) => {
                self.status_message = format!("Audio decoding error: {}", e);
                self.status_is_error = true;
            }
        }
    }

    pub fn apply_cuts_to_resolve(&mut self) {
        let analysis = match &self.analysis_result {
            Some(a) if !a.segments.is_empty() => a,
            _ => {
                self.status_message = "Please analyze the clip first before applying cuts.".to_string();
                self.status_is_error = true;
                return;
            }
        };

        match self.settings.mode {
            CutMode::CleanTimeline => {
                match self.bridge.apply_clean_timeline(&analysis.segments, &self.timeline_suffix) {
                    Ok(new_name) => {
                        self.status_message = format!("✓ Success: Created and switched to clean timeline '{}'!", new_name);
                        self.status_is_error = false;
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to create clean timeline: {}", e);
                        self.status_is_error = true;
                    }
                }
            }
            CutMode::ActiveTimeline => {
                match self.bridge.apply_active_timeline(&analysis.segments) {
                    Ok(count) => {
                        self.status_message = format!("✓ Success: Appended {} cut speech segments directly to active timeline!", count);
                        self.status_is_error = false;
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to append to active timeline: {}", e);
                        self.status_is_error = true;
                    }
                }
            }
            CutMode::AddMarkers => {
                match self.bridge.apply_markers(&analysis.segments) {
                    Ok(count) => {
                        self.status_message = format!("✓ Success: Added {} speech markers to clip and timeline!", count);
                        self.status_is_error = false;
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to add markers: {}", e);
                        self.status_is_error = true;
                    }
                }
            }
        }
    }
}

trait EmptyPathExt {
    fn empty_path(&self) -> bool;
}

impl EmptyPathExt for String {
    fn empty_path(&self) -> bool {
        self.trim().is_empty()
    }
}

impl eframe::App for AeonCutApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ensure_textures(ui.ctx());
        let t = get_strings(self.lang);

        // Auto refresh status every 4 seconds if disconnected
        if self.last_status_check.elapsed().as_secs() > 4 {
            if self.status.as_ref().map_or(true, |s| !s.connected) {
                if let Ok(stat) = self.bridge.get_status() {
                    self.status = Some(stat);
                }
            }
            self.last_status_check = Instant::now();
        }

        let is_connected = self.status.as_ref().map_or(false, |s| s.connected);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

            // ================= HEADER BAR =================
            ui.horizontal(|ui| {
                // Official Aeon-Cut Logo
                if let Some(tex) = &self.logo_texture {
                    ui.add(egui::Image::new((tex.id(), Vec2::new(28.0, 28.0))));
                }

                ui.heading(
                    egui::RichText::new("AEON-CUT")
                        .size(22.0)
                        .strong()
                        .color(Color32::from_rgb(0, 229, 255)),
                );

                ui.label(
                    egui::RichText::new("v0.1.0")
                        .size(11.0)
                        .color(Color32::from_rgb(110, 110, 130)),
                );

                // Right side: Status Badge, Refresh, and Clean Language Selector
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language_selector")
                        .width(90.0)
                        .selected_text(self.lang.display_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.lang, Language::English, "English");
                            ui.selectable_value(&mut self.lang, Language::Turkish, "Türkçe");
                            ui.selectable_value(&mut self.lang, Language::Chinese, "简体中文");
                            ui.selectable_value(&mut self.lang, Language::Japanese, "日本語");
                        });

                    ui.separator();

                    if ui.button(t.refresh_btn).clicked() {
                        self.refresh_resolve_status();
                    }

                    if is_connected {
                        let project_name = self.status.as_ref()
                            .and_then(|s| s.project_name.as_deref())
                            .unwrap_or("Unknown Project");
                        ui.label(
                            egui::RichText::new(format!("{} ({})", t.connected, project_name))
                                .size(12.0)
                                .color(Color32::from_rgb(46, 213, 115)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(t.waiting_resolve)
                                .size(12.0)
                                .color(Color32::from_rgb(255, 171, 0)),
                        );
                    }
                });
            });

            // Subtitle
            ui.label(
                egui::RichText::new(t.app_subtitle)
                    .size(11.5)
                    .color(Color32::from_rgb(140, 140, 160)),
            );

            ui.separator();

            // ================= ACTIVE CLIP CARD =================
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(28, 28, 36))
                .stroke(Stroke::new(1.0, Color32::from_rgb(45, 45, 58)))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(t.selected_clip).strong().color(Color32::WHITE));
                        if let Some(clip) = &self.selected_clip {
                            if clip.has_selection {
                                let name = clip.clip_name.as_deref().unwrap_or("Untitled Clip");
                                let dur = clip.duration_frames.unwrap_or(0);
                                ui.label(egui::RichText::new(name).strong().color(Color32::from_rgb(0, 229, 255)));
                                ui.label(format!("({} frames, {:.2} FPS)", dur, clip.fps));
                            } else {
                                ui.label(egui::RichText::new(t.no_clip_selected).color(Color32::from_rgb(255, 171, 0)));
                            }
                        } else {
                            ui.label("No data available");
                        }
                    });

                    if let Some(clip) = &self.selected_clip {
                        if let Some(p) = &clip.file_path {
                            if !p.empty_path() {
                                ui.label(
                                    egui::RichText::new(format!("{} {}", t.file_label, p))
                                        .size(11.0)
                                        .color(Color32::from_rgb(140, 140, 155)),
                                );
                            }
                        }
                    }
                });

            // ================= SETTINGS SLIDERS =================
            egui::CollapsingHeader::new(egui::RichText::new(t.settings_header).strong())
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(t.silence_threshold);
                            ui.horizontal(|ui| {
                                ui.add(egui::Slider::new(&mut self.settings.threshold_db, -60.0..=-10.0).suffix(" dB"));
                                ui.label(egui::RichText::new(t.silence_threshold_hint).size(11.0).color(Color32::GRAY));
                            });
                            ui.end_row();

                            ui.label(t.min_silence_dur);
                            ui.horizontal(|ui| {
                                ui.add(egui::Slider::new(&mut self.settings.min_silence_ms, 100..=1500).suffix(" ms"));
                                ui.label(egui::RichText::new(t.min_silence_hint).size(11.0).color(Color32::GRAY));
                            });
                            ui.end_row();

                            ui.label(t.padding_start);
                            ui.horizontal(|ui| {
                                ui.add(egui::Slider::new(&mut self.settings.padding_start_ms, 0..=400).suffix(" ms"));
                                ui.label(egui::RichText::new(t.padding_start_hint).size(11.0).color(Color32::GRAY));
                            });
                            ui.end_row();

                            ui.label(t.padding_end);
                            ui.horizontal(|ui| {
                                ui.add(egui::Slider::new(&mut self.settings.padding_end_ms, 0..=400).suffix(" ms"));
                                ui.label(egui::RichText::new(t.padding_end_hint).size(11.0).color(Color32::GRAY));
                            });
                            ui.end_row();
                        });
                });

            // ================= MODE SELECTOR =================
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t.cut_mode).strong());
                ui.selectable_value(
                    &mut self.settings.mode,
                    CutMode::CleanTimeline,
                    t.mode_clean_timeline,
                );
                ui.selectable_value(
                    &mut self.settings.mode,
                    CutMode::ActiveTimeline,
                    t.mode_active_timeline,
                );
                ui.selectable_value(
                    &mut self.settings.mode,
                    CutMode::AddMarkers,
                    t.mode_markers,
                );
            });

            if self.settings.mode == CutMode::CleanTimeline {
                ui.horizontal(|ui| {
                    ui.label(t.suffix_label);
                    ui.text_edit_singleline(&mut self.timeline_suffix);
                });
            }

            ui.separator();

            // ================= ACTION BUTTONS =================
            ui.horizontal(|ui| {
                let analyze_btn = ui.add_sized(
                    [200.0, 38.0],
                    egui::Button::new(
                        egui::RichText::new(t.analyze_btn)
                            .size(15.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(37, 99, 235)),
                );

                if analyze_btn.clicked() {
                    self.run_analysis();
                }

                let can_apply = self.analysis_result.is_some() && is_connected;
                let apply_btn = ui.add_sized(
                    [260.0, 38.0],
                    egui::Button::new(
                        egui::RichText::new(t.apply_btn)
                            .size(15.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(if can_apply {
                        Color32::from_rgb(16, 185, 129)
                    } else {
                        Color32::from_rgb(70, 70, 80)
                    }),
                );

                if apply_btn.clicked() && can_apply {
                    self.apply_cuts_to_resolve();
                }
            });

            // ================= ANALYSIS & VISUAL TIMELINE PREVIEW =================
            if let Some(result) = &self.analysis_result {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgb(24, 24, 30))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(45, 45, 60)))
                    .corner_radius(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t.summary_header).strong().color(Color32::from_rgb(0, 229, 255)));
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
                        let (rect, _response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::hover());
                        let painter = ui.painter();

                        // Background (silence = dark red)
                        painter.rect_filled(rect, 4.0, Color32::from_rgb(40, 20, 25));

                        let total_dur = result.total_duration_seconds.max(0.001);
                        for seg in &result.segments {
                            let x_start = rect.left() + (seg.start_seconds / total_dur) as f32 * rect.width();
                            let x_end = rect.left() + (seg.end_seconds / total_dur) as f32 * rect.width();
                            let seg_rect = Rect::from_min_max(
                                Pos2::new(x_start, rect.top() + 2.0),
                                Pos2::new(x_end.max(x_start + 1.0), rect.bottom() - 2.0),
                            );
                            // Speech blocks = vibrant emerald green
                            painter.rect_filled(seg_rect, 2.0, Color32::from_rgb(16, 185, 129));
                        }
                    });
            }

            ui.add_space(8.0);

            // ================= STATUS MESSAGE BANNER =================
            let banner_color = if self.status_is_error {
                Color32::from_rgb(239, 68, 68)
            } else {
                Color32::from_rgb(16, 185, 129)
            };

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&self.status_message)
                        .color(banner_color)
                        .size(13.0),
                );
            });
        });
    }
}
