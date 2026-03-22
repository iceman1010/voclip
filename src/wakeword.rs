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
use crate::websocket;

/// Record voice samples and build a .rpw reference file.
pub async fn train(
    name: &str,
    num_samples: u32,
    wakeword_path: &Path,
) -> Result<(), VoclipError> {
    eprintln!("Training: recording {num_samples} samples of \"{name}\"");
    eprintln!("Speak clearly, about 1-2 seconds per sample.\n");

    let mut wav_samples: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 1..=num_samples {
        eprintln!("Press Enter to start recording sample {i}/{num_samples}...");
        tokio::task::spawn_blocking(|| {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)
        })
        .await
        .map_err(|e| VoclipError::WakeWord(format!("Failed to read stdin: {e}")))?
        .map_err(|e| VoclipError::WakeWord(format!("Failed to read stdin: {e}")))?;

        let (tx, mut rx) = mpsc::channel::<Vec<i16>>(50);
        let capture = audio_capture::start_capture(tx)?;
        let device_rate = capture.device_sample_rate;

        if let Err(e) = beep::play_start_beep() {
            eprintln!("Beep failed: {e}");
        }

        eprintln!("  Recording...");

        // Collect audio for 3 seconds
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

        // Resample to 16kHz
        let mut resampler = Resampler::new(device_rate, 16000);
        let resampled = resampler.process(&all_samples);

        // Encode as WAV
        let wav_bytes = encode_wav(&resampled, 16000);
        wav_samples.insert(format!("sample_{i}.wav"), wav_bytes);

        eprintln!("  Sample {i} recorded ({} samples at 16kHz)\n", resampled.len());
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

/// Create a configured rustpotter detector with all voice patterns loaded.
fn create_detector(
    device_rate: u32,
    patterns: &[VoicePattern],
    sensitivity: WakewordSensitivity,
) -> Result<Rustpotter, VoclipError> {
    let mut config = RustpotterConfig::default();
    config.fmt.sample_rate = device_rate as usize;
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
            eprintln!(
                "Warning: voice pattern file not found: {} (skipping \"{}\")",
                pattern.path.display(),
                pattern.name
            );
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

/// Test detection of all trained voice patterns.
pub async fn test(
    patterns: &[VoicePattern],
    sensitivity: WakewordSensitivity,
) -> Result<(), VoclipError> {
    let trained: Vec<_> = patterns.iter().filter(|p| p.path.exists()).collect();
    if trained.is_empty() {
        return Err(VoclipError::WakeWord(
            "No trained voice patterns found. Run --train-wakeword or --train-command first."
                .to_string(),
        ));
    }

    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture(tx)?;
    let device_rate = capture.device_sample_rate;

    let mut detector = create_detector(device_rate, patterns, sensitivity)?;
    let samples_per_frame = detector.get_samples_per_frame();
    let mut buffer: Vec<i16> = Vec::new();

    eprintln!("Testing detection (sensitivity: {sensitivity:?}, Ctrl+C to stop)...");
    eprintln!(
        "Loaded {} pattern(s). Say any trained phrase.\n",
        trained.len()
    );

    loop {
        tokio::select! {
            chunk = rx.recv() => {
                match chunk {
                    Some(samples) => {
                        buffer.extend_from_slice(&samples);
                        while buffer.len() >= samples_per_frame {
                            let frame: Vec<i16> =
                                buffer.drain(..samples_per_frame).collect();
                            if let Some(detection) = detector.process_samples(frame) {
                                let label = if let Some(p) = patterns.iter().find(|p| p.name == detection.name) {
                                    match &p.action {
                                        VoiceAction::Transcribe => "wake word",
                                        VoiceAction::Key(_) => "command word",
                                    }
                                } else {
                                    "unknown"
                                };
                                eprintln!(
                                    "  DETECTED [{label}]: \"{}\" (score: {:.3})",
                                    detection.name, detection.score
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
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\nDone testing.");
                break;
            }
        }
    }

    drop(capture);
    Ok(())
}

/// Always-on listen mode: detect voice patterns → dispatch action → repeat.
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

    eprintln!("Using model: {} ({})", config.model, config.model.description());
    keyboard::check_keyboard_deps();

    let wake_count = trained
        .iter()
        .filter(|p| p.action == VoiceAction::Transcribe)
        .count();
    let cmd_count = trained.len() - wake_count;

    eprintln!(
        "Loaded {} wake word(s) and {} command word(s):",
        wake_count, cmd_count
    );
    for p in &trained {
        let label = match &p.action {
            VoiceAction::Transcribe => "Wake word:   ",
            VoiceAction::Key(_) => "Command word:",
        };
        eprintln!("  {label} \"{}\" → {}", p.name, p.action);
    }
    eprintln!("Listening... (Ctrl+C to stop)\n");

    loop {
        // --- Detection phase ---
        let detected = detect(config).await?;
        let Some(pattern) = detected else {
            // Ctrl+C
            break;
        };

        // --- Action dispatch ---
        match &pattern.action {
            VoiceAction::Transcribe => {
                if let Err(e) = run_transcription(config).await {
                    let _ = beep::play_error_beep();
                    eprintln!("Transcription error: {e}");
                }
            }
            VoiceAction::Key(key_name) => {
                if let Err(e) = beep::play_start_beep() {
                    eprintln!("Beep failed: {e}");
                }
                match keyboard::press_key(key_name) {
                    Ok(()) => eprintln!("Pressed: {key_name}"),
                    Err(e) => eprintln!("Key press error: {e}"),
                }
            }
        }

        eprintln!("\nListening...\n");
    }

    Ok(())
}

/// Listen for any voice pattern. Returns the matched pattern on detection, None on Ctrl+C.
async fn detect(config: &Config) -> Result<Option<VoicePattern>, VoclipError> {
    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture(tx)?;
    let device_rate = capture.device_sample_rate;

    let mut detector = create_detector(
        device_rate,
        &config.voice_patterns,
        config.wakeword_sensitivity,
    )?;
    let samples_per_frame = detector.get_samples_per_frame();
    let mut buffer: Vec<i16> = Vec::new();

    loop {
        tokio::select! {
            chunk = rx.recv() => {
                match chunk {
                    Some(samples) => {
                        buffer.extend_from_slice(&samples);
                        while buffer.len() >= samples_per_frame {
                            let frame: Vec<i16> =
                                buffer.drain(..samples_per_frame).collect();
                            if let Some(detection) = detector.process_samples(frame)
                                && let Some(pattern) = config.voice_patterns.iter().find(|p| p.name == detection.name)
                            {
                                let label = match &pattern.action {
                                    VoiceAction::Transcribe => "Wake word",
                                    VoiceAction::Key(_) => "Command word",
                                };
                                eprintln!(
                                    "{label} detected: \"{}\" (score: {:.3})",
                                    pattern.name, detection.score
                                );
                                drop(capture);
                                return Ok(Some(pattern.clone()));
                            }
                        }
                    }
                    None => {
                        drop(capture);
                        return Ok(None);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                eprint!("\r\x1b[2K");
                eprintln!("Interrupted.");
                drop(capture);
                return Ok(None);
            }
        }
    }
}

/// Run a single transcription cycle: authenticate → capture → transcribe → type output.
async fn run_transcription(config: &Config) -> Result<(), VoclipError> {
    if let Err(e) = beep::play_start_beep() {
        eprintln!("Start beep failed: {e}");
    }

    eprintln!("Authenticating...");
    let token = token::fetch_token(&config.api_key).await?;

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture(audio_tx)?;
    let device_rate = capture.device_sample_rate;

    eprintln!("Connecting...");
    let (ws_tx, ws_rx) =
        websocket::connect(&token, config.timeout, config.model.api_name()).await?;

    eprintln!(
        "Listening... (speak, then wait {}s silence to finish)",
        config.timeout
    );

    let result = websocket::stream(ws_tx, ws_rx, device_rate, audio_rx).await?;

    drop(capture);

    let transcript = result.transcript.trim().to_string();

    if transcript.is_empty() {
        eprintln!("No speech detected.");
        if let Err(e) = beep::play_stop_beep() {
            eprintln!("Stop beep failed: {e}");
        }
        return Ok(());
    }

    // Listen mode always types output
    keyboard::type_text(&transcript)?;
    eprintln!("Typed: {transcript}");

    if let Err(e) = beep::play_stop_beep() {
        eprintln!("Stop beep failed: {e}");
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
