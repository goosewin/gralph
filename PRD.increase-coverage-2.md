# Project Requirements Document (Template)

## Overview

Gralph is a Rust CLI for autonomous AI coding loops. This branch refactors a large `src/main.rs` into more modular entrypoint wiring while preserving CLI behavior, and lifts test coverage from 66% to at least 70% for the Rust codebase.

## Problem Statement

- The current `src/main.rs` is too large and mixes CLI parsing, dependency wiring, and command dispatch, making maintenance harder.
- Coverage dropped to 66% during the refactor, below the 70% recovery target and below existing quality expectations noted in repo docs.

## Solution

Extract entrypoint wiring from `src/main.rs` into existing library modules, keep the CLI interface stable, and add focused tests to raise tarpaulin coverage to >=70% using the configured coverage command.

---

## Functional Requirements

### FR-1: Modular Entry Point

The CLI entrypoint remains stable, with `src/main.rs` reduced to a thin wrapper that delegates to library helpers for parsing and command dispatch.

### FR-2: Coverage Recovery

Add tests around refactored entrypoint logic and related helpers so that workspace coverage meets or exceeds 70%.

---

## Non-Functional Requirements

### NFR-1: Performance

- Preserve current CLI startup behavior and response times; no new blocking work added to the entrypoint path.

### NFR-2: Reliability

- All changes must pass `cargo test --workspace` and the tarpaulin coverage command defined in `config/default.yaml`, with coverage >= 70%.
- Maintain doc and module map accuracy per existing process guardrails.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task MR-1

- **ID** MR-1
- **Context Bundle** `src/main.rs`, `src/lib.rs`, `src/cli.rs`, `ARCHITECTURE.md`
- **DoD** `src/main.rs` is a thin wrapper calling library entrypoint helpers, command dispatch and dependency wiring live in library modules, and `ARCHITECTURE.md` reflects the updated entrypoint flow.
- **Checklist**
  * Keep CLI behavior stable with existing `src/cli.rs` parsing tests.
  * Update the module map in `ARCHITECTURE.md` to match the new entrypoint wiring.
- **Dependencies** None
- [x] MR-1 Extract main entrypoint wiring into library modules
### Task DOC-1

- **ID** DOC-1
- **Context Bundle** `CHANGELOG.md`, `PROCESS.md`, `ARCHITECTURE.md`
- **DoD** Shared docs record the refactor and coverage recovery without violating ASCII and process requirements.
- **Checklist**
  * Add a changelog entry tagged with this task ID.
  * Ensure documentation remains ASCII-only and consistent with module layout.
- **Dependencies** MR-1
- [ ] DOC-1 Update shared docs for refactor and coverage recovery
### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/lib.rs`, `src/cli.rs`, `src/config.rs`, `config/default.yaml`
- **DoD** Targeted tests are added to lift tarpaulin coverage to >= 70% using the configured coverage command.
- **Checklist**
  * Add or extend unit tests in `src/lib.rs` or `src/cli.rs` for refactored entrypoint helpers.
  * Verify coverage with `cargo tarpaulin --workspace --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`.
- **Dependencies** MR-1
- [ ] COV-1 Add tests to raise coverage to 70 percent
---

## Success Criteria

- `src/main.rs` delegates to library entrypoint helpers and remains minimal.
- Workspace tests pass with `cargo test --workspace`.
- Tarpaulin coverage reaches or exceeds 70% using the configured coverage command.
- `ARCHITECTURE.md` and `CHANGELOG.md` reflect the refactor and coverage recovery.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
