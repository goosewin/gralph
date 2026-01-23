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
# Install Path Determination
# =============================================================================

# Check if a directory is in PATH
is_in_path() {
    local dir="$1"
    [[ ":$PATH:" == *":$dir:"* ]]
}

# Check if we can write to a directory (or create it)
can_write_to() {
    local dir="$1"
    if [ -d "$dir" ]; then
        [ -w "$dir" ]
    else
        # Check if parent is writable (could create dir)
        local parent=$(dirname "$dir")
        [ -d "$parent" ] && [ -w "$parent" ]
    fi
}

# Determine the best install path
# Priority:
#   1. ~/.local/bin if in PATH (user-local, no sudo)
#   2. /usr/local/bin if writable (system-wide)
#   3. Create ~/.local/bin and add to PATH
determine_install_path() {
    local user_bin="$HOME/.local/bin"
    local system_bin="/usr/local/bin"

    echo_info "Determining install path..."

    # Option 1: ~/.local/bin already in PATH
    if is_in_path "$user_bin"; then
        if can_write_to "$user_bin"; then
            echo_info "  ✓ $user_bin is in PATH and writable"
            INSTALL_PATH="$user_bin"
            NEEDS_PATH_UPDATE=false
            return 0
        fi
    fi

    # Option 2: /usr/local/bin is writable (running as root or has permissions)
    if can_write_to "$system_bin" && is_in_path "$system_bin"; then
        echo_info "  ✓ $system_bin is writable"
        INSTALL_PATH="$system_bin"
        NEEDS_PATH_UPDATE=false
        return 0
    fi

    # Option 3: ~/.local/bin exists but not in PATH
    if [ -d "$user_bin" ] && can_write_to "$user_bin"; then
        echo_warn "  $user_bin exists but is not in PATH"
        INSTALL_PATH="$user_bin"
        NEEDS_PATH_UPDATE=true
        return 0
    fi

    # Option 4: Create ~/.local/bin
    if can_write_to "$HOME/.local" || can_write_to "$HOME"; then
        echo_info "  Creating $user_bin..."
        mkdir -p "$user_bin"
        INSTALL_PATH="$user_bin"
        NEEDS_PATH_UPDATE=true
        return 0
    fi

    # Fallback: Ask user
    echo ""
    echo_warn "Could not determine a writable install path."
    echo "      Options:"
    echo "        1) Use sudo to install to $system_bin"
    echo "        2) Specify a custom path"
    echo ""
    read -p "Choice [1/2]: " -n 1 -r
    echo ""

    case "$REPLY" in
        1)
            INSTALL_PATH="$system_bin"
            USE_SUDO=true
            NEEDS_PATH_UPDATE=false
            ;;
        2)
            read -p "Enter install path: " custom_path
            custom_path="${custom_path/#\~/$HOME}"  # Expand ~
            if [ -z "$custom_path" ]; then
                echo_error "No path specified."
                return 1
            fi
            mkdir -p "$custom_path" 2>/dev/null || {
                echo_error "Cannot create $custom_path"
                return 1
            }
            INSTALL_PATH="$custom_path"
            NEEDS_PATH_UPDATE=! is_in_path "$custom_path"
            ;;
        *)
            echo_error "Invalid choice."
            return 1
            ;;
    esac

    return 0
}

# Print PATH update instructions for various shells
print_path_instructions() {
    local install_dir="$1"

    echo ""
    echo_warn "Add the following to your shell configuration:"
    echo ""

    # Detect current shell
    local shell_name=$(basename "$SHELL")

    case "$shell_name" in
        bash)
            echo "    # Add to ~/.bashrc or ~/.bash_profile:"
            echo "    export PATH=\"$install_dir:\$PATH\""
            ;;
        zsh)
            echo "    # Add to ~/.zshrc:"
            echo "    export PATH=\"$install_dir:\$PATH\""
            ;;
        fish)
            echo "    # Add to ~/.config/fish/config.fish:"
            echo "    set -gx PATH $install_dir \$PATH"
            ;;
        *)
            echo "    export PATH=\"$install_dir:\$PATH\""
            ;;
    esac

    echo ""
    echo "    Then run: source ~/.<shell>rc  (or start a new terminal)"
}

# =============================================================================
# Main Installation (to be implemented in subsequent tasks)
# =============================================================================

# Global variables set by determine_install_path
INSTALL_PATH=""
NEEDS_PATH_UPDATE=false
USE_SUDO=false

echo "rloop installer"
echo "==============="
echo ""

# Check dependencies first
if ! check_dependencies; then
    echo_error "Please install missing dependencies and run installer again."
    exit 1
fi

# Determine install path
echo ""
if ! determine_install_path; then
    echo_error "Could not determine install path."
    exit 1
fi

