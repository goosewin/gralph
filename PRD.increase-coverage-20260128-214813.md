# Project Requirements Document (Template)

## Overview

Raise test coverage for the gralph Rust CLI from the current 68.56% baseline toward 90% by focusing first on core loop correctness, state handling, and PRD/task parsing. Prioritize invariants and error paths in high impact modules, then expand to public APIs. Coverage remains a signal while tests stabilize, not a merge gate.

## Problem Statement

- Coverage is below the 90% goal, with large modules like `src/main.rs` and `src/verifier.rs` under-tested relative to their behavior surface.
- Core logic (loop orchestration, state persistence, PRD/task parsing) includes state machines and parsing rules with error paths that are not fully exercised.
- Current coverage enforcement is inconsistent across config and CI expectations, and should not block merges while correctness work is in flight.

## Solution

Phase testing to maximize correctness gains quickly. Treat the top 20% of modules as the critical path: `src/core.rs`, `src/state.rs`, `src/prd.rs`, and task parsing helpers. Add property-based tests for invariants and a small set of focused unit tests for public APIs and error paths in supporting modules (backends, server, verifier, update, notify, config, CLI helpers). Introduce soft coverage targets later as non-blocking signals, and raise them as the codebase stabilizes.

---

## Functional Requirements

### FR-1: Core Coverage

Improve coverage and correctness of the core loop, state store, and PRD/task parsing by adding tests that validate invariants (parsing boundaries, completion markers, lock behavior) and error handling.

### FR-2: Public API and Error Path Coverage

Add tests for backends, server, verifier, update, notify, config, and CLI helper logic that cover public API behavior and meaningful error paths without targeting trivial glue or getters.

---

## Non-Functional Requirements

### NFR-1: Performance

- Property-based tests must be bounded and deterministic.
- Tests should run under `cargo test --workspace` without long sleeps or external dependencies.

### NFR-2: Reliability

- Tests must use the `ENV_LOCK` helper and avoid direct env mutation.
- Coverage remains informational until soft targets are explicitly enabled.

---

## Implementation Tasks

### Task COV90-CORE-1

- **ID** COV90-CORE-1
- **Context Bundle** `src/core.rs`, `PROCESS.md`
- **DoD** Add unit and property-based tests for run_iteration/run_loop input validation, prompt rendering with context files, and completion marker handling when no tasks remain.
- **Checklist**
  * Tests cover missing task file, empty project dir, and empty backend output error paths.
  * Property tests exercise task block selection and completion promise invariants with bounded cases.
  * Tests use temp dirs and do not touch real repo state.
- **Dependencies** None
- [x] COV90-CORE-1 Add core loop tests for task selection and completion handling
### Task COV90-STATE-1

- **ID** COV90-STATE-1
- **Context Bundle** `src/state.rs`, `PROCESS.md`
- **DoD** Expand state store tests to cover lock timeout behavior, corrupted state recovery, invalid session names, and parse_value edge cases.
- **Checklist**
  * Tests cover lock contention timeout and lock file IO errors.
  * Tests validate corrupted JSON recovery and empty state protection.
  * Tests use env_lock and temporary directories for state files.
- **Dependencies** None
- [x] COV90-STATE-1 Add state store tests for lock and recovery edge cases
### Task COV90-PRD-1

- **ID** COV90-PRD-1
- **Context Bundle** `src/prd.rs`, `PRD.template.md`
- **DoD** Add property-based tests for PRD validation and sanitization invariants, including open questions removal and context bundle filtering.
- **Checklist**
  * Property tests exercise context bundle filtering with allowed lists and base dir overrides.
  * Validation tests cover stray unchecked lines and missing required fields.
  * Sanitization tests ensure only one unchecked task line remains per block.
- **Dependencies** None
- [x] COV90-PRD-1 Add PRD validation and sanitize invariant tests
### Task COV90-TASK-1

- **ID** COV90-TASK-1
- **Context Bundle** `src/core.rs`, `src/prd.rs`
- **DoD** Add property-based tests for task block parsing boundaries, including separator and heading termination, CRLF handling, and unchecked line detection.
- **Checklist**
  * Tests cover termination on `---` and `##` headings across whitespace variants.
  * Tests cover CRLF input and ensure parsed blocks do not retain carriage returns.
  * Tests verify unchecked line detection only within task blocks.
