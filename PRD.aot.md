# Project Requirements Document

## Overview

Port gralph, an autonomous AI coding loop CLI, from bash to .NET 10 with AOT (Ahead-of-Time) compilation. The new implementation will be a cross-platform, single-file native executable that maintains full feature parity with the existing bash implementation while gaining improved startup time, type safety, and maintainability.

Intended users are developers who run autonomous AI coding sessions using Claude Code, OpenCode, Gemini CLI, or Codex CLI as backends.

## Problem Statement

- The current bash implementation requires bash 4+, jq, tmux, and platform-specific tools like flock or socat, limiting portability and increasing installation friction.
- Bash scripts are harder to maintain, test, and extend as complexity grows.
- Startup overhead and runtime performance suffer compared to compiled executables.
- Cross-platform support (especially Windows) is limited with bash.

## Solution

Rewrite gralph as a .NET 10 AOT console application using Spectre.Console for terminal UI. The new implementation will:

- Compile to a single native executable per platform (macOS, Linux, Windows).
- Eliminate external dependencies (jq, tmux, flock, socat).
- Use built-in .NET file locking and process management.
- Maintain identical CLI interface and configuration file formats.
- Support all existing backends (Claude, OpenCode, Gemini, Codex).

---

## Functional Requirements

### FR-1: CLI Command Parity

Implement all existing gralph commands with identical arguments and behavior:

- `start <dir>` with all existing options (--name, --max-iterations, --task-file, --backend, --model, etc.)
- `stop <name>` and `stop --all`
- `status` with formatted table output
- `logs <name>` with --follow support
- `resume [name]`
- `prd check <file>` and `prd create` with all options
- `worktree create <ID>` and `worktree finish <ID>`
- `backends`
- `config`, `config list`, `config get <key>`, `config set <key> <value>`
- `server` with --host, --port, --token, --open options
- `version` and `help`

### FR-2: Configuration Management

Support the existing YAML configuration format:

- Load default, global (~/.config/gralph/config.yaml), and project (.gralph.yaml) configs.
- Merge configs in order with later sources overriding earlier.
- Support environment variable overrides (GRALPH_*).
- Maintain backward compatibility with existing config files.

### FR-3: State Persistence

Implement session state management:

- Store state in ~/.config/gralph/state.json.
- Use cross-platform file locking (not flock).
- Support concurrent session operations safely.
- Detect and clean up stale sessions.

### FR-4: Backend Abstraction

Port the pluggable backend system:

- Define common interface for backends.
- Implement Claude, OpenCode, Gemini, and Codex backends.
- Parse JSON streaming output from each backend.
- Support model overrides per backend.

### FR-5: PRD Processing

Implement PRD validation and generation:

- Parse task blocks with headers, Context Bundle, DoD, Checklist, Dependencies.
- Validate required fields and detect stray unchecked tasks.
- Sanitize generated PRDs (remove Open Questions, validate Context Bundle paths).
- Detect project stack from ecosystem files.

### FR-6: Loop Execution

Port the core execution loop:

- Run iterations until completion or max iterations.
- Detect completion via zero remaining tasks and promise marker.
- Support foreground (--no-tmux) and background execution.
- Implement prompt template rendering with variable substitution.

### FR-7: Status Server

Implement the HTTP status API:

- GET /status - list all sessions
- GET /status/:name - get specific session
- POST /stop/:name - stop a session
- Support Bearer token authentication.
- Bind to configurable host/port.

### FR-8: Notifications

Support webhook notifications:

- Detect Discord, Slack, and generic webhook formats.
- Send completion and failure notifications.
- Include session details, iterations, duration in payloads.

---

## Non-Functional Requirements

### NFR-1: Cross-Platform Compatibility

- Build native executables for macOS (arm64, x64), Linux (x64, arm64), and Windows (x64).
- Ensure consistent behavior across all platforms.
- Use platform-agnostic file paths and process management.

### NFR-2: AOT Compilation

- Use .NET 10 Native AOT for single-file, self-contained executables.
- Minimize binary size through trimming.
- Ensure sub-100ms startup time.

### NFR-3: Terminal UI

- Use Spectre.Console for tables, prompts, progress, and markup.
- Support ANSI color codes with automatic capability detection.
- Graceful fallback for non-interactive terminals.

### NFR-4: Testability

- Design with unit testing in mind.
- Use dependency injection for external dependencies.
- Achieve minimum 80% code coverage on core logic.

---

## Implementation Tasks

### Task AOT-1

