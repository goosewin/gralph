# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/main.rs` is the Rust CLI entrypoint. It wires clap subcommands to Rust modules, manages session state updates, coordinates backend execution, and runs the verifier quality gates.
`src/lib.rs` holds shared types and helpers used by the Rust modules.
`src/cli.rs` defines the clap command tree and options; `build.rs` generates bash/zsh completions during build.

`src/core.rs` owns the execution loop for iteration execution, task counting, completion checks, and loop orchestration.
`src/state.rs` manages persistent session state with file locking and atomic writes.
`src/server.rs` implements the HTTP status server, CORS handling, and bearer auth.
`src/config.rs` loads default/global/project YAML config with env overrides.
`src/prd.rs` provides PRD validation, sanitization, and stack detection utilities.

`src/backend` defines the backend trait and CLI-backed implementations (`backend/mod.rs` plus `backend/claude.rs`, `backend/opencode.rs`, `backend/gemini.rs`, `backend/codex.rs`).
`src/notify.rs` formats and sends webhook notifications via reqwest.

## Runtime Flow

The CLI initializes configuration and starts the core loop. The loop
prepares logging, counts remaining tasks, and iterates until completion
or max iterations. Each iteration builds the prompt, invokes the backend,
parses the result, checks for completion promises, and updates optional
state callbacks with remaining task counts. On completion or failure, the
loop records duration, writes final status to logs, and optionally sends
notifications. When completion succeeds and `verifier.auto_run` is true,
the verifier pipeline runs in the active worktree to execute tests,
coverage, and static checks, open a PR via `gh`, wait for the configured
review gate, and merge after approvals.

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
checks, and enforces the review gate before merge.
