#!/usr/bin/env sh
# ==============================================================================
#            MQN Masteraid - Linux Auto-Installer & Uninstaller
# ==============================================================================
# Installs Masteraid CLAP and/or VST3 plugins with interactive selection or CLI flags.
#
# Usage:
#   ./install.sh                Interactive wizard (prompts for CLAP, VST3, scope)
#   ./install.sh --all          Install both CLAP and VST3 for current user
#   ./install.sh --clap         Install only CLAP for current user
#   ./install.sh --vst3         Install only VST3 for current user
#   ./install.sh --system       Install to system-wide paths (/usr/lib/clap, /usr/lib/vst3)
#   ./install.sh --uninstall    Uninstall Masteraid plugins
# ==============================================================================
set -eu

# Color codes (if supported)
if [ -t 1 ]; then
    BOLD="\033[1m"
    GREEN="\033[1;32m"
    MAGENTA="\033[1;35m"
    CYAN="\033[1;36m"
    YELLOW="\033[1;33m"
    RED="\033[1;31m"
    RESET="\033[0m"
else
    BOLD=""
    GREEN=""
    MAGENTA=""
    CYAN=""
    YELLOW=""
    RED=""
    RESET=""
fi

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# 1. Locate plugin artifacts (checks current folder, then target/bundled)
CLAP_SRC=""
VST3_SRC=""

if [ -f "$HERE/Masteraid.clap" ]; then
    CLAP_SRC="$HERE/Masteraid.clap"
elif [ -f "$HERE/../target/bundled/Masteraid.clap" ]; then
    CLAP_SRC="$HERE/../target/bundled/Masteraid.clap"
elif [ -f "$HERE/target/bundled/Masteraid.clap" ]; then
    CLAP_SRC="$HERE/target/bundled/Masteraid.clap"
fi

if [ -d "$HERE/Masteraid.vst3" ]; then
    VST3_SRC="$HERE/Masteraid.vst3"
elif [ -d "$HERE/../target/bundled/Masteraid.vst3" ]; then
    VST3_SRC="$HERE/../target/bundled/Masteraid.vst3"
elif [ -d "$HERE/target/bundled/Masteraid.vst3" ]; then
    VST3_SRC="$HERE/target/bundled/Masteraid.vst3"
fi

# Manual PDF location
MANUAL_SRC=""
for p in "$HERE/Masteraid_User_Manual.pdf" "$HERE/assets/Masteraid_User_Manual.pdf" "$HERE/../assets/Masteraid_User_Manual.pdf"; do
    if [ -f "$p" ]; then
        MANUAL_SRC="$p"
        break
    fi
done

# Parse CLI options
INSTALL_CLAP=0
INSTALL_VST3=0
SCOPE="user"
MODE="install"
INTERACTIVE=1

for arg in "$@"; do
    case "$arg" in
        --all)
            INSTALL_CLAP=1
            INSTALL_VST3=1
            INTERACTIVE=0
            ;;
        --clap|--clap-only)
            INSTALL_CLAP=1
            INSTALL_VST3=0
            INTERACTIVE=0
            ;;
        --vst3|--vst3-only)
            INSTALL_CLAP=0
            INSTALL_VST3=1
            INTERACTIVE=0
            ;;
        --user)
            SCOPE="user"
            ;;
        --system)
            SCOPE="system"
            ;;
        --uninstall)
            MODE="uninstall"
            INTERACTIVE=0
            ;;
        -h|--help)
            echo "Usage: ./install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --all              Install both CLAP and VST3"
            echo "  --clap             Install CLAP plugin only"
            echo "  --vst3             Install VST3 plugin only"
            echo "  --user             Install for current user only (~/.clap, ~/.vst3) [Default]"
            echo "  --system           Install system-wide (/usr/lib/clap, /usr/lib/vst3)"
            echo "  --uninstall        Remove installed Masteraid plugins"
            echo "  -h, --help         Show this help"
            exit 0
            ;;
    esac
done

# Interactive Wizard
if [ "$INTERACTIVE" -eq 1 ] && [ -t 0 ]; then
    printf "${MAGENTA}${BOLD}"
    cat <<'BANNER'
  ==============================================================
             MQN DSP &bull; MASTERAID LINUX INSTALLER
  ==============================================================
BANNER
    printf "${RESET}\n"

    printf "${BOLD}Which plugin format(s) would you like to install?${RESET}\n"
    printf "  ${CYAN}[1]${RESET} Both CLAP & VST3 ${GREEN}(Recommended)${RESET}\n"
    printf "  ${CYAN}[2]${RESET} CLAP only\n"
    printf "  ${CYAN}[3]${RESET} VST3 only\n"
    printf "  ${CYAN}[4]${RESET} Uninstall Masteraid\n"
    printf "  ${CYAN}[q]${RESET} Quit\n"
    printf "${BOLD}Selection [1-4, q] (default: 1): ${RESET}"
    read -r choice || choice="1"
    case "$choice" in
        2)
            INSTALL_CLAP=1
            INSTALL_VST3=0
            ;;
        3)
            INSTALL_CLAP=0
            INSTALL_VST3=1
            ;;
        4)
            MODE="uninstall"
            ;;
        q|Q)
            echo "Cancelled."
            exit 0
            ;;
        *)
            INSTALL_CLAP=1
            INSTALL_VST3=1
            ;;
    esac

    if [ "$MODE" = "install" ]; then
        printf "\n${BOLD}Where would you like to install?${RESET}\n"
        printf "  ${CYAN}[1]${RESET} Current User only (~/.clap, ~/.vst3) ${GREEN}(No root needed)${RESET}\n"
        printf "  ${CYAN}[2]${RESET} System-wide (/usr/lib/clap, /usr/lib/vst3) ${YELLOW}(Requires root/sudo)${RESET}\n"
        printf "${BOLD}Selection [1-2] (default: 1): ${RESET}"
        read -r scope_choice || scope_choice="1"
        case "$scope_choice" in
            2)
                SCOPE="system"
                ;;
            *)
                SCOPE="user"
                ;;
        esac
    fi
