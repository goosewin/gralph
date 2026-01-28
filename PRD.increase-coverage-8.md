# Project Requirements Document (Template)

## Overview

Raise test coverage of the gralph Rust CLI from 61.59% to 90% by adding focused tests in core logic, state handling, PRD validation, verifier pipeline, and remaining modules. Prioritize correctness of core behavior and error paths, treat coverage as a signal (not a merge gate), and use property-based tests for parsing invariants where they provide broader confidence.

## Problem Statement

- Overall line coverage is 61.59%, with several critical modules below 90% (for example: `src/main.rs` 33%, `src/verifier.rs` 51%, `src/prd.rs` 71%, `src/update.rs` 63%).
- Core loop, PRD validation, and verifier logic have untested error paths that risk regressions during automation and verification flows.
- Coverage expectations exist in tooling, but merges should not be blocked yet; improvements must focus on correctness over raw numbers.

## Solution

Focus first on the top 20% of modules that drive correctness and safety (core loop, state store, PRD validation, verifier pipeline), adding tests around public APIs and error paths. Use property-based tests for parsing invariants instead of many narrow unit tests. After core logic stabilizes, expand to supporting modules and backend adapters. Keep merges unblocked by coverage during this effort, and later propose a soft CI target of 65-70%, then 75-80% when the codebase slows down.

---

## Functional Requirements

### FR-1: Core Feature

Increase coverage in core logic modules (`src/core.rs`, `src/state.rs`, `src/prd.rs`, `src/verifier.rs`) by adding tests for public APIs and error paths, emphasizing correctness of loop orchestration, state locking, PRD validation, and verifier gating.

### FR-2: Secondary Feature

Increase coverage in supporting modules and backend adapters (`src/main.rs`, `src/config.rs`, `src/server.rs`, `src/notify.rs`, `src/update.rs`, `src/backend/*`, `src/task.rs`, `src/test_support.rs`) with targeted tests for non-trivial behaviors and error handling.

---

## Non-Functional Requirements

### NFR-1: Performance

- Tests must be deterministic, offline, and bounded in runtime (use temp dirs and local fixtures; avoid network dependencies).

### NFR-2: Reliability

- Do not refactor purely to increase coverage; follow KISS, YAGNI, TDA, DRY, and SOLID.
- Do not add new coverage-based merge gates during this effort; coverage remains a signal.

---

