#!/usr/bin/env bash
#
# voclip Linux installer - GNOME & XFCE support
#

set -e

VOCLIP_PATH=""
API_KEY=""
HOTKEY=""
HOTKEY_DISPLAY=""
DESKTOP_ENV=""
ENV_PATH="$HOME/.config/voclip/.env"
WRAPPER_PATH="$HOME/.local/bin/voclip-run"
AUTOSTART_PATH="$HOME/.config/autostart/voclip.desktop"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

print_step()    { echo ""; echo -e "${BLUE}➜${RESET} ${BOLD}$1${RESET}"; }
print_success() { echo -e "${GREEN}✓${RESET} $1"; }
print_error()   { echo -e "${RED}✗${RESET} $1" >&2; }
print_warning() { echo -e "${YELLOW}⚠${RESET} $1"; }
print_info()    { echo -e "${CYAN}ℹ${RESET} $1"; }

# Detect desktop environment
detect_desktop() {
    local de="${XDG_CURRENT_DESKTOP:-${DESKTOP_SESSION:-}}"
    case "${de,,}" in
        *gnome*)    echo "gnome" ;;
        *xfce*)     echo "xfce" ;;
        *kde*)      echo "kde" ;;
        *cinnamon*) echo "cinnamon" ;;
        *)
            if command -v gnome-shell &>/dev/null; then
                echo "gnome"
            elif command -v xfconf-query &>/dev/null; then
                echo "xfce"
            else
                echo "unknown"
            fi
            ;;
    esac
}

# Convert user-friendly hotkey to GTK/XFCE format
# e.g. "Ctrl+Shift+F1" -> "<Primary><Shift>F1"
convert_hotkey() {
    local input="$1"
    local result=""

    IFS='+' read -ra parts <<< "$input"
    for part in "${parts[@]}"; do
        part="$(echo "$part" | xargs)"
        case "${part,,}" in
            ctrl|control) result="${result}<Primary>" ;;
            alt)          result="${result}<Alt>" ;;
            super|win)    result="${result}<Super>" ;;
            shift)        result="${result}<Shift>" ;;
            *)            result="${result}${part}" ;;
        esac
    done

    echo "$result"
}

# Find voclip binary
find_voclip() {
    print_step "Finding voclip binary..."
    for path in \
        "$(command -v voclip 2>/dev/null || true)" \
        "$HOME/.cargo/bin/voclip" \
        "$HOME/.local/bin/voclip" \
        "/usr/local/bin/voclip" \
        "/usr/bin/voclip"
    do
        if [[ -n "$path" && -x "$path" ]]; then
            VOCLIP_PATH="$path"
            print_success "Found: $VOCLIP_PATH"
            return 0
        fi
    done

    print_error "voclip not found!"
    echo ""
    echo "Please install voclip first:"
    echo "  cargo install voclip"
    echo ""
    echo "Or download from:"
    echo "  https://github.com/iceman1010/voclip/releases"
    exit 1
}

# Ask for API key
ask_api_key() {
    echo ""
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│                   ${BOLD}AssemblyAI API Key${RESET}                         │"
    echo "├─────────────────────────────────────────────────────────────┤"
    echo "│ You need an API key from AssemblyAI to use voclip.         │"
    echo "│ Get one free at: https://www.assemblyai.com/               │"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""

    # Check for existing key
    if [[ -f "$ENV_PATH" ]]; then
        local existing
        existing=$(grep -oP 'ASSEMBLYAI_API_KEY=\K.*' "$ENV_PATH" 2>/dev/null || true)
        if [[ -n "$existing" ]]; then
            local masked="${existing:0:4}...${existing: -4}"
            print_info "Existing API key found: $masked"
            echo -ne "${BOLD}Keep existing key? (Y/n)${RESET} "
            read -r -n 1 reply
            echo
            if [[ ! "$reply" =~ ^[Nn]$ ]]; then
                API_KEY="$existing"
                return
            fi
        fi
    fi

    echo -ne "${BOLD}Enter your AssemblyAI API key:${RESET} "
    read -r API_KEY

    if [[ -z "$API_KEY" ]]; then
        print_error "API key cannot be empty!"
        exit 1
    fi
}

