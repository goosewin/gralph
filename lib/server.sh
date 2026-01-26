#!/bin/bash
# server.sh - Simple HTTP status server for gralph
# Uses netcat (nc) or socat for a minimal HTTP server

# Default server configuration
GRALPH_SERVER_PORT="${GRALPH_SERVER_PORT:-8080}"
GRALPH_SERVER_HOST="${GRALPH_SERVER_HOST:-127.0.0.1}"
GRALPH_SERVER_TOKEN="${GRALPH_SERVER_TOKEN:-}"
GRALPH_SERVER_PID_FILE="${GRALPH_STATE_DIR:-$HOME/.config/gralph}/server.pid"
GRALPH_SERVER_MAX_BODY_BYTES="${GRALPH_SERVER_MAX_BODY_BYTES:-4096}"
GRALPH_SERVER_OPEN="${GRALPH_SERVER_OPEN:-false}"

# Source state.sh if not already sourced
if ! declare -f list_sessions &>/dev/null; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    source "$SCRIPT_DIR/state.sh"
fi

# _check_auth() - Verify Bearer token authentication
# Arguments:
#   $1 - The Authorization header value
# Returns:
#   0 if authenticated (or no token configured), 1 if unauthorized
_check_auth() {
    local auth_header="$1"

    # If no token is configured, allow all requests
    if [[ -z "$GRALPH_SERVER_TOKEN" ]]; then
        return 0
    fi

    # Extract token from "Bearer <token>" format
    local provided_token
    if [[ "$auth_header" =~ ^Bearer[[:space:]]+(.+)$ ]]; then
        provided_token="${BASH_REMATCH[1]}"
    else
        return 1
    fi

    # Compare tokens (constant-time comparison would be better but this is acceptable)
    if [[ "$provided_token" == "$GRALPH_SERVER_TOKEN" ]]; then
        return 0
    fi

    return 1
}

