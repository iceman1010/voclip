#!/usr/bin/env bash
#
# voclip installer - Entry point
# Supports: Linux (GNOME), Windows
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Print functions
print_header() {
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                    ${BOLD}voclip Installer${CYAN}                      ║"
    echo "║           Voice to Clipboard - Hotkey Setup            ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${RESET}"
}

print_step() {
    echo -e "${BLUE}➜${RESET} ${BOLD}$1${RESET}"
}

print_success() {
    echo -e "${GREEN}✓${RESET} $1"
}

print_error() {
    echo -e "${RED}✗${RESET} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${RESET} $1"
}

print_info() {
    echo -e "${CYAN}ℹ${RESET} $1"
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

# Detect Linux desktop environment
detect_desktop() {
    if [[ -n "$XDG_CURRENT_DESKTOP" ]]; then
        case "${XDG_CURRENT_DESKTOP,,}" in
            *gnome*) echo "gnome" ;;
            *kde*) echo "kde" ;;
            *xfce*) echo "xfce" ;;
            *cinnamon*) echo "cinnamon" ;;
            *) echo "unknown" ;;
        esac
    elif [[ -n "$DESKTOP_SESSION" ]]; then
        case "${DESKTOP_SESSION,,}" in
            *gnome*) echo "gnome" ;;
            *kde*) echo "kde" ;;
            *xfce*) echo "xfce" ;;
            *cinnamon*) echo "cinnamon" ;;
            *) echo "unknown" ;;
        esac
    elif command -v gnome-shell &> /dev/null; then
        echo "gnome"
    else
        echo "unknown"
    fi
}

# Find voclip binary
find_voclip() {
    local os="$1"
    
    print_step "Searching for voclip binary..."
    
    if [[ "$os" == "linux" ]]; then
        # Check common locations
        for path in \
            "$(which voclip 2>/dev/null)" \
            "$HOME/.cargo/bin/voclip" \
            "$HOME/.local/bin/voclip" \
            "/usr/local/bin/voclip" \
            "/usr/bin/voclip"
        do
            if [[ -x "$path" ]]; then
                print_success "Found voclip at: $path"
                echo "$path"
                return 0
            fi
        done
    elif [[ "$os" == "windows" ]]; then
        # Check using where command first
        local where_path
        if where_path=$(where voclip 2>/dev/null); then
            local first_path=$(echo "$where_path" | head -1)
            if [[ -x "$first_path" ]]; then
                print_success "Found voclip at: $first_path"
                echo "$first_path"
                return 0
            fi
        fi
        
        # Check common install locations
        for path in \
            "C:\\Program Files\\voclip\\voclip.exe" \
            "C:\\Program Files\\voclip\\bin\\voclip.exe" \
            "$LOCALAPPDATA\\voclip\\voclip.exe"
        do
            if [[ -x "$path" ]]; then
                print_success "Found voclip at: $path"
                echo "$path"
                return 0
            fi
        done
    fi
    
    print_error "voclip not found!"
    echo ""
    echo "Please install voclip first:"
    echo "  Linux/macOS:  cargo install voclip"
    echo "  Windows:       cargo install voclip (in MSYS2/WSL) or download from GitHub"
    echo ""
    echo "Alternatively, download a release from:"
    echo "  https://github.com/iceman1010/voclip/releases"
    return 1
}

# Main
main() {
    print_header
    
    local os=$(detect_os)
    
    echo ""
    echo "Detected OS: ${BOLD}${os}${RESET}"
    
    case "$os" in
        linux)
            local desktop=$(detect_desktop)
            echo "Detected Desktop: ${BOLD}${desktop}${RESET}"
            echo ""
            
            if [[ "$desktop" != "gnome" ]]; then
                print_warning "This installer currently supports GNOME desktop only."
                print_warning "Detected: $desktop"
                echo ""
                read -p "Continue anyway? (y/N) " -n 1 -r
                echo
                if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                    echo "Installation cancelled."
                    exit 0
                fi
            fi
            
            if [[ -x "$0/../install-linux.sh" ]]; then
                "$0/../install-linux.sh" "$@"
            else
                bash "$(dirname "$0")/install-linux.sh" "$@"
            fi
            ;;
        windows)
            echo ""
            echo "Running Windows installer..."
            echo ""
            powershell -ExecutionPolicy Bypass -File "$(dirname "$0")/install-windows.ps1" "$@"
            ;;
        macos)
            print_error "macOS support is not yet implemented."
            echo ""
            echo "For now, you can manually set up a hotkey using:"
            echo "  - skhd (brew install koekeishiya/formulae/skhd)"
            echo "  - BetterTouchTool"
            echo ""
            echo "See: https://github.com/iceman1010/voclip#keyboard-shortcut"
            exit 1
            ;;
        *)
            print_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

main "$@"
