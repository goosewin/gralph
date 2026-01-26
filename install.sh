#!/bin/bash
# Installation script for gralph
# See PRD.md Unit 06 for full specification
#
# Dual-mode installer:
#   - Local mode: Run from cloned repo (./install.sh)
#   - Bootstrap mode: Run via curl|bash (curl ... | bash)

set -e
set -o pipefail

# =============================================================================
# Install Mode Detection (INS-1)
# =============================================================================

# Detect install context: local repo vs piped install
# Sets INSTALL_MODE to "local" or "bootstrap"
detect_install_mode() {
    # When piped via curl|bash, BASH_SOURCE[0] is typically empty or "bash"
    # When run from a file, BASH_SOURCE[0] contains the script path
    local script_source="${BASH_SOURCE[0]:-}"

    # Check if we're being piped (stdin is not a terminal and BASH_SOURCE is empty/bash)
    if [ -z "$script_source" ] || [ "$script_source" = "bash" ] || [ "$script_source" = "/bin/bash" ] || [ "$script_source" = "/usr/bin/bash" ]; then
        # Piped execution - bootstrap mode
        INSTALL_MODE="bootstrap"
        SCRIPT_DIR=""
        return 0
    fi

    # We have a script path - check if it's a local repo
    local script_dir
    script_dir="$(cd "$(dirname "$script_source")" && pwd)"

    # Check if the local repo structure exists
    if [ -f "$script_dir/bin/gralph" ]; then
        INSTALL_MODE="local"
        SCRIPT_DIR="$script_dir"
        return 0
    fi

    # Script exists but no bin/gralph - likely extracted tarball or incomplete
    # Treat as bootstrap to trigger download
    INSTALL_MODE="bootstrap"
    SCRIPT_DIR=""
    return 0
}

# Global variables for install mode
INSTALL_MODE=""
SCRIPT_DIR=""

# Detect mode immediately
detect_install_mode

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
# Note: Either 'claude' or 'opencode' is required (at least one backend)
REQUIRED_DEPS=("jq" "tmux")
BACKEND_DEPS=("claude" "opencode")
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
        opencode)
            echo "See https://opencode.ai/docs/cli/ for installation"
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
        opencode)
            # OpenCode installation varies by platform
            echo_error "OpenCode auto-install not supported."
            echo "      See https://opencode.ai/docs/cli/ for installation instructions."
            return 1
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
        echo_info "  $dep installed successfully"
        return 0
    else
        echo_error "  Failed to install $dep"
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
    local available_backends=()
    local missing_backends=()

    echo_info "Checking required dependencies..."

    # Check required dependencies
    for dep in "${REQUIRED_DEPS[@]}"; do
        if check_command "$dep"; then
            echo_info "  $dep found"
        else
            echo_error "  $dep not found"
            missing_required+=("$dep")
        fi
    done

    # Check backend dependencies (at least one required)
    echo_info "Checking AI backend dependencies (at least one required)..."
    for dep in "${BACKEND_DEPS[@]}"; do
        if check_command "$dep"; then
            echo_info "  $dep found"
            available_backends+=("$dep")
        else
            echo_warn "  - $dep not found"
            missing_backends+=("$dep")
        fi
    done

    # Check optional dependencies
    echo_info "Checking optional dependencies..."
    for dep in "${OPTIONAL_DEPS[@]}"; do
        if check_command "$dep"; then
            echo_info "  $dep found"
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

    # Handle missing backend dependencies
    if [ ${#available_backends[@]} -eq 0 ]; then
        echo ""
        echo_error "No AI backend found. At least one is required:"
        echo_error "  - claude (Claude Code): $(get_install_hint "claude")"
        echo_error "  - opencode (OpenCode): $(get_install_hint "opencode")"
        echo ""
        echo_info "Would you like to install Claude Code (requires npm)?"
        read -p "Install Claude Code? [y/N] " -n 1 -r
        echo ""

        if [[ $REPLY =~ ^[Yy]$ ]]; then
            if install_dependency "claude"; then
                available_backends+=("claude")
            else
                echo ""
                echo_error "Please install at least one AI backend manually."
                return 1
            fi
        else
            echo ""
            echo_error "Please install at least one AI backend manually."
            return 1
        fi
    else
        echo ""
        echo_info "Available backends: ${available_backends[*]}"
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
            echo_info "  $user_bin is in PATH and writable"
            INSTALL_PATH="$user_bin"
            NEEDS_PATH_UPDATE=false
            return 0
        fi
    fi

    # Option 2: /usr/local/bin is writable (running as root or has permissions)
    if can_write_to "$system_bin"; then
        echo_info "  $system_bin is writable"
        INSTALL_PATH="$system_bin"
        if is_in_path "$system_bin"; then
            NEEDS_PATH_UPDATE=false
        else
            NEEDS_PATH_UPDATE=true
        fi
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
            if is_in_path "$custom_path"; then
                NEEDS_PATH_UPDATE=false
            else
                NEEDS_PATH_UPDATE=true
            fi
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
    local shell_name="${SHELL##*/}"
    if [ -z "$shell_name" ]; then
        shell_name="unknown"
    fi

    case "$shell_name" in
        bash)
            echo "    # Add to ~/.bashrc (Linux) or ~/.bash_profile (macOS):"
            echo "    export PATH=\"$install_dir:\$PATH\""
            echo ""
            echo "    Then run: source ~/.bashrc  (or start a new terminal)"
            ;;
        zsh)
            echo "    # Add to ~/.zshrc:"
            echo "    export PATH=\"$install_dir:\$PATH\""
            echo ""
            echo "    Then run: source ~/.zshrc  (or start a new terminal)"
            ;;
        fish)
            echo "    # Add to ~/.config/fish/config.fish:"
            echo "    set -gx PATH $install_dir \$PATH"
            echo ""
            echo "    Then run: source ~/.config/fish/config.fish  (or start a new terminal)"
            ;;
        *)
            echo "    export PATH=\"$install_dir:\$PATH\""
            echo ""
            echo "    Then start a new terminal to pick up PATH"
            ;;
    esac
}