# _send_response() - Send HTTP response
# Arguments:
#   $1 - HTTP status code (e.g., 200, 401, 404)
#   $2 - Status text (e.g., "OK", "Unauthorized")
#   $3 - Content type (e.g., "application/json")
#   $4 - Response body
#   $5 - Allowed CORS origin (optional, empty to omit)
_send_response() {
    local status_code="$1"
    local status_text="$2"
    local content_type="$3"
    local body="$4"
    local cors_origin="${5:-}"
    local body_length=${#body}

    printf "HTTP/1.1 %s %s\r\n" "$status_code" "$status_text"
    printf "Content-Type: %s\r\n" "$content_type"
    printf "Content-Length: %d\r\n" "$body_length"
    # CORS headers for browser access (only when allowed)
    if [[ -n "$cors_origin" ]]; then
        printf "Access-Control-Allow-Origin: %s\r\n" "$cors_origin"
        if [[ "$cors_origin" != "*" ]]; then
            printf "Vary: Origin\r\n"
        fi
        printf "Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n"
        printf "Access-Control-Allow-Headers: Authorization, Content-Type\r\n"
        printf "Access-Control-Expose-Headers: Content-Length, Content-Type\r\n"
        printf "Access-Control-Max-Age: 86400\r\n"
    fi
    printf "Connection: close\r\n"
    printf "\r\n"
    printf "%s" "$body"
}

# _send_json() - Send JSON response with 200 OK
# Arguments:
#   $1 - JSON body
_send_json() {
    _send_response 200 "OK" "application/json" "$1" "${2:-}"
}

# _send_error() - Send JSON error response
# Arguments:
#   $1 - HTTP status code
#   $2 - Status text
#   $3 - Error message
_send_error() {
    local status_code="$1"
    local status_text="$2"
    local message="$3"
    local cors_origin="${4:-}"
    local body
    body=$(jq -n --arg msg "$message" '{"error": $msg}')
    _send_response "$status_code" "$status_text" "application/json" "$body" "$cors_origin"
}

# _resolve_cors_origin() - Resolve allowed CORS origin
# Arguments:
#   $1 - Origin header value
# Returns:
#   Prints allowed origin (or empty if not allowed)
_resolve_cors_origin() {
    local origin="$1"
    local host="$GRALPH_SERVER_HOST"

    if [[ -z "$origin" ]]; then
        return
    fi

    if [[ "$GRALPH_SERVER_OPEN" == "true" ]]; then
        echo "*"
        return
    fi

    case "$origin" in
        http://localhost|http://127.0.0.1|http://[::1])
            echo "$origin"
            return
            ;;
    esac

    if [[ -n "$host" ]] && [[ "$host" != "0.0.0.0" ]] && [[ "$host" != "::" ]]; then
        if [[ "$origin" == "http://$host" ]]; then
            echo "$origin"
            return
        fi
    fi
}

# _get_all_sessions_json() - Get JSON array of all sessions with current status
# Returns JSON to stdout
_get_all_sessions_json() {
    # Ensure state is initialized
    init_state >/dev/null 2>&1 || true

    # Get all sessions
    local sessions
    sessions=$(list_sessions 2>/dev/null)

    if [[ -z "$sessions" ]] || [[ "$sessions" == "null" ]]; then
        echo '{"sessions":[]}'
        return
    fi

    # Enrich each session with current task count
    local enriched_sessions="[]"
    local session_count
    session_count=$(echo "$sessions" | jq -r 'length' 2>/dev/null || echo "0")

    for ((i=0; i<session_count; i++)); do
        local session
        session=$(echo "$sessions" | jq ".[$i]" 2>/dev/null)

        local dir task_file
        dir=$(echo "$session" | jq -r '.dir // ""' 2>/dev/null)
        task_file=$(echo "$session" | jq -r '.task_file // "PRD.md"' 2>/dev/null)

        # Get current remaining task count
        local current_remaining=0
        if [[ -d "$dir" ]] && [[ -f "$dir/$task_file" ]]; then
            current_remaining=$(grep -cE '^\s*- \[ \]' "$dir/$task_file" 2>/dev/null || echo "0")
        fi

        # Check if PID is alive for running sessions
        local status pid is_alive
        status=$(echo "$session" | jq -r '.status // "unknown"' 2>/dev/null)
        pid=$(echo "$session" | jq -r '.pid // ""' 2>/dev/null)
        is_alive=false

        if [[ "$status" == "running" ]] && [[ -n "$pid" ]] && [[ "$pid" != "null" ]]; then
            if kill -0 "$pid" 2>/dev/null; then
                is_alive=true
            else
                # Mark as stale if PID is dead
                status="stale"
            fi
        fi

        # Add current_remaining and is_alive to session
        session=$(echo "$session" | jq \
            --argjson remaining "$current_remaining" \
            --argjson alive "$is_alive" \
            --arg status "$status" \
            '. + {current_remaining: $remaining, is_alive: $alive, status: $status}' 2>/dev/null)

        enriched_sessions=$(echo "$enriched_sessions" | jq --argjson s "$session" '. + [$s]' 2>/dev/null)
    done

    echo "$enriched_sessions" | jq '{sessions: .}'
}

# _get_session_json() - Get JSON for a specific session
# Arguments:
#   $1 - Session name
# Returns JSON to stdout
_get_session_json() {
    local name="$1"

    # Ensure state is initialized
    init_state >/dev/null 2>&1 || true

    # Get session
    local session
    if ! session=$(get_session "$name" 2>/dev/null); then
        echo '{"error":"Session not found"}'
        return 1
    fi

    local dir task_file
    dir=$(echo "$session" | jq -r '.dir // ""' 2>/dev/null)
    task_file=$(echo "$session" | jq -r '.task_file // "PRD.md"' 2>/dev/null)

    # Get current remaining task count
    local current_remaining=0
    if [[ -d "$dir" ]] && [[ -f "$dir/$task_file" ]]; then
        current_remaining=$(grep -cE '^\s*- \[ \]' "$dir/$task_file" 2>/dev/null || echo "0")
    fi

    # Check if PID is alive
    local status pid is_alive
    status=$(echo "$session" | jq -r '.status // "unknown"' 2>/dev/null)
    pid=$(echo "$session" | jq -r '.pid // ""' 2>/dev/null)
    is_alive=false

    if [[ "$status" == "running" ]] && [[ -n "$pid" ]] && [[ "$pid" != "null" ]]; then
        if kill -0 "$pid" 2>/dev/null; then
            is_alive=true
        else
            status="stale"
        fi
    fi

    # Add enriched fields
    echo "$session" | jq \
        --argjson remaining "$current_remaining" \
        --argjson alive "$is_alive" \
        --arg status "$status" \
        '. + {current_remaining: $remaining, is_alive: $alive, status: $status}'
}

# _stop_session() - Stop a specific session
# Arguments:
#   $1 - Session name
# Returns JSON response to stdout
_stop_session() {
    local name="$1"

    # Get session
    local session
    if ! session=$(get_session "$name" 2>/dev/null); then
        echo '{"error":"Session not found"}'
        return 1
    fi

    local tmux_session pid
    tmux_session=$(echo "$session" | jq -r '.tmux_session // ""' 2>/dev/null)
    pid=$(echo "$session" | jq -r '.pid // ""' 2>/dev/null)

    # Kill tmux session if it exists
    if [[ -n "$tmux_session" ]] && [[ "$tmux_session" != "null" ]]; then
        tmux kill-session -t "$tmux_session" 2>/dev/null || true
    elif [[ -n "$pid" ]] && [[ "$pid" != "null" ]]; then
        kill "$pid" 2>/dev/null || true
    fi

    # Update state
    set_session "$name" status="stopped" 2>/dev/null

    echo '{"success":true,"message":"Session stopped"}'
}

# _handle_request() - Parse and handle an HTTP request
# Reads request from stdin, writes response to stdout
# Endpoints:
#   GET  /            - Health check
#   GET  /status      - List all sessions
#   GET  /status/:id  - Get a single session
#   POST /stop/:id    - Stop a session
_handle_request() {
    local request_line
    local method path protocol
    local auth_header=""
    local content_length=0
    local body=""
    local origin_header=""
    local cors_origin=""

    # Read request line
    read -r request_line
    request_line=$(echo "$request_line" | tr -d '\r')

    # Parse request line
    read -r method path protocol <<< "$request_line"
    path="${path%%\?*}"

    # Read headers
    while IFS= read -r header_line; do
        header_line=$(echo "$header_line" | tr -d '\r')

        # Empty line signals end of headers
        if [[ -z "$header_line" ]]; then
            break
        fi

        # Parse Authorization header
        if [[ "$header_line" =~ ^[Aa]uthorization:[[:space:]]*(.+)$ ]]; then
            auth_header="${BASH_REMATCH[1]}"
        fi

        # Parse Origin header
        if [[ "$header_line" =~ ^[Oo]rigin:[[:space:]]*(.+)$ ]]; then
            origin_header="${BASH_REMATCH[1]}"
        fi

        # Parse Content-Length header
        if [[ "$header_line" =~ ^[Cc]ontent-[Ll]ength:[[:space:]]*([0-9]+)$ ]]; then
            content_length="${BASH_REMATCH[1]}"
        fi
    done

    # Resolve CORS origin after headers
    cors_origin=$(_resolve_cors_origin "$origin_header")

    # Read body if present (guard against oversized payloads)
    if [[ "$content_length" -gt 0 ]]; then
        if [[ "$content_length" -gt "$GRALPH_SERVER_MAX_BODY_BYTES" ]]; then
            _send_error 413 "Payload Too Large" "Request body too large" "$cors_origin"
            return
        fi
        read -n "$content_length" body
    fi

    # Handle CORS preflight
    if [[ "$method" == "OPTIONS" ]]; then
        _send_response 204 "No Content" "text/plain" "" "$cors_origin"
        return
    fi

    # Check authentication (skip for OPTIONS)
    if ! _check_auth "$auth_header"; then
        _send_error 401 "Unauthorized" "Invalid or missing Bearer token" "$cors_origin"
        return
    fi

    # Route the request
    case "$method $path" in
        "GET /")
            _send_json '{"status":"ok","service":"gralph-server"}' "$cors_origin"
            ;;
        "GET /status")
            local json
            json=$(_get_all_sessions_json)
            _send_json "$json" "$cors_origin"
            ;;
        "GET /status/"*)
            local session_name="${path#/status/}"
            # URL decode the session name (basic: replace %20 with space)
            session_name=$(echo "$session_name" | sed 's/%20/ /g')
            local json
            if json=$(_get_session_json "$session_name"); then
                _send_json "$json" "$cors_origin"
            else
                _send_error 404 "Not Found" "Session not found: $session_name" "$cors_origin"
            fi
            ;;
        "POST /stop/"*)
            local session_name="${path#/stop/}"
            session_name=$(echo "$session_name" | sed 's/%20/ /g')
            local json
            if json=$(_stop_session "$session_name"); then
                _send_json "$json" "$cors_origin"
            else
                _send_error 404 "Not Found" "Session not found: $session_name" "$cors_origin"
            fi
            ;;
        *)
            _send_error 404 "Not Found" "Unknown endpoint: $method $path" "$cors_origin"
            ;;
    esac
}

