# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/Gralph` is the .NET console application scaffold for the upcoming port. It will host the CLI entrypoint, command wiring, and runtime services.

`src/Gralph/State` manages session state persistence in JSON, including locking, CRUD operations, and stale session cleanup.

`src/Gralph/Configuration` loads YAML configuration, applies hierarchical merges, and resolves environment variable overrides for shared config access.

`src/Gralph/Backends` defines the backend abstraction (IBackend) plus adapter implementations and discovery/registry helpers for selecting AI backends.

`src/Gralph/Core` ports the core loop logic (task parsing, prompt rendering, completion detection, iteration execution, and log handling).

`src/Gralph/Prd` handles PRD task validation, generation prompts, stack detection, and sanitization of generated PRDs.

`src/Gralph/Commands/ServerCommandHandler.cs` hosts the HTTP status server using minimal APIs with auth, CORS, and session status/stop endpoints.

`bin/gralph` is the CLI entrypoint. It parses commands, loads configuration defaults, wires in the shared libraries, and dispatches to subcommands like start/stop/status/server while managing session setup and logging paths.

`lib/core.sh` owns the execution loop. It loads backend adapters, renders prompt templates, runs iterations, counts remaining tasks, checks completion conditions, and handles loop lifecycle including logs, duration, and notifications.

`lib/state.sh` provides persistent session state. It manages the state file, locking, and CRUD operations for sessions plus stale session cleanup so concurrent loops remain consistent.

`lib/server.sh` implements the lightweight status API. It exposes endpoints for session status and stop commands, handles auth, and supports running via netcat or socat.

`lib/config.sh` handles configuration loading and overrides. It merges default, global, and project YAML into a cache and exposes getters and setters used by the CLI and core loop.

`lib/notify.sh` formats and sends webhook notifications. It detects webhook targets, builds payloads for Slack/Discord/generic endpoints, and posts completion or failure events.
`src/Gralph/Notifications` provides the .NET webhook notification payloads, detection, and HttpClient delivery for Discord, Slack, and generic endpoints.

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
