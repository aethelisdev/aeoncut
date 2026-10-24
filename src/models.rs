use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStatus {
    pub connected: bool,
    pub version: Option<String>,
    pub project_name: Option<String>,
    pub timeline_name: Option<String>,
    pub timeline_fps: Option<f64>,
    pub timeline_start_frame: Option<i64>,
    pub timeline_end_frame: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedClipInfo {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub has_selection: bool,
    pub clip_name: Option<String>,
    pub file_path: Option<String>,
    #[serde(default = "default_fps")]
    pub fps: f64,
    pub timeline_start_frame: Option<i64>,
    pub timeline_end_frame: Option<i64>,
    pub duration_frames: Option<i64>,
    pub source_start_frame: Option<i64>,
    pub source_end_frame: Option<i64>,
    pub timeline_name: Option<String>,
    pub error: Option<String>,
}

fn default_fps() -> f64 {
    24.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CutMode {
    CleanTimeline,
    ActiveTimeline,
    AddMarkers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutSettings {
    pub threshold_db: f32,
    pub min_silence_ms: u32,
    pub min_speech_ms: u32,
    pub padding_start_ms: u32,
    pub padding_end_ms: u32,
    pub mode: CutMode,
}

impl Default for CutSettings {
    fn default() -> Self {
        Self {
            threshold_db: -32.0,
            min_silence_ms: 350,
            min_speech_ms: 150,
            padding_start_ms: 120,
            padding_end_ms: 150,
            mode: CutMode::CleanTimeline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub source_start_frame: i64,
    pub source_end_frame: i64,
    pub timeline_start_frame: i64,
    pub timeline_end_frame: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub total_duration_seconds: f64,
    pub speech_duration_seconds: f64,
    pub silence_duration_seconds: f64,
    pub speech_percentage: f32,
    pub segments: Vec<SpeechSegment>,
}
