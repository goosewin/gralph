# Project Requirements Document (Template)

## Overview

Raise Rust test coverage in gralph from 69.95 percent to 90 percent by focusing first on the highest risk core modules and then expanding to the rest of the codebase. Primary users are gralph maintainers and contributors who need higher confidence in loop execution, PRD parsing, state persistence, and verifier gating.

## Problem Statement

- Coverage is below the 90 percent bar while core logic (loop orchestration, PRD validation, state storage, verifier pipeline) carries high correctness risk.
- Error paths and invariants are not consistently tested, increasing the chance of silent failures in state files, PRD validation, and backend execution.
- Coverage must be treated as a signal, not a merge blocker, while correctness of core logic is improved first.

## Solution

Prioritize the top 20 percent of critical modules: `src/core.rs`, `src/prd.rs`, `src/state.rs`, and `src/verifier.rs`. Add tests around public APIs and error paths, and use property-based tests for invariants. After the core stabilizes, expand coverage to remaining modules without chasing trivial getters or glue code. Maintain non-blocking coverage warnings (soft target 65 to 70 percent), and later raise the warning bar to 75 to 80 percent once the codebase stabilizes.

---

## Functional Requirements

### FR-1: Core Correctness Coverage

Increase coverage to 90 percent by focusing first on core logic modules (loop orchestration, PRD parsing, state persistence, verifier gating) with tests that stress error paths and invariants.

### FR-2: Coverage Signal Management

Keep coverage non-blocking during the ramp-up. Use a warning-only soft target (65 to 70 percent) and plan a later increase to 75 to 80 percent after stability improves.

---

## Non-Functional Requirements

### NFR-1: Performance

- Property-based tests must be bounded to keep `cargo test --workspace` within reasonable CI runtimes.

### NFR-2: Reliability

- Tests must be deterministic, avoid external network calls, and use `ENV_LOCK` for env var mutation per `PROCESS.md`.

---

## Implementation Tasks

### Task COV90-CORE-1

- **ID** COV90-CORE-1
- **Context Bundle** `src/core.rs`, `ARCHITECTURE.md`, `PROCESS.md`
- **DoD** Add unit and property-based tests that cover `run_iteration` and `run_loop` error paths, completion promise invariants, and max-iteration exit handling without refactoring core logic.
- **Checklist**
  * Add tests for `run_iteration` input validation and empty-output error branches with raw output logging.
  * Add tests for `run_loop` max-iteration exit path and callback updates on failure.
  * Add a property-based test for completion promise invariants (last non-empty line and negation handling).
- **Dependencies** None
- [x] COV90-CORE-1 Add core loop tests for error paths and invariants
### Task COV90-PRD-1

- **ID** COV90-PRD-1
- **Context Bundle** `src/prd.rs`, `PRD.template.md`, `README.md`
- **DoD** Add tests that cover PRD validation and sanitization edge cases, including context bundle path rules and checkbox invariants.
- **Checklist**
  * Add tests for `validate_task_block` with absolute paths inside and outside repo base.
  * Add property-based tests that `sanitize_task_block` keeps exactly one unchecked task line.
  * Add tests for context bundle fallback selection when entries are invalid.
- **Dependencies** None
- [ ] COV90-PRD-1 Expand PRD validation and sanitization coverage
### Task COV90-STATE-1

