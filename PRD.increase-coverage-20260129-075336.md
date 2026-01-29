# Project Requirements Document (Template)

## Overview

Raise Rust test coverage from 69.70% to 90% by targeting core logic, public APIs, and error paths in the gralph CLI. Prioritize the modules that drive loop orchestration, state, PRD validation, verifier gates, and backend adapters, while using property-based tests for invariants and avoiding coverage-only work.

## Problem Statement

- Overall coverage is 69.70% (3356/4815), with gaps in core flow, verifier pipeline, and backend adapters.
- Error paths and boundary behavior in core modules are under-tested, reducing confidence in correctness.
- Coverage should be a signal, not a merge gate, until stability improves.

## Solution

Focus on the top 20 percent of modules that drive correctness and risk: core loop, state store, PRD validation, verifier pipeline, and backend adapters. Add tests around public APIs, error handling, and invariants, using property-based tests for parsers and normalization rules. Avoid trivial glue and getters. After tests stabilize, introduce a soft coverage warning target at 65 to 70 percent, then later raise it to 75 to 80 percent.

---

## Functional Requirements

### FR-1: Core Logic Coverage

Add tests for core loop, state management, PRD validation, verifier pipeline, and backend adapters, emphasizing public APIs and error paths.

### FR-2: Invariant-Focused Testing

Introduce property-based tests for parsers and normalization behavior (task blocks, PRD sanitization, config key handling, coverage parsing) to guard invariants without excessive narrow unit tests.

---

## Non-Functional Requirements

### NFR-1: Performance

- Keep test runtime reasonable; cap property-based test sizes and use deterministic seeds when needed.

### NFR-2: Reliability

- Coverage is a non-blocking signal until stability improves.
- Tests that mutate environment variables must use the env lock helper.

---

## Implementation Tasks

### Task COV-CORE-1

- **ID** COV-CORE-1
- **Context Bundle** `src/core.rs`, `ARCHITECTURE.md`
- **DoD** Core loop tests cover prompt rendering, completion detection, and key error paths without changing runtime behavior.
- **Checklist**
  * Add tests for run_iteration error paths (missing task file, empty backend output, parse failures) and raw log copy behavior.
  * Add property-based tests for completion marker and negated promise parsing invariants.
- **Dependencies** None
- [x] COV-CORE-1 Expand core loop coverage for prompt and completion handling
### Task COV-STATE-1

- **ID** COV-STATE-1
- **Context Bundle** `src/state.rs`, `PROCESS.md`
- **DoD** State store tests cover lock, read/write, and cleanup edge cases with correct error reporting.
- **Checklist**
  * Add tests for lock acquisition failures, invalid lock paths, and write_state failure paths.
  * Add tests for cleanup_stale behavior with mixed session data and dead PID handling.
- **Dependencies** None
- [x] COV-STATE-1 Add state store tests for lock and cleanup edges
### Task COV-PRD-1

- **ID** COV-PRD-1
- **Context Bundle** `src/prd.rs`, `PRD.template.md`
- **DoD** PRD validation and sanitization tests cover context bundle filtering, Open Questions removal, and parser invariants.
- **Checklist**
  * Add tests for sanitize_task_block context filtering, fallback selection, and unchecked line normalization.
  * Add property-based tests for task block validation invariants and context entry parsing.
- **Dependencies** None
- [x] COV-PRD-1 Expand PRD validation and sanitization coverage
### Task COV-VERIFIER-1

- **ID** COV-VERIFIER-1
- **Context Bundle** `src/main.rs`, `config/default.yaml`
- **DoD** Verifier tests cover command parsing, coverage extraction, review gate parsing, and static check path filters.
- **Checklist**
  * Add tests for parse_verifier_command, extract_coverage_percent, and coverage warn thresholds.
  * Add tests for review gate rating parsing and check gate pending or failed states.
- **Dependencies** None
- [x] COV-VERIFIER-1 Add verifier parsing and gate evaluation tests
### Task COV-BACKEND-MOD-1

- **ID** COV-BACKEND-MOD-1
- **Context Bundle** `src/backend/mod.rs`, `ARCHITECTURE.md`
- **DoD** Backend utilities tests cover PATH scanning and stream handling error paths.
- **Checklist**
  * Add tests for command_in_path with empty, missing, and mixed PATH entries.
  * Add tests for stream_command_output when stdout or stderr pipes are missing.
- **Dependencies** None
- [x] COV-BACKEND-MOD-1 Expand backend utility coverage for PATH and stream cases
### Task COV-BACKEND-CLAUDE-1

- **ID** COV-BACKEND-CLAUDE-1
- **Context Bundle** `src/backend/claude.rs`, `src/backend/mod.rs`
- **DoD** Claude backend tests cover parsing fallbacks, stream filtering, and argument handling.
- **Checklist**
  * Add tests for parse_text fallback when no result entry exists and for invalid JSON lines.
  * Add tests for run_iteration argument ordering and empty model handling.
- **Dependencies** None
- [x] COV-BACKEND-CLAUDE-1 Add Claude adapter tests for parsing and args
### Task COV-BACKEND-OPENCODE-1

- **ID** COV-BACKEND-OPENCODE-1
- **Context Bundle** `src/backend/opencode.rs`, `src/backend/mod.rs`
- **DoD** OpenCode backend tests cover env flags, stdout and stderr capture, and arg ordering.
- **Checklist**
  * Add tests for OPENCODE_EXPERIMENTAL_LSP_TOOL env propagation and prompt-last ordering.
  * Add tests for model and variant combinations, including empty values.
- **Dependencies** None
- [x] COV-BACKEND-OPENCODE-1 Expand OpenCode adapter tests for env and ordering
### Task COV-BACKEND-GEMINI-1

