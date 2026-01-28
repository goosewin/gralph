# Project Requirements Document (Template)

## Overview

Increase automated test coverage in the gralph Rust CLI from the current ~51.85% to at least 90% by adding targeted tests across core modules and backends. Intended users are maintainers and CI reviewers who rely on verifier coverage gates.

## Problem Statement

- Overall line coverage is ~51.85% with several key modules (main, verifier, prd, update, backends) below target.
- PROCESS.md and config/default.yaml require coverage >= 90%, so current coverage risks CI failures and regressions.
- Gaps cluster in error paths, validation branches, and CLI argument handling, leaving critical behavior untested.

## Solution

Add focused unit and integration tests for uncovered branches in each listed module, using existing test patterns and minimal helpers. Keep changes behavior-neutral and follow KISS, YAGNI, DRY, SOLID, and TDA.

---

## Functional Requirements

### FR-1: Coverage Expansion Per Module

Each listed Rust source file gains tests that exercise currently uncovered branches (validation, error handling, parsing, path resolution, and CLI wiring).

### FR-2: Preserve Runtime Behavior

Tests verify existing behavior without changing production logic, CLI outputs, or configuration semantics.

---

## Non-Functional Requirements

### NFR-1: Coverage Target

- Overall line coverage >= 90% using the verifier coverage command in config/default.yaml and cargo test --workspace.

### NFR-2: Reliability

- Tests are deterministic and local-only: no network calls or external CLI dependencies; use temp dirs and stub commands.

---

## Implementation Tasks

### Task COV-17

- **ID** COV-17
- **Context Bundle** `src/backend/claude.rs`
- **DoD** Add unit tests covering assistant text extraction, result parsing fallback, and run_iteration error or flag handling without changing runtime behavior.
- **Checklist**
  * Cover extract_assistant_texts with missing content and non-text items.
  * Cover parse_text when result lines are missing or not last.
  * Add a stub command test for run_iteration model flag handling and non-zero exit propagation.
- **Dependencies** None
- [x] COV-17 Expand Claude backend parsing and failure path coverage
---

### Task COV-18

- **ID** COV-18
- **Context Bundle** `src/backend/codex.rs`
- **DoD** Add tests for check_installed, parse_text IO errors, and run_iteration command failures.
- **Checklist**
  * Cover check_installed true/false using a temporary PATH entry.
  * Assert parse_text returns BackendError::Io on missing file.
  * Verify run_iteration returns BackendError::Command when command is missing or exits non-zero.
- **Dependencies** None
- [x] COV-18 Expand Codex backend installation and error coverage
---

### Task COV-19

- **ID** COV-19
- **Context Bundle** `src/backend/gemini.rs`
- **DoD** Add tests for check_installed, model flag handling, and spawn failures.
- **Checklist**
  * Cover check_installed with PATH missing and present.
  * Verify run_iteration includes --headless and --model only when non-empty.
  * Assert run_iteration returns BackendError::Command on spawn failure.
- **Dependencies** None
- [x] COV-19 Expand Gemini backend command and error coverage
---

### Task COV-20

- **ID** COV-20
- **Context Bundle** `src/backend/opencode.rs`
- **DoD** Add tests for check_installed, env flag propagation, and command failure handling.
- **Checklist**
  * Cover check_installed true/false with PATH control.
  * Verify OPENCODE_EXPERIMENTAL_LSP_TOOL is set and model or variant flags are omitted when empty.
  * Assert run_iteration returns BackendError::Command on spawn failure or non-zero exit.
- **Dependencies** None
- [x] COV-20 Expand OpenCode backend env and failure coverage
---

### Task COV-21

- **ID** COV-21
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for stream_command_output error propagation and command_in_path edge cases.
- **Checklist**
  * Verify stream_command_output returns BackendError when on_line returns an error.
  * Cover command_in_path behavior with empty PATH and non-file entries.
  * Validate backend_from_name returns the expected error string for unknown backends.
- **Dependencies** None
- [ ] COV-21 Expand backend module utility coverage
---

### Task COV-22

- **ID** COV-22
- **Context Bundle** `src/config.rs`
- **DoD** Add tests for config path resolution, env override precedence, and value rendering edge cases.
- **Checklist**
  * Cover config_paths when project config exists and name override is set.
  * Cover resolve_env_override precedence for legacy, normalized, and compat forms.
  * Cover value_to_string returning None for mappings and flatten_value ignoring non-string keys.
- **Dependencies** None
- [ ] COV-22 Expand config loader and override coverage
---

### Task COV-23

