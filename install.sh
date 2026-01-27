#!/usr/bin/env bash
# Gralph installer for Unix systems (Linux/macOS)
set -euo pipefail

REPO="goosewin/gralph"
INSTALL_DIR="${GRALPH_INSTALL_DIR:-/usr/local/bin}"
VERSION="${GRALPH_VERSION:-latest}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

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
    local tmp_dir
    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

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
    if [[ -w "$INSTALL_DIR" ]]; then
        cp "$binary" "$INSTALL_DIR/gralph"
        chmod +x "$INSTALL_DIR/gralph"
    else
        warn "Need sudo to install to $INSTALL_DIR"
        sudo cp "$binary" "$INSTALL_DIR/gralph"
        sudo chmod +x "$INSTALL_DIR/gralph"
    fi

    # Verify
    if command -v gralph >/dev/null 2>&1; then
        echo
        info "Successfully installed gralph $(gralph --version 2>/dev/null || echo "$VERSION")"
        info "Run 'gralph --help' to get started"
    else
        warn "Installed to $INSTALL_DIR/gralph but it's not in PATH"
        warn "Add $INSTALL_DIR to your PATH or move the binary"
    fi
}

main "$@"
