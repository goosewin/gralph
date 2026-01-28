# Project Requirements Document (Template)

## Overview

Raise Rust test coverage for the gralph CLI from 57.20% to 90% by adding focused tests for core loop logic, PRD parsing, state management, verifier gating, and backend adapters. Intended users are gralph maintainers and contributors who rely on stable automation for PRD-driven development.

## Problem Statement

- Current line coverage is 57.20% with several core modules below target, weakening confidence in core flows and error handling.
- Critical paths in PRD validation, state persistence, verifier gating, and backend adapter execution lack comprehensive tests.
- Coverage gates are configured to expect 90% in the verifier pipeline, but current coverage is well below that threshold.

## Solution

Phase test expansion to follow the highest-impact 20% of modules first: `src/core.rs`, `src/prd.rs`, `src/state.rs`, and verifier-related logic (invoked from `src/main.rs` and configured in `config/default.yaml`). Add targeted unit and property-based tests for invariants and error paths in those modules, then expand to remaining CLI, server, notify, backend adapters, update flow, and task parsing. Avoid refactors that exist only to raise coverage, and focus on correctness of core logic.

---

## Functional Requirements

### FR-1: Core Test Coverage Expansion

Add tests covering core loop orchestration, PRD validation and sanitization, state store behavior, and verifier gate parsing to quickly raise coverage.

### FR-2: Module-Wide Coverage Completion

Add tests for remaining modules (CLI helpers, server endpoints, notifications, backend adapters, update/install utilities, task parsing) emphasizing error paths and public API behaviors.

---

## Non-Functional Requirements

### NFR-1: Performance

- All tests must run under `cargo test --workspace`, and coverage must be measurable with the tarpaulin command configured in `config/default.yaml` without introducing long-running integration tests.

### NFR-2: Reliability

- Tests must emphasize error paths and invariants, using property-based tests where appropriate.
- Do not change CI or verifier thresholds as part of this effort; coverage is used as a signal until stabilized.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/core.rs`, `src/main.rs`
- **DoD** Add unit tests in `src/core.rs` that cover run_iteration error paths, prompt template selection order, and completion promise parsing; coverage for `src/core.rs` increases.
- **Checklist**
  * Add tests for resolve_prompt_template ordering (explicit template, env var, project file, default).
  * Add tests for run_iteration when backend output is empty or parse_text returns whitespace, asserting CoreError::InvalidInput.
  * Add tests for check_completion using last non-empty line handling and negated promise prefixes.
- **Dependencies** None
- [x] COV-1 Expand core loop error-path coverage
---

### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/prd.rs`, `PRD.template.md`
- **DoD** Add tests in `src/prd.rs` covering task block validation edges, sanitization fallback behavior, and stack detection evidence; coverage for `src/prd.rs` increases.
- **Checklist**
  * Add tests for extract_context_entries with multi-line Context Bundle and mixed backtick entries.
  * Add tests for sanitize_task_block fallback when context entries are invalid, including allowed list behavior.
  * Add tests for prd_detect_stack with Cargo.toml presence and for open questions detection case behavior.
- **Dependencies** None
- [x] COV-2 Expand PRD validation and sanitization tests
---

### Task COV-3

- **ID** COV-3
- **Context Bundle** `src/state.rs`, `src/server.rs`
- **DoD** Add unit tests in `src/state.rs` covering lock acquisition failures, cleanup behavior for non-object sessions, and parse_value edge cases; coverage for `src/state.rs` increases.
- **Checklist**
  * Add tests for lock file path errors and ensure init_state handles missing or corrupted state safely.
  * Add tests for cleanup_stale when session values are non-object or missing fields.
  * Add tests for parse_value with negative numbers and mixed alphanumeric inputs.
- **Dependencies** None
- [ ] COV-3 Expand state store edge-case coverage
---

### Task COV-4

- **ID** COV-4
- **Context Bundle** `src/main.rs`, `config/default.yaml`
- **DoD** Add unit tests covering verifier parsing and gate evaluation logic; coverage for verifier-related code paths increases.
- **Checklist**
  * Add tests for extract_coverage_percent and parse_percent_from_line with multiple percent tokens and fallback lines.
  * Add tests for parse_review_rating and parse_review_issue_count covering fraction, percent, and rating text formats.
  * Add tests for wildcard_match and static check file selection edge cases.
- **Dependencies** None
- [ ] COV-4 Expand verifier parsing and gate evaluation tests
---

### Task COV-5

- **ID** COV-5
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Add tests in `src/config.rs` that cover key normalization, env override precedence, and list output stability; coverage for `src/config.rs` increases.
- **Checklist**
  * Add tests for lookup_value with nested, mixed-case, and hyphenated keys.
  * Add tests for resolve_env_override precedence between legacy aliases, normalized keys, and compat env vars.
  * Add tests for list() with sequences and null values to confirm value_to_string behavior.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-5 Expand config normalization and override tests
---

### Task COV-6

- **ID** COV-6
- **Context Bundle** `src/main.rs`, `src/cli.rs`
- **DoD** Add unit tests in `src/main.rs` for session naming, boolean parsing, and log resolution helpers; coverage for `src/main.rs` increases.
- **Checklist**
  * Add tests for session_name and sanitize_session_name with empty, whitespace, and special-character names.
  * Add tests for parse_bool_value accepted and rejected variants and for resolve_auto_worktree using a temp config.
  * Add tests for resolve_log_file fallback when session metadata omits log_file.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-6 Expand CLI helper coverage in main
