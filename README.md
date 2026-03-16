# voclip

Voice to clipboard — speak and paste. A CLI tool that listens to your microphone, streams audio to AssemblyAI for real-time transcription, and copies the final transcript to your clipboard with an audible beep.

No UI, no browser — just run `voclip`, speak, and paste.

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

## Configuration

Set your AssemblyAI API key as an environment variable:

```bash
export ASSEMBLYAI_API_KEY=your_key_here
```

Or create a `.env` file in the directory you run voclip from:

```
ASSEMBLYAI_API_KEY=your_key_here
```

## Usage

```
voclip [OPTIONS]

Options:
  --timeout <SECONDS>    Silence timeout in seconds (default: 3)
  --language <CODE>      Language code or "multi" for auto-detect (default: "multi")
  -h, --help             Print help
```

### Examples

```bash
# Default: auto-detect language, 3s silence timeout
voclip

# English only, 5 second silence timeout
voclip --language en --timeout 5

# Quick dictation with short silence timeout
voclip --timeout 2
```

### How it works

1. Authenticates with AssemblyAI and gets a temporary streaming token
2. Opens your default microphone (rising beep confirms recording started)
3. Streams audio to AssemblyAI via WebSocket for real-time transcription
4. Shows partial transcripts on stderr as you speak
5. After the silence timeout, copies the final transcript to your clipboard
6. Plays a falling beep to confirm, then exits

Press `Ctrl+C` at any time to stop early.

## Supported Platforms

| Platform | Audio | Clipboard | Notes |
|----------|-------|-----------|-------|
| Linux (X11) | ALSA | xclip / xsel | Needs `libasound2-dev` to build |
| Linux (Wayland) | ALSA | wl-clipboard | Needs `libasound2-dev` to build |
| macOS | CoreAudio | pbcopy | Grant mic permission on first run |
| Windows | WASAPI | Native | Works out of the box |

## License

MIT
