#!/usr/bin/env pwsh
#
# voclip Windows installer
#

$ErrorActionPreference = "Stop"

$script:VoclipPath = ""
$script:ApiKey = ""
$script:Hotkey = "Ctrl+F1"
$script:EnvPath = "$env:APPDATA\voclip\.env"
$script:WrapperPath = "$env:LOCALAPPDATA\voclip\voclip-run.bat"
$script:HotkeyScriptPath = "$env:LOCALAPPDATA\voclip\voclip-hotkey.ps1"
$script:AutostartPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\voclip.lnk"

function Write-Step    { param([string]$Msg); Write-Host ""; Write-Host "=> $Msg" -ForegroundColor Blue }
function Write-Ok      { param([string]$Msg); Write-Host "[OK] $Msg" -ForegroundColor Green }
function Write-Warn    { param([string]$Msg); Write-Host "[!] $Msg" -ForegroundColor Yellow }
function Write-Fail    { param([string]$Msg); Write-Host "[X] $Msg" -ForegroundColor Red }
function Write-Info    { param([string]$Msg); Write-Host "[i] $Msg" -ForegroundColor Cyan }

# Find voclip binary
function Find-Voclip {
    Write-Step "Searching for voclip binary..."

    $cmd = Get-Command voclip -ErrorAction SilentlyContinue
    if ($cmd) {
        $script:VoclipPath = $cmd.Source
        Write-Ok "Found: $script:VoclipPath"
        return
    }

    $paths = @(
        "$env:USERPROFILE\.cargo\bin\voclip.exe",
        "$env:LOCALAPPDATA\voclip\voclip.exe",
        "C:\Program Files\voclip\voclip.exe"
    )

    foreach ($p in $paths) {
        if (Test-Path $p) {
            $script:VoclipPath = $p
            Write-Ok "Found: $p"
            return
        }
    }

    Write-Fail "voclip not found!"
    Write-Host ""
    Write-Host "Please install voclip first:"
    Write-Host "  - Download from: https://github.com/iceman1010/voclip/releases"
    Write-Host "  - Or install via cargo: cargo install voclip"
    exit 1
}

# Ask for API key
function Ask-ApiKey {
    Write-Host ""
    Write-Host "--- AssemblyAI API Key ---"
    Write-Host "You need an API key from AssemblyAI to use voclip."
    Write-Host "Get one free at: https://www.assemblyai.com/"
    Write-Host ""

    # Check for existing key
    if (Test-Path $script:EnvPath) {
        $content = Get-Content $script:EnvPath -Raw -ErrorAction SilentlyContinue
        if ($content -match 'ASSEMBLYAI_API_KEY=(.+)') {
            $existing = $matches[1].Trim()
            if ($existing.Length -gt 8) {
                $masked = $existing.Substring(0, 4) + "..." + $existing.Substring($existing.Length - 4)
                Write-Info "Existing API key found: $masked"
                $reply = Read-Host "Keep existing key? (Y/n)"
                if ($reply -notmatch "^[Nn]$") {
                    $script:ApiKey = $existing
                    return
                }
            }
        }
    }

    $script:ApiKey = Read-Host "Enter your AssemblyAI API key"

    if ([string]::IsNullOrWhiteSpace($script:ApiKey)) {
        Write-Fail "API key cannot be empty!"
        exit 1
    }
}

# Ask for hotkey
function Ask-Hotkey {
    Write-Host ""
    Write-Host "--- Keyboard Hotkey ---"
    Write-Host "Type the hotkey combination you want to use."
    Write-Host "Examples: Ctrl+F1, Alt+Space, F12"
    Write-Host "Press Enter for default: Ctrl+F1"
    Write-Host ""

    $input = Read-Host "Enter hotkey (e.g., Ctrl+F1)"

    if ([string]::IsNullOrWhiteSpace($input)) {
        $script:Hotkey = "Ctrl+F1"
    } else {
        $script:Hotkey = $input
    }

    Write-Ok "Using hotkey: $script:Hotkey"
}

# Confirm installation
function Confirm-Install {
    Write-Host ""
    Write-Host "--- Installation Summary ---"
    Write-Host "  voclip binary:  $script:VoclipPath"
    Write-Host "  API key file:   $script:EnvPath"
    Write-Host "  Wrapper script: $script:WrapperPath"
    Write-Host "  Hotkey:         $script:Hotkey"
    Write-Host "  Autostart:      Enabled"
    Write-Host ""

    $reply = Read-Host "Continue with installation? (Y/n)"
    if ($reply -match "^[Nn]$") {
        Write-Host "Installation cancelled."
        exit 0
    }
}