---

### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Add tests in `src/server.rs` covering auth and CORS edge cases and error responses; coverage for `src/server.rs` increases.
- **Checklist**
  * Add tests for check_auth with missing/malformed Authorization headers and with token disabled.
  * Add tests for resolve_cors_origin on host-specific matches and open mode wildcard behavior.
  * Add tests asserting CORS headers on error responses from stop/status endpoints.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-7 Expand server auth and CORS error-path tests
---

### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/notify.rs`, `config/default.yaml`
- **DoD** Add tests in `src/notify.rs` covering payload generation, failure reason mappings, and webhook error handling; coverage for `src/notify.rs` increases.
- **Checklist**
  * Add tests for build_generic_payload optional fields and message assembly.
  * Add tests for format_failure_description and format_*_failed outputs for unknown reasons.
  * Add tests for send_webhook error handling on invalid URL or non-success status.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-8 Expand notification formatting and HTTP error tests
---

### Task COV-9

- **ID** COV-9
- **Context Bundle** `src/backend/mod.rs`, `src/backend/claude.rs`
- **DoD** Add tests in `src/backend/mod.rs` for BackendError display paths and stream_command_output edge behavior; coverage for `src/backend/mod.rs` increases.
- **Checklist**
  * Add tests for BackendError::InvalidInput display output and source behavior.
  * Add tests for stream_command_output with empty stdout/stderr but successful exit.
  * Add tests for command_in_path with PATH entries that are directories and files.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-9 Expand backend module utility coverage
---

### Task COV-10

- **ID** COV-10
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`
- **DoD** Add tests in `src/backend/claude.rs` for command installation checks and parse_text fallbacks; coverage for `src/backend/claude.rs` increases.
- **Checklist**
  * Add tests for check_installed using a temp executable returning success/failure.
  * Add tests for parse_text when no result type is present, returning raw contents.
  * Add tests for extract_assistant_texts with missing content and non-text items.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-10 Expand Claude backend parsing and install tests
---

### Task COV-11

- **ID** COV-11
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** Add tests in `src/backend/opencode.rs` for command assembly and parse_text behavior; coverage for `src/backend/opencode.rs` increases.
- **Checklist**
  * Add tests for check_installed with PATH overrides and for command() accessor.
  * Add tests for run_iteration ensuring prompt is last arg and output file contains stdout.
  * Add tests for parse_text with empty file and trailing whitespace.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-11 Expand OpenCode backend run_iteration tests
---

### Task COV-12

- **ID** COV-12
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Add tests in `src/backend/gemini.rs` for command assembly and parse_text behavior; coverage for `src/backend/gemini.rs` increases.
- **Checklist**
  * Add tests for check_installed with PATH overrides and for command() accessor.
  * Add tests for run_iteration ensuring --headless is always included and prompt is last arg.
  * Add tests for parse_text with empty file and missing file errors.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-12 Expand Gemini backend command tests
---

### Task COV-13

- **ID** COV-13
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Add tests in `src/backend/codex.rs` for command flags and parse_text behavior; coverage for `src/backend/codex.rs` increases.
- **Checklist**
  * Add tests for check_installed with PATH overrides and for command() accessor.
  * Add tests for run_iteration ensuring --quiet/--auto-approve and model flag behavior.
  * Add tests for parse_text with empty file and missing file errors.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-13 Expand Codex backend command tests
---

### Task COV-14

- **ID** COV-14
- **Context Bundle** `src/main.rs`, `README.md`
- **DoD** Add tests in `src/update.rs` covering release URL resolution, download failures, and install path handling; coverage for `src/update.rs` increases.
- **Checklist**
  * Add tests for release_url honoring GRALPH_TEST_RELEASE_URL and for resolve_install_version using a local test tag.
  * Add tests for download_release error handling using a local server with non-200 responses.
  * Add tests for install_binary path creation and PermissionDenied messaging with GRALPH_INSTALL_DIR.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-14 Expand update and install error-path coverage
---

### Task COV-15

- **ID** COV-15
- **Context Bundle** `src/core.rs`, `src/prd.rs`
- **DoD** Add tests in `src/task.rs` for task header and block boundary edge cases; coverage for `src/task.rs` increases.
- **Checklist**
  * Add tests for is_task_header with trailing space and no ID, and for is_task_block_end on empty h2 headings.
  * Add tests for task_blocks_from_contents with adjacent task blocks and trailing sections.
  * Extend property-based tests to assert no stray lines are included in blocks.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-15 Expand task parsing edge-case tests
---

### Task COV-16

- **ID** COV-16
- **Context Bundle** `src/lib.rs`, `src/backend/mod.rs`
- **DoD** Add tests for env_lock behavior in `src/test_support.rs`, including poisoned mutex recovery; coverage for `src/test_support.rs` remains complete and behavior verified.
- **Checklist**
  * Add a test that poisons ENV_LOCK in a thread and asserts env_lock still returns a guard without panicking.
  * Add a test that multiple env_lock calls serialize access via the mutex.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-16 Add env_lock behavior tests
---

## Success Criteria

- Overall coverage reported by the tarpaulin command in `config/default.yaml` is at least 90%.
- Each file listed in the coverage report has new tests covering core logic or error paths, with coverage increasing from the current baseline.
- `cargo test --workspace` passes without introducing slow or flaky tests.
- No refactor-only changes or CI threshold changes are introduced solely to move coverage.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
