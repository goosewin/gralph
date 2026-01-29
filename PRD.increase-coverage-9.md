# Project Requirements Document (Template)

## Overview

Raise gralph test coverage to 90 percent by focusing first on the core loop, state store, PRD parsing, and verifier pipeline, then expanding coverage across public APIs and error paths in supporting modules and backend adapters. Coverage remains a signal, not a merge blocker, and the work prioritizes correctness of core logic over chasing line counts.

## Problem Statement

- Overall coverage is low at 63.92 percent, with large core modules under-tested.
- Error paths and state transitions are not consistently exercised, increasing risk of regressions.
- Backend adapters and CLI glue have uneven test depth, reducing confidence in runtime behavior.

## Solution

Target the top 20 percent of modules that matter most for correctness: `src/core.rs`, `src/state.rs`, `src/prd.rs`, and `src/verifier.rs`. Add tests around public APIs and error paths in the CLI, server, notify, config, and backend adapters. Prefer property-based tests for parsing invariants over many narrow unit tests. After these stabilize, introduce a soft CI coverage target at 65 to 70 percent, then later raise to 75 to 80 percent once the codebase slows down.

---

## Functional Requirements

### FR-1: Core Coverage Focus

Increase coverage in core logic modules with tests for boundary conditions, state transitions, and validation invariants.

### FR-2: Error Path and API Coverage

Expand tests around public APIs, adapter command assembly, and failure handling without refactoring purely to raise coverage.

---

## Non-Functional Requirements

### NFR-1: Performance

- Keep tests fast and deterministic, with bounded property-based cases.

### NFR-2: Reliability

- Do not block merges on coverage yet; treat coverage as a signal.
- Avoid tests that rely on external services or the real filesystem outside temp dirs.
- Env var mutation in tests must use the existing ENV_LOCK helpers.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/core.rs`, `ARCHITECTURE.md`
- **DoD** Add unit tests covering core loop boundaries and prompt rendering edge cases in `src/core.rs`, increasing coverage for core loop logic.
- **Checklist**
  * Tests cover `check_completion` negated promise phrases and last-line matching.
  * Tests cover `resolve_prompt_template` fallback ordering when env and project templates are missing.
  * Tests cover `cleanup_old_logs` behavior when log dir is missing and when retention is disabled.
- **Dependencies** None
- [x] COV-1 Expand core loop and prompt handling tests in src/core.rs
---

### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for state lock error paths and session cleanup edge cases in `src/state.rs`, increasing coverage for state management.
- **Checklist**
  * Tests cover lock acquisition failures when the lock file cannot be created.
  * Tests cover `cleanup_stale` handling of malformed session entries and remove mode.
  * Tests cover `parse_value` for tricky numeric and boolean inputs that are currently unexercised.
- **Dependencies** None
- [x] COV-2 Add state store lock and cleanup edge case tests in src/state.rs
---

### Task COV-3

- **ID** COV-3
- **Context Bundle** `src/prd.rs`, `ARCHITECTURE.md`
- **DoD** Add unit and property-based tests for PRD validation and sanitization invariants in `src/prd.rs`, increasing coverage for parsing logic.
- **Checklist**
  * Property-based tests confirm task block parsing is stable across whitespace and separator variations.
  * Tests cover context bundle validation for paths outside repo root.
  * Tests cover sanitization of context entries and removal of stray unchecked lines.
- **Dependencies** None
- [x] COV-3 Expand PRD validation and sanitization tests in src/prd.rs
---

### Task COV-4

- **ID** COV-4
- **Context Bundle** `ARCHITECTURE.md`, `config/default.yaml`, `src/main.rs`
- **DoD** Add tests for verifier pipeline command assembly and review gate parsing in `src/verifier.rs`, increasing coverage for verification orchestration.
- **Checklist**
  * Tests cover `resolve_verifier_auto_run` with missing config and explicit overrides.
  * Tests cover review gate parsing for min rating, max issues, and timeout behavior.
  * Tests cover command construction for test, coverage, and static checks without invoking external tools.
- **Dependencies** None
- [x] COV-4 Add verifier pipeline parsing and command assembly tests in src/verifier.rs
---

### Task COV-5

- **ID** COV-5
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for PATH scanning and streaming error handling in `src/backend/mod.rs`, increasing coverage for backend utilities.
- **Checklist**
  * Tests cover `command_in_path` with empty PATH segments and non-existent dirs.
  * Tests cover `stream_command_output` when child exits early and receiver closes.
  * Tests cover `BackendError` formatting for each error variant.
- **Dependencies** None
- [x] COV-5 Expand backend utility tests in src/backend/mod.rs
 - [x] COV-5 Expand backend utility tests in src/backend/mod.rs
---

### Task COV-6

- **ID** COV-6
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`
- **DoD** Add tests for Claude stream parsing and argument handling in `src/backend/claude.rs`, increasing coverage for parsing logic.
- **Checklist**
  * Tests cover `parse_text` when result entries are missing or interleaved.
  * Tests cover `extract_assistant_texts` for malformed content entries.
  * Tests cover `run_iteration` model flag behavior with empty and non-empty values.
- **Dependencies** None
- [x] COV-6 Expand Claude backend parsing and argument tests in src/backend/claude.rs
---

### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** Add tests for OpenCode env flags, argument order, and error paths in `src/backend/opencode.rs`, increasing coverage for adapter behavior.
- **Checklist**
  * Tests verify OPENCODE_EXPERIMENTAL_LSP_TOOL is set and prompt is last argument.
  * Tests cover empty model and variant handling.
  * Tests cover spawn failure error formatting and parse_text behavior for empty files.