# Save API key
function Save-ApiKey {
    Write-Step "Saving API key..."

    $dir = Split-Path $script:EnvPath -Parent
    if (!(Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }

    Set-Content -Path $script:EnvPath -Value "ASSEMBLYAI_API_KEY=$($script:ApiKey)" -Encoding UTF8

    # Restrict permissions to current user only
    $acl = Get-Acl $script:EnvPath
    $acl.SetAccessRuleProtection($true, $false)
    $rule = New-Object System.Security.AccessControl.FileSystemAccessRule(
        $env:USERNAME, "FullControl", "Allow"
    )
    $acl.SetAccessRule($rule)
    Set-Acl -Path $script:EnvPath -AclObject $acl

    Write-Ok "Saved to: $script:EnvPath (restricted permissions)"
}

# Create wrapper batch file
function Create-Wrapper {
    Write-Step "Creating wrapper script..."

    $dir = Split-Path $script:WrapperPath -Parent
    if (!(Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }

    # Read key from .env at runtime rather than embedding it
    @"
@echo off
REM voclip wrapper script - generated by voclip installer

REM Load API key from config
for /f "tokens=1,* delims==" %%a in ('type "$($script:EnvPath)"') do (
    set "%%a=%%b"
)

REM Run voclip
"$($script:VoclipPath)" %*
"@ | Out-File -FilePath $script:WrapperPath -Encoding ASCII

    Write-Ok "Created: $script:WrapperPath"
}

# Parse hotkey string into modifier and VK values
function Parse-Hotkey {
    param([string]$HotkeyStr)

    $parts = $HotkeyStr.Split('+') | ForEach-Object { $_.Trim() }
    $modifier = 0
    $vk = 0

    foreach ($part in $parts) {
        switch ($part.ToUpper()) {
            "CTRL"    { $modifier = $modifier -bor 0x0002 }
            "CONTROL" { $modifier = $modifier -bor 0x0002 }
            "ALT"     { $modifier = $modifier -bor 0x0001 }
            "SHIFT"   { $modifier = $modifier -bor 0x0004 }
            "WIN"     { $modifier = $modifier -bor 0x0008 }
            "SUPER"   { $modifier = $modifier -bor 0x0008 }
            "SPACE"   { $vk = 0x20 }
            default {
                if ($part -match "^F(\d+)$") {
                    $fnum = [int]$matches[1]
                    $vk = 0x6F + $fnum  # F1=0x70, F2=0x71, etc.
                } elseif ($part.Length -eq 1) {
                    $vk = [int][char]$part.ToUpper()
                }
            }
        }
    }

    return @{ Modifier = $modifier; VK = $vk }
}

# Create hotkey listener script and scheduled task
function Setup-Hotkey {
    Write-Step "Setting up global hotkey..."

    $parsed = Parse-Hotkey $script:Hotkey
    if ($parsed.VK -eq 0) {
        Write-Fail "Could not parse hotkey: $($script:Hotkey)"
        Write-Warn "Skipping hotkey registration. You can set it up manually."
        return
    }

    $escapedWrapper = $script:WrapperPath -replace '\\', '\\\\'

    $hotkeyScript = @"
# voclip hotkey listener - generated by installer
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public class VoclipHotkey {
    [DllImport("user32.dll")]
    public static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);

    [DllImport("user32.dll")]
    public static extern bool UnregisterHotKey(IntPtr hWnd, int id);

    public const int HOTKEY_ID = 9000;
}

public class VoclipForm : Form {
    protected override void WndProc(ref Message m) {
        base.WndProc(ref m);
        if (m.Msg == 0x0312 && m.WParam.ToInt32() == VoclipHotkey.HOTKEY_ID) {
            System.Diagnostics.Process.Start("$escapedWrapper");
        }
    }
}
'@ -ReferencedAssemblies System.Windows.Forms

`$form = [VoclipForm]::new()
`$form.ShowInTaskbar = `$false
`$form.WindowState = [System.Windows.Forms.FormWindowState]::Minimized

if (-not [VoclipHotkey]::RegisterHotKey(`$form.Handle, [VoclipHotkey]::HOTKEY_ID, $($parsed.Modifier), $($parsed.VK))) {
    Write-Host "Failed to register hotkey. It may already be in use."
    exit 1
}

Write-Host "voclip hotkey active: $($script:Hotkey)"
[System.Windows.Forms.Application]::Run(`$form)
"@

    $dir = Split-Path $script:HotkeyScriptPath -Parent
    if (!(Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Set-Content -Path $script:HotkeyScriptPath -Value $hotkeyScript -Encoding UTF8

    # Register as scheduled task for login
    $taskName = "voclip-Hotkey"
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue

    $action = New-ScheduledTaskAction `
        -Execute "powershell.exe" `
        -Argument "-WindowStyle Hidden -ExecutionPolicy Bypass -File `"$($script:HotkeyScriptPath)`""
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable

    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger `
        -Principal $principal -Settings $settings -Force | Out-Null

    Write-Ok "Hotkey registered: $($script:Hotkey)"
    Write-Info "The hotkey listener will start on your next login."
    Write-Info "To start it now, run: Start-ScheduledTask -TaskName '$taskName'"
}

# Create autostart shortcut
function Create-Autostart {
    Write-Step "Creating autostart shortcut..."

    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($script:AutostartPath)
    $shortcut.TargetPath = $script:WrapperPath
    $shortcut.Arguments = "--update"
    $shortcut.Description = "voclip - check for updates on login"
    $shortcut.Save()

    Write-Ok "Created: $($script:AutostartPath)"
    Write-Info "On login, voclip will check for updates automatically."
}

# Finalize
function Show-Complete {
    Write-Host ""
    Write-Host "=========================================" -ForegroundColor Green
    Write-Host "       Installation Complete!" -ForegroundColor Green
    Write-Host "=========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "  [OK] API key saved (restricted permissions)" -ForegroundColor Green
    Write-Host "  [OK] Wrapper script created" -ForegroundColor Green
    Write-Host "  [OK] Hotkey registered ($($script:Hotkey))" -ForegroundColor Green
    Write-Host "  [OK] Autostart enabled" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Press your hotkey to start voice recording."
    Write-Host "  Uninstall: install\uninstall-windows.ps1"
    Write-Host ""
}

# Main
function Main {
    Write-Host ""
    Write-Host "--- voclip Windows Installer ---" -ForegroundColor Cyan
    Write-Host ""

    Find-Voclip
    Ask-ApiKey
    Ask-Hotkey
    Confirm-Install

    Save-ApiKey
    Create-Wrapper
    Create-Autostart
    Setup-Hotkey
    Show-Complete
}

Main
