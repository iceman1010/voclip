# voclip Installer

Interactive installer for setting up voclip with a global keyboard shortcut.

## Features

- Detects voclip binary location
- Prompts for AssemblyAI API key
- Configurable keyboard shortcut
- Creates wrapper script with environment variables
- Sets up global hotkey via desktop environment
- Uninstaller to remove all changes

## Supported Platforms

| Platform | Desktop | Status |
|---------|---------|--------|
| Linux | GNOME | ✅ Supported |
| Linux | KDE | 🔜 Planned |
| Windows | - | 🔜 Planned |
| macOS | - | 🔜 Planned |

## Usage

### Install

```bash
git clone https://github.com/iceman1010/voclip.git
cd voclip
./install/install.sh
```

### Uninstall

```bash
./install/uninstall.sh
```

## What it does

### Linux (GNOME)

1. Creates wrapper script at `~/.local/bin/voclip-run`
2. Sets up global hotkey via GNOME Settings (`gsettings`)
3. Creates autostart entry at `~/.config/autostart/voclip.desktop`

### Windows (Planned)

1. Creates wrapper batch file
2. Registers global hotkey via registry
3. Creates startup entry

## Customization

The installer asks for:
- **API Key**: Your AssemblyAI API key (stored in wrapper script)
- **Hotkey**: Keyboard shortcut (default: `Ctrl+F1`)

## Requirements

- voclip binary must be installed first
- For Linux: `gsettings` command (part of GNOME)
