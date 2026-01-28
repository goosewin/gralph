# Project Requirements Document (Template)

## Overview

Raise automated test coverage for the gralph Rust CLI from 42.76% (2035/4759) to 90% by adding deterministic tests across the backend adapters, core loop, PRD validation, server, notifier, update, and verifier logic. Intended users are gralph maintainers and contributors who rely on stable CLI behavior and CI coverage gates.

## Problem Statement

- Coverage is well below the 90% gate and heavily uneven across core modules.
- Error handling, parsing, and CLI argument wiring paths are under-tested, increasing regression risk.
- Several modules rely on external commands or IO; tests must cover these paths without using real network or CLIs.

## Solution

Add focused unit and small integration-style tests in existing module test blocks. Use temp directories and stub executables to simulate external CLIs, avoid network calls, and keep changes minimal. Add small test hooks only when required for isolation, following KISS and DRY principles.

---

## Functional Requirements

### FR-1: Targeted Module Coverage

Add tests for each file listed in the coverage report, with explicit coverage of key branches and error paths in that file.

### FR-2: Behavior Preservation

Increase coverage without changing user-visible CLI behavior, error messages, or output formats.

---

## Non-Functional Requirements

### NFR-1: Coverage Gate

- Project coverage must be >= 90% using the verifier coverage command from `config/default.yaml`.

### NFR-2: Deterministic Tests

- Tests must run offline, use local stubs instead of real CLIs or network calls, and be repeatable on CI.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/backend/mod.rs`, `Cargo.toml`
- **DoD** Add unit tests in `src/backend/mod.rs` covering `command_in_path` PATH edge cases, `stream_command_output` success and failure paths, and error handling when stdout/stderr are not piped.
- **Checklist**
  * Add tests that set PATH empty/missing and assert `command_in_path` returns false and true for a temp executable.
  * Add tests that run a stub command emitting stdout and stderr and verify `stream_command_output` returns Ok on exit 0.
  * Add tests for non-zero exit and missing stdout/stderr pipes that assert `BackendError::Command`.
- **Dependencies** None
- [x] COV-1 Expand backend module command and stream coverage
### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `extract_assistant_texts`, `extract_result_text`, `parse_text` fallback behavior, and `run_iteration` invalid input handling in `src/backend/claude.rs`.
- **Checklist**
  * Test `extract_assistant_texts` with assistant/non-assistant messages and mixed content types.
  * Test `parse_text` returning the last result when present and raw contents when no result exists.
  * Add a `run_iteration` test asserting empty prompt returns `BackendError::InvalidInput`.
- **Dependencies** None
- [x] COV-2 Extend Claude backend parsing and input validation coverage
### Task COV-3

- **ID** COV-3
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `OpenCodeBackend::run_iteration` to verify prompt validation, env var injection, and optional model/variant args.
- **Checklist**
  * Use a temp stub executable to echo args and env, asserting `OPENCODE_EXPERIMENTAL_LSP_TOOL` is set.
  * Assert `--model` and `--variant` are passed only when non-empty.
  * Add a test that empty prompt returns `BackendError::InvalidInput`.
- **Dependencies** None
- [ ] COV-3 Cover OpenCode backend run_iteration argument and env handling
### Task COV-4

- **ID** COV-4
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `GeminiBackend::run_iteration` covering `--headless` behavior, optional model arg, and invalid prompt handling.
- **Checklist**
  * Use a temp stub executable to confirm `--headless` is always present.
  * Assert `--model` is included only when a non-empty model is provided.
  * Add a test that empty prompt returns `BackendError::InvalidInput`.
- **Dependencies** None
- [ ] COV-4 Cover Gemini backend run_iteration flags and validation
### Task COV-5

- **ID** COV-5
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Add tests for `CodexBackend::run_iteration` covering `--quiet` and `--auto-approve` flags, optional model arg, and invalid prompt handling.
- **Checklist**
  * Use a temp stub executable to confirm fixed flags are present and prompt is passed.
  * Assert `--model` is included only when non-empty.
  * Add a test that empty prompt returns `BackendError::InvalidInput`.
- **Dependencies** None
- [ ] COV-5 Cover Codex backend run_iteration flags and validation
### Task COV-6

- **ID** COV-6
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Add unit tests in `src/config.rs` to cover key normalization, mapping lookup edge cases, and env override compatibility branches.
- **Checklist**
  * Test `lookup_mapping_value` with mixed case and hyphenated keys resolving to normalized keys.
  * Test `value_to_string` for `Value::Null` and sequences with mixed types.
  * Add tests for legacy hyphenated env overrides and `config_paths` behavior with missing project dirs.
- **Dependencies** None
- [ ] COV-6 Increase config normalization and env override coverage
### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/core.rs`, `config/default.yaml`
- **DoD** Add tests in `src/core.rs` for completion validation edge cases, prompt template resolution precedence, and log cleanup behavior.
- **Checklist**
  * Test `check_completion` for negated promises, mismatched markers, and remaining tasks.
  * Test `resolve_prompt_template` precedence for explicit template, env file, .gralph file, and default.
  * Test `cleanup_old_logs` removes only old .log files and preserves non-log files.
- **Dependencies** None
- [ ] COV-7 Expand core loop helpers and logging coverage
### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `config/default.yaml`
- **DoD** Add unit tests for `src/main.rs` helpers: boolean parsing, session name sanitation, context file discovery, and log file resolution.
- **Checklist**
  * Test `parse_bool_value` for accepted true/false strings and invalid values.
  * Test `sanitize_session_name` and `session_name` fallback when names are empty or contain invalid chars.
  * Test `read_readme_context_files`, `build_context_file_list`, and `resolve_log_file` with temp dirs and minimal fixtures.