# _run_server_nc() - Run HTTP server using netcat
# Arguments:
#   $1 - Port number
#   $2 - Host/IP to bind to
_run_server_nc() {
    local port="$1"
    local host="$2"

    echo "Starting gralph status server on $host:$port using netcat..."
    echo "Endpoints:"
    echo "  GET  /status        - Get all sessions"
    echo "  GET  /status/:name  - Get specific session"
    echo "  POST /stop/:name    - Stop a session"
    if [[ -n "$GRALPH_SERVER_TOKEN" ]]; then
        echo "Authentication: Bearer token required"
    else
        echo "Authentication: None (use --token to enable)"
    fi
    echo ""
    echo "Press Ctrl+C to stop"
    echo ""

    # Create a named pipe for communication
    local fifo="/tmp/gralph-server-$$"
    mkfifo "$fifo"
    trap "rm -f $fifo" EXIT

    while true; do
        # Use netcat to listen for connections
        # -l: listen mode
        # -p: port (some nc versions use -l port directly)
        # Different versions of nc have different syntax, try both
        if nc -h 2>&1 | grep -q "GNU"; then
            # GNU netcat
            cat "$fifo" | nc -l -s "$host" -p "$port" | _handle_request > "$fifo"
        else
            # BSD/OpenBSD netcat (macOS)
            cat "$fifo" | nc -l "$host" "$port" | _handle_request > "$fifo"
        fi
    done
}

