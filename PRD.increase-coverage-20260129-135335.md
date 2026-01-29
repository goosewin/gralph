# Project Requirements Document (Template)

## Overview

Increase Rust test coverage from the current 71.30% to 90% by adding targeted tests in the highest value modules first, then expanding to public APIs and adapter error paths. Focus on correctness of core logic and invariants, prefer property-based tests where they add coverage and resilience, and avoid coverage gaming or refactors that exist only to move the number.

## Problem Statement

- Current coverage is 71.30% (3431/4812 lines), leaving gaps in core loop execution, PRD validation, verifier parsing, and backend adapter error handling.
- Coverage must be treated as a signal, not a merge blocker, and tests should prioritize correctness of core logic and public APIs.
- Many paths involve IO, parsing, and state transitions; missing tests in these areas increase the risk of regressions.

## Solution

Prioritize the top 20% of modules that carry the most logic and risk: `src/core.rs`, `src/state.rs`, `src/prd.rs`, `src/verifier.rs`, `src/config.rs`. Add tests around public APIs and error paths, then expand to CLI helpers, server handlers, notification formatting, update flow, task parsing, and backend adapters. Use property-based tests for invariants (task parsing boundaries, prompt rendering, PRD context selection) instead of many narrow unit tests. Keep coverage thresholds warning-only in the ramp-up phase and do not change hard gates or block merges during this effort.

---

## Functional Requirements

### FR-1: Core Logic Coverage

Add tests that cover core loop execution, state persistence, PRD validation/sanitization, and verifier parsing and gating, including error paths and invariant checks.

### FR-2: Public API and Adapter Coverage

Add tests around CLI helpers, server handlers, notification payloads, update workflow, task parsing utilities, and backend adapter command assembly and error propagation.

---

## Non-Functional Requirements

### NFR-1: Performance

- Keep test runtime reasonable by bounding property-based cases and avoiding slow external calls.

### NFR-2: Reliability

- Do not change hard coverage gates or block merges; any coverage targets added are warning-only.
- Follow existing env mutation guardrails and avoid refactors done only for coverage.
- Avoid chasing line coverage in glue code or trivial getters.

---

## Implementation Tasks

### Task COV-CORE-1

- **ID** COV-CORE-1
- **Context Bundle** `src/core.rs`, `src/config.rs`
- **DoD** Add unit and property tests that cover run_iteration error branches (invalid inputs, missing task file, backend not installed, empty output or parse result) and prompt rendering invariants (context section only when non-empty, task_block fallback behavior).
- **Checklist**
  * Add stub backend tests that hit empty output and empty parse-result paths.
  * Add proptest cases for prompt rendering and context normalization invariants.
- **Dependencies** None
- [x] COV-CORE-1 Expand core loop error-path and prompt invariant coverage
### Task COV-STATE-1

- **ID** COV-STATE-1
- **Context Bundle** `src/state.rs`
- **DoD** Add tests for state lock initialization failure modes (state_dir is a file, lock file path invalid) and parse_value edge cases that should remain strings (whitespace, mixed digits and symbols).
- **Checklist**
  * Add tests that force with_lock to fail when state_dir or lock paths are invalid.
  * Add tests for parse_value with whitespace and mixed numeric tokens.
- **Dependencies** None
- [x] COV-STATE-1 Cover state lock path failures and parse_value edge cases
### Task COV-PRD-1

- **ID** COV-PRD-1
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for sanitize_task_block context filtering (allowed list enforcement, absolute paths inside repo) and property-based invariants for context extraction and stray unchecked removal.
- **Checklist**
  * Add unit tests for allowed context selection and absolute path handling.
  * Add proptest cases for context bundle parsing and checkbox sanitization.
- **Dependencies** None
- [ ] COV-PRD-1 Expand PRD sanitize and context selection invariants
### Task COV-VER-1

- **ID** COV-VER-1
- **Context Bundle** `config/default.yaml`
- **DoD** Add tests for coverage percent parsing with multiple percent tokens, review gate rating and issue parsing, and static check file selection plus TODO marker detection.
- **Checklist**
  * Add unit tests for extract_coverage_percent and rating parsing with fractions and percents.
  * Add tests for static checks allow/ignore patterns and TODO marker boundaries.
- **Dependencies** None
- [ ] COV-VER-1 Cover verifier parsing, review gate, and static check edges
### Task COV-CONFIG-1

- **ID** COV-CONFIG-1
- **Context Bundle** `src/config.rs`
- **DoD** Add tests for normalize_key and env override precedence with mixed hyphen/underscore keys and empty override values; add property tests for value_to_string with mixed sequences.
- **Checklist**
  * Add unit tests for env override precedence with compat keys and empty values.
  * Add proptest coverage for value_to_string sequence rendering.
- **Dependencies** None
- [ ] COV-CONFIG-1 Add config normalization and env override edge coverage
### Task COV-TASK-1

- **ID** COV-TASK-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add property-based tests that enforce task block boundaries across CRLF, separators, and H2 headings, ensuring no non-task lines leak into blocks.
- **Checklist**
  * Add proptest cases for CRLF and separator near-miss handling.
  * Add unit tests for edge headings and spacing near-misses.
- **Dependencies** None
- [ ] COV-TASK-1 Strengthen task parsing invariants with property tests
### Task COV-MAIN-1

