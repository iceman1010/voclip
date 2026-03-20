#!/usr/bin/env bash
#
# voclip uninstaller - Linux & Windows
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

print_step() {
    echo ""
    echo -e "${BLUE}➜${RESET} ${BOLD}$1${RESET}"
}

print_success() {
    echo -e "${GREEN}✓${RESET} $1"
}

print_error() {
    echo -e "${RED}✗${RESET} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}⚠${RESET} $1"
}

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
        echo "windows"
    else
        echo "unknown"
    fi
}

# Linux uninstaller
uninstall_linux() {
    echo ""
    echo -e "${CYAN}╭─────────────────────────────────────────────────────────╮${RESET}"
    echo -e "${CYAN}│${RESET}           ${BOLD}voclip Linux Uninstaller${RESET}                    ${CYAN}│${RESET}"
    echo -e "${CYAN}╰─────────────────────────────────────────────────────────╯${RESET}"
    
    local removed=false
    
    # Remove wrapper script
    print_step "Removing wrapper script..."
    local wrapper="$HOME/.local/bin/voclip-run"
    if [[ -f "$wrapper" ]]; then
        rm -f "$wrapper"
        print_success "Removed: $wrapper"
        removed=true
    else
        print_warning "Wrapper not found: $wrapper"
    fi
    
    # Remove desktop file
    print_step "Removing autostart desktop file..."
    local desktop="$HOME/.config/autostart/voclip.desktop"
    if [[ -f "$desktop" ]]; then
        rm -f "$desktop"
        print_success "Removed: $desktop"
        removed=true
    else
        print_warning "Desktop file not found: $desktop"
    fi
    
    # Remove GNOME hotkey
    print_step "Removing GNOME hotkey registration..."
    local CUSTOM_KEYS="org.gnome.settings-daemon.plugins.media-keys.custom-keybindings"
    local found=false
    
    # Search for voclip in custom keybindings
    for i in {0..9}; do
        local key_path="$CUSTOM_KEYS/custom$i"
        local name=$(gsettings get "$key_path" name 2>/dev/null || echo "")
        if [[ "$name" == "'voclip'" ]]; then
            gsettings set "$key_path" binding "''" 2>/dev/null || true
            gsettings set "$key_path" enabled false 2>/dev/null || true
            print_success "Removed hotkey from: $key_path"
            found=true
            removed=true
        fi
    done
    
    if [[ "$found" == "false" ]]; then
        print_warning "No voclip hotkey found in GNOME settings"
    fi
    
    # Summary
    echo ""
    if [[ "$removed" == "true" ]]; then
        echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${RESET}"
        echo -e "${GREEN}║$RESET              ${BOLD}Uninstallation Complete!${RESET}                    ${GREEN}║${RESET}"
        echo -e "${GREEN}╠═══════════════════════════════════════════════════════════╣${RESET}"
        echo -e "${GREEN}║$RESET                                                           ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET   ${GREEN}✓${RESET} Removed wrapper script                              ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET   ${GREEN}✓${RESET} Removed hotkey registration                          ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET   ${GREEN}✓${RESET} Removed autostart entry                              ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET                                                           ${GREEN}║${RESET}"
        echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${RESET}"
    else
        echo -e "${YELLOW}╔═══════════════════════════════════════════════════════════╗${RESET}"
        echo -e "${YELLOW}║$RESET              ${BOLD}Nothing to Remove${RESET}                            ${YELLOW}║${RESET}"
        echo -e "${YELLOW}╠═══════════════════════════════════════════════════════════╣${RESET}"
        echo -e "${YELLOW}║$RESET                                                           ${YELLOW}║${RESET}"
        echo -e "${YELLOW}║$RESET   No voclip installation found to remove.                ${YELLOW}║${RESET}"
        echo -e "${YELLOW}║$RESET                                                           ${YELLOW}║${RESET}"
        echo -e "${YELLOW}╚═══════════════════════════════════════════════════════════╝${RESET}"
    fi
}

