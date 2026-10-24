use crate::engine::audio_loader::DecodedAudio;
use crate::models::{AnalysisResult, CutSettings, SelectedClipInfo, SpeechSegment};

pub struct VadDetector;

impl VadDetector {
    /// Performs Voice Activity and Silence Detection using sliding RMS energy and dBFS thresholding.
    /// Applies silence duration merging, speech length filtering, and breath padding margins.
    pub fn analyze(
        audio: &DecodedAudio,
        settings: &CutSettings,
        clip_info: &SelectedClipInfo,
    ) -> AnalysisResult {
        let sample_rate = audio.sample_rate as usize;
        // 20ms analysis window (320 samples at 16kHz)
        let frame_size = (sample_rate * 20) / 1000;
        let total_samples = audio.samples.len();

        let num_frames = total_samples / frame_size;
        let mut frame_is_speech = Vec::with_capacity(num_frames);

        // 1. Calculate RMS and dBFS for each 20ms sliding window
        for i in 0..num_frames {
            let start = i * frame_size;
            let end = (start + frame_size).min(total_samples);
            let frame = &audio.samples[start..end];

            let sum_sq: f32 = frame.iter().map(|&s| s * s).sum();
            let rms = (sum_sq / frame.len() as f32).sqrt();
            let db = 20.0 * (rms.max(1e-6)).log10();

            frame_is_speech.push(db >= settings.threshold_db);
        }

        let frame_duration_sec = 0.020; // 20ms
        let min_silence_frames = (settings.min_silence_ms as f32 / 20.0).round() as usize;
        let min_speech_frames = (settings.min_speech_ms as f32 / 20.0).round() as usize;

        // 2. Identify raw contiguous speech segments
        let mut raw_segments: Vec<(usize, usize)> = Vec::new(); // (start_frame_idx, end_frame_idx)
        let mut in_speech = false;
        let mut cur_start = 0;

        for (idx, &is_speech) in frame_is_speech.iter().enumerate() {
            if is_speech && !in_speech {
                in_speech = true;
                cur_start = idx;
            } else if !is_speech && in_speech {
                in_speech = false;
                raw_segments.push((cur_start, idx));
            }
        }
        if in_speech {
            raw_segments.push((cur_start, num_frames));
        }

        // 3. Merge speech segments separated by silence shorter than min_silence_ms
        let mut merged_segments: Vec<(usize, usize)> = Vec::new();
        for seg in raw_segments {
            if let Some(last) = merged_segments.last_mut() {
                let silence_gap = seg.0.saturating_sub(last.1);
                if silence_gap < min_silence_frames {
                    // Merge across micro-pauses
                    last.1 = seg.1;
                    continue;
                }
            }
            merged_segments.push(seg);
        }

        // 4. Discard transient spikes shorter than min_speech_ms
        let valid_segments: Vec<(usize, usize)> = merged_segments
            .into_iter()
            .filter(|(start, end)| (end - start) >= min_speech_frames)
            .collect();

        // 5. Convert to seconds and apply breath padding margins
        let pad_start_sec = settings.padding_start_ms as f64 / 1000.0;
        let pad_end_sec = settings.padding_end_ms as f64 / 1000.0;
        let total_duration = audio.duration_seconds;

        let mut padded_ranges: Vec<(f64, f64)> = Vec::new();
        for (f_start, f_end) in valid_segments {
            let start_sec = (f_start as f64 * frame_duration_sec - pad_start_sec).max(0.0);
            let end_sec = (f_end as f64 * frame_duration_sec + pad_end_sec).min(total_duration);

            if let Some(last) = padded_ranges.last_mut() {
                if start_sec <= last.1 {
                    // Overlapping due to padding expansion -> merge smoothly
                    last.1 = last.1.max(end_sec);
                    continue;
                }
            }
            padded_ranges.push((start_sec, end_sec));
        }

        // 6. Map seconds to DaVinci Resolve Timeline and Source timecode frames
        let fps = if clip_info.fps > 0.0 { clip_info.fps } else { 24.0 };
        let source_start_frame = clip_info.source_start_frame.unwrap_or(0);
        let timeline_start_frame = clip_info.timeline_start_frame.unwrap_or(0);

        let mut speech_duration = 0.0;
        let mut segments = Vec::new();

        for (start_sec, end_sec) in padded_ranges {
            let dur_sec = end_sec - start_sec;
            speech_duration += dur_sec;

            let s_start_f = source_start_frame + (start_sec * fps).round() as i64;
            let s_end_f = source_start_frame + (end_sec * fps).round() as i64;

            let t_start_f = timeline_start_frame + (start_sec * fps).round() as i64;
            let t_end_f = timeline_start_frame + (end_sec * fps).round() as i64;

            segments.push(SpeechSegment {
                start_seconds: start_sec,
                end_seconds: end_sec,
                source_start_frame: s_start_f,
                source_end_frame: s_end_f,
                timeline_start_frame: t_start_f,
                timeline_end_frame: t_end_f,
            });
        }

        let silence_duration = (total_duration - speech_duration).max(0.0);
        let speech_percentage = if total_duration > 0.0 {
            ((speech_duration / total_duration) * 100.0) as f32
        } else {
            100.0
        };

        AnalysisResult {
            total_duration_seconds: total_duration,
            speech_duration_seconds: speech_duration,
            silence_duration_seconds: silence_duration,
            speech_percentage,
            segments,
        }
    }
}
