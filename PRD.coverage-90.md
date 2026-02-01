# Project Requirements Document: Coverage 90%

## Overview

Increase test coverage of the gralph Rust CLI to at least 90% by adding targeted tests for low-coverage modules and verifying results with the configured tarpaulin command.

## Problem Statement

- Coverage is currently below the 90% target, leaving critical paths under-tested.
- Core loop, verifier, and server paths include error handling and edge cases without test coverage.

## Solution

Add focused tests in the modules with the largest coverage gaps, then verify coverage using the existing tarpaulin configuration and record the result.

---

## Functional Requirements

### FR-1: Loop Session Coverage

Add tests for remaining low-coverage paths in loop session handling.

### FR-2: Verifier Coverage

Add tests for verifier parsing, static checks, and error paths.

### FR-3: Server Coverage

Add tests for server configuration and request handling edge cases.

---

## Non-Functional Requirements

### NFR-1: Deterministic Tests

- Tests must be deterministic and not require network access.

### NFR-2: Coverage Threshold

- Tarpaulin coverage must be >= 90% using the configured exclusions.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task COV90-LOOP-1

- **ID** COV90-LOOP-1
- **Context Bundle** `src/app/loop_session.rs`, `src/app.rs`
- **DoD** Tests cover remaining low-coverage loop session paths, including error handling and edge cases.
- **Checklist**
  * Add tests for tmux error propagation and session collision handling.
  * Add tests for notification and status formatting edge cases.
  * Ensure new tests exercise uncovered branches reported by tarpaulin.
- **Dependencies** None
- [ ] COV90-LOOP-1 Add loop session coverage tests

---

### Task COV90-VER-1

- **ID** COV90-VER-1
- **Context Bundle** `src/verifier.rs`, `src/app.rs`
- **DoD** Tests cover verifier parsing and static checks with failure and success cases.
- **Checklist**
  * Add tests for coverage parsing edge cases.
  * Add tests for static check filtering and sorting behavior.
  * Cover verifier error paths that are currently untested.
- **Dependencies** COV90-LOOP-1
- [ ] COV90-VER-1 Add verifier parsing and static check tests

---

### Task COV90-SRV-1

- **ID** COV90-SRV-1
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** Tests cover server configuration and request handling edge cases.
- **Checklist**
  * Add tests for server configuration defaults and overrides.
  * Cover error handling for invalid inputs or missing configuration.
  * Ensure new tests increase server coverage above 90%.
- **Dependencies** None
- [ ] COV90-SRV-1 Add server configuration tests

---

### Task COV90-FINAL-1

- **ID** COV90-FINAL-1
- **Context Bundle** `CHANGELOG.md`, `PROCESS.md`, `config/default.yaml`
- **DoD** Tarpaulin and tests run successfully with coverage >= 90% and results recorded.
- **Checklist**
  * Run `cargo test --workspace`.
  * Run the configured tarpaulin command and confirm >= 90%.
  * Add a verification note in CHANGELOG.md.
- **Dependencies** COV90-LOOP-1, COV90-VER-1, COV90-SRV-1
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
