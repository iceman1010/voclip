mod audio_capture;
mod beep;
mod clipboard;
mod config;
mod error;
mod resample;
mod token;
mod update;
mod websocket;

use clap::Parser;
use config::Config;
use error::VoclipError;
use single_instance::SingleInstance;

fn acquire_lock() -> Result<SingleInstance, String> {
    let instance = SingleInstance::new("voclip").map_err(|e| {
        format!("Failed to create lock: {e}")
    })?;
    if instance.is_single() {
        Ok(instance)
    } else {
        Err("Another instance of voclip is already running".to_string())
    }
}

#[tokio::main]
async fn main() {
    let args = config::Args::parse();

    let _lock = match acquire_lock() {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if args.update {
        if let Err(e) = update::update() {
            eprintln!("Update failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(e) = run(&args).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(args: &config::Args) -> Result<(), VoclipError> {
    let config = Config::load(args)?;

    clipboard::check_clipboard_deps();

    eprintln!("Authenticating...");
    let token = token::fetch_token(&config.api_key).await?;

    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture(audio_tx)?;
    let device_rate = capture.device_sample_rate;

    eprintln!("Recording starts in {}s...", config.delay);
    tokio::time::sleep(std::time::Duration::from_secs(config.delay as u64)).await;

    if let Err(e) = beep::play_start_beep() {
        eprintln!("Start beep failed: {e}");
    }

    eprintln!("Listening... (speak, then wait {}s silence to finish, or Ctrl+C)", config.timeout);

    let result = websocket::run_session(
        &token,
        device_rate,
        config.timeout,
        &config.language,
        audio_rx,
    )
    .await?;

    drop(capture);

    let transcript = result.transcript.trim().to_string();

    if transcript.is_empty() {
        eprintln!("No speech detected.");
        return Ok(());
    }

    let verified = clipboard::copy_and_verify(&transcript)?;
    if verified {
        eprintln!("Copied to clipboard: {transcript}");
    } else {
        eprintln!("Clipboard write may have failed. Transcript: {transcript}");
    }

    if let Err(e) = beep::play_stop_beep() {
        eprintln!("Stop beep failed: {e}");
    }

    Ok(())
}
