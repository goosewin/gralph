#!/bin/bash
# Uninstallation script for gralph
# Removes installed binary, libraries, completions, and config
# See PRD.md UNS-1 for specification
#
# Usage:
#   ./uninstall.sh           Interactive mode with prompts
#   ./uninstall.sh --all     Remove everything including logs/state
#   ./uninstall.sh --force   Skip confirmation prompts
#   ./uninstall.sh --help    Show help

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# =============================================================================
# Configuration
# =============================================================================

# Standard install locations (must match install.sh)
CONFIG_DIR="$HOME/.config/gralph"
LIB_DIR="$CONFIG_DIR/lib"
COMPLETIONS_DIR="$CONFIG_DIR/completions"
GRALPH_STATE_DIR="$HOME/.gralph"

# Possible binary locations
BINARY_LOCATIONS=(
    "$HOME/.local/bin/gralph"
    "/usr/local/bin/gralph"
)

# Shell completion locations
BASH_COMPLETION_LOCATIONS=(
    "/etc/bash_completion.d/gralph"
    "$HOME/.local/share/bash-completion/completions/gralph"
)

ZSH_COMPLETION_LOCATIONS=(
    "/usr/local/share/zsh/site-functions/_gralph"
    "$HOME/.zsh/completions/_gralph"
)

# =============================================================================
# Command line parsing
# =============================================================================

REMOVE_ALL=false
FORCE=false
SHOW_HELP=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --all|-a)
            REMOVE_ALL=true
            shift
            ;;
        --force|-f)
            FORCE=true
            shift
            ;;
        --help|-h)
            SHOW_HELP=true
            shift
            ;;
        *)
            echo_error "Unknown option: $1"
            SHOW_HELP=true
            shift
            ;;
    esac
done

if [ "$SHOW_HELP" = true ]; then
    cat << 'EOF'
gralph uninstaller
==================

Usage:
    ./uninstall.sh [options]

Options:
    --all, -a     Remove everything including logs and state data
    --force, -f   Skip confirmation prompts
    --help, -h    Show this help message

What gets removed by default:
    - gralph binary (from ~/.local/bin or /usr/local/bin)
    - Library files (~/.config/gralph/lib/)
    - Shell completions (~/.config/gralph/completions/ and system locations)
    - Configuration file (~/.config/gralph/config.yaml)