- **ID** COV90-STATE-1
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`, `PROCESS.md`
- **DoD** Add tests for lock acquisition failure modes, state read/write error paths, and stale-session cleanup behavior.
- **Checklist**
  * Add tests for `with_lock` timeout and invalid lock path handling.
  * Add tests for `write_state` failure modes and `validate_state_content` empty content rejection.
  * Add tests for `cleanup_stale` with malformed session entries and dead pid detection.
- **Dependencies** None
- [ ] COV90-STATE-1 Expand state store error-path and cleanup coverage
### Task COV90-VERIFIER-1

- **ID** COV90-VERIFIER-1
- **Context Bundle** `src/main.rs`, `config/default.yaml`, `PROCESS.md`
- **DoD** Add tests for coverage parsing, review gate evaluation, static check detection, and PR creation error handling in the verifier pipeline.
- **Checklist**
  * Add tests for `extract_coverage_percent` with multiple tarpaulin output formats and fallbacks.
  * Add tests for review gate decisions (pending, failed, passed) based on rating and issues.
  * Add tests for static check settings parsing and failure reporting.
- **Dependencies** None
- [ ] COV90-VERIFIER-1 Expand verifier parsing and review gate coverage
### Task COV90-CONFIG-1

- **ID** COV90-CONFIG-1
- **Context Bundle** `src/config.rs`, `config/default.yaml`, `README.md`
- **DoD** Add tests for config merge precedence, env override behavior, and list rendering edge cases.
- **Checklist**
  * Add tests for `merge_values` with nested mappings and override precedence.
  * Add tests for env overrides with empty values and legacy compatibility keys.
  * Add tests for sequence rendering and null handling in `value_to_string`.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-CONFIG-1 Expand config precedence and rendering coverage
### Task COV90-TASK-1

- **ID** COV90-TASK-1
- **Context Bundle** `src/core.rs`, `src/prd.rs`, `ARCHITECTURE.md`
- **DoD** Add tests that deepen coverage of task block parsing rules and separators without changing parsing behavior.
- **Checklist**
  * Add property-based tests for CRLF and near-miss separator handling in `task_blocks_from_contents`.
  * Add tests for `is_task_block_end` with tabbed headings and spacing near misses.
  * Add tests for unchecked checkbox detection with mixed whitespace.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-TASK-1 Add task parsing invariant tests
### Task COV90-MAIN-1

- **ID** COV90-MAIN-1
- **Context Bundle** `src/main.rs`, `PROCESS.md`, `README.md`
- **DoD** Add tests for main CLI helpers covering session naming, worktree branch naming, and auto-worktree skip logic.
- **Checklist**
  * Add tests for `session_name` and `sanitize_session_name` fallbacks and character filtering.
  * Add tests for `auto_worktree_branch_name` and `ensure_unique_worktree_branch` branch collisions.
  * Add tests for `parse_bool_value` and `resolve_auto_worktree` config overrides.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-MAIN-1 Expand main CLI helper coverage
### Task COV90-CLI-1

- **ID** COV90-CLI-1
- **Context Bundle** `src/cli.rs`, `README.md`
- **DoD** Add clap parsing tests for run-loop, verifier, and PRD interactive flag conflicts.
- **Checklist**
  * Add tests for `RunLoopArgs` parsing with optional flags.
  * Add tests for `VerifierArgs` overrides and defaults.
  * Add tests for `PrdCreateArgs` interactive and no-interactive conflict handling.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-CLI-1 Expand CLI parsing coverage
### Task COV90-LIB-1

- **ID** COV90-LIB-1
- **Context Bundle** `src/lib.rs`, `ARCHITECTURE.md`
- **DoD** Add a minimal test module in `src/lib.rs` to exercise crate wiring and ensure module entry points are reachable.
- **Checklist**
  * Add a test that calls a small public API (for example `backend::backend_from_name`) through the lib crate.
  * Add a test that ensures the lib crate compiles and exposes expected modules.
  * Avoid new public APIs or refactors.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-LIB-1 Add lib crate wiring coverage
### Task COV90-VERSION-1

- **ID** COV90-VERSION-1
- **Context Bundle** `Cargo.toml`, `src/main.rs`
- **DoD** Add tests for version constants to ensure formatting and consistency with the package version.
- **Checklist**
  * Add a test that `VERSION_TAG` equals `format!("v{}", VERSION)`.
  * Add a test that `VERSION` matches `env!("CARGO_PKG_VERSION")` and parses as a semantic version.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-VERSION-1 Add version constant tests
### Task COV90-NOTIFY-1

- **ID** COV90-NOTIFY-1
- **Context Bundle** `src/notify.rs`, `README.md`, `config/default.yaml`
- **DoD** Add tests for remaining notification formatting and failure branches without altering webhook behavior.
- **Checklist**
  * Add tests for `notify_failed` reason mapping when reason is unknown or empty.
  * Add tests for `send_webhook` error handling when payload is empty or timeout is zero.
  * Add tests for webhook type detection on case-insensitive URLs.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-NOTIFY-1 Expand notification error-path coverage
### Task COV90-SERVER-1

- **ID** COV90-SERVER-1
- **Context Bundle** `src/server.rs`, `PROCESS.md`, `README.md`
- **DoD** Add tests for server auth, CORS resolution, and session enrichment edge cases.
- **Checklist**
  * Add tests for `resolve_cors_origin` with wildcard host and explicit host matches.
  * Add tests for `enrich_session` when task file is missing or pid is stale.
  * Add tests for stop handler error responses when session is missing.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-SERVER-1 Expand server edge-case coverage
### Task COV90-BACKEND-MOD-1

- **ID** COV90-BACKEND-MOD-1
- **Context Bundle** `src/backend/mod.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for backend utility helpers, including PATH detection and streaming error propagation.
- **Checklist**
  * Add tests for `command_in_path` with empty PATH, relative entries, and non-directory entries.
  * Add tests for `stream_command_output` when the callback fails mid-stream.
  * Add tests for missing stdout or stderr handles in child processes.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-BACKEND-MOD-1 Expand backend helper coverage
### Task COV90-BACKEND-CLAUDE-1

