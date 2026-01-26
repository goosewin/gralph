# Project Requirements Document

## Overview

Port gralph from its current bash implementation to .NET 10, creating a cross-platform CLI tool that provides the same autonomous AI coding loop functionality. The tool will use Spectre.Console for rich terminal output and maintain feature parity with the existing bash version while leveraging .NET's type safety, testability, and cross-platform capabilities.

## Problem Statement

- The current bash implementation requires bash 4+ and platform-specific dependencies (tmux, jq, flock, socat/netcat) that limit portability.
- Windows support is limited to WSL2 only.
- Bash scripting lacks type safety, making refactoring and testing more difficult.
- Complex JSON parsing and state management in bash is fragile and hard to maintain.

## Solution

Rewrite gralph as a .NET 10 console application using System.CommandLine for CLI parsing and Spectre.Console for rich terminal output. The new implementation will be a single self-contained executable that runs on Windows, macOS, and Linux without requiring external dependencies beyond the AI backend CLIs.

---

## Functional Requirements

### FR-1: Core Loop Execution

The CLI must support starting, stopping, and monitoring autonomous AI coding loops. The loop reads a task file (PRD.md), counts unchecked tasks, invokes an AI backend, and repeats until completion or max iterations.

### FR-2: Session and State Management

The CLI must persist session state to disk using JSON, support concurrent sessions, and handle crash recovery through session resume functionality.

### FR-3: Backend Abstraction

The CLI must support multiple AI backends (claude, opencode, gemini, codex) through a pluggable adapter pattern, with each backend implementing a common interface.

### FR-4: Configuration System

The CLI must support hierarchical configuration (default, global, project-level) using YAML files, with environment variable overrides.

### FR-5: PRD Validation and Generation

The CLI must validate PRD task blocks for required fields and support generating spec-compliant PRDs using AI backends.

### FR-6: Status Server

The CLI must provide an HTTP status server for remote monitoring and session control.

### FR-7: Worktree Management

The CLI must support git worktree operations for task isolation.

### FR-8: Notifications

The CLI must support webhook notifications to Discord, Slack, and generic endpoints.

---

## Non-Functional Requirements

### NFR-1: Cross-Platform

The application must run on Windows, macOS, and Linux from a single codebase without platform-specific dependencies.

### NFR-2: Performance

Response times for CLI commands should be under 200ms for non-network operations.

### NFR-3: Testability

The codebase must be structured for unit testing with dependency injection and interface-based design.

---

## Implementation Tasks

### Task DOTNET-1

- **ID** DOTNET-1
- **Context Bundle** `README.md`, `bin/gralph`
- **DoD** .NET 10 solution scaffolded with project structure and Spectre.Console reference.
- **Checklist**
  * Solution file created with console app project.
  * Spectre.Console and System.CommandLine packages referenced.
  * Project targets net10.0 and builds successfully.
- **Dependencies** None
- [x] DOTNET-1 Scaffold .NET 10 solution with Spectre.Console

### Task DOTNET-2

- **ID** DOTNET-2
- **Context Bundle** `bin/gralph`, `README.md`
- **DoD** CLI entrypoint with root command and subcommand structure matching bash version.
- **Checklist**
  * Root command defined with version and help options.
  * Subcommands registered: start, stop, status, logs, resume, prd, worktree, backends, config, server, version.
  * Help output matches bash version structure.
- **Dependencies** DOTNET-1
- [x] DOTNET-2 Implement CLI command structure with System.CommandLine

### Task DOTNET-3

- **ID** DOTNET-3
- **Context Bundle** `lib/config.sh`, `config/default.yaml`
- **DoD** Configuration system ported to C# with YAML parsing and environment variable overrides.
- **Checklist**
  * YAML configuration file loading implemented.
  * Hierarchical merge logic (default, global, project) working.
  * Environment variable override pattern implemented.
  * Configuration cache accessible throughout application.
- **Dependencies** DOTNET-1
- [x] DOTNET-3 Implement configuration loading and merge system

### Task DOTNET-4

- **ID** DOTNET-4
- **Context Bundle** `lib/state.sh`
- **DoD** Session state management ported to C# with JSON persistence and file locking.
- **Checklist**
  * State file JSON schema defined.
  * CRUD operations for sessions implemented.
  * Cross-platform file locking implemented.
  * Atomic write with temp file and rename working.
  * Stale session cleanup logic ported.
- **Dependencies** DOTNET-1
- [x] DOTNET-4 Implement session state persistence with JSON

### Task DOTNET-5

- **ID** DOTNET-5
- **Context Bundle** `lib/backends/common.sh`, `lib/backends/claude.sh`
- **DoD** Backend abstraction layer implemented with IBackend interface and Claude adapter.
- **Checklist**
  * IBackend interface defined with required methods.
  * Backend discovery and loading mechanism implemented.
  * Claude backend adapter implemented.
  * Backend CLI check and install hint methods working.
- **Dependencies** DOTNET-1
- [x] DOTNET-5 Implement backend abstraction and Claude adapter

### Task DOTNET-6

- **ID** DOTNET-6
- **Context Bundle** `lib/core.sh`
- **DoD** Core loop logic ported including task counting, prompt rendering, and completion detection.
- **Checklist**
  * Task block parsing from PRD files implemented.
  * Prompt template rendering with variable substitution working.
  * Completion detection logic ported (zero tasks + valid promise).
  * Iteration execution with backend invocation working.
