# Project Requirements Document

## Overview

Port the gralph CLI from bash to Go while preserving all existing functionality. The Go implementation will provide a single static binary with improved performance, better error handling, type safety, and cross-platform compatibility. Target users are developers running autonomous AI coding loops via Claude Code, OpenCode, Gemini CLI, or Codex CLI backends.

## Problem Statement

- Bash implementation requires bash 4+, jq, tmux, and other system dependencies making installation complex.
- Shell scripts are difficult to test, debug, and maintain as the codebase grows.
- No compile-time type checking leads to runtime errors from typos or missing variables.
- Cross-platform portability issues between Linux and macOS shell behaviors.
- Performance overhead from spawning subprocesses for JSON parsing and string manipulation.

## Solution

Rewrite gralph in Go using Cobra for CLI scaffolding, Viper for configuration, and Bubble Tea or Lip Gloss for optional TUI enhancements. The Go binary will embed all functionality in a single executable with no external dependencies beyond the AI backend CLIs themselves.

---

## Functional Requirements

### FR-1: Core Loop Engine

Port lib/core.sh loop logic to Go including task counting, iteration execution, completion detection, and prompt rendering with template substitution.

### FR-2: CLI Command Parity

Implement all existing commands: start, stop, status, logs, resume, prd (check/create), worktree (create/finish), backends, config, server, version, and help.

### FR-3: State Management

Port lib/state.sh session persistence using JSON state file with file locking, atomic writes, and stale session cleanup.

### FR-4: Configuration System

Port lib/config.sh YAML configuration loading with default, global, and project-level config merging plus environment variable overrides.

### FR-5: Backend Abstraction