echo ""
echo_info "Install path: $INSTALL_PATH"
if [ "$NEEDS_PATH_UPDATE" = true ]; then
    echo_warn "Note: $INSTALL_PATH is not in your PATH"
fi
if [ "$USE_SUDO" = true ]; then
    echo_warn "Note: Will use sudo for installation"
fi

# =============================================================================
# Copy rloop binary to install path
# =============================================================================

copy_rloop_binary() {
    local source_bin="$SCRIPT_DIR/bin/rloop"
    local dest_bin="$INSTALL_PATH/rloop"

    # Verify source exists
    if [ ! -f "$source_bin" ]; then
        echo_error "Source binary not found: $source_bin"
        return 1
    fi

    echo_info "Installing rloop binary..."

    # Copy with or without sudo
    if [ "$USE_SUDO" = true ]; then
        if sudo cp "$source_bin" "$dest_bin"; then
            sudo chmod +x "$dest_bin"
            echo_info "  ✓ Copied to $dest_bin (with sudo)"
        else
            echo_error "  ✗ Failed to copy rloop binary"
            return 1
        fi
    else
        if cp "$source_bin" "$dest_bin"; then
            chmod +x "$dest_bin"
            echo_info "  ✓ Copied to $dest_bin"
        else
            echo_error "  ✗ Failed to copy rloop binary"
            return 1
        fi
    fi

    return 0
}

echo ""
if ! copy_rloop_binary; then
    echo_error "Failed to install rloop binary."
    exit 1
fi

# =============================================================================
# Copy lib/ to ~/.config/rloop/lib/
# =============================================================================

CONFIG_DIR="$HOME/.config/rloop"
LIB_DIR="$CONFIG_DIR/lib"

copy_lib_files() {
    local source_lib="$SCRIPT_DIR/lib"

    # Verify source lib directory exists
    if [ ! -d "$source_lib" ]; then
        echo_error "Source lib directory not found: $source_lib"
        return 1
    fi

    echo_info "Installing library files..."

    # Create config directory structure
    if ! mkdir -p "$LIB_DIR"; then
        echo_error "  ✗ Failed to create $LIB_DIR"
        return 1
    fi
    echo_info "  ✓ Created $CONFIG_DIR"

    # Copy all .sh files from lib/
    local copied=0
    local failed=0
    for file in "$source_lib"/*.sh; do
        if [ -f "$file" ]; then
            local filename=$(basename "$file")
            if cp "$file" "$LIB_DIR/$filename"; then
                chmod +x "$LIB_DIR/$filename"
                echo_info "  ✓ Copied $filename"
                ((copied++))
            else
                echo_error "  ✗ Failed to copy $filename"
                ((failed++))
            fi
        fi
    done

    if [ $failed -gt 0 ]; then
        echo_error "Failed to copy $failed file(s)"
        return 1
    fi

    echo_info "  ✓ Installed $copied library files to $LIB_DIR"
    return 0
}

echo ""
if ! copy_lib_files; then
    echo_error "Failed to install library files."
    exit 1
fi

# =============================================================================
# Create default config
# =============================================================================

create_default_config() {
    local source_config="$SCRIPT_DIR/config/default.yaml"
    local dest_config="$CONFIG_DIR/config.yaml"

    echo_info "Setting up configuration..."

    # Ensure config directory exists (should already exist from lib copy)
    if ! mkdir -p "$CONFIG_DIR"; then
        echo_error "  ✗ Failed to create $CONFIG_DIR"
        return 1
    fi

    # Check if config already exists
    if [ -f "$dest_config" ]; then
        echo_info "  ✓ Config already exists at $dest_config (keeping existing)"
        return 0
    fi

    # Verify source config exists
    if [ ! -f "$source_config" ]; then
        echo_warn "  Default config not found at $source_config"
        echo_warn "  Creating minimal default config..."

        # Create minimal default config inline
        cat > "$dest_config" << 'YAML_EOF'
# rloop configuration
defaults:
  max_iterations: 30
  task_file: PRD.md
  completion_marker: COMPLETE
  model: claude-sonnet-4-20250514

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

notifications:
  on_complete: true

logging:
  level: info
  retain_days: 7
YAML_EOF
        echo_info "  ✓ Created default config at $dest_config"
        return 0
    fi

    # Copy default config
    if cp "$source_config" "$dest_config"; then
        echo_info "  ✓ Created config at $dest_config"
        return 0
    else
        echo_error "  ✗ Failed to create config file"
        return 1
    fi
}

echo ""
if ! create_default_config; then
    echo_error "Failed to create default config."
    exit 1
fi

echo ""
echo_warn "Remaining installation steps not yet implemented."
echo "See PRD.md Unit 06 for remaining tasks."

# Print PATH update instructions if needed
if [ "$NEEDS_PATH_UPDATE" = true ]; then
    print_path_instructions "$INSTALL_PATH"
fi