- **Dependencies** DOTNET-3, DOTNET-4, DOTNET-5
- [x] DOTNET-6 Implement core loop execution logic

### Task DOTNET-7

- **ID** DOTNET-7
- **Context Bundle** `bin/gralph`, `lib/core.sh`
- **DoD** Start command fully implemented with all options.
- **Checklist**
  * All start command options parsed and validated.
  * Session creation and state persistence working.
  * Background execution mode implemented (replacement for tmux).
  * Foreground mode (no-tmux equivalent) working.
- **Dependencies** DOTNET-2, DOTNET-6
- [x] DOTNET-7 Implement start command with session management

### Task DOTNET-8

- **ID** DOTNET-8
- **Context Bundle** `bin/gralph`, `lib/state.sh`
- **DoD** Status, stop, logs, and resume commands implemented.
- **Checklist**
  * Status command displays session table with Spectre.Console.
  * Stop command terminates sessions by name or all.
  * Logs command reads and optionally follows log files.
  * Resume command restarts stale or stopped sessions.
- **Dependencies** DOTNET-2, DOTNET-4
- [x] DOTNET-8 Implement status, stop, logs, and resume commands

### Task DOTNET-9

- **ID** DOTNET-9
- **Context Bundle** `lib/prd.sh`
- **DoD** PRD validation and generation commands implemented.
- **Checklist**
  * PRD task block extraction and validation logic ported.
  * Validation rules enforced (required fields, single unchecked line, etc.).
  * PRD generation command with AI backend integration working.
  * Sanitization of generated PRDs implemented.
- **Dependencies** DOTNET-2, DOTNET-5
- [x] DOTNET-9 Implement PRD validation and generation

### Task DOTNET-10

- **ID** DOTNET-10
- **Context Bundle** `lib/server.sh`
- **DoD** HTTP status server implemented using Kestrel or minimal APIs.
- **Checklist**
  * HTTP endpoints for /status, /status/:name, /stop/:name implemented.
  * Bearer token authentication working.
  * CORS headers for browser access configured.
  * Server binds to configurable host and port.
- **Dependencies** DOTNET-2, DOTNET-4
- [x] DOTNET-10 Implement HTTP status server

### Task DOTNET-11

- **ID** DOTNET-11
- **Context Bundle** `lib/notify.sh`
- **DoD** Webhook notification system implemented for Discord, Slack, and generic endpoints.
- **Checklist**
  * Webhook type detection from URL implemented.
  * Discord embed payload format implemented.
  * Slack block kit payload format implemented.
  * Generic JSON payload format implemented.
  * HttpClient-based webhook delivery working.
- **Dependencies** DOTNET-1
- [x] DOTNET-11 Implement webhook notifications

### Task DOTNET-12

- **ID** DOTNET-12
- **Context Bundle** `bin/gralph`
- **DoD** Git worktree commands implemented.
- **Checklist**
  * Worktree create command with task-ID naming convention working.
  * Worktree finish command with merge and cleanup working.
  * Dirty working tree detection implemented.
- **Dependencies** DOTNET-2
- [ ] DOTNET-12 Implement worktree create and finish commands

### Task DOTNET-13

- **ID** DOTNET-13
- **Context Bundle** `lib/backends/codex.sh`, `lib/backends/gemini.sh`, `lib/backends/opencode.sh`
- **DoD** Additional backend adapters implemented (opencode, gemini, codex).
- **Checklist**
  * OpenCode backend adapter implemented.
  * Gemini backend adapter implemented.
  * Codex backend adapter implemented.
  * Backend listing command shows all adapters with install status.
- **Dependencies** DOTNET-5
- [ ] DOTNET-13 Implement opencode, gemini, and codex backend adapters

### Task DOTNET-14

- **ID** DOTNET-14
- **Context Bundle** `README.md`, `bin/gralph`
- **DoD** Config command implemented for viewing and modifying configuration.
- **Checklist**
  * Config list subcommand displays merged configuration.
  * Config get subcommand retrieves specific keys.
  * Config set subcommand writes to global config file.
- **Dependencies** DOTNET-2, DOTNET-3
- [ ] DOTNET-14 Implement config command

### Task DOTNET-15

- **ID** DOTNET-15
- **Context Bundle** `tests/state-test.sh`, `tests/config-test.sh`
- **DoD** Unit test project created with tests for core components.
- **Checklist**
  * Test project created with xUnit or NUnit.
  * State management tests ported.
  * Configuration loading tests ported.
  * PRD validation tests ported.
- **Dependencies** DOTNET-3, DOTNET-4, DOTNET-9
- [ ] DOTNET-15 Create unit test project with core component tests

### Task DOTNET-16

- **ID** DOTNET-16
- **Context Bundle** `README.md`
- **DoD** Build produces self-contained executables for Windows, macOS, and Linux.
- **Checklist**
  * PublishSingleFile and SelfContained build configuration added.
  * Windows x64 executable builds successfully.
  * macOS x64 and arm64 executables build successfully.
  * Linux x64 executable builds successfully.
- **Dependencies** DOTNET-7
- [ ] DOTNET-16 Configure cross-platform publish profiles

---

## Success Criteria

- All commands from the bash version have equivalent functionality in the .NET version.
- The tool runs on Windows, macOS, and Linux without requiring bash or platform-specific dependencies.
- Unit tests pass and cover core functionality.
- Build produces self-contained executables under 50MB per platform.

---

## Sources

- https://github.com/spectreconsole/spectre.console
