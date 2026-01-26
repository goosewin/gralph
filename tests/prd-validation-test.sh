#!/usr/bin/env bash
# prd-validation-test.sh - Tests for PRD validation
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

cleanup() {
    rm -rf "$TEST_TMP_DIR"
}
trap cleanup EXIT

# Source PRD module
# shellcheck source=../lib/prd.sh
source "$ROOT_DIR/lib/prd.sh"

echo "Running PRD validation tests..."
echo "Test tmp dir: $TEST_TMP_DIR"
echo ""

# -----------------------------------------------------------------------------
# Test: prd_validate_file accepts valid PRD
# -----------------------------------------------------------------------------
test_prd_valid() {
    local test_file="$TEST_TMP_DIR/prd-valid.md"
    mkdir -p "$TEST_TMP_DIR/lib"
    touch "$TEST_TMP_DIR/lib/context.txt"
    cat > "$test_file" << 'EOF'
# PRD

### Task D-1
- **ID** D-1
- **Context Bundle** `lib/context.txt`
- **DoD** Implement the feature.
- **Checklist**
  * Task implemented.
- **Dependencies** None
- [ ] D-1 Implement PRD validation
EOF

    if prd_validate_file "$test_file" >/dev/null 2>&1; then
        pass "prd_validate_file accepts valid PRD"
    else
        fail "prd_validate_file should accept valid PRD"
    fi
}

# -----------------------------------------------------------------------------
# Test: prd_validate_file reports missing field
# -----------------------------------------------------------------------------
test_prd_missing_field() {
    local test_file="$TEST_TMP_DIR/prd-missing-field.md"
    mkdir -p "$TEST_TMP_DIR/lib"
    touch "$TEST_TMP_DIR/lib/context.txt"
    cat > "$test_file" << 'EOF'
# PRD

### Task D-2
- **ID** D-2
- **Context Bundle** `lib/context.txt`
- **Checklist**
  * Missing DoD field.
- **Dependencies** D-1
- [ ] D-2 Missing DoD
EOF

    local output
    local exit_code=0
    output=$(prd_validate_file "$test_file" 2>&1) || exit_code=$?

    if [[ $exit_code -ne 0 ]] && [[ "$output" == *"Missing required field: DoD"* ]]; then
        pass "prd_validate_file reports missing DoD field"
    else
        fail "prd_validate_file should fail on missing DoD field"
    fi
}

# -----------------------------------------------------------------------------
# Test: prd_validate_file rejects multiple unchecked task lines
# -----------------------------------------------------------------------------
test_prd_multiple_unchecked() {
    local test_file="$TEST_TMP_DIR/prd-multiple-unchecked.md"
    mkdir -p "$TEST_TMP_DIR/lib"
    touch "$TEST_TMP_DIR/lib/context.txt"
    cat > "$test_file" << 'EOF'
# PRD

### Task D-3
- **ID** D-3
- **Context Bundle** `lib/context.txt`
- **DoD** Add strict PRD validation.
- **Checklist**
  * Validation added.
- **Dependencies** D-2
- [ ] D-3 Add strict PRD validation
- [ ] D-3 Update error handling
EOF

    local output
    local exit_code=0
    output=$(prd_validate_file "$test_file" 2>&1) || exit_code=$?

    if [[ $exit_code -ne 0 ]] && [[ "$output" == *"Multiple unchecked task lines"* ]]; then
        pass "prd_validate_file rejects multiple unchecked task lines"
    else
        fail "prd_validate_file should fail on multiple unchecked task lines"
    fi
}

# -----------------------------------------------------------------------------
# Test: prd_validate_file rejects stray unchecked checkbox
# -----------------------------------------------------------------------------
test_prd_stray_checkbox() {
    local test_file="$TEST_TMP_DIR/prd-stray-checkbox.md"
    mkdir -p "$TEST_TMP_DIR/context"
    touch "$TEST_TMP_DIR/context/valid.txt"
    cat > "$test_file" << 'EOF'
# PRD

- [ ] Stray unchecked outside task block

### Task D-4
- **ID** D-4
- **Context Bundle** `context/valid.txt`
- **DoD** Fix validation.
- **Checklist**
  * Add guard.
- **Dependencies** None
- [ ] D-4 Add guard
EOF

    local output
    local exit_code=0
    output=$(prd_validate_file "$test_file" 2>&1) || exit_code=$?

    if [[ $exit_code -ne 0 ]] && [[ "$output" == *"Unchecked task line outside task block"* ]]; then
        pass "prd_validate_file rejects stray unchecked checkbox"
    else
        fail "prd_validate_file should reject stray unchecked checkbox"
    fi
}

# -----------------------------------------------------------------------------
# Test: prd_validate_file rejects missing context bundle path
# -----------------------------------------------------------------------------
test_prd_missing_context() {
    local test_file="$TEST_TMP_DIR/prd-missing-context.md"
    cat > "$test_file" << 'EOF'
# PRD

### Task D-5
- **ID** D-5
- **Context Bundle** `missing/file.txt`
- **DoD** Ensure context exists.
- **Checklist**
  * Validation fails.
- **Dependencies** None
- [ ] D-5 Missing context
EOF

    local output
    local exit_code=0
    output=$(prd_validate_file "$test_file" 2>&1) || exit_code=$?

    if [[ $exit_code -ne 0 ]] && [[ "$output" == *"Context Bundle path not found"* ]]; then
        pass "prd_validate_file rejects missing context bundle path"
    else
        fail "prd_validate_file should reject missing context bundle path"
    fi
}

# -----------------------------------------------------------------------------
# Test: prd_validate_file allows missing context with flag
# -----------------------------------------------------------------------------
test_prd_allow_missing_context() {
    local test_file="$TEST_TMP_DIR/prd-allow-missing-context.md"
    cat > "$test_file" << 'EOF'
# PRD

### Task D-6
- **ID** D-6
- **Context Bundle** `missing/ok.txt`
- **DoD** Skip context validation.
- **Checklist**
  * Validation passes.
- **Dependencies** None
- [ ] D-6 Allow missing context
EOF

    if prd_validate_file "$test_file" "true" >/dev/null 2>&1; then
        pass "prd_validate_file allows missing context with flag"
    else
        fail "prd_validate_file should allow missing context with flag"
    fi
}

# -----------------------------------------------------------------------------
# Test: stack detection ambiguity
# -----------------------------------------------------------------------------
test_stack_detection_ambiguity() {
    local stack_dir="$TEST_TMP_DIR/stack-detect"
    mkdir -p "$stack_dir"
    cat > "$stack_dir/package.json" << 'EOF'
{
  "name": "stack-detect",
  "version": "1.0.0"
}
EOF
    cat > "$stack_dir/go.mod" << 'EOF'
module example.com/stack

go 1.21
EOF

    prd_detect_stack "$stack_dir"

    if [[ ${#PRD_STACK_IDS[@]} -gt 1 ]] && [[ " ${PRD_STACK_IDS[*]} " == *"Node.js"* ]] && [[ " ${PRD_STACK_IDS[*]} " == *"Go"* ]]; then
        pass "prd_detect_stack identifies multiple stacks"
    else
        fail "prd_detect_stack should identify multiple stacks"
    fi
}

# -----------------------------------------------------------------------------
# Run all tests
# -----------------------------------------------------------------------------
test_prd_valid
test_prd_missing_field
test_prd_multiple_unchecked
test_prd_stray_checkbox
test_prd_missing_context
test_prd_allow_missing_context
test_stack_detection_ambiguity

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "========================================"
echo "PRD validation tests completed"
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo "All PRD validation tests passed."
