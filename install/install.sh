#!/usr/bin/env bash
#
# voclip installer - Entry point
# Supports: Linux (GNOME), Windows
#

set -e

CYAN='\033[0;36m'
BOLD='\033[1m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
RESET='\033[0m'

print_header() {
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                    ${BOLD}voclip Installer${CYAN}                      ║"
    echo "║           Voice to Clipboard - Hotkey Setup              ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${RESET}"
}

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

detect_desktop() {
    local de="${XDG_CURRENT_DESKTOP:-${DESKTOP_SESSION:-}}"
    case "${de,,}" in
        *gnome*)    echo "gnome" ;;
        *kde*)      echo "kde" ;;
        *xfce*)     echo "xfce" ;;
        *cinnamon*) echo "cinnamon" ;;
        *)
            if command -v gnome-shell &>/dev/null; then
                echo "gnome"
            else
                echo "unknown"
            fi
            ;;
    esac
}

main() {
    print_header

    local os
    os=$(detect_os)
    echo "Detected OS: ${BOLD}${os}${RESET}"

    case "$os" in
        linux)
            local desktop
            desktop=$(detect_desktop)
            echo "Detected Desktop: ${BOLD}${desktop}${RESET}"

            if [[ "$desktop" != "gnome" && "$desktop" != "xfce" ]]; then
                echo ""
                echo -e "${YELLOW}⚠${RESET} This installer supports GNOME and XFCE (detected: $desktop)."
                read -p "Continue anyway? (y/N) " -n 1 -r
                echo
                [[ $REPLY =~ ^[Yy]$ ]] || exit 0
            fi

            bash "$(dirname "$0")/install-linux.sh" "$@"
            ;;
        windows)
            echo ""
            echo "Running Windows installer..."
            powershell -ExecutionPolicy Bypass -File "$(dirname "$0")/install-windows.ps1" "$@"
            ;;
        macos)
            echo ""
            echo -e "${RED}✗${RESET} macOS support is not yet implemented."
            echo ""
            echo "For now, you can manually set up a hotkey using:"
            echo "  - skhd (brew install koekeishiya/formulae/skhd)"
            echo "  - BetterTouchTool"
            echo ""
            echo "See: https://github.com/iceman1010/voclip#keyboard-shortcut"
            exit 1
            ;;
        *)
            echo -e "${RED}✗${RESET} Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

main "$@"
