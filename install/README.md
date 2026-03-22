# voclip Installer

Interactive installer for setting up voclip with a global keyboard shortcut.

## Features

- Detects voclip binary location
- Prompts for AssemblyAI API key (stored securely, not in scripts)
- Configurable keyboard shortcut
- Creates wrapper script with environment setup
- Sets up global hotkey via desktop environment
- Uninstaller to remove all changes

## Supported Platforms

| Platform | Desktop | Status |
|---------|---------|--------|
| Linux | GNOME | Supported |
| Linux | XFCE | Supported |
| Windows | - | Supported |
| macOS | - | Planned |

## Usage

### Install

```bash
git clone https://github.com/iceman1010/voclip.git
cd voclip
./install/install.sh
```

On Windows (PowerShell):
```powershell
.\install\install-windows.ps1
```

### Uninstall

Linux:
```bash
./install/uninstall.sh
```

Windows (PowerShell):
```powershell
.\install\uninstall-windows.ps1
```

## What it does

### Linux (GNOME)

1. Saves API key to `~/.config/voclip/.env` (chmod 600)
2. Creates wrapper script at `~/.local/bin/voclip-run`
3. Sets up global hotkey via GNOME Settings (`gsettings`)
4. Creates autostart entry at `~/.config/autostart/voclip.desktop`

### Windows

1. Saves API key to `%APPDATA%\voclip\.env` (restricted ACL)
2. Creates wrapper batch file at `%LOCALAPPDATA%\voclip\voclip-run.bat`
3. Registers global hotkey via scheduled task + PowerShell listener
4. Creates startup shortcut

## Customization

The installer asks for:
- **API Key**: Your AssemblyAI API key (stored in a separate .env file, not embedded in scripts)
- **Hotkey**: Keyboard shortcut (default: `Ctrl+F1`)

## Requirements

- voclip binary must be installed first
- Linux GNOME: `gsettings` command
- Linux XFCE: `xfconf-query` command
- Windows: PowerShell 5.1+ and Task Scheduler access
