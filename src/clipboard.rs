use std::process::{Command, Stdio};
use std::io::Write;

use crate::error::VoclipError;

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check clipboard prerequisites on Linux. Call at startup.
pub fn check_clipboard_deps() {
    if cfg!(not(target_os = "linux")) {
        return;
    }

    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let is_x11 = std::env::var("DISPLAY").is_ok();

    if is_wayland {
        if !command_exists("wl-copy") {
            eprintln!("Warning: wl-copy not found. Clipboard will not persist after exit.");
            eprintln!("  Install with: sudo apt install wl-clipboard");
        }
    } else if is_x11 {
        if !command_exists("xclip") && !command_exists("xsel") {
            eprintln!("Warning: xclip/xsel not found. Clipboard will not persist after exit.");
            eprintln!("  Install with: sudo apt install xclip");
        }
    } else {
        eprintln!("Warning: No display server detected. Clipboard may not work.");
    }
}

/// Copy text to clipboard using platform-native tools that persist after process exit.
/// Falls back to arboard if no CLI tool is found.
pub fn copy_and_verify(text: &str) -> Result<bool, VoclipError> {
    // Try platform-specific CLI tools first (they fork/persist the clipboard)
    if try_cli_copy(text) {
        let readback = cli_read().unwrap_or_default();
        return Ok(readback.trim() == text.trim());
    }

    // Fallback: arboard (clipboard may not survive process exit on X11)
    let mut cb = arboard::Clipboard::new()?;
    cb.set_text(text)?;
    let readback = cb.get_text().unwrap_or_default();
    Ok(readback == text)
}

fn try_cli_copy(text: &str) -> bool {
    // Wayland
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            return child.wait().map(|s| s.success()).unwrap_or(false);
        }
    }

    // X11: try xclip, then xsel
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    if let Ok(mut child) = Command::new("xsel")
        .args(["--clipboard", "--input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    // macOS
    if let Ok(mut child) = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    false
}

fn cli_read() -> Option<String> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        if let Ok(out) = Command::new("wl-paste").output() {
            if out.status.success() {
                return Some(String::from_utf8_lossy(&out.stdout).to_string());
            }
        }
    }

    if let Ok(out) = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
    {
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }

    if let Ok(out) = Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output()
    {
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }

    if let Ok(out) = Command::new("pbpaste").output() {
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }

    None
}
