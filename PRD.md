# Project Requirements Document (Template)

## Overview

Gralph is a CLI loop runner for PRD-driven AI work. This PRD focuses on reducing UX friction and making defaults safer while preserving the power-user workflow (stateless iterations, context files, auto worktrees, verifier pipeline, status server, and notifications). Intended users are current power users plus newcomers who need clearer run modes, better observability, and safer verification defaults.

## Problem Statement

- The CLI is power-user first; defaults and docs imply stability while behaviors are more implicit and advanced.
- "tmux optional" messaging is unclear; background runs hide stdout and stderr, and log locations are not surfaced up front.
- Verifier defaults in `config/default.yaml` are Rust-specific and `verifier.auto_run` defaults to true, which is surprising in non-Rust repos.
- Auto-merge can proceed without explicit approvals (`verifier.review.require_approval: false`), which feels risky by default.
- Status output and the status server do not surface last error, last task ID, or the latest log line, making background runs opaque.
- Resume and reruns can drift when prompt template or backend overrides are not captured in session state.
- Update checks run on every loop start with no opt-out for offline or locked-down environments.
- Strict PRD validation is easy to trip without tools like dry-run or single-step execution.

## Solution

Provide safe defaults and clearer observability without removing power-user workflows. Add diagnostic and maintenance commands, expose raw backend output, add dry-run and single-step flows, make verifier defaults stack-aware with safe auto-merge gating, and introduce an update check opt-out plus clearer tmux and worktree messaging.

---

## Functional Requirements

### FR-1: Safer Verifier Defaults and Auto-Merge Gating

- Use stack detection in `src/prd.rs` to decide default verifier behavior.
- For Rust/Cargo projects, keep existing default commands from `config/default.yaml`.
- For non-Rust or unknown stacks, default `verifier.auto_run` to false and require explicit test and coverage commands to enable verifier.
- Auto-merge is opt-in: require approvals by default or introduce `verifier.auto_merge: false` and gate merge on explicit opt-in.

### FR-2: Observability and Raw Output Access

- Add `gralph logs <name> --raw` to display `.gralph/<session>.raw.log` when present.
- Add `gralph status --json` with machine-readable session data including `current_remaining`, `last_task_id`, `last_log_line`, `last_error`, `log_file`, `raw_log_file`, `is_alive`, and `status`.
- Add `gralph status --verbose` to surface log paths and last error line.
- `/status` and `/status/:name` in `src/server.rs` include the same fields.

### FR-3: Dry-Run and Single-Step Execution

- `gralph start --dry-run` prints the next unchecked task block and the resolved prompt template without running a backend or creating tmux sessions.
- `gralph step` runs exactly one iteration using the same config and prompt rendering as the loop and does not auto-run the verifier.

### FR-4: Doctor and Cleanup Utilities

- `gralph doctor` checks backend CLIs, `gh` installation and auth, git clean state for the target directory, config readability, and state store access; it outputs actionable remediation with a non-zero exit on failure.
- `gralph cleanup` marks stale sessions by default and supports `--remove` and `--purge` for destructive cleanup, with a clear summary of affected sessions.

### FR-5: Update Check Controls and Clear Run Messaging

- Add `defaults.check_updates` and `GRALPH_NO_UPDATE_CHECK=1` to skip update checks on loop start.
- When auto-worktree is skipped, print a hint for `--no-worktree` and any configured worktree mode option.
- Update `--no-tmux` help and start output to clearly state where logs are written and how to tail them.

---

## Non-Functional Requirements

### NFR-1: Performance

- `gralph doctor`, `gralph status`, and `gralph cleanup` should complete quickly on local checks and avoid long network waits.

### NFR-2: Reliability and Safety

- No destructive cleanup happens by default; state file deletion requires an explicit flag.
- Raw logs remain preserved and discoverable when backends emit structured output.

### NFR-3: Compatibility

- Existing CLI flags and defaults remain valid for Rust projects; new behaviors are opt-in or stack-aware.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task UX-1

- **ID** UX-1
- **Context Bundle** `src/cli.rs`, `src/backend/mod.rs`, `src/config.rs`, `src/state.rs`, `README.md`
- **DoD** `gralph doctor` runs local checks for backend CLIs, `gh` install and auth, git clean state, config readability, and state store access, prints actionable output, and exits non-zero on required failures.
- **Checklist**
  * CLI help lists `doctor` and its options.
  * Output includes per-check status and a summary exit code.
  * README documents doctor usage and common failure hints.
- **Dependencies** None
- [x] UX-1 Add doctor command and checks
---

### Task UX-2

