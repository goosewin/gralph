# Project Requirements Document: Port gralph from Bash to Rust

## Overview

Port the gralph CLI from its current Bash implementation to Rust, preserving all existing functionality while gaining type safety, cross-platform compatibility, and improved performance. The intended users are developers who run autonomous AI coding loops using Claude Code, OpenCode, Gemini CLI, or Codex CLI backends.

## Problem Statement

- The current Bash implementation requires bash 4+ and platform-specific utilities (flock, jq, tmux, socat/nc) limiting portability.
- Shell scripts lack static type checking, making refactoring error-prone.
- Concurrent state access relies on flock with a fallback, which is fragile across platforms.
- YAML parsing uses a custom awk script that supports only limited features.
- Testing requires spawning subshells and is slow to run.

## Solution

Rewrite the gralph CLI in Rust using clap for argument parsing, serde/serde_yaml for configuration, and crossterm/indicatif/console for terminal output. Provide the same commands, flags, and behaviors so existing users can switch without workflow changes.

---

## Functional Requirements

### FR-1: CLI Commands

Implement all existing subcommands: start, stop, status, logs, resume, prd (check, create), worktree (create, finish), backends, config (list, get, set), server, version, and help.

### FR-2: Backend Abstraction

Support pluggable backends (claude, opencode, gemini, codex) with the same interface: check_installed, get_install_hint, run_iteration, parse_text, and get_models.

### FR-3: State Management

Persist session state to ~/.config/gralph/state.json with file locking for concurrent access. Provide CRUD operations and stale session cleanup.

### FR-4: Configuration Loading

Load and merge YAML configuration from default, global, project, and environment sources using the same precedence as the Bash version.

### FR-5: Notifications

Send webhook notifications for completion and failure events to Discord, Slack, or generic endpoints with auto-detected payload formats.

### FR-6: PRD Validation and Generation

Validate task blocks for required fields and sanitize generated PRDs by filtering Context Bundle entries and removing disallowed sections.

### FR-7: HTTP Status Server

Expose /status and /stop endpoints with optional bearer token authentication and CORS support.

---

## Non-Functional Requirements

### NFR-1: Performance

- CLI startup under 50ms on modern hardware.
- Iteration overhead under 100ms excluding backend latency.

### NFR-2: Portability

- Compile to static binaries for Linux (x86_64, aarch64) and macOS (x86_64, aarch64).
- No runtime dependency on jq, awk, or flock.

### NFR-3: Compatibility

- Accept all existing command-line flags and environment variable overrides.
- Read and write state.json in the same format as the Bash version.

---

## Implementation Tasks

### Task RS-1

- **ID** RS-1
- **Context Bundle** `bin/gralph`, `README.md`
- **DoD** Rust project initialised with Cargo.toml, clap dependency, and skeleton main.rs that prints version.
- **Checklist**
  * cargo new gralph-rs created.
  * Cargo.toml includes clap with derive feature.
  * Running cargo run -- --version prints version string.
- **Dependencies** None
- [x] RS-1 Scaffold Rust project with clap CLI skeleton

---

### Task RS-2

- **ID** RS-2
- **Context Bundle** `lib/config.sh`, `config/default.yaml`
- **DoD** Config module loads, merges, and queries YAML configuration using serde_yaml.
- **Checklist**
  * Reads default.yaml, global config, and project config.
  * Environment variable overrides work.
  * Unit tests cover merge precedence.
- **Dependencies** RS-1
- [x] RS-2 Implement configuration loading with serde_yaml

---

### Task RS-3

- **ID** RS-3
- **Context Bundle** `lib/state.sh`
- **DoD** State module provides session CRUD with file locking via fs2 or fd-lock.
- **Checklist**
  * init_state, get_session, set_session, list_sessions, delete_session, cleanup_stale implemented.
  * State file format matches Bash version.
  * Unit tests for locking and atomic writes.
- **Dependencies** RS-1
- [x] RS-3 Implement state management with file locking

---

### Task RS-4

- **ID** RS-4
- **Context Bundle** `lib/backends/common.sh`, `lib/backends/claude.sh`
- **DoD** Backend trait defined with Claude implementation that spawns the CLI and parses JSON output.
- **Checklist**
  * Backend trait has check_installed, run_iteration, parse_text, get_models.
  * Claude backend spawns claude CLI and streams output.
  * Unit tests mock backend execution.
