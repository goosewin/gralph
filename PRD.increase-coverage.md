# Project Requirements Document (Template)

## Overview

Improve gralph CLI test coverage from roughly 60 percent to at least 90 percent by adding targeted unit tests in the Rust codebase and aligning the verifier coverage gate. Intended users are maintainers and CI/verifier workflows that rely on `cargo test` and tarpaulin.

## Problem Statement

- Coverage is currently below the 90 percent bar defined in PROCESS and ARCHITECTURE.
- Error paths and edge cases in PRD validation, config loading, state management, and CLI parsing are under-tested.
- The default verifier coverage command allows a lower threshold, weakening enforcement.

## Solution

Add focused unit tests for PRD validation/sanitization, config resolution, state lifecycle, and CLI parsing while keeping runtime behavior unchanged. Align the default coverage command to enforce a 90 percent threshold and verify with the existing tarpaulin workflow. Follow KISS, YAGNI, DRY, SOLID, and keep tests deterministic.

---

## Functional Requirements

### FR-1: PRD Validation Coverage

The test suite covers PRD validation, sanitization, and stack detection edge cases so error branches are exercised.

### FR-2: Config, State, and CLI Coverage

The test suite covers configuration resolution and overrides, state store lifecycle and error handling, and CLI argument parsing for major commands and flags.

---

## Non-Functional Requirements

### NFR-1: Performance

- Tests remain unit-level and run under `cargo test --workspace` without external services.

### NFR-2: Reliability

- Coverage is enforced at 90 percent or higher using the tarpaulin command configured in `config/default.yaml`.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/prd.rs`, `ARCHITECTURE.md`, `PROCESS.md`
- **DoD** Add unit tests that exercise PRD validation and sanitization edge cases, including open-questions detection, stray unchecked lines, context bundle validation for absolute and out-of-repo paths, and fallback context selection.
- **Checklist**
  * Add tests for `has_open_questions_section`, `validate_stray_unchecked`, and context path validation branches.
  * Add tests that cover sanitize behavior when allowed context is empty or filtered.
- **Dependencies** None
- [x] COV-1 Expand PRD validation and sanitization coverage
### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/config.rs`, `config/default.yaml`, `PROCESS.md`
- **DoD** Add unit tests for config error handling, key normalization, env override precedence, and value rendering for sequences, tagged values, and map values.
- **Checklist**
  * Add a test for invalid YAML parse error propagation from `Config::load`.
  * Add tests for `normalize_key`, env override precedence, and `value_to_string` handling of sequences and tagged values.
- **Dependencies** None
- [ ] COV-2 Expand config loader and override coverage
### Task COV-3

- **ID** COV-3
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`, `PROCESS.md`
- **DoD** Add unit tests for state store error paths, cleanup modes, and value parsing so invalid session names and missing sessions are covered.
- **Checklist**
  * Add tests for invalid session names in get/set/delete and for delete of missing session.
  * Add tests for `cleanup_stale` with `CleanupMode::Remove` and for `parse_value` bool and numeric cases.
- **Dependencies** None
- [ ] COV-3 Expand state store error-path coverage
### Task COV-4

- **ID** COV-4
- **Context Bundle** `src/cli.rs`, `README.md`, `PROCESS.md`
- **DoD** Add CLI parsing tests that cover start, prd create, server, config, and worktree command flags and defaults.
- **Checklist**
  * Add tests for parsing `gralph prd create` options and `gralph server` flags.
  * Add tests for `--no-worktree`, `--strict-prd`, and worktree create/finish IDs.
- **Dependencies** None
- [ ] COV-4 Expand CLI parsing coverage for core commands
### Task COV-5

- **ID** COV-5
- **Context Bundle** `config/default.yaml`, `PROCESS.md`, `ARCHITECTURE.md`
- **DoD** Update the default verifier coverage command to enforce `--fail-under 90` while keeping the existing exclude list intact.
- **Checklist**
  * `verifier.coverage_command` uses `--fail-under 90` with the current excludes.
  * `verifier.coverage_min` remains 90.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-5 Align default coverage command with 90 percent gate
### Task COV-6

- **ID** COV-6
- **Context Bundle** `PROCESS.md`, `config/default.yaml`, `CHANGELOG.md`
- **DoD** Run tests and coverage with the configured commands and record verification results in CHANGELOG.
- **Checklist**
  * Run `cargo test --workspace` successfully.
  * Run `cargo tarpaulin --workspace --fail-under 90 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*` and confirm coverage >= 90 percent.
  * Update `CHANGELOG.md` verification note with commands and coverage.
- **Dependencies** COV-5
- [ ] COV-6 Verify coverage and record results
---

## Success Criteria

- `cargo test --workspace` passes and tarpaulin coverage is at least 90 percent using the configured command.
- No production behavior changes outside test additions and coverage configuration.
- Coverage enforcement in `config/default.yaml` uses `--fail-under 90`.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