# Windows uninstaller
uninstall_windows() {
    echo ""
    echo -e "${CYAN}╭─────────────────────────────────────────────────────────╮${RESET}"
    echo -e "${CYAN}│${RESET}          ${BOLD}voclip Windows Uninstaller${RESET}                    ${CYAN}│${RESET}"
    echo -e "${CYAN}╰─────────────────────────────────────────────────────────╯${RESET}"
    
    local removed=false
    
    # Remove wrapper script
    print_step "Removing wrapper script..."
    local wrapper="$LOCALAPPDATA\voclip\voclip-run.bat"
    if [[ -f "$wrapper" ]]; then
        rm -f "$wrapper"
        print_success "Removed: $wrapper"
        removed=true
    else
        print_warning "Wrapper not found: $wrapper"
    fi
    
    # Remove autostart shortcut
    print_step "Removing autostart shortcut..."
    local shortcut="$APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\voclip.lnk"
    if [[ -f "$shortcut" ]]; then
        rm -f "$shortcut"
        print_success "Removed: $shortcut"
        removed=true
    else
        print_warning "Shortcut not found: $shortcut"
    fi
    
    # Remove scheduled task (hotkey service)
    print_step "Removing hotkey service..."
    if [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]]; then
        # In Git Bash/MSYS
        local taskName="voclip-Hotkey"
    else
        local taskName="voclip-Hotkey"
    fi
    
    powershell -Command "Unregister-ScheduledTask -TaskName 'voclip-Hotkey' -Confirm:\$false -ErrorAction SilentlyContinue" 2>/dev/null || true
    print_success "Removed hotkey service (if existed)"
    removed=true
    
    # Remove voclip directory if empty
    print_step "Cleaning up..."
    local voclipDir="$LOCALAPPDATA\voclip"
    if [[ -d "$voclipDir" ]]; then
        if [[ -z "$(ls -A "$voclipDir" 2>/dev/null)" ]]; then
            rmdir "$voclipDir" 2>/dev/null || true
            print_success "Removed empty directory: $voclipDir"
        else
            print_info "Kept non-empty directory: $voclipDir"
        fi
    fi
    
    # Summary
    echo ""
    if [[ "$removed" == "true" ]]; then
        echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${RESET}"
        echo -e "${GREEN}║$RESET              ${BOLD}Uninstallation Complete!${RESET}                    ${GREEN}║${RESET}"
        echo -e "${GREEN}╠═══════════════════════════════════════════════════════════╣${RESET}"
        echo -e "${GREEN}║$RESET                                                           ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET   ${GREEN}✓${RESET} Removed wrapper script                              ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET   ${GREEN}✓${RESET} Removed hotkey registration                          ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET   ${GREEN}✓${RESET} Removed autostart entry                              ${GREEN}║${RESET}"
        echo -e "${GREEN}║$RESET                                                           ${GREEN}║${RESET}"
        echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${RESET}"
    else
        echo -e "${YELLOW}╔═══════════════════════════════════════════════════════════╗${RESET}"
        echo -e "${YELLOW}║$RESET              ${BOLD}Nothing to Remove${RESET}                            ${YELLOW}║${RESET}"
        echo -e "${YELLOW}╠═══════════════════════════════════════════════════════════╣${RESET}"
        echo -e "${YELLOW}║$RESET                                                           ${YELLOW}║${RESET}"
        echo -e "${YELLOW}║$RESET   No voclip installation found to remove.                ${YELLOW}║${RESET}"
        echo -e "${YELLOW}║$RESET                                                           ${YELLOW}║${RESET}"
        echo -e "${YELLOW}╚═══════════════════════════════════════════════════════════╝${RESET}"
    fi
}

# Main
main() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${CYAN}║$RESET                  ${BOLD}voclip Uninstaller${RESET}                       ${CYAN}║${RESET}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════╝${RESET}"
    
    local os=$(detect_os)
    
    echo ""
    echo "Detected OS: ${BOLD}${os}${RESET}"
    
    case "$os" in
        linux)
            uninstall_linux
            ;;
        windows|cygwin|msys)
            uninstall_windows
            ;;
        macos)
            print_error "macOS uninstaller not yet implemented."
            exit 1
            ;;
        *)
            print_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

main "$@"
