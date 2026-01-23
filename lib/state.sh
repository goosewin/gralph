#!/bin/bash
# state.sh - State management for rloop sessions

# State file location
RLOOP_STATE_DIR="${RLOOP_STATE_DIR:-$HOME/.config/rloop}"
RLOOP_STATE_FILE="${RLOOP_STATE_FILE:-$RLOOP_STATE_DIR/state.json}"

# init_state() - Create state file and directory if missing
# Creates ~/.config/rloop/ directory and initializes empty state.json
# Returns 0 on success, 1 on failure
init_state() {
    # Create config directory if it doesn't exist
    if [[ ! -d "$RLOOP_STATE_DIR" ]]; then
        if ! mkdir -p "$RLOOP_STATE_DIR"; then
            echo "Error: Failed to create state directory: $RLOOP_STATE_DIR" >&2
            return 1
        fi
    fi

    # Create state file with empty sessions object if it doesn't exist
    if [[ ! -f "$RLOOP_STATE_FILE" ]]; then
        if ! echo '{"sessions":{}}' > "$RLOOP_STATE_FILE"; then
            echo "Error: Failed to create state file: $RLOOP_STATE_FILE" >&2
            return 1
        fi
    fi

    # Validate state file is valid JSON
    if ! jq empty "$RLOOP_STATE_FILE" 2>/dev/null; then
        echo "Warning: State file is invalid JSON, reinitializing..." >&2
        if ! echo '{"sessions":{}}' > "$RLOOP_STATE_FILE"; then
            echo "Error: Failed to reinitialize state file" >&2
            return 1
        fi
    fi

    return 0
}