# _run_server_socat() - Run HTTP server using socat
# Arguments:
#   $1 - Port number
#   $2 - Host/IP to bind to
_run_server_socat() {
    local port="$1"
    local host="$2"

    echo "Starting gralph status server on $host:$port using socat..."
    echo "Endpoints:"
    echo "  GET  /status        - Get all sessions"
    echo "  GET  /status/:name  - Get specific session"
    echo "  POST /stop/:name    - Stop a session"
    if [[ -n "$GRALPH_SERVER_TOKEN" ]]; then
        echo "Authentication: Bearer token required"
    else
        echo "Authentication: None (use --token to enable)"
    fi
    echo ""
    echo "Press Ctrl+C to stop"
    echo ""

    # Get the path to this script for the handler
    local handler_script
    handler_script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/server.sh"

    # Build environment variables to pass to handler
    local env_vars
    printf -v env_vars "GRALPH_SERVER_TOKEN=%q" "$GRALPH_SERVER_TOKEN"
    if [[ -n "$GRALPH_STATE_DIR" ]]; then
        local state_dir_escaped
        printf -v state_dir_escaped "%q" "$GRALPH_STATE_DIR"
        env_vars="$env_vars GRALPH_STATE_DIR=$state_dir_escaped"
    fi
    env_vars="$env_vars GRALPH_SERVER_OPEN=$GRALPH_SERVER_OPEN"

    # socat forks a new process for each connection
    # We use EXEC to run a bash command that sources this file and handles the request
    socat TCP-LISTEN:"$port",bind="$host",reuseaddr,fork EXEC:"bash -c '$env_vars source $handler_script && _handle_request'"
}

