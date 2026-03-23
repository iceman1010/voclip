use std::collections::HashMap;
use std::path::Path;

use rustpotter::{Rustpotter, RustpotterConfig, SampleFormat, WakewordRefBuildFromBuffers};
use tokio::sync::mpsc;

use crate::audio_capture;
use crate::beep;
use crate::config::{Config, VoiceAction, VoicePattern, WakewordSensitivity};
use crate::error::VoclipError;
use crate::keyboard;
use crate::resample::Resampler;
use crate::token;
use crate::ui;
use crate::websocket;

/// Record voice samples and build a .rpw reference file.
pub async fn train(
    name: &str,
    num_samples: u32,
    wakeword_path: &Path,
    audio_device: Option<&str>,
) -> Result<(), VoclipError> {
    eprintln!("Training: recording {num_samples} samples of \"{name}\"");
    eprintln!("Speak clearly, about 1-2 seconds per sample.\n");

    let mut wav_samples: HashMap<String, Vec<u8>> = HashMap::new();
    let mut i = 1u32;

    while i <= num_samples {
        eprintln!("Press Enter to start recording sample {i}/{num_samples}...");
        read_stdin_line().await?;

        let wav_bytes = record_one_sample(audio_device).await?;

        eprintln!("  Sample {i} recorded. Press Enter to keep, or 'r' + Enter to redo:");
        let input = read_stdin_line().await?;

        if input.trim().eq_ignore_ascii_case("r") {
            eprintln!("  Redoing sample {i}...\n");
            continue;
        }

        wav_samples.insert(format!("sample_{i}.wav"), wav_bytes);
        eprintln!();
        i += 1;
    }

    eprintln!("Building model...");
    let wakeword = rustpotter::WakewordRef::new_from_sample_buffers(
        name.to_string(),
        None,
        None,
        wav_samples,
        16,
    )
    .map_err(VoclipError::WakeWord)?;

    if let Some(parent) = wakeword_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| VoclipError::WakeWord(format!("Failed to create directory: {e}")))?;
    }

    let path_str = wakeword_path.to_str().ok_or_else(|| {
        VoclipError::WakeWord("Invalid path".to_string())
    })?;
    rustpotter::WakewordSave::save_to_file(&wakeword, path_str)
        .map_err(VoclipError::WakeWord)?;

    eprintln!("Saved to: {}", wakeword_path.display());

    Ok(())
}

async fn read_stdin_line() -> Result<String, VoclipError> {
    tokio::task::spawn_blocking(|| {
        let mut buf = String::new();
        std::io::stdin()
            .read_line(&mut buf)
            .map(|_| buf)
    })
    .await
    .map_err(|e| VoclipError::WakeWord(format!("Failed to read stdin: {e}")))?
    .map_err(|e| VoclipError::WakeWord(format!("Failed to read stdin: {e}")))
}

async fn record_one_sample(audio_device: Option<&str>) -> Result<Vec<u8>, VoclipError> {
    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture_with_device(tx, audio_device)?;
    let device_rate = capture.device_sample_rate;

    if let Err(e) = beep::play_start_beep() {
        eprintln!("Beep failed: {e}");
    }

    eprintln!("  Recording...");

    let mut all_samples: Vec<i16> = Vec::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(3);
    loop {
        tokio::select! {
            chunk = rx.recv() => {
                match chunk {
                    Some(samples) => all_samples.extend_from_slice(&samples),
                    None => break,
                }
            }
            _ = tokio::time::sleep_until(deadline) => break,
        }
    }

    drop(capture);

    if let Err(e) = beep::play_stop_beep() {
        eprintln!("Beep failed: {e}");
    }

    let mut resampler = Resampler::new(device_rate, 16000);
    let resampled = resampler.process(&all_samples);

    Ok(encode_wav(&resampled, 16000))
}

/// The sample rate used for wake word detection.
/// Models are trained at 16kHz, so we resample to this rate before detection.
/// This reduces CPU usage ~3x compared to processing at typical 48kHz device rates.
const DETECT_SAMPLE_RATE: u32 = 16000;