# =============================================================================
# Bootstrap Download (INS-2)
# =============================================================================

# Default bootstrap settings
GRALPH_REPO="${GRALPH_REPO:-goosewin/gralph}"
GRALPH_REF="${GRALPH_REF:-}"  # Empty means latest release
GRALPH_ASSET_URL="${GRALPH_ASSET_URL:-}"  # Direct URL override

# Cleanup function for bootstrap mode
bootstrap_cleanup() {
    if [ -n "${BOOTSTRAP_TEMP_DIR:-}" ] && [ -d "$BOOTSTRAP_TEMP_DIR" ]; then
        rm -rf "$BOOTSTRAP_TEMP_DIR"
    fi
}

# Download and extract release tarball for bootstrap mode
# Sets SCRIPT_DIR to the extracted directory on success
bootstrap_download() {
    echo_info "Downloading gralph..."

    # Verify tar is available for extraction
    if ! check_command tar; then
        echo_error "tar is required to extract the release archive."
        echo "      Please install tar and try again."
        return 1
    fi

    # Verify curl is available
    if ! check_command curl; then
        echo_error "curl is required for bootstrap installation."
        echo "      Please install curl and try again."
        return 1
    fi

    # Create temp directory
    BOOTSTRAP_TEMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t 'gralph-install')
    if [ ! -d "$BOOTSTRAP_TEMP_DIR" ]; then
        echo_error "Failed to create temporary directory."
        return 1
    fi

    # Set trap to clean up on exit
    trap bootstrap_cleanup EXIT

    local tarball_url=""

    if [ -n "$GRALPH_ASSET_URL" ]; then
        # Direct URL override
        tarball_url="$GRALPH_ASSET_URL"
        echo_info "  Using custom asset URL"
    elif [ -n "$GRALPH_REF" ]; then
        # Specific ref (tag or branch)
        tarball_url="https://github.com/${GRALPH_REPO}/archive/refs/tags/${GRALPH_REF}.tar.gz"
        echo_info "  Downloading ref: $GRALPH_REF"
    else
        # Latest release - get the tarball URL from GitHub API
        echo_info "  Fetching latest release..."
        local release_url="https://api.github.com/repos/${GRALPH_REPO}/releases/latest"
        local tarball_url_raw

        tarball_url_raw=$(curl -fsSL "$release_url" 2>/dev/null | grep '"tarball_url"' | head -1 | cut -d'"' -f4)

        if [ -z "$tarball_url_raw" ]; then
            # Fallback to main branch if no releases
            echo_warn "  No releases found, falling back to main branch"
            tarball_url="https://github.com/${GRALPH_REPO}/archive/refs/heads/main.tar.gz"
        else
            tarball_url="$tarball_url_raw"
            echo_info "  Found latest release"
        fi
    fi

    # Download tarball
    local tarball_path="$BOOTSTRAP_TEMP_DIR/gralph.tar.gz"
    echo_info "  Downloading from: $tarball_url"

    if ! curl -fsSL -o "$tarball_path" "$tarball_url"; then
        echo_error "Failed to download gralph."
        echo "      URL: $tarball_url"
        return 1
    fi

    # Verify download
    if [ ! -f "$tarball_path" ] || [ ! -s "$tarball_path" ]; then
        echo_error "Downloaded file is empty or missing."
        return 1
    fi

    echo_info "  Downloaded successfully"

    # Extract tarball
    echo_info "  Extracting..."
    local extract_dir="$BOOTSTRAP_TEMP_DIR/extract"
    mkdir -p "$extract_dir"

    if ! tar -xzf "$tarball_path" -C "$extract_dir" 2>/dev/null; then
        echo_error "Failed to extract tarball."
        return 1
    fi

    # Find the extracted directory (GitHub archives have a prefix like repo-ref/)
    local extracted_root
    extracted_root=$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d | head -1)

    if [ -z "$extracted_root" ] || [ ! -d "$extracted_root" ]; then
        echo_error "Could not find extracted directory."
        return 1
    fi

    # Verify expected layout
    if [ ! -f "$extracted_root/bin/gralph" ]; then
        echo_error "Invalid archive layout: bin/gralph not found."
        echo "      Expected: bin/gralph"
        echo "      Archive may be corrupt or from wrong source."
        return 1
    fi

    if [ ! -d "$extracted_root/lib" ]; then
        echo_error "Invalid archive layout: lib/ directory not found."
        return 1
    fi

    # Verify completions directory exists (warn but don't fail - INS-5)
    if [ ! -d "$extracted_root/completions" ]; then
        echo_warn "Completions directory not found in archive."
        echo_warn "Shell completions will not be installed."
    fi

    # Set SCRIPT_DIR to extracted location
    SCRIPT_DIR="$extracted_root"
    echo_info "  Extracted to: $SCRIPT_DIR"

    return 0
}

