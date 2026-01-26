# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/Gralph/Program.cs` wires the CLI entrypoint, help text, and command routing with System.CommandLine.

`src/Gralph/Commands` contains handlers for start/stop/status/logs/resume/prd/worktree/config/server/backends.

`src/Gralph/Core` runs the loop, prompt rendering, completion detection, and log handling.

`src/Gralph/Backends` defines the backend abstraction plus adapters (Claude, OpenCode, Gemini, Codex).

`src/Gralph/Configuration` loads YAML config, applies overrides, and exposes config access.

`src/Gralph/State` stores session state in JSON with locking and stale-session cleanup.

`src/Gralph/Prd` validates PRD task blocks, generates PRDs, and performs stack detection.

`src/Gralph/Notifications` formats and delivers webhook notifications for Discord, Slack, and generic endpoints.

`config/default.yaml` ships the default configuration values.

`tests/Gralph.Tests` contains xUnit tests covering state, config, and PRD validation logic.

## Runtime Flow

The CLI loads configuration, validates inputs, and resolves the backend adapter. By default, `gralph start`
launches a background child process and records its PID in the state file; `--no-tmux` runs the loop in the
foreground for debugging. The core loop counts remaining tasks, renders the prompt template, invokes the
backend, parses the response, checks completion promises, updates session state, and writes log output. On
completion or failure, the loop updates session status and optionally sends webhook notifications.

## Storage

Session state is stored in `~/.config/gralph/state.json` with a lock at `~/.config/gralph/state.lock`
(or a lock dir fallback). Loop logs are written to `.gralph/<session>.log` inside the target project
directory. Configuration is loaded from `config/default.yaml`, `~/.config/gralph/config.yaml`, and an
optional project `.gralph.yaml`.
