#!/bin/bash
# core.sh - Core loop logic for ralph loop

# Source dependencies
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# jq filters for parsing Claude JSON stream output
JQ_STREAM_TEXT='select(.type == "assistant").message.content[]? | select(.type == "text").text // empty | gsub("\n"; "\r\n") | . + "\r\n\n"'
JQ_FINAL_RESULT='select(.type == "result").result // empty'

# Default prompt template with placeholder variables:
#   {task_file}          - Name of the task file (e.g., PRD.md)
#   {completion_marker}  - The completion promise marker (e.g., COMPLETE)
#   {iteration}          - Current iteration number
#   {max_iterations}     - Maximum iterations allowed
DEFAULT_PROMPT_TEMPLATE='Read {task_file} carefully. Find any task marked '\''- [ ]'\'' (unchecked).

If unchecked tasks exist:
- Complete ONE task fully
- Mark it '\''- [x]'\'' in {task_file}
- Commit changes
- Exit normally (do NOT output completion promise)

If ZERO '\''- [ ]'\'' remain (all complete):
- Verify by searching the file
- Output ONLY: <promise>{completion_marker}</promise>

CRITICAL: Never mention the promise unless outputting it as the completion signal.

Iteration: {iteration}/{max_iterations}'

# render_prompt_template() - Substitute variables in a prompt template
#
# Arguments:
#   $1 - Template string (required)
#   $2 - Task file name (required)
#   $3 - Completion marker (required)
#   $4 - Current iteration (required)
#   $5 - Max iterations (required)
#
# Returns:
#   Prints the rendered prompt to stdout
#
render_prompt_template() {
    local template="$1"
    local task_file="$2"
    local completion_marker="$3"
    local iteration="$4"
    local max_iterations="$5"

    # Substitute all template variables
    local rendered="$template"
    rendered="${rendered//\{task_file\}/$task_file}"
    rendered="${rendered//\{completion_marker\}/$completion_marker}"
    rendered="${rendered//\{iteration\}/$iteration}"
    rendered="${rendered//\{max_iterations\}/$max_iterations}"

    echo "$rendered"
}

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
#   $8 - Prompt template (optional, uses DEFAULT_PROMPT_TEMPLATE if not provided)
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
    local prompt_template="${8:-$DEFAULT_PROMPT_TEMPLATE}"

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

    # Build the prompt using template
    local prompt
    prompt=$(render_prompt_template "$prompt_template" "$task_file" "$completion_marker" "$iteration" "$max_iterations")

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