elif [ "$INTERACTIVE" -eq 1 ]; then
    # Non-interactive default (e.g. piped or automated)
    INSTALL_CLAP=1
    INSTALL_VST3=1
    SCOPE="user"
fi

# Define destination directories based on scope
if [ "$SCOPE" = "system" ]; then
    CLAP_DIR="/usr/lib/clap"
    VST3_DIR="/usr/lib/vst3"
    DOC_DIR="/usr/share/doc/masteraid"
    SUDO=""
    if [ "$(id -u)" -ne 0 ]; then
        if command -v sudo >/dev/null 2>&1; then
            SUDO="sudo"
            echo "System install selected: using sudo for elevated permissions."
        else
            echo "${RED}Error: System-wide install requires root privileges or sudo.${RESET}" >&2
            exit 1
        fi
    fi
else
    CLAP_DIR="${HOME}/.clap"
    VST3_DIR="${HOME}/.vst3"
    DOC_DIR="${HOME}/.local/share/doc/masteraid"
    SUDO=""
fi

# ==============================================================================
# UNINSTALL MODE
# ==============================================================================
if [ "$MODE" = "uninstall" ]; then
    printf "\n${YELLOW}${BOLD}Uninstalling Masteraid...${RESET}\n"
    for base in "${HOME}/.clap" "/usr/lib/clap" "/usr/lib64/clap"; do
        if [ -f "$base/Masteraid.clap" ] || [ -f "$base/masteraid.clap" ]; then
            CMD="$SUDO rm -f $base/Masteraid.clap $base/masteraid.clap"
            echo "Removing: $base/Masteraid.clap"
            $CMD
        fi
    done
    for base in "${HOME}/.vst3" "/usr/lib/vst3" "/usr/lib64/vst3"; do
        if [ -d "$base/Masteraid.vst3" ]; then
            CMD="$SUDO rm -rf $base/Masteraid.vst3"
            echo "Removing: $base/Masteraid.vst3"
            $CMD
        fi
    done
    printf "${GREEN}${BOLD}&check; Masteraid plugins uninstalled successfully.${RESET}\n\n"
    exit 0
fi

# ==============================================================================
# INSTALL MODE
# ==============================================================================
printf "\n${MAGENTA}${BOLD}Installing Masteraid ($SCOPE)...${RESET}\n"

# Verify source files
if [ "$INSTALL_CLAP" -eq 1 ] && [ -z "$CLAP_SRC" ]; then
    echo "${RED}Error: Could not find Masteraid.clap binary.${RESET}" >&2
    echo "Make sure to run 'cargo xtask bundle masteraid --release' first." >&2
    exit 1
fi

if [ "$INSTALL_VST3" -eq 1 ] && [ -z "$VST3_SRC" ]; then
    echo "${RED}Error: Could not find Masteraid.vst3 bundle.${RESET}" >&2
    echo "Make sure to run 'cargo xtask bundle masteraid --release' first." >&2
    exit 1
fi

# 1. Install CLAP
if [ "$INSTALL_CLAP" -eq 1 ]; then
    $SUDO mkdir -p "$CLAP_DIR"
    $SUDO rm -f "$CLAP_DIR/Masteraid.clap" "$CLAP_DIR/masteraid.clap"
    $SUDO cp "$CLAP_SRC" "$CLAP_DIR/Masteraid.clap"
    $SUDO chmod 755 "$CLAP_DIR/Masteraid.clap"
    printf "  ${GREEN}&check; Installed CLAP:${RESET}  $CLAP_DIR/Masteraid.clap\n"
fi

# 2. Install VST3
if [ "$INSTALL_VST3" -eq 1 ]; then
    $SUDO mkdir -p "$VST3_DIR"
    $SUDO rm -rf "$VST3_DIR/Masteraid.vst3"
    $SUDO cp -R "$VST3_SRC" "$VST3_DIR/"
    $SUDO chmod -R 755 "$VST3_DIR/Masteraid.vst3"
    printf "  ${GREEN}&check; Installed VST3:${RESET}  $VST3_DIR/Masteraid.vst3\n"
fi

# 3. Install Documentation / Manual
if [ -n "$MANUAL_SRC" ]; then
    $SUDO mkdir -p "$DOC_DIR"
    $SUDO cp "$MANUAL_SRC" "$DOC_DIR/Masteraid_User_Manual.pdf"
    printf "  ${GREEN}&check; User Manual:${RESET}     $DOC_DIR/Masteraid_User_Manual.pdf\n"
fi

printf "\n${GREEN}${BOLD}Installation Complete!${RESET}\n"
echo "Rescan plugins in your DAW (Reaper, Bitwig, Ardour, Renoise, etc.)."
echo ""
