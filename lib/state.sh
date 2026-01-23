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

# get_session() - Read session by name from state file
# Arguments:
#   $1 - Session name to retrieve
# Outputs:
#   Prints JSON object of session to stdout if found
# Returns:
#   0 if session found, 1 if not found or error
get_session() {
    local name="$1"

    if [[ -z "$name" ]]; then
        echo "Error: Session name is required" >&2
        return 1
    fi

    # Ensure state is initialized
    if ! init_state; then
        return 1
    fi

    # Read session from state file using jq
    local session
    session=$(jq -e ".sessions[\"$name\"]" "$RLOOP_STATE_FILE" 2>/dev/null)
    local exit_code=$?

    if [[ $exit_code -ne 0 ]] || [[ "$session" == "null" ]]; then
        return 1
    fi

    echo "$session"
    return 0
}

# set_session() - Upsert session (update if exists, insert if not)
# Arguments:
#   $1 - Session name (required)
#   Remaining args are key=value pairs for session properties:
#     dir, task_file, pid, tmux_session, started_at, iteration,
#     max_iterations, status, last_task_count, completion_marker, log_file
# Example:
#   set_session "myapp" dir="/path/to/project" status="running" iteration=5
# Returns:
#   0 on success, 1 on failure
set_session() {
    local name="$1"
    shift

    if [[ -z "$name" ]]; then
        echo "Error: Session name is required" >&2
        return 1
    fi

    # Ensure state is initialized
    if ! init_state; then
        return 1
    fi

    # Build the session object from arguments
    local session_json
    local existing_session

    # Get existing session if it exists, or create empty object
    existing_session=$(jq -r ".sessions[\"$name\"] // {}" "$RLOOP_STATE_FILE" 2>/dev/null)
    if [[ -z "$existing_session" ]] || [[ "$existing_session" == "null" ]]; then
        existing_session="{}"
    fi

    # Always set the name field
    session_json=$(echo "$existing_session" | jq --arg name "$name" '. + {name: $name}')

    # Parse key=value arguments and add to session JSON
    for arg in "$@"; do
        if [[ "$arg" =~ ^([a-z_]+)=(.*)$ ]]; then
            local key="${BASH_REMATCH[1]}"
            local value="${BASH_REMATCH[2]}"

            # Determine if value should be a number or string
            if [[ "$value" =~ ^[0-9]+$ ]]; then
                # Integer value
                session_json=$(echo "$session_json" | jq --arg k "$key" --argjson v "$value" '. + {($k): $v}')
            elif [[ "$value" == "true" ]] || [[ "$value" == "false" ]]; then
                # Boolean value
                session_json=$(echo "$session_json" | jq --arg k "$key" --argjson v "$value" '. + {($k): $v}')
            else
                # String value
                session_json=$(echo "$session_json" | jq --arg k "$key" --arg v "$value" '. + {($k): $v}')
            fi
        else
            echo "Warning: Ignoring invalid argument format: $arg (expected key=value)" >&2
        fi
    done

    # Update the state file with the new/updated session
    local new_state
    new_state=$(jq --arg name "$name" --argjson session "$session_json" \
        '.sessions[$name] = $session' "$RLOOP_STATE_FILE" 2>/dev/null)

    if [[ $? -ne 0 ]] || [[ -z "$new_state" ]]; then
        echo "Error: Failed to construct new state JSON" >&2
        return 1
    fi

    # Write the updated state back to the file
    if ! echo "$new_state" > "$RLOOP_STATE_FILE"; then
        echo "Error: Failed to write state file" >&2
        return 1
    fi

    return 0
}
