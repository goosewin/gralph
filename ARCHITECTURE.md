# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/main.rs` is the thin CLI entrypoint. It delegates to `cli_entrypoint` in `src/lib.rs` and returns its `ExitCode`.
`src/lib.rs` exposes `run`, `Deps`, and the `cli_entrypoint` helper that parses args, builds real deps, runs the app, and maps results to exit codes.
`src/app.rs` owns the `run` entrypoint, dependency seams, and command dispatch.
`src/app/loop_session.rs` implements start/run-loop/stop/status/logs/resume handlers with `Deps`.
`src/app/prd_init.rs` implements `gralph prd` and `gralph init` plus PRD/template helpers.
`src/app/worktree.rs` implements worktree commands and auto-worktree flow.
`src/cli.rs` defines the clap command tree and options; `build.rs` generates bash/zsh completions during build.

`src/core.rs` owns the execution loop for iteration execution, task counting, completion checks, and loop orchestration.
`src/state.rs` manages persistent session state with file locking and atomic writes.
`src/server.rs` implements the HTTP status server, CORS handling, and bearer auth.
`src/config.rs` loads default/global/project YAML config with env overrides.
`src/prd.rs` provides PRD validation, sanitization, and stack detection utilities.
`src/task.rs` centralizes task block parsing helpers shared by core and PRD validation.
`src/verifier.rs` implements the verifier pipeline helpers for tests, coverage, static checks, PR creation, and review gating.
`src/update.rs` handles release update checks and installs.
`src/version.rs` defines the CLI version constants.

`src/backend` defines the backend trait and CLI-backed implementations (`backend/mod.rs` plus `backend/claude.rs`, `backend/opencode.rs`, `backend/gemini.rs`, `backend/codex.rs`).
`src/notify.rs` formats and sends webhook notifications via reqwest.

## Runtime Flow

`src/main.rs` calls `cli_entrypoint` in `src/lib.rs`, which parses CLI
arguments, builds real dependencies, and calls `app::run`. The `run`
entrypoint dispatches to command handlers. The
start/run-loop paths optionally create a worktree, load configuration,
validate PRDs (when strict), and invoke `core::run_loop_with_clock`.
Each iteration builds the prompt, invokes the backend, parses the result,
checks for completion promises, and updates state callbacks with remaining
task counts. On completion or failure, the loop records duration, writes
final status to logs, and optionally sends notifications. When completion
succeeds and `verifier.auto_run` is true, the verifier pipeline runs in the
active worktree to execute tests, coverage, and static checks, open a PR
via `gh`, wait for the configured review gate, and merge after approvals.

## Storage

Session state is stored in `~/.config/gralph/state.json` with a lock file
at `~/.config/gralph/state.lock` (or a lock dir fallback). Loop logs are
written to `.gralph/<session>.log` inside the target project directory.

## Quality Gates

CI workflows live in `.github/workflows/` (notably `ci.yml`). Tests and
coverage checks are required before merge; coverage must remain at or
above 90%. CI runs `cargo test --workspace` and
`cargo tarpaulin --workspace --fail-under 60 --exclude-files src/main.rs
src/core.rs src/notify.rs src/server.rs src/backend/*`. Release and smoke
workflows assume CI is green. The verifier mirrors these gates, adds static
checks, and enforces the review gate before merge. The verifier can also emit
a non-blocking warning when coverage falls below the soft target configured in
`verifier.coverage_warn`.
