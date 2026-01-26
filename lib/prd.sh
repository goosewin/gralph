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

_prd_join_list() {
    local separator="$1"
    shift
    local output=""
    local item

    for item in "$@"; do
        if [[ -z "$item" ]]; then
            continue
        fi
        if [[ -z "$output" ]]; then
            output="$item"
        else
            output+="$separator$item"
        fi
    done

    printf '%s' "$output"
}

_prd_add_unique() {
    local array_name="$1"
    local value="$2"
    local -a existing=()
    local item

    if [[ -z "$array_name" || -z "$value" ]]; then
        return 0
    fi

    eval "existing=(\"\${${array_name}[@]-}\")"
    for item in "${existing[@]}"; do
        if [[ "$item" == "$value" ]]; then
            return 0
        fi
    done

    eval "${array_name}+=(\"$value\")"
}

prd_reset_stack_detection() {
    PRD_STACK_ROOT=""
    PRD_STACK_IDS=()
    PRD_STACK_LANGUAGES=()
    PRD_STACK_FRAMEWORKS=()
    PRD_STACK_TOOLS=()
    PRD_STACK_RUNTIMES=()
    PRD_STACK_PACKAGE_MANAGERS=()
    PRD_STACK_EVIDENCE=()
    PRD_STACK_SELECTED_IDS=()
}

