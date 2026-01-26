# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`main.go` initializes the CLI and wires the Cobra command tree in `cmd`.

`cmd` owns the CLI surface, parses flags, and dispatches subcommands like start/stop/status/logs/resume/prd/worktree/config/server/backends.

`internal/core` owns the execution loop, prompt rendering, task selection, completion checks, log handling, and iteration pacing.

`internal/prd` provides PRD parsing, validation, sanitization, and stack detection utilities.

`internal/backend` defines the backend interface and registry. Implementations under `internal/backend/<name>` wrap AI CLI backends for iteration runs and response parsing.

`internal/state` persists session state with locking, atomic writes, and stale-session cleanup so concurrent loops remain consistent.

`internal/server` exposes the status API with REST endpoints, bearer auth, and CORS handling.

`internal/config` loads and merges default, global, and project YAML config using Viper with GRALPH_ environment overrides.

`internal/notify` formats and sends webhook notifications to Discord, Slack, or generic JSON endpoints.

## Go Package Structure

The Go port organizes functionality into internal packages that mirror the bash
modules. The command surface lives in `cmd` and binds flags to the core
packages listed below.

- `cmd` Cobra command tree and CLI wiring.
- `internal/core` loop engine, prompt rendering, task selection.
- `internal/prd` PRD parsing, validation, sanitization helpers.
- `internal/backend` backend interfaces and CLI adapters.
- `internal/state` session state storage and locking.
- `internal/config` configuration loading and overrides.
- `internal/server` HTTP status API.
- `internal/notify` webhook notifications.

## Module Diagram

Package dependencies flow from `cmd` into service packages, with the core loop
depending on config, state, PRD helpers, and backends.

```
cmd
 |-- internal/core
 |     |-- internal/config
 |     |-- internal/state
 |     |-- internal/prd
 |     `-- internal/backend
 |-- internal/server
 |     `-- internal/state
 `-- internal/notify
```

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
written to `.gralph/<session>.log` and `.gralph/<session>.raw.log` inside the
target project directory.

## Build System

Go releases are defined in `.goreleaser.yaml`, producing linux/darwin binaries
for amd64 and arm64. Version metadata is injected via ldflags into
`github.com/goosewin/gralph/cmd.Version`. CI runs `go test` and a Goreleaser
snapshot build to ensure cross-platform artifacts are generated. Tagged
releases use the Goreleaser GitHub Action.
