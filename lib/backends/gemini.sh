#!/bin/bash
# gemini.sh - Gemini CLI backend implementation
#
# This backend uses the Gemini CLI from Google.
# Install: npm install -g @google/gemini-cli
# See: https://geminicli.com/docs/

# backend_name() - Returns the backend name
backend_name() {
    echo "gemini"
}

# backend_check_installed() - Check if gemini CLI is installed
#
# Returns:
#   0 if installed, 1 if not
backend_check_installed() {
    command -v gemini &> /dev/null
}

# backend_get_install_hint() - Get installation instructions
backend_get_install_hint() {
    echo "npm install -g @google/gemini-cli"
}

# backend_run_iteration() - Execute a single Gemini CLI iteration
#
# Arguments:
#   $1 - prompt (required)
#   $2 - model override (optional)
#   $3 - output file for raw response (required)
#
# Returns:
#   0 on success, non-zero on failure
#
# Side effects:
#   - Writes raw response to output file
#   - Streams text to stdout
backend_run_iteration() {
    local prompt="$1"
    local model="$2"
    local output_file="$3"
    local raw_output_file="${GRALPH_RAW_OUTPUT_FILE:-}"

    if [[ -z "$prompt" ]]; then
        echo "Error: prompt is required" >&2
        return 1
    fi

    if [[ -z "$output_file" ]]; then
        echo "Error: output_file is required" >&2
        return 1
    fi

    # Build gemini command arguments
    # Use --headless flag for non-interactive execution
    local gemini_args=(
        --headless
    )

    # Add model override if specified
    if [[ -n "$model" ]]; then
        gemini_args+=(--model "$model")
    fi

    # Add the prompt
    gemini_args+=("$prompt")

    # Execute Gemini and capture/stream output
    # Gemini CLI in headless mode outputs text directly
    if [[ -n "$raw_output_file" ]]; then
        : > "$raw_output_file"
        gemini "${gemini_args[@]}" 2>&1 \
            | tee "$raw_output_file" \
            | tee "$output_file"
    else
        gemini "${gemini_args[@]}" 2>&1 \
            | tee "$output_file"
    fi

    return "${PIPESTATUS[0]}"
}

# backend_parse_text() - Extract text content from raw response
#
# Arguments:
#   $1 - raw response file
#
# Outputs:
#   Extracted text content
backend_parse_text() {
    local response_file="$1"

    if [[ ! -f "$response_file" ]]; then
        return 1
    fi

    # Gemini CLI in headless mode outputs plain text, so just return the file contents
    cat "$response_file"
}

# backend_get_models() - Get list of supported models
#
# Gemini CLI supports Gemini models:
# - gemini-1.5-pro
backend_get_models() {
    echo "gemini-1.5-pro"
}

# backend_get_default_model() - Get the default model for this backend
backend_get_default_model() {
    echo "gemini-1.5-pro"
}
