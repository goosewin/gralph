# Project Requirements Document (Template)

## Overview

Raise Rust test coverage to 90 percent by focusing first on core logic modules and correctness-critical paths, then expanding to public APIs and error handling across the rest of the codebase. Priority modules (top 20 percent by criticality) are `src/core.rs`, `src/prd.rs`, `src/state.rs`, and the verifier pipeline logic. Coverage is a signal, not a quality verdict, and should not block merges yet.

## Problem Statement

- Overall coverage is 71.45 percent, below the desired 90 percent target.
- Core loop, PRD validation, state storage, and verifier logic have error paths and invariants that are not fully exercised.
- Tests exist but are unevenly distributed, with some files already saturated and others missing targeted edge-case coverage.
- Current efforts risk chasing line coverage rather than correctness if not scoped to core logic and public APIs.

## Solution

Concentrate testing on the most critical modules first, especially core loop orchestration, PRD validation, state persistence, and verifier gating logic. Add tests for public APIs and error paths across all remaining modules, and introduce property-based tests for invariants where they reduce redundant unit tests. After coverage stabilizes, configure a warning-only soft coverage target at 65 to 70 percent, then raise the warning target to 75 to 80 percent later when development pace slows.

---

## Functional Requirements

### FR-1: Core Logic Coverage

Core loop, PRD validation, state storage, and verifier pipeline logic must have tests that cover happy paths, error paths, and invariants.

### FR-2: Public APIs and Error Paths

Each module listed in the coverage report must have targeted tests for public APIs and failure conditions, avoiding trivial getters or glue code.

### FR-3: Property-Based Invariants

Where invariants exist (parsers, formatting, selection logic), add property-based tests to cover broad input space without excessive unit tests.

---

## Non-Functional Requirements

### NFR-1: Test Discipline

- Coverage is a signal, not a gate; do not block merges on coverage in this phase.
- Prefer correctness tests over line-count targets.
- Avoid tests that only exist to move the number.

### NFR-2: Maintainability

- No refactors that exist only to increase coverage.
- Follow KISS, DRY, YAGNI, SOLID, and TDA.
- Environment mutation in tests must use the existing env_lock helpers.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task COV-CORE-1

- **ID** COV-CORE-1
- **Context Bundle** `src/core.rs`, `ARCHITECTURE.md`
- **DoD** Add tests in `src/core.rs` for run_iteration input validation, backend output handling, and prompt rendering invariants, including at least one property-based test.
- **Checklist**
  * Add unit tests for run_iteration invalid inputs (empty project_dir, iteration 0, max_iterations 0, missing task file, backend not installed).
  * Add tests for empty backend output handling and raw log path naming.
  * Add a proptest that asserts render_prompt_template replaces placeholders and only emits Context Files section when provided.
- **Dependencies** None
- [x] COV-CORE-1 Cover core loop validation and prompt rendering invariants
---

### Task COV-PRD-1

- **ID** COV-PRD-1
- **Context Bundle** `src/prd.rs`, `PRD.template.md`, `ARCHITECTURE.md`
- **DoD** Add tests in `src/prd.rs` for validation and sanitization around context bundles, open questions removal, and task block field detection, with property-based invariants for context parsing.
- **Checklist**
  * Add unit tests for sanitize_task_block filtering invalid context entries and fallback selection.
  * Add tests for validate_task_block with absolute paths inside and outside repo base, plus missing context.
  * Add a proptest to ensure extract_context_entries collects only backticked paths and de-duplicates entries.
- **Dependencies** None
- [x] COV-PRD-1 Expand PRD validation and sanitize invariants
---

### Task COV-STATE-1

- **ID** COV-STATE-1
- **Context Bundle** `src/state.rs`, `ARCHITECTURE.md`
- **DoD** Add tests in `src/state.rs` for lock acquisition boundaries, state file error propagation, and cleanup_stale behavior for malformed sessions.
- **Checklist**
  * Add tests for with_lock error paths when lock file parent is missing or a file.
  * Add tests for cleanup_stale with pid <= 0 and malformed session maps preserving state.
  * Add tests for read_state and write_state when state_file is a directory or temp write fails.
- **Dependencies** None
- [x] COV-STATE-1 Cover state lock and cleanup edge cases
---

### Task COV-VER-1

- **ID** COV-VER-1
- **Context Bundle** `src/main.rs`, `config/default.yaml`, `ARCHITECTURE.md`
- **DoD** Add tests in `src/verifier.rs` for command parsing, coverage percent extraction, review gate parsing, and check rollup evaluation for pending and failed states.
- **Checklist**
  * Add tests for parse_verifier_command with quoted args and empty command errors.
  * Add tests for extract_coverage_percent using typical tarpaulin output patterns and fallback rules.
  * Add tests for parse_review_rating and parse_review_issue_count with fractions and percent formats.