- **ID** COV-23
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for run_iteration invalid input paths, backend not installed, and completion checks with missing data.
- **Checklist**
  * Cover run_iteration validation for empty project_dir, iteration 0, max_iterations 0, and missing task file.
  * Cover backend not installed path returning CoreError::InvalidInput.
  * Cover check_completion error when task file is missing and false on empty result.
- **Dependencies** None
- [ ] COV-23 Expand core loop validation and completion coverage
---

### Task COV-24

- **ID** COV-24
- **Context Bundle** `src/main.rs`
- **DoD** Add tests for helper utilities and validation paths in main.rs without changing CLI behavior.
- **Checklist**
  * Cover sanitize_session_name and session_name fallback for invalid names or dirs.
  * Cover validate_task_id rejecting invalid formats and accepting valid ones.
  * Cover parse_bool_value for accepted, rejected, and mixed-case inputs, plus resolve_log_file fallback behavior.
- **Dependencies** None
- [ ] COV-24 Expand main CLI helper coverage
---

### Task COV-25

- **ID** COV-25
- **Context Bundle** `src/notify.rs`
- **DoD** Add tests for formatting edge cases and webhook error handling.
- **Checklist**
  * Cover format_failure_description for unknown reasons and format_duration for hour-level values.
  * Cover detect_webhook_type with uppercase or mixed-case URLs.
  * Add stub HTTP server test for send_webhook timeout default and non-2xx status handling.
- **Dependencies** None
- [ ] COV-25 Expand notification formatting and HTTP error coverage
---

### Task COV-26

- **ID** COV-26
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for PRD sanitization, context handling, and stack summary formatting branches.
- **Checklist**
  * Cover sanitize_task_block context filtering and unchecked line collapse behavior.
  * Cover extract_context_entries across multiline Context Bundle sections.
  * Cover context_display_path for absolute paths inside repo and prd_format_stack_summary stack focus line.
- **Dependencies** None
- [ ] COV-26 Expand PRD sanitization and stack summary coverage
---

### Task COV-27

- **ID** COV-27
- **Context Bundle** `src/server.rs`
- **DoD** Add tests for auth failures, CORS rejection, session enrichment edge cases, and fallback routing.
- **Checklist**
  * Cover check_auth with missing header, invalid scheme, and wrong token.
  * Cover resolve_cors_origin returning None for untrusted origins when open is false.
  * Cover enrich_session converting running sessions to stale when pid is dead and fallback_handler 404 path.
- **Dependencies** None
- [ ] COV-27 Expand server auth, CORS, and session enrichment coverage
---

### Task COV-28

- **ID** COV-28
- **Context Bundle** `src/state.rs`
- **DoD** Add tests for state normalization and field parsing branches.
- **Checklist**
  * Cover set_session skipping empty field keys.
  * Cover list_sessions handling non-object session values in state.json.
  * Cover parse_value behavior for numeric strings with leading zeros and mixed inputs.
- **Dependencies** None
- [ ] COV-28 Expand state store normalization coverage
---

### Task COV-29

- **ID** COV-29
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for task block parsing at EOF and block end detection edge cases.
- **Checklist**
  * Cover task_blocks_from_contents when the last block has no separator.
  * Verify is_task_block_end behavior with headings and separators only.
  * Verify is_task_header strictness for malformed headings.
- **Dependencies** None
- [ ] COV-29 Expand task parsing edge coverage
---

### Task COV-30

- **ID** COV-30
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for release parsing and archive extraction failure paths without network calls.
- **Checklist**
  * Cover parse_release_tag with missing or non-string tag_name values.
  * Cover extract_archive failure handling with invalid or empty tar input.
  * Cover resolve_install_version and normalize_version error paths with invalid versions.
- **Dependencies** None
- [ ] COV-30 Expand update parsing and extraction coverage
---

### Task COV-31

- **ID** COV-31
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add unit tests covering command parsing, coverage parsing, review gate logic, and static checks branches.
- **Checklist**
  * Cover parse_verifier_command errors on empty or malformed commands.
  * Cover extract_coverage_percent and parse_percent_from_line with common tarpaulin output shapes.
  * Cover evaluate_review_gate and evaluate_check_gate for pending, failed, and passed cases.
  * Cover static checks for TODO markers, verbose comment limits, and wildcard path matching.
- **Dependencies** None
- [ ] COV-31 Expand verifier parsing and static check coverage
---

## Success Criteria

- Overall line coverage reaches >= 90% and the verifier coverage gate passes with config/default.yaml.
- Each listed Rust file shows increased coverage from the current baseline report.
- cargo test --workspace passes locally without external network or CLI dependencies.
- No production behavior changes; only tests and minimal helpers are added.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
