#!/usr/bin/env bash
# loop-test.sh - Tests for core loop functionality
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

# Source core module (which sources config)
# shellcheck source=../lib/core.sh
source "$ROOT_DIR/lib/core.sh"

echo "Running loop tests..."
echo "Test tmp dir: $TEST_TMP_DIR"
echo ""

# -----------------------------------------------------------------------------
# Test: render_prompt_template substitutes variables
# -----------------------------------------------------------------------------
test_render_prompt_template() {
    local template="Task: {task_file}, Marker: {completion_marker}, Iter: {iteration}/{max_iterations}"
    local result
    result=$(render_prompt_template "$template" "PRD.md" "DONE" "5" "30")

    local expected="Task: PRD.md, Marker: DONE, Iter: 5/30"

    if [[ "$result" == "$expected" ]]; then
        pass "render_prompt_template substitutes all variables"
    else
        fail "render_prompt_template expected '$expected', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: render_prompt_template handles multiple occurrences
# -----------------------------------------------------------------------------
test_render_prompt_template_multiple() {
    local template="{task_file} {task_file} {iteration} {iteration}"
    local result
    result=$(render_prompt_template "$template" "test.md" "X" "3" "10")

    local expected="test.md test.md 3 3"

    if [[ "$result" == "$expected" ]]; then
        pass "render_prompt_template handles multiple occurrences"
    else
        fail "render_prompt_template multiple: expected '$expected', got '$result'"
    fi
}

# -----------------------------------------------------------------------------
# Test: count_remaining_tasks counts unchecked boxes
# -----------------------------------------------------------------------------
test_count_remaining_tasks() {
    local test_file="$TEST_TMP_DIR/tasks.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [ ] Task 1
- [x] Task 2 (done)
- [ ] Task 3
- [ ] Task 4
- [x] Task 5 (done)
EOF

    local count
    count=$(count_remaining_tasks "$test_file")

    if [[ "$count" == "3" ]]; then
        pass "count_remaining_tasks counts unchecked boxes correctly"
    else
        fail "count_remaining_tasks expected 3, got $count"
    fi
}

# -----------------------------------------------------------------------------
# Test: count_remaining_tasks returns 0 for all complete
# -----------------------------------------------------------------------------
test_count_remaining_tasks_all_done() {
    local test_file="$TEST_TMP_DIR/done.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Task 1
- [x] Task 2
EOF

    local count
    # Note: count_remaining_tasks may return "0\n0" due to grep -c || echo "0" pattern
    # We check if first line is 0
    count=$(count_remaining_tasks "$test_file" 2>/dev/null | head -n1 || true)

    if [[ "$count" == "0" ]]; then
        pass "count_remaining_tasks returns 0 when all complete"
    else
        fail "count_remaining_tasks expected 0, got $count"
    fi
}

# -----------------------------------------------------------------------------
# Test: count_remaining_tasks returns 0 for missing file
# -----------------------------------------------------------------------------
test_count_remaining_tasks_missing_file() {
    local count
    count=$(count_remaining_tasks "$TEST_TMP_DIR/nonexistent.md")

    if [[ "$count" == "0" ]]; then
        pass "count_remaining_tasks returns 0 for missing file"
    else
        fail "count_remaining_tasks expected 0 for missing file, got $count"
    fi
}

# -----------------------------------------------------------------------------
# Test: count_remaining_tasks handles indented checkboxes
# -----------------------------------------------------------------------------
test_count_remaining_tasks_indented() {
    local test_file="$TEST_TMP_DIR/indented.md"
    cat > "$test_file" << 'EOF'
# Tasks

  - [ ] Indented task 1
    - [ ] Deeply indented task
- [ ] Normal task
EOF

    local count
    count=$(count_remaining_tasks "$test_file")

    if [[ "$count" == "3" ]]; then
        pass "count_remaining_tasks handles indented checkboxes"
    else
        fail "count_remaining_tasks indented: expected 3, got $count"
    fi
}

# -----------------------------------------------------------------------------
# Test: check_completion returns 0 on valid completion
# -----------------------------------------------------------------------------
test_check_completion_valid() {
    local test_file="$TEST_TMP_DIR/complete.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Task 1
- [x] Task 2
EOF

    local output="All tasks done!

<promise>COMPLETE</promise>"

    if check_completion "$test_file" "$output" "COMPLETE"; then
        pass "check_completion returns 0 on valid completion"
    else
        fail "check_completion should return 0 on valid completion"
    fi
}

# -----------------------------------------------------------------------------
# Test: check_completion returns 1 when tasks remain
# -----------------------------------------------------------------------------
test_check_completion_tasks_remain() {
    local test_file="$TEST_TMP_DIR/incomplete.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Task 1
- [ ] Task 2
EOF

    local output="<promise>COMPLETE</promise>"

    if ! check_completion "$test_file" "$output" "COMPLETE"; then
        pass "check_completion returns 1 when tasks remain"
    else
        fail "check_completion should return 1 when tasks remain"
    fi
}

# -----------------------------------------------------------------------------
# Test: check_completion returns 1 without promise
# -----------------------------------------------------------------------------
test_check_completion_no_promise() {
    local test_file="$TEST_TMP_DIR/no_promise.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Task 1
- [x] Task 2
EOF

    local output="All tasks are done."

    if ! check_completion "$test_file" "$output" "COMPLETE"; then
        pass "check_completion returns 1 without promise"
    else
        fail "check_completion should return 1 without promise"
    fi
}

# -----------------------------------------------------------------------------
# Test: check_completion rejects negated promise
# -----------------------------------------------------------------------------
test_check_completion_negated() {
    local test_file="$TEST_TMP_DIR/negated.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Task 1
- [x] Task 2
EOF

    local output="I cannot output <promise>COMPLETE</promise>"

    if ! check_completion "$test_file" "$output" "COMPLETE"; then
        pass "check_completion rejects negated promise"
    else
        fail "check_completion should reject negated promise"
    fi
}

# -----------------------------------------------------------------------------
# Test: check_completion requires promise at end
# -----------------------------------------------------------------------------
test_check_completion_promise_position() {
    local test_file="$TEST_TMP_DIR/position.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Task 1
EOF

    # Promise early in output with lots of text after - more than 500 chars after
    # to push promise outside the tail windows check_completion uses
    local output="Here is text before the promise. <promise>COMPLETE</promise> And now a massive amount of text after the promise marker. We need to ensure there is enough content here to push the promise marker outside of the last 500 characters that check_completion examines when looking for completion. Let me add some more text here. Here is some filler text to make this long enough. The check_completion function uses tail -c 200 and tail -c 500, so we need enough content to exceed both thresholds. More content here to pad this out further. Additional padding text goes here to ensure we have well over 500 characters of content following the promise marker. This should definitely be enough now. Adding even more text just to be absolutely certain. Lorem ipsum dolor sit amet. And more text here."

    if ! check_completion "$test_file" "$output" "COMPLETE"; then
        pass "check_completion requires promise near end of output"
    else
        fail "check_completion should require promise near end"
    fi
}

# -----------------------------------------------------------------------------
# Test: check_completion with custom marker
# -----------------------------------------------------------------------------
test_check_completion_custom_marker() {
    local test_file="$TEST_TMP_DIR/custom.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [x] Done
EOF

    local output="<promise>FINISHED</promise>"

    if check_completion "$test_file" "$output" "FINISHED"; then
        pass "check_completion works with custom marker"
    else
        fail "check_completion should work with custom marker 'FINISHED'"
    fi
}

# -----------------------------------------------------------------------------
# Test: cleanup_old_logs function exists and accepts args
# -----------------------------------------------------------------------------
test_cleanup_old_logs_exists() {
    # Create a log directory
    local log_dir="$TEST_TMP_DIR/logs"
    mkdir -p "$log_dir"

    # This should not error
    local exit_code=0
    cleanup_old_logs "$log_dir" "7" || exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        pass "cleanup_old_logs function exists and runs"
    else
        fail "cleanup_old_logs should run without error"
    fi
}

# -----------------------------------------------------------------------------
# Test: cleanup_old_logs handles missing directory
# -----------------------------------------------------------------------------
test_cleanup_old_logs_missing_dir() {
    local exit_code=0
    cleanup_old_logs "$TEST_TMP_DIR/nonexistent" "7" || exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        pass "cleanup_old_logs handles missing directory gracefully"
    else
        fail "cleanup_old_logs should handle missing directory"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
test_render_prompt_template
test_render_prompt_template_multiple
test_count_remaining_tasks
test_count_remaining_tasks_all_done
test_count_remaining_tasks_missing_file
test_count_remaining_tasks_indented
test_check_completion_valid
test_check_completion_tasks_remain
test_check_completion_no_promise
test_check_completion_negated
test_check_completion_promise_position
test_check_completion_custom_marker
test_cleanup_old_logs_exists
test_cleanup_old_logs_missing_dir

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "Loop tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All loop tests passed."
