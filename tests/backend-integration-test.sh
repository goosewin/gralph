#!/usr/bin/env bash
# backend-integration-test.sh - Integration tests for all backend modules
# Verifies all four backends can be loaded and validated.
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

# Source the common backend module
# shellcheck source=../lib/backends/common.sh
source "$ROOT_DIR/lib/backends/common.sh"

echo "Running backend integration tests..."
echo "Test tmp dir: $TEST_TMP_DIR"
echo ""

# Required functions that every backend must implement
REQUIRED_FUNCTIONS=(
    "backend_name"
    "backend_check_installed"
    "backend_get_install_hint"
    "backend_run_iteration"
    "backend_parse_text"
    "backend_get_models"
)

# -----------------------------------------------------------------------------
# test_backend_loads_and_validates() - Generic test for any backend
#
# Arguments:
#   $1 - backend name (e.g., "claude", "opencode", "gemini", "codex")
# -----------------------------------------------------------------------------
test_backend_loads_and_validates() {
    local backend_name="$1"

    # Clear any previously defined backend functions
    for func in "${REQUIRED_FUNCTIONS[@]}"; do
        unset -f "$func" 2>/dev/null || true
    done

    # Try to load the backend
    local load_exit=0
    load_backend "$backend_name" || load_exit=$?

    if [[ $load_exit -ne 0 ]]; then
        fail "$backend_name backend: failed to load"
        return
    fi

    pass "$backend_name backend: loaded successfully"

    # Validate all required functions exist
    local missing_functions=()
    for func in "${REQUIRED_FUNCTIONS[@]}"; do
        if ! declare -f "$func" > /dev/null 2>&1; then
            missing_functions+=("$func")
        fi
    done

    if [[ ${#missing_functions[@]} -gt 0 ]]; then
        fail "$backend_name backend: missing required functions: ${missing_functions[*]}"
        return
    fi

    pass "$backend_name backend: all required functions exist"

    # Validate backend_name returns the correct name
    local returned_name
    returned_name=$(backend_name)

    if [[ "$returned_name" == "$backend_name" ]]; then
        pass "$backend_name backend: backend_name returns '$backend_name'"
    else
        fail "$backend_name backend: backend_name should return '$backend_name', got '$returned_name'"
    fi

    # Validate backend_get_models returns non-empty result
    local models
    models=$(backend_get_models)

    if [[ -n "$models" ]]; then
        pass "$backend_name backend: backend_get_models returns model list"
    else
        fail "$backend_name backend: backend_get_models returned empty"
    fi

    # Validate backend_get_install_hint returns non-empty result
    local hint
    hint=$(backend_get_install_hint)

    if [[ -n "$hint" ]]; then
        pass "$backend_name backend: backend_get_install_hint returns hint"
    else
        fail "$backend_name backend: backend_get_install_hint returned empty"
    fi

    # Validate backend_check_installed returns 0 or 1
    local check_exit=0
    backend_check_installed || check_exit=$?

    if [[ $check_exit -eq 0 ]] || [[ $check_exit -eq 1 ]]; then
        pass "$backend_name backend: backend_check_installed returns valid exit code ($check_exit)"
    else
        fail "$backend_name backend: backend_check_installed returned invalid exit code ($check_exit)"
    fi
}

# -----------------------------------------------------------------------------
# Test: All backends in list_available_backends
# -----------------------------------------------------------------------------
test_list_available_backends() {
    local backends
    backends=$(list_available_backends)

    local expected_backends=("claude" "opencode" "gemini" "codex")
    local missing=()

    for backend in "${expected_backends[@]}"; do
        if [[ "$backends" != *"$backend"* ]]; then
            missing+=("$backend")
        fi
    done

    if [[ ${#missing[@]} -eq 0 ]]; then
        pass "list_available_backends includes all expected backends"
    else
        fail "list_available_backends missing: ${missing[*]} (got: $backends)"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
echo "=== Testing list_available_backends ==="
test_list_available_backends
echo ""

echo "=== Testing claude backend ==="
test_backend_loads_and_validates "claude"
echo ""

echo "=== Testing opencode backend ==="
test_backend_loads_and_validates "opencode"
echo ""

echo "=== Testing gemini backend ==="
test_backend_loads_and_validates "gemini"
echo ""

echo "=== Testing codex backend ==="
test_backend_loads_and_validates "codex"
echo ""

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo "========================================"
echo "Backend integration tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All backend integration tests passed."