# run_loop() - Execute the main ralph loop
#
# Arguments:
#   $1 - Project directory (required)
#   $2 - Task file path relative to project (default: PRD.md)
#   $3 - Max iterations (default: 30)
#   $4 - Completion marker (default: COMPLETE)
#   $5 - Model override (optional)
#   $6 - Session name (optional, for state updates)
#   $7 - Prompt template (optional, uses DEFAULT_PROMPT_TEMPLATE if not provided)
#
# Returns:
#   0 - All tasks completed successfully
#   1 - Max iterations reached or error occurred
#
# Environment:
#   RLOOP_STATE_CALLBACK - Optional function name to call for state updates
#                          Called with: session_name iteration status remaining_tasks
#
run_loop() {
    local project_dir="$1"
    local task_file="${2:-PRD.md}"
    local max_iterations="${3:-30}"
    local completion_marker="${4:-COMPLETE}"
    local model="$5"
    local session_name="$6"
    local prompt_template="${7:-$DEFAULT_PROMPT_TEMPLATE}"

    # Validate required arguments
    if [[ -z "$project_dir" ]]; then
        echo "Error: project_dir is required" >&2
        return 1
    fi

    # Resolve to absolute path
    project_dir=$(cd "$project_dir" && pwd)

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

    # Set up logging
    local rloop_dir="$project_dir/.rloop"
    mkdir -p "$rloop_dir"
    local log_file="$rloop_dir/ralph.log"

    # Initialize iteration counter
    local iteration=1

    # Log startup information
    echo "Starting ralph loop in $project_dir" | tee "$log_file"
    echo "Task file: $task_file" | tee -a "$log_file"
    echo "Max iterations: $max_iterations" | tee -a "$log_file"
    echo "Completion marker: $completion_marker" | tee -a "$log_file"
    if [[ -n "$model" ]]; then
        echo "Model: $model" | tee -a "$log_file"
    fi
    echo "Started at: $(date -Iseconds)" | tee -a "$log_file"

    local initial_remaining
    initial_remaining=$(count_remaining_tasks "$full_task_path")
    echo "Initial remaining tasks: $initial_remaining" | tee -a "$log_file"

    # Main loop
    while [[ $iteration -le $max_iterations ]]; do
        local remaining_before
        remaining_before=$(count_remaining_tasks "$full_task_path")

        echo "" | tee -a "$log_file"
        echo "=== Iteration $iteration/$max_iterations (Remaining: $remaining_before) ===" | tee -a "$log_file"

        # Update state if callback is defined
        if [[ -n "$RLOOP_STATE_CALLBACK" ]] && declare -f "$RLOOP_STATE_CALLBACK" > /dev/null; then
            "$RLOOP_STATE_CALLBACK" "$session_name" "$iteration" "running" "$remaining_before"
        fi

        # Check if already complete before running iteration
        if [[ "$remaining_before" -eq 0 ]]; then
            echo "Zero tasks remaining before iteration, verifying completion..." | tee -a "$log_file"
        fi

        # Run single iteration
        run_iteration \
            "$project_dir" \
            "$task_file" \
            "$iteration" \
            "$max_iterations" \
            "$completion_marker" \
            "$model" \
            "$log_file" \
            "$prompt_template"

        local iteration_exit_code=$?

        # Get the result from the iteration
        local result="$RLOOP_ITERATION_RESULT"

        # Check for genuine completion
        if check_completion "$full_task_path" "$result" "$completion_marker"; then
            echo "" | tee -a "$log_file"
            echo "✅ Ralph complete after $iteration iterations." | tee -a "$log_file"
            echo "FINISHED: $(date -Iseconds)" | tee -a "$log_file"

            # Update state if callback is defined
            if [[ -n "$RLOOP_STATE_CALLBACK" ]] && declare -f "$RLOOP_STATE_CALLBACK" > /dev/null; then
                "$RLOOP_STATE_CALLBACK" "$session_name" "$iteration" "complete" "0"
            fi

            return 0
        fi

        # Log remaining tasks after iteration
        local remaining_after
        remaining_after=$(count_remaining_tasks "$full_task_path")
        echo "Tasks remaining after iteration: $remaining_after" | tee -a "$log_file"

        # Update state with new task count
        if [[ -n "$RLOOP_STATE_CALLBACK" ]] && declare -f "$RLOOP_STATE_CALLBACK" > /dev/null; then
            "$RLOOP_STATE_CALLBACK" "$session_name" "$iteration" "running" "$remaining_after"
        fi

        # Increment iteration counter
        ((iteration++))

        # Small delay between iterations to avoid hammering the API
        sleep 2
    done

    # Max iterations reached
    local final_remaining
    final_remaining=$(count_remaining_tasks "$full_task_path")

    echo "" | tee -a "$log_file"
    echo "⚠️ Hit max iterations ($max_iterations)" | tee -a "$log_file"
    echo "Remaining tasks: $final_remaining" | tee -a "$log_file"
    echo "FINISHED: $(date -Iseconds)" | tee -a "$log_file"

    # Update state if callback is defined
    if [[ -n "$RLOOP_STATE_CALLBACK" ]] && declare -f "$RLOOP_STATE_CALLBACK" > /dev/null; then
        "$RLOOP_STATE_CALLBACK" "$session_name" "$max_iterations" "max_iterations" "$final_remaining"
    fi

    return 1
}