# =============================================================================
# Main Installation
# =============================================================================

# Global variables set by determine_install_path
INSTALL_PATH=""
NEEDS_PATH_UPDATE=false
USE_SUDO=false
BOOTSTRAP_TEMP_DIR=""

echo "gralph installer"
echo "==============="
echo ""

# Display detected install mode
if [ "$INSTALL_MODE" = "local" ]; then
    echo_info "Install mode: local (from cloned repo)"
    echo_info "Source directory: $SCRIPT_DIR"
elif [ "$INSTALL_MODE" = "bootstrap" ]; then
    echo_info "Install mode: bootstrap (curl|bash)"

    # Download and extract in bootstrap mode
    if ! bootstrap_download; then
        echo_error "Bootstrap download failed."
        exit 1
    fi
fi
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
# Copy gralph binary to install path
# =============================================================================

copy_gralph_binary() {
    local source_bin="$SCRIPT_DIR/bin/gralph"
    local dest_bin="$INSTALL_PATH/gralph"

    # Verify source exists
    if [ ! -f "$source_bin" ]; then
        echo_error "Source binary not found: $source_bin"
        return 1
    fi

    echo_info "Installing gralph binary..."

    # Copy with or without sudo
    if [ "$USE_SUDO" = true ]; then
        if sudo cp "$source_bin" "$dest_bin"; then
            sudo chmod +x "$dest_bin"
            echo_info "  Copied to $dest_bin (with sudo)"
        else
            echo_error "  Failed to copy gralph binary"
            return 1
        fi
    else
        if cp "$source_bin" "$dest_bin"; then
            chmod +x "$dest_bin"
            echo_info "  Copied to $dest_bin"
        else
            echo_error "  Failed to copy gralph binary"
            return 1
        fi
    fi

    return 0
}

