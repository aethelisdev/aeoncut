use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{self, Color32};

use crate::app::AeonCutApp;
use crate::engine::{VadDetector, load_audio_pcm_16k};
use crate::models::CutMode;

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
            LogLevel::Info => Color32::from_rgb(161, 161, 170), // Matte light gray (#a1a1aa)
            LogLevel::Ready => Color32::from_rgb(74, 222, 128), // Soft mint green (#4ade80)
            LogLevel::Vad => Color32::from_rgb(192, 132, 252),  // Soft violet (#c084fc)
            LogLevel::Success => Color32::from_rgb(74, 222, 128), // Soft mint green (#4ade80)
            LogLevel::Warn => Color32::from_rgb(245, 158, 11),  // Amber (#f59e0b)
            LogLevel::Error => Color32::from_rgb(248, 113, 113), // Soft rose red (#f87171)
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

pub(crate) trait EmptyPathExt {
    fn empty_path(&self) -> bool;
}

impl EmptyPathExt for String {
    fn empty_path(&self) -> bool {
        self.trim().is_empty()
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
            let img_bytes = include_bytes!("../../assets/Aeoncut.png");
            if let Ok(loaded) = image::load_from_memory(img_bytes) {
                let rgba = loaded.to_rgba8();
                let (w, h) = rgba.dimensions();
                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                self.logo_texture =
                    Some(ctx.load_texture("aeoncut_logo", color_image, Default::default()));
            }
        }
    }

    pub fn refresh_resolve_status(&mut self) {
        match self.bridge.get_status() {
            Ok(stat) => {
                self.status = Some(stat);
                if self.status.as_ref().is_some_and(|s| s.connected) {
                    if let Ok(clip) = self.bridge.get_selected_clip() {
                        self.selected_clip = Some(clip);
                    }
                    let proj = self
                        .status
                        .as_ref()
                        .and_then(|s| s.project_name.as_deref())
                        .unwrap_or("Active Project");
                    self.status_message = "Synchronized with DaVinci Resolve.".to_string();
                    self.status_is_error = false;
                    self.add_log(
                        LogLevel::Info,
                        format!("Synchronized with Resolve project: '{}'.", proj),
                    );
                } else {
                    self.status_message =
                        "DaVinci Resolve is not running or scripting API is disabled.".to_string();
                    self.status_is_error = true;
                    self.add_log(
                        LogLevel::Warn,
                        "DaVinci Resolve is not running or scripting API is disabled.",
                    );
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

        let clip_name = clip
            .clip_name
            .as_deref()
            .unwrap_or("Selected Clip")
            .to_string();
        self.add_log(
            LogLevel::Vad,
            format!("Analyzing audio stream for '{}'...", clip_name),
        );

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
                    format!(
                        "Analysis completed in {} ms: found {} speech segments.",
                        elapsed, seg_count
                    ),
                );
                self.add_log(
                    LogLevel::Ready,
                    format!(
                        "Speech: {:.1}% | Silences removed: {:.1}%. Ready to apply cuts.",
                        speech_pct, silence_pct
                    ),
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
                self.add_log(
                    LogLevel::Info,
                    format!(
                        "Creating clean timeline with suffix '{}'...",
                        self.timeline_suffix
                    ),
                );
                match self
                    .bridge
                    .apply_clean_timeline(&segments, &self.timeline_suffix)
                {
                    Ok(new_name) => {
                        self.status_message = format!(
                            "✓ Success: Created and switched to clean timeline '{}'!",
                            new_name
                        );
                        self.status_is_error = false;
                        self.add_log(
                            LogLevel::Success,
                            format!("Cuts applied to clean timeline: '{}'.", new_name),
                        );
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
                self.add_log(
                    LogLevel::Info,
                    "Appending cut slices directly to active timeline...",
                );
                match self.bridge.apply_active_timeline(&segments) {
                    Ok(count) => {
                        self.status_message = format!(
                            "✓ Success: Appended {} cut speech segments directly to active timeline!",
                            count
                        );
                        self.status_is_error = false;
                        self.add_log(
                            LogLevel::Success,
                            format!("Appended {} cut speech segments to active timeline.", count),
                        );
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
                self.add_log(
                    LogLevel::Info,
                    "Placing speech cut markers on clip and timeline...",
                );
                match self.bridge.apply_markers(&segments) {
                    Ok(count) => {
                        self.status_message = format!(
                            "✓ Success: Added {} speech markers to clip and timeline!",
                            count
                        );
                        self.status_is_error = false;
                        self.add_log(
                            LogLevel::Success,
                            format!("Added {} speech markers across clip and timeline.", count),
                        );
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
