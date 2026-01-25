#!/bin/bash
# notify.sh - Notifications

# Get the directory where this script is located
NOTIFY_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source utils if available
if [[ -f "$NOTIFY_SCRIPT_DIR/utils.sh" ]]; then
  source "$NOTIFY_SCRIPT_DIR/utils.sh"
fi

# detect_webhook_type - Detect webhook platform from URL
# Arguments:
#   $1 - webhook URL
# Returns:
#   Prints: "discord", "slack", or "generic"
detect_webhook_type() {
  local url="$1"

  if [[ "$url" =~ discord\.com/api/webhooks || "$url" =~ discordapp\.com/api/webhooks ]]; then
    echo "discord"
  elif [[ "$url" =~ hooks\.slack\.com ]]; then
    echo "slack"
  else
    echo "generic"
  fi
}

# format_discord_complete - Format completion notification for Discord
# Arguments:
#   $1 - session name
#   $2 - project directory
#   $3 - iterations
#   $4 - duration string
#   $5 - timestamp
# Returns:
#   Prints JSON payload formatted for Discord webhooks
format_discord_complete() {
  local session_name="$1"
  local project_dir="$2"
  local iterations="$3"
  local duration_str="$4"
  local timestamp="$5"

  # Escape strings for JSON
  local escaped_name escaped_dir
  escaped_name=$(echo "$session_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_dir=$(echo "$project_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')

  cat <<EOF
{
  "embeds": [{
    "title": "✅ Gralph Complete",
    "description": "Session **${escaped_name}** has finished all tasks successfully.",
    "color": 5763719,
    "fields": [
      {
        "name": "Project",
        "value": "\`${escaped_dir}\`",
        "inline": false
      },
      {
        "name": "Iterations",
        "value": "${iterations}",
        "inline": true
      },
      {
        "name": "Duration",
        "value": "${duration_str}",
        "inline": true
      }
    ],
    "footer": {
      "text": "Gralph CLI"
    },
    "timestamp": "${timestamp}"
  }]
}
EOF
}

# format_slack_complete - Format completion notification for Slack
# Arguments:
#   $1 - session name
#   $2 - project directory
#   $3 - iterations
#   $4 - duration string
#   $5 - timestamp
# Returns:
#   Prints JSON payload formatted for Slack webhooks
format_slack_complete() {
  local session_name="$1"
  local project_dir="$2"
  local iterations="$3"
  local duration_str="$4"
  local timestamp="$5"

  # Escape strings for JSON
  local escaped_name escaped_dir
  escaped_name=$(echo "$session_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_dir=$(echo "$project_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')

  cat <<EOF
{
  "attachments": [{
    "color": "#57F287",
    "blocks": [
      {
        "type": "header",
        "text": {
          "type": "plain_text",
          "text": "✅ Gralph Complete",
          "emoji": true
        }
      },
      {
        "type": "section",
        "text": {
          "type": "mrkdwn",
          "text": "Session *${escaped_name}* has finished all tasks successfully."
        }
      },
      {
        "type": "section",
        "fields": [
          {
            "type": "mrkdwn",
            "text": "*Project:*\n\`${escaped_dir}\`"
          },
          {
            "type": "mrkdwn",
            "text": "*Iterations:*\n${iterations}"
          },
          {
            "type": "mrkdwn",
            "text": "*Duration:*\n${duration_str}"
          }
        ]
      },
      {
        "type": "context",
        "elements": [
          {
            "type": "mrkdwn",
            "text": "Gralph CLI • ${timestamp}"
          }
        ]
      }
    ]
  }]
}
EOF
}

# format_slack_failed - Format failure notification for Slack
# Arguments:
#   $1 - session name
#   $2 - project directory
#   $3 - failure reason
#   $4 - iterations
#   $5 - max iterations
#   $6 - remaining tasks
#   $7 - duration string
#   $8 - timestamp
# Returns:
#   Prints JSON payload formatted for Slack webhooks
format_slack_failed() {
  local session_name="$1"
  local project_dir="$2"
  local failure_reason="$3"
  local iterations="$4"
  local max_iterations="$5"
  local remaining_tasks="$6"
  local duration_str="$7"
  local timestamp="$8"

  # Escape strings for JSON
  local escaped_name escaped_dir escaped_reason
  escaped_name=$(echo "$session_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_dir=$(echo "$project_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_reason=$(echo "$failure_reason" | sed 's/\\/\\\\/g; s/"/\\"/g')

  # Build human-readable description based on failure reason
  local description
  case "$failure_reason" in
    max_iterations)
      description="Session *${escaped_name}* hit maximum iterations limit."
      ;;
    error)
      description="Session *${escaped_name}* encountered an error."
      ;;
    manual_stop)
      description="Session *${escaped_name}* was manually stopped."
      ;;
    *)
      description="Session *${escaped_name}* failed: ${escaped_reason}"
      ;;
  esac

  cat <<EOF
{
  "attachments": [{
    "color": "#ED4245",
    "blocks": [
      {
        "type": "header",
        "text": {
          "type": "plain_text",
          "text": "❌ Gralph Failed",
          "emoji": true
        }
      },
      {
        "type": "section",
        "text": {
          "type": "mrkdwn",
          "text": "${description}"
        }
      },
      {
        "type": "section",
        "fields": [
          {
            "type": "mrkdwn",
            "text": "*Project:*\n\`${escaped_dir}\`"
          },
          {
            "type": "mrkdwn",
            "text": "*Reason:*\n${escaped_reason}"
          },
          {
            "type": "mrkdwn",
            "text": "*Iterations:*\n${iterations}/${max_iterations}"
          },
          {
            "type": "mrkdwn",
            "text": "*Remaining Tasks:*\n${remaining_tasks}"
          },
          {
            "type": "mrkdwn",
            "text": "*Duration:*\n${duration_str}"
          }
        ]
      },
      {
        "type": "context",
        "elements": [
          {
            "type": "mrkdwn",
            "text": "Gralph CLI • ${timestamp}"
          }
        ]
      }
    ]
  }]
}
EOF
}

