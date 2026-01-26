# prd.sh - PRD validation utilities

# _prd_trim() - Trim leading/trailing whitespace
# Arguments:
#   $1 - Input string
# Returns:
#   Prints trimmed string
_prd_trim() {
    local value="$1"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    printf '%s' "$value"
}

# prd_get_task_blocks() - Extract task blocks grouped by task headers
# Arguments:
#   $1 - Task file path (required)
# Output:
#   Prints each task block separated by NUL characters
prd_get_task_blocks() {
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
        else
            if [[ $in_block -eq 1 ]]; then
                block+=$'\n'"$line"
            fi
        fi
    done < "$task_file"

    if [[ $in_block -eq 1 ]]; then
        printf '%s\0' "$block"
    fi

    return 0
}

# _prd_extract_task_header_id() - Extract ID from task header line
# Arguments:
#   $1 - Task block text
# Returns:
#   Prints header ID if found
_prd_extract_task_header_id() {
    local block="$1"
    local line

    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*###\ Task[[:space:]]+(.+)$ ]]; then
            printf '%s' "${BASH_REMATCH[1]}"
            return 0
        fi
    done <<< "$block"

    return 0
}

# _prd_extract_task_id_field() - Extract ID from **ID** field
# Arguments:
#   $1 - Task block text
# Returns:
#   Prints ID field value if found
_prd_extract_task_id_field() {
    local block="$1"
    local line

    line=$(echo "$block" | grep -m 1 -E '^[[:space:]]*-[[:space:]]*\*\*ID\*\*' || true)
    if [[ -z "$line" ]]; then
        return 0
    fi

    line=$(echo "$line" | sed -E 's/^[[:space:]]*-[[:space:]]*\*\*ID\*\*[[:space:]]*//')
    line=$(_prd_trim "$line")
    printf '%s' "$line"
}

# _prd_task_label() - Select best task label for error reporting
# Arguments:
#   $1 - Task block text
# Returns:
#   Prints task ID or header ID
_prd_task_label() {
    local block="$1"
    local id_field
    local header_id

    id_field=$(_prd_extract_task_id_field "$block")
    if [[ -n "$id_field" ]]; then
        printf '%s' "$id_field"
        return 0
    fi

    header_id=$(_prd_extract_task_header_id "$block")
    if [[ -n "$header_id" ]]; then
        printf '%s' "$header_id"
        return 0
    fi

    printf '%s' "unknown"
}

# _prd_block_has_field() - Check for required field in a task block
# Arguments:
#   $1 - Task block text
#   $2 - Field name (e.g., "ID")
# Returns:
#   0 if present, 1 if missing
_prd_block_has_field() {
    local block="$1"
    local field="$2"

    echo "$block" | grep -qE "^[[:space:]]*-[[:space:]]*\*\*${field}\*\*"
}

# prd_emit_error() - Print a validation error line
# Arguments:
#   $1 - Task file name
#   $2 - Task label
#   $3 - Error message
prd_emit_error() {
    local task_file="$1"
    local task_label="$2"
    local message="$3"

    echo "PRD validation error: $task_file: $task_label: $message" >&2
}

# prd_validate_task_block() - Validate a single task block
# Arguments:
#   $1 - Task block text
#   $2 - Task file name (for error reporting)
# Returns:
#   0 if valid, 1 if errors found
prd_validate_task_block() {
    local block="$1"
    local task_file="$2"
    local errors=0
    local task_label
    local field

    task_label=$(_prd_task_label "$block")

    for field in "ID" "Context Bundle" "DoD" "Checklist" "Dependencies"; do
        if ! _prd_block_has_field "$block" "$field"; then
            prd_emit_error "$task_file" "$task_label" "Missing required field: $field"
            errors=$((errors + 1))
        fi
    done

    local unchecked_count
    unchecked_count=$(echo "$block" | grep -cE '^[[:space:]]*-[[:space:]]*\[ \]' || true)

    if [[ "$unchecked_count" -eq 0 ]]; then
        prd_emit_error "$task_file" "$task_label" "Missing unchecked task line"
        errors=$((errors + 1))
    elif [[ "$unchecked_count" -gt 1 ]]; then
        prd_emit_error "$task_file" "$task_label" "Multiple unchecked task lines ($unchecked_count)"
        errors=$((errors + 1))
    fi

    if [[ "$errors" -gt 0 ]]; then
        return 1
    fi

    return 0
}

# prd_validate_file() - Validate all task blocks in a PRD file
# Arguments:
#   $1 - Task file path (required)
# Returns:
#   0 if valid, 1 if errors found
prd_validate_file() {
    local task_file="$1"
    local errors=0
    local block

    if [[ -z "$task_file" ]]; then
        echo "Error: task_file is required" >&2
        return 1
    fi

    if [[ ! -f "$task_file" ]]; then
        echo "Error: Task file does not exist: $task_file" >&2
        return 1
    fi

    while IFS= read -r -d '' block; do
        if ! prd_validate_task_block "$block" "$task_file"; then
            errors=$((errors + 1))
        fi
    done < <(prd_get_task_blocks "$task_file")

    if [[ "$errors" -gt 0 ]]; then
        return 1
    fi

    return 0
}