echo ""
if ! copy_gralph_binary; then
    echo_error "Failed to install gralph binary."
    exit 1
fi

# =============================================================================
# Copy lib/ to ~/.config/gralph/lib/
# =============================================================================

CONFIG_DIR="$HOME/.config/gralph"
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
        echo_error "  Failed to create $LIB_DIR"
        return 1
    fi
    echo_info "  Created $CONFIG_DIR"

    # Copy all .sh files from lib/
    local copied=0
    local failed=0
    for file in "$source_lib"/*.sh; do
        if [ -f "$file" ]; then
            local filename=$(basename "$file")
            if cp "$file" "$LIB_DIR/$filename"; then
                chmod +x "$LIB_DIR/$filename"
                echo_info "  Copied $filename"
                ((copied++))
            else
                echo_error "  Failed to copy $filename"
                ((failed++))
            fi
        fi
    done

    # Copy backends directory
    if [ -d "$source_lib/backends" ]; then
        local backends_dir="$LIB_DIR/backends"
        if ! mkdir -p "$backends_dir"; then
            echo_error "  Failed to create $backends_dir"
            return 1
        fi

        for file in "$source_lib/backends"/*.sh; do
            if [ -f "$file" ]; then
                local filename=$(basename "$file")
                if cp "$file" "$backends_dir/$filename"; then
                    chmod +x "$backends_dir/$filename"
                    echo_info "  Copied backends/$filename"
                    ((copied++))
                else
                    echo_error "  Failed to copy backends/$filename"
                    ((failed++))
                fi
            fi
        done
    fi

    if [ $failed -gt 0 ]; then
        echo_error "Failed to copy $failed file(s)"
        return 1
    fi

    echo_info "  Installed $copied library files to $LIB_DIR"
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
        echo_error "  Failed to create $CONFIG_DIR"
        return 1
    fi

    # Check if config already exists
    if [ -f "$dest_config" ]; then
        echo_info "  Config already exists at $dest_config (keeping existing)"
        return 0
    fi

    # Verify source config exists
    if [ ! -f "$source_config" ]; then
        echo_warn "  Default config not found at $source_config"
        echo_warn "  Creating minimal default config..."

        # Create minimal default config inline
        cat > "$dest_config" << 'YAML_EOF'
# gralph configuration
defaults:
  max_iterations: 30
  task_file: PRD.md
  completion_marker: COMPLETE
  model: claude-opus-4-5

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
        echo_info "  Created default config at $dest_config"
        return 0
    fi

    # Copy default config
    if cp "$source_config" "$dest_config"; then
        echo_info "  Created config at $dest_config"
        return 0
    else
        echo_error "  Failed to create config file"
        return 1
    fi
}

echo ""
if ! create_default_config; then
    echo_error "Failed to create default config."
    exit 1
fi

# Ensure default config template is available for runtime defaults
install_default_config_template() {
    local source_config="$SCRIPT_DIR/config/default.yaml"
    local dest_dir="$CONFIG_DIR/config"
    local dest_config="$dest_dir/default.yaml"

    if [ -f "$dest_config" ]; then
        return 0
    fi

    if [ ! -f "$source_config" ]; then
        echo_warn "  Default config template not found at $source_config"
        return 0
    fi

    if ! mkdir -p "$dest_dir"; then
        echo_warn "  Failed to create $dest_dir"
        return 0
    fi

    if cp "$source_config" "$dest_config"; then
        echo_info "  Installed default config template at $dest_config"
    else
        echo_warn "  Failed to install default config template at $dest_config"
    fi
}

install_default_config_template

# =============================================================================
# Install shell completions
# =============================================================================

COMPLETIONS_DIR="$CONFIG_DIR/completions"

install_shell_completions() {
    local source_completions="$SCRIPT_DIR/completions"

    echo_info "Installing shell completions..."

    # Verify source completions directory exists
    if [ ! -d "$source_completions" ]; then
        echo_warn "  Completions directory not found: $source_completions"
        echo_warn "  Skipping shell completions installation"
        return 0
    fi

    # Create completions directory in config
    if ! mkdir -p "$COMPLETIONS_DIR"; then
        echo_warn "  Failed to create $COMPLETIONS_DIR"
        echo_warn "  Skipping shell completions installation"
        return 0
    fi

    # Copy bash completion
    if [ -f "$source_completions/gralph.bash" ]; then
        if cp "$source_completions/gralph.bash" "$COMPLETIONS_DIR/gralph.bash"; then
            echo_info "  Copied bash completions"
        else
            echo_warn "  Failed to copy bash completions"
        fi
    fi

    # Copy zsh completion
    if [ -f "$source_completions/gralph.zsh" ]; then
        if cp "$source_completions/gralph.zsh" "$COMPLETIONS_DIR/_gralph"; then
            echo_info "  Copied zsh completions"
        else
            echo_warn "  Failed to copy zsh completions"
        fi
    fi

    # Detect shell and provide setup instructions
    local shell_name="${SHELL##*/}"
    if [ -z "$shell_name" ]; then
        shell_name="unknown"
    fi

    # Try to install to system directories if possible
    local bash_installed=false
    local zsh_installed=false

    # Bash: Try /etc/bash_completion.d or ~/.local/share/bash-completion/completions
    if [ -f "$COMPLETIONS_DIR/gralph.bash" ]; then
        local bash_system_dir="/etc/bash_completion.d"
        local bash_user_dir="$HOME/.local/share/bash-completion/completions"

        if [ -d "$bash_system_dir" ] && [ -w "$bash_system_dir" ]; then
            if cp "$COMPLETIONS_DIR/gralph.bash" "$bash_system_dir/gralph"; then
                echo_info "  Installed bash completions to $bash_system_dir"
                bash_installed=true
            fi
        elif [ "$USE_SUDO" = true ] && [ -d "$bash_system_dir" ]; then
            if sudo cp "$COMPLETIONS_DIR/gralph.bash" "$bash_system_dir/gralph"; then
                echo_info "  Installed bash completions to $bash_system_dir (with sudo)"
                bash_installed=true
            fi
        fi

        if [ "$bash_installed" = false ]; then
            mkdir -p "$bash_user_dir" 2>/dev/null
            if [ -d "$bash_user_dir" ] && cp "$COMPLETIONS_DIR/gralph.bash" "$bash_user_dir/gralph" 2>/dev/null; then
                echo_info "  Installed bash completions to $bash_user_dir"
                bash_installed=true
            fi
        fi
    fi

    # Zsh: Try /usr/local/share/zsh/site-functions or user fpath
    if [ -f "$COMPLETIONS_DIR/_gralph" ]; then
        local zsh_system_dir="/usr/local/share/zsh/site-functions"
        local zsh_user_dir="$HOME/.zsh/completions"

        if [ -d "$zsh_system_dir" ] && [ -w "$zsh_system_dir" ]; then
            if cp "$COMPLETIONS_DIR/_gralph" "$zsh_system_dir/_gralph"; then
                echo_info "  Installed zsh completions to $zsh_system_dir"
                zsh_installed=true
            fi
        elif [ "$USE_SUDO" = true ] && [ -d "$zsh_system_dir" ]; then
            if sudo cp "$COMPLETIONS_DIR/_gralph" "$zsh_system_dir/_gralph"; then
                echo_info "  Installed zsh completions to $zsh_system_dir (with sudo)"
                zsh_installed=true
            fi
        fi

        if [ "$zsh_installed" = false ]; then
            mkdir -p "$zsh_user_dir" 2>/dev/null
            if [ -d "$zsh_user_dir" ] && cp "$COMPLETIONS_DIR/_gralph" "$zsh_user_dir/_gralph" 2>/dev/null; then
                echo_info "  Installed zsh completions to $zsh_user_dir"
                zsh_installed=true
            fi
        fi
    fi

    # Store installation status for later instructions
    BASH_COMPLETION_INSTALLED=$bash_installed
    ZSH_COMPLETION_INSTALLED=$zsh_installed

    return 0
}