- **Dependencies** None
- [x] COV-7 Expand OpenCode backend tests in src/backend/opencode.rs
---

### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Add tests for Gemini adapter flags and error handling in `src/backend/gemini.rs`, increasing coverage for adapter behavior.
- **Checklist**
  * Tests verify `--headless` is always passed and prompt is last argument.
  * Tests cover model flag skipping on empty values.
  * Tests cover parse_text errors for missing files and directories.
- **Dependencies** None
- [x] COV-8 Expand Gemini backend tests in src/backend/gemini.rs
---

### Task COV-9

- **ID** COV-9
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Add tests for Codex adapter flags and error paths in `src/backend/codex.rs`, increasing coverage for adapter behavior.
- **Checklist**
  * Tests verify `--quiet` and `--auto-approve` ordering with and without model.
  * Tests cover parse_text errors for missing files and directories.
  * Tests cover non-zero exit handling in run_iteration.
- **Dependencies** None
- [x] COV-9 Expand Codex backend tests in src/backend/codex.rs
---

### Task COV-10

- **ID** COV-10
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Add tests for config override precedence and key normalization in `src/config.rs`, increasing coverage for config resolution.
- **Checklist**
  * Tests cover normalized vs legacy env precedence with empty values.
  * Tests cover `normalize_key` and `lookup_value` for mixed case and hyphenated keys.
  * Tests cover list rendering for sequences and nulls.
- **Dependencies** None
- [x] COV-10 Expand config resolution and listing tests in src/config.rs
---

### Task COV-11

- **ID** COV-11
- **Context Bundle** `src/server.rs`
- **DoD** Add tests for CORS resolution and session enrichment edge cases in `src/server.rs`, increasing coverage for server behavior.
- **Checklist**
  * Tests cover `resolve_cors_origin` for open mode and host-matching origins.
  * Tests cover `check_auth` for invalid header encodings and missing bearer tokens.
  * Tests cover `enrich_session` when dir or task_file is missing and status is updated.
- **Dependencies** None
- [x] COV-11 Expand server CORS, auth, and enrichment tests in src/server.rs
---

### Task COV-12

- **ID** COV-12
- **Context Bundle** `src/notify.rs`
- **DoD** Add tests for notification formatting boundaries and webhook timeout defaults in `src/notify.rs`, increasing coverage for notification helpers.
- **Checklist**
  * Tests cover `format_duration` for large durations and None.
  * Tests cover generic payload optional fields for failure reasons.
  * Tests cover send_webhook default timeout when None or zero.
- **Dependencies** None
- [x] COV-12 Expand notification formatting and timeout tests in src/notify.rs
---

### Task COV-13

- **ID** COV-13
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `ARCHITECTURE.md`
- **DoD** Add unit tests for CLI helper logic in `src/main.rs`, increasing coverage for session naming, validation, and worktree logic.
- **Checklist**
  * Tests cover `session_name` and `sanitize_session_name` for invalid inputs.
  * Tests cover `validate_task_id` for valid and invalid formats.
  * Tests cover `resolve_log_file` fallback and `parse_bool_value` option parsing.
- **Dependencies** None
- [x] COV-13 Add CLI helper and worktree logic tests in src/main.rs
---

### Task COV-14

- **ID** COV-14
- **Context Bundle** `ARCHITECTURE.md`, `src/main.rs`
- **DoD** Add tests in `src/update.rs` for update checking and install error handling, increasing coverage for update logic.
- **Checklist**
  * Tests cover check_for_update handling of invalid version strings and error responses.
  * Tests cover install_release failure paths for missing archives or extraction errors.
  * Tests cover PATH resolution behavior when a different binary is found first.
- **Dependencies** None
- [x] COV-14 Expand update check and install error-path tests in src/update.rs
---

### Task COV-15

- **ID** COV-15
- **Context Bundle** `src/prd.rs`
- **DoD** Add unit and property-based tests for task parsing invariants in `src/task.rs`, increasing coverage for parsing helpers.
- **Checklist**
  * Property-based tests cover task block termination on separators and headings.
  * Tests cover `is_unchecked_line` handling of tabs, CRLF, and leading spaces.
  * Tests cover `is_task_header` and `is_task_block_end` with malformed headings.
- **Dependencies** None
- [ ] COV-15 Expand task parsing invariants tests in src/task.rs
---

### Task COV-16

- **ID** COV-16
- **Context Bundle** `src/lib.rs`
- **DoD** Add tests for env lock helpers in `src/test_support.rs`, increasing coverage for test utilities and safety helpers.
- **Checklist**
  * Tests verify env_lock serializes concurrent env updates.
  * Tests cover poison recovery behavior for env_lock guard.
  * Tests demonstrate safe env mutation pattern used by other tests.
- **Dependencies** None
- [ ] COV-16 Add test_support env lock safety tests in src/test_support.rs
---

## Success Criteria

- Overall coverage reaches 90 percent with `cargo tarpaulin --workspace --fail-under 90` using the existing exclusions.
- Coverage improves in each listed Rust module with tests focused on core logic and error paths.
- Core modules (`src/core.rs`, `src/state.rs`, `src/prd.rs`, `src/verifier.rs`) have explicit tests for edge cases and invariants.
- Coverage is tracked as a signal and does not block merges yet.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
