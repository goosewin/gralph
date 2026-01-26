#!/usr/bin/env bash
# config-test.sh - Tests for config get/set functionality
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Colors for output (if terminal supports it)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    NC=''
fi

pass() {
    echo -e "${GREEN}PASS${NC}: $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() {
    echo -e "${RED}FAIL${NC}: $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

# Setup: Create isolated test environment
TEST_TMP_DIR="$(mktemp -d)"
export GRALPH_CONFIG_DIR="$TEST_TMP_DIR/config"
export GRALPH_GLOBAL_CONFIG="$GRALPH_CONFIG_DIR/config.yaml"
export GRALPH_DEFAULT_CONFIG="$ROOT_DIR/config/default.yaml"

cleanup() {
    rm -rf "$TEST_TMP_DIR"
}
trap cleanup EXIT

# Source config module
# shellcheck source=../lib/config.sh
source "$ROOT_DIR/lib/config.sh"

echo "Running config tests..."
echo "Test config dir: $GRALPH_CONFIG_DIR"
echo ""

# -----------------------------------------------------------------------------
# Test: load_config loads default config
# -----------------------------------------------------------------------------
test_load_default_config() {
    load_config
    local max_iter
    max_iter=$(get_config "defaults.max_iterations" "")

    if [[ -n "$max_iter" ]]; then
        pass "load_config loads defaults.max_iterations from default config"
    else
        fail "load_config should load defaults.max_iterations (got empty)"
    fi
}

# -----------------------------------------------------------------------------
# Test: get_config returns default when key not found
# -----------------------------------------------------------------------------
test_get_config_default() {
    local result
    result=$(get_config "nonexistent.key" "fallback_value") || true

    if [[ "$result" == "fallback_value" ]]; then
        pass "get_config returns default for missing key"
    else
        fail "get_config should return 'fallback_value', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: get_config returns empty and exit 1 for missing key with no default
# -----------------------------------------------------------------------------
test_get_config_missing_no_default() {
    local result
    local exit_code=0
    result=$(get_config "nonexistent.key" "") || exit_code=$?

    if [[ $exit_code -eq 0 && -z "$result" ]]; then
        pass "get_config returns empty for missing key with no default"
    else
        fail "get_config should return exit 0 and empty, got exit=$exit_code result='$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_config creates config dir if missing
# -----------------------------------------------------------------------------
test_set_config_creates_dir() {
    # Remove config dir if exists
    rm -rf "$GRALPH_CONFIG_DIR"

    set_config "test.key" "test_value"

    if [[ -d "$GRALPH_CONFIG_DIR" ]]; then
        pass "set_config creates config directory"
    else
        fail "set_config should create config directory"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_config writes simple key
# -----------------------------------------------------------------------------
test_set_config_simple_key() {
    set_config "simple_key" "simple_value"

    # Re-load and check
    load_config
    local result
    result=$(get_config "simple_key" "") || true

    if [[ "$result" == "simple_value" ]]; then
        pass "set_config writes and reads simple key"
    else
        fail "set_config simple key: expected 'simple_value', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_config writes nested key
# -----------------------------------------------------------------------------
test_set_config_nested_key() {
    set_config "parent.child" "nested_value"

    # Re-load and check
    load_config
    local result
    result=$(get_config "parent.child" "") || true

    if [[ "$result" == "nested_value" ]]; then
        pass "set_config writes and reads nested key"
    else
        fail "set_config nested key: expected 'nested_value', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_config updates existing key
# -----------------------------------------------------------------------------
test_set_config_updates_existing() {
    set_config "update_test" "original"
    set_config "update_test" "updated"

    load_config
    local result
    result=$(get_config "update_test" "") || true

    if [[ "$result" == "updated" ]]; then
        pass "set_config updates existing key"
    else
        fail "set_config update: expected 'updated', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: config_exists returns 0 for existing key
# -----------------------------------------------------------------------------
test_config_exists_true() {
    set_config "exists_test" "value"
    load_config

    if config_exists "exists_test"; then
        pass "config_exists returns 0 for existing key"
    else
        fail "config_exists should return 0 for existing key"
    fi
}

# -----------------------------------------------------------------------------
# Test: config_exists returns 1 for missing key
# -----------------------------------------------------------------------------
test_config_exists_false() {
    if ! config_exists "definitely_not_exists_xyz"; then
        pass "config_exists returns 1 for missing key"
    else
        fail "config_exists should return 1 for missing key"
    fi
}

# -----------------------------------------------------------------------------
# Test: Environment variable override
# -----------------------------------------------------------------------------
test_env_override() {
    export GRALPH_TEST_ENV_KEY="env_override_value"

    local result
    result=$(get_config "test.env.key" "default")

    if [[ "$result" == "env_override_value" ]]; then
        pass "get_config honors GRALPH_* environment override"
    else
        fail "get_config env override: expected 'env_override_value', got '$result'"
    fi

    unset GRALPH_TEST_ENV_KEY
}

# -----------------------------------------------------------------------------
# Test: Legacy environment variable override
# -----------------------------------------------------------------------------
test_legacy_env_override() {
    export GRALPH_MAX_ITERATIONS="99"

    load_config
    local result
    result=$(get_config "defaults.max_iterations" "")

    if [[ "$result" == "99" ]]; then
        pass "get_config honors legacy GRALPH_MAX_ITERATIONS override"
    else
        fail "get_config legacy override: expected '99', got '$result'"
    fi

    unset GRALPH_MAX_ITERATIONS
}

# -----------------------------------------------------------------------------
# Test: list_config shows all keys
# -----------------------------------------------------------------------------
test_list_config() {
    # Clear and set known config
    rm -f "$GRALPH_GLOBAL_CONFIG"
    set_config "list_test_a" "value_a"
    set_config "list_test_b" "value_b"
    load_config

    local output
    output=$(list_config)

    if echo "$output" | grep -q "list_test_a=value_a" && echo "$output" | grep -q "list_test_b=value_b"; then
        pass "list_config shows all configured keys"
    else
        fail "list_config should show all keys, got: $output"
    fi
}

# -----------------------------------------------------------------------------
# Test: YAML parsing handles comments
# -----------------------------------------------------------------------------
test_yaml_ignores_comments() {
    # Create a config with comments
    mkdir -p "$GRALPH_CONFIG_DIR"
    cat > "$GRALPH_GLOBAL_CONFIG" << 'EOF'
# This is a comment
comment_test: actual_value  # inline comment
EOF

    load_config
    local result
    result=$(get_config "comment_test" "") || true

    if [[ "$result" == "actual_value" ]]; then
        pass "YAML parser ignores comments correctly"
    else
        fail "YAML parser comment handling: expected 'actual_value', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: YAML parsing handles simple arrays
# -----------------------------------------------------------------------------
test_yaml_parses_arrays() {
    mkdir -p "$GRALPH_CONFIG_DIR"
    cat > "$GRALPH_GLOBAL_CONFIG" << 'EOF'
test:
  flags:
    - --headless
    - "--verbose"  # inline comment
EOF

    load_config
    local result
    result=$(get_config "test.flags" "") || true

    if [[ "$result" == "--headless,--verbose" ]]; then
        pass "YAML parser flattens simple arrays"
    else
        fail "YAML parser array handling: expected '--headless,--verbose', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
test_load_default_config
test_get_config_default
test_get_config_missing_no_default
test_set_config_creates_dir
test_set_config_simple_key
test_set_config_nested_key
test_set_config_updates_existing
test_config_exists_true
test_config_exists_false
test_env_override
test_legacy_env_override
test_list_config
test_yaml_ignores_comments
test_yaml_parses_arrays

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "Config tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All config tests passed."
