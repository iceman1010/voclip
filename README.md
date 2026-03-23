<p align="center">
  <img src="favicon/android-chrome-192x192.png" alt="voclip icon" width="128">
</p>

<h1 align="center">voclip</h1>

<p align="center">Voice to clipboard — speak and paste.</p> A CLI tool that listens to your microphone, streams audio to AssemblyAI for real-time transcription, and copies the final transcript to your clipboard or types it directly via keyboard simulation.

Includes local wake word detection — say a custom phrase (e.g., "hey voclip") and it starts transcribing hands-free. You can also train **command words** that trigger keyboard actions like pressing Enter or Backspace. No UI, no browser — just run `voclip`, speak, and paste.

## Demo

```
$ voclip
Authenticating...
Listening... (speak, then wait 3s silence to finish, or Ctrl+C)
Session started.
Hello, this is a test of voice to clipboard.
Copied to clipboard: Hello, this is a test of voice to clipboard.
```

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) toolchain
- An [AssemblyAI](https://www.assemblyai.com/) API key

#### Linux

```bash
sudo apt install libasound2-dev pkg-config libssl-dev  # build deps
sudo apt install xclip                                  # clipboard (X11)
# or: sudo apt install wl-clipboard                     # clipboard (Wayland)
sudo apt install xdotool                                # keyboard typing (X11, for --type/--listen)
# or: sudo apt install wtype                             # keyboard typing (Wayland)
```

#### macOS

No extra dependencies. Terminal will request microphone permission on first run.

#### Windows

No extra dependencies.

### Build

```bash
git clone https://github.com/iceman1010/voclip.git
cd voclip
cargo build --release
```

The binary is at `target/release/voclip`. Copy it to a directory in your `$PATH`:

```bash
cp target/release/voclip ~/.local/bin/
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/iceman1010/voclip/releases):

- **Linux**: Requires GLIBC 2.39+ (Ubuntu 24.04+, Debian 12+, Fedora 40+, etc.)
- **macOS**: Universal binary (Intel + Apple Silicon)
- **Windows**: x86_64 binary

> **Note for older Linux distributions**: If you see an error like `GLIBC_2.XX not found`,
> your system is too old for the pre-built binaries. Please build from source instead:
> ```bash
> cargo install voclip
> ```

## Configuration

### Get an API key

1. Go to [assemblyai.com](https://www.assemblyai.com/) and create a free account
2. Navigate to your [Dashboard](https://www.assemblyai.com/app)
3. Copy your API key from the dashboard

### Set the API key

#### Linux / macOS

Add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
export ASSEMBLYAI_API_KEY=your_key_here
```

Then reload your shell (`source ~/.bashrc`) or open a new terminal.

#### Windows (Command Prompt)

Set it permanently for your user account:

```cmd
setx ASSEMBLYAI_API_KEY your_key_here
```

Then open a new Command Prompt window (setx changes don't apply to the current window).

#### Windows (PowerShell)

```powershell
[System.Environment]::SetEnvironmentVariable("ASSEMBLYAI_API_KEY", "your_key_here", "User")
```

Then open a new PowerShell window.

#### Alternative: .env file (all platforms)

Create a `.env` file in the directory you run voclip from:

```
ASSEMBLYAI_API_KEY=your_key_here
```

## Usage

```
voclip [OPTIONS]

Options:
  --timeout <SECONDS>             Silence timeout in seconds (default: 3)
  --model <MODEL>                 Speech model (u3-rt-pro, english, multilingual, whisper-rt)
  --delay <SECONDS>               Delay before recording starts (default: 1)
  --type                          Type text via keyboard instead of copying to clipboard
  --listen                        Always-on mode with wake word and command word detection
  --train-wakeword                Train the wake word (triggers transcription)
  --train-command                  Train a command word (triggers a key press action)
  --test-wakeword                 Test/debug detection of all trained voice patterns
  --wakeword-name <NAME>          Name for the wake word (used with --train-wakeword)
  --command-name <NAME>           Name for the command word (used with --train-command)
  --command-action <ACTION>       Action for the command word: "key:<keyname>" (used with --train-command)
  --wakeword-samples <N>          Number of training samples (default: 8)
  --wakeword-sensitivity <LEVEL>  Detection sensitivity: low, medium, high, or a number (default: medium)
  --list-wakewords                List all configured wake word and command words
  --remove-wakeword <NAME>        Remove a trained voice pattern by name
  --list-devices                  List available audio input devices
  --audio-device <NAME>           Audio input device (substring match, saved to config)
  --list-models                   List available speech models
  --set-default-model <MODEL>     Save default speech model to config
  --set-default-timeout <SECS>    Save default timeout to config
  --update                        Check for updates and self-update
  --version                       Print version
  -h, --help                      Print help
```

### Examples

```bash
# Default: one-shot transcription to clipboard
voclip

# Type directly instead of clipboard
voclip --type

# Quick dictation with short silence timeout
voclip --timeout 2

# Use a specific speech model
voclip --model english
```

### Wake Word & Command Words

Train a **wake word** for hands-free transcription, and **command words** for keyboard actions:

```bash
# Train the wake word (triggers transcription)
voclip --train-wakeword --wakeword-name "Computer"

# Train command words (trigger key presses)
voclip --train-command --command-name "press enter" --command-action "key:Return"
voclip --train-command --command-name "go back" --command-action "key:BackSpace"

# Test all trained patterns
voclip --test-wakeword

# Run in always-on mode
voclip --listen

# List all configured patterns
voclip --list-wakewords
```

Wake word detection runs entirely locally using [rustpotter](https://github.com/GiviMAD/rustpotter) — no cloud API needed for detection. Only the transcription phase uses AssemblyAI.

You can tune detection with `--wakeword-sensitivity`:
- `low` — fewer false positives, may miss some utterances
- `medium` (default) — balanced
- `high` — catches more, may occasionally false-trigger
- A number like `0.5` for fine-grained control

### How it works

**One-shot mode** (default):
1. Authenticates with AssemblyAI and gets a temporary streaming token
2. Opens your default microphone (rising beep confirms recording started)
3. Streams audio to AssemblyAI via WebSocket for real-time transcription
4. Shows partial transcripts on stderr as you speak
5. After the silence timeout, copies the transcript to clipboard (or types it with `--type`)
6. Plays a falling beep to confirm, then exits

**Listen mode** (`--listen`):
1. Continuously listens for all trained voice patterns (local, ~2% CPU)
2. Wake word detected → plays a beep, transcribes via AssemblyAI, types the text
3. Command word detected → plays a beep and presses the configured key
4. Returns to listening

Press `Ctrl+C` at any time to stop.

### Auto-Update

voclip can update itself to the latest version:

```bash
voclip --update
```

### Keyboard Shortcut

See the [installer documentation](install/README.md) for setting up a global hotkey to trigger voclip.

## Supported Platforms

| Platform | Audio | Clipboard | Keyboard Typing | Notes |
|----------|-------|-----------|-----------------|-------|
| Linux (X11) | ALSA | xclip / xsel | xdotool / enigo | Needs `libasound2-dev` to build |
| Linux (Wayland) | ALSA | wl-clipboard | wtype / ydotool / enigo | Needs `libasound2-dev` to build |
| macOS | CoreAudio | pbcopy | enigo | Grant mic permission on first run |
| Windows | WASAPI | Native | enigo | Works out of the box |

## License

MIT
