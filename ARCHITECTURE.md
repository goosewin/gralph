# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`bin/gralph` is the CLI entrypoint. It parses commands, loads configuration defaults, wires in the shared libraries, and dispatches to subcommands like start/stop/status/server while managing session setup and logging paths.

`lib/core.sh` owns the execution loop. It loads backend adapters, renders prompt templates, runs iterations, counts remaining tasks, checks completion conditions, and handles loop lifecycle including logs, duration, and notifications.

`lib/state.sh` provides persistent session state. It manages the state file, locking, and CRUD operations for sessions plus stale session cleanup so concurrent loops remain consistent.

`lib/server.sh` implements the lightweight status API. It exposes endpoints for session status and stop commands, handles auth, and supports running via netcat or socat.

`lib/config.sh` handles configuration loading and overrides. It merges default, global, and project YAML into a cache and exposes getters and setters used by the CLI and core loop.

`lib/notify.sh` formats and sends webhook notifications. It detects webhook targets, builds payloads for Slack/Discord/generic endpoints, and posts completion or failure events.

## Runtime Flow

Placeholder for the main loop flow, including major phases and handoffs.

## Storage

Placeholder for state, log, and configuration storage locations.
