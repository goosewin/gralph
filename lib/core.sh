#!/bin/bash
# core.sh - Core loop logic for gralph

# Source dependencies
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source backend abstraction layer
if [[ -f "$SCRIPT_DIR/backends/common.sh" ]]; then
    source "$SCRIPT_DIR/backends/common.sh"
fi

# Source notification module
if [[ -f "$SCRIPT_DIR/notify.sh" ]]; then
    source "$SCRIPT_DIR/notify.sh"
fi

# Source config module
if [[ -f "$SCRIPT_DIR/config.sh" ]]; then
    source "$SCRIPT_DIR/config.sh"
fi

# Default backend (can be overridden via config or CLI)
GRALPH_BACKEND="${GRALPH_BACKEND:-claude}"

# cleanup_old_logs() - Delete log files older than retain_days
#
# Arguments:
#   $1 - Log directory path (required)
#   $2 - Retention days (optional, defaults to config value or 7)
#
# Returns:
#   0 on success
#
# Note:
#   Uses `find` with -mtime to identify files older than retain_days.
#   Only deletes .log files to avoid removing other state files.
#
cleanup_old_logs() {
    local log_dir="$1"
    local retain_days="${2:-}"

    # If no retain_days provided, get from config
    if [[ -z "$retain_days" ]]; then
        retain_days=$(get_config "logging.retain_days" "7")
    fi

    # Validate inputs
    if [[ -z "$log_dir" ]] || [[ ! -d "$log_dir" ]]; then
        return 0
    fi

    # Ensure retain_days is a positive integer
    if ! [[ "$retain_days" =~ ^[0-9]+$ ]] || [[ "$retain_days" -le 0 ]]; then
        retain_days=7
    fi

    # Find and delete log files older than retain_days
    # Using -mtime +N means "more than N days ago"
    find "$log_dir" -maxdepth 1 -name "*.log" -type f -mtime +"$retain_days" -delete 2>/dev/null || true

    return 0
}

# init_backend() - Initialize the backend for use
#
# Arguments:
#   $1 - backend name (optional, uses GRALPH_BACKEND if not specified)
#
# Returns:
#   0 on success, 1 on failure
init_backend() {
    local backend="${1:-$GRALPH_BACKEND}"

    # Load the backend
    if ! load_backend "$backend"; then
        return 1
    fi

    # Validate the backend has all required functions
    if ! validate_backend; then
        return 1
    fi

    # Check if backend CLI is installed
    if ! backend_check_installed; then
        echo "Error: Backend '$backend' CLI is not installed" >&2
        echo "Install with: $(backend_get_install_hint)" >&2
        return 1
    fi

    return 0
}

# Default prompt template with placeholder variables:
#   {task_file}          - Name of the task file (e.g., PRD.md)
#   {completion_marker}  - The completion promise marker (e.g., COMPLETE)
#   {iteration}          - Current iteration number
#   {max_iterations}     - Maximum iterations allowed
#   {task_block}         - Selected task block or placeholder string
#   {context_files_section} - Context files section (optional)
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

{context_files_section}
Task Block:
{task_block}

Iteration: {iteration}/{max_iterations}'

