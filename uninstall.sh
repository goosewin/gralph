#!/usr/bin/env bash
# Gralph uninstaller for Unix systems (Linux/macOS)
set -euo pipefail

INSTALL_DIR="${GRALPH_INSTALL_DIR:-/usr/local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

main() {
    info "Gralph Uninstaller"
    echo

    local binary="$INSTALL_DIR/gralph"

    if [[ ! -f "$binary" ]]; then
        # Try to find it in PATH
        binary="$(command -v gralph 2>/dev/null || true)"
        if [[ -z "$binary" ]]; then
            error "gralph not found in $INSTALL_DIR or PATH"
        fi
    fi

    info "Found gralph at: $binary"

    # Remove binary
    if [[ -w "$(dirname "$binary")" ]]; then
        rm -f "$binary"
    else
        warn "Need sudo to remove $binary"
        sudo rm -f "$binary"
    fi

    info "Successfully uninstalled gralph"

    # Check for config directory
    local config_dir="${HOME}/.config/gralph"
    if [[ -d "$config_dir" ]]; then
        warn "Config directory exists at $config_dir"
        read -p "Remove config directory? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$config_dir"
            info "Removed config directory"
        else
            info "Config directory preserved"
        fi
    fi
}

main "$@"