# format_discord_failed - Format failure notification for Discord
# Arguments:
#   $1 - session name
#   $2 - project directory
#   $3 - failure reason
#   $4 - iterations
#   $5 - max iterations
#   $6 - remaining tasks
#   $7 - duration string
#   $8 - timestamp
# Returns:
#   Prints JSON payload formatted for Discord webhooks
format_discord_failed() {
  local session_name="$1"
  local project_dir="$2"
  local failure_reason="$3"
  local iterations="$4"
  local max_iterations="$5"
  local remaining_tasks="$6"
  local duration_str="$7"
  local timestamp="$8"

  # Escape strings for JSON
  local escaped_name escaped_dir escaped_reason
  escaped_name=$(echo "$session_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_dir=$(echo "$project_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_reason=$(echo "$failure_reason" | sed 's/\\/\\\\/g; s/"/\\"/g')

  # Build human-readable description based on failure reason
  local description
  case "$failure_reason" in
    max_iterations)
      description="Session **${escaped_name}** hit maximum iterations limit."
      ;;
    error)
      description="Session **${escaped_name}** encountered an error."
      ;;
    manual_stop)
      description="Session **${escaped_name}** was manually stopped."
      ;;
    *)
      description="Session **${escaped_name}** failed: ${escaped_reason}"
      ;;
  esac

  cat <<EOF
{
  "embeds": [{
    "title": "❌ Gralph Failed",
    "description": "${description}",
    "color": 15548997,
    "fields": [
      {
        "name": "Project",
        "value": "\`${escaped_dir}\`",
        "inline": false
      },
      {
        "name": "Reason",
        "value": "${escaped_reason}",
        "inline": true
      },
      {
        "name": "Iterations",
        "value": "${iterations}/${max_iterations}",
        "inline": true
      },
      {
        "name": "Remaining Tasks",
        "value": "${remaining_tasks}",
        "inline": true
      },
      {
        "name": "Duration",
        "value": "${duration_str}",
        "inline": true
      }
    ],
    "footer": {
      "text": "Gralph CLI"
    },
    "timestamp": "${timestamp}"
  }]
}
EOF
}

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

  # Detect webhook type and format accordingly
  local webhook_type
  webhook_type=$(detect_webhook_type "$webhook_url")

  local payload
  case "$webhook_type" in
    discord)
      payload=$(format_discord_complete "$session_name" "$project_dir" "$iterations" "$duration_str" "$timestamp")
      ;;
    slack)
      payload=$(format_slack_complete "$session_name" "$project_dir" "$iterations" "$duration_str" "$timestamp")
      ;;
    *)
      # Generic JSON payload (default)
      payload=$(cat <<EOF
{
  "event": "complete",
  "status": "success",
  "session": "${escaped_name}",
  "project": "${escaped_dir}",
  "iterations": "${iterations}",
  "duration": "${duration_str}",
  "timestamp": "${timestamp}",
  "message": "Gralph loop '${escaped_name}' completed successfully after ${iterations} iterations (${duration_str})"
}
EOF
)
      ;;
  esac

  # Send the webhook
  send_webhook "$webhook_url" "$payload"
}

