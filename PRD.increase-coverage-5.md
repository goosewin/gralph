# Project Requirements Document (Template)

## Overview

Raise automated test coverage for the gralph Rust CLI from 51.85% (2471/4766) to 90% by adding targeted tests across core logic, PRD validation, verifier gating, state management, and backend adapters. Intended users are maintainers and contributors who rely on stable loop execution and verifier quality gates. Scope is test additions only; no product behavior changes. Constraints: none beyond repository standards and existing process.

## Problem Statement

- Current coverage is 51.85% overall, with low coverage in core modules such as `src/core.rs`, `src/prd.rs`, `src/verifier.rs`, and `src/main.rs`.
- Error paths and public API behavior (loop orchestration, PRD validation, verifier gating, update/install) are under-tested, reducing confidence.
- Coverage needs to reach 90% without refactoring solely to move the number.

## Solution

Focus first on the top 20% modules that drive correctness: core loop, PRD validation, verifier pipeline, state store, server, and config loader. Add tests around public APIs and error paths, then expand to backends, notifications, update helpers, and task parsing. Introduce property-based tests for parsing invariants, avoid tests for trivial getters or glue code, and treat coverage as a signal rather than a merge blocker. After the first stability pass, add a soft CI target at 65 to 70 percent, then raise to 75 to 80 percent when the codebase slows down.

---

## Functional Requirements

### FR-1: Core Logic Coverage

Add tests for core loop iteration, completion detection, PRD validation and sanitization, state store operations, and verifier gate logic, prioritizing error paths and public APIs.

### FR-2: System and Adapter Coverage

Add tests for backend adapters, server endpoints, notifications, update/install helpers, and CLI utilities to cover configuration behavior and failure cases.

---

## Non-Functional Requirements

### NFR-1: Performance

- `cargo test --workspace` remains reasonably fast and avoids network calls or long-running processes; use local fakes and temp dirs.

### NFR-2: Reliability

- Tests must be deterministic and not depend on external services or real CLI installations.
- Do not refactor only to increase coverage; do not write tests just to move the number.
- Coverage is a signal and must not block merges yet.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/core.rs`
- **DoD** Add unit tests covering `run_iteration` error paths for empty output and empty parsed results, `check_completion` negated promise handling, and `render_prompt_template` context files section behavior.
- **Checklist**
  * Add a backend stub that writes an empty output file and assert `run_iteration` returns CoreError::InvalidInput for empty output and empty parsed text.
  * Cover `check_completion` with negated promise lines and verify `render_prompt_template` includes the context files section when provided.
- **Dependencies** None
- [x] COV-1 Expand core loop error-path and prompt rendering coverage
---

### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for `prd_validate_file` and `validate_task_block` to catch open questions, stray unchecked lines, missing required fields, and multiple unchecked lines; extend `sanitize_task_block` and context path handling tests.
- **Checklist**
  * Add validation tests that exercise `has_open_questions_section`, `validate_stray_unchecked`, and missing field errors.
  * Add sanitization tests for `sanitize_task_block`, `context_entry_exists`, and `remove_unchecked_checkbox` with allowed context lists.
- **Dependencies** None
- [x] COV-2 Expand PRD validation and sanitization coverage
---

### Task COV-3

- **ID** COV-3
- **Context Bundle** `config/default.yaml`
- **DoD** Add tests covering `extract_coverage_percent` and `parse_percent_from_line` across multiple output formats, `parse_verifier_command` error handling, and review/check gate evaluation using synthetic gh PR JSON.
- **Checklist**
  * Cover `extract_coverage_percent` with lines that include "coverage results", "line coverage", and generic "coverage".
  * Validate `evaluate_review_gate`, `evaluate_check_gate`, and `resolve_review_gate_merge_method` with synthetic JSON and invalid values without invoking gh.
- **Dependencies** None
- [x] COV-3 Expand verifier parsing and gate evaluation coverage
---

### Task COV-4

- **ID** COV-4
- **Context Bundle** `src/state.rs`
- **DoD** Add tests that cover `cleanup_stale` behavior for alive vs dead pids and edge inputs for `parse_value` and `validate_state_content`.
- **Checklist**
  * Spawn a short-lived child process and verify `cleanup_stale` does not mark sessions with live pids as stale.
  * Add edge-case tests for `parse_value` (negative and mixed strings) and `validate_state_content` (empty content rejection).
- **Dependencies** None
- [x] COV-4 Expand state store edge-case coverage
---

### Task COV-5

- **ID** COV-5
- **Context Bundle** `src/server.rs`, `src/state.rs`, `src/core.rs`
- **DoD** Add tests for `enrich_session` remaining task calculation and `stop_handler` behavior when `tmux_session` is set.
- **Checklist**
  * Create a temp task file and verify `enrich_session` reports `current_remaining` based on `task_file`.
  * Add a stop endpoint test that sets `tmux_session` and verifies the session status update path.
- **Dependencies** None
- [x] COV-5 Expand server session enrichment and stop flow coverage
---

### Task COV-6

- **ID** COV-6
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Add tests for `default_config_path` precedence and key normalization helpers such as `key_to_env`, `key_to_env_legacy`, and `normalize_segment`.
- **Checklist**
  * Validate `default_config_path` resolution order for env override, installed default, and manifest default.
  * Add unit tests for `key_to_env`, `key_to_env_legacy`, and `normalize_segment` with hyphen and case variants.
- **Dependencies** None
- [x] COV-6 Expand config path and key normalization coverage
---

### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/main.rs`, `src/cli.rs`
- **DoD** Add tests for `session_name` fallback logic, `sanitize_session_name`, `parse_bool_value`, `resolve_auto_worktree`, `auto_worktree_branch_name`, and `resolve_log_file` fallback.
- **Checklist**
  * Cover `session_name` for explicit, directory-based, and fallback names, and verify `sanitize_session_name` output.
  * Add unit tests for `parse_bool_value`, `resolve_auto_worktree`, `auto_worktree_branch_name`, and `resolve_log_file` when log path is missing.
