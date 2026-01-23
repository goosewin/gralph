#!/bin/bash
# opencode.sh - OpenCode backend implementation
#
# This backend uses the OpenCode CLI, an open-source AI coding assistant.
# Install: See https://opencode.ai/docs/cli/
#
# OpenCode supports multiple AI providers and models in the format "provider/model"

# backend_name() - Returns the backend name
backend_name() {
    echo "opencode"
}

# backend_check_installed() - Check if opencode CLI is installed
#
# Returns:
#   0 if installed, 1 if not
backend_check_installed() {
    command -v opencode &> /dev/null
}

# backend_get_install_hint() - Get installation instructions
backend_get_install_hint() {
    echo "See https://opencode.ai/docs/cli/ for installation instructions"
}

# backend_run_iteration() - Execute a single OpenCode iteration
#
# Arguments:
#   $1 - prompt (required)
#   $2 - model override (optional, format: provider/model)
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

    if [[ -z "$prompt" ]]; then
        echo "Error: prompt is required" >&2
        return 1
    fi

    if [[ -z "$output_file" ]]; then
        echo "Error: output_file is required" >&2
        return 1
    fi

    # Build opencode command arguments
    local opencode_args=()

    # Add model override if specified (format: provider/model)
    if [[ -n "$model" ]]; then
        opencode_args+=(--model "$model")
    fi

    # OpenCode uses 'run' subcommand for non-interactive execution
    # The output is streamed directly to stdout

    # Execute OpenCode and capture/stream output
    # OpenCode 'run' command outputs text directly, not JSON
    opencode run "${opencode_args[@]}" "$prompt" 2>&1 \
        | tee "$output_file"

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

    # OpenCode outputs plain text, so just return the file contents
    cat "$response_file"
}

# backend_get_models() - Get list of supported models
#
# OpenCode supports multiple providers. Common models include:
# - anthropic/claude-3-5-sonnet-latest
# - openai/gpt-4o
# - google/gemini-2.0-flash
# - deepseek/deepseek-chat
backend_get_models() {
    echo "anthropic/claude-sonnet-4-20250514 anthropic/claude-opus-4-20250514 openai/gpt-4o google/gemini-2.0-flash deepseek/deepseek-chat"
}

# backend_get_default_model() - Get the default model for this backend
backend_get_default_model() {
    echo "anthropic/claude-sonnet-4-20250514"
}
