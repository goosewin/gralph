# Project Requirements Document: Increase Coverage to 90%

## Overview

Increase the test coverage of the gralph Rust CLI to at least 90% by adding targeted unit tests for uncovered paths in core modules. The primary users are maintainers of the CLI and its verification workflow.

## Problem Statement

- Coverage is below the 90% target, leaving important error paths untested.
- Key modules like core loop handling, PRD validation, and verifier checks have uncovered branches.
- The coverage gap increases regression risk for autonomous loop execution.

## Solution

Add focused unit tests across core loop handling, PRD validation/sanitization, and verifier parsing to raise coverage and maintain stability, then verify with tarpaulin and record the results.

---

## Functional Requirements

### FR-1: Core Loop Coverage

Add tests that cover core loop validation, remaining task counting, and completion checks.

### FR-2: PRD Validation Coverage

Add tests that cover PRD validation errors and sanitize behavior for generated PRDs.

### FR-3: Verifier Coverage

Add tests for static file collection and coverage parsing.

---

## Non-Functional Requirements

### NFR-1: Deterministic Tests

- Tests must be deterministic, local-only, and avoid network access.

### NFR-2: Coverage Threshold

- Tarpaulin coverage must be >= 90% using the configured exclusions.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task COV90-CORE-1

- **ID** COV90-CORE-1
- **Context Bundle** `src/core.rs`, `src/task.rs`
- **DoD** Tests cover core loop validation, remaining task counting, and completion checks.
- **Checklist**
  * Add tests for missing task file and invalid iteration inputs.
  * Cover count_remaining_tasks with header and non-header formats.
  * Cover check_completion with success and failure cases.
- **Dependencies** None
- [ ] COV90-CORE-1 Add core loop validation tests

---

### Task COV90-PRD-1

- **ID** COV90-PRD-1
- **Context Bundle** `src/prd.rs`, `src/app/prd_init.rs`
- **DoD** Tests cover PRD validation error paths and sanitize behavior.
- **Checklist**
  * Add tests for missing required fields in task blocks.
  * Cover sanitize removal of Open Questions and stray checkboxes.
  * Verify validation passes after sanitize with allowed context.
- **Dependencies** COV90-CORE-1
- [ ] COV90-PRD-1 Add PRD validation and sanitize tests

---

### Task COV90-VER-1

- **ID** COV90-VER-1
- **Context Bundle** `src/verifier.rs`, `config/default.yaml`, `CHANGELOG.md`
- **DoD** Tests cover static file collection and coverage parsing logic; changelog updated after verification.
- **Checklist**
  * Add tests for static file inclusion and ignore patterns.
  * Cover coverage parsing for tarpaulin output.
  * Record verification results in CHANGELOG.md.
- **Dependencies** COV90-PRD-1
- [ ] COV90-VER-1 Add verifier static check and coverage tests

---

### Task COV90-FINAL-1

- **ID** COV90-FINAL-1
- **Context Bundle** `CHANGELOG.md`, `PROCESS.md`, `config/default.yaml`
- **DoD** Tests and tarpaulin run cleanly with coverage >= 90% and verification note recorded.
- **Checklist**
  * Run `cargo test --workspace`.
  * Run the configured tarpaulin command and confirm >= 90%.
  * Add verification line to CHANGELOG.md.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-VER-1
- [ ] COV90-FINAL-1 Verify coverage target

---

## Success Criteria

- Tarpaulin coverage is at least 90% with configured exclusions.
- All workspace tests pass.
- CHANGELOG.md includes a verification note.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
