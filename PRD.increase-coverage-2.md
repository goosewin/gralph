# Project Requirements Document (Template)

## Overview

Raise Rust test coverage from about 60% to at least 90% for the gralph CLI by expanding unit tests around PRD validation, state persistence, update checks, and the verifier pipeline. This targets maintainers and contributors running `cargo test --workspace` and coverage gates defined in the repo.

## Problem Statement

- Current coverage is below the 90% gate required by the verifier and process documentation, risking CI failure and violating R-004 in the risk register.
- The logic in PRD validation, state storage, update handling, and verifier parsing has untested edge cases that can regress silently.

## Solution

Add focused, deterministic unit tests in the Rust modules handling PRD validation, state persistence, update behavior, and verifier parsing. Keep tests simple and local (no network, no external gh/tar execution), align with existing config defaults, and ensure coverage meets the repository gate.

---

## Functional Requirements

### FR-1: PRD Validation Coverage

Add tests that exercise PRD validation, sanitization, context bundle enforcement, and stack detection paths so that `src/prd.rs` is covered beyond happy paths.

### FR-2: State Store Coverage

Add tests that cover lock contention, invalid inputs, atomic state writes, and stale cleanup behavior so that `src/state.rs` reaches coverage parity with core usage.

### FR-3: Update and Verifier Coverage

Add tests that cover update version parsing and verifier parsing/gate logic without external commands, improving coverage for `src/update.rs` and `src/verifier.rs`.

---

## Non-Functional Requirements

### NFR-1: Performance

- Tests complete quickly and avoid long sleeps or network calls.

### NFR-2: Reliability

- Tests are deterministic, use temp directories, and avoid reliance on external tools or services.

### NFR-3: Maintainability

- Apply KISS, YAGNI, DRY, SOLID, and TDA by keeping test helpers minimal, avoiding over-abstraction, and mirroring existing patterns.

---

## Implementation Tasks

### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/prd.rs`, `PRD.template.md`, `ARCHITECTURE.md`
- **DoD** New unit tests in `src/prd.rs` cover validation errors, sanitization filtering, and context bundle handling, and all tests pass.
- **Checklist**
  * Add tests for context bundle filtering and fallback behavior in sanitization.
  * Add tests for validation errors around missing fields, empty task files, and Open Questions removal.
- **Dependencies** None
- [x] COV-7 Expand PRD validation and sanitization coverage
### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`, `RISK_REGISTER.md`
- **DoD** New unit tests in `src/state.rs` cover lock timeout, invalid session inputs, and atomic write safety, and all tests pass.
- **Checklist**
  * Add tests for lock contention and timeout handling using short timeouts.
  * Add tests for invalid input paths and state write guardrails.
- **Dependencies** None
- [x] COV-8 Expand state store edge case coverage
### Task COV-9

- **ID** COV-9
- **Context Bundle** `src/main.rs`, `README.md`, `ARCHITECTURE.md`
- **DoD** New unit tests in `src/update.rs` cover version parsing and normalization paths without network calls, and all tests pass.
- **Checklist**
  * Add tests for `Version::parse` and normalization error paths using invalid inputs.
  * Add tests for `resolve_install_version` when a concrete version string is provided.
- **Dependencies** None
- [x] COV-9 Expand update module parsing coverage
### Task COV-10

- **ID** COV-10
- **Context Bundle** `src/main.rs`, `config/default.yaml`, `PROCESS.md`
- **DoD** New unit tests in `src/verifier.rs` cover coverage parsing, review gate parsing, and static check helpers without invoking external commands, and all tests pass.
- **Checklist**
  * Add tests for coverage percent extraction and percent parsing from representative output lines.
  * Add tests for review gate parsing helpers and static check path matching logic.
- **Dependencies** None
- [x] COV-10 Expand verifier parsing and static check coverage
### Task COV-11

- **ID** COV-11
- **Context Bundle** `CHANGELOG.md`, `PROCESS.md`, `config/default.yaml`, `ARCHITECTURE.md`
- **DoD** CHANGELOG updated with COV task entries and a verification line reflecting test and coverage commands; coverage meets or exceeds 90%.
- **Checklist**
  * Update `CHANGELOG.md` with new COV entries and verification note.
  * Run `cargo test --workspace` and `cargo tarpaulin --workspace --fail-under 90 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`.
- **Dependencies** COV-7, COV-8, COV-9, COV-10
- [ ] COV-11 Verify coverage and record changelog
---

## Success Criteria

- Total coverage is at least 90% using the configured tarpaulin command.
- New unit tests cover previously untested paths in PRD validation, state store, update checks, and verifier parsing.
- `cargo test --workspace` passes with no flaky or network-dependent tests.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