What gets removed with --all:
    - All of the above
    - Session logs (~/.gralph/*.log)
    - State files (~/.gralph/sessions.json)
    - The entire ~/.gralph directory

Note: User data (logs, state) is NOT removed by default.
      Use --all to include user data removal.

EOF
    exit 0
fi

# =============================================================================
# Helper functions
# =============================================================================

# Prompt for confirmation (skipped if FORCE=true)
confirm() {
    local prompt="$1"
    local default="${2:-n}"

    if [ "$FORCE" = true ]; then
        return 0
    fi

    local yn_prompt
    if [ "$default" = "y" ]; then
        yn_prompt="[Y/n]"
    else
        yn_prompt="[y/N]"
    fi

    read -p "$prompt $yn_prompt " -n 1 -r
    echo ""

    if [[ -z "$REPLY" ]]; then
        REPLY="$default"
    fi

    [[ "$REPLY" =~ ^[Yy]$ ]]
}

# Remove a file if it exists
remove_file() {
    local file="$1"
    local use_sudo="${2:-false}"

    if [ -f "$file" ]; then
        if [ "$use_sudo" = true ]; then
            if sudo rm -f "$file" 2>/dev/null; then
                echo_info "  Removed: $file (with sudo)"
                return 0
            else
                echo_warn "  Failed to remove: $file"
                return 1
            fi
        else
            if rm -f "$file" 2>/dev/null; then
                echo_info "  Removed: $file"
                return 0
            else
                echo_warn "  Failed to remove: $file"
                return 1
            fi
        fi
    fi
    return 0
}

# Remove a directory if it exists
remove_dir() {
    local dir="$1"

    if [ -d "$dir" ]; then
        if rm -rf "$dir" 2>/dev/null; then
            echo_info "  Removed: $dir/"
            return 0
        else
            echo_warn "  Failed to remove: $dir/"
            return 1
        fi
    fi
    return 0
}

# Check if file/dir requires sudo to remove
needs_sudo() {
    local path="$1"

    if [ ! -e "$path" ]; then
        return 1
    fi

    # Check if we can write to the parent directory
    local parent
    parent=$(dirname "$path")
    [ ! -w "$parent" ]
}

# =============================================================================
# Uninstallation steps
# =============================================================================

# Find and remove the gralph binary
uninstall_binary() {
    echo_info "Removing gralph binary..."

    local found=false
    for location in "${BINARY_LOCATIONS[@]}"; do
        if [ -f "$location" ]; then
            found=true
            local use_sudo=false
            if needs_sudo "$location"; then
                use_sudo=true
            fi
            remove_file "$location" "$use_sudo"
        fi
    done

    # Also check if it's in a custom location via which
    local current_binary
    current_binary=$(command -v gralph 2>/dev/null || true)
    if [ -n "$current_binary" ] && [ -f "$current_binary" ]; then
        # Check if we already removed it
        local already_removed=false
        for location in "${BINARY_LOCATIONS[@]}"; do
            if [ "$current_binary" = "$location" ]; then
                already_removed=true
                break
            fi
        done

        if [ "$already_removed" = false ]; then
            found=true
            echo_warn "  Found gralph at non-standard location: $current_binary"
            if confirm "  Remove $current_binary?"; then
                local use_sudo=false
                if needs_sudo "$current_binary"; then
                    use_sudo=true
                fi
                remove_file "$current_binary" "$use_sudo"
            fi
        fi
    fi

    if [ "$found" = false ]; then
        echo_info "  No gralph binary found"
    fi
}

# Remove library files
uninstall_libraries() {
    echo_info "Removing library files..."

    if [ -d "$LIB_DIR" ]; then
        remove_dir "$LIB_DIR"
    else
        echo_info "  No library directory found"
    fi
}

# Remove shell completions
uninstall_completions() {
    echo_info "Removing shell completions..."

    local found=false

    # Remove from config directory
    if [ -d "$COMPLETIONS_DIR" ]; then
        found=true
        remove_dir "$COMPLETIONS_DIR"
    fi

    # Remove bash completions from system locations
    for location in "${BASH_COMPLETION_LOCATIONS[@]}"; do
        if [ -f "$location" ]; then
            found=true
            local use_sudo=false
            if needs_sudo "$location"; then
                use_sudo=true
            fi
            remove_file "$location" "$use_sudo"
        fi
    done

    # Remove zsh completions from system locations
    for location in "${ZSH_COMPLETION_LOCATIONS[@]}"; do
        if [ -f "$location" ]; then
            found=true
            local use_sudo=false
            if needs_sudo "$location"; then
                use_sudo=true
            fi
            remove_file "$location" "$use_sudo"
        fi
    done

    if [ "$found" = false ]; then
        echo_info "  No shell completions found"
    fi
}

# Remove configuration
uninstall_config() {
    echo_info "Removing configuration..."

    local config_file="$CONFIG_DIR/config.yaml"

    if [ -f "$config_file" ]; then
        remove_file "$config_file"
    else
        echo_info "  No configuration file found"
    fi

    # Remove config directory if empty (after removing lib/completions)
    if [ -d "$CONFIG_DIR" ]; then
        # Check if directory is empty
        if [ -z "$(ls -A "$CONFIG_DIR" 2>/dev/null)" ]; then
            remove_dir "$CONFIG_DIR"
        else
            echo_warn "  Config directory not empty, keeping: $CONFIG_DIR"
            echo "        Contents:"
            ls -la "$CONFIG_DIR" 2>/dev/null | head -10 | while read -r line; do
                echo "          $line"
            done
        fi
    fi
}

# Remove logs and state (only with --all)
uninstall_user_data() {
    echo_info "Removing user data (logs and state)..."

    if [ -d "$GRALPH_STATE_DIR" ]; then
        # Count files for user feedback
        local log_count
        log_count=$(find "$GRALPH_STATE_DIR" -name "*.log" 2>/dev/null | wc -l | tr -d ' ')

        if [ "$log_count" -gt 0 ]; then
            echo_warn "  Found $log_count log file(s)"
        fi

        if [ -f "$GRALPH_STATE_DIR/sessions.json" ]; then
            echo_warn "  Found session state file"
        fi

        remove_dir "$GRALPH_STATE_DIR"
    else
        echo_info "  No user data directory found"
    fi
}

# =============================================================================
# Main uninstallation
# =============================================================================

echo "gralph uninstaller"
echo "=================="
echo ""

# Check what's installed
echo_info "Checking installed components..."

installed_components=()

# Check binary
for location in "${BINARY_LOCATIONS[@]}"; do
    if [ -f "$location" ]; then
        installed_components+=("Binary: $location")
        break
    fi
done

# Check via which as backup
if [ ${#installed_components[@]} -eq 0 ]; then
    current_binary=$(command -v gralph 2>/dev/null || true)
    if [ -n "$current_binary" ] && [ -f "$current_binary" ]; then
        installed_components+=("Binary: $current_binary")
    fi
fi

# Check libraries
if [ -d "$LIB_DIR" ]; then
    lib_count=$(find "$LIB_DIR" -name "*.sh" 2>/dev/null | wc -l | tr -d ' ')
    installed_components+=("Libraries: $lib_count files in $LIB_DIR")
fi

# Check config
if [ -f "$CONFIG_DIR/config.yaml" ]; then
    installed_components+=("Config: $CONFIG_DIR/config.yaml")
fi

# Check completions
if [ -d "$COMPLETIONS_DIR" ]; then
    installed_components+=("Completions: $COMPLETIONS_DIR")
fi

# Check user data
if [ -d "$GRALPH_STATE_DIR" ]; then
    log_count=$(find "$GRALPH_STATE_DIR" -name "*.log" 2>/dev/null | wc -l | tr -d ' ')
    installed_components+=("User data: $GRALPH_STATE_DIR ($log_count logs)")
fi

echo ""
if [ ${#installed_components[@]} -eq 0 ]; then
    echo_info "No gralph installation found."
    exit 0
fi

echo "Found installed components:"
for component in "${installed_components[@]}"; do
    echo "  - $component"
done
echo ""

# Confirm uninstallation
if [ "$REMOVE_ALL" = true ]; then
    echo_warn "This will remove ALL gralph data including logs and state."
else
    echo_info "This will remove gralph (logs and state will be preserved)."
    echo "      Use --all to also remove logs and state."
fi
echo ""

if ! confirm "Proceed with uninstallation?"; then
    echo_info "Uninstallation cancelled."
    exit 0
fi

echo ""

# Perform uninstallation
uninstall_binary
echo ""

uninstall_libraries
echo ""

uninstall_completions
echo ""

uninstall_config
echo ""

if [ "$REMOVE_ALL" = true ]; then
    uninstall_user_data
    echo ""
fi

# =============================================================================
# Summary
# =============================================================================

echo "============================================================"
echo -e "${GREEN} gralph uninstalled successfully!${NC}"
echo "============================================================"
echo ""

if [ "$REMOVE_ALL" = true ]; then
    echo "All gralph components and data have been removed."
else
    echo "gralph has been removed."
    if [ -d "$GRALPH_STATE_DIR" ]; then
        echo ""
        echo "Note: User data was preserved in $GRALPH_STATE_DIR"
        echo "      To remove it, run: rm -rf $GRALPH_STATE_DIR"
        echo "      Or use: ./uninstall.sh --all"
    fi
fi

echo ""
echo "If you added gralph to your PATH or shell completions manually,"
echo "remember to remove those lines from your shell configuration."
echo ""
