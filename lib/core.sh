#!/bin/bash
# core.sh - Core loop logic for ralph loop

# Source dependencies
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# jq filters for parsing Claude JSON stream output
JQ_STREAM_TEXT='select(.type == "assistant").message.content[]? | select(.type == "text").text // empty | gsub("\n"; "\r\n") | . + "\r\n\n"'
JQ_FINAL_RESULT='select(.type == "result").result // empty'

# run_iteration() - Execute a single Claude Code iteration
#
# Arguments:
#   $1 - Project directory (required)
#   $2 - Task file path relative to project (default: PRD.md)
#   $3 - Iteration number (required)
#   $4 - Max iterations (required)
#   $5 - Completion marker (default: COMPLETE)
#   $6 - Model override (optional)
#   $7 - Log file path (optional)
#
# Returns:
#   0 - Iteration completed successfully
#   1 - Iteration failed
#
# Output:
#   Writes full result to RLOOP_ITERATION_RESULT variable
#   Streams output to stdout and log file
#
run_iteration() {
    local project_dir="$1"
    local task_file="${2:-PRD.md}"
    local iteration="$3"
    local max_iterations="$4"
    local completion_marker="${5:-COMPLETE}"
    local model="$6"
    local log_file="$7"

    # Validate required arguments
    if [[ -z "$project_dir" ]]; then
        echo "Error: project_dir is required" >&2
        return 1
    fi
    if [[ -z "$iteration" ]]; then
        echo "Error: iteration number is required" >&2
        return 1
    fi
    if [[ -z "$max_iterations" ]]; then
        echo "Error: max_iterations is required" >&2
        return 1
    fi

    # Validate project directory exists
    if [[ ! -d "$project_dir" ]]; then
        echo "Error: Project directory does not exist: $project_dir" >&2
        return 1
    fi

    # Validate task file exists
    local full_task_path="$project_dir/$task_file"
    if [[ ! -f "$full_task_path" ]]; then
        echo "Error: Task file does not exist: $full_task_path" >&2
        return 1
    fi

    # Create temp file for capturing output
    local tmpfile
    tmpfile=$(mktemp)
    trap "rm -f '$tmpfile'" RETURN

    # Build the prompt
    local prompt
    prompt="Read $task_file carefully. Find any task marked '- [ ]' (unchecked).

If unchecked tasks exist:
- Complete ONE task fully
- Mark it '- [x]' in $task_file
- Commit changes
- Exit normally (do NOT output completion promise)

If ZERO '- [ ]' remain (all complete):
- Verify by searching the file
- Output ONLY: <promise>$completion_marker</promise>

CRITICAL: Never mention the promise unless outputting it as the completion signal.

Iteration: $iteration/$max_iterations"

    # Build claude command arguments
    local claude_args=(
        --dangerously-skip-permissions
        --verbose
        --print
        --output-format stream-json
    )

    # Add model override if specified
    if [[ -n "$model" ]]; then
        claude_args+=(--model "$model")
    fi

    # Add the prompt
    claude_args+=(-p "$prompt")

    # Change to project directory and run Claude
    pushd "$project_dir" > /dev/null || return 1

    # Execute Claude and capture/stream output
    # - Capture full JSON to tmpfile
    # - Stream human-readable text to stdout and log
    if [[ -n "$log_file" ]]; then
        IS_SANDBOX=1 claude "${claude_args[@]}" 2>&1 \
            | grep --line-buffered '^{' \
            | tee "$tmpfile" \
            | jq --unbuffered -rj "$JQ_STREAM_TEXT" \
            | tee -a "$log_file"
    else
        IS_SANDBOX=1 claude "${claude_args[@]}" 2>&1 \
            | grep --line-buffered '^{' \
            | tee "$tmpfile" \
            | jq --unbuffered -rj "$JQ_STREAM_TEXT"
    fi

    local claude_exit_code=${PIPESTATUS[0]}

    popd > /dev/null || return 1

    # Extract the final result from the JSON stream
    local result
    result=$(jq -r "$JQ_FINAL_RESULT" "$tmpfile" 2>/dev/null || cat "$tmpfile")

    # Export result for caller to access
    export RLOOP_ITERATION_RESULT="$result"

    # Return based on Claude's exit code
    return $claude_exit_code
}

# count_remaining_tasks() - Count unchecked tasks in a file
#
# Arguments:
#   $1 - Task file path (required)
#
# Returns:
#   Prints the count of remaining tasks to stdout
#
count_remaining_tasks() {
    local task_file="$1"

    if [[ -z "$task_file" ]]; then
        echo "0"
        return
    fi

    if [[ ! -f "$task_file" ]]; then
        echo "0"
        return
    fi

    # Count lines matching '- [ ]' pattern (unchecked checkbox)
    # Using grep -c with || echo "0" to handle no matches case
    grep -cE '^\s*- \[ \]' "$task_file" 2>/dev/null || echo "0"
}

# check_completion() - Check if loop should be considered complete
#
# Arguments:
#   $1 - Task file path (required)
#   $2 - Claude output/result text (required)
#   $3 - Completion marker (default: COMPLETE)
#
# Returns:
#   0 - Completion detected (zero tasks AND valid promise at end)
#   1 - Not complete (tasks remain OR no valid promise)
#
# Logic:
#   1. Count remaining '- [ ]' tasks in file
#   2. If count > 0, return 1 (not complete)
#   3. Check if promise appears at END of output (last 500 chars)
#   4. Verify promise is not negated (e.g., "cannot output <promise>...")
#   5. Return 0 only if both conditions met
#
check_completion() {
    local task_file="$1"
    local result="$2"
    local completion_marker="${3:-COMPLETE}"

    # Validate required arguments
    if [[ -z "$task_file" ]]; then
        echo "Error: task_file is required" >&2
        return 1
    fi
    if [[ -z "$result" ]]; then
        # No output means not complete
        return 1
    fi

    # Count remaining tasks
    local remaining
    remaining=$(count_remaining_tasks "$task_file")

    # Must have zero remaining tasks
    if [[ "$remaining" -gt 0 ]]; then
        return 1
    fi

    # Promise must appear at the end (last 500 chars), not just mentioned
    local tail_result
    tail_result=$(echo "$result" | tail -c 500)

    # Check if promise pattern exists in tail
    if ! echo "$tail_result" | grep -qE "<promise>$completion_marker</promise>"; then
        return 1
    fi

    # Verify it's not negated (common patterns like "cannot", "won't", etc.)
    # Check if negation words appear before the promise in the tail
    if echo "$tail_result" | grep -qiE "(cannot|can't|won't|will not|do not|don't|should not|shouldn't|must not|mustn't)[^<]*<promise>"; then
        return 1
    fi

    # All checks passed - genuine completion
    return 0
}
