#!/bin/bash
# config.sh - Configuration management for gralph

# Configuration directories and files
GRALPH_CONFIG_DIR="${GRALPH_CONFIG_DIR:-$HOME/.config/gralph}"
GRALPH_GLOBAL_CONFIG="${GRALPH_GLOBAL_CONFIG:-$GRALPH_CONFIG_DIR/config.yaml}"

# Determine script directory for default config location
_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_DEFAULT_CONFIG_DIR="${_SCRIPT_DIR}/../config"

# Alternative default config location when installed
if [[ ! -d "$_DEFAULT_CONFIG_DIR" ]] && [[ -d "${HOME}/.config/gralph/config" ]]; then
    _DEFAULT_CONFIG_DIR="${HOME}/.config/gralph/config"
fi

GRALPH_DEFAULT_CONFIG="${GRALPH_DEFAULT_CONFIG:-$_DEFAULT_CONFIG_DIR/default.yaml}"

# Project config filename (to be prepended with project dir)
GRALPH_PROJECT_CONFIG_NAME="${GRALPH_PROJECT_CONFIG_NAME:-.gralph.yaml}"

# Cache for loaded configuration (associative array)
declare -gA _GRALPH_CONFIG_CACHE

# _yaml_to_flat() - Convert YAML to flat key=value format
# Handles simple YAML (nested objects, strings, numbers, booleans)
# Arguments:
#   $1 - YAML file path
#   $2 - (optional) key prefix for nested values
# Outputs:
#   Flat key=value pairs, one per line
#   Nested keys are joined with dots (e.g., defaults.max_iterations)
_yaml_to_flat() {
    local yaml_file="$1"
    local prefix="${2:-}"

    if [[ ! -f "$yaml_file" ]]; then
        return 0
    fi

    # Use awk to parse simple YAML format
    # This handles:
    #   - key: value
    #   - nested:
    #       key: value
    #   - arrays (as comma-separated values)
    #   - comments (lines starting with #)
    awk -v prefix="$prefix" '
    BEGIN {
        current_indent = 0
        parent_key[0] = prefix
        indent_level[0] = -1
        level = 0
    }

    # Skip empty lines and comments
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*#/ { next }

    {
        # Calculate indentation (number of leading spaces)
        match($0, /^[[:space:]]*/)
        indent = RLENGTH

        # Remove leading/trailing whitespace for processing
        gsub(/^[[:space:]]+|[[:space:]]+$/, "")

        # Skip if line is empty after trimming
        if (length($0) == 0) next

        # Handle array items (- item)
        if (/^- /) {
            # Array item - skip for now (we handle arrays differently)
            next
        }

        # Parse key: value
        if (match($0, /^([a-zA-Z_][a-zA-Z0-9_]*)[[:space:]]*:[[:space:]]*(.*)$/, arr)) {
            key = arr[1]
            value = arr[2]

            # Determine the current level based on indentation
            while (level > 0 && indent <= indent_level[level]) {
                level--
            }

            # Build full key path
            full_key = ""
            for (i = 0; i <= level; i++) {
                if (parent_key[i] != "") {
                    if (full_key != "") full_key = full_key "."
                    full_key = full_key parent_key[i]
                }
            }
            if (full_key != "") full_key = full_key "."
            full_key = full_key key

            if (value == "" || value ~ /^[[:space:]]*$/) {
                # This is a parent key (has children)
                level++
                indent_level[level] = indent
                parent_key[level] = key
            } else {
                # Remove quotes from value if present
                gsub(/^["'"'"']|["'"'"']$/, "", value)
                # Remove inline comments
                gsub(/[[:space:]]+#.*$/, "", value)
                print full_key "=" value
            }
        }
    }
    ' "$yaml_file"
}

# _merge_configs() - Merge multiple config sources into cache
# Later sources override earlier ones
# Arguments:
#   $@ - Config file paths in order of priority (lowest to highest)
_merge_configs() {
    # Clear the cache
    _GRALPH_CONFIG_CACHE=()

    for config_file in "$@"; do
        if [[ -f "$config_file" ]]; then
            while IFS='=' read -r key value; do
                if [[ -n "$key" ]]; then
                    _GRALPH_CONFIG_CACHE["$key"]="$value"
                fi
            done < <(_yaml_to_flat "$config_file")
        fi
    done
}

# load_config() - Load and merge configuration from all sources
# Merges configs in order: default -> global -> project (later overrides earlier)
# Arguments:
#   $1 - (optional) Project directory for project-level config
# Returns:
#   0 on success
# Side effects:
#   Populates _GRALPH_CONFIG_CACHE associative array
load_config() {
    local project_dir="${1:-}"
    local project_config=""

    # Determine project config path if project dir provided
    if [[ -n "$project_dir" ]] && [[ -d "$project_dir" ]]; then
        project_config="$project_dir/$GRALPH_PROJECT_CONFIG_NAME"
    fi

    # Build list of config files to merge (in order of priority)
    local config_files=()

    # 1. Default config (lowest priority)
    if [[ -f "$GRALPH_DEFAULT_CONFIG" ]]; then
        config_files+=("$GRALPH_DEFAULT_CONFIG")
    fi

    # 2. Global config
    if [[ -f "$GRALPH_GLOBAL_CONFIG" ]]; then
        config_files+=("$GRALPH_GLOBAL_CONFIG")
    fi

    # 3. Project config (highest priority)
    if [[ -n "$project_config" ]] && [[ -f "$project_config" ]]; then
        config_files+=("$project_config")
    fi

    # Merge all configs
    _merge_configs "${config_files[@]}"

    return 0
}

# _legacy_env_override() - Check for legacy environment variable overrides
# Arguments:
#   $1 - Configuration key
# Outputs:
#   Prints the legacy environment variable value if set
# Returns:
#   0 if legacy override found, 1 otherwise
_legacy_env_override() {
    local key="$1"
    local legacy_env=""

    case "$key" in
        defaults.max_iterations)
            legacy_env="GRALPH_MAX_ITERATIONS"
            ;;
        defaults.task_file)
            legacy_env="GRALPH_TASK_FILE"
            ;;
        defaults.completion_marker)
            legacy_env="GRALPH_COMPLETION_MARKER"
            ;;
        defaults.backend)
            legacy_env="GRALPH_BACKEND"
            ;;
        defaults.model)
            legacy_env="GRALPH_MODEL"
            ;;
    esac

    if [[ -n "$legacy_env" && -n "${!legacy_env:-}" ]]; then
        echo "${!legacy_env}"
        return 0
    fi

    return 1
}

