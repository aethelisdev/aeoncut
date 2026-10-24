use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    #[allow(dead_code)]
    pub channels: u16,
    pub duration_seconds: f64,
}

/// Decodes audio file or video container audio track directly to 16kHz mono PCM using FFmpeg pipe.
pub fn load_audio_pcm_16k(file_path: &Path) -> Result<DecodedAudio, String> {
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    // Fast audio extraction directly to 16kHz mono s16le via stdout pipe
    let mut child = Command::new("ffmpeg")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(file_path)
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-f")
        .arg("s16le")
        .arg("pipe:1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn FFmpeg process: {}", e))?;

    let mut stdout = child.stdout.take().ok_or("Failed to open FFmpeg stdout pipe")?;
    let mut raw_bytes = Vec::new();
    stdout
        .read_to_end(&mut raw_bytes)
        .map_err(|e| format!("Failed to read audio stream from FFmpeg: {}", e))?;

    let status = child.wait().map_err(|e| format!("FFmpeg process wait error: {}", e))?;
    if !status.success() {
        return Err("FFmpeg failed to decode audio track from media file".to_string());
    }

    if raw_bytes.is_empty() {
        return Err("Extracted audio stream is empty".to_string());
    }

    // Convert raw s16le bytes to normalized f32 samples in range [-1.0, 1.0]
    let sample_count = raw_bytes.len() / 2;
    let mut samples = Vec::with_capacity(sample_count);

    for chunk in raw_bytes.chunks_exact(2) {
        let sample_i16 = i16::from_le_bytes([chunk[0], chunk[1]]);
        samples.push(sample_i16 as f32 / 32768.0);
    }

    let sample_rate = 16000;
    let duration_seconds = samples.len() as f64 / sample_rate as f64;

    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels: 1,
        duration_seconds,
    })
}
