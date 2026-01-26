# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`bin/gralph` is the CLI entrypoint. It parses commands, loads configuration defaults, wires in the shared libraries, and dispatches to subcommands like start/stop/status/server while managing session setup and logging paths.

`src/Gralph` is the .NET 10 console application entrypoint that will replace the bash CLI, providing the same command surface with Native AOT publishing.

`src/Gralph/Core` implements the .NET execution loop. It renders prompts, runs backend iterations, checks completion markers, and reports loop progress back to state callbacks.

`lib/core.sh` owns the execution loop. It loads backend adapters, renders prompt templates, runs iterations, counts remaining tasks, checks completion conditions, and handles loop lifecycle including logs, duration, and notifications.

`lib/state.sh` provides persistent session state. It manages the state file, locking, and CRUD operations for sessions plus stale session cleanup so concurrent loops remain consistent.

`src/Gralph/State` mirrors state persistence in .NET, including JSON state storage, file locking, CRUD operations, and stale session cleanup.

`lib/server.sh` implements the lightweight status API. It exposes endpoints for session status and stop commands, handles auth, and supports running via netcat or socat.
`src/Gralph/Server` implements the .NET status API server for session status and stop commands with bearer auth.

`lib/config.sh` handles configuration loading and overrides. It merges default, global, and project YAML into a cache and exposes getters and setters used by the CLI and core loop.
`src/Gralph/Config` mirrors the configuration system in .NET, including YAML parsing, precedence merges, and environment overrides.

`lib/prd.sh` provides PRD task block parsing, validation, sanitization, and stack detection for bash flows.
`src/Gralph/Prd` mirrors PRD task block parsing and validation in .NET.

`src/Gralph/Backends` defines the backend abstraction layer and implementations for invoking model CLIs. It includes backend discovery, model metadata, and JSON stream parsing utilities.

`lib/notify.sh` formats and sends webhook notifications. It detects webhook targets, builds payloads for Slack/Discord/generic endpoints, and posts completion or failure events.
`src/Gralph/Notify` mirrors webhook notification handling in .NET, generating Slack/Discord/generic payloads and sending completion or failure events.

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

## Build and Release

Native AOT builds are published with the scripts in `scripts/`:

- `scripts/publish-aot.sh` builds single-file, self-contained executables into
  `dist/aot/<rid>` for macOS, Linux, and Windows RIDs.
- `scripts/verify-aot.sh` builds a target RID and prints startup timing and
  binary size metrics for documentation.
