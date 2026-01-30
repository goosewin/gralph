# Project Requirements Document (Template)

## Overview

Raise automated test coverage for the gralph Rust CLI to 90 percent by targeting the highest-risk core logic first, then expanding to public APIs and error paths across supporting modules. The focus is on correctness of the core loop, PRD validation and parsing, session state persistence, and verifier pipeline behavior, while avoiding coverage chasing in trivial glue code.

## Problem Statement

- Current coverage is 71.49 percent (3445/4819 lines) with uneven depth across key modules.
- Core execution paths (loop orchestration, PRD parsing, state store, verifier gating) still have untested error paths and invariants.
- Coverage is treated as a signal, but current gates and config defaults do not yet reflect a staged, warning-only rollout plan.

## Solution

Prioritize the top 20 percent of critical modules: core loop orchestration, PRD validation and task parsing, state store, and verifier pipeline. Add property-based tests for invariants and targeted unit tests for public APIs and error paths. Keep coverage signal-only for now and stage soft targets after stability, without introducing new blocking gates.

Primary coverage targets by criticality:
- Core loop orchestration in src/core.rs
- PRD validation and task parsing in src/prd.rs and task parsing helpers
- Session state persistence in src/state.rs
- Verifier pipeline parsing and review gate logic

---

## Functional Requirements

### FR-1: Core correctness coverage

Add tests for the core loop, PRD validation, task parsing, and state store with emphasis on error paths and invariants.

### FR-2: Public API and integration coverage

Add tests for public-facing command flows and module APIs across main CLI, server, notify, update, and backend adapters with focus on error handling and argument construction.

### FR-3: Coverage signal staging

Keep coverage as a warning-only signal for now. After stability, introduce a soft target of 65 to 70 percent, then raise to 75 to 80 percent later, without blocking merges.

---

## Non-Functional Requirements

### NFR-1: Test runtime and determinism

- Use bounded proptest case counts and local fixtures only.
- Avoid external network calls and long-running commands; use local stubs.

### NFR-2: Safety and scope control

- Do not refactor behavior solely to increase coverage.
- Do not chase line coverage in trivial getters or glue code.
- Follow env var mutation rules using ENV_LOCK helpers.

---

## Implementation Tasks

### Task COV-CORE-1

- **ID** COV-CORE-1
- **Context Bundle** `src/core.rs`, `ARCHITECTURE.md`, `src/main.rs`
- **DoD** Core loop error paths and prompt rendering invariants are covered with unit and property-based tests, increasing src/core.rs coverage without behavior changes.
- **Checklist**
  * Add tests for run_iteration invalid inputs (missing dir, missing task file, backend not installed) using stub backends.
  * Add proptest invariants for render_prompt_template to ensure placeholders are fully replaced and context section is stable.
  * Add tests for completion detection edge cases (negated promises, trailing whitespace, zero tasks).
- **Dependencies** None
- [x] COV-CORE-1 Increase core loop error-path and invariant coverage
### Task COV-PRD-1

- **ID** COV-PRD-1
- **Context Bundle** `src/prd.rs`, `PRD.template.md`, `PROCESS.md`
- **DoD** PRD validation and sanitization invariants are verified via targeted unit tests and proptest, improving src/prd.rs coverage.
- **Checklist**
  * Add tests for sanitize_task_block context filtering with allowed context lists and fallback selection.
  * Add proptest for stray unchecked line detection and Open Questions removal invariants.
  * Cover absolute path handling when base_dir is set and when canonicalization fails.
- **Dependencies** None
- [x] COV-PRD-1 Expand PRD validation and sanitize invariants
### Task COV-TASK-1

- **ID** COV-TASK-1
- **Context Bundle** `src/core.rs`, `src/prd.rs`
- **DoD** Task block parsing invariants are covered with property-based tests for boundary cases and termination rules.
- **Checklist**
  * Add proptest cases for CRLF and tabbed indentation around task headers and separators.
  * Add tests ensuring H2 headings terminate blocks only when valid and non-empty.
  * Verify near-miss patterns do not terminate blocks or create unchecked matches.
- **Dependencies** COV-PRD-1
- [x] COV-TASK-1 Add property-based task parsing boundary coverage
### Task COV-STATE-1

