# Project Requirements Document (Template)

## Overview

Improve maintainability and testability of the gralph Rust CLI by separating pure decision logic from side effects in orchestration-heavy modules and adding high-value tests for hard-to-reach behavior. Intended users are maintainers and contributors who evolve the CLI loop, verifier pipeline, and PRD validation.

## Problem Statement

- Loop orchestration in `src/app/loop_session.rs` mixes config resolution, state updates, process spawning, notifications, and verifier auto-run, making decisions hard to unit test.
- The verifier pipeline in `src/verifier.rs` blends parsing, coverage gating, PR creation, review gate polling, and static checks, leaving key outcomes under-tested.
- PRD validation and sanitization in `src/prd.rs` combines file IO and parsing, limiting coverage for serialization output, open questions removal, and context bundle path handling.
- Coverage targets and guardrails require tests focused on state transitions, stop conditions, retries, verifier outcomes, and PRD parsing/serialization.

## Solution

Introduce small, pure helper functions inside the existing modules and redirect side-effecting code to call those helpers. Add targeted unit tests for decision logic and parsing edge cases, using existing test infrastructure and keeping external behavior unchanged.

---

## Functional Requirements

### FR-1: Loop Session Behavior Parity

The loop session commands must preserve current behavior for start, run-loop, resume, stop, status, and logs, including strict PRD validation, state store updates, and notification triggers. Auto worktree handling and session tracking must remain consistent with existing config precedence and defaults.

### FR-2: Verifier Pipeline Parity

The verifier pipeline must continue to run tests, parse coverage output, enforce coverage_min, warn below coverage_warn, run static checks, create PRs via gh using templates, evaluate the review gate, and merge when gates pass. No changes to CLI outputs or user-facing flows.

---

## Non-Functional Requirements

### NFR-1: Performance

- Preserve existing command execution order and polling intervals.
- Avoid adding new external commands or heavy abstractions.

### NFR-2: Reliability

- All existing tests pass; coverage remains at or above the configured coverage_min (90 by default in config).
- Stop conditions and error paths remain deterministic and observable via state updates.

### NFR-3: Maintainability and Testability

- Prefer small, incremental refactors with minimal dependency seams.
- Add high-value tests only; avoid log-only or boilerplate assertions.

---

## Implementation Tasks

### Task LS-1

- **ID** LS-1
- **Context Bundle** `ARCHITECTURE.md`, `PROCESS.md`, `src/core.rs`, `src/config.rs`, `src/cli.rs`
- **DoD** Extract pure helpers in loop session for resolving task_file, max_iterations, completion_marker, backend/model defaults, and strict PRD gating; update run-loop flow to use helpers without behavior change; add unit tests for config and CLI precedence, including opencode default model fallback.
- **Checklist**
  * Unit tests cover args vs config vs default precedence for loop settings.
  * Helper outputs match existing behavior for strict PRD validation and model resolution.
- **Dependencies** None
- [x] LS-1 Extract loop setting decisions into pure helpers
---

### Task LS-2

- **ID** LS-2
- **Context Bundle** `ARCHITECTURE.md`, `src/core.rs`, `src/state.rs`, `src/notify.rs`
- **DoD** Separate resume eligibility and outcome-to-status/notification decisions into pure helpers; add tests for state transitions and stop conditions (failed, max_iterations, complete with verifier auto-run) and resume eligibility across status and pid combinations.
- **Checklist**
  * Tests cover resume decisions for stale, stopped, failed, and dead-pid running sessions.
  * Tests cover status updates and notification selection for complete, failed, and max_iterations outcomes.
- **Dependencies** LS-1
- [x] LS-2 Isolate session lifecycle decisions and add transition tests
---

### Task VER-1

- **ID** VER-1
- **Context Bundle** `ARCHITECTURE.md`, `config/default.yaml`, `src/config.rs`, `src/cli.rs`
- **DoD** Isolate verifier command parsing and coverage percent extraction into pure helpers; add unit tests for empty or whitespace commands and for coverage parsing priority and fallback rules; keep coverage_min and coverage_warn behavior unchanged.
- **Checklist**
  * Tests cover "coverage results" priority and fallback percent extraction.
  * Tests cover command parsing errors for empty or whitespace-only commands.
- **Dependencies** None
- [ ] VER-1 Extract verifier parsing helpers and add coverage tests
---

### Task VER-2

- **ID** VER-2
- **Context Bundle** `ARCHITECTURE.md`, `config/default.yaml`, `src/config.rs`
- **DoD** Add pure decision tests for review gate and check gate evaluation, including rating scaling, issue count detection, and pending vs failed outcomes; keep poll loop semantics unchanged while improving retry decision coverage.
- **Checklist**
  * Tests cover rating parsing for fractions, percentages, and 0-10 scales.
  * Tests cover check rollup states for pending, failed, and passed decisions.
- **Dependencies** VER-1
- [ ] VER-2 Add review gate and check gate decision tests
---

### Task PRD-1

- **ID** PRD-1
- **Context Bundle** `src/prd.rs`, `PRD.template.md`, `PROCESS.md`
- **DoD** Refactor PRD sanitization and validation to separate content transformation from file IO; add tests for open questions removal, stray unchecked cleanup, context bundle normalization, and CRLF handling; ensure serialized output remains consistent.
- **Checklist**
  * Tests cover open questions removal and stray unchecked cleanup in sanitized output.
  * Tests cover context bundle path handling for absolute and relative entries.
- **Dependencies** None
- [ ] PRD-1 Isolate PRD content sanitization and add parsing tests
---

## Success Criteria

- New unit tests cover loop session decisions, verifier outcomes, and PRD parsing/serialization edge cases.
- `cargo test --workspace` passes and coverage remains at or above the configured coverage_min.
- CLI behavior, verifier pipeline flow, and PRD validation outcomes remain unchanged for end users.

---

## Sources

- https://doc.rust-lang.org/book/
- https://doc.rust-lang.org/rust-by-example/
- https://doc.rust-lang.org/std/
- https://docs.rs/assert_cmd/
- https://docs.rs/predicates/
- https://docs.rs/tempfile/
- https://docs.rs/proptest/