/// Create a configured rustpotter detector with all voice patterns loaded.
fn create_detector(
    patterns: &[VoicePattern],
    sensitivity: WakewordSensitivity,
) -> Result<Rustpotter, VoclipError> {
    let mut config = RustpotterConfig::default();
    config.fmt.sample_rate = DETECT_SAMPLE_RATE as usize;
    config.fmt.sample_format = SampleFormat::I16;
    config.fmt.channels = 1;

    // Tune detection parameters based on sensitivity
    match sensitivity {
        WakewordSensitivity::Low => {
            config.detector.threshold = 0.55;
            config.detector.avg_threshold = 0.25;
            config.detector.min_scores = 5;
        }
        WakewordSensitivity::Medium => {
            config.detector.threshold = 0.4;
            config.detector.avg_threshold = 0.15;
            config.detector.min_scores = 3;
        }
        WakewordSensitivity::High => {
            config.detector.threshold = 0.3;
            config.detector.avg_threshold = 0.1;
            config.detector.min_scores = 2;
            config.detector.eager = true;
        }
        WakewordSensitivity::Custom(threshold) => {
            config.detector.threshold = threshold;
            config.detector.avg_threshold = threshold * 0.4;
            config.detector.min_scores = 3;
        }
    }

    // Enable gain normalization to handle volume variations
    config.filters.gain_normalizer.enabled = true;

    let mut detector = Rustpotter::new(&config)
        .map_err(|e| VoclipError::WakeWord(format!("Failed to create detector: {e}")))?;

    for pattern in patterns {
        if !pattern.path.exists() {
            ui::warn(&format!(
                "Voice pattern file not found: {} (skipping \"{}\")",
                pattern.path.display(),
                pattern.name
            ));
            continue;
        }
        detector
            .add_wakeword_from_file(
                &pattern.name,
                pattern.path.to_str().unwrap_or_default(),
            )
            .map_err(|e| {
                VoclipError::WakeWord(format!(
                    "Failed to load voice pattern \"{}\": {e}",
                    pattern.name
                ))
            })?;
    }

    Ok(detector)
}

/// Find the voice pattern that matches a detection result.
/// Rustpotter's detection.name is the internal .rpw training name, which may differ
/// from the configured pattern name (e.g., legacy files trained as "hey voclip" but
/// renamed to "Computer" in config). Falls back to first loaded pattern if no exact match.
fn find_pattern<'a>(patterns: &'a [VoicePattern], detection_name: &str) -> Option<&'a VoicePattern> {
    patterns
        .iter()
        .find(|p| p.name == detection_name)
        .or_else(|| patterns.iter().find(|p| p.path.exists()))
}

