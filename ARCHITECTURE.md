# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/Gralph/Program.cs` is the CLI entrypoint. It parses commands, loads configuration, resolves backends, manages sessions, and spawns background workers for loops.

`src/Gralph/Core` implements the execution loop. It renders prompts, runs backend iterations, checks completion markers, and reports loop progress back to state updates.

`src/Gralph/State` provides persistent session state with JSON storage, file locking, CRUD operations, and stale session cleanup.

`src/Gralph/Server` implements the HTTP status API for session status and stop commands with bearer auth.

`src/Gralph/Config` handles configuration loading and overrides. It merges default, global, and project YAML and resolves environment overrides.

`src/Gralph/Prd` provides PRD task block parsing, validation, sanitization, and stack detection.

`src/Gralph/Backends` defines the backend abstraction layer and implementations for invoking model CLIs and parsing their output streams.

`src/Gralph/Notify` formats and sends webhook notifications for completion and failure events.

## Runtime Flow

The CLI initializes configuration and state, then either spawns a background
worker process (default) or runs in the foreground with `--no-tmux`. The loop
prepares logging, counts remaining tasks, and iterates until completion or
max iterations. Each iteration builds the prompt, invokes the backend CLI,
parses the result, checks for completion promises, and updates session state.
On completion or failure, the loop records duration, updates status, and
optionally sends notifications.

## Storage

Session state is stored in `~/.config/gralph/state.json` with a lock file
at `~/.config/gralph/state.lock` (or a lock dir fallback). Loop logs are
written to `.gralph/<session>.log` inside the target project directory.

## Build and Release

Native AOT builds are published with the scripts in `scripts/` or via
`dotnet publish` directly:

- `scripts/publish-aot.sh` builds single-file, self-contained executables into
  `dist/aot/<rid>` for supported RIDs.
- `scripts/verify-aot.sh` builds a target RID and prints startup timing and
  binary size metrics for documentation.