- **ID** COV90-BACKEND-CLAUDE-1
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`, `README.md`
- **DoD** Add tests for Claude backend argument ordering, parse-text fallbacks, and malformed stream handling.
- **Checklist**
  * Add tests for `run_iteration` argument ordering and model flag placement.
  * Add tests for `parse_text` fallback to raw contents when no result entries exist.
  * Add tests for malformed stream entries in `extract_assistant_texts`.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-BACKEND-CLAUDE-1 Expand Claude backend coverage
### Task COV90-BACKEND-OPENCODE-1

- **ID** COV90-BACKEND-OPENCODE-1
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`, `README.md`
- **DoD** Add tests for OpenCode backend env flags, argument ordering, and error handling.
- **Checklist**
  * Add tests that `OPENCODE_EXPERIMENTAL_LSP_TOOL` is set and captured.
  * Add tests for model and variant ordering and skipping empty values.
  * Add tests for non-zero exit propagation.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-BACKEND-OPENCODE-1 Expand OpenCode backend coverage
### Task COV90-BACKEND-GEMINI-1

- **ID** COV90-BACKEND-GEMINI-1
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`, `README.md`
- **DoD** Add tests for Gemini backend flag ordering, model handling, and parse-text error paths.
- **Checklist**
  * Add tests for `--headless` placement and prompt ordering.
  * Add tests for skipping empty model values.
  * Add tests for invalid UTF-8 handling in `parse_text`.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-BACKEND-GEMINI-1 Expand Gemini backend coverage
### Task COV90-BACKEND-CODEX-1

- **ID** COV90-BACKEND-CODEX-1
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`, `README.md`
- **DoD** Add tests for Codex backend flags, model handling, and error propagation.
- **Checklist**
  * Add tests for `--quiet --auto-approve` ordering and prompt placement.
  * Add tests for skipping empty model values and invalid UTF-8 handling.
  * Add tests for non-zero exit propagation.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-BACKEND-CODEX-1 Expand Codex backend coverage
### Task COV90-UPDATE-1

- **ID** COV90-UPDATE-1
- **Context Bundle** `src/main.rs`, `README.md`, `PROCESS.md`
- **DoD** Add tests for update flow error handling, version parsing, and archive extraction edge cases.
- **Checklist**
  * Add tests for `resolve_install_version` and invalid version formats.
  * Add tests for `extract_archive` error handling when tar fails or PATH is empty.
  * Add tests for `install_binary` permission-denied behavior.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-UPDATE-1 Expand update workflow coverage
### Task COV90-TESTSUPPORT-1

- **ID** COV90-TESTSUPPORT-1
- **Context Bundle** `PROCESS.md`, `src/config.rs`
- **DoD** Add tests to harden `env_lock` behavior under contention and recovery without modifying its API.
- **Checklist**
  * Add a test verifying env restore behavior after dropping the guard.
  * Add a contention test to ensure only one holder mutates env at a time.
  * Add a recovery test after a poisoned lock to ensure subsequent guards work.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-TESTSUPPORT-1 Expand env_lock test coverage
### Task COV90-CI-1

- **ID** COV90-CI-1
- **Context Bundle** `config/default.yaml`, `README.md`, `PROCESS.md`
- **DoD** Ensure the soft coverage warning target is set to 65 to 70 percent and documented as warning-only.
- **Checklist**
  * Verify `verifier.coverage_warn` is 70 and does not change `verifier.coverage_min`.
  * Update README text to match the soft warning target and non-blocking behavior.
  * Note the staged target plan in README without blocking merges.
- **Dependencies** COV90-CORE-1, COV90-PRD-1, COV90-STATE-1, COV90-VERIFIER-1
- [ ] COV90-CI-1 Align soft coverage warning target and docs
### Task COV90-CI-2

- **ID** COV90-CI-2
- **Context Bundle** `config/default.yaml`, `README.md`, `PROCESS.md`
- **DoD** After sustained stability, raise the soft coverage warning target to 75 to 80 percent and document the change as warning-only.
- **Checklist**
  * Increase `verifier.coverage_warn` to 75 or 80 only after coverage stabilizes.
  * Update README to reflect the new warning target and staged policy.
  * Keep `verifier.coverage_min` unchanged to avoid blocking merges.
- **Dependencies** COV90-CI-1
- [ ] COV90-CI-2 Raise soft coverage warning target after stabilization
---

## Success Criteria

- Overall Rust test coverage reaches 90 percent with emphasis on core logic correctness.
- Core modules (`src/core.rs`, `src/prd.rs`, `src/state.rs`, `src/verifier.rs`) have property-based tests for invariants and expanded error-path coverage.
- Public APIs and error paths are covered without adding tests for trivial getters or glue code.
- Soft coverage warning target is staged at 65 to 70 percent and later raised to 75 to 80 percent, with warnings only and no merge blocking.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
