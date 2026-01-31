# Project Requirements Document (Template)

## Overview

Restore gralph test coverage from 63.83% to at least 70% after the tmux requirement and worktree enforcement changes. The primary users are maintainers running the Rust CLI and its test suite to keep quality gates stable.

## Problem Statement

- Coverage dropped below the previous 70% baseline after introducing tmux-required and worktree changes.
- Untested code paths in tmux/session handling and auto worktree logic increase regression risk.
- The current coverage gap undermines confidence in changes that gate loop execution behavior.

## Solution

Add targeted unit tests for tmux-required paths and auto worktree enforcement logic, then verify coverage with the existing tarpaulin command to restore the >=70% baseline.

---

## Functional Requirements

### FR-1: Tmux Requirement Coverage

Add tests that exercise tmux availability checks, session collision handling, and run-loop command assembly when tmux is required.

### FR-2: Auto Worktree Coverage

Add tests that exercise auto worktree skip reasons and path mapping behavior so the enforcement logic is fully covered.

---

## Non-Functional Requirements

### NFR-1: Performance

- New tests must be deterministic, local-only, and avoid long sleeps or network calls.

### NFR-2: Reliability

- Coverage must be >=70% using the current tarpaulin command in `config/default.yaml`, and `cargo test --workspace` must pass.

---

## Implementation Tasks

### Task COV-70-1

- **ID** COV-70-1
- **Context Bundle** `ARCHITECTURE.md`, `README.md`, `src/cli.rs`
- **DoD** Unit tests cover tmux availability and session collision branches, including success, non-zero exit, and not-found paths.
- **Checklist**
  * Stub tmux via PATH to simulate success and failure exit codes.
  * Cover collision handling in tmux session naming.
- **Dependencies** None
- [x] COV-70-1 Add tmux availability and session collision tests
### Task COV-70-2

- **ID** COV-70-2
- **Context Bundle** `ARCHITECTURE.md`, `src/cli.rs`
- **DoD** Tests validate run-loop command assembly for tmux_session None, empty, and non-empty branches, including argument inclusion rules.
- **Checklist**
  * Capture program and args via a ProcessRunner test double.
  * Verify `--tmux-session` is only added for non-empty values.
- **Dependencies** None
- [x] COV-70-2 Test run-loop command assembly for tmux paths
### Task COV-70-3

- **ID** COV-70-3
- **Context Bundle** `PROCESS.md`, `README.md`, `config/default.yaml`, `ARCHITECTURE.md`
- **DoD** Auto worktree tests cover skip reasons (not a repo, git missing, no commits, dirty repo) and subdir path mapping behavior.
- **Checklist**
  * Use temp git repos to validate clean/dirty and no-commit cases.
  * Assert `.worktree/<branch>` creation preserves subdir paths.
- **Dependencies** None
- [ ] COV-70-3 Add auto worktree skip and path mapping tests
### Task COV-70-4

- **ID** COV-70-4
- **Context Bundle** `CHANGELOG.md`, `PROCESS.md`, `config/default.yaml`
- **DoD** Tests and tarpaulin run cleanly with coverage >=70%, and the CHANGELOG verification line is updated.
- **Checklist**
  * Run `cargo test --workspace` and the configured tarpaulin command.
  * Record test and coverage results in `CHANGELOG.md`.
- **Dependencies** COV-70-1, COV-70-2, COV-70-3
- [ ] COV-70-4 Verify coverage and record results in CHANGELOG
---

## Success Criteria

- `cargo tarpaulin --workspace --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*` reports coverage >=70%.
- `cargo test --workspace` passes.
- Tmux and auto worktree branches are covered by new tests.
- `CHANGELOG.md` includes a verification line with the updated test and coverage results.

---

## Sources

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
