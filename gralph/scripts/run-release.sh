#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
GRALPH_BIN="$ROOT_DIR/bin/gralph"

STAGE_P_PRD="gralph/examples/PRD-Stage-P-Example.md"
STAGE_A_PRD="gralph/examples/PRD-Stage-A-Example.md"

if [[ ! -f "$ROOT_DIR/$STAGE_A_PRD" ]]; then
    STAGE_A_PRD="examples/PRD-Stage-A-Example.md"
fi

if [[ ! -x "$GRALPH_BIN" ]]; then
    echo "Error: gralph binary not found at $GRALPH_BIN" >&2
    exit 1
fi

"$GRALPH_BIN" start "$ROOT_DIR" --task-file "$STAGE_P_PRD" --no-tmux
"$GRALPH_BIN" start "$ROOT_DIR" --task-file "$STAGE_A_PRD" --no-tmux
