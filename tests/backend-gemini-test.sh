#!/usr/bin/env bash
# backend-gemini-test.sh - Tests for gemini backend
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

# Source the common backend module and gemini backend
# shellcheck source=../lib/backends/common.sh
source "$ROOT_DIR/lib/backends/common.sh"
# shellcheck source=../lib/backends/gemini.sh
source "$ROOT_DIR/lib/backends/gemini.sh"

echo "Running gemini backend tests..."
echo "Test tmp dir: $TEST_TMP_DIR"
echo ""

# -----------------------------------------------------------------------------
# Test: backend_name returns "gemini"
# -----------------------------------------------------------------------------
test_backend_name() {
    local result
    result=$(backend_name)

    if [[ "$result" == "gemini" ]]; then
        pass "backend_name returns 'gemini'"
    else
        fail "backend_name should return 'gemini', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: backend_get_models returns expected model list
# -----------------------------------------------------------------------------
test_backend_get_models() {
    local result
    result=$(backend_get_models)

    # Check that expected models are present
    if [[ "$result" == *"gemini-2.5-pro"* ]] && \
       [[ "$result" == *"gemini-2.5-flash"* ]] && \
       [[ "$result" == *"gemini-2.0-pro"* ]]; then
        pass "backend_get_models returns expected model list"
    else
        fail "backend_get_models should return gemini models, got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: backend_get_default_model returns a valid default
# -----------------------------------------------------------------------------
test_backend_get_default_model() {
    local result
    result=$(backend_get_default_model)
    local models
    models=$(backend_get_models)

    # Verify the default model is in the list of available models
    if [[ -n "$result" ]] && [[ "$models" == *"$result"* ]]; then
        pass "backend_get_default_model returns a valid default ('$result')"
    else
        fail "backend_get_default_model should return a model from the list, got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: backend_check_installed returns correct status based on CLI presence
# -----------------------------------------------------------------------------
test_backend_check_installed() {
    local exit_code=0
    backend_check_installed || exit_code=$?

    # We can't control whether gemini is actually installed, but we can verify
    # the function returns a valid exit code (0 or 1)
    if [[ $exit_code -eq 0 ]] || [[ $exit_code -eq 1 ]]; then
        if command -v gemini &> /dev/null; then
            if [[ $exit_code -eq 0 ]]; then
                pass "backend_check_installed returns 0 when gemini CLI is present"
            else
                fail "backend_check_installed should return 0 when gemini CLI is present"
            fi
        else
            if [[ $exit_code -eq 1 ]]; then
                pass "backend_check_installed returns 1 when gemini CLI is absent"
            else
                fail "backend_check_installed should return 1 when gemini CLI is absent"
            fi
        fi
    else
        fail "backend_check_installed should return 0 or 1, got $exit_code"
    fi
}

# -----------------------------------------------------------------------------
# Test: backend_get_install_hint returns non-empty hint
# -----------------------------------------------------------------------------
test_backend_get_install_hint() {
    local result
    result=$(backend_get_install_hint)

    if [[ -n "$result" ]]; then
        pass "backend_get_install_hint returns non-empty hint"
    else
        fail "backend_get_install_hint should return installation instructions"
    fi
}

# -----------------------------------------------------------------------------
# Test: backend_parse_text extracts file contents
# -----------------------------------------------------------------------------
test_backend_parse_text() {
    local test_file="$TEST_TMP_DIR/test-response.txt"
    echo "This is a test response from Gemini." > "$test_file"

    local result
    result=$(backend_parse_text "$test_file")

    if [[ "$result" == "This is a test response from Gemini." ]]; then
        pass "backend_parse_text extracts file contents correctly"
    else
        fail "backend_parse_text should extract file contents, got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: backend_parse_text returns error for missing file
# -----------------------------------------------------------------------------
test_backend_parse_text_missing_file() {
    local exit_code=0
    backend_parse_text "$TEST_TMP_DIR/nonexistent-file.txt" > /dev/null 2>&1 || exit_code=$?

    if [[ $exit_code -eq 1 ]]; then
        pass "backend_parse_text returns error for missing file"
    else
        fail "backend_parse_text should return 1 for missing file, got $exit_code"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
test_backend_name
test_backend_get_models
test_backend_get_default_model
test_backend_check_installed
test_backend_get_install_hint
test_backend_parse_text
test_backend_parse_text_missing_file

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "Gemini backend tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All gemini backend tests passed."
