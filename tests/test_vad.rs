use aeon_cut::engine::audio_loader::DecodedAudio;
use aeon_cut::engine::VadDetector;
use aeon_cut::models::{CutMode, CutSettings, SelectedClipInfo};

#[test]
fn test_vad_silence_detection() {
    let sample_rate = 16000;
    // 1.0s speech + 1.0s silence + 1.0s speech = 3.0s total audio
    let mut samples = Vec::new();

    // 1.0s of sine wave tone (~ -10 dBFS)
    for i in 0..sample_rate {
        let t = i as f32 / sample_rate as f32;
        samples.push((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5);
    }

    // 1.0s of digital silence (0.0)
    for _ in 0..sample_rate {
        samples.push(0.0);
    }

    // 1.0s of sine wave tone (~ -10 dBFS)
    for i in 0..sample_rate {
        let t = i as f32 / sample_rate as f32;
        samples.push((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5);
    }

    let audio = DecodedAudio {
        samples,
        sample_rate: sample_rate as u32,
        channels: 1,
        duration_seconds: 3.0,
    };

    let settings = CutSettings {
        threshold_db: -30.0,
        min_silence_ms: 300,
        min_speech_ms: 100,
        padding_start_ms: 50,
        padding_end_ms: 50,
        mode: CutMode::CleanTimeline,
    };

    let clip_info = SelectedClipInfo {
        success: true,
        has_selection: true,
        clip_name: Some("test_clip".to_string()),
        file_path: Some("/test/test.wav".to_string()),
        fps: 24.0,
        timeline_start_frame: Some(0),
        timeline_end_frame: Some(72),
        duration_frames: Some(72),
        source_start_frame: Some(0),
        source_end_frame: Some(72),
        timeline_name: Some("Timeline 1".to_string()),
        error: None,
    };

    let result = VadDetector::analyze(&audio, &settings, &clip_info);

    println!("Detected segments: {}", result.segments.len());
    for (i, seg) in result.segments.iter().enumerate() {
        println!(
            "Segment {}: {:.2}s -> {:.2}s (frames: {} -> {})",
            i, seg.start_seconds, seg.end_seconds, seg.source_start_frame, seg.source_end_frame
        );
    }

    assert_eq!(result.segments.len(), 2, "Expected exactly 2 speech segments");
    assert!(result.speech_duration_seconds > 1.8 && result.speech_duration_seconds < 2.3);
    assert!(result.silence_duration_seconds > 0.7 && result.silence_duration_seconds < 1.2);
}
