#!/bin/bash
# common.sh - Common backend interface and utilities
#
# This file defines the interface that all backends must implement
# and provides utilities for backend management.

# Currently loaded backend
GRALPH_CURRENT_BACKEND=""

# Backend interface functions that must be implemented:
#
# backend_name()
#   Returns the name of the backend (e.g., "claude", "opencode")
#
# backend_check_installed()
#   Returns 0 if the backend CLI is installed, 1 otherwise
#
# backend_get_install_hint()
#   Prints installation instructions for the backend
#
# backend_run_iteration()
#   Arguments:
#     $1 - prompt (required)
#     $2 - model override (optional)
#     $3 - output file for raw response (required)
#   Returns:
#     0 on success, 1 on failure
#   Side effects:
#     Writes raw response to output file
#     Streams human-readable text to stdout
#
# backend_parse_text()
#   Arguments:
#     $1 - raw response file
#   Outputs:
#     Extracted text content from the response
#
# backend_get_models()
#   Outputs:
#     Space-separated list of supported model names

# get_backend_dir() - Get the directory containing backend implementations
get_backend_dir() {
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    echo "$script_dir"
}

# list_available_backends() - List all available backend implementations
list_available_backends() {
    local backend_dir
    backend_dir=$(get_backend_dir)

    local backends=()
    for file in "$backend_dir"/*.sh; do
        local basename
        basename=$(basename "$file" .sh)
        # Skip common.sh
        if [[ "$basename" != "common" ]]; then
            backends+=("$basename")
        fi
    done

    echo "${backends[*]}"
}

# load_backend() - Load a specific backend implementation
#
# Arguments:
#   $1 - backend name (required)
#
# Returns:
#   0 on success, 1 on failure
load_backend() {
    local backend_name="$1"

    if [[ -z "$backend_name" ]]; then
        echo "Error: backend name is required" >&2
        return 1
    fi

    local backend_dir
    backend_dir=$(get_backend_dir)
    local backend_file="$backend_dir/${backend_name}.sh"

    if [[ ! -f "$backend_file" ]]; then
        echo "Error: Backend '$backend_name' not found at $backend_file" >&2
        echo "Available backends: $(list_available_backends)" >&2
        return 1
    fi

    # Source the backend implementation
    # shellcheck source=/dev/null
    source "$backend_file"

    GRALPH_CURRENT_BACKEND="$backend_name"
    return 0
}

# validate_backend() - Check if current backend has all required functions
#
# Returns:
#   0 if valid, 1 if missing required functions
validate_backend() {
    local required_funcs=(
        "backend_name"
        "backend_check_installed"
        "backend_get_install_hint"
        "backend_run_iteration"
        "backend_parse_text"
        "backend_get_models"
    )

    local missing=()
    for func in "${required_funcs[@]}"; do
        if ! declare -f "$func" > /dev/null 2>&1; then
            missing+=("$func")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        echo "Error: Backend '$GRALPH_CURRENT_BACKEND' is missing required functions: ${missing[*]}" >&2
        return 1
    fi

    return 0
}

# get_default_backend() - Get the default backend name
get_default_backend() {
    echo "claude"
}