- **Dependencies** None
- [x] COV90-TASK-1 Add property tests for task parsing invariants
### Task COV90-CONFIG-1

- **ID** COV90-CONFIG-1
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Add tests for config precedence, env override resolution, key normalization, and list rendering edge cases.
- **Checklist**
  * Tests cover default-global-project merge precedence and env override priority.
  * Tests validate normalize_key for hyphen and case handling.
  * Tests cover sequence and null flattening into CSV values.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-CONFIG-1 Add config precedence and normalization tests
 - [x] COV90-CONFIG-1 Add config precedence and normalization tests
### Task COV90-VER-1

- **ID** COV90-VER-1
- **Context Bundle** `src/main.rs`, `config/default.yaml`, `PROCESS.md`
- **DoD** Add tests for verifier command parsing, coverage percent extraction, review gate rating and issue parsing, and gh output error handling.
- **Checklist**
  * Tests validate parse_verifier_command error cases and quoting behavior.
  * Tests cover extract_coverage_percent for common tarpaulin output variants.
  * Tests cover review gate rating scaling and issue count detection.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-VER-1 Add verifier parsing and review gate tests
### Task COV90-MAIN-1

- **ID** COV90-MAIN-1
- **Context Bundle** `src/main.rs`, `ARCHITECTURE.md`, `PROCESS.md`
- **DoD** Add tests for CLI helper logic including session_name sanitization, validate_task_id, parse_bool_value, and auto worktree name generation.
- **Checklist**
  * Tests cover sanitize_session_name and default fallback behaviors.
  * Tests cover validate_task_id acceptance and rejection cases.
  * Tests cover auto_worktree_branch_name format and parse_bool_value variants.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-MAIN-1 Add CLI helper tests for worktree and session logic
### Task COV90-SERVER-1

