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

#[cfg(unix)]
mod lock {
    use std::fs::{self, OpenOptions};
    use std::io::{self, Write};
    use std::os::fd::AsRawFd;
    use std::path::PathBuf;

    pub fn get_lock_path() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
                return PathBuf::from(xdg).join("voclip.lock");
            }
            if let Ok(uid) = std::env::var("UID") {
                return PathBuf::from(format!("/run/user/{uid}/voclip.lock"));
            }
        }
        PathBuf::from("/tmp/voclip.lock")
    }

    #[allow(deprecated)]
    pub fn acquire_lock() -> io::Result<std::fs::File> {
        let lock_path = get_lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&lock_path)?;
        let fd = file.as_raw_fd();
        match nix::fcntl::flock(fd, nix::fcntl::FlockArg::LockExclusiveNonblock) {
            Ok(()) => {
                let pid = std::process::id().to_string();
                file.set_len(0)?;
                file.write_all(pid.as_bytes())?;
                file.flush()?;
                Ok(file)
            }
            Err(e) => Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Another instance of voclip is already running: {e}"),
            )),
        }
    }
}

#[cfg(not(unix))]
mod lock {
    use std::io;

    pub fn acquire_lock() -> io::Result<std::fs::File> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Single-instance lock not supported on this platform",
        ))
    }
}

#[tokio::main]
async fn main() {
    let args = config::Args::parse();

    if args.update {
        if let Err(e) = update::update() {
            eprintln!("Update failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    #[cfg(unix)]
    {
        let _lock = match lock::acquire_lock() {
            Ok(lock) => lock,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        };
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

    if let Err(e) = beep::play_start_beep() {
        eprintln!("Start beep failed: {e}");
    }

    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(50);
    let capture = audio_capture::start_capture(audio_tx)?;
    let device_rate = capture.device_sample_rate;

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
