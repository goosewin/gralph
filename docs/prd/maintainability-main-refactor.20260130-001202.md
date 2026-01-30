# Project Requirements Document (Template)

## Overview

This PRD defines a refactor to break down `src/main.rs` into small, testable modules while preserving gralph CLI behavior. It targets maintainers who need clearer module ownership, thinner wiring, and easier unit and integration testing. The repository is a Rust CLI using Cargo.

## Goals

- Reduce `src/main.rs` to thin wiring (parse args, build deps, call `run`, map result to exit code).
- Move real logic into library modules (`src/lib.rs` plus `src/app.rs` or equivalent).
- Add dependency seams to enable unit tests without heavy side effects.
- Add a small set of high-value CLI integration tests.
- Improve maintainability with minimal churn and clear rationale.

## Non-goals

- Chasing coverage with low-value tests (logging-only lines, clap boilerplate).
- Large behavior changes or product feature work.

## Problem Statement

`src/main.rs` mixes CLI wiring with application logic, IO, process control, and tests. This makes changes risky and testing costly.

Current `src/main.rs` responsibilities (rough LOC, based on file contents):
- CLI entry, dispatch, error types, intro/version/update output: ~120 LOC.
- Loop/session commands (start/run-loop/resume/stop/status/logs) plus state updates and notifier wiring: ~500 LOC.
- Worktree and git helpers (manual create/finish, auto worktree, git status helpers): ~400 LOC.
- PRD create/check/init and context file scaffolding: ~350 LOC.
- Config get/set/list plus YAML editing helpers: ~160 LOC.
- Logging/tail/table utilities and process helpers: ~120 LOC.
- Unit tests embedded in `src/main.rs`: ~1200+ LOC.

Side effects currently embedded in `src/main.rs` include filesystem access, environment reads/writes, process spawning, git commands, HTTP notifications, time and sleep calls.

## Solution

Introduce a library entrypoint and move CLI command logic into focused modules. Add a minimal dependency injection layer for IO, process, time, and network so tests can run without side effects.

Target module structure (example):
- `src/app.rs`: `run(...)` entrypoint, `Deps`, shared command dispatch.
- `src/app/commands/*.rs`: command handlers by domain (loop, worktree, prd, config, server, update).
- `src/infra/*.rs`: minimal adapters for FS, process, clock, network (optional grouping).

Minimal new types/functions:
- `pub async fn run(args: Args, deps: Deps) -> anyhow::Result<ExitCode>` or equivalent.
- `struct Deps` with small, focused traits or function pointers (fs, process, clock, network).
- Exit code mapping in one place (`run` or `main`).

Migration plan (commit-sized steps):
- Add `run` entrypoint and thin `main` wiring.
- Extract command handlers into library modules by domain.
- Introduce `Deps` seams and update command modules to use them.
- Move unit tests out of `src/main.rs` and add CLI integration tests.
- Update `ARCHITECTURE.md` and `CHANGELOG.md`.

---

## Functional Requirements

### FR-1: Thin main wiring

`src/main.rs` only parses CLI args, constructs real deps, calls `run`, and maps errors to an exit code.

### FR-2: Library entrypoint

A `run` entrypoint exists in the library (`src/lib.rs` exposes it) and is the single entry for CLI execution.

### FR-3: Dependency seams

Side-effectful operations (fs, process exec, network, clock/timeouts, optional logging) are behind minimal abstractions so unit tests can inject fakes.

### FR-4: CLI integration tests

Add integration tests using `assert_cmd` that cover:
- `--help` and invalid args exit codes.
- One happy path command (using temp dir/env overrides).
- One failure path (for example, missing PRD file or invalid PRD).

---

## Non-Functional Requirements

### NFR-1: Maintainability

Module boundaries are explicit and `src/main.rs` is reduced to under 300 LOC unless strongly justified.

### NFR-2: Reliability and behavior parity

No behavior regressions in CLI commands. Existing tests pass.

### NFR-3: Quality gates

Commands and tests align with CI expectations and coverage remains at or above 90 percent.

---

## Implementation Tasks

### Task MD-1

- **ID** MD-1
- **Context Bundle** `src/main.rs`, `src/lib.rs`, `src/cli.rs`, `src/core.rs`
- **DoD** A library `run` entrypoint exists and `src/main.rs` only parses args, builds deps, calls `run`, and maps the exit code.
- **Checklist**
  * `run` lives outside `src/main.rs` and is exported by `src/lib.rs`.
  * `src/main.rs` no longer contains command dispatch or loop orchestration.
  * Exit code mapping is centralized and tested.