- **ID** COV-MAIN-1
- **Context Bundle** `src/main.rs`, `src/cli.rs`
- **DoD** Add unit tests for CLI helpers and worktree utilities (validate_task_id, session_name fallbacks, parse_bool_value, auto_worktree_branch_name uniqueness).
- **Checklist**
  * Add tests for task ID validation failures and session name fallback paths.
  * Add tests for auto worktree naming and boolean parsing variants.
- **Dependencies** None
- [ ] COV-MAIN-1 Expand CLI helper and worktree utility coverage
### Task COV-SERVER-1

- **ID** COV-SERVER-1
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Add tests for enrich_session transitions (running to stale on dead pid), current_remaining calculation when dir missing, and stop handler error responses.
- **Checklist**
  * Add tests for enrich_session state updates and remaining task computation.
  * Add handler tests for missing sessions and CORS in open mode.
- **Dependencies** None
- [ ] COV-SERVER-1 Cover server session enrichment and error responses
### Task COV-NOTIFY-1

- **ID** COV-NOTIFY-1
- **Context Bundle** `src/notify.rs`
- **DoD** Add tests for payload formatting boundaries (duration formatting, unknown failure reasons) and webhook type detection variants.
- **Checklist**
  * Add unit tests for format_duration boundary values and unknown reasons.
  * Add tests for detect_webhook_type and generic payload field inclusion.
- **Dependencies** None
- [ ] COV-NOTIFY-1 Extend notification formatting and detection coverage
### Task COV-UPDATE-1

- **ID** COV-UPDATE-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for resolve_install_version when GRALPH_VERSION is empty or whitespace and for resolve_install_version invalid tag formats.
- **Checklist**
  * Add tests for empty and whitespace version env values.
  * Add tests for invalid tag formats with prerelease or build metadata.
- **Dependencies** None
- [ ] COV-UPDATE-1 Cover update version resolution error paths
### Task COV-BACKEND-MOD-1

- **ID** COV-BACKEND-MOD-1
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for command_in_path ignoring relative paths and for stream_command_output when streams close early or emit stderr-only lines.
- **Checklist**
  * Add unit tests for relative PATH entries and missing directories.
  * Add tests for stream_command_output with stderr-only and early-close scenarios.
- **Dependencies** None
- [ ] COV-BACKEND-MOD-1 Add backend helper error-path coverage
### Task COV-BACKEND-CLAUDE-1

- **ID** COV-BACKEND-CLAUDE-1
- **Context Bundle** `src/backend/claude.rs`
- **DoD** Add tests for parse_text fallback behavior when no result entries exist and for assistant text extraction with malformed or null content.
- **Checklist**
  * Add tests that validate parse_text returns raw contents when result is absent.
  * Add tests for extract_assistant_texts with nulls and mixed entry types.
- **Dependencies** None
- [ ] COV-BACKEND-CLAUDE-1 Cover Claude adapter parse fallbacks and malformed content
### Task COV-BACKEND-OPENCODE-1

- **ID** COV-BACKEND-OPENCODE-1
- **Context Bundle** `src/backend/opencode.rs`
- **DoD** Add tests for parse_text invalid UTF-8 handling and run_iteration behavior when model or variant are whitespace-only.
- **Checklist**
  * Add unit tests for parse_text invalid UTF-8 error propagation.
  * Add tests for model and variant whitespace trimming behavior.
- **Dependencies** None
- [ ] COV-BACKEND-OPENCODE-1 Expand OpenCode adapter error and flag handling coverage
### Task COV-BACKEND-GEMINI-1

- **ID** COV-BACKEND-GEMINI-1
- **Context Bundle** `src/backend/gemini.rs`
- **DoD** Add tests for run_iteration argument ordering with and without model and parse_text invalid UTF-8 errors.
- **Checklist**
  * Add tests for headless flag ordering and prompt placement.
  * Add tests for parse_text invalid UTF-8 error handling.
- **Dependencies** None
- [ ] COV-BACKEND-GEMINI-1 Add Gemini adapter ordering and invalid UTF-8 coverage
### Task COV-BACKEND-CODEX-1

- **ID** COV-BACKEND-CODEX-1
- **Context Bundle** `src/backend/codex.rs`
- **DoD** Add tests for run_iteration ordering when model is omitted and for parse_text invalid UTF-8 errors.
- **Checklist**
  * Add tests for flag ordering when model is None or empty.
  * Add tests for invalid UTF-8 parsing error propagation.
- **Dependencies** None
- [ ] COV-BACKEND-CODEX-1 Expand Codex adapter ordering and parse error coverage
### Task COV-TESTSUPPORT-1

- **ID** COV-TESTSUPPORT-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for env_lock reuse after drop and for sequential lock acquisition in a single thread.
- **Checklist**
  * Add tests that acquire, drop, and reacquire env_lock safely.
  * Add tests that confirm lock serialization in a single-thread sequence.
- **Dependencies** None
- [ ] COV-TESTSUPPORT-1 Extend env_lock safety and reuse coverage
---

## Success Criteria

- Overall coverage reaches 90% with new tests focused on core logic and error paths.
- Core modules (`src/core.rs`, `src/state.rs`, `src/prd.rs`, `src/verifier.rs`, `src/config.rs`) have added property-based or invariant tests.
- All listed Rust modules in the coverage report have at least one new coverage-focused test task completed.
- Coverage thresholds remain warning-only during ramp-up and do not block merges.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