- **Dependencies** None
- [x] COV-7 Expand CLI helper coverage in main
---

### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/notify.rs`
- **DoD** Add tests for notification payload formatting when optional values are None and for generic failure message mapping.
- **Checklist**
  * Verify `notify_complete` and `notify_failed` use "unknown" defaults when iterations or duration are missing.
  * Cover `format_generic_failed` mappings for known and custom reasons in the generated payload.
- **Dependencies** None
- [ ] COV-8 Expand notification formatting and defaults coverage
---

### Task COV-9

- **ID** COV-9
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for `check_for_update` when latest is not newer, `fetch_latest_release_tag` parsing with a local HTTP server, and archive extraction failures.
- **Checklist**
  * Use `GRALPH_TEST_LATEST_TAG` to validate `check_for_update` returns None when current is latest.
  * Add tests for `fetch_latest_release_tag` and `extract_archive` error handling using local test fixtures.
- **Dependencies** None
- [ ] COV-9 Expand update check and archive error coverage
---

### Task COV-10

- **ID** COV-10
- **Context Bundle** `Cargo.toml`
- **DoD** Introduce property-based tests for task parsing invariants and add any required dev-dependencies.
- **Checklist**
  * Add a property-based test crate in `Cargo.toml` and generate randomized inputs for `task_blocks_from_contents`.
  * Validate invariants for `is_task_header`, `is_task_block_end`, and unchecked line detection without false positives.
- **Dependencies** None
- [ ] COV-10 Add property-based tests for task parsing invariants
---

### Task COV-11

- **ID** COV-11
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add unit tests for `BackendError` Display and source mapping, and additional `command_in_path` edge cases.
- **Checklist**
  * Validate Display formatting and `source()` behavior for Io and Json variants of `BackendError`.
  * Add edge-case tests for `command_in_path` with missing or empty PATH.
- **Dependencies** None
- [ ] COV-11 Expand backend module error formatting coverage
---

### Task COV-12

- **ID** COV-12
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `parse_text` missing file error handling and `run_iteration` output file creation failures.
- **Checklist**
  * Verify `parse_text` returns BackendError::Io with the missing path.
  * Simulate a read-only output directory and assert `run_iteration` returns BackendError::Io when creating the output file.
- **Dependencies** None
- [ ] COV-12 Expand Claude adapter error-path coverage
---

### Task COV-13

- **ID** COV-13
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `parse_text` missing file errors and `run_iteration` output file creation failures.
- **Checklist**
  * Validate `parse_text` propagates BackendError::Io for missing response files.
  * Simulate an unwritable output path and assert `run_iteration` returns BackendError::Io.
- **Dependencies** None
- [ ] COV-13 Expand OpenCode adapter error-path coverage
---

### Task COV-14

- **ID** COV-14
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `parse_text` missing file errors and `run_iteration` output file creation failures.
- **Checklist**
  * Validate `parse_text` propagates BackendError::Io for missing response files.
  * Simulate an unwritable output path and assert `run_iteration` returns BackendError::Io.
- **Dependencies** None
- [ ] COV-14 Expand Gemini adapter error-path coverage
---

### Task COV-15

- **ID** COV-15
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `run_iteration` output file creation failures and `check_installed` behavior when PATH is unset or empty.
- **Checklist**
  * Simulate an unwritable output path and assert `run_iteration` returns BackendError::Io.
  * Add PATH guard cases that unset or empty PATH and verify `check_installed` returns false.
- **Dependencies** None
- [ ] COV-15 Expand Codex adapter error-path coverage
---

## Success Criteria

- Overall coverage reaches 90% as reported by the configured tarpaulin command in `config/default.yaml`.
- Each listed Rust file in the coverage report has new tests that cover error paths and public APIs described in the tasks.
- Property-based tests exist for task parsing invariants and run deterministically under `cargo test --workspace`.
- Coverage remains a signal only; a soft CI target at 65 to 70 percent is added after stabilization, with a plan to raise to 75 to 80 percent later.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
