use std::time::Instant;

use eframe::egui::{self, TextureHandle};

use crate::app::state::{LogEntry, LogLevel};
use crate::app::ui;
use crate::bridge::ResolveBridge;
use crate::i18n::{get_strings, Language};
use crate::models::{AnalysisResult, CutSettings, SelectedClipInfo, TimelineStatus};

pub struct AeonCutApp {
    pub(crate) bridge: ResolveBridge,
    pub(crate) status: Option<TimelineStatus>,
    pub(crate) selected_clip: Option<SelectedClipInfo>,
    pub(crate) settings: CutSettings,
    pub(crate) analysis_result: Option<AnalysisResult>,
    pub(crate) status_message: String,
    pub(crate) status_is_error: bool,
    pub(crate) timeline_suffix: String,
    pub(crate) last_status_check: Instant,
    pub lang: Language,
    pub always_on_top: bool,
    pub(crate) logo_texture: Option<TextureHandle>,
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
            always_on_top: false,
            logo_texture: None,
            logs: Vec::new(),
        };

        app.add_log(
            LogLevel::Info,
            "Engine initialized. DaVinci Resolve 21 bridge active.",
        );
        if app.status.as_ref().is_some_and(|s| s.connected) {
            let proj = app
                .status
                .as_ref()
                .and_then(|s| s.project_name.as_deref())
                .unwrap_or("Active Project");
            app.add_log(
                LogLevel::Ready,
                format!("Connected to DaVinci Resolve project: '{}'.", proj),
            );
        } else {
            app.add_log(
                LogLevel::Warn,
                "Waiting for DaVinci Resolve connection (ensure scripting is enabled).",
            );
        }

        app
    }
}

impl eframe::App for AeonCutApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::render_central_panel(self, ui);
    }
}