- **ID** UX-2
- **Context Bundle** `src/cli.rs`, `src/state.rs`, `PROCESS.md`, `README.md`
- **DoD** `gralph cleanup` marks stale sessions by default using `StateStore::cleanup_stale`, supports `--remove` and `--purge`, and reports the list or count of affected sessions.
- **Checklist**
  * Default mode marks stale sessions without deleting state.
  * `--remove` deletes only stale sessions; `--purge` requires explicit opt-in.
  * README and PROCESS describe cleanup behavior.
- **Dependencies** None
- [x] UX-2 Add cleanup command for stale sessions
---

### Task UX-3

- **ID** UX-3
- **Context Bundle** `src/core.rs`, `src/cli.rs`, `src/state.rs`, `README.md`
- **DoD** Session state records the `.raw.log` path derived from `.gralph/<session>.log`, and `gralph logs --raw` prints that file or a clear error when missing.
- **Checklist**
  * Raw log path uses the same suffix logic as `raw_log_path` in `src/core.rs`.
  * `logs --raw` handles both present and missing files.
  * README documents raw log access and file location.
- **Dependencies** None
- [x] UX-3 Expose raw log access and paths
---

### Task UX-4

- **ID** UX-4
- **Context Bundle** `src/server.rs`, `src/state.rs`, `src/core.rs`, `src/prd.rs`, `src/cli.rs`
- **DoD** `gralph status --json` and server `/status` include `last_task_id`, `last_log_line`, `last_error`, and `raw_log_file`, and `gralph status --verbose` surfaces log paths and the last error line.
- **Checklist**
  * JSON output uses stable keys and includes required fields when available.
  * Last error is derived from log entries such as "Error:" and "Iteration failed:" in `src/core.rs`.
  * Task ID uses existing PRD parsing logic in `src/prd.rs` or equivalent.
- **Dependencies** UX-3
- [x] UX-4 Add status JSON and error context
---

### Task UX-5

- **ID** UX-5
- **Context Bundle** `src/cli.rs`, `src/core.rs`, `src/prd.rs`, `README.md`
- **DoD** `gralph start --dry-run` prints the next task block and resolved prompt template without running a backend, and `gralph step` runs exactly one iteration without auto-running the verifier.
- **Checklist**
  * Dry-run respects prompt template resolution order in `src/core.rs`.
  * Step uses the same prompt rendering and strict PRD behavior as the loop.
  * README documents dry-run and step usage.
- **Dependencies** None
- [x] UX-5 Add dry-run start and step execution
---

### Task UX-6

- **ID** UX-6
- **Context Bundle** `config/default.yaml`, `src/config.rs`, `src/prd.rs`, `ARCHITECTURE.md`, `README.md`
- **DoD** Verifier defaults are stack-aware: Rust/Cargo retains current defaults, while non-Rust or unknown stacks default `verifier.auto_run` to false and require explicit commands.
- **Checklist**
  * Stack detection drives verifier defaults.
  * Rust defaults remain unchanged from `config/default.yaml`.
  * README and ARCHITECTURE describe stack-aware behavior.
- **Dependencies** None
- [x] UX-6 Make verifier defaults stack-aware
---

### Task UX-7

- **ID** UX-7
- **Context Bundle** `config/default.yaml`, `PROCESS.md`, `ARCHITECTURE.md`, `README.md`
- **DoD** Auto-merge is opt-in by default via `verifier.review.require_approval: true` or a new `verifier.auto_merge: false`, with documentation aligned to the new safe default.
- **Checklist**
  * Default config prevents merge without explicit approval or opt-in.
  * PROCESS and ARCHITECTURE reflect the new merge gate.
  * README documents how to enable auto-merge.
- **Dependencies** None
- [ ] UX-7 Enforce safe auto-merge defaults
---

### Task UX-8

- **ID** UX-8
- **Context Bundle** `config/default.yaml`, `src/config.rs`, `src/cli.rs`, `README.md`, `PROCESS.md`
- **DoD** Update checks can be disabled via `defaults.check_updates` or `GRALPH_NO_UPDATE_CHECK`, and start output includes clear tmux and worktree skip hints with log locations.
- **Checklist**
  * Update check is bypassed when config or env disables it.
  * Start output mentions log file path and how to tail logs in background runs.
  * README and PROCESS document new flags and hints.
- **Dependencies** None
- [ ] UX-8 Add update opt-out and clearer messaging
---

## Success Criteria

- Non-Rust repos do not auto-run the verifier unless explicitly configured.
- `gralph status --json` includes `last_task_id`, `last_log_line`, `last_error`, and raw log paths for active sessions.
- `gralph logs --raw` provides access to raw backend output when available.
- `gralph doctor` exits non-zero when required tools or auth are missing and provides actionable hints.
- Update checks can be disabled via config or env and are skipped when disabled.

---

## Sources

- None

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
