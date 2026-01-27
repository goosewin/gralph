#!/usr/bin/env bash
# Gralph installer for Unix systems (Linux/macOS)
set -euo pipefail

REPO="goosewin/gralph"
# Default to user-local bin to avoid permission issues
INSTALL_DIR="${GRALPH_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${GRALPH_VERSION:-latest}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# Ensure install directory is on PATH
ensure_path() {
    local target_dir="$1"
    if [[ ":$PATH:" == *":${target_dir}:"* ]]; then
        return 0
    fi

    local shell_name rc_file
    shell_name="$(basename "${SHELL:-}")"
    case "$shell_name" in
        zsh)  rc_file="$HOME/.zshrc" ;;
        bash) rc_file="$HOME/.bashrc" ;;
        *)    rc_file="$HOME/.profile" ;;
    esac

    if [[ ! -f "$rc_file" ]]; then
        touch "$rc_file"
    fi

    if ! grep -Fqs "$target_dir" "$rc_file"; then
        echo "" >> "$rc_file"
        echo "# Added by Gralph installer" >> "$rc_file"
        echo "export PATH=\"$target_dir:\$PATH\"" >> "$rc_file"
        info "Added PATH entry to $rc_file"
    else
        info "PATH entry already present in $rc_file"
    fi

    info "Run 'source $rc_file' or open a new terminal"
}

# Detect OS and architecture
detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      error "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="arm64" ;;
        *)              error "Unsupported architecture: $arch" ;;
    esac

    # Linux uses aarch64 naming, macOS uses arm64
    if [[ "$os" == "linux" && "$arch" == "arm64" ]]; then
        arch="aarch64"
    fi

    echo "${os}-${arch}"
}

# Get latest version from GitHub
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name":' \
        | sed -E 's/.*"v([^"]+)".*/\1/'
}

main() {
    info "Gralph Installer"
    echo

    # Check for required tools
    command -v curl >/dev/null 2>&1 || error "curl is required but not installed"
    command -v tar >/dev/null 2>&1 || error "tar is required but not installed"

    # Detect platform
    local platform
    platform="$(detect_platform)"
    info "Detected platform: $platform"

    # Get version
    if [[ "$VERSION" == "latest" ]]; then
        info "Fetching latest version..."
        VERSION="$(get_latest_version)"
    fi
    info "Installing version: $VERSION"

    # Download
    local url="https://github.com/${REPO}/releases/download/v${VERSION}/gralph-${VERSION}-${platform}.tar.gz"
    local tmp_dir=""
    cleanup() {
        if [[ -n "${tmp_dir:-}" && -d "${tmp_dir}" ]]; then
            rm -rf "${tmp_dir}"
        fi
    }
    trap cleanup EXIT
    tmp_dir="$(mktemp -d)"

    info "Downloading from $url..."
    curl -fsSL "$url" -o "$tmp_dir/gralph.tar.gz" || error "Download failed. Check if version $VERSION exists for platform $platform"

    # Extract
    info "Extracting..."
    tar -xzf "$tmp_dir/gralph.tar.gz" -C "$tmp_dir"

    # Install
    local binary="$tmp_dir/gralph-${VERSION}/gralph"
    if [[ ! -f "$binary" ]]; then
        error "Binary not found in archive"
    fi

    info "Installing to $INSTALL_DIR..."
    # Create install directory if it doesn't exist
    if [[ ! -d "$INSTALL_DIR" ]]; then
        mkdir -p "$INSTALL_DIR"
    fi
    if [[ -w "$INSTALL_DIR" ]]; then
        cp "$binary" "$INSTALL_DIR/gralph"
        chmod +x "$INSTALL_DIR/gralph"
    else
        warn "Need sudo to install to $INSTALL_DIR"
        sudo cp "$binary" "$INSTALL_DIR/gralph"
        sudo chmod +x "$INSTALL_DIR/gralph"
    fi

    # Verify
    local installed_bin="$INSTALL_DIR/gralph"
    if [[ -x "$installed_bin" ]]; then
        local installed_version
        installed_version="$($installed_bin --version 2>/dev/null || echo "$VERSION")"
        echo
        info "Successfully installed gralph ${installed_version}"
        if command -v gralph >/dev/null 2>&1; then
            local resolved_path
            resolved_path="$(command -v gralph)"
            if [[ "$resolved_path" != "$installed_bin" ]]; then
                warn "PATH resolves gralph to $resolved_path"
                warn "Run $installed_bin or update PATH to prefer $INSTALL_DIR"
            fi
        else
            warn "Installed to $installed_bin but it's not in PATH"
            ensure_path "$INSTALL_DIR"
        fi
        info "Run 'gralph --help' to get started"
    else
        error "Installed binary not found at $installed_bin"
    fi
}

main "$@"