# get_config() - Get a specific configuration value
# Arguments:
#   $1 - Configuration key (dot notation, e.g., "defaults.max_iterations")
#   $2 - (optional) Default value if key not found
# Outputs:
#   The configuration value, or default if not found
# Returns:
#   0 if key found, 1 if using default
get_config() {
    local key="$1"
    local default_value="${2:-}"

    if [[ -z "$key" ]]; then
        echo "$default_value"
        return 1
    fi

    # Check legacy environment variable override first
    if _legacy_env_override "$key"; then
        return 0
    fi

    # Check environment variable override (full dotted key)
    # Convert key to env var format: defaults.max_iterations -> GRALPH_DEFAULTS_MAX_ITERATIONS
    local env_key="GRALPH_$(echo "$key" | tr '[:lower:].' '[:upper:]_')"
    if [[ -n "${!env_key:-}" ]]; then
        echo "${!env_key}"
        return 0
    fi

    # Check cache
    if [[ -v "_GRALPH_CONFIG_CACHE[$key]" ]]; then
        echo "${_GRALPH_CONFIG_CACHE[$key]}"
        return 0
    fi

    # Return default
    echo "$default_value"
    return 1
}

# set_config() - Set a global configuration value
# Writes to the global config file (~/.config/gralph/config.yaml)
# Arguments:
#   $1 - Configuration key (dot notation)
#   $2 - Value to set
# Returns:
#   0 on success, 1 on failure
set_config() {
    local key="$1"
    local value="$2"

    if [[ -z "$key" ]]; then
        echo "Error: Configuration key is required" >&2
        return 1
    fi

    # Ensure config directory exists
    if [[ ! -d "$GRALPH_CONFIG_DIR" ]]; then
        if ! mkdir -p "$GRALPH_CONFIG_DIR"; then
            echo "Error: Failed to create config directory: $GRALPH_CONFIG_DIR" >&2
            return 1
        fi
    fi

    # Create config file if it doesn't exist
    if [[ ! -f "$GRALPH_GLOBAL_CONFIG" ]]; then
        touch "$GRALPH_GLOBAL_CONFIG"
    fi

    # Parse the key into parts
    IFS='.' read -ra key_parts <<< "$key"
    local num_parts=${#key_parts[@]}

    if [[ $num_parts -eq 0 ]]; then
        echo "Error: Invalid configuration key" >&2
        return 1
    fi

    # For simple keys (no dots), just add/update at root level
    if [[ $num_parts -eq 1 ]]; then
        local simple_key="${key_parts[0]}"
        # Check if key exists and update, otherwise append
        if grep -qE "^${simple_key}[[:space:]]*:" "$GRALPH_GLOBAL_CONFIG" 2>/dev/null; then
            # Update existing key
            sed -i "s|^${simple_key}[[:space:]]*:.*|${simple_key}: ${value}|" "$GRALPH_GLOBAL_CONFIG"
        else
            # Append new key
            echo "${simple_key}: ${value}" >> "$GRALPH_GLOBAL_CONFIG"
        fi
    else
        # For nested keys, we need more complex handling
        # This is a simplified implementation that handles common 2-level nesting
        local parent="${key_parts[0]}"
        local child="${key_parts[1]}"

        # Check if parent section exists
        if grep -qE "^${parent}[[:space:]]*:" "$GRALPH_GLOBAL_CONFIG" 2>/dev/null; then
            # Parent exists, check if child exists under it
            # This is simplified - for full YAML manipulation, consider using yq
            if grep -qE "^[[:space:]]+${child}[[:space:]]*:" "$GRALPH_GLOBAL_CONFIG" 2>/dev/null; then
                # Update existing nested key (simplified approach)
                sed -i "/^[[:space:]]*${child}[[:space:]]*:/s|:.*|: ${value}|" "$GRALPH_GLOBAL_CONFIG"
            else
                # Add child under parent (insert after parent line)
                sed -i "/^${parent}[[:space:]]*:/a\\  ${child}: ${value}" "$GRALPH_GLOBAL_CONFIG"
            fi
        else
            # Parent doesn't exist, add both
            echo "" >> "$GRALPH_GLOBAL_CONFIG"
            echo "${parent}:" >> "$GRALPH_GLOBAL_CONFIG"
            echo "  ${child}: ${value}" >> "$GRALPH_GLOBAL_CONFIG"
        fi
    fi

    # Update the cache
    _GRALPH_CONFIG_CACHE["$key"]="$value"

    return 0
}

# config_exists() - Check if a configuration key exists
# Arguments:
#   $1 - Configuration key
# Returns:
#   0 if key exists, 1 if not
config_exists() {
    local key="$1"

    # Check legacy environment variable override first
    if _legacy_env_override "$key" > /dev/null; then
        return 0
    fi

    # Check environment variable override (full dotted key)
    local env_key="GRALPH_$(echo "$key" | tr '[:lower:].' '[:upper:]_')"
    if [[ -n "${!env_key:-}" ]]; then
        return 0
    fi

    # Check cache
    [[ -v "_GRALPH_CONFIG_CACHE[$key]" ]]
}

# list_config() - List all configuration values
# Outputs:
#   All configuration key=value pairs, one per line
list_config() {
    for key in "${!_GRALPH_CONFIG_CACHE[@]}"; do
        echo "$key=${_GRALPH_CONFIG_CACHE[$key]}"
    done | sort
}
