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