- **ID** COV-STATE-1
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`
- **DoD** State store error paths and recovery behaviors are covered with deterministic tests that improve src/state.rs coverage.
- **Checklist**
  * Add tests for write_state failure modes (tmp file collisions, rename errors) and validate_state_content errors.
  * Add tests for cleanup_stale with malformed sessions and non-object entries.
  * Cover lock timeout boundaries with short timeouts and contention simulation.
- **Dependencies** None
- [x] COV-STATE-1 Expand state store error-path coverage
### Task COV-VER-1

- **ID** COV-VER-1
- **Context Bundle** `src/main.rs`, `config/default.yaml`, `PROCESS.md`, `README.md`
- **DoD** Verifier command parsing, coverage extraction, and review gate evaluation are tested with edge-case inputs, increasing verifier module coverage.
- **Checklist**
  * Add tests for parse_verifier_command and extract_coverage_percent with mixed output formats.
  * Add tests for review gate parsing when gh JSON fields are missing or malformed.
  * Validate static check settings parsing for invalid numeric and boolean values.
- **Dependencies** COV-CORE-1
- [x] COV-VER-1 Add verifier parsing and gate evaluation tests
### Task COV-CONFIG-1

- **ID** COV-CONFIG-1
- **Context Bundle** `src/config.rs`, `config/default.yaml`, `PROCESS.md`
- **DoD** Config merge and env override invariants are strengthened with unit and property-based tests to raise src/config.rs coverage.
- **Checklist**
  * Add proptest for normalize_key and lookup_value consistency across mixed case and hyphenated keys.
  * Add tests for env override precedence with empty values and legacy aliases.
  * Add tests for value_to_string with nested sequences and tagged values.
- **Dependencies** None
- [x] COV-CONFIG-1 Expand config normalization and override coverage
### Task COV-MAIN-1

- **ID** COV-MAIN-1
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `ARCHITECTURE.md`
- **DoD** CLI helper functions and worktree utilities have new unit tests covering error paths and naming rules, improving src/main.rs coverage.
- **Checklist**
  * Add tests for session_name sanitization and fallback behavior for empty or invalid names.
  * Add tests for auto worktree branch naming and uniqueness helpers with collisions.
  * Add tests for parse_bool_value and resolve_auto_worktree handling invalid inputs.
- **Dependencies** COV-CORE-1
- [ ] COV-MAIN-1 Add CLI helper and worktree utility coverage
### Task COV-SERVER-1

- **ID** COV-SERVER-1
- **Context Bundle** `src/server.rs`, `ARCHITECTURE.md`
- **DoD** Server auth, CORS, and session enrichment edge cases are covered with tests to improve src/server.rs coverage.
- **Checklist**
  * Add tests for resolve_cors_origin with wildcard host and invalid Origin headers.
  * Add tests for stop_handler behavior when tmux session is empty and pid is stale.
  * Add tests for enrich_session when task file is missing or unreadable.
- **Dependencies** COV-STATE-1
- [ ] COV-SERVER-1 Expand server CORS and stop flow coverage
### Task COV-NOTIFY-1

- **ID** COV-NOTIFY-1
- **Context Bundle** `src/notify.rs`, `README.md`
- **DoD** Notification payload formatting and error paths are expanded with tests to increase src/notify.rs coverage.
- **Checklist**
  * Add tests for timeout defaults and invalid timeout inputs in send_webhook.
  * Add tests for format_duration boundary values and message formatting for unknown reasons.
  * Add tests for webhook type detection with mixed-case and query params.
- **Dependencies** None
- [ ] COV-NOTIFY-1 Expand notification formatting and timeout tests
### Task COV-UPDATE-1

- **ID** COV-UPDATE-1
- **Context Bundle** `src/main.rs`, `README.md`, `ARCHITECTURE.md`
- **DoD** Update workflow error paths and version normalization edge cases are tested to improve update module coverage.
- **Checklist**
  * Add tests for resolve_install_version with empty or invalid GRALPH_VERSION env values.
  * Add tests for detect_platform_for with unsupported OS and arch combinations.
  * Add tests for extract_archive error messaging when tar fails or PATH is empty.
- **Dependencies** None
- [ ] COV-UPDATE-1 Expand update workflow error-path tests
### Task COV-BACKEND-MOD-1

- **ID** COV-BACKEND-MOD-1
- **Context Bundle** `src/backend/mod.rs`, `ARCHITECTURE.md`
- **DoD** Backend helper utilities have additional tests for error propagation and stream handling, improving src/backend/mod.rs coverage.
- **Checklist**
  * Add tests for stream_command_output handling mid-stream callback errors and early close.
  * Add tests for command_in_path with relative path segments and non-directory entries.
  * Add tests for backend_from_name error messages on invalid names.
- **Dependencies** None
- [ ] COV-BACKEND-MOD-1 Expand backend helper utility coverage
### Task COV-BACKEND-CLAUDE-1

- **ID** COV-BACKEND-CLAUDE-1
- **Context Bundle** `src/backend/claude.rs`, `config/default.yaml`
- **DoD** Claude adapter parsing and run_iteration argument handling are covered with additional tests to increase coverage.
- **Checklist**
  * Add tests for parse_text fallback when result entries are missing or malformed.
  * Add tests for extract_assistant_texts with mixed content types and nulls.
  * Add tests for run_iteration model flag ordering and prompt placement.
- **Dependencies** COV-BACKEND-MOD-1
- [ ] COV-BACKEND-CLAUDE-1 Add Claude adapter parsing and ordering tests
### Task COV-BACKEND-OPENCODE-1

- **ID** COV-BACKEND-OPENCODE-1
- **Context Bundle** `src/backend/opencode.rs`, `config/default.yaml`
- **DoD** OpenCode adapter argument ordering, env flag usage, and parse_text errors are covered to raise coverage.
- **Checklist**
  * Add tests for variant-only, model-only, and no-flag argument ordering.
  * Add tests for OPENCODE_EXPERIMENTAL_LSP_TOOL env usage and prompt placement.
  * Add tests for parse_text invalid UTF-8 and directory paths.
- **Dependencies** COV-BACKEND-MOD-1
- [ ] COV-BACKEND-OPENCODE-1 Expand OpenCode adapter coverage
### Task COV-BACKEND-GEMINI-1

- **ID** COV-BACKEND-GEMINI-1
- **Context Bundle** `src/backend/gemini.rs`, `config/default.yaml`
- **DoD** Gemini adapter headless flag ordering and error paths are covered with tests to improve coverage.
- **Checklist**
  * Add tests for headless flag ordering with and without model.
  * Add tests for parse_text invalid UTF-8 and missing file paths.
  * Add tests for run_iteration failure propagation on non-zero exit.
- **Dependencies** COV-BACKEND-MOD-1
- [ ] COV-BACKEND-GEMINI-1 Expand Gemini adapter tests
### Task COV-BACKEND-CODEX-1

- **ID** COV-BACKEND-CODEX-1
- **Context Bundle** `src/backend/codex.rs`, `config/default.yaml`
- **DoD** Codex adapter argument ordering and parse_text errors are covered with tests to improve coverage.
- **Checklist**
  * Add tests for quiet and auto-approve flag ordering with prompt placement.
  * Add tests for skipping empty model and preserving prompts with spaces.
  * Add tests for parse_text invalid UTF-8 and missing files.
- **Dependencies** COV-BACKEND-MOD-1
- [ ] COV-BACKEND-CODEX-1 Expand Codex adapter tests
### Task COV-TEST-SUPPORT-1

- **ID** COV-TEST-SUPPORT-1
- **Context Bundle** `PROCESS.md`, `src/config.rs`, `src/state.rs`
- **DoD** Additional env_lock tests cover recovery and sequencing edge cases to maintain or improve test_support coverage.
- **Checklist**
  * Add tests for env_lock reacquisition after multiple sequential drops.
  * Add tests for env_lock ensuring env restoration after panic in guarded scopes.
  * Add tests for env_lock contention with multiple threads and enforced serialization.
- **Dependencies** None
- [ ] COV-TEST-SUPPORT-1 Expand env_lock coverage and resilience tests
---

## Success Criteria

- Total project coverage reaches at least 90 percent, with measurable gains in core logic modules.
- Core modules (loop orchestration, PRD parsing, state store, verifier pipeline) have explicit error-path and invariant tests.
- Property-based tests are added for task parsing and prompt or config invariants.
- Coverage remains a warning-only signal; no new blocking gates are introduced in this phase.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
