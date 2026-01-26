#!/bin/bash
# codex.sh - Codex CLI backend implementation
#
# This backend uses the OpenAI Codex CLI.
# Install: npm install -g @openai/codex
# See: https://developers.openai.com/codex/cli/

# backend_name() - Returns the backend name
backend_name() {
    echo "codex"
}

# backend_check_installed() - Check if codex CLI is installed
#
# Returns:
#   0 if installed, 1 if not
backend_check_installed() {
    command -v codex &> /dev/null
}

# backend_get_install_hint() - Get installation instructions
backend_get_install_hint() {
    echo "npm install -g @openai/codex"
}

# backend_run_iteration() - Execute a single Codex CLI iteration
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

    # Build codex command arguments
    # Use --quiet flag for non-interactive execution (suppresses interactive prompts)
    # Use --auto-approve to automatically approve all actions
    local codex_args=(
        --quiet
        --auto-approve
    )

    # Add model override if specified
    if [[ -n "$model" ]]; then
        codex_args+=(--model "$model")
    fi

    # Add the prompt
    codex_args+=("$prompt")

    # Execute Codex and capture/stream output
    # Codex CLI in quiet mode outputs text directly
    if [[ -n "$raw_output_file" ]]; then
        : > "$raw_output_file"
        codex "${codex_args[@]}" 2>&1 \
            | tee "$raw_output_file" \
            | tee "$output_file"
    else
        codex "${codex_args[@]}" 2>&1 \
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

    # Codex CLI in quiet mode outputs plain text, so just return the file contents
    cat "$response_file"
}

# backend_get_models() - Get list of supported models
#
# Codex CLI supports various OpenAI models:
# - o3
# - o4-mini
# - gpt-4.1
backend_get_models() {
    echo "o3 o4-mini gpt-4.1"
}

# backend_get_default_model() - Get the default model for this backend
backend_get_default_model() {
    echo "o3"
}
