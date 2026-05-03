mod audio_capture;
mod beep;
mod clipboard;
mod config;
mod error;
mod keyboard;
mod resample;
mod speech_model;
mod token;
mod ui;
mod update;
mod wakeword;
mod websocket;

use clap::Parser;
use config::Config;
use error::VoclipError;

use std::fs;
use std::path::PathBuf;

struct PidLock {
    path: PathBuf,
}

impl PidLock {
    fn acquire() -> Result<Self, String> {
        let run_dir = dirs_next::runtime_dir()
            .or_else(dirs_next::cache_dir)
            .unwrap_or_else(std::env::temp_dir);
        let path = run_dir.join("voclip.pid");

        // Check for stale lock
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
        {
            if let Ok(pid) = content.trim().parse::<u32>() {
                // Check if process is still alive
                if std::path::Path::new(&format!("/proc/{pid}")).exists() {
                    return Err("Another instance of voclip is already running".to_string());
                }
            }
            // Stale lock — process is dead, remove it
            let _ = fs::remove_file(&path);
        }

        fs::write(&path, std::process::id().to_string())
            .map_err(|e| format!("Failed to create lock: {e}"))?;

        Ok(PidLock { path })
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[tokio::main]
async fn main() {
    let args = config::Args::parse();

    if args.version {
        println!("voclip {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.list_models {
        config::print_models();
        return;
    }

    if args.list_devices {
        match audio_capture::list_input_devices() {
            Ok(devices) => {
                if devices.is_empty() {
                    println!("No audio input devices found.");
                } else {
                    println!("Audio input devices:\n");
                    for (i, name) in devices.iter().enumerate() {
                        println!("  {}: {name}", i + 1);
                    }
                    println!("\nUse --audio-device <name> to select one.");
                }
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
        return;
    }

    if args.list_wakewords {
        config::list_voice_patterns();
        return;
    }

    if let Some(ref name) = args.set_default_model {
        let result = if name.eq_ignore_ascii_case("list") {
            config::prompt_and_save_default_model()
        } else {
            config::save_default_model(name)
        };
        match result {
            Ok(model) => {
                println!("Default model set to: {} ({})", model, model.description());
                return;
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
    }

    if let Some(secs) = args.set_default_timeout {
        match config::save_default_timeout(secs) {
            Ok(()) => {
                println!("Default timeout set to: {}s", secs);
                return;
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
    }

    if args.apikey {
        match config::prompt_and_save_api_key() {
            Ok(true) => {
                println!("API key saved to ~/.config/voclip/.env");
                return;
            }
            Ok(false) => {
                println!("API key unchanged.");
                return;
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
    }

    if let Some(ref name) = args.remove_wakeword {
        match config::remove_voice_pattern(name) {
            Ok(true) => {
                println!("Removed: \"{name}\"");
                return;
            }
            Ok(false) => {
                ui::error(&format!("Not found: \"{name}\""));
                std::process::exit(1);
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
    }

    // --audio-device alone: save to config
    if let Some(ref name) = args.audio_device
        && !args.train_wakeword
        && !args.train_command
        && !args.listen
        && !args.test_wakeword
    {
        match config::save_audio_device(name) {
            Ok(()) => {
                println!("Audio device set to: \"{name}\"");
                return;
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
    }

    // --wakeword-name alone: save to config as wake word
    if args.wakeword_name != "hey voclip"
        && !args.train_wakeword
        && !args.train_command
        && !args.listen
        && !args.test_wakeword
    {
        let action = config::VoiceAction::Transcribe;
        match config::save_voice_pattern(&args.wakeword_name, &action) {
            Ok(()) => {
                println!("Wake word name set to: \"{}\"", args.wakeword_name);
                return;
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
    }

    // Resolve audio device for training/testing (doesn't need full Config)
    let audio_device = args
        .audio_device
        .clone()
        .or_else(|| config::ConfigFile::load().audio_device);

    // Train wake word
    if args.train_wakeword {
        let path = config::voice_pattern_path_for_name(&args.wakeword_name);
        if let Err(e) = wakeword::train(
            &args.wakeword_name,
            args.wakeword_samples,
            &path,
            audio_device.as_deref(),
        )
        .await
        {
            let _ = beep::play_error_beep();
            ui::error(&format!("Training failed: {e}"));
            std::process::exit(1);
        }
        // Register in config
        let action = config::VoiceAction::Transcribe;
        if let Err(e) = config::save_voice_pattern(&args.wakeword_name, &action) {
            ui::warn(&format!("Trained but failed to save config: {e}"));
        }
        return;
    }

    // Train command word
    if args.train_command {
        let Some(ref name) = args.command_name else {
            ui::error("--train-command requires --command-name");
            std::process::exit(1);
        };
        let Some(ref action_str) = args.command_action else {
            ui::error("--train-command requires --command-action (e.g. \"key:Return\")");
            std::process::exit(1);
        };
        let Some(action) = config::VoiceAction::parse(action_str) else {
            ui::error(&format!(
                "Invalid action \"{action_str}\". Use \"key:<keyname>\" (e.g. \"key:Return\")"
            ));
            std::process::exit(1);
        };

        let path = config::voice_pattern_path_for_name(name);
        if let Err(e) =
            wakeword::train(name, args.wakeword_samples, &path, audio_device.as_deref()).await
        {
            let _ = beep::play_error_beep();
            ui::error(&format!("Training failed: {e}"));
            std::process::exit(1);
        }
        // Register in config
        if let Err(e) = config::save_voice_pattern(name, &action) {
            ui::warn(&format!("Trained but failed to save config: {e}"));
        }
        return;
    }

    let _lock = match PidLock::acquire() {
        Ok(lock) => lock,
        Err(e) => {
            ui::error(&e);
            std::process::exit(1);
        }
    };

    if args.update {
        if let Err(e) = update::update() {
            ui::error(&format!("Update failed: {e}"));
            std::process::exit(1);
        }
        return;
    }

    if args.test_wakeword {
        let file_config = config::ConfigFile::load();
        let patterns = config::load_voice_patterns_from(&file_config);
        if patterns.is_empty() {
            ui::error("No voice patterns configured. Train one first.");
            std::process::exit(1);
        }
        let sensitivity = config::WakewordSensitivity::parse(&args.wakeword_sensitivity)
            .unwrap_or(config::WakewordSensitivity::Medium);
        if let Err(e) = wakeword::test(&patterns, sensitivity, audio_device.as_deref()).await {
            let _ = beep::play_error_beep();
            ui::error(&format!("{e}"));
            std::process::exit(1);
        }
        return;
    }

    if args.listen {
        match Config::load(&args) {
            Ok(config) => {
                if let Err(e) = wakeword::listen(&config).await {
                    let _ = beep::play_error_beep();
                    ui::error(&format!("{e}"));
                    std::process::exit(1);
                }
            }
            Err(e) => {
                ui::error(&format!("{e}"));
                std::process::exit(1);
            }
        }
        return;
    }

    if let Err(e) = run(&args).await {
        let _ = beep::play_error_beep();
        ui::error(&format!("{e}"));
        std::process::exit(1);
    }
}

async fn run(args: &config::Args) -> Result<(), VoclipError> {
    let config = Config::load(args)?;

    eprintln!(
        "Using model: {} ({})",
        config.model,
        config.model.description()
    );

    match config.output_mode {
        config::OutputMode::Clipboard => clipboard::check_clipboard_deps(),
        config::OutputMode::Type => keyboard::check_keyboard_deps(),
    }

    eprintln!("Authenticating...");
    let token = token::fetch_token(&config.api_key).await?;

    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(50);
    let capture =
        audio_capture::start_capture_with_device(audio_tx, config.audio_device.as_deref())?;
    let device_rate = capture.device_sample_rate;

    eprintln!("Recording starts in {}s...", config.delay);
    tokio::time::sleep(std::time::Duration::from_secs(config.delay as u64)).await;

    eprintln!("Connecting...");
    let (ws_tx, ws_rx) =
        websocket::connect(&token, config.timeout, config.model.api_name()).await?;

    if let Err(e) = beep::play_start_beep() {
        eprintln!("Start beep failed: {e}");
    }

    eprintln!(
        "Listening... (speak, then wait {}s silence to finish, or Ctrl+C)",
        config.timeout
    );

    let result = websocket::stream(ws_tx, ws_rx, device_rate, audio_rx).await?;

    drop(capture);

    let transcript = result.transcript.trim().to_string();

    if transcript.is_empty() {
        eprintln!("No speech detected.");
        return Ok(());
    }

    match config.output_mode {
        config::OutputMode::Clipboard => {
            let verified = clipboard::copy_and_verify(&transcript)?;
            if verified {
                eprintln!("Copied to clipboard: {transcript}");
            } else {
                eprintln!("Clipboard write may have failed. Transcript: {transcript}");
            }
        }
        config::OutputMode::Type => {
            keyboard::type_text(&transcript)?;
            eprintln!("Typed: {transcript}");
        }
    }

    if let Err(e) = beep::play_stop_beep() {
        eprintln!("Stop beep failed: {e}");
    }

    Ok(())
}
