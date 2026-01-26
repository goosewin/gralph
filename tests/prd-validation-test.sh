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
    cat > "$test_file" << 'EOF'
# PRD

### Task D-1
- **ID** D-1
- **Context Bundle** `lib/`
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
    cat > "$test_file" << 'EOF'
# PRD

### Task D-2
- **ID** D-2
- **Context Bundle** `lib/`
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
    cat > "$test_file" << 'EOF'
# PRD

### Task D-3
- **ID** D-3
- **Context Bundle** `lib/`
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
# Run all tests
# -----------------------------------------------------------------------------
test_prd_valid
test_prd_missing_field
test_prd_multiple_unchecked

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