- **Dependencies** None
- [ ] COV-8 Increase main.rs helper and context list coverage
### Task COV-9

- **ID** COV-9
- **Context Bundle** `src/notify.rs`, `config/default.yaml`
- **DoD** Add tests for notify validation errors and failed webhook payload formats across Discord, Slack, and generic cases.
- **Checklist**
  * Add tests for `notify_complete` and `notify_failed` rejecting empty session name or webhook URL.
  * Add tests for `format_discord_failed`, `format_slack_failed`, and `format_generic_failed` reason mappings.
  * Add tests for `format_duration` with None and hour/minute formatting plus `send_webhook` empty payload errors.
- **Dependencies** None
- [ ] COV-9 Cover notify validation and failure formatting paths
### Task COV-10

- **ID** COV-10
- **Context Bundle** `src/prd.rs`, `README.md`
- **DoD** Add unit tests in `src/prd.rs` for context bundle parsing, sanitization, and label helpers.
- **Checklist**
  * Test `extract_context_entries` and `context_bundle_indent` for multi-line Context Bundle fields.
  * Test `context_entry_exists` and `context_display_path` for absolute and relative paths inside and outside base dir.
  * Test `sanitize_task_block` removing extra unchecked lines and rebuilding Context Bundle with fallback.
- **Dependencies** None
- [ ] COV-10 Expand PRD context parsing and sanitization coverage
### Task COV-11

- **ID** COV-11
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Add tests for server configuration validation, CORS origin resolution, and root/options handlers.
- **Checklist**
  * Test `ServerConfig::validate` for port 0, non-localhost token requirement, and open mode bypass.
  * Test `resolve_cors_origin` for open mode, localhost origins, and host-specific matches.
  * Test `options_handler` and `root_handler` include CORS headers and return OK with valid auth.
- **Dependencies** None
- [ ] COV-11 Expand server config and CORS handler coverage
### Task COV-12

- **ID** COV-12
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for state helper functions and cleanup edge cases in `src/state.rs`.
- **Checklist**
  * Test `parse_value` for empty strings, non-numeric mixed values, and boolean strings.
  * Test `cleanup_stale` does not modify sessions that are not running or have pid <= 0.
  * Test `default_state_dir` uses HOME when set and falls back when missing.
- **Dependencies** None
- [ ] COV-12 Increase state helper and cleanup coverage
### Task COV-13

- **ID** COV-13
- **Context Bundle** `src/core.rs`, `src/prd.rs`, `ARCHITECTURE.md`
- **DoD** Add unit tests for task block parsing boundaries and header detection in `src/task.rs`.
- **Checklist**
  * Test `task_blocks_from_contents` when no blocks exist and when blocks end on `---` or `## `.
  * Test `is_task_header` and `is_unchecked_line` with leading whitespace.
  * Test `is_task_block_end` for section headings and separator lines.
- **Dependencies** None
- [ ] COV-13 Expand task parsing edge-case coverage
### Task COV-14

- **ID** COV-14
- **Context Bundle** `src/main.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for update helpers in `src/update.rs` and introduce a test-only override to avoid network calls for latest version resolution.
- **Checklist**
  * Test `parse_release_tag` trimming and missing tag behavior, plus `normalize_version` with v-prefixed input.
  * Test `resolve_in_path` with PATH unset/empty and `install_binary` permission denied (cfg unix).
  * Add a test-only override (env var or injected hook) to exercise `resolve_install_version("latest")` without network.
- **Dependencies** None
- [ ] COV-14 Cover update helper edge cases without network access
### Task COV-15

- **ID** COV-15
- **Context Bundle** `src/main.rs`, `PROCESS.md`, `config/default.yaml`
- **DoD** Add unit tests in `src/verifier.rs` for command parsing and review/check gate evaluation logic.
- **Checklist**
  * Test `resolve_verifier_command`, `resolve_verifier_coverage_min`, and `parse_verifier_command` for empty and invalid inputs.
  * Test `evaluate_review_gate` with sample PR JSON for pending, failed, and passed outcomes.
  * Test `evaluate_check_gate` and review parsing helpers (`parse_review_rating`, `parse_review_issue_count`).
- **Dependencies** None
- [ ] COV-15 Cover verifier command parsing and review gate logic
### Task COV-16

- **ID** COV-16
- **Context Bundle** `src/main.rs`, `PROCESS.md`, `config/default.yaml`
- **DoD** Add unit tests in `src/verifier.rs` for static check utilities, wildcard matching, and duplicate block detection.
- **Checklist**
  * Test `wildcard_match`, `path_matches_any`, `path_is_allowed`, and `path_is_ignored` with allow/ignore patterns and directory paths.
  * Test `line_contains_marker`, `comment_style_for_path`, and `comment_text_len` boundary cases.
  * Test `split_nonempty_blocks`, `block_is_substantive`, and `find_duplicate_blocks` for duplicate detection outputs.
- **Dependencies** None
- [ ] COV-16 Cover verifier static check and duplicate detection helpers
---

## Success Criteria

- `cargo test --workspace` passes and coverage is >= 90% using the verifier coverage command in `config/default.yaml`.
- Each file listed in the coverage report has new tests covering previously untested branches.
- Tests are deterministic and do not require network access or real external CLIs.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
