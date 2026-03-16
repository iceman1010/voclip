mod audio_capture;
mod beep;
mod clipboard;
mod config;
mod error;
mod resample;
mod token;
mod websocket;

use config::Config;
use error::VoclipError;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), VoclipError> {
    let config = Config::load()?;

    clipboard::check_clipboard_deps();

    eprintln!("Authenticating...");
    let token = token::fetch_token(&config.api_key).await?;

    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture(audio_tx)?;
    let device_rate = capture.device_sample_rate;

    // Rising chirp: recording started
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

    // Drop mic stream before playing stop beep (avoids ALSA conflicts on Linux)
    drop(capture);

    let transcript = result.transcript.trim().to_string();

    if transcript.is_empty() {
        eprintln!("No speech detected.");
        return Ok(());
    }

    // Copy to clipboard and verify
    let verified = clipboard::copy_and_verify(&transcript)?;
    if verified {
        eprintln!("Copied to clipboard: {transcript}");
    } else {
        eprintln!("Clipboard write may have failed. Transcript: {transcript}");
    }

    // Falling chirp: done
    if let Err(e) = beep::play_stop_beep() {
        eprintln!("Stop beep failed: {e}");
    }

    Ok(())
}
