# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`gralph-rs/src/main.rs` is the Rust CLI entrypoint. It wires clap subcommands to Rust modules, manages session state updates, and coordinates backend execution.
`gralph-rs/src/lib.rs` holds shared types and helpers used by the Rust modules.
`gralph-rs/src/cli.rs` defines the clap command tree and options; `gralph-rs/build.rs` generates bash/zsh completions during build.

`gralph-rs/src/core.rs` owns the execution loop for iteration execution, task counting, completion checks, and loop orchestration.
`gralph-rs/src/state.rs` manages persistent session state with file locking and atomic writes.
`gralph-rs/src/server.rs` implements the HTTP status server, CORS handling, and bearer auth.
`gralph-rs/src/config.rs` loads default/global/project YAML config with env overrides.
`gralph-rs/src/prd.rs` provides PRD validation, sanitization, and stack detection utilities.

`gralph-rs/src/backend` defines the backend trait and CLI-backed implementations (`backend/mod.rs` plus `backend/claude.rs`, `backend/opencode.rs`, `backend/gemini.rs`, `backend/codex.rs`).
`gralph-rs/src/notify.rs` formats and sends webhook notifications via reqwest.

## Runtime Flow

The CLI initializes configuration and starts the core loop. The loop
prepares logging, counts remaining tasks, and iterates until completion
or max iterations. Each iteration builds the prompt, invokes the backend,
parses the result, checks for completion promises, and updates optional
state callbacks with remaining task counts. On completion or failure, the
loop records duration, writes final status to logs, and optionally sends
notifications.

## Storage

Session state is stored in `~/.config/gralph/state.json` with a lock file
at `~/.config/gralph/state.lock` (or a lock dir fallback). Loop logs are
written to `.gralph/<session>.log` inside the target project directory.