# get_task_blocks() - Extract task blocks grouped by task headers
#
# Arguments:
#   $1 - Task file path (required)
#
# Output:
#   Prints each task block separated by NUL characters
#   Blocks include the '### Task <ID>' header line
#
# Returns:
#   0 on success
#
get_task_blocks() {
    local task_file="$1"

    if [[ -z "$task_file" ]] || [[ ! -f "$task_file" ]]; then
        return 0
    fi

    local line
    local in_block=0
    local block=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ "$line" =~ ^[[:space:]]*###\ Task[[:space:]]+ ]]; then
            if [[ $in_block -eq 1 ]]; then
                printf '%s\0' "$block"
            fi
            in_block=1
            block="$line"
            continue
        fi

        if [[ $in_block -eq 1 ]] && [[ "$line" =~ ^[[:space:]]*---[[:space:]]*$ || "$line" =~ ^[[:space:]]*##[[:space:]]+ ]]; then
            printf '%s\0' "$block"
            in_block=0
            block=""
            continue
        fi

        if [[ $in_block -eq 1 ]]; then
            block+=$'\n'"$line"
        fi
    done < "$task_file"

    if [[ $in_block -eq 1 ]]; then
        printf '%s\0' "$block"
    fi

    return 0
}

# get_next_unchecked_task_block() - Select first task block with unchecked task line
#
# Arguments:
#   $1 - Task file path (required)
#
# Output:
#   Prints the first task block containing an unchecked task line
#   Prints nothing if no unchecked task lines exist in any block
#
# Returns:
#   0 on success
#
get_next_unchecked_task_block() {
    local task_file="$1"

    if [[ -z "$task_file" ]] || [[ ! -f "$task_file" ]]; then
        return 0
    fi

    local block
    while IFS= read -r -d '' block; do
        if echo "$block" | grep -qE '^\s*- \[ \]'; then
            printf '%s' "$block"
            return 0
        fi
    done < <(get_task_blocks "$task_file")

    return 0
}

# render_prompt_template() - Substitute variables in a prompt template
#
# Arguments:
#   $1 - Template string (required)
#   $2 - Task file name (required)
#   $3 - Completion marker (required)
#   $4 - Current iteration (required)
#   $5 - Max iterations (required)
#   $6 - Task block text (optional)
#   $7 - Context files list (optional)
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
    local task_block="${6:-}"
    local context_files="${7:-}"

    if [[ -z "$task_block" ]]; then
        task_block="No task block available."
    fi

    local context_files_section=""
    if [[ -n "$context_files" ]]; then
        context_files_section=$'Context Files (read these first):\n'"$context_files"$'\n'
    fi

    # Substitute all template variables
    local rendered="$template"
    rendered="${rendered//\{task_file\}/$task_file}"
    rendered="${rendered//\{completion_marker\}/$completion_marker}"
    rendered="${rendered//\{iteration\}/$iteration}"
    rendered="${rendered//\{max_iterations\}/$max_iterations}"
    rendered="${rendered//\{task_block\}/$task_block}"
    rendered="${rendered//\{context_files\}/$context_files}"
    rendered="${rendered//\{context_files_section\}/$context_files_section}"

    echo "$rendered"
}

# normalize_context_files() - Normalize a comma-separated context file list
#
# Arguments:
#   $1 - Raw context files string (optional)
#
# Returns:
#   Prints a newline-separated list of trimmed entries
#
normalize_context_files() {
    local raw="${1:-}"
    local entry
    local normalized=""

    if [[ -z "$raw" ]]; then
        return 0
    fi

    IFS=',' read -r -a context_entries <<< "$raw"
    for entry in "${context_entries[@]}"; do
        entry="${entry#"${entry%%[![:space:]]*}"}"
        entry="${entry%"${entry##*[![:space:]]}"}"
        if [[ -n "$entry" ]]; then
            if [[ -n "$normalized" ]]; then
                normalized+=$'\n'
            fi
            normalized+="$entry"
        fi
    done

    printf '%s' "$normalized"
}

# run_iteration() - Execute a single AI coding iteration
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
#   Writes full result to GRALPH_ITERATION_RESULT variable
#   Streams output to stdout and log file
#
# Note:
#   Requires init_backend() to be called first, or GRALPH_BACKEND to be set
#
run_iteration() {
    local project_dir="$1"
    local task_file="${2:-PRD.md}"
    local iteration="$3"
    local max_iterations="$4"
    local completion_marker="${5:-COMPLETE}"
    local model="$6"
    local log_file="$7"
    local prompt_template="$8"
    local raw_output_file=""

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

    # Ensure backend is loaded
    if [[ -z "$GRALPH_CURRENT_BACKEND" ]]; then
        if ! init_backend "$GRALPH_BACKEND"; then
            return 1
        fi
    fi

    # Create temp file for capturing output
    local tmpfile
    tmpfile=$(mktemp)
    trap "rm -f '$tmpfile'" RETURN

    if [[ -n "$log_file" ]]; then
        if [[ "$log_file" == *.log ]]; then
            raw_output_file="${log_file%.log}.raw.log"
        else
            raw_output_file="${log_file}.raw.log"
        fi
    fi

    # Resolve prompt template (argument > file override > default)
    if [[ -z "$prompt_template" ]]; then
        local template_path=""
        if [[ -n "$GRALPH_PROMPT_TEMPLATE_FILE" ]] && [[ -f "$GRALPH_PROMPT_TEMPLATE_FILE" ]]; then
            template_path="$GRALPH_PROMPT_TEMPLATE_FILE"
        else
            template_path="$project_dir/.gralph/prompt-template.txt"
        fi

        if [[ -n "$template_path" ]] && [[ -f "$template_path" ]]; then
            prompt_template=$(cat "$template_path")
        else
            prompt_template="$DEFAULT_PROMPT_TEMPLATE"
        fi
    fi

    # Build task block for prompt injection
    local task_block
    task_block=$(get_next_unchecked_task_block "$full_task_path")
    if [[ -z "$task_block" ]]; then
        local remaining_tasks
        remaining_tasks=$(count_remaining_tasks "$full_task_path")
        if [[ "$remaining_tasks" -gt 0 ]]; then
            task_block=$(grep -m 1 -E '^\s*- \[ \]' "$full_task_path" 2>/dev/null || true)
        fi
    fi

    # Read context files list from config
    local context_files
    context_files=$(get_config "defaults.context_files" "")

    local normalized_context_files
    normalized_context_files=$(normalize_context_files "$context_files")

    # Build the prompt using template
    local prompt
    prompt=$(render_prompt_template "$prompt_template" "$task_file" "$completion_marker" "$iteration" "$max_iterations" "$task_block" "$normalized_context_files")

    # Change to project directory and run backend
    pushd "$project_dir" > /dev/null || return 1

    # Execute backend and capture/stream output
    local backend_exit_code
    if [[ -n "$log_file" ]]; then
        GRALPH_RAW_OUTPUT_FILE="$raw_output_file" backend_run_iteration "$prompt" "$model" "$tmpfile" | tee -a "$log_file"
        backend_exit_code=${PIPESTATUS[0]}
    else
        GRALPH_RAW_OUTPUT_FILE="$raw_output_file" backend_run_iteration "$prompt" "$model" "$tmpfile"
        backend_exit_code=$?
    fi

    popd > /dev/null || return 1

    # If no output was produced, treat as failure and surface raw output path
    if [[ ! -s "$tmpfile" ]]; then
        if [[ -n "$log_file" ]]; then
            echo "Error: backend '$GRALPH_BACKEND' produced no JSON output." | tee -a "$log_file"
            if [[ -n "$raw_output_file" ]] && [[ -s "$raw_output_file" ]]; then
                echo "Raw output saved to: $raw_output_file" | tee -a "$log_file"
            fi
        fi
        backend_exit_code=1
    fi

    # Extract the final result using backend's parser
    local result
    local parse_exit_code
    result=$(backend_parse_text "$tmpfile")
    parse_exit_code=$?

    # Export result for caller to access
    export GRALPH_ITERATION_RESULT="$result"

    if [[ $parse_exit_code -ne 0 || -z "$result" ]]; then
        if [[ -n "$log_file" ]]; then
            echo "Error: backend '$GRALPH_BACKEND' returned no parsed result." | tee -a "$log_file"
            if [[ -n "$raw_output_file" ]] && [[ -s "$raw_output_file" ]]; then
                echo "Raw output saved to: $raw_output_file" | tee -a "$log_file"
            fi
        fi
        backend_exit_code=1
    fi

    # Return based on backend's exit code
    return $backend_exit_code
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

    local count=0

    if grep -qE '^[[:space:]]*###\ Task[[:space:]]+' "$task_file" 2>/dev/null; then
        local block
        while IFS= read -r -d '' block; do
            local block_count
            block_count=$(echo "$block" | grep -cE '^\s*- \[ \]' || true)
            count=$((count + block_count))
        done < <(get_task_blocks "$task_file")
    else
        # Count lines matching '- [ ]' pattern (unchecked checkbox)
        # Ensure a single numeric output even when grep returns non-zero
        count=$(grep -cE '^\s*- \[ \]' "$task_file" 2>/dev/null || true)
    fi

    echo "$count"
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
#   3. Require promise as the last non-empty line
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
    if [[ ! -f "$task_file" ]]; then
        echo "Error: task_file does not exist: $task_file" >&2
        return 1
    fi

    # Count remaining tasks
    local remaining
    remaining=$(count_remaining_tasks "$task_file")

    # Must have zero remaining tasks
    if [[ "$remaining" -gt 0 ]]; then
        return 1
    fi

    # Promise must be the last non-empty line
    local promise_line
    promise_line=$(printf '%s' "$result" | awk 'NF{line=$0} END{print line}')
    if ! printf '%s' "$promise_line" | grep -qE "^[[:space:]]*<promise>$completion_marker</promise>[[:space:]]*$"; then
        return 1
    fi

    # Verify it's not negated (common patterns like "cannot", "won't", etc.)
    if printf '%s' "$promise_line" | grep -qiE "(cannot|can't|won't|will not|do not|don't|should not|shouldn't|must not|mustn't)[^<]*<promise>"; then
        return 1
    fi

    # All checks passed - genuine completion
    return 0
}

# run_loop() - Execute the main gralph loop
#
# Arguments:
#   $1 - Project directory (required)
#   $2 - Task file path relative to project (default: PRD.md)
#   $3 - Max iterations (default: 30)
#   $4 - Completion marker (default: COMPLETE)
#   $5 - Model override (optional)
#   $6 - Session name (optional, for state updates and per-session logs)
#   $7 - Prompt template (optional, uses DEFAULT_PROMPT_TEMPLATE if not provided)
#
# Returns:
#   0 - All tasks completed successfully
#   1 - Max iterations reached or error occurred
#
# Environment:
#   GRALPH_STATE_CALLBACK - Optional function name to call for state updates
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
    if ! [[ "$max_iterations" =~ ^[0-9]+$ ]] || [[ "$max_iterations" -le 0 ]]; then
        echo "Error: max_iterations must be a positive integer" >&2
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

    # Set up logging with per-session log file
    local gralph_dir="$project_dir/.gralph"
    mkdir -p "$gralph_dir"

    # Clean up old log files based on retention policy
    cleanup_old_logs "$gralph_dir"

    # Use session name for log file if provided, otherwise default to 'gralph'
    local log_name="${session_name:-gralph}"
    local log_file="$gralph_dir/${log_name}.log"

    # Track loop start time for duration calculation
    local loop_start_time
    loop_start_time=$(date +%s)

    # Initialize iteration counter
    local iteration=1

    # Log startup information
    echo "Starting gralph loop in $project_dir" | tee "$log_file"
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
        if [[ -n "$GRALPH_STATE_CALLBACK" ]] && declare -f "$GRALPH_STATE_CALLBACK" > /dev/null; then
            "$GRALPH_STATE_CALLBACK" "$session_name" "$iteration" "running" "$remaining_before"
        fi

        # Check if already complete before running iteration
        if [[ "$remaining_before" -eq 0 ]]; then
            echo "Zero tasks remaining before iteration, verifying completion..." | tee -a "$log_file"
        fi

        # Run single iteration (capture failures without exiting due to set -e)
        local iteration_exit_code=0
        if run_iteration \
            "$project_dir" \
            "$task_file" \
            "$iteration" \
            "$max_iterations" \
            "$completion_marker" \
            "$model" \
            "$log_file" \
            "$prompt_template"; then
            iteration_exit_code=0
        else
            iteration_exit_code=$?
        fi

        if [[ $iteration_exit_code -ne 0 ]]; then
            local raw_log
            if [[ "$log_file" == *.log ]]; then
                raw_log="${log_file%.log}.raw.log"
            else
                raw_log="${log_file}.raw.log"
            fi

            echo "Iteration failed with exit code $iteration_exit_code." | tee -a "$log_file"
            if [[ -s "$raw_log" ]]; then
                echo "Raw backend output: $raw_log" | tee -a "$log_file"
            fi

            if [[ -n "$GRALPH_STATE_CALLBACK" ]] && declare -f "$GRALPH_STATE_CALLBACK" > /dev/null; then
                "$GRALPH_STATE_CALLBACK" "$session_name" "$iteration" "failed" "$remaining_before"
            fi

            return 1
        fi

        # Get the result from the iteration
        local result="$GRALPH_ITERATION_RESULT"

        # Check for genuine completion
        if check_completion "$full_task_path" "$result" "$completion_marker"; then
            # Calculate loop duration
            local loop_end_time loop_duration_secs
            loop_end_time=$(date +%s)
            loop_duration_secs=$((loop_end_time - loop_start_time))
            export GRALPH_LOOP_DURATION_SECS="$loop_duration_secs"

            echo "" | tee -a "$log_file"
            echo "Gralph complete after $iteration iterations." | tee -a "$log_file"
            echo "Duration: ${loop_duration_secs}s" | tee -a "$log_file"
            echo "FINISHED: $(date -Iseconds)" | tee -a "$log_file"

            # Update state if callback is defined
            if [[ -n "$GRALPH_STATE_CALLBACK" ]] && declare -f "$GRALPH_STATE_CALLBACK" > /dev/null; then
                "$GRALPH_STATE_CALLBACK" "$session_name" "$iteration" "complete" "0"
            fi

            # Send completion notification if configured
            local notify_on_complete webhook_url
            notify_on_complete=$(get_config "notifications.on_complete" "false")
            webhook_url=$(get_config "notifications.webhook" "")

            if [[ "$notify_on_complete" == "true" && -n "$webhook_url" ]]; then
                if declare -f notify_complete > /dev/null; then
                    notify_complete "$session_name" "$webhook_url" "$project_dir" "$iteration" "$loop_duration_secs" || \
                        echo "Warning: Failed to send completion notification" >&2
                fi
            fi

            return 0
        fi

        # Log remaining tasks after iteration
        local remaining_after
        remaining_after=$(count_remaining_tasks "$full_task_path")
        echo "Tasks remaining after iteration: $remaining_after" | tee -a "$log_file"

        # Update state with new task count
        if [[ -n "$GRALPH_STATE_CALLBACK" ]] && declare -f "$GRALPH_STATE_CALLBACK" > /dev/null; then
            "$GRALPH_STATE_CALLBACK" "$session_name" "$iteration" "running" "$remaining_after"
        fi

        # Increment iteration counter
        ((iteration++))

        # Small delay between iterations to avoid hammering the API
        if [[ $iteration -le $max_iterations ]]; then
            sleep 2
        fi
    done

    # Max iterations reached
    local final_remaining
    final_remaining=$(count_remaining_tasks "$full_task_path")

    # Calculate loop duration
    local loop_end_time loop_duration_secs
    loop_end_time=$(date +%s)
    loop_duration_secs=$((loop_end_time - loop_start_time))
    export GRALPH_LOOP_DURATION_SECS="$loop_duration_secs"

    echo "" | tee -a "$log_file"
    echo "Hit max iterations ($max_iterations)" | tee -a "$log_file"
    echo "Remaining tasks: $final_remaining" | tee -a "$log_file"
    echo "Duration: ${loop_duration_secs}s" | tee -a "$log_file"
    echo "FINISHED: $(date -Iseconds)" | tee -a "$log_file"

    # Update state if callback is defined
    if [[ -n "$GRALPH_STATE_CALLBACK" ]] && declare -f "$GRALPH_STATE_CALLBACK" > /dev/null; then
        "$GRALPH_STATE_CALLBACK" "$session_name" "$max_iterations" "max_iterations" "$final_remaining"
    fi

    # Send failure notification if configured
    local notify_on_fail webhook_url
    notify_on_fail=$(get_config "notifications.on_fail" "false")
    webhook_url=$(get_config "notifications.webhook" "")

    if [[ "$notify_on_fail" == "true" && -n "$webhook_url" ]]; then
        if declare -f notify_failed > /dev/null; then
            notify_failed "$session_name" "$webhook_url" "max_iterations" "$project_dir" \
                "$max_iterations" "$max_iterations" "$final_remaining" "$loop_duration_secs" || \
                echo "Warning: Failed to send failure notification" >&2
        fi
    fi

    return 1
}