# Print shell completion setup instructions
print_completion_instructions() {
    local shell_name="${SHELL##*/}"
    if [ -z "$shell_name" ]; then
        shell_name="unknown"
    fi

    # Only print if completions weren't auto-installed
    if [ "$BASH_COMPLETION_INSTALLED" = true ] && [ "$ZSH_COMPLETION_INSTALLED" = true ]; then
        return 0
    fi

    echo ""
    echo_info "Shell completion setup:"

    case "$shell_name" in
        bash)
            if [ "$BASH_COMPLETION_INSTALLED" != true ]; then
                echo ""
                echo "    # Add to ~/.bashrc:"
                echo "    source $COMPLETIONS_DIR/gralph.bash"
            fi
            ;;
        zsh)
            if [ "$ZSH_COMPLETION_INSTALLED" != true ]; then
                echo ""
                echo "    # Add to ~/.zshrc (before compinit):"
                echo "    fpath=($COMPLETIONS_DIR \$fpath)"
                echo "    autoload -Uz compinit && compinit"
            fi
            ;;
        *)
            echo ""
            echo "    For bash, add to ~/.bashrc:"
            echo "      source $COMPLETIONS_DIR/gralph.bash"
            echo ""
            echo "    For zsh, add to ~/.zshrc (before compinit):"
            echo "      fpath=($COMPLETIONS_DIR \$fpath)"
            ;;
    esac
}