- **Dependencies** None
- [x] MD-1 Add library run entrypoint and thin main wiring
### Task MD-2

- **ID** MD-2
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `src/state.rs`, `src/config.rs`
- **DoD** Loop and session command handlers are moved out of `src/main.rs` into a library module and use `Deps`.
- **Checklist**
  * Start/run-loop/resume/stop/status/logs behavior matches current output and errors.
  * Session state writes remain in `src/state.rs` and are invoked via the new module.
  * Main no longer owns these handlers.
- **Dependencies** MD-1
- [x] MD-2 Extract loop and session commands into app module
### Task MD-3

- **ID** MD-3
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `src/prd.rs`, `PRD.template.md`
- **DoD** PRD and init commands (create/check/init and related helpers) move to a library module.
- **Checklist**
  * PRD template resolution and context file handling preserve current behavior.
  * `gralph prd create` and `gralph prd check` outputs and errors are unchanged.
  * `gralph init` scaffolding behavior remains consistent with README/config defaults.
- **Dependencies** MD-1
- [ ] MD-3 Extract PRD and init command handlers
### Task MD-4

- **ID** MD-4
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `src/state.rs`, `src/config.rs`
- **DoD** Worktree and git helpers move to a dedicated module and are invoked via `Deps`.
- **Checklist**
  * Worktree create/finish behavior and error messages are preserved.
  * Auto worktree logic still respects clean repo checks and subdir mapping.
  * Main no longer calls git or worktree helpers directly.
- **Dependencies** MD-2
- [ ] MD-4 Move worktree and git helpers into app module
### Task MD-5

- **ID** MD-5
- **Context Bundle** `src/main.rs`, `src/notify.rs`, `src/config.rs`, `src/core.rs`
- **DoD** `Deps` provides minimal seams for fs, process, clock, and network; command modules use these seams to enable unit testing.
- **Checklist**
  * Unit tests can run without real network calls or process spawning.
  * Time-based behavior (sleep, timestamps) is injectable.
  * Dependencies stay small and avoid over-abstracting.
- **Dependencies** MD-1
- [ ] MD-5 Introduce dependency seams for side effects
### Task MD-6

- **ID** MD-6
- **Context Bundle** `Cargo.toml`, `src/cli.rs`, `src/prd.rs`
- **DoD** Add CLI integration tests using `assert_cmd` and wire dev dependencies.
- **Checklist**
  * Tests cover `--help`, invalid args, one happy path, and one failure path.
  * Tests use temp dirs and env overrides to avoid touching user state.
  * `Cargo.toml` dev-dependencies updated accordingly.
- **Dependencies** MD-1
- [ ] MD-6 Add CLI integration tests with assert_cmd
### Task MD-7

- **ID** MD-7
- **Context Bundle** `ARCHITECTURE.md`, `CHANGELOG.md`, `PROCESS.md`
- **DoD** Architecture and changelog reflect the new entrypoint flow and module map.
- **Checklist**
  * `ARCHITECTURE.md` describes `run` entrypoint and command module layout.
  * `CHANGELOG.md` includes a task-tagged entry with verification note.
  * Changes align with PROCESS guardrails.
- **Dependencies** MD-2, MD-3, MD-4, MD-5, MD-6
- [ ] MD-7 Update architecture docs and changelog
### Task MD-8

- **ID** MD-8
- **Context Bundle** `PROCESS.md`, `config/default.yaml`, `Cargo.toml`
- **DoD** Build, test, and coverage commands are run and results are recorded.
- **Checklist**
  * Run `cargo build --workspace`.
  * Run `cargo test --workspace`.
  * Run the coverage command from `config/default.yaml`.
- **Dependencies** MD-7
- [ ] MD-8 Verify build, tests, and coverage gates
---

## Success Criteria

- `src/main.rs` reduced to under 200 to 300 LOC unless strongly justified.
- `run(...)` entrypoint exists and is the single execution path for CLI.
- At least three CLI integration tests cover help, invalid args, a happy path, and a failure path.
- Architecture documentation reflects new module boundaries and entrypoint flow.
- Existing tests pass and coverage remains at or above 90 percent.
- PR description includes rationale for each structural change.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
