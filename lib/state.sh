#!/bin/bash
# state.sh - State management for rloop sessions

# State file location
RLOOP_STATE_DIR="${RLOOP_STATE_DIR:-$HOME/.config/rloop}"
RLOOP_STATE_FILE="${RLOOP_STATE_FILE:-$RLOOP_STATE_DIR/state.json}"
RLOOP_LOCK_FILE="${RLOOP_LOCK_FILE:-$RLOOP_STATE_DIR/state.lock}"

# Lock timeout in seconds (default 10 seconds)
RLOOP_LOCK_TIMEOUT="${RLOOP_LOCK_TIMEOUT:-10}"

# File descriptor for lock file (using 200 to avoid conflicts with common FDs)
RLOOP_LOCK_FD=200

# _acquire_lock() - Acquire exclusive lock on state file
# Uses flock for POSIX-compliant file locking
# Arguments:
#   $1 - (optional) timeout in seconds (default: RLOOP_LOCK_TIMEOUT)
# Returns:
#   0 on success, 1 on failure (lock not acquired within timeout)
_acquire_lock() {
    local timeout="${1:-$RLOOP_LOCK_TIMEOUT}"

    # Ensure lock directory exists
    if [[ ! -d "$RLOOP_STATE_DIR" ]]; then
        mkdir -p "$RLOOP_STATE_DIR" 2>/dev/null || return 1
    fi

    # Open lock file on designated FD
    eval "exec $RLOOP_LOCK_FD>\"$RLOOP_LOCK_FILE\""

    # Try to acquire exclusive lock with timeout
    if ! flock -x -w "$timeout" "$RLOOP_LOCK_FD" 2>/dev/null; then
        echo "Error: Failed to acquire state lock within ${timeout}s" >&2
        return 1
    fi

    return 0
}

# _release_lock() - Release exclusive lock on state file
# Returns:
#   0 on success
_release_lock() {
    # Release the lock by closing the file descriptor
    eval "exec $RLOOP_LOCK_FD>&-" 2>/dev/null
    return 0
}

# _with_lock() - Execute a function while holding the state lock
# Arguments:
#   $1 - Function name to execute
#   $@ - Arguments to pass to the function
# Returns:
#   Return value of the executed function, or 1 if lock acquisition fails
_with_lock() {
    local func="$1"
    shift

    if ! _acquire_lock; then
        return 1
    fi

    # Execute the function and capture its return code
    local result
    "$func" "$@"
    result=$?

    _release_lock

    return $result
}

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

# _set_session_unlocked() - Internal: Upsert session without locking
# Called by set_session() which handles locking
_set_session_unlocked() {
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

# set_session() - Upsert session (update if exists, insert if not)
# Uses file locking to ensure concurrent access safety
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
    _with_lock _set_session_unlocked "$@"
}

# list_sessions() - Get all sessions from state file
# Arguments:
#   None
# Outputs:
#   Prints JSON array of all sessions to stdout
#   Each session object includes its name as a field
# Returns:
#   0 on success (even if no sessions), 1 on error
list_sessions() {
    # Ensure state is initialized
    if ! init_state; then
        return 1
    fi

    # Extract all sessions as a JSON array
    local sessions
    sessions=$(jq -r '[.sessions | to_entries[] | .value]' "$RLOOP_STATE_FILE" 2>/dev/null)
    local exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        echo "Error: Failed to read sessions from state file" >&2
        return 1
    fi

    # Handle empty sessions case
    if [[ -z "$sessions" ]] || [[ "$sessions" == "null" ]]; then
        echo "[]"
        return 0
    fi

    echo "$sessions"
    return 0
}

# _delete_session_unlocked() - Internal: Remove session without locking
# Called by delete_session() which handles locking
_delete_session_unlocked() {
    local name="$1"

    if [[ -z "$name" ]]; then
        echo "Error: Session name is required" >&2
        return 1
    fi

    # Ensure state is initialized
    if ! init_state; then
        return 1
    fi

    # Check if session exists
    if ! jq -e ".sessions[\"$name\"]" "$RLOOP_STATE_FILE" >/dev/null 2>&1; then
        echo "Error: Session '$name' not found" >&2
        return 1
    fi

    # Remove the session from the state file
    local new_state
    new_state=$(jq --arg name "$name" 'del(.sessions[$name])' "$RLOOP_STATE_FILE" 2>/dev/null)

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

# delete_session() - Remove session from state file
# Uses file locking to ensure concurrent access safety
# Arguments:
#   $1 - Session name to delete
# Returns:
#   0 on success, 1 if session not found or error
delete_session() {
    _with_lock _delete_session_unlocked "$@"
}

# _cleanup_stale_unlocked() - Internal: Cleanup stale sessions without locking
# Called by cleanup_stale() which handles locking
_cleanup_stale_unlocked() {
    local mode="${1:-mark}"  # "remove" or "mark"

    # Ensure state is initialized
    if ! init_state; then
        return 1
    fi

    # Get all sessions (read-only, no lock needed for list_sessions)
    local sessions
    sessions=$(jq -r '[.sessions | to_entries[] | .value]' "$RLOOP_STATE_FILE" 2>/dev/null)
    if [[ $? -ne 0 ]]; then
        echo "Error: Failed to read sessions from state file" >&2
        return 1
    fi

    # Handle empty sessions case
    if [[ -z "$sessions" ]] || [[ "$sessions" == "null" ]]; then
        return 0
    fi

    # Track cleaned up sessions
    local cleaned_count=0

    # Iterate through sessions looking for stale ones
    local session_names
    session_names=$(echo "$sessions" | jq -r '.[].name // empty' 2>/dev/null)

    for name in $session_names; do
        # Get session details directly from our already-read sessions
        local session
        session=$(echo "$sessions" | jq -r ".[] | select(.name == \"$name\")" 2>/dev/null)
        if [[ -z "$session" ]]; then
            continue
        fi

        local status pid
        status=$(echo "$session" | jq -r '.status // empty' 2>/dev/null)
        pid=$(echo "$session" | jq -r '.pid // empty' 2>/dev/null)

        # Only check sessions that are marked as "running"
        if [[ "$status" != "running" ]]; then
            continue
        fi

        # Skip if no PID recorded
        if [[ -z "$pid" ]] || [[ "$pid" == "null" ]]; then
            continue
        fi

        # Check if PID is still alive
        if kill -0 "$pid" 2>/dev/null; then
            # Process is still running, skip
            continue
        fi

        # PID is dead - session is stale
        echo "$name"
        cleaned_count=$((cleaned_count + 1))

        if [[ "$mode" == "remove" ]]; then
            # Remove the stale session entirely (use unlocked version since we hold the lock)
            _delete_session_unlocked "$name" >/dev/null 2>&1
        else
            # Mark the session as stale (use unlocked version since we hold the lock)
            _set_session_unlocked "$name" status="stale" >/dev/null 2>&1
        fi
    done

    return 0
}

# cleanup_stale() - Remove sessions with dead PIDs
# Uses file locking to ensure concurrent access safety
# Finds sessions marked as "running" whose PIDs no longer exist
# and either removes them or marks them as "stale"
# Arguments:
#   $1 - (optional) "remove" to delete stale sessions, otherwise marks as "stale"
# Outputs:
#   Prints names of cleaned up sessions to stdout (one per line)
# Returns:
#   0 on success (even if no stale sessions found), 1 on error
cleanup_stale() {
    _with_lock _cleanup_stale_unlocked "$@"
}