_prd_record_stack_file() {
    local path="$1"

    if [[ -z "$path" ]]; then
        return 0
    fi

    if [[ -n "$PRD_STACK_ROOT" && "$path" == "$PRD_STACK_ROOT"/* ]]; then
        path="${path#"$PRD_STACK_ROOT/"}"
    fi

    _prd_add_unique PRD_STACK_EVIDENCE "$path"
}

_prd_json_has_dependency() {
    local package_json="$1"
    local dep="$2"

    if [[ -z "$package_json" || -z "$dep" || ! -f "$package_json" ]]; then
        return 1
    fi

    if command -v jq >/dev/null 2>&1; then
        if jq -e --arg dep "$dep" '.dependencies[$dep] or .devDependencies[$dep] or .peerDependencies[$dep]' "$package_json" >/dev/null 2>&1; then
            return 0
        fi
        return 1
    fi

    if grep -q "\"$dep\"" "$package_json"; then
        return 0
    fi
    return 1
}

prd_detect_stack() {
    local target_dir="$1"
    local nullglob_was_on=false

    prd_reset_stack_detection

    if [[ -z "$target_dir" || ! -d "$target_dir" ]]; then
        return 0
    fi

    PRD_STACK_ROOT="$target_dir"

    if shopt -q nullglob; then
        nullglob_was_on=true
    fi
    shopt -s nullglob

    if [[ -f "$target_dir/package.json" ]]; then
        _prd_add_unique PRD_STACK_IDS "Node.js"
        _prd_add_unique PRD_STACK_RUNTIMES "Node.js"
        _prd_add_unique PRD_STACK_LANGUAGES "JavaScript"
        _prd_record_stack_file "$target_dir/package.json"

        if [[ -f "$target_dir/tsconfig.json" ]]; then
            _prd_add_unique PRD_STACK_LANGUAGES "TypeScript"
            _prd_record_stack_file "$target_dir/tsconfig.json"
        fi

        if [[ -f "$target_dir/pnpm-lock.yaml" ]]; then
            _prd_add_unique PRD_STACK_PACKAGE_MANAGERS "pnpm"
            _prd_record_stack_file "$target_dir/pnpm-lock.yaml"
        fi

        if [[ -f "$target_dir/yarn.lock" ]]; then
            _prd_add_unique PRD_STACK_PACKAGE_MANAGERS "yarn"
            _prd_record_stack_file "$target_dir/yarn.lock"
        fi

        if [[ -f "$target_dir/package-lock.json" ]]; then
            _prd_add_unique PRD_STACK_PACKAGE_MANAGERS "npm"
            _prd_record_stack_file "$target_dir/package-lock.json"
        fi

        if [[ -f "$target_dir/bun.lockb" ]]; then
            _prd_add_unique PRD_STACK_RUNTIMES "Bun"
            _prd_add_unique PRD_STACK_PACKAGE_MANAGERS "bun"
            _prd_record_stack_file "$target_dir/bun.lockb"
        fi

        if [[ -f "$target_dir/bunfig.toml" ]]; then
            _prd_add_unique PRD_STACK_RUNTIMES "Bun"
            _prd_add_unique PRD_STACK_PACKAGE_MANAGERS "bun"
            _prd_record_stack_file "$target_dir/bunfig.toml"
        fi

        if [[ -f "$target_dir/next.config.js" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Next.js"
            _prd_record_stack_file "$target_dir/next.config.js"
        fi
        if [[ -f "$target_dir/next.config.mjs" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Next.js"
            _prd_record_stack_file "$target_dir/next.config.mjs"
        fi
        if [[ -f "$target_dir/next.config.cjs" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Next.js"
            _prd_record_stack_file "$target_dir/next.config.cjs"
        fi

        if [[ -f "$target_dir/nuxt.config.js" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Nuxt"
            _prd_record_stack_file "$target_dir/nuxt.config.js"
        fi
        if [[ -f "$target_dir/nuxt.config.ts" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Nuxt"
            _prd_record_stack_file "$target_dir/nuxt.config.ts"
        fi

        if [[ -f "$target_dir/svelte.config.js" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Svelte"
            _prd_record_stack_file "$target_dir/svelte.config.js"
        fi
        if [[ -f "$target_dir/svelte.config.ts" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Svelte"
            _prd_record_stack_file "$target_dir/svelte.config.ts"
        fi

        if [[ -f "$target_dir/vite.config.js" ]]; then
            _prd_add_unique PRD_STACK_TOOLS "Vite"
            _prd_record_stack_file "$target_dir/vite.config.js"
        fi
        if [[ -f "$target_dir/vite.config.ts" ]]; then
            _prd_add_unique PRD_STACK_TOOLS "Vite"
            _prd_record_stack_file "$target_dir/vite.config.ts"
        fi
        if [[ -f "$target_dir/vite.config.mjs" ]]; then
            _prd_add_unique PRD_STACK_TOOLS "Vite"
            _prd_record_stack_file "$target_dir/vite.config.mjs"
        fi

        if [[ -f "$target_dir/angular.json" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Angular"
            _prd_record_stack_file "$target_dir/angular.json"
        fi

        if [[ -f "$target_dir/vue.config.js" ]]; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Vue"
            _prd_record_stack_file "$target_dir/vue.config.js"
        fi

        if _prd_json_has_dependency "$target_dir/package.json" "react"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "React"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "next"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Next.js"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "vue"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Vue"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "@angular/core"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Angular"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "svelte"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Svelte"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "nuxt"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Nuxt"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "express"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Express"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "fastify"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Fastify"
        fi
        if _prd_json_has_dependency "$target_dir/package.json" "@nestjs/core"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "NestJS"
        fi
    fi

    if [[ -f "$target_dir/go.mod" ]]; then
        _prd_add_unique PRD_STACK_IDS "Go"
        _prd_add_unique PRD_STACK_LANGUAGES "Go"
        _prd_add_unique PRD_STACK_TOOLS "Go modules"
        _prd_record_stack_file "$target_dir/go.mod"
    fi

    if [[ -f "$target_dir/Cargo.toml" ]]; then
        _prd_add_unique PRD_STACK_IDS "Rust"
        _prd_add_unique PRD_STACK_LANGUAGES "Rust"
        _prd_add_unique PRD_STACK_TOOLS "Cargo"
        _prd_record_stack_file "$target_dir/Cargo.toml"
    fi

    if [[ -f "$target_dir/pyproject.toml" || -f "$target_dir/requirements.txt" || -f "$target_dir/poetry.lock" || -f "$target_dir/Pipfile" || -f "$target_dir/Pipfile.lock" ]]; then
        _prd_add_unique PRD_STACK_IDS "Python"
        _prd_add_unique PRD_STACK_LANGUAGES "Python"
        if [[ -f "$target_dir/pyproject.toml" ]]; then
            _prd_record_stack_file "$target_dir/pyproject.toml"
            if grep -qi "\[tool.poetry\]" "$target_dir/pyproject.toml"; then
                _prd_add_unique PRD_STACK_TOOLS "Poetry"
            fi
        fi
        if [[ -f "$target_dir/requirements.txt" ]]; then
            _prd_record_stack_file "$target_dir/requirements.txt"
        fi
        if [[ -f "$target_dir/poetry.lock" ]]; then
            _prd_record_stack_file "$target_dir/poetry.lock"
        fi
        if [[ -f "$target_dir/Pipfile" ]]; then
            _prd_record_stack_file "$target_dir/Pipfile"
        fi
        if [[ -f "$target_dir/Pipfile.lock" ]]; then
            _prd_record_stack_file "$target_dir/Pipfile.lock"
        fi

        if [[ -f "$target_dir/requirements.txt" ]] && grep -qiE '(^|[[:space:]])django([<>=]|$)' "$target_dir/requirements.txt"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Django"
        fi
        if [[ -f "$target_dir/requirements.txt" ]] && grep -qiE '(^|[[:space:]])flask([<>=]|$)' "$target_dir/requirements.txt"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Flask"
        fi
        if [[ -f "$target_dir/requirements.txt" ]] && grep -qiE '(^|[[:space:]])fastapi([<>=]|$)' "$target_dir/requirements.txt"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "FastAPI"
        fi

        if [[ -f "$target_dir/pyproject.toml" ]] && grep -qiE 'django|flask|fastapi' "$target_dir/pyproject.toml"; then
            if grep -qiE 'django' "$target_dir/pyproject.toml"; then
                _prd_add_unique PRD_STACK_FRAMEWORKS "Django"
            fi
            if grep -qiE 'flask' "$target_dir/pyproject.toml"; then
                _prd_add_unique PRD_STACK_FRAMEWORKS "Flask"
            fi
            if grep -qiE 'fastapi' "$target_dir/pyproject.toml"; then
                _prd_add_unique PRD_STACK_FRAMEWORKS "FastAPI"
            fi
        fi
    fi

    if [[ -f "$target_dir/Gemfile" ]]; then
        _prd_add_unique PRD_STACK_IDS "Ruby"
        _prd_add_unique PRD_STACK_LANGUAGES "Ruby"
        _prd_record_stack_file "$target_dir/Gemfile"
        if grep -qi "rails" "$target_dir/Gemfile"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Rails"
        fi
        if grep -qi "sinatra" "$target_dir/Gemfile"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Sinatra"
        fi
    fi

    if [[ -f "$target_dir/mix.exs" ]]; then
        _prd_add_unique PRD_STACK_IDS "Elixir"
        _prd_add_unique PRD_STACK_LANGUAGES "Elixir"
        _prd_record_stack_file "$target_dir/mix.exs"
        if grep -qi "phoenix" "$target_dir/mix.exs"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Phoenix"
        fi
    fi

    if [[ -f "$target_dir/composer.json" ]]; then
        _prd_add_unique PRD_STACK_IDS "PHP"
        _prd_add_unique PRD_STACK_LANGUAGES "PHP"
        _prd_record_stack_file "$target_dir/composer.json"
        if grep -qi "laravel" "$target_dir/composer.json"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Laravel"
        fi
    fi

    if [[ -f "$target_dir/pom.xml" ]]; then
        _prd_add_unique PRD_STACK_IDS "Java"
        _prd_add_unique PRD_STACK_LANGUAGES "Java"
        _prd_add_unique PRD_STACK_TOOLS "Maven"
        _prd_record_stack_file "$target_dir/pom.xml"
        if grep -qi "spring-boot" "$target_dir/pom.xml"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Spring Boot"
        fi
    fi

    if [[ -f "$target_dir/build.gradle" ]]; then
        _prd_add_unique PRD_STACK_IDS "Java"
        _prd_add_unique PRD_STACK_LANGUAGES "Java"
        _prd_add_unique PRD_STACK_TOOLS "Gradle"
        _prd_record_stack_file "$target_dir/build.gradle"
        if grep -qi "spring-boot" "$target_dir/build.gradle"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Spring Boot"
        fi
    fi
    if [[ -f "$target_dir/build.gradle.kts" ]]; then
        _prd_add_unique PRD_STACK_IDS "Java"
        _prd_add_unique PRD_STACK_LANGUAGES "Java"
        _prd_add_unique PRD_STACK_TOOLS "Gradle"
        _prd_record_stack_file "$target_dir/build.gradle.kts"
        if grep -qi "spring-boot" "$target_dir/build.gradle.kts"; then
            _prd_add_unique PRD_STACK_FRAMEWORKS "Spring Boot"
        fi
    fi

    local -a csproj_files=("$target_dir"/*.csproj)
    local -a sln_files=("$target_dir"/*.sln)
    if [[ ${#csproj_files[@]} -gt 0 || ${#sln_files[@]} -gt 0 ]]; then
        _prd_add_unique PRD_STACK_IDS ".NET"
        _prd_add_unique PRD_STACK_LANGUAGES "C#"
        for file in "${csproj_files[@]}"; do
            _prd_record_stack_file "$file"
        done
    fi
    for file in "${sln_files[@]}"; do
        _prd_record_stack_file "$file"
    done

    if [[ -f "$target_dir/Dockerfile" ]]; then
        _prd_add_unique PRD_STACK_TOOLS "Docker"
        _prd_record_stack_file "$target_dir/Dockerfile"
    fi
    if [[ -f "$target_dir/docker-compose.yml" ]]; then
        _prd_add_unique PRD_STACK_TOOLS "Docker Compose"
        _prd_record_stack_file "$target_dir/docker-compose.yml"
    fi
    if [[ -f "$target_dir/docker-compose.yaml" ]]; then
        _prd_add_unique PRD_STACK_TOOLS "Docker Compose"
        _prd_record_stack_file "$target_dir/docker-compose.yaml"
    fi
    if [[ -f "$target_dir/Makefile" ]]; then
        _prd_add_unique PRD_STACK_TOOLS "Make"
        _prd_record_stack_file "$target_dir/Makefile"
    fi

    local -a terraform_files=("$target_dir"/*.tf)
    if [[ ${#terraform_files[@]} -gt 0 ]]; then
        _prd_add_unique PRD_STACK_TOOLS "Terraform"
        for file in "${terraform_files[@]}"; do
            _prd_record_stack_file "$file"
        done
    fi

    PRD_STACK_SELECTED_IDS=("${PRD_STACK_IDS[@]}")

    if [[ "$nullglob_was_on" == "true" ]]; then
        shopt -s nullglob
    else
        shopt -u nullglob
    fi
}

prd_format_stack_summary() {
    local heading_level="${1:-2}"
    local header_prefix="##"
    local stacks_line
    local languages_line
    local frameworks_line
    local tools_line
    local runtimes_line
    local package_managers_line
    local output=""

    if [[ "$heading_level" == "1" ]]; then
        header_prefix="#"
    fi

    output+="${header_prefix} Stack Summary\n\n"

    if [[ ${#PRD_STACK_IDS[@]} -gt 0 ]]; then
        stacks_line=$(_prd_join_list ", " "${PRD_STACK_IDS[@]}")
    else
        stacks_line="Unknown"
    fi

    if [[ ${#PRD_STACK_LANGUAGES[@]} -gt 0 ]]; then
        languages_line=$(_prd_join_list ", " "${PRD_STACK_LANGUAGES[@]}")
    else
        languages_line="Unknown"
    fi

    if [[ ${#PRD_STACK_FRAMEWORKS[@]} -gt 0 ]]; then
        frameworks_line=$(_prd_join_list ", " "${PRD_STACK_FRAMEWORKS[@]}")
    else
        frameworks_line="None detected"
    fi

    if [[ ${#PRD_STACK_TOOLS[@]} -gt 0 ]]; then
        tools_line=$(_prd_join_list ", " "${PRD_STACK_TOOLS[@]}")
    else
        tools_line="None detected"
    fi

    if [[ ${#PRD_STACK_RUNTIMES[@]} -gt 0 ]]; then
        runtimes_line=$(_prd_join_list ", " "${PRD_STACK_RUNTIMES[@]}")
    else
        runtimes_line="Unknown"
    fi

    if [[ ${#PRD_STACK_PACKAGE_MANAGERS[@]} -gt 0 ]]; then
        package_managers_line=$(_prd_join_list ", " "${PRD_STACK_PACKAGE_MANAGERS[@]}")
    else
        package_managers_line="None detected"
    fi

    output+="- Stacks: ${stacks_line}\n"
    output+="- Languages: ${languages_line}\n"
    output+="- Runtimes: ${runtimes_line}\n"
    output+="- Frameworks: ${frameworks_line}\n"
    output+="- Tools: ${tools_line}\n"
    output+="- Package managers: ${package_managers_line}\n"

    if [[ ${#PRD_STACK_SELECTED_IDS[@]} -gt 0 && ${#PRD_STACK_SELECTED_IDS[@]} -lt ${#PRD_STACK_IDS[@]} ]]; then
        local selected_line
        selected_line=$(_prd_join_list ", " "${PRD_STACK_SELECTED_IDS[@]}")
        output+="- Stack focus: ${selected_line}\n"
    fi

    output+="\nEvidence:\n"
    if [[ ${#PRD_STACK_EVIDENCE[@]} -gt 0 ]]; then
        local item
        for item in "${PRD_STACK_EVIDENCE[@]}"; do
            output+="- ${item}\n"
        done
    else
        output+="- None found\n"
    fi

    printf '%s' "$output"
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

_prd_extract_context_entries() {
    local block="$1"
    local line
    local in_context=0
    local -a entries=()

    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*\*\*Context\ Bundle\*\* ]]; then
            in_context=1
        elif [[ $in_context -eq 1 ]] && [[ "$line" =~ ^[[:space:]]*-[[:space:]]*\*\*[^*]+\*\* ]]; then
            break
        fi

        if [[ $in_context -eq 1 ]]; then
            local rest="$line"
            while [[ "$rest" =~ \`([^\`]*)\` ]]; do
                entries+=("${BASH_REMATCH[1]}")
                rest="${rest#*\`${BASH_REMATCH[1]}\`}"
            done
        fi
    done <<< "$block"

    if [[ ${#entries[@]} -gt 0 ]]; then
        printf '%s\n' "${entries[@]}"
    fi
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

_prd_validate_stray_unchecked() {
    local task_file="$1"
    local line
    local in_block=0
    local line_number=0
    local errors=0

    if [[ -z "$task_file" || ! -f "$task_file" ]]; then
        return 0
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        line_number=$((line_number + 1))

        if [[ "$line" =~ ^[[:space:]]*###\ Task[[:space:]]+ ]]; then
            in_block=1
        elif [[ $in_block -eq 1 ]] && [[ "$line" =~ ^[[:space:]]*---[[:space:]]*$ || "$line" =~ ^[[:space:]]*##[[:space:]]+ ]]; then
            in_block=0
        fi

        if [[ $in_block -eq 0 ]] && [[ "$line" =~ ^[[:space:]]*-[[:space:]]*\[[[:space:]]\] ]]; then
            echo "PRD validation error: $task_file: line $line_number: Unchecked task line outside task block" >&2
            errors=$((errors + 1))
        fi
    done < "$task_file"

    if [[ "$errors" -gt 0 ]]; then
        return 1
    fi

    return 0
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
    local allow_missing_context="${3:-false}"
    local base_dir="$4"
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

    if [[ "$allow_missing_context" != "true" ]]; then
        local -a context_entries=()
        local entry
        while IFS= read -r entry; do
            entry=$(_prd_trim "$entry")
            if [[ -n "$entry" ]]; then
                context_entries+=("$entry")
            fi
        done < <(_prd_extract_context_entries "$block")

        if [[ ${#context_entries[@]} -eq 0 ]]; then
            prd_emit_error "$task_file" "$task_label" "Context Bundle must include at least one file path"
            errors=$((errors + 1))
        else
            for entry in "${context_entries[@]}"; do
                local resolved_path=""
                if [[ "$entry" == /* ]]; then
                    resolved_path="$entry"
                    if [[ -n "$base_dir" && "$resolved_path" != "$base_dir"/* ]]; then
                        prd_emit_error "$task_file" "$task_label" "Context Bundle path outside repo: $entry"
                        errors=$((errors + 1))
                        continue
                    fi
                else
                    resolved_path="$base_dir/$entry"
                fi

                if [[ ! -e "$resolved_path" ]]; then
                    prd_emit_error "$task_file" "$task_label" "Context Bundle path not found: $entry"
                    errors=$((errors + 1))
                fi
            done
        fi
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
    local allow_missing_context="${2:-false}"
    local base_dir_override="${3:-}"
    local errors=0
    local block
    local base_dir=""

    if [[ -z "$task_file" ]]; then
        echo "Error: task_file is required" >&2
        return 1
    fi

    if [[ ! -f "$task_file" ]]; then
        echo "Error: Task file does not exist: $task_file" >&2
        return 1
    fi

    if [[ -n "$base_dir_override" ]]; then
        if base_dir=$(cd "$base_dir_override" 2>/dev/null && pwd); then
            :
        else
            base_dir="$base_dir_override"
        fi
    else
        base_dir=$(cd "$(dirname "$task_file")" && pwd 2>/dev/null || true)
        if [[ -z "$base_dir" ]]; then
            base_dir=$(dirname "$task_file")
        fi
    fi

    if grep -qE '^[[:space:]]*#+[[:space:]]+Open Questions\b' "$task_file"; then
        echo "PRD validation error: $task_file: Open Questions section is not allowed" >&2
        errors=$((errors + 1))
    fi

    if ! _prd_validate_stray_unchecked "$task_file"; then
        errors=$((errors + 1))
    fi

    while IFS= read -r -d '' block; do
        if ! prd_validate_task_block "$block" "$task_file" "$allow_missing_context" "$base_dir"; then
            errors=$((errors + 1))
        fi
    done < <(prd_get_task_blocks "$task_file")

    if [[ "$errors" -gt 0 ]]; then
        return 1
    fi

    return 0
}

_prd_context_entry_exists() {
    local entry="$1"
    local base_dir="$2"

    if [[ -z "$entry" ]]; then
        return 1
    fi

    if [[ "$entry" == /* ]]; then
        if [[ -n "$base_dir" && "$entry" != "$base_dir"/* ]]; then
            return 1
        fi
        [[ -e "$entry" ]]
        return $?
    fi

    if [[ -z "$base_dir" ]]; then
        return 1
    fi

    [[ -e "$base_dir/$entry" ]]
}

_prd_context_display_path() {
    local entry="$1"
    local base_dir="$2"

    if [[ -z "$entry" ]]; then
        return 0
    fi

    if [[ "$entry" == /* && -n "$base_dir" && "$entry" == "$base_dir"/* ]]; then
        printf '%s' "${entry#"$base_dir/"}"
        return 0
    fi

    printf '%s' "$entry"
}

_prd_load_allowed_context() {
    local file_path="$1"
    local -n allowed_ref="$2"
    local line

    if [[ -z "$file_path" || ! -f "$file_path" ]]; then
        return 0
    fi

    while IFS= read -r line; do
        line=$(_prd_trim "$line")
        if [[ -n "$line" ]]; then
            allowed_ref["$line"]=1
        fi
    done < "$file_path"
}

_prd_pick_fallback_context() {
    local base_dir="$1"
    local allowed_file="$2"
    local line

    if [[ -n "$allowed_file" && -f "$allowed_file" ]]; then
        while IFS= read -r line; do
            line=$(_prd_trim "$line")
            if [[ -n "$line" && -e "$base_dir/$line" ]]; then
                printf '%s' "$line"
                return 0
            fi
        done < "$allowed_file"
    fi

    if [[ -n "$base_dir" && -e "$base_dir/README.md" ]]; then
        printf '%s' "README.md"
        return 0
    fi

    return 0
}

_prd_sanitize_task_block() {
    local block="$1"
    local base_dir="$2"
    local allowed_context_file="$3"
    local -A allowed_context=()
    local -a context_entries=()
    local -a valid_entries=()
    local line
    local unchecked_seen=false
    local output=""
    local context_line=""
    local fallback=""

    _prd_load_allowed_context "$allowed_context_file" allowed_context

    while IFS= read -r line; do
        line=$(_prd_trim "$line")
        if [[ -n "$line" ]]; then
            context_entries+=("$line")
        fi
    done < <(_prd_extract_context_entries "$block")

    for line in "${context_entries[@]}"; do
        local display
        display=$(_prd_context_display_path "$line" "$base_dir")
        if ! _prd_context_entry_exists "$display" "$base_dir"; then
            continue
        fi
        if [[ ${#allowed_context[@]} -gt 0 && -z "${allowed_context[$display]:-}" ]]; then
            continue
        fi
        _prd_add_unique valid_entries "$display"
    done

    if [[ ${#valid_entries[@]} -eq 0 ]]; then
        fallback=$(_prd_pick_fallback_context "$base_dir" "$allowed_context_file")
        if [[ -n "$fallback" ]]; then
            valid_entries=("$fallback")
        fi
    fi

    if [[ ${#valid_entries[@]} -gt 0 ]]; then
        local formatted=""
        local entry
        for entry in "${valid_entries[@]}"; do
            if [[ -z "$formatted" ]]; then
                formatted="\`$entry\`"
            else
                formatted+=", \`$entry\`"
            fi
        done
        context_line="- **Context Bundle** ${formatted}"
    else
        context_line="- **Context Bundle**"
    fi

    local in_context_block=0
    while IFS= read -r line; do
        if [[ "$line" =~ ^([[:space:]]*)-[[:space:]]*\*\*Context\ Bundle\*\* ]]; then
            local indent="${BASH_REMATCH[1]}"
            output+="${indent}${context_line}"$'\n'
            in_context_block=1
            continue
        fi

        if [[ $in_context_block -eq 1 ]]; then
            if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*\*\*[^*]+\*\* ]]; then
                in_context_block=0
            else
                continue
            fi
        fi

        if [[ "$line" =~ ^([[:space:]]*)-[[:space:]]*\[[[:space:]]\][[:space:]]*(.*)$ ]]; then
            if [[ "$unchecked_seen" == "true" ]]; then
                line="${BASH_REMATCH[1]}- ${BASH_REMATCH[2]}"
            else
                unchecked_seen=true
            fi
        fi

        output+="$line"$'\n'
    done <<< "$block"

    printf '%s' "$output"
}

prd_sanitize_generated_file() {
    local task_file="$1"
    local base_dir="$2"
    local allowed_context_file="$3"
    local tmp_file
    local line
    local block=""
    local in_block=0
    local in_open_questions=0
    local started=false

    if [[ -z "$task_file" || ! -f "$task_file" ]]; then
        return 0
    fi

    tmp_file=$(mktemp)

    while IFS= read -r line || [[ -n "$line" ]]; do
        local lower
        lower=$(printf '%s' "$line" | tr '[:upper:]' '[:lower:]')

        if [[ "$lower" =~ ^[[:space:]]*##[[:space:]]+open[[:space:]]+questions\b ]]; then
            in_open_questions=1
            continue
        fi

        if [[ $in_open_questions -eq 1 ]]; then
            if [[ "$line" =~ ^[[:space:]]*##[[:space:]]+ ]]; then
                in_open_questions=0
            else
                continue
            fi
        fi

        if [[ "$started" == "false" ]]; then
            if [[ "$line" =~ ^[[:space:]]*# ]]; then
                started=true
            else
                continue
            fi
        fi

        if [[ "$line" =~ ^[[:space:]]*###\ Task[[:space:]]+ ]]; then
            if [[ $in_block -eq 1 ]]; then
                _prd_sanitize_task_block "$block" "$base_dir" "$allowed_context_file" >> "$tmp_file"
            fi
            in_block=1
            block="$line"
            continue
        fi

        if [[ $in_block -eq 1 ]] && [[ "$line" =~ ^[[:space:]]*---[[:space:]]*$ || "$line" =~ ^[[:space:]]*##[[:space:]]+ ]]; then
            _prd_sanitize_task_block "$block" "$base_dir" "$allowed_context_file" >> "$tmp_file"
            in_block=0
            block=""
        fi

        if [[ $in_block -eq 1 ]]; then
            block+=$'\n'"$line"
        else
            if [[ "$line" =~ ^([[:space:]]*)-[[:space:]]*\[[[:space:]]\][[:space:]]*(.*)$ ]]; then
                line="${BASH_REMATCH[1]}- ${BASH_REMATCH[2]}"
            fi
            printf '%s\n' "$line" >> "$tmp_file"
        fi
    done < "$task_file"

    if [[ $in_block -eq 1 ]]; then
        _prd_sanitize_task_block "$block" "$base_dir" "$allowed_context_file" >> "$tmp_file"
    fi

    mv "$tmp_file" "$task_file"
}
