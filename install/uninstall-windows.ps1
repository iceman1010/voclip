#!/usr/bin/env pwsh
#
# voclip Windows uninstaller
#

$ErrorActionPreference = "Continue"

$removed = $false

function Remove-IfExists {
    param([string]$Path, [string]$Label)

    Write-Host ""
    Write-Host "=> Removing $Label..." -ForegroundColor Blue

    if (Test-Path $Path) {
        Remove-Item -Path $Path -Force
        Write-Host "[OK] Removed: $Path" -ForegroundColor Green
        $script:removed = $true
    } else {
        Write-Host "[!] Not found: $Path" -ForegroundColor Yellow
    }
}

function Main {
    Write-Host ""
    Write-Host "--- voclip Windows Uninstaller ---" -ForegroundColor Cyan
    Write-Host ""

    Remove-IfExists "$env:LOCALAPPDATA\voclip\voclip-run.bat" "wrapper script"
    Remove-IfExists "$env:LOCALAPPDATA\voclip\voclip-hotkey.ps1" "hotkey script"
    Remove-IfExists "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\voclip.lnk" "autostart shortcut"

    # Remove scheduled task
    Write-Host ""
    Write-Host "=> Removing hotkey service..." -ForegroundColor Blue
    $task = Get-ScheduledTask -TaskName "voclip-Hotkey" -ErrorAction SilentlyContinue
    if ($task) {
        Unregister-ScheduledTask -TaskName "voclip-Hotkey" -Confirm:$false
        Write-Host "[OK] Removed scheduled task: voclip-Hotkey" -ForegroundColor Green
        $script:removed = $true
    } else {
        Write-Host "[!] Scheduled task not found" -ForegroundColor Yellow
    }

    # Ask about API key
    $envPath = "$env:APPDATA\voclip\.env"
    if (Test-Path $envPath) {
        Write-Host ""
        $reply = Read-Host "Also remove API key file ($envPath)? (y/N)"
        if ($reply -match "^[Yy]$") {
            Remove-Item -Path $envPath -Force
            # Remove directory if empty
            $dir = Split-Path $envPath -Parent
            if ((Get-ChildItem $dir -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
                Remove-Item -Path $dir -Force
            }
            Write-Host "[OK] Removed: $envPath" -ForegroundColor Green
        } else {
            Write-Host "[!] Kept: $envPath" -ForegroundColor Yellow
        }
    }

    # Clean up empty voclip directory
    $voclipDir = "$env:LOCALAPPDATA\voclip"
    if (Test-Path $voclipDir) {
        if ((Get-ChildItem $voclipDir -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
            Remove-Item -Path $voclipDir -Force
            Write-Host "[OK] Removed empty directory: $voclipDir" -ForegroundColor Green
        }
    }

    Write-Host ""
    if ($script:removed) {
        Write-Host "Uninstallation complete." -ForegroundColor Green
    } else {
        Write-Host "Nothing to remove - voclip was not installed via the installer." -ForegroundColor Yellow
    }
    Write-Host ""
}

Main