# _is_localhost() - Check if a host is localhost
# Arguments:
#   $1 - Host/IP to check
# Returns:
#   0 if localhost, 1 otherwise
_is_localhost() {
    local host="$1"
    case "$host" in
        127.0.0.1|localhost|::1)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# start_server() - Start the HTTP status server
# Arguments:
#   $1 - Port number (optional, defaults to GRALPH_SERVER_PORT)
#   $2 - Host/IP to bind to (optional, defaults to GRALPH_SERVER_HOST)
#   $3 - Authentication token (optional, defaults to GRALPH_SERVER_TOKEN)
#   $4 - Open mode flag (optional, disables token requirement for non-localhost)
# Returns:
#   0 on success (when server stops), 1 on error
start_server() {
    local port="${1:-$GRALPH_SERVER_PORT}"
    local host="${2:-$GRALPH_SERVER_HOST}"
    local token="${3:-$GRALPH_SERVER_TOKEN}"
    local open="${4:-false}"

    # Security check: require token for non-localhost unless --open is specified
    if ! _is_localhost "$host" && [[ -z "$token" ]] && [[ "$open" != "true" ]]; then
        echo "Error: Token required when binding to non-localhost address ($host)" >&2
        echo "" >&2
        echo "For security, a token is required when exposing the server to the network." >&2
        echo "Either:" >&2
        echo "  1. Provide a token: --token <your-secret-token>" >&2
        echo "  2. Explicitly disable security: --open (not recommended)" >&2
        echo "  3. Bind to localhost only: --host 127.0.0.1" >&2
        return 1
    fi

    # Warn when using --open on non-localhost
    if ! _is_localhost "$host" && [[ "$open" == "true" ]] && [[ -z "$token" ]]; then
        echo "Warning: Server exposed without authentication (--open flag used)" >&2
        echo "Anyone with network access can view and control your sessions!" >&2
        echo "" >&2
    fi

    # Update globals
    GRALPH_SERVER_HOST="$host"
    GRALPH_SERVER_TOKEN="$token"
    GRALPH_SERVER_OPEN="$open"

    # Validate port
    if ! [[ "$port" =~ ^[0-9]+$ ]] || [[ "$port" -lt 1 ]] || [[ "$port" -gt 65535 ]]; then
        echo "Error: Invalid port number: $port" >&2
        return 1
    fi

    # Check if port is already in use
    if command -v lsof &>/dev/null; then
        if lsof -i :"$port" &>/dev/null; then
            echo "Error: Port $port is already in use" >&2
            return 1
        fi
    fi

    # Ensure state is initialized
    init_state || return 1

    # Store PID for tracking
    echo $$ > "$GRALPH_SERVER_PID_FILE"
    trap "rm -f '$GRALPH_SERVER_PID_FILE'" EXIT

    # Try socat first (better for concurrent connections), fall back to netcat
    if command -v socat &>/dev/null; then
        _run_server_socat "$port" "$host"
    elif command -v nc &>/dev/null; then
        _run_server_nc "$port" "$host"
    else
        echo "Error: Neither socat nor netcat (nc) is available" >&2
        echo "Install one of them:" >&2
        echo "  apt install socat    # Recommended" >&2
        echo "  apt install netcat" >&2
        return 1
    fi
}

# stop_server() - Stop the HTTP status server
# Returns:
#   0 on success, 1 if server not running
stop_server() {
    if [[ ! -f "$GRALPH_SERVER_PID_FILE" ]]; then
        echo "Server is not running (no PID file found)" >&2
        return 1
    fi

    local pid
    pid=$(cat "$GRALPH_SERVER_PID_FILE")

    if [[ -z "$pid" ]] || ! kill -0 "$pid" 2>/dev/null; then
        echo "Server is not running (stale PID file)" >&2
        rm -f "$GRALPH_SERVER_PID_FILE"
        return 1
    fi

    kill "$pid"
    rm -f "$GRALPH_SERVER_PID_FILE"
    echo "Server stopped (PID: $pid)"
    return 0
}
