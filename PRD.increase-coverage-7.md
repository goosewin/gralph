# Project Requirements Document (Template)

## Overview

Raise Rust test coverage to 90 percent for the gralph CLI by adding focused tests across core logic, PRD validation, verifier gating, state persistence, backend adapters, and supporting modules. Intended users are gralph maintainers and contributors who rely on stable loop behavior and deterministic verification.

## Problem Statement

- Current coverage is 58.92 percent with several core modules below the desired threshold, increasing regression risk.
- Error paths, parsing invariants, and configuration edge cases are under-tested across core loop, PRD validation, and verifier helpers.
- Coverage work can drift into glue code without a plan, leading to slow gains and low confidence.

## Solution

Focus first on the top 20 percent of modules that carry core logic risk (core loop, PRD validation, state store, verifier, config). Then expand to public API and error-path tests in main, server, notify, update, and backend adapters. Use property-based tests for parsing invariants instead of many narrow unit tests. Do not refactor only to increase coverage and do not add new coverage gates in this effort; coverage remains a signal while tests are added.

---

## Functional Requirements

### FR-1: Core Logic Coverage

Add tests that exercise core loop behavior, PRD validation, state store, and verifier parsing and gating logic, including error paths and invariant checks.

### FR-2: Public API and Adapter Coverage

Add tests for CLI helpers, server endpoints, notification formatting, update/install logic, and backend adapter command construction and error handling.

---

## Non-Functional Requirements

### NFR-1: Performance

- Tests must be deterministic and fast, using temp directories and stub commands instead of network calls or external CLIs.

### NFR-2: Reliability

- Behavior must remain unchanged; avoid refactors aimed only at coverage.
- Do not change verifier or CI coverage thresholds in this effort; coverage is reported but no new gates are introduced.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/core.rs`, `PROCESS.md`
- **DoD** Add unit tests in src/core.rs that cover error-path logging, prompt template resolution, and file handling without changing loop behavior.
- **Checklist**
  * Add tests for log_message, raw_log_path, copy_if_exists, and cleanup_old_logs retention behavior.
  * Cover resolve_prompt_template for explicit, env, project, and default paths.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-1 Expand core loop coverage for logging, templates, and file helpers
---

### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/state.rs`, `PROCESS.md`
- **DoD** Add tests in src/state.rs for lock acquisition failures, read and write error paths, and state validation edge cases.
- **Checklist**
  * Cover acquire_lock timeout and non-contention error mapping.
  * Cover read_state and write_state error propagation and validate_state_content failures.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-2 Expand state store coverage for lock and IO failure paths
---

### Task COV-3

- **ID** COV-3
- **Context Bundle** `src/prd.rs`, `PRD.template.md`
- **DoD** Add tests in src/prd.rs for context filtering, task block sanitization, and stack detection edge cases.
- **Checklist**
  * Add tests for sanitize_task_block and context_display_path handling of absolute and relative entries.
  * Add property-based tests for extract_context_entries and task block validation invariants.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-3 Expand PRD validation and sanitization coverage
---

### Task COV-4

- **ID** COV-4
- **Context Bundle** `src/main.rs`, `config/default.yaml`, `PROCESS.md`
- **DoD** Add tests targeting verifier helper logic in src/verifier.rs using config-driven settings and parsing helpers.
- **Checklist**
  * Add tests for parse_percent_from_line, extract_coverage_percent, and parse_verifier_command error cases.
  * Add tests for review gate parsing, merge method resolution, and check rollup evaluation.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-4 Expand verifier parsing, static checks, and review gate coverage
---

### Task COV-5

- **ID** COV-5
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Add tests in src/config.rs that cover key normalization, env override resolution, and value rendering for edge cases.
- **Checklist**
  * Cover resolve_env_override when legacy and compat keys conflict and when values are empty.
  * Cover value_to_string for tagged and mixed sequences and lookup_value for mixed-case keys.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-5 Expand config normalization and env override coverage
---

### Task COV-6

- **ID** COV-6
- **Context Bundle** `src/main.rs`, `src/cli.rs`
- **DoD** Add tests for CLI helper functions in src/main.rs that do not require git or external commands.
- **Checklist**
  * Cover set_yaml_value, parse_yaml_value, ensure_mapping, and read_yaml_or_empty edge cases.
  * Cover resolve_init_context_files, is_markdown_path, and format_display_path behavior.
  * `cargo test --workspace` passes.
- **Dependencies** COV-1
- [x] COV-6 Expand CLI helper coverage in main without external dependencies
---

### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/server.rs`, `README.md`
- **DoD** Add tests in src/server.rs for CORS, auth header parsing, and enrich_session edge cases.
- **Checklist**
  * Cover check_auth with invalid header encodings and missing Bearer token formats.
  * Cover resolve_cors_origin for non-localhost host matches and open mode behavior.
  * `cargo test --workspace` passes.
- **Dependencies** COV-2
- [x] COV-7 Expand server auth, CORS, and session enrichment coverage
---

### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/notify.rs`, `README.md`
- **DoD** Add tests in src/notify.rs for formatting, payload assembly, and error-path validation.
- **Checklist**
  * Cover format_duration for hour-scale values and format_failure_description for manual_stop.
  * Cover send_webhook input validation and detect_webhook_type case handling.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-8 Expand notification formatting and error handling coverage
---

### Task COV-9

- **ID** COV-9
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for backend module helpers in src/backend/mod.rs, especially command execution edge cases.
- **Checklist**
  * Cover stream_command_output error paths for missing stdout or stderr and non-zero exits.
  * Cover command_in_path behavior with empty PATH and mixed entries.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-9 Expand backend module utility coverage
---

### Task COV-10

- **ID** COV-10
- **Context Bundle** `src/backend/claude.rs`
- **DoD** Add tests for claude backend parsing and error handling in src/backend/claude.rs.
- **Checklist**
  * Cover extract_assistant_texts with missing content and non-text items.
  * Cover parse_text fallback to raw contents when no result entries exist.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-10 Expand Claude backend parsing and error-path coverage
---

### Task COV-11

- **ID** COV-11
- **Context Bundle** `src/backend/opencode.rs`
- **DoD** Add tests for opencode backend command construction and environment handling in src/backend/opencode.rs.
- **Checklist**
  * Cover OPENCODE_EXPERIMENTAL_LSP_TOOL env injection and argument ordering.
  * Cover run_iteration error paths and skip-empty model or variant behavior.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-11 Expand OpenCode backend command and env coverage
---

### Task COV-12

- **ID** COV-12
- **Context Bundle** `src/backend/gemini.rs`
- **DoD** Add tests for gemini backend flags and error handling in src/backend/gemini.rs.
- **Checklist**
  * Cover headless flag placement, model flag inclusion, and prompt ordering.
  * Cover run_iteration error paths and parse_text IO errors.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-12 Expand Gemini backend command and error coverage
---

### Task COV-13

- **ID** COV-13
- **Context Bundle** `src/backend/codex.rs`
- **DoD** Add tests for codex backend flags and error handling in src/backend/codex.rs.
- **Checklist**
  * Cover quiet and auto-approve flag inclusion and model handling.
  * Cover run_iteration spawn failures and parse_text IO errors.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [x] COV-13 Expand Codex backend command and error coverage
---

### Task COV-14

- **ID** COV-14
- **Context Bundle** `src/core.rs`, `src/prd.rs`
- **DoD** Add property-based tests in src/task.rs to harden task parsing invariants used by core and PRD validation.
- **Checklist**
  * Add proptest cases for task block termination on separators and H2 headings.
  * Add proptest cases for unchecked line detection with whitespace and malformed prefixes.
  * `cargo test --workspace` passes.
- **Dependencies** COV-1
- [ ] COV-14 Expand task parsing invariants with property-based tests
---

### Task COV-15

- **ID** COV-15
- **Context Bundle** `src/backend/mod.rs`, `src/config.rs`
- **DoD** Add tests in src/test_support.rs to validate env_lock behavior under contention and after panics.
- **Checklist**
  * Add a multi-thread test that confirms env_lock serializes access under concurrent attempts.
  * Add a test that confirms env_lock remains usable after a panic in a prior holder.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [ ] COV-15 Expand test_support env_lock coverage for contention scenarios
---

### Task COV-16

- **ID** COV-16
- **Context Bundle** `src/main.rs`, `README.md`
- **DoD** Add tests in src/update.rs for update parsing, download, and install error handling using local fixtures.
- **Checklist**
  * Cover resolve_install_version latest handling, parse_release_tag failures, and normalize_version validation.
  * Cover extract_archive error paths and install_binary permission errors where supported.
  * `cargo test --workspace` passes.
- **Dependencies** None
- [ ] COV-16 Expand update workflow parsing and install error coverage
---

## Success Criteria

- Overall coverage reaches 90 percent as reported by `cargo tarpaulin --workspace --fail-under 90 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`.
- Each listed module has new tests covering error paths or invariants, especially core loop, PRD validation, state, and verifier helpers.
- `cargo test --workspace` passes without requiring network access or external CLI installations.
- Verifier and CI coverage threshold configuration is unchanged in this effort.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