## Implementation Tasks

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/core.rs`, `src/config.rs`
- **DoD** Add tests for run_loop/run_iteration validation and prompt template resolution so remaining uncovered branches in core orchestration are exercised.
- **Checklist**
  * Add tests for run_loop invalid inputs (empty project dir, missing task file) and completion checks when the promise line is malformed.
  * Add tests for resolve_prompt_template fallback order and raw output logging when backend output is empty.
- **Dependencies** None
- [x] COV-1 Expand core loop error-path coverage
### Task COV-2

- **ID** COV-2
- **Context Bundle** `src/state.rs`
- **DoD** Cover StateStore environment overrides and cleanup behavior for stale sessions in both Mark and Remove modes.
- **Checklist**
  * Add tests for StateStore::new_from_env reading GRALPH_STATE_DIR/FILE/LOCK/TIMEOUT and handling invalid timeout values.
  * Add tests for cleanup_stale updates when running sessions have dead PIDs, confirming Mark vs Remove outcomes.
- **Dependencies** None
- [ ] COV-2 Expand state store env and cleanup coverage
### Task COV-3

- **ID** COV-3
- **Context Bundle** `src/prd.rs`, `README.md`
- **DoD** Expand PRD validation and sanitization tests for base_dir overrides, allowed context filtering, and parsing invariants.
- **Checklist**
  * Add tests for prd_validate_file with base_dir_override and absolute context paths inside/outside the repo, plus allowed_context filtering.
  * Add property-based tests for extract_context_entries and sanitize_task_block to ensure a single unchecked line and valid fallback context.
- **Dependencies** None
- [ ] COV-3 Expand PRD validation and sanitization coverage
### Task COV-4

- **ID** COV-4
- **Context Bundle** `src/config.rs`
- **DoD** Increase coverage for verifier helper logic across static checks, PR base resolution, and review/check gate parsing.
- **Checklist**
  * Add tests for resolve_verifier_pr_base (origin/HEAD present vs missing) and resolve_pr_template_path error handling.
  * Add tests for static check helpers (normalize_pattern, path_is_allowed/ignored, read_text_file size and utf8) and gate parsing edge cases.
- **Dependencies** None
- [ ] COV-4 Expand verifier helper coverage
### Task COV-5

- **ID** COV-5
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Cover remaining config path resolution and lookup edge cases without changing behavior.
- **Checklist**
  * Add tests for config_paths when project_dir is a file or missing, and for custom project config names that do not exist.
  * Add tests for normalize_key/lookup_value with empty segments and for exists() returning false on invalid keys or mapping-only values.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-5 Expand config path and lookup coverage
### Task COV-6

- **ID** COV-6
- **Context Bundle** `src/main.rs`, `src/config.rs`, `README.md`
- **DoD** Add CLI helper tests for config mutation, PRD output resolution, and log resolution error paths.
- **Checklist**
  * Add tests for cmd_config_set writing nested keys and preserving mappings, and for resolve_prd_output when output exists without force.
  * Add tests for resolve_log_file when session data is missing dir, and for read_readme_context_files skipping non-md or spaced entries.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-6 Expand CLI helper coverage in main
### Task COV-7

- **ID** COV-7
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Cover server configuration parsing, auth success paths, and CORS behavior branches.
- **Checklist**
  * Add tests for ServerConfig::from_env and ServerConfig::addr failures on invalid host/port inputs.
  * Add tests for check_auth success with valid Bearer token and apply_cors behavior for open mode and explicit host.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-7 Expand server config and auth coverage
### Task COV-8

- **ID** COV-8
- **Context Bundle** `src/notify.rs`
- **DoD** Increase coverage for notification formatting helpers and failure reason mappings.
- **Checklist**
  * Add tests for emphasized_session and format_complete_description, and for format_failure_description mappings (max_iterations, error, manual_stop).
  * Add tests for discord/slack/generic payloads using manual_stop and unknown failure reasons to ensure message and fields align.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-8 Expand notification formatting coverage
### Task COV-9

- **ID** COV-9
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Cover update version resolution and download/extract error paths with local fixtures.
- **Checklist**
  * Add tests for resolve_install_version when GRALPH_VERSION is set to a concrete version and when GRALPH_TEST_LATEST_TAG is blank.
  * Add tests for download_release non-200 responses and extract_archive behavior with empty PATH or missing tar binary.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-9 Expand update workflow error-path coverage
### Task COV-10

- **ID** COV-10
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Cover remaining backend helper branches around output streaming and PATH scanning.
- **Checklist**
  * Add tests for spawn_reader/stream_command_output with trailing lines without newline and for early channel closure.
  * Add tests for command_in_path with PATH entries that are files or relative paths to ensure only directories are scanned.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-10 Expand backend helper coverage
### Task COV-11

- **ID** COV-11
- **Context Bundle** `src/backend/claude.rs`
- **DoD** Increase parser coverage for mixed JSON/non-JSON streams.
- **Checklist**
  * Add tests for parse_text ignoring invalid JSON lines and returning raw contents when no result entries exist.
  * Add tests for extract_result_text and extract_assistant_texts when required fields are missing or types are mismatched.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-11 Expand Claude adapter parsing coverage
### Task COV-12

- **ID** COV-12
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** Expand adapter coverage for streaming and argument edge cases.
- **Checklist**
  * Add tests for run_iteration when output is emitted only on stderr and ensure output file captures all lines.
  * Add tests for OPENCODE_LSP_ENV and argument ordering when only model or only variant is provided.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-12 Expand OpenCode adapter coverage
### Task COV-13

- **ID** COV-13
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Add tests for gemini adapter error paths and argument defaults.
- **Checklist**
  * Add tests for parse_text when response_file is a directory and for run_iteration with empty model still including --headless.
  * Add tests for check_installed when PATH is unset and when PATH contains non-directory entries.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-13 Expand Gemini adapter coverage
### Task COV-14

- **ID** COV-14
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Cover codex adapter edge cases for parse_text and argument handling.
- **Checklist**
  * Add tests for parse_text on directory paths and for run_iteration with empty model omitting --model while keeping quiet/auto-approve flags.
  * Add tests for check_installed when PATH is unset or empty.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-14 Expand Codex adapter coverage
### Task COV-15

- **ID** COV-15
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add property-based tests for parsing invariants beyond line coverage.
- **Checklist**
  * Add proptest cases for CRLF and mixed whitespace to ensure task_blocks_from_contents preserves only task blocks.
  * Add tests for is_task_block_end and is_unchecked_line with tab spacing and near-miss patterns.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-15 Expand task parsing invariants
### Task COV-16

- **ID** COV-16
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Expand concurrency tests for env_lock robustness.
- **Checklist**
  * Add a repeated panic/recovery test across multiple threads to confirm the lock remains usable.
  * Add a higher-contention stress test to ensure max_active never exceeds 1.
- **Dependencies** COV-1, COV-2, COV-3, COV-4
- [ ] COV-16 Expand env_lock stress coverage
---

## Success Criteria

- Overall tarpaulin line coverage reaches 90% or higher without adding new coverage-based merge gates.
- Core modules (`src/core.rs`, `src/state.rs`, `src/prd.rs`, `src/verifier.rs`) have new tests covering public APIs and error paths.
- Property-based tests exist for task and PRD parsing invariants, and tests remain deterministic and offline.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
