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

require_command bash
require_command jq
require_command tmux

bash_major_version="$(bash -c 'echo ${BASH_VERSINFO[0]}')"
if [[ "$bash_major_version" -lt 4 ]]; then
    echo "bash 4+ required (found $bash_major_version)" >&2
    exit 1
fi

./bin/rloop version | grep -q '^rloop v'
./bin/rloop help >/dev/null
./bin/rloop status >/dev/null

echo "macOS smoke test passed."
