#!/usr/bin/env pwsh
#
# voclip Windows installer
#

$ErrorActionPreference = "Stop"

# Colors (ANSI codes work in modern PowerShell)
$RED = "`e[0;31m"
$GREEN = "`e[0;32m"
$YELLOW = "`e[1;33m"
$BLUE = "`e[0;34m"
$CYAN = "`e[0;36m"
$BOLD = "`e[1m"
$RESET = "`e[0m"

function Write-Step {
    Write-Host ""
    Write-Host "$BLUE➜$RESET $BOLD$args$RESET"
}

function Write-Success {
    Write-Host "$GREEN✓$RESET $args"
}

function Write-Error {
    Write-Host "$RED✗$RESET $args" 2>&1
}

function Write-Warning {
    Write-Host "$YELLOW⚠$RESET $args"
}

function Write-Info {
    Write-Host "$CYANℹ$RESET $args"
}

function Write-Question {
    param([string]$Prompt)
    Write-Host -NoNewline "$BOLD$Prompt$RESET "
}

# Find voclip binary
function Find-Voclip {
    Write-Step "Searching for voclip binary..."
    
    # Try where command first
    $whereResult = Get-Command voclip -ErrorAction SilentlyContinue
    if ($whereResult) {
        Write-Success "Found: $($whereResult.Source)"
        return $whereResult.Source
    }
    
    # Check common install locations
    $paths = @(
        "C:\Program Files\voclip\voclip.exe",
        "C:\Program Files\voclip\bin\voclip.exe",
        "$env:LOCALAPPDATA\voclip\voclip.exe"
    )
    
    foreach ($path in $paths) {
        if (Test-Path $path) {
            Write-Success "Found: $path"
            return $path
        }
    }
    
    Write-Error "voclip not found!"
    Write-Host ""
    Write-Host "Please install voclip first:"
    Write-Host "  - Download from: https://github.com/iceman1010/voclip/releases"
    Write-Host "  - Or install via cargo: cargo install voclip"
    exit 1
}

# Ask for API key
function Ask-ApiKey {
    Write-Host ""
    Write-Host "$CYAN┌─────────────────────────────────────────────────────────────┐$RESET"
    Write-Host "$CYAN│$RESET                   ${BOLD}AssemblyAI API Key${RESET}                         $CYAN│$RESET"
    Write-Host "$CYAN├─────────────────────────────────────────────────────────────┤$RESET"
    Write-Host "$CYAN│$RESET You need an API key from AssemblyAI to use voclip.         $CYAN│$RESET"
    Write-Host "$CYAN│$RESET Get one free at: https://www.assemblyai.com/               $CYAN│$RESET"
    Write-Host "$CYAN└─────────────────────────────────────────────────────────────┘$RESET"
    Write-Host ""
    Write-Question "Enter your AssemblyAI API key:"
    $script:ApiKey = Read-Host -AsSecureString
    $script:ApiKey = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($script:ApiKey))
    
    if ([string]::IsNullOrWhiteSpace($script:ApiKey)) {
        Write-Error "API key cannot be empty!"
        exit 1
    }
}

# Ask for hotkey
function Ask-Hotkey {
    Write-Host ""
    Write-Host "$CYAN┌─────────────────────────────────────────────────────────────┐$RESET"
    Write-Host "$CYAN│$RESET                      ${BOLD}Keyboard Hotkey${RESET}                        $CYAN│$RESET"
    Write-Host "$CYAN├─────────────────────────────────────────────────────────────┤$RESET"
    Write-Host "$CYAN│$RESET Type the hotkey combination you want to use.               $CYAN│$RESET"
    Write-Host "$CYAN│$RESET Examples: Ctrl+F1, Alt+Space, F12                          $CYAN│$RESET"
    Write-Host "$CYAN│$RESET Press Enter for default: ${BOLD}Ctrl+F1${RESET}                            $CYAN│$RESET"
    Write-Host "$CYAN└─────────────────────────────────────────────────────────────┘$RESET"
    Write-Host ""
    Write-Question "Enter hotkey (e.g., Ctrl+F1):"
    $hotkeyInput = Read-Host
    
    if ([string]::IsNullOrWhiteSpace($hotkeyInput)) {
        $script:Hotkey = "Ctrl+F1"
    } else {
        $script:Hotkey = $hotkeyInput
    }
    
    Write-Success "Using hotkey: $script:Hotkey"
}