- **ID** COV90-SERVER-1
- **Context Bundle** `src/server.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for auth and CORS edge cases plus session enrichment when task files are missing or unreadable.
- **Checklist**
  * Tests cover open mode CORS wildcard and non-localhost token requirements.
  * Tests cover unauthorized responses for malformed Authorization headers.
  * Tests cover status handlers when session data is incomplete.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-SERVER-1 Add server auth and CORS error path tests
### Task COV90-NOTIFY-1

- **ID** COV90-NOTIFY-1
- **Context Bundle** `src/notify.rs`, `README.md`
- **DoD** Add tests for webhook payload formatting, duration formatting edge cases, and HTTP error handling.
- **Checklist**
  * Tests cover formatting for discord, slack, and generic payloads.
  * Tests cover failure reason mapping and default values.
  * Tests cover non-2xx status handling and timeout defaults.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-NOTIFY-1 Add notify formatting and HTTP error tests
### Task COV90-BACKEND-MOD-1

- **ID** COV90-BACKEND-MOD-1
- **Context Bundle** `src/backend/mod.rs`, `config/default.yaml`
- **DoD** Add tests for PATH scanning and stream_command_output error handling.
- **Checklist**
  * Tests cover empty PATH, relative PATH entries, and file entries.
  * Tests cover missing stdout or stderr streams and non-zero exit status.
  * Tests cover on_line error propagation.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-BACKEND-MOD-1 Add backend module PATH and streaming tests
### Task COV90-BACKEND-CLAUDE-1

- **ID** COV90-BACKEND-CLAUDE-1
- **Context Bundle** `src/backend/claude.rs`, `config/default.yaml`
- **DoD** Add tests for Claude stream parsing, result selection, model flag handling, and IO error paths.
- **Checklist**
  * Tests cover extract_assistant_texts with malformed entries and mixed content.
  * Tests cover parse_text selection of last valid result.
  * Tests cover run_iteration model flag inclusion and empty prompt validation.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-BACKEND-CLAUDE-1 Expand Claude backend parsing and error tests
### Task COV90-BACKEND-OPENCODE-1

- **ID** COV90-BACKEND-OPENCODE-1
- **Context Bundle** `src/backend/opencode.rs`, `config/default.yaml`
- **DoD** Add tests for OpenCode env flags, argument ordering, and parse_text IO error cases.
- **Checklist**
  * Tests cover OPENCODE_EXPERIMENTAL_LSP_TOOL environment flag handling.
  * Tests cover model and variant ordering with prompt last.
  * Tests cover parse_text errors for missing file and directory path.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-BACKEND-OPENCODE-1 Expand OpenCode backend env and ordering tests
### Task COV90-BACKEND-GEMINI-1

- **ID** COV90-BACKEND-GEMINI-1
- **Context Bundle** `src/backend/gemini.rs`, `config/default.yaml`
- **DoD** Add tests for Gemini headless flag ordering, model handling, and parse_text IO errors.
- **Checklist**
  * Tests cover headless flag placement and prompt last ordering.
  * Tests cover empty model skipping.
  * Tests cover parse_text errors for missing file or directory.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-BACKEND-GEMINI-1 Expand Gemini backend command and error tests
### Task COV90-BACKEND-CODEX-1

- **ID** COV90-BACKEND-CODEX-1
- **Context Bundle** `src/backend/codex.rs`, `config/default.yaml`
- **DoD** Add tests for Codex flag ordering, model handling, and parse_text IO error cases.
- **Checklist**
  * Tests cover quiet and auto-approve flags with and without model.
  * Tests cover empty model skipping.
  * Tests cover parse_text errors for missing file or directory.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-BACKEND-CODEX-1 Expand Codex backend flags and error tests
### Task COV90-UPDATE-1

- **ID** COV90-UPDATE-1
- **Context Bundle** `src/main.rs`, `README.md`
- **DoD** Add update workflow tests for invalid version parsing, download failures, extract failures, and permission denied install paths using test overrides.
- **Checklist**
  * Tests cover invalid version strings and missing release tag handling.
  * Tests cover download failures and empty archive extraction failures.
  * Tests cover permission denied behavior with GRALPH_INSTALL_DIR overrides.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-UPDATE-1 Add update workflow error path tests
### Task COV90-TESTSUPPORT-1

- **ID** COV90-TESTSUPPORT-1
- **Context Bundle** `PROCESS.md`, `src/config.rs`
- **DoD** Add env_lock stress tests for poison recovery and serialized env mutation under contention.
- **Checklist**
  * Tests simulate panics while holding env_lock and verify recovery.
  * Tests confirm env values are restored safely after guarded changes.
  * Tests avoid direct env mutation outside env_lock.
- **Dependencies** COV90-CORE-1, COV90-STATE-1, COV90-PRD-1, COV90-TASK-1
- [ ] COV90-TESTSUPPORT-1 Add env_lock stress and recovery tests
### Task COV90-CI-1

- **ID** COV90-CI-1
- **Context Bundle** `config/default.yaml`, `PROCESS.md`, `ARCHITECTURE.md`
- **DoD** Add a non-blocking soft coverage target (65 to 70%) as a warning signal and document it in process and architecture guidance.
- **Checklist**
  * Config includes a soft coverage target distinct from the hard minimum.
  * Documentation states coverage is informational until the soft target is enabled.
  * Verifier output warns when below the soft target without blocking merges.
- **Dependencies** COV90-VER-1
- [x] COV90-CI-1 Introduce soft coverage warning target at 65 to 70 percent
### Task COV90-CI-2

- **ID** COV90-CI-2
- **Context Bundle** `config/default.yaml`, `PROCESS.md`, `CHANGELOG.md`
- **DoD** Raise the soft coverage target to 75 to 80 percent after stabilization and document the change.
- **Checklist**
  * Config soft target updated to 75 to 80 percent range.
  * PROCESS guidance updated to reflect the new signal level.
  * CHANGELOG entry added with the task ID.
- **Dependencies** COV90-CI-1
- [ ] COV90-CI-2 Raise soft coverage target to 75 to 80 percent
---

## Success Criteria

- Core modules (`src/core.rs`, `src/state.rs`, `src/prd.rs`, task parsing) have property-based tests covering key invariants and error paths.
- Public APIs and error paths in backends, server, verifier, update, notify, config, and CLI helpers are covered by focused tests.
- Overall coverage reported by `cargo tarpaulin --workspace --fail-under 90 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*` reaches 90% without adding trivial tests.
- Coverage remains non-blocking until soft targets are explicitly enabled.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
