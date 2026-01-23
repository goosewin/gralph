#!/bin/bash
# Installation script for rloop
# See PRD.md Unit 06 for full specification

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
# Dependency Checking
# =============================================================================

# Required dependencies per NFR-1 in PRD.md
REQUIRED_DEPS=("claude" "jq" "tmux")
OPTIONAL_DEPS=("curl")

# Check if a command exists
check_command() {
    command -v "$1" &> /dev/null
}

# Get package manager
get_package_manager() {
    if check_command apt-get; then
        echo "apt"
    elif check_command brew; then
        echo "brew"
    elif check_command dnf; then
        echo "dnf"
    elif check_command yum; then
        echo "yum"
    elif check_command pacman; then
        echo "pacman"
    else
        echo "unknown"
    fi
}

# Get install command for a dependency
get_install_hint() {
    local dep="$1"
    local pkg_mgr=$(get_package_manager)

    case "$dep" in
        claude)
            echo "npm install -g @anthropic-ai/claude-code"
            ;;
        jq)
            case "$pkg_mgr" in
                apt) echo "sudo apt-get install jq" ;;
                brew) echo "brew install jq" ;;
                dnf|yum) echo "sudo $pkg_mgr install jq" ;;
                pacman) echo "sudo pacman -S jq" ;;
                *) echo "Install jq from https://stedolan.github.io/jq/" ;;
            esac
            ;;
        tmux)
            case "$pkg_mgr" in
                apt) echo "sudo apt-get install tmux" ;;
                brew) echo "brew install tmux" ;;
                dnf|yum) echo "sudo $pkg_mgr install tmux" ;;
                pacman) echo "sudo pacman -S tmux" ;;
                *) echo "Install tmux from https://github.com/tmux/tmux" ;;
            esac
            ;;
        curl)
            case "$pkg_mgr" in
                apt) echo "sudo apt-get install curl" ;;
                brew) echo "brew install curl" ;;
                dnf|yum) echo "sudo $pkg_mgr install curl" ;;
                pacman) echo "sudo pacman -S curl" ;;
                *) echo "Install curl from https://curl.se/" ;;
            esac
            ;;
        *)
            echo "Install $dep using your package manager"
            ;;
    esac
}

# Install a dependency using the detected package manager
# Returns 0 on success, 1 on failure
install_dependency() {
    local dep="$1"
    local pkg_mgr=$(get_package_manager)

    echo_info "Attempting to install $dep..."

    case "$dep" in
        claude)
            # Claude requires npm
            if check_command npm; then
                npm install -g @anthropic-ai/claude-code
            else
                echo_error "npm not found. Cannot auto-install claude."
                echo "      Install Node.js first: https://nodejs.org/"
                return 1
            fi
            ;;
        jq|tmux|curl)
            case "$pkg_mgr" in
                apt)
                    sudo apt-get update -qq && sudo apt-get install -y "$dep"
                    ;;
                brew)
                    brew install "$dep"
                    ;;
                dnf)
                    sudo dnf install -y "$dep"
                    ;;
                yum)
                    sudo yum install -y "$dep"
                    ;;
                pacman)
                    sudo pacman -S --noconfirm "$dep"
                    ;;
                *)
                    echo_error "Unknown package manager. Cannot auto-install $dep."
                    return 1
                    ;;
            esac
            ;;
        *)
            echo_error "Unknown dependency: $dep"
            return 1
            ;;
    esac

    # Verify installation succeeded
    if check_command "$dep"; then
        echo_info "  ✓ $dep installed successfully"
        return 0
    else
        echo_error "  ✗ Failed to install $dep"
        return 1
    fi
}

# Prompt user to install missing dependencies
# Args: dependency names...
prompt_install_missing() {
    local deps=("$@")
    local pkg_mgr=$(get_package_manager)

    if [ "$pkg_mgr" = "unknown" ]; then
        echo_error "No supported package manager found (apt, brew, dnf, yum, pacman)."
        echo "      Please install the following dependencies manually:"
        for dep in "${deps[@]}"; do
            echo "        - $dep: $(get_install_hint "$dep")"
        done
        return 1
    fi

    echo ""
    echo_info "Would you like to install missing dependencies automatically?"
    echo "      Package manager detected: $pkg_mgr"
    echo "      Dependencies to install: ${deps[*]}"
    echo ""
    read -p "Install now? [y/N] " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        local failed=()
        for dep in "${deps[@]}"; do
            if ! install_dependency "$dep"; then
                failed+=("$dep")
            fi
        done

        if [ ${#failed[@]} -gt 0 ]; then
            echo ""
            echo_error "Failed to install some dependencies:"
            for dep in "${failed[@]}"; do
                echo "      - $dep: $(get_install_hint "$dep")"
            done
            return 1
        fi
        return 0
    else
        echo ""
        echo_info "Skipping automatic installation."
        echo "      Please install manually:"
        for dep in "${deps[@]}"; do
            echo "        $(get_install_hint "$dep")"
        done
        return 1
    fi
}

# Check for required dependencies
# Returns 0 if all required deps are present, 1 otherwise
check_dependencies() {
    local missing_required=()
    local missing_optional=()

    echo_info "Checking required dependencies..."

    # Check required dependencies
    for dep in "${REQUIRED_DEPS[@]}"; do
        if check_command "$dep"; then
            echo_info "  ✓ $dep found"
        else
            echo_error "  ✗ $dep not found"
            missing_required+=("$dep")
        fi
    done

    # Check optional dependencies
    echo_info "Checking optional dependencies..."
    for dep in "${OPTIONAL_DEPS[@]}"; do
        if check_command "$dep"; then
            echo_info "  ✓ $dep found"
        else
            echo_warn "  - $dep not found (optional)"
            missing_optional+=("$dep")
        fi
    done

    # Handle missing required dependencies
    if [ ${#missing_required[@]} -gt 0 ]; then
        echo ""
        echo_error "Missing required dependencies:"
        for dep in "${missing_required[@]}"; do
            echo_error "  $dep"
            echo "      Install: $(get_install_hint "$dep")"
        done

        # Prompt to install missing required dependencies
        if prompt_install_missing "${missing_required[@]}"; then
            echo ""
            echo_info "All required dependencies now installed!"
        else
            echo ""
            return 1
        fi
    fi

    # Handle missing optional dependencies
    if [ ${#missing_optional[@]} -gt 0 ]; then
        echo ""
        echo_warn "Missing optional dependencies (notifications may not work):"
        for dep in "${missing_optional[@]}"; do
            echo_warn "  $dep"
            echo "      Install: $(get_install_hint "$dep")"
        done

        echo ""
        read -p "Install optional dependencies? [y/N] " -n 1 -r
        echo ""

        if [[ $REPLY =~ ^[Yy]$ ]]; then
            for dep in "${missing_optional[@]}"; do
                install_dependency "$dep" || true  # Don't fail on optional deps
            done
        fi
    fi

    echo ""
    echo_info "All required dependencies satisfied!"
    return 0
}

# =============================================================================
# Main Installation (to be implemented in subsequent tasks)
# =============================================================================

echo "rloop installer"
echo "==============="
echo ""

# Check dependencies first
if ! check_dependencies; then
    echo_error "Please install missing dependencies and run installer again."
    exit 1
fi

echo ""
echo_warn "Installation steps not yet implemented."
echo "See PRD.md Unit 06 for remaining tasks."