/// Test detection of all trained voice patterns.
pub async fn test(
    patterns: &[VoicePattern],
    sensitivity: WakewordSensitivity,
    audio_device: Option<&str>,
) -> Result<(), VoclipError> {
    let trained: Vec<_> = patterns.iter().filter(|p| p.path.exists()).collect();
    if trained.is_empty() {
        return Err(VoclipError::WakeWord(
            "No trained voice patterns found. Run --train-wakeword or --train-command first."
                .to_string(),
        ));
    }

    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture_with_device(tx, audio_device)?;
    let device_rate = capture.device_sample_rate;

    let mut detector = create_detector(patterns, sensitivity)?;
    let samples_per_frame = detector.get_samples_per_frame();
    let mut buffer: Vec<i16> = Vec::new();
    let mut resampler = Resampler::new(device_rate, DETECT_SAMPLE_RATE);

    eprintln!("Testing detection (sensitivity: {sensitivity:?}, Ctrl+C to stop)...");
    eprintln!(
        "Loaded {} pattern(s). Say any trained phrase.\n",
        trained.len()
    );

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            chunk = rx.recv() => {
                match chunk {
                    Some(samples) => {
                        let resampled = resampler.process(&samples);
                        buffer.extend_from_slice(&resampled);
                        while buffer.len() >= samples_per_frame {
                            let frame: Vec<i16> =
                                buffer.drain(..samples_per_frame).collect();
                            if let Some(detection) = detector.process_samples(frame) {
                                let (label, display_name) = if let Some(p) = find_pattern(patterns, &detection.name) {
                                    let l = match &p.action {
                                        VoiceAction::Transcribe => "wake word",
                                        VoiceAction::Key(_) => "command word",
                                    };
                                    (l, p.name.as_str())
                                } else {
                                    ("unknown", detection.name.as_str())
                                };
                                eprintln!(
                                    "  DETECTED [{label}]: \"{}\" (score: {:.3})",
                                    display_name, detection.score
                                );
                                if let Err(e) = beep::play_start_beep() {
                                    eprintln!("Beep failed: {e}");
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
            _ = &mut ctrl_c => {
                eprintln!("\nDone testing.");
                break;
            }
        }
    }

    drop(capture);
    Ok(())
}

/// Always-on listen mode: detect voice patterns → dispatch action → repeat.
/// Keeps a single audio capture stream and detector alive across cycles.
pub async fn listen(config: &Config) -> Result<(), VoclipError> {
    let trained: Vec<_> = config
        .voice_patterns
        .iter()
        .filter(|p| p.path.exists())
        .collect();

    if trained.is_empty() {
        return Err(VoclipError::WakeWord(
            "No trained voice patterns found. Run --train-wakeword or --train-command first."
                .to_string(),
        ));
    }

    ui::header();
    ui::label("Model", &format!("{} ({})", config.model, config.model.description()));
    keyboard::check_keyboard_deps();

    let wake_count = trained
        .iter()
        .filter(|p| p.action == VoiceAction::Transcribe)
        .count();
    let cmd_count = trained.len() - wake_count;

    eprintln!(
        "  Loaded {} wake word(s) and {} command word(s):",
        wake_count, cmd_count
    );
    for p in &trained {
        let label = match &p.action {
            VoiceAction::Transcribe => "Wake word:   ",
            VoiceAction::Key(_) => "Command word:",
        };
        eprintln!("    {label} \"{}\" → {}", p.name, p.action);
    }
    eprintln!();
    ui::info("Listening... (Ctrl+C to stop)\n");

    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture_with_device(tx, config.audio_device.as_deref())?;
    let device_rate = capture.device_sample_rate;

    let mut detector = create_detector(
        &config.voice_patterns,
        config.wakeword_sensitivity,
    )?;
    let samples_per_frame = detector.get_samples_per_frame();
    let mut buffer: Vec<i16> = Vec::new();
    let mut resampler = Resampler::new(device_rate, DETECT_SAMPLE_RATE);

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        // --- Detection phase: wait for a voice pattern ---
        let pattern = loop {
            tokio::select! {
                chunk = rx.recv() => {
                    match chunk {
                        Some(samples) => {
                            let resampled = resampler.process(&samples);
                            buffer.extend_from_slice(&resampled);
                            let mut detected = None;
                            while buffer.len() >= samples_per_frame {
                                let frame: Vec<i16> =
                                    buffer.drain(..samples_per_frame).collect();
                                if let Some(detection) = detector.process_samples(frame)
                                    && let Some(p) = find_pattern(
                                        &config.voice_patterns,
                                        &detection.name,
                                    )
                                {
                                    let label = match &p.action {
                                        VoiceAction::Transcribe => "Wake word",
                                        VoiceAction::Key(_) => "Command word",
                                    };
                                    ui::success(&format!(
                                        "{label} detected: \"{}\" (score: {:.3})",
                                        p.name, detection.score
                                    ));
                                    detected = Some(p.clone());
                                    break;
                                }
                            }
                            if detected.is_some() {
                                break detected;
                            }
                            continue;
                        }
                        None => break None,
                    }
                }
                _ = &mut ctrl_c => {
                    eprint!("\r\x1b[2K");
                    ui::warn("Interrupted.");
                    break None;
                }
            }
        };

        let Some(pattern) = pattern else {
            break;
        };

        // Drain the audio channel while we handle the action so the buffer
        // doesn't build up and cause a stale detection on resume.
        drain_channel(&mut rx);
        buffer.clear();

        // --- Action dispatch ---
        match &pattern.action {
            VoiceAction::Transcribe => {
                if let Err(e) = run_transcription(config).await {
                    let _ = beep::play_error_beep();
                    ui::error(&format!("Transcription error: {e}"));
                }
            }
            VoiceAction::Key(key_name) => {
                if let Err(e) = beep::play_start_beep() {
                    ui::warn(&format!("Beep failed: {e}"));
                }
                match keyboard::press_key(key_name) {
                    Ok(()) => ui::success(&format!("Pressed: {key_name}")),
                    Err(e) => ui::error(&format!("Key press error: {e}")),
                }
            }
        }

        // Drain again after action completes to discard audio captured during it.
        drain_channel(&mut rx);
        buffer.clear();

        eprintln!();
        ui::info("Listening...\n");
    }

    drop(capture);
    Ok(())
}

/// Drain all pending messages from an audio channel.
fn drain_channel(rx: &mut mpsc::Receiver<Vec<i16>>) {
    while rx.try_recv().is_ok() {}
}

/// Run a single transcription cycle: authenticate → capture → transcribe → type output.
async fn run_transcription(config: &Config) -> Result<(), VoclipError> {
    if let Err(e) = beep::play_start_beep() {
        ui::warn(&format!("Start beep failed: {e}"));
    }

    ui::dim("Authenticating...");
    let token = token::fetch_token(&config.api_key).await?;

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture_with_device(audio_tx, config.audio_device.as_deref())?;
    let device_rate = capture.device_sample_rate;

    ui::dim("Connecting...");
    let (ws_tx, ws_rx) =
        websocket::connect(&token, config.timeout, config.model.api_name()).await?;

    ui::info(&format!(
        "Listening... (speak, then wait {}s silence to finish)",
        config.timeout
    ));

    let result = websocket::stream(ws_tx, ws_rx, device_rate, audio_rx).await?;

    drop(capture);

    let transcript = result.transcript.trim().to_string();

    if transcript.is_empty() {
        ui::dim("No speech detected.");
        if let Err(e) = beep::play_stop_beep() {
            ui::warn(&format!("Stop beep failed: {e}"));
        }
        return Ok(());
    }

    // Listen mode always types output
    keyboard::type_text(&transcript)?;
    ui::success(&format!("Typed: {transcript}"));

    if let Err(e) = beep::play_stop_beep() {
        ui::warn(&format!("Stop beep failed: {e}"));
    }

    Ok(())
}

/// Encode i16 PCM samples as a WAV file in memory.
fn encode_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len() as u32;
    let data_size = num_samples * 2; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size as usize + 8);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &sample in samples {
        buf.extend_from_slice(&sample.to_le_bytes());
    }

    buf
}
