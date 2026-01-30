# Project Requirements Document (Template)

## Overview

Refactor the Rust CLI entrypoint to be more modular by moving entrypoint wiring out of `src/main.rs` while preserving existing clap command behavior in `src/cli.rs`. Raise code coverage from the current ~65% baseline to at least 70% by adding targeted tests in covered modules. Intended users are gralph CLI maintainers and contributors.

## Problem Statement

- The entrypoint wiring is still concentrated in `src/main.rs`, making it harder to test and evolve even though the architecture calls for a thin entrypoint.
- Coverage has regressed (CHANGELOG reports 65.49%), and the risk register flags coverage regression as a high-impact risk.
- This branch needs an incremental coverage recovery to 70% without changing CLI behavior or relaxing verifier settings.

## Solution

Introduce a library-level entrypoint helper in `src/lib.rs` that encapsulates CLI parsing and exit-code mapping, with `src/main.rs` delegating to it. Update architecture and changelog entries to document the refactor. Add unit tests around the new helper and related CLI parsing paths to lift coverage to at least 70% using the tarpaulin command configured in `config/default.yaml`.

---

## Functional Requirements

### FR-1: Modular CLI Entry

`src/main.rs` delegates to a new helper in `src/lib.rs` that performs CLI parsing and run/exit mapping, while preserving the clap command tree and flags defined in `src/cli.rs`.

### FR-2: Coverage Recovery

Add tests for the new entrypoint helper and relevant CLI paths so that `cargo tarpaulin --workspace --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*` reports at least 70% coverage.

---

## Non-Functional Requirements

### NFR-1: Performance

- Keep the startup path lightweight with no new IO beyond existing CLI parsing and dependency wiring.

### NFR-2: Reliability

- `cargo test --workspace` passes.
- Coverage is at least 70% with the configured tarpaulin command, and `verifier.coverage_min` in `config/default.yaml` is not lowered.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task MR-1

- **ID** MR-1
- **Context Bundle** `src/main.rs`, `src/lib.rs`, `src/cli.rs`
- **DoD** A new library entrypoint helper encapsulates CLI parse and exit mapping, and `src/main.rs` delegates to it without changing command behavior.
- **Checklist**
  * Entry point logic is relocated into `src/lib.rs` and `src/main.rs` remains a thin wrapper.
  * CLI parsing behavior matches existing tests and help text in `src/cli.rs`.
- **Dependencies** None
- [x] MR-1 Extract CLI entrypoint helper from main.rs
---

### Task MR-2

- **ID** MR-2
- **Context Bundle** `ARCHITECTURE.md`, `CHANGELOG.md`, `PROCESS.md`
- **DoD** Architecture module map and changelog document the entrypoint refactor with an MR-2 entry and a verification note in the required format.
- **Checklist**
  * `ARCHITECTURE.md` reflects the new entrypoint helper location and flow.
  * `CHANGELOG.md` includes an Unreleased MR-2 entry and verification line format.
- **Dependencies** MR-1
- [x] MR-2 Document entrypoint refactor and verification
---

### Task COV-1

- **ID** COV-1
- **Context Bundle** `src/lib.rs`, `src/cli.rs`, `config/default.yaml`
- **DoD** New tests cover the entrypoint helper and CLI paths, and tarpaulin coverage reaches at least 70%.
- **Checklist**
  * Added unit tests for entrypoint helper and CLI parsing paths.
  * `cargo test --workspace` passes.
  * `cargo tarpaulin --workspace --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*` reports >= 70.
- **Dependencies** MR-1
- [ ] COV-1 Add tests to raise coverage to 70
---

## Success Criteria

- `src/main.rs` is reduced to a thin wrapper that calls a library helper.
- CLI behavior and flags remain consistent with `src/cli.rs` tests.
- Coverage is at least 70% under the configured tarpaulin command and tests pass.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