- **ID** AOT-1
- **Context Bundle** `bin/gralph`, `README.md`, `ARCHITECTURE.md`
- **DoD** .NET 10 project structure created with solution file, main project, and AOT publish profile.
- **Checklist**
  * Solution file exists at root.
  * Main project targets net10.0 with PublishAot enabled.
  * Project builds and produces native executable.
  * Spectre.Console package referenced.
- **Dependencies** None
- [x] AOT-1 Create .NET 10 solution and project structure with AOT configuration

### Task AOT-2

- **ID** AOT-2
- **Context Bundle** `bin/gralph`, `lib/config.sh`, `config/default.yaml`
- **DoD** Configuration system ported with YAML parsing, merging, and environment variable overrides.
- **Checklist**
  * Parses default, global, and project YAML configs.
  * Merges configs with correct precedence.
  * Environment variables override config values.
  * get_config and set_config methods work correctly.
- **Dependencies** AOT-1
- [x] AOT-2 Implement configuration loading and merging system

### Task AOT-3

- **ID** AOT-3
- **Context Bundle** `lib/state.sh`, `ARCHITECTURE.md`
- **DoD** State persistence module implemented with cross-platform file locking.
- **Checklist**
  * State stored in JSON format at ~/.config/gralph/state.json.
  * File locking prevents concurrent write races.
  * CRUD operations for sessions work correctly.
  * Stale session cleanup implemented.
- **Dependencies** AOT-1
- [x] AOT-3 Implement state persistence with file locking

### Task AOT-4

- **ID** AOT-4
- **Context Bundle** `lib/backends/common.sh`, `lib/backends/claude.sh`
- **DoD** Backend abstraction layer with Claude backend implemented.
- **Checklist**
  * IBackend interface defined with required methods.
  * Claude backend parses JSON streaming output.
  * Backend loader selects correct implementation.
  * Model override support works.
- **Dependencies** AOT-1
- [x] AOT-4 Create backend abstraction layer and Claude backend

### Task AOT-5

- **ID** AOT-5
- **Context Bundle** `lib/backends/opencode.sh`, `lib/backends/gemini.sh`, `lib/backends/codex.sh`
- **DoD** OpenCode, Gemini, and Codex backends implemented.
- **Checklist**
  * OpenCode backend supports provider/model format.
  * Gemini backend runs in headless mode.
  * Codex backend supports quiet and auto-approve flags.
  * All backends parse output correctly.
- **Dependencies** AOT-4
- [x] AOT-5 Implement OpenCode, Gemini, and Codex backends

### Task AOT-6

- **ID** AOT-6
- **Context Bundle** `lib/core.sh`, `lib/prd.sh`
- **DoD** Task block parsing and PRD validation ported.
- **Checklist**
  * Task blocks extracted with headers and fields.
  * Required fields validated (ID, Context Bundle, DoD, Checklist, Dependencies).
  * Stray unchecked tasks detected.
  * Context Bundle paths validated against repo.
- **Dependencies** AOT-1
- [x] AOT-6 Implement task block parsing and PRD validation

### Task AOT-7

- **ID** AOT-7
- **Context Bundle** `lib/core.sh`, `ARCHITECTURE.md`
- **DoD** Core execution loop implemented with iteration handling.
- **Checklist**
  * Prompt template rendered with variable substitution.
  * Iterations run until completion or max.
  * Completion detected via zero tasks and promise marker.
  * State callbacks update session progress.
- **Dependencies** AOT-3, AOT-4, AOT-6
- [x] AOT-7 Implement core execution loop with completion detection

### Task AOT-8

- **ID** AOT-8
- **Context Bundle** `bin/gralph`, `README.md`
- **DoD** CLI entry point with all commands registered.
- **Checklist**
  * All commands from usage help implemented.
  * Arguments parsed with correct types and defaults.
  * Help and version commands work.
  * Unknown commands show error with suggestion.
- **Dependencies** AOT-2, AOT-3, AOT-7
- [x] AOT-8 Implement CLI entry point with command routing

### Task AOT-9

- **ID** AOT-9
- **Context Bundle** `bin/gralph`, `lib/core.sh`
- **DoD** Start command implemented with foreground and background modes.
- **Checklist**
  * Session created in state.
  * Foreground mode runs loop directly.
  * Background mode spawns detached process.
  * Webhook option stored in session.
- **Dependencies** AOT-7, AOT-8
- [x] AOT-9 Implement start command with session management

### Task AOT-10

