#!/usr/bin/env bash
#
# voclip uninstaller - Linux (GNOME & XFCE)
#

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

print_step()    { echo ""; echo -e "${BLUE}➜${RESET} ${BOLD}$1${RESET}"; }
print_success() { echo -e "${GREEN}✓${RESET} $1"; }
print_warning() { echo -e "${YELLOW}⚠${RESET} $1"; }

removed=false

remove_file() {
    local path="$1"
    local label="$2"
    print_step "Removing $label..."
    if [[ -f "$path" ]]; then
        rm -f "$path"
        print_success "Removed: $path"
        removed=true
    else
        print_warning "Not found: $path"
    fi
}

remove_gnome_hotkey() {
    if ! command -v gsettings &>/dev/null; then
        return
    fi

    print_step "Removing GNOME hotkey..."

    local SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
    local BASE_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"
    local KEY_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"
    local found=false

    for i in $(seq 0 19); do
        local path="$BASE_PATH/custom$i/"
        local name
        name=$(gsettings get "$KEY_SCHEMA:$path" name 2>/dev/null || echo "")
        if [[ "$name" == "'voclip'" ]]; then
            gsettings reset "$KEY_SCHEMA:$path" name 2>/dev/null || true
            gsettings reset "$KEY_SCHEMA:$path" binding 2>/dev/null || true
            gsettings reset "$KEY_SCHEMA:$path" command 2>/dev/null || true

            local existing
            existing=$(gsettings get "$SCHEMA" custom-keybindings 2>/dev/null || echo "[]")
            local new_list
            new_list=$(echo "$existing" | sed "s|'$path'||g" | sed "s|, ,|,|g" | sed "s|\[, |\[|" | sed "s|, \]|\]|")
            gsettings set "$SCHEMA" custom-keybindings "$new_list" 2>/dev/null || true

            print_success "Removed GNOME hotkey: $path"
            found=true
            removed=true
        fi
    done

    if [[ "$found" == "false" ]]; then
        print_warning "No voclip hotkey found in GNOME settings"
    fi
}

remove_xfce_hotkey() {
    if ! command -v xfconf-query &>/dev/null; then
        return
    fi

    print_step "Removing XFCE hotkey..."

    local channel="xfce4-keyboard-shortcuts"
    local found=false

    local bindings
    bindings=$(xfconf-query -c "$channel" -l -v 2>/dev/null | grep "voclip" || true)
    if [[ -n "$bindings" ]]; then
        while IFS= read -r line; do
            local prop
            prop=$(echo "$line" | awk '{print $1}')
            xfconf-query -c "$channel" -p "$prop" -r 2>/dev/null || true
            print_success "Removed XFCE hotkey: $prop"
            found=true
            removed=true
        done <<< "$bindings"
    fi

    if [[ "$found" == "false" ]]; then
        print_warning "No voclip hotkey found in XFCE settings"
    fi
}

main() {
    echo ""
    echo -e "${CYAN}╭─────────────────────────────────────────────────────────╮${RESET}"
    echo -e "${CYAN}│${RESET}           ${BOLD}voclip Linux Uninstaller${RESET}                    ${CYAN}│${RESET}"
    echo -e "${CYAN}╰─────────────────────────────────────────────────────────╯${RESET}"

    remove_file "$HOME/.local/bin/voclip-run" "wrapper script"
    remove_file "$HOME/.config/autostart/voclip.desktop" "autostart entry"

    # Also check for old wrapper location
    if [[ -f "$HOME/bin/voclip-wrapper" ]]; then
        remove_file "$HOME/bin/voclip-wrapper" "old wrapper script"
    fi

    # Remove hotkeys from all supported DEs
    remove_gnome_hotkey
    remove_xfce_hotkey

    # Ask about API key file
    local env_path="$HOME/.config/voclip/.env"
    if [[ -f "$env_path" ]]; then
        echo ""
        echo -ne "${BOLD}Also remove API key file ($env_path)? (y/N)${RESET} "
        read -r -n 1 reply
        echo
        if [[ "$reply" =~ ^[Yy]$ ]]; then
            rm -f "$env_path"
            print_success "Removed: $env_path"
        else
            print_warning "Kept: $env_path"
        fi
    fi

    echo ""
    if [[ "$removed" == "true" ]]; then
        echo -e "${GREEN}Uninstallation complete.${RESET}"
    else
        echo -e "${YELLOW}Nothing to remove — voclip was not installed via the installer.${RESET}"
    fi
    echo ""
}

main "$@"