# Confirm installation
function Confirm-Install {
    Write-Host ""
    Write-Host "$CYAN┌─────────────────────────────────────────────────────────────┐$RESET"
    Write-Host "$CYAN│$RESET                   ${BOLD}Installation Summary${RESET}                        $CYAN│$RESET"
    Write-Host "$CYAN├─────────────────────────────────────────────────────────────┤$RESET"
    Write-Host "$CYAN│$RESET voclip path:          ${BOLD}$VoclipPath${RESET}" 
    $padding = 59 - $VoclipPath.Length
    if ($padding -gt 0) { Write-Host "$CYAN│$RESET$(' ' * $padding)│$RESET" }
    Write-Host "$CYAN│$RESET Wrapper script:       ${BOLD}$WrapperPath${RESET}"
    $padding = 59 - $WrapperPath.Length
    if ($padding -gt 0) { Write-Host "$CYAN│$RESET$(' ' * $padding)│$RESET" }
    Write-Host "$CYAN│$RESET Hotkey:               ${BOLD}$Hotkey${RESET}                            $CYAN│$RESET"
    Write-Host "$CYAN│$RESET Autostart:            ${BOLD}Enabled${RESET}                              $CYAN│$RESET"
    Write-Host "$CYAN└─────────────────────────────────────────────────────────────┘$RESET"
    Write-Host ""
    Write-Warning "This will create a wrapper script and register a global hotkey."
    Write-Host ""
    $response = Read-Host "Continue with installation? (Y/n)"
    if ($response -match "^n|N$") {
        Write-Host "Installation cancelled."
        exit 0
    }
}

# Create wrapper script
function Create-Wrapper {
    Write-Step "Creating wrapper script..."
    
    $wrapperDir = Split-Path $WrapperPath -Parent
    if (!(Test-Path $wrapperDir)) {
        New-Item -ItemType Directory -Path $wrapperDir -Force | Out-Null
    }
    
    # Create wrapper batch file
    @"
@echo off
REM voclip wrapper script - generated by voclip installer
REM Do not edit manually - re-run installer to update

set "ASSEMBLYAI_API_KEY=$ApiKey"

REM Try to find pulse audio server
if exist "%XDG_RUNTIME_DIR%\pulse\native" (
    set "PULSE_SERVER=unix:%XDG_RUNTIME_DIR%\pulse\native"
)

REM Run voclip
"$VoclipPath" %*
"@ | Out-File -FilePath $WrapperPath -Encoding ASCII
    
    Write-Success "Created: $WrapperPath"
}