Port lib/backends/*.sh backend interface supporting Claude, OpenCode, Gemini, and Codex CLIs with pluggable architecture.

### FR-6: Notification System

Port lib/notify.sh webhook notifications for Discord, Slack, and generic JSON endpoints.

### FR-7: HTTP Status Server

Port lib/server.sh REST API for remote monitoring with bearer token authentication.

### FR-8: PRD Utilities

Port lib/prd.sh stack detection, task block parsing, validation, and sanitization logic.

---

## Non-Functional Requirements

### NFR-1: Performance

- CLI startup time under 50ms for simple commands.
- State file operations complete within 100ms.

### NFR-2: Reliability

- Atomic state file writes prevent corruption.
- Graceful handling of backend CLI failures.
- Signal handling for clean shutdown.

### NFR-3: Portability

- Single static binary for Linux amd64, Linux arm64, macOS amd64, macOS arm64.
- No runtime dependencies beyond system libc.

### NFR-4: Maintainability

- Idiomatic Go code with clear package boundaries.
- Comprehensive test coverage targeting 80%+.
- Structured logging with configurable levels.

---

## Implementation Tasks

### Task GO-1

- **ID** GO-1
- **Context Bundle** `bin/gralph`, `README.md`, `ARCHITECTURE.md`
- **DoD** Go module initialized with Cobra CLI scaffold and basic main entrypoint compiling successfully.
- **Checklist**
  * go.mod and go.sum exist with module path.
  * cmd/root.go defines root command with version and help.
  * main.go calls root command Execute.
  * go build produces working binary.
- **Dependencies** None
- [x] GO-1 Initialize Go module with Cobra CLI scaffold

### Task GO-2

- **ID** GO-2
- **Context Bundle** `lib/config.sh`, `config/default.yaml`
- **DoD** Configuration package loads and merges YAML files with Viper supporting environment overrides.
- **Checklist**
  * internal/config package implements LoadConfig, GetConfig, SetConfig.
  * Supports default, global, and project-level config files.
  * Environment variable overrides work with GRALPH_ prefix.
  * Unit tests cover config merging and overrides.
- **Dependencies** GO-1
- [ ] GO-2 Implement configuration loading package

### Task GO-3

- **ID** GO-3
- **Context Bundle** `lib/state.sh`
- **DoD** State management package handles session CRUD with file locking and atomic writes.
- **Checklist**
  * internal/state package implements InitState, GetSession, SetSession, DeleteSession, ListSessions, CleanupStale.
  * File locking prevents concurrent access corruption.
  * Atomic writes via temp file and rename.
  * Unit tests cover all state operations.
- **Dependencies** GO-1
- [ ] GO-3 Implement state management package

### Task GO-4

- **ID** GO-4
- **Context Bundle** `lib/backends/common.sh`, `lib/backends/claude.sh`
- **DoD** Backend abstraction interface defined with Claude backend implementation.
- **Checklist**
  * internal/backend package defines Backend interface with CheckInstalled, GetModels, RunIteration, ParseText.
  * internal/backend/claude implements Claude Code backend.
  * Backend registry loads backends by name.
  * Unit tests verify interface compliance.
- **Dependencies** GO-1
- [ ] GO-4 Implement backend abstraction with Claude backend

### Task GO-5

- **ID** GO-5
- **Context Bundle** `lib/backends/opencode.sh`, `lib/backends/gemini.sh`, `lib/backends/codex.sh`
- **DoD** OpenCode, Gemini, and Codex backends implemented and registered.
- **Checklist**
  * internal/backend/opencode implements OpenCode backend.
  * internal/backend/gemini implements Gemini CLI backend.
  * internal/backend/codex implements Codex CLI backend.
  * All backends pass interface validation.
- **Dependencies** GO-4
- [ ] GO-5 Implement OpenCode, Gemini, and Codex backends

### Task GO-6

- **ID** GO-6
- **Context Bundle** `lib/core.sh`
- **DoD** Core loop engine ported to Go with task counting, iteration execution, and completion detection.
- **Checklist**
  * internal/core package implements RunLoop, RunIteration, CountRemainingTasks, CheckCompletion.
  * Prompt template rendering with variable substitution.
  * Task block extraction and selection.
  * Integration tests verify loop behavior.
- **Dependencies** GO-2, GO-3, GO-4
- [ ] GO-6 Implement core loop engine

### Task GO-7

- **ID** GO-7
- **Context Bundle** `bin/gralph`
- **DoD** Start command implemented launching loops in foreground or tmux session.
- **Checklist**
  * cmd/start.go implements start command with all flags.
  * Foreground mode runs loop directly.
  * Background mode spawns tmux session.
  * State saved on start.
- **Dependencies** GO-6
- [ ] GO-7 Implement start command

### Task GO-8

- **ID** GO-8
- **Context Bundle** `bin/gralph`
- **DoD** Stop, status, logs, and resume commands implemented.
- **Checklist**
  * cmd/stop.go stops sessions by name or all.
  * cmd/status.go lists sessions with formatted output.
  * cmd/logs.go displays session logs with follow mode.
  * cmd/resume.go restarts stale sessions.
- **Dependencies** GO-3, GO-7
- [ ] GO-8 Implement stop, status, logs, and resume commands

### Task GO-9

- **ID** GO-9
- **Context Bundle** `lib/notify.sh`
- **DoD** Notification package sends webhooks for Discord, Slack, and generic endpoints.
- **Checklist**
  * internal/notify package implements NotifyComplete, NotifyFailed.
  * Webhook type detection from URL.
  * Platform-specific payload formatting.
  * Unit tests verify payload structure.
- **Dependencies** GO-1
- [ ] GO-9 Implement notification package

### Task GO-10

- **ID** GO-10
- **Context Bundle** `lib/server.sh`
- **DoD** HTTP status server implemented with REST endpoints and bearer auth.
- **Checklist**
  * internal/server package implements StartServer with /status, /status/:name, /stop/:name endpoints.
  * Bearer token authentication.
  * CORS headers for browser access.
  * cmd/server.go exposes server command.
- **Dependencies** GO-3
- [ ] GO-10 Implement HTTP status server

### Task GO-11

- **ID** GO-11
- **Context Bundle** `lib/prd.sh`
- **DoD** PRD utilities ported including stack detection, validation, and sanitization.
- **Checklist**
  * internal/prd package implements DetectStack, ValidateFile, SanitizeGeneratedFile.
  * Task block parsing and extraction.
  * Context Bundle path validation.
  * Unit tests cover validation rules.
- **Dependencies** GO-1
- [ ] GO-11 Implement PRD utilities package

### Task GO-12

- **ID** GO-12
- **Context Bundle** `bin/gralph`
- **DoD** PRD check and create commands implemented.
- **Checklist**
  * cmd/prd.go implements prd check and prd create subcommands.
  * Check validates PRD against schema rules.
  * Create generates PRD via backend with interactive prompts.
  * Sanitization applied to generated output.
- **Dependencies** GO-4, GO-11
- [ ] GO-12 Implement PRD check and create commands

### Task GO-13

- **ID** GO-13
- **Context Bundle** `bin/gralph`
- **DoD** Worktree create and finish commands implemented.
- **Checklist**
  * cmd/worktree.go implements worktree create and worktree finish subcommands.
  * Creates task branch and worktree directory.
  * Merges branch and removes worktree on finish.
  * Validates clean git state before operations.
- **Dependencies** GO-1
- [ ] GO-13 Implement worktree commands

### Task GO-14

- **ID** GO-14
- **Context Bundle** `bin/gralph`
- **DoD** Backends and config commands implemented.
- **Checklist**
  * cmd/backends.go lists available backends with install status.
  * cmd/config.go implements config get, set, and list subcommands.
- **Dependencies** GO-2, GO-4
- [ ] GO-14 Implement backends and config commands

### Task GO-15

- **ID** GO-15
- **Context Bundle** `completions/gralph.bash`, `completions/gralph.zsh`
- **DoD** Shell completions generated via Cobra for bash, zsh, and fish.
- **Checklist**
  * Root command has completion subcommand.
  * Generates bash completions.
  * Generates zsh completions.
  * Generates fish completions.
- **Dependencies** GO-8, GO-12, GO-13, GO-14
- [ ] GO-15 Add shell completion generation

### Task GO-16

- **ID** GO-16
- **Context Bundle** `README.md`, `ARCHITECTURE.md`
- **DoD** Build system produces cross-platform binaries with versioning.
- **Checklist**
  * Makefile or goreleaser config for builds.
  * Builds for linux/amd64, linux/arm64, darwin/amd64, darwin/arm64.
  * Version injected via ldflags.
  * CI workflow runs tests and builds.
- **Dependencies** GO-15
- [ ] GO-16 Set up build system and CI

### Task GO-17

- **ID** GO-17
- **Context Bundle** `README.md`
- **DoD** Installation documentation updated for Go binary distribution.
- **Checklist**
  * README includes Go binary installation instructions.
  * Release artifacts listed with download links.
  * Migration notes from bash version.
  * Shell completion setup documented.
- **Dependencies** GO-16
- [ ] GO-17 Update installation documentation

### Task GO-18

- **ID** GO-18
- **Context Bundle** `ARCHITECTURE.md`, `DECISIONS.md`
- **DoD** Architecture and decisions documents updated for Go implementation.
- **Checklist**
  * ARCHITECTURE reflects Go package structure.
  * DECISIONS records Go port rationale.
  * Module diagram shows package dependencies.
- **Dependencies** GO-17
- [ ] GO-18 Update architecture documentation

---

## Success Criteria

- All existing gralph commands work identically in the Go implementation.
- Test coverage reaches 80% across packages.
- Single binary runs on Linux and macOS without external dependencies.
- Benchmark shows 2x faster startup than bash version.
- Existing configuration files and state files remain compatible.

---

## Sources

- https://www.reddit.com/r/golang/comments/seg2sx/recommended_frameworklibrary_for_creating_cli/
- https://github.com/charmbracelet/bubbletea
- https://github.com/spf13/cobra
- https://github.com/charmbracelet
- https://github.com/charmbracelet/lipgloss
- https://github.com/charmbracelet/glow
- https://github.com/charmbracelet/crush