BASH_COMPLETION_INSTALLED=false
ZSH_COMPLETION_INSTALLED=false

echo ""
install_shell_completions

# Print PATH update instructions if needed
if [ "$NEEDS_PATH_UPDATE" = true ]; then
    print_path_instructions "$INSTALL_PATH"
fi

# Print completion instructions if needed
print_completion_instructions

# =============================================================================
# Print success message with usage examples
# =============================================================================

print_success_message() {
    echo ""
    echo "============================================================"
    echo -e "${GREEN} gralph installed successfully!${NC}"
    echo "============================================================"
    echo ""
    echo "Installation summary:"
    echo "  Binary:       $INSTALL_PATH/gralph"
    echo "  Libraries:    $LIB_DIR/"
    echo "  Config:       $CONFIG_DIR/config.yaml"
    echo "  Completions:  $COMPLETIONS_DIR/"
    echo ""
    echo "Quick start:"
    echo ""
    echo "  # Start a loop in the current directory"
    echo "  gralph start ."
    echo ""
    echo "  # Start with custom options"
    echo "  gralph start ~/my-project --name myapp --max-iterations 50"
    echo ""
    echo "  # Check status of running loops"
    echo "  gralph status"
    echo ""
    echo "  # View logs for a running loop"
    echo "  gralph logs myapp --follow"
    echo ""
    echo "  # Stop a running loop"
    echo "  gralph stop myapp"
    echo ""
    echo "  # Resume loops after reboot/crash"
    echo "  gralph resume"
    echo ""
    echo "  # Generate a spec-compliant PRD"
    echo "  gralph prd create --dir . --output PRD.generated.md --goal \"Add a billing dashboard\""
    echo ""
    echo "  # Validate an existing PRD"
    echo "  gralph prd check PRD.generated.md"
    echo ""
    echo "Documentation:"
    echo "  gralph --help              Show all commands and options"
    echo "  gralph <command> --help    Show help for a specific command"
    echo ""

    if [ "$NEEDS_PATH_UPDATE" = true ]; then
        echo_warn "Remember to update your PATH (see instructions above)"
        echo ""
    fi

    echo "Happy coding!"
    echo ""
}

print_success_message
