use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, TextureHandle, Vec2};

use crate::bridge::ResolveBridge;
use crate::engine::{load_audio_pcm_16k, VadDetector};
use crate::i18n::{get_strings, Language};
use crate::models::{AnalysisResult, CutMode, CutSettings, SelectedClipInfo, TimelineStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Ready,
    Vad,
    Success,
    Warn,
    Error,
}

impl LogLevel {
    pub fn tag(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Ready => "READY",
            LogLevel::Vad => "VAD",
            LogLevel::Success => "SUCCESS",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            LogLevel::Info => Color32::from_rgb(161, 161, 170),   // Matte light gray (#a1a1aa)
            LogLevel::Ready => Color32::from_rgb(74, 222, 128),   // Soft mint green (#4ade80)
            LogLevel::Vad => Color32::from_rgb(192, 132, 252),     // Soft violet (#c084fc)
            LogLevel::Success => Color32::from_rgb(74, 222, 128), // Soft mint green (#4ade80)
            LogLevel::Warn => Color32::from_rgb(245, 158, 11),    // Amber (#f59e0b)
            LogLevel::Error => Color32::from_rgb(248, 113, 113),  // Soft rose red (#f87171)
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

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
    pub logs: Vec<LogEntry>,
}

impl Default for AeonCutApp {
    fn default() -> Self {
        let bridge = ResolveBridge::new();
        let status = bridge.get_status().ok();
        let selected_clip = bridge.get_selected_clip().ok();
        let lang = Language::English;
        let t = get_strings(lang);

        let mut app = Self {
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
            logs: Vec::new(),
        };

        app.add_log(LogLevel::Info, "Engine initialized. DaVinci Resolve 21 bridge active.");
        if app.status.as_ref().map_or(false, |s| s.connected) {
            let proj = app.status.as_ref().and_then(|s| s.project_name.as_deref()).unwrap_or("Active Project");
            app.add_log(LogLevel::Ready, format!("Connected to DaVinci Resolve project: '{}'.", proj));
        } else {
            app.add_log(LogLevel::Warn, "Waiting for DaVinci Resolve connection (ensure scripting is enabled).");
        }

        app
    }
}

impl AeonCutApp {
    pub fn add_log(&mut self, level: LogLevel, message: impl Into<String>) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(LogEntry {
            timestamp,
            level,
            message: message.into(),
        });
        if self.logs.len() > 250 {
            self.logs.remove(0);
        }
    }

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
                    let proj = self.status.as_ref().and_then(|s| s.project_name.as_deref()).unwrap_or("Active Project");
                    self.status_message = "Synchronized with DaVinci Resolve.".to_string();
                    self.status_is_error = false;
                    self.add_log(LogLevel::Info, format!("Synchronized with Resolve project: '{}'.", proj));
                } else {
                    self.status_message = "DaVinci Resolve is not running or scripting API is disabled.".to_string();
                    self.status_is_error = true;
                    self.add_log(LogLevel::Warn, "DaVinci Resolve is not running or scripting API is disabled.");
                }
            }
            Err(e) => {
                self.status_message = format!("Resolve error: {}", e);
                self.status_is_error = true;
                self.add_log(LogLevel::Error, format!("Resolve connection error: {}", e));
            }
        }
    }

    pub fn run_analysis(&mut self) {
        if let Ok(clip) = self.bridge.get_selected_clip() {
            self.selected_clip = Some(clip);
        }

        let clip = match &self.selected_clip {
            Some(c) if c.has_selection => c.clone(),
            _ => {
                let err = "No clip selected in timeline. Please select a clip.";
                self.status_message = format!("Error: {}", err);
                self.status_is_error = true;
                self.add_log(LogLevel::Warn, err);
                return;
            }
        };

        let file_path = match &clip.file_path {
            Some(p) if !p.empty_path() => PathBuf::from(p),
            _ => {
                let err = "Clip media file path could not be resolved from Resolve.";
                self.status_message = format!("Error: {}", err);
                self.status_is_error = true;
                self.add_log(LogLevel::Error, err);
                return;
            }
        };

        let clip_name = clip.clip_name.as_deref().unwrap_or("Selected Clip").to_string();
        self.add_log(LogLevel::Vad, format!("Analyzing audio stream for '{}'...", clip_name));

        let start_time = Instant::now();
        match load_audio_pcm_16k(&file_path) {
            Ok(audio) => {
                let result = VadDetector::analyze(&audio, &self.settings, &clip);
                let elapsed = start_time.elapsed().as_millis();
                let seg_count = result.segments.len();
                let speech_pct = result.speech_percentage;
                let silence_pct = 100.0 - speech_pct;

                self.status_message = format!(
                    "✓ Analysis completed in {} ms! Detected {} speech segments ({:.1}% speech, {:.1}% silence).",
                    elapsed, seg_count, speech_pct, silence_pct
                );
                self.status_is_error = false;
                self.analysis_result = Some(result);

                self.add_log(
                    LogLevel::Vad,
                    format!("Analysis completed in {} ms: found {} speech segments.", elapsed, seg_count),
                );
                self.add_log(
                    LogLevel::Ready,
                    format!("Speech: {:.1}% | Silences removed: {:.1}%. Ready to apply cuts.", speech_pct, silence_pct),
                );
            }
            Err(e) => {
                let err_msg = format!("Audio decoding error: {}", e);
                self.status_message = err_msg.clone();
                self.status_is_error = true;
                self.add_log(LogLevel::Error, err_msg);
            }
        }
    }

    pub fn apply_cuts_to_resolve(&mut self) {
        let segments = match &self.analysis_result {
            Some(a) if !a.segments.is_empty() => a.segments.clone(),
            _ => {
                let err = "Please analyze the clip first before applying cuts.";
                self.status_message = err.to_string();
                self.status_is_error = true;
                self.add_log(LogLevel::Warn, err);
                return;
            }
        };

        match self.settings.mode {
            CutMode::CleanTimeline => {
                self.add_log(LogLevel::Info, format!("Creating clean timeline with suffix '{}'...", self.timeline_suffix));
                match self.bridge.apply_clean_timeline(&segments, &self.timeline_suffix) {
                    Ok(new_name) => {
                        self.status_message = format!("✓ Success: Created and switched to clean timeline '{}'!", new_name);
                        self.status_is_error = false;
                        self.add_log(LogLevel::Success, format!("Cuts applied to clean timeline: '{}'.", new_name));
                    }
                    Err(e) => {
                        let err = format!("Failed to create clean timeline: {}", e);
                        self.status_message = err.clone();
                        self.status_is_error = true;
                        self.add_log(LogLevel::Error, err);
                    }
                }
            }
            CutMode::ActiveTimeline => {
                self.add_log(LogLevel::Info, "Appending cut slices directly to active timeline...");
                match self.bridge.apply_active_timeline(&segments) {
                    Ok(count) => {
                        self.status_message = format!("✓ Success: Appended {} cut speech segments directly to active timeline!", count);
                        self.status_is_error = false;
                        self.add_log(LogLevel::Success, format!("Appended {} cut speech segments to active timeline.", count));
                    }
                    Err(e) => {
                        let err = format!("Failed to append to active timeline: {}", e);
                        self.status_message = err.clone();
                        self.status_is_error = true;
                        self.add_log(LogLevel::Error, err);
                    }
                }
            }
            CutMode::AddMarkers => {
                self.add_log(LogLevel::Info, "Placing speech cut markers on clip and timeline...");
                match self.bridge.apply_markers(&segments) {
                    Ok(count) => {
                        self.status_message = format!("✓ Success: Added {} speech markers to clip and timeline!", count);
                        self.status_is_error = false;
                        self.add_log(LogLevel::Success, format!("Added {} speech markers across clip and timeline.", count));
                    }
                    Err(e) => {
                        let err = format!("Failed to add markers: {}", e);
                        self.status_message = err.clone();
                        self.status_is_error = true;
                        self.add_log(LogLevel::Error, err);
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

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(17, 16, 22)))
            .show(ui, |ui| {
                let screen_w = ui.max_rect().width();
                let header_height = 56.0;
                let header_rect = Rect::from_min_max(
                    Pos2::new(0.0, 0.0),
                    Pos2::new(screen_w, header_height),
                );

                // 1. Paint subtle blackish-purple hardware gradient FIRST (behind widgets)
                let mut bg_mesh = egui::Mesh::default();
                let c_tl = Color32::from_rgb(36, 18, 48); // refined obsidian violet
                let c_tr = Color32::from_rgb(22, 14, 32); // deep charcoal purple
                let c_br = Color32::from_rgb(15, 14, 20); // dark obsidian charcoal
                let c_bl = Color32::from_rgb(22, 14, 30); // subtle dark violet

                let idx = bg_mesh.vertices.len() as u32;
                bg_mesh.vertices.push(egui::epaint::Vertex { pos: header_rect.left_top(), uv: Pos2::ZERO, color: c_tl });
                bg_mesh.vertices.push(egui::epaint::Vertex { pos: header_rect.right_top(), uv: Pos2::ZERO, color: c_tr });
                bg_mesh.vertices.push(egui::epaint::Vertex { pos: header_rect.right_bottom(), uv: Pos2::ZERO, color: c_br });
                bg_mesh.vertices.push(egui::epaint::Vertex { pos: header_rect.left_bottom(), uv: Pos2::ZERO, color: c_bl });
                bg_mesh.add_triangle(idx, idx + 1, idx + 2);
                bg_mesh.add_triangle(idx, idx + 2, idx + 3);
                ui.painter().add(egui::Shape::mesh(bg_mesh));

                // 1px hairline gradient accent divider at bottom of header
                let mut border_mesh = egui::Mesh::default();
                let b_tl = Pos2::new(0.0, header_rect.max.y - 1.0);
                let b_tr = Pos2::new(screen_w, header_rect.max.y - 1.0);
                let b_br = Pos2::new(screen_w, header_rect.max.y);
                let b_bl = Pos2::new(0.0, header_rect.max.y);

                let bc_left = Color32::from_rgba_unmultiplied(170, 50, 220, 85);
                let bc_right = Color32::from_rgba_unmultiplied(50, 38, 70, 40);

                let b_idx = border_mesh.vertices.len() as u32;
                border_mesh.vertices.push(egui::epaint::Vertex { pos: b_tl, uv: Pos2::ZERO, color: bc_left });
                border_mesh.vertices.push(egui::epaint::Vertex { pos: b_tr, uv: Pos2::ZERO, color: bc_right });
                border_mesh.vertices.push(egui::epaint::Vertex { pos: b_br, uv: Pos2::ZERO, color: bc_right });
                border_mesh.vertices.push(egui::epaint::Vertex { pos: b_bl, uv: Pos2::ZERO, color: bc_left });
                border_mesh.add_triangle(b_idx, b_idx + 1, b_idx + 2);
                border_mesh.add_triangle(b_idx, b_idx + 2, b_idx + 3);
                ui.painter().add(egui::Shape::mesh(border_mesh));

                // 2. ================= TOP HEADER WIDGETS (Drawn ON TOP of gradient) =================
                egui::Frame::new()
                    .inner_margin(egui::Margin { left: 18, right: 18, top: 10, bottom: 10 })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Left: Logo + App Titles
                            ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
                            if let Some(tex) = &self.logo_texture {
                                ui.add(egui::Image::new((tex.id(), Vec2::new(32.0, 32.0))));
                            }

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                                ui.horizontal(|ui| {
                                    ui.heading(
                                        egui::RichText::new("AEON-CUT")
                                            .size(20.0)
                                            .strong()
                                            .color(Color32::from_rgb(245, 240, 255)),
                                    );
                                    ui.label(
                                        egui::RichText::new("v0.1.0")
                                            .size(11.0)
                                            .color(Color32::from_rgb(155, 130, 185)),
                                    );
                                });

                                ui.label(
                                    egui::RichText::new(t.app_subtitle)
                                        .size(11.0)
                                        .color(Color32::from_rgb(150, 145, 170)),
                                );
                            });

                            // Right side: Status Badge, Refresh Button, Language Selector
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                                // Clean Language Selector ComboBox
                                egui::ComboBox::from_id_salt("language_selector")
                                    .width(90.0)
                                    .selected_text(self.lang.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.lang, Language::English, "English");
                                        ui.selectable_value(&mut self.lang, Language::Turkish, "Türkçe");
                                        ui.selectable_value(&mut self.lang, Language::Chinese, "简体中文");
                                        ui.selectable_value(&mut self.lang, Language::Japanese, "日本語");
                                    });

                                // Refresh Button (styled with subtle purple tone)
                                let refresh_btn = egui::Button::new(
                                    egui::RichText::new(t.refresh_btn)
                                        .size(12.0)
                                        .color(Color32::from_rgb(220, 215, 235))
                                )
                                .fill(Color32::from_rgb(32, 22, 44))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(68, 50, 92)))
                                .corner_radius(6.0);

                                if ui.add(refresh_btn).clicked() {
                                    self.refresh_resolve_status();
                                }

                                // Connection Status Pill Badge
                                let (bg_color, stroke_color, dot_color, dot_char, text_color, status_text) = if is_connected {
                                    let project_name = self.status.as_ref()
                                        .and_then(|s| s.project_name.as_deref())
                                        .unwrap_or("Unknown Project");
                                    (
                                        Color32::from_rgb(18, 30, 24),
                                        Color32::from_rgb(38, 80, 52),
                                        Color32::from_rgb(46, 213, 115),
                                        "●",
                                        Color32::from_rgb(180, 245, 205),
                                        format!("{} ({})", t.connected, project_name),
                                    )
                                } else {
                                    (
                                        Color32::from_rgb(32, 26, 20),
                                        Color32::from_rgb(85, 65, 32),
                                        Color32::from_rgb(255, 171, 0),
                                        "○",
                                        Color32::from_rgb(255, 205, 120),
                                        t.waiting_resolve.to_string(),
                                    )
                                };

                                egui::Frame::new()
                                    .fill(bg_color)
                                    .stroke(Stroke::new(1.0, stroke_color))
                                    .corner_radius(12.0)
                                    .inner_margin(egui::Margin::symmetric(10, 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                                            ui.label(egui::RichText::new(dot_char).size(10.0).color(dot_color));
                                            ui.label(egui::RichText::new(status_text).size(11.5).color(text_color));
                                        });
                                    });
                            });
                        });
                    });

                // Body content below header
                egui::Frame::new()
                    .inner_margin(egui::Margin { left: 18, right: 18, top: 18, bottom: 14 })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 12.0);

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
                                    ui.label(egui::RichText::new(t.selected_clip).strong().color(Color32::from_rgb(220, 215, 235)));
                                    if let Some(clip) = &self.selected_clip {
                                        if clip.has_selection {
                                            let name = clip.clip_name.as_deref().unwrap_or("Untitled Clip");
                                            let dur = clip.duration_frames.unwrap_or(0);
                                            ui.label(egui::RichText::new(name).strong().color(Color32::from_rgb(0, 229, 255)));
                                            ui.label(
                                                egui::RichText::new(format!("({} frames, {:.2} FPS)", dur, clip.fps))
                                                    .size(11.5)
                                                    .color(Color32::from_rgb(150, 145, 165)),
                                            );
                                        } else {
                                            ui.label(egui::RichText::new(t.no_clip_selected).color(Color32::from_rgb(255, 171, 0)));
                                        }
                                    } else {
                                        ui.label(egui::RichText::new("No data available").color(Color32::from_rgb(140, 135, 150)));
                                    }
                                });

                                if let Some(clip) = &self.selected_clip {
                                    if let Some(p) = &clip.file_path {
                                        if !p.empty_path() {
                                            ui.label(
                                                egui::RichText::new(format!("{} {}", t.file_label, p))
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(140, 135, 155)),
                                            );
                                        }
                                    }
                                }

                                ui.separator();

                                // 2. SETTINGS SLIDERS (Purple Knobs)
                                ui.scope(|ui| {
                                    let widgets = &mut ui.visuals_mut().widgets;
                                    // Inactive slider handle (çubuk tutucusu):
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

                                    egui::CollapsingHeader::new(egui::RichText::new(t.settings_header).strong().color(Color32::from_rgb(225, 218, 240)))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            egui::Grid::new("settings_grid")
                                                .num_columns(2)
                                                .spacing([20.0, 10.0])
                                                .show(ui, |ui| {
                                                    ui.label(t.silence_threshold);
                                                    ui.horizontal(|ui| {
                                                        ui.add(egui::Slider::new(&mut self.settings.threshold_db, -60.0..=-10.0).suffix(" dB"));
                                                        ui.label(egui::RichText::new(t.silence_threshold_hint).size(11.0).color(Color32::from_rgb(140, 135, 155)));
                                                    });
                                                    ui.end_row();

                                                    ui.label(t.min_silence_dur);
                                                    ui.horizontal(|ui| {
                                                        ui.add(egui::Slider::new(&mut self.settings.min_silence_ms, 100..=1500).suffix(" ms"));
                                                        ui.label(egui::RichText::new(t.min_silence_hint).size(11.0).color(Color32::from_rgb(140, 135, 155)));
                                                    });
                                                    ui.end_row();

                                                    ui.label(t.padding_start);
                                                    ui.horizontal(|ui| {
                                                        ui.add(egui::Slider::new(&mut self.settings.padding_start_ms, 0..=400).suffix(" ms"));
                                                        ui.label(egui::RichText::new(t.padding_start_hint).size(11.0).color(Color32::from_rgb(140, 135, 155)));
                                                    });
                                                    ui.end_row();

                                                    ui.label(t.padding_end);
                                                    ui.horizontal(|ui| {
                                                        ui.add(egui::Slider::new(&mut self.settings.padding_end_ms, 0..=400).suffix(" ms"));
                                                        ui.label(egui::RichText::new(t.padding_end_hint).size(11.0).color(Color32::from_rgb(140, 135, 155)));
                                                    });
                                                    ui.end_row();
                                                });
                                        });
                                });

                                ui.separator();

                                // 3. CUT MODE SELECTOR (Dark Purple Themed)
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(t.cut_mode).strong().color(Color32::from_rgb(225, 218, 240)));
                                    let modes = [
                                        (CutMode::CleanTimeline, t.mode_clean_timeline),
                                        (CutMode::ActiveTimeline, t.mode_active_timeline),
                                        (CutMode::AddMarkers, t.mode_markers),
                                    ];
                                    for (mode, label) in modes {
                                        let is_selected = self.settings.mode == mode;
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

                                        let btn = egui::Button::new(
                                            egui::RichText::new(label)
                                                .size(12.5)
                                                .color(text_color)
                                        )
                                        .fill(fill)
                                        .stroke(stroke)
                                        .corner_radius(6.0);

                                        if ui.add(btn).clicked() {
                                            self.settings.mode = mode;
                                        }
                                    }
                                });

                                if self.settings.mode == CutMode::CleanTimeline {
                                    ui.horizontal(|ui| {
                                        ui.label(t.suffix_label);
                                        ui.text_edit_singleline(&mut self.timeline_suffix);
                                    });
                                }
                            });

                        // ================= ACTION BUTTONS (Dark Purple Theme) =================
                        ui.horizontal(|ui| {
                let analyze_btn = ui.add_sized(
                    [200.0, 38.0],
                    egui::Button::new(
                        egui::RichText::new(t.analyze_btn)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(245, 240, 255)),
                    )
                    .fill(Color32::from_rgb(54, 26, 78)) // karanlık mor zemin
                    .stroke(Stroke::new(1.2, Color32::from_rgb(170, 52, 225))) // zarif neon mor kontur
                    .corner_radius(8.0),
                );

                if analyze_btn.clicked() {
                    self.run_analysis();
                }

                let can_apply = self.analysis_result.is_some() && is_connected;
                let (apply_fill, apply_stroke, apply_text) = if can_apply {
                    (
                        Color32::from_rgb(20, 85, 52), // zümrüt hazır
                        Stroke::new(1.2, Color32::from_rgb(46, 213, 115)),
                        Color32::from_rgb(240, 255, 245),
                    )
                } else {
                    (
                        Color32::from_rgb(30, 24, 38), // inaktif koyu mor/antrasit
                        Stroke::new(1.0, Color32::from_rgb(56, 46, 70)),
                        Color32::from_rgb(120, 112, 132),
                    )
                };

                let apply_btn = ui.add_sized(
                    [260.0, 38.0],
                    egui::Button::new(
                        egui::RichText::new(t.apply_btn)
                            .size(15.0)
                            .strong()
                            .color(apply_text),
                    )
                    .fill(apply_fill)
                    .stroke(apply_stroke)
                    .corner_radius(8.0),
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

            ui.add_space(6.0);

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
                                    .color(Color32::from_rgb(160, 155, 175))
                            )
                            .fill(Color32::from_rgb(22, 20, 28))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(48, 42, 60)))
                            .corner_radius(4.0);

                            if ui.add(clear_btn).clicked() {
                                self.logs.clear();
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
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 3.0);
                            for entry in &self.logs {
                                ui.horizontal(|ui| {
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
        });
    });
}
}
