use crossterm::style::Stylize;

/// Print styled "voclip vX.Y.Z" header to stderr.
pub fn header() {
    eprintln!(
        "\n{}",
        format!("voclip {}", env!("CARGO_PKG_VERSION"))
            .bold()
            .cyan()
    );
}

/// Print a cyan "●" prefixed info message to stderr.
pub fn info(msg: &str) {
    eprintln!("{} {}", "●".cyan(), msg);
}

/// Print a green "✓" prefixed success message to stderr.
pub fn success(msg: &str) {
    eprintln!("{} {}", "✓".green(), msg);
}

/// Print a red "✗" prefixed error message to stderr.
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// Print a yellow "⚠" prefixed warning message to stderr.
pub fn warn(msg: &str) {
    eprintln!("{} {}", "⚠".yellow(), msg);
}

/// Print a "key: value" line with bold key to stderr.
pub fn label(key: &str, value: &str) {
    eprintln!("  {} {}", format!("{key}:").bold(), value);
}

/// Print dimmed text to stderr.
pub fn dim(msg: &str) {
    eprintln!("{}", msg.dim());
}