- **ID** AOT-10
- **Context Bundle** `bin/gralph`, `lib/state.sh`
- **DoD** Stop, status, logs, and resume commands implemented.
- **Checklist**
  * Stop kills process and updates state.
  * Status shows table with all sessions.
  * Logs tails session log file with --follow.
  * Resume restarts stale sessions.
- **Dependencies** AOT-8, AOT-3
- [x] AOT-10 Implement stop, status, logs, and resume commands

### Task AOT-11

- **ID** AOT-11
- **Context Bundle** `bin/gralph`, `lib/prd.sh`
- **DoD** PRD check and create commands implemented.
- **Checklist**
  * Check validates PRD and reports errors.
  * Create prompts for goal, constraints, sources.
  * Stack detection infers project technologies.
  * Generated PRD sanitized before writing.
- **Dependencies** AOT-6, AOT-8
- [ ] AOT-11 Implement prd check and prd create commands

### Task AOT-12

- **ID** AOT-12
- **Context Bundle** `bin/gralph`, `README.md`
- **DoD** Worktree create and finish commands implemented.
- **Checklist**
  * Create makes branch and worktree directory.
  * Finish merges branch and removes worktree.
  * Dirty git state rejected.
  * Task ID format validated.
- **Dependencies** AOT-8
- [ ] AOT-12 Implement worktree create and finish commands

### Task AOT-13

- **ID** AOT-13
- **Context Bundle** `lib/server.sh`, `ARCHITECTURE.md`
- **DoD** HTTP status server implemented.
- **Checklist**
  * GET /status returns all sessions.
  * GET /status/:name returns single session.
  * POST /stop/:name stops session.
  * Bearer token authentication works.
- **Dependencies** AOT-3, AOT-8
- [ ] AOT-13 Implement HTTP status server with authentication

### Task AOT-14

- **ID** AOT-14
- **Context Bundle** `lib/notify.sh`, `README.md`
- **DoD** Webhook notifications implemented.
- **Checklist**
  * Discord embed format generated correctly.
  * Slack block kit format generated correctly.
  * Generic JSON payload sent to other URLs.
  * Completion and failure events sent.
- **Dependencies** AOT-1
- [ ] AOT-14 Implement webhook notification system

### Task AOT-15

- **ID** AOT-15
- **Context Bundle** `bin/gralph`, `config/default.yaml`
- **DoD** Backends and config commands implemented.
- **Checklist**
  * Backends lists available backends with install status.
  * Config list shows merged configuration.
  * Config get retrieves specific key.
  * Config set updates global config file.
- **Dependencies** AOT-2, AOT-4, AOT-8
- [ ] AOT-15 Implement backends and config commands

### Task AOT-16

- **ID** AOT-16
- **Context Bundle** `README.md`, `ARCHITECTURE.md`
- **DoD** Cross-platform build and publish configuration complete.
- **Checklist**
  * Build script produces executables for macOS, Linux, Windows.
  * Executables are single-file and self-contained.
  * Startup time under 100ms verified.
  * Binary sizes documented.
- **Dependencies** AOT-8, AOT-9, AOT-10, AOT-11, AOT-12, AOT-13, AOT-14, AOT-15
- [ ] AOT-16 Configure cross-platform AOT builds and verify binaries

### Task AOT-17

- **ID** AOT-17
- **Context Bundle** `tests/config-test.sh`, `tests/state-test.sh`, `tests/prd-validation-test.sh`
- **DoD** Unit tests written for core modules.
- **Checklist**
  * Config loading and merging tested.
  * State persistence and locking tested.
  * PRD validation logic tested.
  * Minimum 80% coverage on core modules.
- **Dependencies** AOT-2, AOT-3, AOT-6
- [ ] AOT-17 Write unit tests for core modules

### Task AOT-18

- **ID** AOT-18
- **Context Bundle** `tests/backend-integration-test.sh`, `tests/loop-test.sh`
- **DoD** Integration tests verify end-to-end functionality.
- **Checklist**
  * Backend execution tested with mock responses.
  * Loop completion detection tested.
  * Command-line argument parsing tested.
  * Cross-platform test matrix passes.
- **Dependencies** AOT-7, AOT-8
- [ ] AOT-18 Write integration tests for loop and backends

---

## Success Criteria

- All existing gralph commands work identically in the .NET version.
- Native executables build for macOS (arm64, x64), Linux (x64, arm64), and Windows (x64).
- Startup time under 100ms on all platforms.
- Existing configuration files and state files are compatible.
- All tests pass with minimum 80% coverage on core logic.
- Binary sizes are under 20MB per platform.

---

## Sources

- https://github.com/spectreconsole/spectre.console
