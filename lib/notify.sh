#!/bin/bash
# notify.sh - Notifications

# Get the directory where this script is located
NOTIFY_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source utils if available
if [[ -f "$NOTIFY_SCRIPT_DIR/utils.sh" ]]; then
  source "$NOTIFY_SCRIPT_DIR/utils.sh"
fi

# send_webhook - POST JSON payload to a webhook URL
# Arguments:
#   $1 - webhook URL
#   $2 - JSON payload
# Returns:
#   0 on success, 1 on failure
send_webhook() {
  local url="$1"
  local payload="$2"
  local timeout="${3:-30}"

  if [[ -z "$url" ]]; then
    echo "Error: No webhook URL provided" >&2
    return 1
  fi

  if [[ -z "$payload" ]]; then
    echo "Error: No payload provided" >&2
    return 1
  fi

  # Check if curl is available
  if ! command -v curl &>/dev/null; then
    echo "Error: curl is required for webhook notifications" >&2
    return 1
  fi

  # Send the webhook request
  local response
  local http_code

  # Use curl to POST the JSON payload
  http_code=$(curl -s -o /dev/null -w "%{http_code}" \
    --max-time "$timeout" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$payload" \
    "$url" 2>/dev/null)

  local curl_exit=$?

  if [[ $curl_exit -ne 0 ]]; then
    echo "Error: curl request failed with exit code $curl_exit" >&2
    return 1
  fi

  # Check HTTP response code (2xx is success)
  if [[ "$http_code" =~ ^2[0-9][0-9]$ ]]; then
    return 0
  else
    echo "Error: Webhook returned HTTP $http_code" >&2
    return 1
  fi
}

# notify_complete - Format and send a completion notification
# Arguments:
#   $1 - session name
#   $2 - webhook URL
#   $3 - project directory (optional)
#   $4 - iterations completed (optional)
#   $5 - total duration in seconds (optional)
# Returns:
#   0 on success, 1 on failure
notify_complete() {
  local session_name="$1"
  local webhook_url="$2"
  local project_dir="${3:-unknown}"
  local iterations="${4:-unknown}"
  local duration_secs="${5:-}"

  if [[ -z "$session_name" ]]; then
    echo "Error: No session name provided" >&2
    return 1
  fi

  if [[ -z "$webhook_url" ]]; then
    echo "Error: No webhook URL provided" >&2
    return 1
  fi

  # Format duration as human-readable string
  local duration_str="unknown"
  if [[ -n "$duration_secs" && "$duration_secs" =~ ^[0-9]+$ ]]; then
    local hours=$((duration_secs / 3600))
    local mins=$(((duration_secs % 3600) / 60))
    local secs=$((duration_secs % 60))
    if [[ $hours -gt 0 ]]; then
      duration_str="${hours}h ${mins}m ${secs}s"
    elif [[ $mins -gt 0 ]]; then
      duration_str="${mins}m ${secs}s"
    else
      duration_str="${secs}s"
    fi
  fi

  # Get current timestamp in ISO 8601 format
  local timestamp
  timestamp=$(date -Iseconds 2>/dev/null || date "+%Y-%m-%dT%H:%M:%S%z")

  # Escape strings for JSON
  local escaped_name
  local escaped_dir
  escaped_name=$(echo "$session_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_dir=$(echo "$project_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')

  # Build JSON payload
  local payload
  payload=$(cat <<EOF
{
  "event": "complete",
  "status": "success",
  "session": "${escaped_name}",
  "project": "${escaped_dir}",
  "iterations": "${iterations}",
  "duration": "${duration_str}",
  "timestamp": "${timestamp}",
  "message": "Ralph loop '${escaped_name}' completed successfully after ${iterations} iterations (${duration_str})"
}
EOF
)

  # Send the webhook
  send_webhook "$webhook_url" "$payload"
}