# Create autostart shortcut
function Create-Autostart {
    Write-Step "Creating autostart shortcut..."
    
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($AutostartPath)
    $shortcut.TargetPath = "cmd.exe"
    $shortcut.Arguments = "/c `"$WrapperPath`""
    $shortcut.Description = "voclip - Voice to clipboard"
    $shortcut.Save()
    
    Write-Success "Created: $AutostartPath"
}

# Setup Windows hotkey using PowerShell
function Setup-Hotkey {
    Write-Step "Setting up global hotkey..."
    
    # For Windows, we'll use a PowerShell script that registers a hotkey via .NET
    # This creates a helper script that runs in background
    
    $hotkeyScript = @"
# voclip hotkey registration - generated by installer
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public class VoclipHotkey {
    [DllImport("user32.dll")]
    public static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);
    
    [DllImport("user32.dll")]
    public static extern bool UnregisterHotKey(IntPtr hWnd, int id);
    
    public const int HOTKEY_ID = 9000;
    
    // Modifier keys
    public const uint MOD_ALT = 0x0001;
    public const uint MOD_CONTROL = 0x0002;
    public const uint MOD_SHIFT = 0x0004;
    public const uint MOD_WIN = 0x0008;
    
    // Virtual key codes
    public const uint VK_F1 = 0x70;
    public const uint VK_F2 = 0x71;
    public const uint VK_F3 = 0x72;
    public const uint VK_F12 = 0x7B;
}

public class VoclipForm : Form {
    protected override void WndProc(ref Message m) {
        base.WndProc(ref m);
        
        if (m.Msg == 0x0312) {  // WM_HOTKEY
            int hotkeyId = m.WParam.ToInt32();
            if (hotkeyId == VoclipHotkey.HOTKEY_ID) {
                System.Diagnostics.Process.Start("$($WrapperPath -replace '\\', '\\\\')");
            }
        }
    }
}
"@

# Parse hotkey
`$hotkey = "$Hotkey".ToUpper()
`$modifier = 0
`$vk = 0

if (`$hotkey -match "CTRL") { `$modifier += [VoclipHotkey]::MOD_CONTROL }
if (`$hotkey -match "ALT") { `$modifier += [VoclipHotkey]::MOD_ALT }
if (`$hotkey -match "SHIFT") { `$modifier += [VoclipHotkey]::MOD_SHIFT }
if (`$hotkey -match "WIN") { `$modifier += [VoclipHotkey]::MOD_WIN }

# Extract key
if (`$hotkey -match "F(\d+)") { 
    `$fnum = [int]`$matches[1]
    `$vk = 0x70 + `$fnum - 1  # F1 = 0x70, etc.
}

if (`$vk -eq 0) {
    Write-Host "Could not parse hotkey: $Hotkey"
    exit 1
}

`$form = [VoclipForm]::new()
`$form.Text = "voclip Hotkey"
`$form.ShowInTaskbar = $false
`$form.WindowState = [FormWindowState]::Minimized

if (-not [VoclipHotkey]::RegisterHotKey(`$form.Handle, [VoclipHotkey]::HOTKEY_ID, `$modifier, `$vk)) {
    Write-Host "Failed to register hotkey. It may already be in use."
    exit 1
}

Write-Host "Hotkey registered: $Hotkey"
[Application]::Run(`$form)
"@
    
    # Save hotkey registration script
    $hotkeyScriptPath = "$env:TEMP\voclip-hotkey.ps1"
    # Escape the backticks properly
    $hotkeyScript = $hotkeyScript -replace '`\$', '$' -replace '`\$', '$'
    Set-Content -Path $hotkeyScriptPath -Value $hotkeyScript -Encoding UTF8
    
    # Create a scheduled task to run on login
    $taskName = "voclip-Hotkey"
    
    # Remove existing task if present
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    
    # Create action to run PowerShell with the hotkey script
    $action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-WindowStyle Hidden -ExecutionPolicy Bypass -File `"$hotkeyScriptPath`""
    
    # Create trigger for logon
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    
    # Create principal to run as current user
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    
    # Create settings
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
    
    # Register the task
    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null
    
    Write-Success "Hotkey registered: $Hotkey"
    Write-Info "The hotkey service will start on your next login."
}

# Finalize
function Finalize {
    Write-Host ""
    Write-Host "$GREEN╔═══════════════════════════════════════════════════════════╗$RESET"
    Write-Host "$GREEN║$RESET              ${BOLD}Installation Complete!${RESET}                       $GREEN║$RESET"
    Write-Host "$GREEN╠═══════════════════════════════════════════════════════════╣$RESET"
    Write-Host "$GREEN║$RESET                                                           $GREEN║$RESET"
    Write-Host "$GREEN║$RESET   ${GREEN}✓${RESET} voclip wrapper created                               $GREEN║$RESET"
    Write-Host "$GREEN║$RESET   ${GREEN}✓${RESET} Hotkey registered ($Hotkey)                        $GREEN║$RESET"
    Write-Host "$GREEN║$RESET   ${GREEN}✓${RESET} Autostart enabled                                   $GREEN║$RESET"
    Write-Host "$GREEN║$RESET                                                           $GREEN║$RESET"
    Write-Host "$GREEN╠═══════════════════════════════════════════════════════════╣$RESET"
    Write-Host "$GREEN║$RESET                                                           $GREEN║$RESET"
    Write-Host "$GREEN║$RESET   Usage:                                                   $GREEN║$RESET"
    Write-Host "$GREEN║$RESET     Press ${BOLD}$Hotkey${RESET} to start voice recording            $GREEN║$RESET"
    Write-Host "$GREEN║$RESET                                                           $GREEN║$RESET"
    Write-Host "$GREEN║$RESET   Uninstall:                                               $GREEN║$RESET"
    Write-Host "$GREEN║$RESET     .\install\uninstall.sh                                 $GREEN║$RESET"
    Write-Host "$GREEN║$RESET                                                           $GREEN║$RESET"
    Write-Host "$GREEN╚═══════════════════════════════════════════════════════════╝$RESET"
    Write-Host ""
}

# Main
function Main {
    Write-Host ""
    Write-Host "$CYAN╭─────────────────────────────────────────────────────────╮$RESET"
    Write-Host "$CYAN│$RESET           ${BOLD}voclip Windows Installer${RESET}                    $CYAN│$RESET"
    Write-Host "$CYAN╰─────────────────────────────────────────────────────────╯$RESET"
    
    # Find voclip
    $script:VoclipPath = Find-Voclip
    
    # Paths
    $script:WrapperPath = "$env:LOCALAPPDATA\voclip\voclip-run.bat"
    $script:AutostartPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\voclip.lnk"
    
    # Interactive setup
    Ask-ApiKey
    Ask-Hotkey
    Confirm-Install
    
    # Install
    Create-Wrapper
    Create-Autostart
    Setup-Hotkey
    Finalize
}

Main
