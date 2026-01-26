#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

require_command() {
    local command_name="$1"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "Missing required command: $command_name" >&2
        return 1
    fi
}

check_output() {
    local pattern="$1"
    shift
    local output
    output=$("$@" 2>&1) || true
    if ! echo "$output" | grep -q "$pattern"; then
        echo "Pattern '$pattern' not found in output of: $*" >&2
        echo "Output was: $output" >&2
        return 1
    fi
}

require_command bash
require_command jq
require_command tmux

bash_major_version="$(bash -c 'echo ${BASH_VERSINFO[0]}')"
if [[ "$bash_major_version" -lt 4 ]]; then
    echo "bash 4+ required (found $bash_major_version)" >&2
    exit 1
fi

# Basic commands
check_output '^gralph v' ./bin/gralph version
./bin/gralph help >/dev/null
./bin/gralph status >/dev/null

# Backends command
check_output 'claude' ./bin/gralph backends

# Config command
./bin/gralph config list >/dev/null

# Help flags work on subcommands
./bin/gralph start --help >/dev/null

# Parse checks for new flags (validation only, no execution)
help_output=$(./bin/gralph help 2>&1)

# --no-tmux flag recognized
echo "$help_output" | grep -q '\-\-no-tmux'
# --backend flag recognized
echo "$help_output" | grep -q '\-\-backend'
# --webhook flag recognized
echo "$help_output" | grep -q '\-\-webhook'
# --variant flag recognized
echo "$help_output" | grep -q '\-\-variant'
# --prompt-template flag recognized
echo "$help_output" | grep -q '\-\-prompt-template'
# Server command available
echo "$help_output" | grep -q 'server'

echo "macOS smoke test passed."
