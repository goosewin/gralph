# Project Requirements Document

## Overview

Improve auto worktree initialization for gralph PRD runs so it is target-directory aware, preserves subdirectory context, and provides clear UX for CLI users while remaining backend-agnostic. The focus is on the `gralph start` and `gralph run-loop` paths, with behavior aligned to existing defaults and the Worktree Protocol.

## Problem Statement

- Auto worktree creation currently resolves the git repo from the process working directory, which can skip or target the wrong repo when `gralph start <dir>` is run from elsewhere.
- When the target directory is a subdirectory, auto worktree creation resets the run directory to the worktree root, which can break PRD/task file discovery and config resolution.
- UX guidance around auto worktree skips (non-git, no commits, dirty state) and Graphite stacking expectations is minimal, making it harder to use `gt` reliably in nested worktree flows.
- There is no dedicated test coverage for auto worktree edge cases, risking regressions against the >= 90% coverage requirement.

## Solution

Refactor auto worktree initialization to be target-directory aware, preserve subdirectory paths in the created worktree, and keep existing flags and defaults intact. Add focused tests around git edge cases and subdirectory behavior, and update docs to clarify workflow expectations, including Graphite CLI stacking in worktrees.

---

## Functional Requirements

### FR-1: Target-Dir Auto Worktree

Auto worktree creation must resolve the git repo using the provided run directory, preserve the relative subdirectory path inside the new worktree, and continue to honor `defaults.auto_worktree` and `--no-worktree`.

### FR-2: Worktree UX and Compatibility

Auto worktree behavior must remain compatible with manual `gralph worktree create/finish` flow and with Graphite CLI stacking inside worktrees. Skip reasons must be explicit and actionable, including how to disable auto worktrees.

---

## Non-Functional Requirements

### NFR-1: Performance

Auto worktree initialization should only add a small fixed set of git commands (rev-parse, status, worktree add) and must not introduce any backend-specific overhead.

### NFR-2: Reliability

Auto worktree changes must preserve the dirty-repo guardrail, keep test coverage at or above 90%, and behave consistently across `start`, `run-loop`, and `resume` entrypoints.

---

## Implementation Tasks

### Task AW-1

- **ID** AW-1
- **Context Bundle** `src/main.rs`, `src/cli.rs`, `config/default.yaml`
- **DoD** Auto worktree resolution uses the target run directory, preserves subdirectory paths in the created worktree, and keeps existing disable flags and defaults unchanged.
- **Checklist**
  * `maybe_create_auto_worktree` resolves the repo root using the target directory and reports skip reasons that reference the target directory.
  * Subdirectory targets map to the same relative path inside the new worktree before the loop starts.
  * Branch naming continues to use `auto_worktree_branch_name` and `ensure_unique_worktree_branch` without collisions.
- **Dependencies** None
- [x] AW-1 Fix auto worktree target-dir and subdir mapping
### Task AW-2

- **ID** AW-2
- **Context Bundle** `src/main.rs`, `src/core.rs`, `Cargo.toml`
- **DoD** Tests cover auto worktree edge cases (non-git, no commits, dirty repo, subdirectory mapping, branch collision) and keep coverage >= 90%.
- **Checklist**
  * Add temp git repo tests that exercise the auto worktree path decisions and skip behaviors.
  * Validate that the resolved run directory is the worktree subdirectory when starting from a subdir.
  * `cargo test --workspace` passes and coverage remains >= 90%.
- **Dependencies** AW-1
- [x] AW-2 Add regression tests for auto worktree behavior
### Task AW-3

- **ID** AW-3
- **Context Bundle** `README.md`, `PROCESS.md`, `CHANGELOG.md`, `DECISIONS.md`
- **DoD** Documentation clearly explains auto worktree behavior, skip reasons, and Graphite stacking expectations, with changes logged in the changelog.
- **Checklist**
  * README and PROCESS describe auto worktree creation, subdirectory behavior, and how to disable via `--no-worktree` or `defaults.auto_worktree`.
  * Guidance includes Graphite CLI usage for stacking inside worktrees.
  * CHANGELOG entry added for the task ID; DECISIONS updated if behavior changes require a new decision record.
- **Dependencies** AW-1
- [ ] AW-3 Document auto worktree UX and Graphite stacking guidance
---

## Success Criteria

- Running `gralph start <repo>` from outside the repo creates the worktree in the correct repo and runs in the expected directory.
- Running `gralph start <repo/subdir>` continues the loop inside the matching subdirectory of the new worktree.
- Auto worktree tests cover skip conditions and collision handling, with coverage >= 90%.
- README and PROCESS describe auto worktree behavior and Graphite stacking expectations.

---

## Sources

- https://graphite.com/docs/command-reference