- **Dependencies** None
- [x] COV-VER-1 Expand verifier parsing and review gate coverage
---

### Task COV-MAIN-1

- **ID** COV-MAIN-1
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `PROCESS.md`
- **DoD** Add tests for CLI helpers and worktree naming logic, including session_name sanitization, parse_bool_value, and auto worktree branch naming.
- **Checklist**
  * Add tests for sanitize_session_name and session_name fallbacks on empty or invalid input.
  * Add tests for parse_bool_value and resolve_auto_worktree with config overrides.
  * Add tests for auto_worktree_branch_name formatting with known timestamp strings.
- **Dependencies** COV-VER-1
- [x] COV-MAIN-1 Cover CLI helper and auto worktree utilities
---

### Task COV-CONFIG-1

- **ID** COV-CONFIG-1
- **Context Bundle** `src/config.rs`, `config/default.yaml`, `README.md`
- **DoD** Add tests for config path precedence and env override edge cases, including empty values and hyphenated keys, plus one property-based test for key normalization.
- **Checklist**
  * Add tests for config_paths ordering with default, global, and project overrides.
  * Add tests for resolve_env_override with empty values and hyphenated keys.
  * Add a proptest for normalize_key stability in lookup behavior.
- **Dependencies** COV-VER-1
- [x] COV-CONFIG-1 Expand config precedence and override coverage
---

### Task COV-SERVER-1

- **ID** COV-SERVER-1
- **Context Bundle** `src/server.rs`, `src/state.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for HTTP handlers around auth, CORS, and session enrichment error paths, including stop_handler and fallback behavior.
- **Checklist**
  * Add tests for resolve_cors_origin with wildcard host, open mode, and mismatched origins.
  * Add tests for status_name_handler and stop_handler when state store returns errors or session is missing.
  * Add tests for enrich_session when dir is missing or status is stale.
- **Dependencies** COV-VER-1
- [x] COV-SERVER-1 Expand server auth, CORS, and session enrichment coverage
---

### Task COV-NOTIFY-1

- **ID** COV-NOTIFY-1
- **Context Bundle** `src/notify.rs`, `README.md`
- **DoD** Add tests for webhook payload formatting and error handling, including invalid inputs and non-2xx responses.
- **Checklist**
  * Add tests for format_duration boundaries and failure_reason mapping.
  * Add tests for detect_webhook_type with mixed case and non-matching URLs.
  * Add tests for send_webhook handling of invalid URLs and timeout behavior with local test servers.
- **Dependencies** COV-VER-1
- [x] COV-NOTIFY-1 Cover notification payload and HTTP error paths
---

### Task COV-BACKEND-MOD-1

- **ID** COV-BACKEND-MOD-1
- **Context Bundle** `src/backend/mod.rs`, `ARCHITECTURE.md`
- **DoD** Add tests for backend selection and command output streaming error cases, including missing stdout or stderr and early termination.
- **Checklist**
  * Add tests for backend_from_name unknown backend errors and stable models list.
  * Add tests for command_in_path with relative or empty PATH segments and non-directory entries.
  * Add tests for stream_command_output with stderr-only output and early close scenarios.
- **Dependencies** COV-VER-1
- [ ] COV-BACKEND-MOD-1 Expand backend helper streaming and PATH coverage
---

### Task COV-BACKEND-CLAUDE-1

- **ID** COV-BACKEND-CLAUDE-1
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`
- **DoD** Add tests for parse_text fallbacks and stream parsing of assistant and result entries, plus run_iteration argument ordering.
- **Checklist**
  * Add tests for extract_assistant_texts and extract_result_text with malformed JSON shapes.
  * Add tests for parse_text returning last valid result vs raw contents fallback.
  * Add tests for run_iteration model flag ordering and skipping empty model values.
- **Dependencies** COV-VER-1
- [ ] COV-BACKEND-CLAUDE-1 Expand Claude backend parsing and arg ordering coverage
---

### Task COV-BACKEND-OPENCODE-1

- **ID** COV-BACKEND-OPENCODE-1
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** Add tests for env flag injection, argument ordering, and output capture behavior for stdout and stderr mixing.
- **Checklist**
  * Add tests for OPENCODE_EXPERIMENTAL_LSP_TOOL env set and run args ordering.
  * Add tests for run_iteration with model and variant combinations, including empty values.
  * Add tests for parse_text invalid UTF-8 and missing file paths.
- **Dependencies** COV-VER-1
- [ ] COV-BACKEND-OPENCODE-1 Expand OpenCode backend env and output coverage
---

