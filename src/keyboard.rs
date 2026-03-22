use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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

/// Check keyboard typing prerequisites on Linux. Call at startup when --type is active.
pub fn check_keyboard_deps() {
    if cfg!(not(target_os = "linux")) {
        return;
    }

    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let is_x11 = std::env::var("DISPLAY").is_ok();

    if is_wayland {
        if !command_exists("wtype") && !command_exists("ydotool") {
            eprintln!("Warning: wtype/ydotool not found. Keyboard typing will use enigo fallback.");
            eprintln!("  Install with: sudo apt install wtype");
            eprintln!("  Or: sudo apt install ydotool");
        }
    } else if is_x11 {
        if !command_exists("xdotool") {
            eprintln!("Warning: xdotool not found. Keyboard typing will use enigo fallback.");
            eprintln!("  Install with: sudo apt install xdotool");
        }
    } else {
        eprintln!("Warning: No display server detected. Keyboard typing may not work.");
    }
}

/// Type text via simulated keyboard input.
/// Tries platform CLI tools first, falls back to enigo.
pub fn type_text(text: &str) -> Result<(), VoclipError> {
    // Small delay to let hotkey modifier keys release
    thread::sleep(Duration::from_millis(50));

    if try_cli_type(text) {
        return Ok(());
    }

    type_with_enigo(text)
}

fn try_cli_type(text: &str) -> bool {
    // Wayland: try wtype first, then ydotool
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        if try_command("wtype", &["--", text]) {
            return true;
        }
        if try_command("ydotool", &["type", "--", text]) {
            return true;
        }
    }

    // X11: xdotool
    if std::env::var("DISPLAY").is_ok()
        && try_command("xdotool", &["type", "--clearmodifiers", "--", text])
    {
        return true;
    }

    // macOS: no good CLI tool, skip to enigo
    // Windows: no good CLI tool, skip to enigo

    false
}

fn try_command(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Simulate a single key press (e.g., "Return", "BackSpace", "Tab").
/// Tries platform CLI tools first, falls back to enigo.
pub fn press_key(key_name: &str) -> Result<(), VoclipError> {
    thread::sleep(Duration::from_millis(50));

    if try_cli_key_press(key_name) {
        return Ok(());
    }

    press_key_with_enigo(key_name)
}

fn try_cli_key_press(key_name: &str) -> bool {
    // Wayland: wtype -k <key>
    if std::env::var("WAYLAND_DISPLAY").is_ok()
        && try_command("wtype", &["-k", key_name])
    {
        return true;
    }

    // X11: xdotool key <key>
    if std::env::var("DISPLAY").is_ok() && try_command("xdotool", &["key", key_name]) {
        return true;
    }

    false
}

fn press_key_with_enigo(key_name: &str) -> Result<(), VoclipError> {
    use enigo::{Enigo, Key, Keyboard, Settings};

    let key = match key_name.to_lowercase().as_str() {
        "return" | "enter" => Key::Return,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "escape" => Key::Escape,
        "space" => Key::Space,
        "delete" => Key::Delete,
        "up" | "uparrow" => Key::UpArrow,
        "down" | "downarrow" => Key::DownArrow,
        "left" | "leftarrow" => Key::LeftArrow,
        "right" | "rightarrow" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "page_up" => Key::PageUp,
        "pagedown" | "page_down" => Key::PageDown,
        _ => {
            return Err(VoclipError::Keyboard(format!(
                "Unknown key: {key_name}. Supported: Return, BackSpace, Tab, Escape, Space, \
                 Delete, Up, Down, Left, Right, Home, End, PageUp, PageDown"
            )));
        }
    };

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| VoclipError::Keyboard(format!("Failed to initialize enigo: {e}")))?;

    enigo
        .key(key, enigo::Direction::Click)
        .map_err(|e| VoclipError::Keyboard(format!("Failed to press key: {e}")))?;

    Ok(())
}

fn type_with_enigo(text: &str) -> Result<(), VoclipError> {
    use enigo::{Enigo, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| VoclipError::Keyboard(format!("Failed to initialize enigo: {e}")))?;

    enigo
        .text(text)
        .map_err(|e| VoclipError::Keyboard(format!("Failed to type text: {e}")))?;

    Ok(())
}