# Ask for hotkey
ask_hotkey() {
    echo ""
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│                      ${BOLD}Keyboard Hotkey${RESET}                        │"
    echo "├─────────────────────────────────────────────────────────────┤"
    echo "│ Type the hotkey combination you want to use.               │"
    echo "│ Examples: Ctrl+F1, Super+V, Alt+Shift+Space                │"
    echo "│ Press Enter for default: ${BOLD}Ctrl+F1${RESET}                            │"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
    echo -ne "${BOLD}Enter hotkey (e.g., Ctrl+F1):${RESET} "
    read -r hotkey_input

    if [[ -z "$hotkey_input" ]]; then
        HOTKEY="<Primary>F1"
        HOTKEY_DISPLAY="Ctrl+F1"
    else
        HOTKEY=$(convert_hotkey "$hotkey_input")
        HOTKEY_DISPLAY="$hotkey_input"
    fi

    print_success "Using hotkey: $HOTKEY_DISPLAY"
}

# Confirm installation
confirm_install() {
    echo ""
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│                   ${BOLD}Installation Summary${RESET}                      │"
    echo "├─────────────────────────────────────────────────────────────┤"
    echo -e "│ Desktop:        ${BOLD}$DESKTOP_ENV${RESET}"
    echo -e "│ voclip binary:  ${BOLD}$VOCLIP_PATH${RESET}"
    echo -e "│ API key file:   ${BOLD}$ENV_PATH${RESET}"
    echo -e "│ Wrapper script: ${BOLD}$WRAPPER_PATH${RESET}"
    echo -e "│ Hotkey:         ${BOLD}$HOTKEY_DISPLAY${RESET}"
    echo -e "│ Autostart:      ${BOLD}Enabled${RESET}"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo ""
    read -p "Continue with installation? (Y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
}

# Save API key to config
save_api_key() {
    print_step "Saving API key..."

    mkdir -p "$(dirname "$ENV_PATH")"
    echo "ASSEMBLYAI_API_KEY=$API_KEY" > "$ENV_PATH"
    chmod 600 "$ENV_PATH"

    print_success "Saved to: $ENV_PATH (permissions: 600)"
}

# Create wrapper script
create_wrapper() {
    print_step "Creating wrapper script..."

    mkdir -p "$(dirname "$WRAPPER_PATH")"

    cat > "$WRAPPER_PATH" << WRAPPER
#!/bin/bash
# voclip wrapper script - generated by voclip installer
export XDG_RUNTIME_DIR="\${XDG_RUNTIME_DIR:-/run/user/\$(id -u)}"
export PULSE_SERVER="\${PULSE_SERVER:-unix:/run/user/\$(id -u)/pulse/native}"
setsid "$VOCLIP_PATH" "\$@" > ~/.voclip.log 2>&1 &
WRAPPER

    chmod +x "$WRAPPER_PATH"
    print_success "Created: $WRAPPER_PATH"
}

# Create desktop file for autostart
create_autostart() {
    print_step "Creating autostart desktop file..."

    mkdir -p "$(dirname "$AUTOSTART_PATH")"

    cat > "$AUTOSTART_PATH" << DESKTOP
[Desktop Entry]
Type=Application
Name=voclip
Comment=Voice to clipboard - check for updates on login
Exec=$VOCLIP_PATH --update
Icon=microphone
Terminal=false
Categories=Utility;
X-GNOME-Autostart-enabled=true
DESKTOP

    print_success "Created: $AUTOSTART_PATH"
    print_info "On login, voclip will check for updates automatically."
}

# Setup GNOME hotkey
setup_gnome_hotkey() {
    print_step "Setting up GNOME hotkey..."

    local SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
    local BASE_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"
    local KEY_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"

    local existing
    existing=$(gsettings get "$SCHEMA" custom-keybindings 2>/dev/null || echo "@as []")

    # Find if voclip already has a slot, or find the next free slot
    local voclip_slot=""
    local next_free=0

    for i in $(seq 0 19); do
        local path="$BASE_PATH/custom$i/"
        local name
        name=$(gsettings get "$KEY_SCHEMA:$path" name 2>/dev/null || echo "")
        if [[ "$name" == "'voclip'" ]]; then
            voclip_slot="$path"
            break
        fi
        if echo "$existing" | grep -q "custom$i"; then
            next_free=$((i + 1))
        fi
    done

    local slot="${voclip_slot:-$BASE_PATH/custom$next_free/}"

    gsettings set "$KEY_SCHEMA:$slot" name "voclip"
    gsettings set "$KEY_SCHEMA:$slot" binding "$HOTKEY"
    gsettings set "$KEY_SCHEMA:$slot" command "$WRAPPER_PATH"

    if [[ -z "$voclip_slot" ]]; then
        if [[ "$existing" == "@as []" || "$existing" == "[]" ]]; then
            gsettings set "$SCHEMA" custom-keybindings "['$slot']"
        else
            local new_list
            new_list=$(echo "$existing" | sed "s|]|, '$slot']|")
            gsettings set "$SCHEMA" custom-keybindings "$new_list"
        fi
        print_success "Hotkey registered: $HOTKEY_DISPLAY -> voclip"
    else
        print_success "Updated existing hotkey: $HOTKEY_DISPLAY -> voclip"
    fi
}