### Task COV-BACKEND-GEMINI-1

- **ID** COV-BACKEND-GEMINI-1
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Add tests for headless flag handling, model ordering, and parse_text error cases.
- **Checklist**
  * Add tests for run_iteration arg ordering with --headless and optional --model.
  * Add tests for skipping whitespace-only model values.
  * Add tests for parse_text invalid UTF-8 and directory path errors.
- **Dependencies** COV-VER-1
- [ ] COV-BACKEND-GEMINI-1 Expand Gemini backend command and error coverage
---

### Task COV-BACKEND-CODEX-1

- **ID** COV-BACKEND-CODEX-1
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Add tests for flag ordering and error propagation for non-zero exit and invalid output paths.
- **Checklist**
  * Add tests for run_iteration arg ordering with --quiet and --auto-approve and optional --model.
  * Add tests for skipping empty model values and keeping prompt last.
  * Add tests for parse_text invalid UTF-8 and directory path errors.
- **Dependencies** COV-VER-1
- [ ] COV-BACKEND-CODEX-1 Expand Codex backend flag ordering and error coverage
---

### Task COV-TASK-1

- **ID** COV-TASK-1
- **Context Bundle** `src/core.rs`, `src/prd.rs`
- **DoD** Add tests in `src/task.rs` for task block parsing invariants with mixed whitespace, CRLF, and near-miss headings, including property-based coverage.
- **Checklist**
  * Add a proptest for task_blocks_from_contents termination on separators and H2 headings with CRLF.
  * Add unit tests for is_task_block_end and is_task_header near-miss spacing and tab edge cases.
  * Add tests for is_unchecked_line with mixed whitespace and invalid spacing.
- **Dependencies** COV-VER-1
- [ ] COV-TASK-1 Cover task parsing boundary conditions
---

### Task COV-TEST-SUPPORT-1

- **ID** COV-TEST-SUPPORT-1
- **Context Bundle** `PROCESS.md`, `src/lib.rs`
- **DoD** Add tests in `src/test_support.rs` that validate env_lock serialization, poison recovery, and safe env restore sequences.
- **Checklist**
  * Add tests for env_lock recovery after panic and subsequent acquisition.
  * Add tests for serialized access under high contention with barriers.
  * Add tests that env_lock restores original env values after guard drop.
- **Dependencies** COV-VER-1
- [ ] COV-TEST-SUPPORT-1 Expand env_lock reliability coverage
---

### Task COV-UPDATE-1

- **ID** COV-UPDATE-1
- **Context Bundle** `src/main.rs`, `ARCHITECTURE.md`, `README.md`
- **DoD** Add tests in `src/update.rs` for release parsing, download and extract error paths, and install_binary permission handling using local test servers.
- **Checklist**
  * Add tests for Version::parse rejecting prerelease, build metadata, and invalid segments.
  * Add tests for resolve_install_version honoring GRALPH_VERSION and test release overrides.
  * Add tests for extract_archive failures and install_binary permission denied behavior.
- **Dependencies** COV-VER-1
- [ ] COV-UPDATE-1 Expand update flow error-path coverage
---

### Task COV-CI-1

- **ID** COV-CI-1
- **Context Bundle** `config/default.yaml`, `README.md`, `PROCESS.md`
- **DoD** Set warning-only coverage target to 65 to 70 percent via verifier.coverage_warn and document that it is non-blocking; keep coverage_min unchanged.
- **Checklist**
  * Update config/default.yaml verifier.coverage_warn to a chosen value in the 65-70 range.
  * Update README and PROCESS to describe warning-only target and no merge block.
  * Record staged plan to raise the warning target after stabilization.
- **Dependencies** COV-UPDATE-1
- [ ] COV-CI-1 Add soft coverage warning target at 65-70 percent
---

### Task COV-CI-2

- **ID** COV-CI-2
- **Context Bundle** `config/default.yaml`, `README.md`, `PROCESS.md`
- **DoD** Raise warning-only coverage target to 75 to 80 percent once coverage stays stable for at least two consecutive cycles; update docs accordingly.
- **Checklist**
  * Bump verifier.coverage_warn to a chosen value in the 75-80 range.
  * Update README and PROCESS to reflect new target and stability criteria.
  * Confirm warning-only behavior remains unchanged.
- **Dependencies** COV-CI-1
- [ ] COV-CI-2 Raise soft coverage warning target to 75-80 percent
---

## Success Criteria

- Overall coverage reaches 90 percent and remains stable across two consecutive test runs.
- Core modules have tests for error paths and invariants, including at least one property-based test per core area.
- Warning-only coverage target is documented and staged at 65-70 percent, with a planned raise to 75-80 percent later.
- No refactors were made solely to increase coverage.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
