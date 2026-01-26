# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`bin/gralph` is the CLI entrypoint. It parses commands, loads configuration defaults, wires in the shared libraries, and dispatches to subcommands like start/stop/status/server while managing session setup and logging paths.
`gralph-rs/src/main.rs` is the Rust CLI entrypoint. It wires clap subcommands to Rust modules, manages session state updates, and coordinates backend execution.
`gralph-rs/src/lib.rs` holds shared types and helpers used by the Rust modules.
`gralph-rs/src/cli.rs` defines the clap command tree and options; `gralph-rs/build.rs` generates bash/zsh completions during build.

`lib/core.sh` owns the execution loop. It loads backend adapters, renders prompt templates, runs iterations, counts remaining tasks, checks completion conditions, and handles loop lifecycle including logs, duration, and notifications.
`gralph-rs/src/core.rs` mirrors core loop logic in Rust for iteration execution, task counting, completion checks, and loop orchestration.

`lib/state.sh` provides persistent session state. It manages the state file, locking, and CRUD operations for sessions plus stale session cleanup so concurrent loops remain consistent.
`gralph-rs/src/state.rs` mirrors state management in Rust with file locking and atomic writes.

`lib/server.sh` implements the lightweight status API. It exposes endpoints for session status and stop commands, handles auth, and supports running via netcat or socat.
`gralph-rs/src/server.rs` mirrors the HTTP status server, CORS handling, and bearer auth in Rust.

`lib/config.sh` handles configuration loading and overrides. It merges default, global, and project YAML into a cache and exposes getters and setters used by the CLI and core loop.
`gralph-rs/src/config.rs` mirrors configuration loading in Rust using serde_yaml for the ported CLI.

`lib/prd.sh` implements PRD validation, sanitization, and stack detection utilities for task blocks.
`gralph-rs/src/prd.rs` mirrors PRD validation, sanitization, and stack detection behavior in Rust.

`lib/backends/*.sh` provides backend adapters that invoke external CLIs.
`gralph-rs/src/backend` defines the backend trait and CLI-backed implementations for the Rust port (`backend/mod.rs` plus `backend/claude.rs`, `backend/opencode.rs`, `backend/gemini.rs`, `backend/codex.rs`).

`lib/notify.sh` formats and sends webhook notifications. It detects webhook targets, builds payloads for Slack/Discord/generic endpoints, and posts completion or failure events.
`gralph-rs/src/notify.rs` mirrors webhook detection, payload formatting, and delivery in Rust via reqwest.

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