# Setup XFCE hotkey
setup_xfce_hotkey() {
    print_step "Setting up XFCE hotkey..."

    local channel="xfce4-keyboard-shortcuts"
    local prop="/commands/custom/$HOTKEY"

    # Remove any existing voclip binding
    local existing_bindings
    existing_bindings=$(xfconf-query -c "$channel" -l -v 2>/dev/null | grep -F "$WRAPPER_PATH" || true)
    if [[ -n "$existing_bindings" ]]; then
        while IFS= read -r line; do
            local old_prop
            old_prop=$(echo "$line" | awk '{print $1}')
            xfconf-query -c "$channel" -p "$old_prop" -r 2>/dev/null || true
        done <<< "$existing_bindings"
    fi

    # Also check for old wrapper path
    local old_bindings
    old_bindings=$(xfconf-query -c "$channel" -l -v 2>/dev/null | grep "voclip" || true)
    if [[ -n "$old_bindings" ]]; then
        while IFS= read -r line; do
            local old_prop
            old_prop=$(echo "$line" | awk '{print $1}')
            xfconf-query -c "$channel" -p "$old_prop" -r 2>/dev/null || true
        done <<< "$old_bindings"
    fi

    # Set the new binding
    xfconf-query -c "$channel" -p "$prop" -n -t string -s "$WRAPPER_PATH"

    print_success "Hotkey registered: $HOTKEY_DISPLAY -> voclip"
}

# Setup hotkey based on desktop environment
setup_hotkey() {
    case "$DESKTOP_ENV" in
        gnome)    setup_gnome_hotkey ;;
        xfce)     setup_xfce_hotkey ;;
        *)
            print_warning "Unsupported desktop for hotkey: $DESKTOP_ENV"
            print_info "Please set up a keyboard shortcut manually:"
            print_info "  Command: $WRAPPER_PATH"
            print_info "  Hotkey:  $HOTKEY_DISPLAY"
            ;;
    esac
}

# Finalize
finalize() {
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${GREEN}║${RESET}              ${BOLD}Installation Complete!${RESET}                       ${GREEN}║${RESET}"
    echo -e "${GREEN}╠═══════════════════════════════════════════════════════════╣${RESET}"
    echo -e "${GREEN}║${RESET}                                                           ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}  ${GREEN}✓${RESET} API key saved (chmod 600)                              ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}  ${GREEN}✓${RESET} Wrapper script created                                 ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}  ${GREEN}✓${RESET} Hotkey registered ($HOTKEY_DISPLAY)                              ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}  ${GREEN}✓${RESET} Autostart enabled                                      ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}                                                           ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}  Press ${BOLD}$HOTKEY_DISPLAY${RESET} to start voice recording.               ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}  Uninstall: ./install/uninstall.sh                        ${GREEN}║${RESET}"
    echo -e "${GREEN}║${RESET}                                                           ${GREEN}║${RESET}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${RESET}"
    echo ""
}

# Main
main() {
    echo ""
    echo -e "${CYAN}╭─────────────────────────────────────────────────────────╮${RESET}"
    echo -e "${CYAN}│${RESET}           ${BOLD}voclip Linux Installer${RESET}                         ${CYAN}│${RESET}"
    echo -e "${CYAN}╰─────────────────────────────────────────────────────────╯${RESET}"

    DESKTOP_ENV=$(detect_desktop)
    echo ""
    echo -e "Detected desktop: ${BOLD}$DESKTOP_ENV${RESET}"

    if [[ "$DESKTOP_ENV" != "gnome" && "$DESKTOP_ENV" != "xfce" ]]; then
        print_warning "This installer supports GNOME and XFCE. Detected: $DESKTOP_ENV"
        print_info "Hotkey will need to be set up manually."
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        [[ $REPLY =~ ^[Yy]$ ]] || exit 0
    fi

    find_voclip
    ask_api_key
    ask_hotkey
    confirm_install

    save_api_key
    create_wrapper
    create_autostart
    setup_hotkey
    finalize
}

main "$@"