- **Dependencies** RS-2
- [x] RS-4 Define backend trait and implement Claude backend

---

### Task RS-5

- **ID** RS-5
- **Context Bundle** `lib/backends/opencode.sh`, `lib/backends/gemini.sh`, `lib/backends/codex.sh`
- **DoD** OpenCode, Gemini, and Codex backends implemented following the backend trait.
- **Checklist**
  * Each backend spawns its respective CLI.
  * Model formats preserved.
  * Integration test stubs created.
- **Dependencies** RS-4
- [x] RS-5 Implement remaining backends (opencode, gemini, codex)

---

### Task RS-6

- **ID** RS-6
- **Context Bundle** `lib/core.sh`
- **DoD** Core loop logic ported: run_iteration, count_remaining_tasks, check_completion, and run_loop.
- **Checklist**
  * Task block extraction works.
  * Completion detection matches Bash logic.
  * Loop respects max iterations and state callbacks.
- **Dependencies** RS-3, RS-4
- [x] RS-6 Port core loop logic to Rust

---

### Task RS-7

- **ID** RS-7
- **Context Bundle** `lib/notify.sh`
- **DoD** Notification module sends webhooks with Discord, Slack, and generic payload formats using reqwest.
- **Checklist**
  * detect_webhook_type implemented.
  * JSON payloads match Bash output.
  * Timeouts and error handling present.
- **Dependencies** RS-1
- [ ] RS-7 Implement webhook notifications with reqwest

---

### Task RS-8

- **ID** RS-8
- **Context Bundle** `lib/prd.sh`
- **DoD** PRD validation and sanitization ported with regex-based task block parsing.
- **Checklist**
  * prd_validate_file and prd_sanitize_generated_file match Bash behavior.
  * Stack detection produces same output.
  * Unit tests for edge cases.
- **Dependencies** RS-2
- [ ] RS-8 Port PRD validation and sanitization

---

### Task RS-9

- **ID** RS-9
- **Context Bundle** `lib/server.sh`
- **DoD** HTTP status server implemented using hyper or axum with bearer token auth.
- **Checklist**
  * GET /status, GET /status/:name, POST /stop/:name endpoints work.
  * CORS headers match Bash version.
  * Integration test for auth.
- **Dependencies** RS-3
- [ ] RS-9 Implement HTTP status server with hyper/axum

---

### Task RS-10

- **ID** RS-10
- **Context Bundle** `bin/gralph`, `completions/gralph.bash`, `completions/gralph.zsh`
- **DoD** All CLI subcommands wired to Rust modules; shell completions generated via clap_complete.
- **Checklist**
  * gralph start, stop, status, logs, resume, prd, worktree, backends, config, server, version, help all functional.
  * Bash and Zsh completions generated at build time.
  * End-to-end smoke test passes.
- **Dependencies** RS-6, RS-7, RS-8, RS-9
- [ ] RS-10 Wire CLI subcommands and generate shell completions

---

### Task RS-11

- **ID** RS-11
- **Context Bundle** `tests/config-test.sh`, `tests/state-test.sh`
- **DoD** Unit and integration tests ported; CI workflow added for cargo test.
- **Checklist**
  * Tests cover config, state, backends, core, prd, notify, server modules.
  * GitHub Actions workflow runs tests on push.
  * Code coverage threshold set.
- **Dependencies** RS-10
- [ ] RS-11 Add unit and integration tests with CI workflow

---

### Task RS-12

- **ID** RS-12
- **Context Bundle** `README.md`, `ARCHITECTURE.md`
- **DoD** Documentation updated for Rust build and install instructions; architecture doc reflects new module layout.
- **Checklist**
  * README includes cargo install instructions.
  * ARCHITECTURE.md describes Rust modules.
  * Migration notes for Bash users added.
- **Dependencies** RS-11
- [ ] RS-12 Update documentation for Rust implementation

---

## Success Criteria

- All existing gralph commands produce equivalent output when run against the Rust binary.
- State file format remains compatible; sessions created by Bash can be read by Rust and vice versa.
- CI passes with unit and integration tests on Linux and macOS.
- Static binaries published for Linux x86_64, Linux aarch64, macOS x86_64, and macOS aarch64.

---

## Sources

- https://github.com/clap-rs/clap
- https://crates.io/crates/console
- https://crates.io/crates/indicatif
- https://crates.io/crates/crossterm
