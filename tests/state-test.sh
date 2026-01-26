#!/usr/bin/env bash
# state-test.sh - Tests for state management functionality
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
export GRALPH_STATE_DIR="$TEST_TMP_DIR/state"
export GRALPH_STATE_FILE="$GRALPH_STATE_DIR/state.json"
export GRALPH_LOCK_FILE="$GRALPH_STATE_DIR/state.lock"
export GRALPH_LOCK_DIR="$GRALPH_STATE_DIR/state.lock.dir"

cleanup() {
    rm -rf "$TEST_TMP_DIR"
}
trap cleanup EXIT

# Source state module
# shellcheck source=../lib/state.sh
source "$ROOT_DIR/lib/state.sh"

echo "Running state tests..."
echo "Test state dir: $GRALPH_STATE_DIR"
echo ""

# -----------------------------------------------------------------------------
# Test: init_state creates directory and file
# -----------------------------------------------------------------------------
test_init_state_creates_files() {
    # Remove state dir if exists
    rm -rf "$GRALPH_STATE_DIR"

    if init_state; then
        if [[ -d "$GRALPH_STATE_DIR" ]] && [[ -f "$GRALPH_STATE_FILE" ]]; then
            pass "init_state creates directory and state file"
        else
            fail "init_state should create directory and state file"
        fi
    else
        fail "init_state returned non-zero exit code"
    fi
}

# -----------------------------------------------------------------------------
# Test: init_state creates valid JSON
# -----------------------------------------------------------------------------
test_init_state_valid_json() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    if jq empty "$GRALPH_STATE_FILE" 2>/dev/null; then
        pass "init_state creates valid JSON"
    else
        fail "init_state should create valid JSON"
    fi
}

# -----------------------------------------------------------------------------
# Test: init_state creates empty sessions object
# -----------------------------------------------------------------------------
test_init_state_empty_sessions() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    local sessions
    sessions=$(jq -r '.sessions' "$GRALPH_STATE_FILE")

    if [[ "$sessions" == "{}" ]]; then
        pass "init_state creates empty sessions object"
    else
        fail "init_state should create empty sessions object, got: $sessions"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_session creates new session
# -----------------------------------------------------------------------------
test_set_session_creates_new() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    if set_session "test-session" "dir=/tmp/test" "status=running"; then
        local session
        session=$(get_session "test-session")
        local status
        status=$(echo "$session" | jq -r '.status')

        if [[ "$status" == "running" ]]; then
            pass "set_session creates new session with properties"
        else
            fail "set_session should set status to 'running', got: $status"
        fi
    else
        fail "set_session returned non-zero exit code"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_session updates existing session
# -----------------------------------------------------------------------------
test_set_session_updates_existing() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    set_session "update-test" "status=running" "iteration=1"
    set_session "update-test" "iteration=5"

    local session
    session=$(get_session "update-test")
    local iteration status
    iteration=$(echo "$session" | jq -r '.iteration')
    status=$(echo "$session" | jq -r '.status')

    if [[ "$iteration" == "5" ]] && [[ "$status" == "running" ]]; then
        pass "set_session updates existing session while preserving other fields"
    else
        fail "set_session should update iteration to 5 and preserve status, got: iteration=$iteration, status=$status"
    fi
}

# -----------------------------------------------------------------------------
# Test: get_session returns session data
# -----------------------------------------------------------------------------
test_get_session_returns_data() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    set_session "get-test" "dir=/path/to/project" "status=complete"

    local session
    session=$(get_session "get-test")
    local dir
    dir=$(echo "$session" | jq -r '.dir')

    if [[ "$dir" == "/path/to/project" ]]; then
        pass "get_session returns session data"
    else
        fail "get_session should return dir='/path/to/project', got: $dir"
    fi
}

# -----------------------------------------------------------------------------
# Test: get_session returns error for missing session
# -----------------------------------------------------------------------------
test_get_session_missing() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    local exit_code=0
    get_session "nonexistent" >/dev/null 2>&1 || exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        pass "get_session returns error for missing session"
    else
        fail "get_session should return error for missing session"
    fi
}

# -----------------------------------------------------------------------------
# Test: list_sessions returns all sessions
# -----------------------------------------------------------------------------
test_list_sessions() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    set_session "session-a" "status=running"
    set_session "session-b" "status=complete"

    local sessions
    sessions=$(list_sessions)
    local count
    count=$(echo "$sessions" | jq 'length')

    if [[ "$count" == "2" ]]; then
        pass "list_sessions returns all sessions"
    else
        fail "list_sessions should return 2 sessions, got: $count"
    fi
}

# -----------------------------------------------------------------------------
# Test: list_sessions returns empty array when no sessions
# -----------------------------------------------------------------------------
test_list_sessions_empty() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    local sessions
    sessions=$(list_sessions)

    if [[ "$sessions" == "[]" ]]; then
        pass "list_sessions returns empty array when no sessions"
    else
        fail "list_sessions should return '[]', got: $sessions"
    fi
}

# -----------------------------------------------------------------------------
# Test: delete_session removes session
# -----------------------------------------------------------------------------
test_delete_session() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    set_session "delete-test" "status=running"

    if delete_session "delete-test"; then
        local exit_code=0
        get_session "delete-test" >/dev/null 2>&1 || exit_code=$?

        if [[ $exit_code -ne 0 ]]; then
            pass "delete_session removes session"
        else
            fail "delete_session should remove the session"
        fi
    else
        fail "delete_session returned non-zero exit code"
    fi
}

# -----------------------------------------------------------------------------
# Test: delete_session returns error for missing session
# -----------------------------------------------------------------------------
test_delete_session_missing() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    local exit_code=0
    delete_session "nonexistent" >/dev/null 2>&1 || exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        pass "delete_session returns error for missing session"
    else
        fail "delete_session should return error for missing session"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_session handles integer values
# -----------------------------------------------------------------------------
test_set_session_integers() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    set_session "int-test" "iteration=42" "max_iterations=100"

    local session
    session=$(get_session "int-test")
    local iteration max_iter
    iteration=$(echo "$session" | jq '.iteration')
    max_iter=$(echo "$session" | jq '.max_iterations')

    if [[ "$iteration" == "42" ]] && [[ "$max_iter" == "100" ]]; then
        pass "set_session stores integers as numbers (not strings)"
    else
        fail "set_session should store integers as numbers, got: iteration=$iteration, max_iterations=$max_iter"
    fi
}

# -----------------------------------------------------------------------------
# Test: set_session handles boolean values
# -----------------------------------------------------------------------------
test_set_session_booleans() {
    rm -rf "$GRALPH_STATE_DIR"
    init_state

    set_session "bool-test" "active=true" "paused=false"

    local session
    session=$(get_session "bool-test")
    local active paused
    active=$(echo "$session" | jq '.active')
    paused=$(echo "$session" | jq '.paused')

    if [[ "$active" == "true" ]] && [[ "$paused" == "false" ]]; then
        pass "set_session stores booleans as booleans (not strings)"
    else
        fail "set_session should store booleans as booleans, got: active=$active, paused=$paused"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
test_init_state_creates_files
test_init_state_valid_json
test_init_state_empty_sessions
test_set_session_creates_new
test_set_session_updates_existing
test_get_session_returns_data
test_get_session_missing
test_list_sessions
test_list_sessions_empty
test_delete_session
test_delete_session_missing
test_set_session_integers
test_set_session_booleans

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "State tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All state tests passed."
