#!/bin/bash
# claude.sh - Claude Code backend implementation
#
# This backend uses the official Claude Code CLI from Anthropic.
# Install: npm install -g @anthropic-ai/claude-code

# jq filters for parsing Claude JSON stream output
_CLAUDE_JQ_STREAM_TEXT='select(.type == "assistant").message.content[]? | select(.type == "text").text // empty | gsub("\n"; "\r\n") | . + "\r\n\n"'
_CLAUDE_JQ_FINAL_RESULT='select(.type == "result").result // empty'

# backend_name() - Returns the backend name
backend_name() {
    echo "claude"
}

# backend_check_installed() - Check if claude CLI is installed
#
# Returns:
#   0 if installed, 1 if not
backend_check_installed() {
    command -v claude &> /dev/null
}

# backend_get_install_hint() - Get installation instructions
backend_get_install_hint() {
    echo "npm install -g @anthropic-ai/claude-code"
}

# backend_run_iteration() - Execute a single Claude Code iteration
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
#   - Writes raw JSON response to output file
#   - Streams human-readable text to stdout
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

    # Build claude command arguments
    local claude_args=(
        --dangerously-skip-permissions
        --verbose
        --print
        --output-format stream-json
    )

    # Add model override if specified
    if [[ -n "$model" ]]; then
        claude_args+=(--model "$model")
    fi

    # Add the prompt
    claude_args+=(-p "$prompt")

    # Execute Claude and capture/stream output
    # - Capture full JSON to output file
    # - Stream human-readable text to stdout
    IS_SANDBOX=1 claude "${claude_args[@]}" 2>&1 \
        | grep --line-buffered '^{' \
        | tee "$output_file" \
        | jq --unbuffered -rj "$_CLAUDE_JQ_STREAM_TEXT"

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

    # Extract final result from JSON stream
    jq -r "$_CLAUDE_JQ_FINAL_RESULT" "$response_file" 2>/dev/null || cat "$response_file"
}

# backend_get_models() - Get list of supported models
backend_get_models() {
    echo "claude-opus-4-5"
}

# backend_get_default_model() - Get the default model for this backend
backend_get_default_model() {
    echo "claude-opus-4-5"
}