- **ID** COV-BACKEND-GEMINI-1
- **Context Bundle** `src/backend/gemini.rs`, `src/backend/mod.rs`
- **DoD** Gemini backend tests cover headless flag behavior and argument ordering.
- **Checklist**
  * Add tests ensuring --headless is always present and prompt is last.
  * Add tests for model flag omission when empty and parse_text error paths.
- **Dependencies** None
- [x] COV-BACKEND-GEMINI-1 Add Gemini adapter tests for headless and args
### Task COV-BACKEND-CODEX-1

- **ID** COV-BACKEND-CODEX-1
- **Context Bundle** `src/backend/codex.rs`, `src/backend/mod.rs`
- **DoD** Codex backend tests cover quiet/auto-approve flags, model handling, and error paths.
- **Checklist**
  * Add tests for prompt-last ordering and model flag inclusion or omission.
  * Add tests for parse_text error cases and spawn failures.
- **Dependencies** None
- [x] COV-BACKEND-CODEX-1 Expand Codex adapter tests for args and errors
### Task COV-CONFIG-1

- **ID** COV-CONFIG-1
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Config tests cover key normalization, env precedence, and list rendering invariants.
- **Checklist**
  * Add tests for normalize_key and lookup_value with mixed case and hyphenated keys.
  * Add tests for env override precedence and list rendering with null or tagged values.
- **Dependencies** None
- [x] COV-CONFIG-1 Add config normalization and env override tests
### Task COV-SERVER-1

- **ID** COV-SERVER-1
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Server tests cover auth failures, CORS resolution, and session enrichment edge cases.
- **Checklist**
  * Add tests for stop handler when sessions are missing or state store errors.
  * Add tests for enrich_session status transitions and remaining task calculations.
- **Dependencies** None
- [ ] COV-SERVER-1 Expand server handler and auth error path tests
### Task COV-NOTIFY-1

- **ID** COV-NOTIFY-1
- **Context Bundle** `src/notify.rs`, `README.md`
- **DoD** Notification tests cover payload formatting and HTTP error handling without changing output schema.
- **Checklist**
  * Add tests for format_duration boundaries and generic payload field inclusion.
  * Add tests for send_webhook timeouts and HTTP status failures.
- **Dependencies** None
- [ ] COV-NOTIFY-1 Add notify formatting and HTTP error tests
### Task COV-MAIN-1

- **ID** COV-MAIN-1
- **Context Bundle** `src/main.rs`, `src/cli.rs`
- **DoD** CLI tests cover session naming, worktree helpers, and parser utilities.
- **Checklist**
  * Add tests for session_name sanitization and fallback rules.
  * Add tests for parse_bool_value, resolve_auto_worktree, and branch naming helpers.
- **Dependencies** None
- [ ] COV-MAIN-1 Expand main CLI helper coverage
### Task COV-TASK-1

- **ID** COV-TASK-1
- **Context Bundle** `src/core.rs`, `src/prd.rs`
- **DoD** Task parsing tests cover block termination and unchecked line recognition invariants.
- **Checklist**
  * Add property-based tests for block termination on separators and headings across CRLF input.
  * Add tests for is_task_header and is_unchecked_line near-miss spacing cases.
- **Dependencies** None
- [ ] COV-TASK-1 Add task parsing invariant tests
### Task COV-TESTSUPPORT-1

- **ID** COV-TESTSUPPORT-1
- **Context Bundle** `src/lib.rs`, `src/config.rs`
- **DoD** env_lock tests expand correctness coverage without introducing deadlocks.
- **Checklist**
  * Add tests to confirm env_lock releases under contention and allows subsequent acquisitions.
  * Add tests for env restore sequencing across multiple threads.
- **Dependencies** None
- [ ] COV-TESTSUPPORT-1 Expand env_lock resilience tests
### Task COV-UPDATE-1

- **ID** COV-UPDATE-1
- **Context Bundle** `src/main.rs`, `CHANGELOG.md`
- **DoD** Update workflow tests cover version parsing, archive extraction failures, and PATH resolution.
- **Checklist**
  * Add tests for release_download_url overrides, detect_platform unsupported targets, and empty PATH resolution.
  * Add tests for extract_archive failures and install_binary permission errors.
- **Dependencies** None
- [ ] COV-UPDATE-1 Expand update workflow error-path coverage
### Task COV-CI-1

- **ID** COV-CI-1
- **Context Bundle** `config/default.yaml`, `PROCESS.md`
- **DoD** Soft coverage warning target set to 65 to 70 percent and documented as non-blocking.
- **Checklist**
  * Update verifier.coverage_warn in config to 65 to 70 and confirm warning-only behavior.
  * Document the soft target guidance in PROCESS.
- **Dependencies** COV-CORE-1, COV-STATE-1, COV-PRD-1, COV-VERIFIER-1
- [ ] COV-CI-1 Set initial soft coverage warning target
### Task COV-CI-2

- **ID** COV-CI-2
- **Context Bundle** `config/default.yaml`, `README.md`
- **DoD** Soft coverage warning target raised to 75 to 80 percent after stability is confirmed.
- **Checklist**
  * Update verifier.coverage_warn to the new range and align README guidance.
  * Confirm warning-only behavior remains unchanged.
- **Dependencies** COV-CI-1
- [x] COV-CI-2 Raise soft coverage warning target after stabilization
---

## Success Criteria

- Total coverage reaches at least 90% while focusing on core logic correctness.
- Core modules and backends have tests for public APIs and error paths.
- Property-based tests cover parser and normalization invariants.
- Soft coverage warning target is applied in two stages: 65-70, then 75-80.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
