## Project Overview

voclip is a Rust CLI tool that captures microphone audio, streams it to AssemblyAI for real-time transcription, and copies the transcript to the clipboard. Supports Linux (X11/Wayland), macOS, and Windows.

## Build Commands

```bash
cargo build              # Development build
cargo build --release    # Release build (optimized)
cargo check              # Check for compilation errors (fast)
cargo run                # Run the application
cargo run -- --timeout 5 --model u3-rt-pro  # Run with arguments
cargo clippy -- -D warnings  # Lint
cargo fmt                # Format code
cargo fmt -- --check     # Format check (CI)
```

## Test Commands

No automated tests. Manual testing:

```bash
cargo build --release
./target/release/voclip --list-models
./target/release/voclip --help
./target/release/voclip --version
```

## Code Style Guidelines

### Imports

Group imports: external crates first (alphabetical), then internal modules with `crate::`:

```rust
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::VoclipError;
use crate::speech_model::SpeechModel;
```

### Formatting

- Use `cargo fmt` before committing
- Max line width: 100 characters
- Indent with 4 spaces
- No trailing whitespace

### Types and Naming

- **Functions/variables**: `snake_case`
- **Types (structs, enums, traits)**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Module/file names**: `snake_case.rs`

### Error Handling

Use `VoclipError` enum for all fallible operations. Add new variants to `src/error.rs`:

```rust
use crate::error::VoclipError;

pub fn do_something() -> Result<(), VoclipError> {
    let data = fs::read_to_string(path)
        .map_err(|e| VoclipError::Config(e.to_string()))?;
    Ok(())
}
```

Implement `From` for external errors when appropriate:

```rust
impl From<external_crate::Error> for VoclipError {
    fn from(e: external_crate::Error) -> Self {
        VoclipError::NewVariant(e.to_string())
    }
}
```

### Struct Organization

```rust
pub struct MyStruct {
    field_one: String,      // Private by default
    pub field_two: u32,     // Only pub when needed
}

impl MyStruct {
    pub fn new() -> Self { ... }
    pub fn public_method(&self) -> Result<(), VoclipError> { ... }
    fn helper(&self) { ... }  // Private helpers
}
```

### Async Code

Use `tokio` runtime with `#[tokio::main]` entry point. Channels: `tokio::sync::mpsc`. Use `tokio::select!` for concurrent operations.

### CLI Arguments

Use `clap` with derive macros in `src/config.rs`:

```rust
#[derive(Parser, Debug)]
#[command(name = "voclip", about = "Description")]
pub struct Args {
    #[arg(long)]
    pub flag_name: bool,

    #[arg(long, default_value_t = 3)]
    pub numeric_arg: u32,
}
```

### Platform-Specific Code

```rust
#[cfg(target_os = "linux")]
fn linux_specific() { ... }

#[cfg(not(target_os = "linux"))]
fn other_platforms() { ... }
```

Use `#[allow(dead_code)]` sparingly.

## Project Structure

```
src/
├── main.rs           # Entry point, CLI orchestration
├── config.rs         # CLI args, configuration loading
├── error.rs          # VoclipError enum
├── audio_capture.rs  # Microphone input via cpal
├── websocket.rs      # AssemblyAI WebSocket streaming
├── token.rs          # API token fetching
├── clipboard.rs      # Cross-platform clipboard
├── keyboard.rs       # Simulated keyboard typing (--type mode)
├── beep.rs           # Audio feedback tones
├── resample.rs       # Audio resampling
├── speech_model.rs   # Speech model enum
├── update.rs         # Self-update functionality
└── wakeword.rs       # Wake word detection and training (rustpotter)
```

## Key Dependencies

- `tokio` - Async runtime
- `clap` - CLI argument parsing
- `serde` / `serde_json` - Serialization
- `thiserror` - Error derive macros
- `cpal` - Cross-platform audio
- `arboard` - Clipboard access
- `rodio` - Audio playback
- `tokio-tungstenite` - WebSocket client
- `reqwest` - HTTP client
- `rustpotter` - Wake word detection and training

## Notes

- Rust edition 2024
- Releases via GitHub Actions (`.github/workflows/release.yml`)
- Config: `~/.config/voclip/config.toml`
- API key: `ASSEMBLYAI_API_KEY` env var or `.env` file
