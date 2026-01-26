#!/usr/bin/env bash
# task-block-test.sh - Tests for task block extraction and fallback
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

echo "Running task block tests..."
echo "Test tmp dir: $TEST_TMP_DIR"
echo ""

# -----------------------------------------------------------------------------
# Test: get_next_unchecked_task_block selects first block with unchecked line
# -----------------------------------------------------------------------------
test_task_block_selection() {
    local test_file="$TEST_TMP_DIR/task-blocks.md"
    cat > "$test_file" << 'EOF'
# PRD

### Task P-1
- [x] Done task
- Notes about P-1

### Task P-2
- [ ] Pending task
- Details about P-2
EOF

    local result
    result=$(get_next_unchecked_task_block "$test_file")

    local expected
    expected=$'### Task P-2\n- [ ] Pending task\n- Details about P-2'

    if [[ "$result" == "$expected" ]]; then
        pass "get_next_unchecked_task_block returns first unchecked block"
    else
        fail "task block selection mismatch"
    fi
}

# -----------------------------------------------------------------------------
# Test: fallback uses first unchecked line when no task headers
# -----------------------------------------------------------------------------
test_task_block_fallback_no_headers() {
    local test_file="$TEST_TMP_DIR/no-headers.md"
    cat > "$test_file" << 'EOF'
# Tasks

- [ ] First unchecked
- [ ] Second unchecked
EOF

    local task_block
    task_block=$(get_next_unchecked_task_block "$test_file")

    local fallback_line=""
    if [[ -z "$task_block" ]]; then
        local remaining_tasks
        remaining_tasks=$(count_remaining_tasks "$test_file")
        if [[ "$remaining_tasks" -gt 0 ]]; then
            fallback_line=$(grep -m 1 -E '^\s*- \[ \]' "$test_file" 2>/dev/null || true)
        fi
    fi

    if [[ "$fallback_line" == "- [ ] First unchecked" ]]; then
        pass "fallback uses first unchecked line without headers"
    else
        fail "fallback mismatch: expected first unchecked line"
    fi
}

# -----------------------------------------------------------------------------
# Test: task blocks stop at --- or ##
# -----------------------------------------------------------------------------
test_task_block_boundaries() {
    local test_file="$TEST_TMP_DIR/task-block-boundaries.md"
    cat > "$test_file" << 'EOF'
# PRD

### Task B-1
- [ ] First task
- Details for B-1
---

## Notes
This should not be part of any task block.

### Task B-2
- [ ] Second task
- Details for B-2
## Next Section
More notes
EOF

    local blocks=()
    local block
    while IFS= read -r -d '' block; do
        blocks+=("$block")
    done < <(get_task_blocks "$test_file")

    if [[ ${#blocks[@]} -eq 2 ]] && [[ "${blocks[0]}" == *"### Task B-1"* ]] && [[ "${blocks[0]}" != *"## Notes"* ]] && [[ "${blocks[1]}" == *"### Task B-2"* ]] && [[ "${blocks[1]}" != *"## Next Section"* ]]; then
        pass "task blocks stop at --- or ##"
    else
        fail "task block boundary detection failed"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
test_task_block_selection
test_task_block_fallback_no_headers
test_task_block_boundaries

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "Task block tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All task block tests passed."