# notify_failed - Format and send a failure notification
# Arguments:
#   $1 - session name
#   $2 - webhook URL
#   $3 - failure reason (e.g., "max_iterations", "error", "manual_stop")
#   $4 - project directory (optional)
#   $5 - iterations completed (optional)
#   $6 - max iterations (optional)
#   $7 - remaining tasks count (optional)
#   $8 - total duration in seconds (optional)
# Returns:
#   0 on success, 1 on failure
notify_failed() {
  local session_name="$1"
  local webhook_url="$2"
  local failure_reason="${3:-unknown}"
  local project_dir="${4:-unknown}"
  local iterations="${5:-unknown}"
  local max_iterations="${6:-unknown}"
  local remaining_tasks="${7:-unknown}"
  local duration_secs="${8:-}"

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
  local escaped_reason
  escaped_name=$(echo "$session_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_dir=$(echo "$project_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')
  escaped_reason=$(echo "$failure_reason" | sed 's/\\/\\\\/g; s/"/\\"/g')

  # Build human-readable message based on failure reason
  local message
  case "$failure_reason" in
    max_iterations)
      message="Gralph loop '${escaped_name}' failed: hit max iterations (${iterations}/${max_iterations}) with ${remaining_tasks} tasks remaining"
      ;;
    error)
      message="Gralph loop '${escaped_name}' failed due to an error after ${iterations} iterations"
      ;;
    manual_stop)
      message="Gralph loop '${escaped_name}' was manually stopped after ${iterations} iterations with ${remaining_tasks} tasks remaining"
      ;;
    *)
      message="Gralph loop '${escaped_name}' failed: ${escaped_reason} after ${iterations} iterations"
      ;;
  esac

  # Detect webhook type and format accordingly
  local webhook_type
  webhook_type=$(detect_webhook_type "$webhook_url")

  local payload
  case "$webhook_type" in
    discord)
      payload=$(format_discord_failed "$session_name" "$project_dir" "$failure_reason" "$iterations" "$max_iterations" "$remaining_tasks" "$duration_str" "$timestamp")
      ;;
    slack)
      payload=$(format_slack_failed "$session_name" "$project_dir" "$failure_reason" "$iterations" "$max_iterations" "$remaining_tasks" "$duration_str" "$timestamp")
      ;;
    *)
      # Generic JSON payload (default)
      payload=$(cat <<EOF
{
  "event": "failed",
  "status": "failure",
  "session": "${escaped_name}",
  "project": "${escaped_dir}",
  "reason": "${escaped_reason}",
  "iterations": "${iterations}",
  "max_iterations": "${max_iterations}",
  "remaining_tasks": "${remaining_tasks}",
  "duration": "${duration_str}",
  "timestamp": "${timestamp}",
  "message": "${message}"
}
EOF
)
      ;;
  esac

  # Send the webhook
  send_webhook "$webhook_url" "$payload"
}
