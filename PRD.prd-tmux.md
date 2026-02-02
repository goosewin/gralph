# Project Requirements Document

## Overview

This PRD defines changes to the `gralph prd create` command to make it run in a background tmux session rather than blocking the terminal. The command currently runs synchronously, blocking the user's terminal until PRD generation completes. This change aligns `prd create` behavior with `gralph start`, which already uses tmux for background execution. Additionally, the multiline and interactive prompt options will be removed as they are no longer needed with background execution.

## Problem Statement

- The `gralph prd create` command blocks the terminal while the AI backend generates a PRD, which can take significant time depending on the backend and model.
- Users cannot continue working in the same terminal session while waiting for PRD generation.
- The multiline and interactive flags add complexity that is unnecessary for a background-based workflow.

## Solution

Refactor `gralph prd create` to spawn a background tmux session that runs the PRD generation, similar to how `gralph start` operates. Remove the `--multiline`, `--interactive`, and `--no-interactive` CLI flags since background execution eliminates the need for interactive prompts. Users can track progress via `gralph status` and view logs via `gralph logs`.

---

## Functional Requirements

### FR-1: Background Tmux Execution

When the user runs `gralph prd create`, the command must spawn a new tmux session in the background, run the PRD generation inside that session, and return control to the terminal immediately. The user should see output indicating the session name and how to check status/logs.

### FR-2: Remove Interactive and Multiline Options

The CLI flags `--multiline`, `--interactive`, and `--no-interactive` must be removed from `gralph prd create`. All references in help text, argument parsing, and implementation must be deleted.

### FR-3: Session State Tracking

The PRD generation session must be tracked in the state store so users can query its status with `gralph status` and stop it with `gralph stop`.

---

## Non-Functional Requirements

### NFR-1: Consistency

- The tmux session naming and spawning pattern must match the existing `gralph start` implementation to maintain a consistent user experience.

### NFR-2: Backward Compatibility

- Existing CLI arguments that are not being removed must continue to work identically.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task PRD-1

- **ID** PRD-1
- **Context Bundle** `src/cli.rs`
- **DoD** The `--multiline`, `--interactive`, and `--no-interactive` flags are removed from `PrdCreateArgs` and CLI help text.
- **Checklist**
  * The `multiline` field is removed from `PrdCreateArgs` struct.
  * The `interactive` field is removed from `PrdCreateArgs` struct.
  * The `no_interactive` field is removed from `PrdCreateArgs` struct.
  * The corresponding `#[arg(...)]` attributes are removed.
  * Help text in `ROOT_AFTER_HELP` no longer references these options.
  * Unit tests that reference these flags are updated or removed.
  * `cargo build` succeeds with no warnings related to these fields.
- **Dependencies** None
- [x] PRD-1 Remove multiline and interactive flags from PrdCreateArgs in cli.rs
### Task PRD-2

- **ID** PRD-2
- **Context Bundle** `src/cli.rs`
- **DoD** The `cmd_prd_create` function no longer references multiline or interactive fields.
- **Checklist**
  * Any code paths referencing `args.multiline` are removed.
  * Any code paths referencing `args.interactive` or `args.no_interactive` are removed.
  * The function compiles without unused field warnings.
- **Dependencies** PRD-1

- [ ] PRD-2 Remove multiline and interactive usage from cmd_prd_create
### Task PRD-3

- **ID** PRD-3
- **Context Bundle** `src/cli.rs`
- **DoD** A new `PrdRunArgs` struct and hidden `prd run` subcommand are added for background execution, mirroring `RunLoopArgs` and `run-loop`.
- **Checklist**
  * `PrdRunArgs` struct is defined with fields: dir, name, output, goal, constraints, context, sources, backend, model, variant, allow_missing_context, force, tmux_session.
  * A hidden `PrdRun` variant is added to `PrdCommand` enum.
  * The CLI parses `gralph prd run` with appropriate arguments.
  * `cargo test` for CLI parsing passes.
- **Dependencies** PRD-2

- [ ] PRD-3 Add PrdRunArgs struct and hidden prd run subcommand
### Task PRD-4

- **ID** PRD-4
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** A new `cmd_prd_run` function implements the actual PRD generation logic (extracted from existing `cmd_prd_create`), designed to run inside a tmux session.
- **Checklist**
  * `cmd_prd_run` accepts `PrdRunArgs` and performs PRD generation.
  * The function writes output to the specified file path.
  * The function updates session state on completion or failure.
  * Error handling logs to the session log file.
- **Dependencies** PRD-3

- [ ] PRD-4 Implement cmd_prd_run for background PRD generation
### Task PRD-5

- **ID** PRD-5
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** The `cmd_prd_create` function is refactored to spawn a background tmux session running `gralph prd run`, matching the `cmd_start` pattern.
- **Checklist**
  * `cmd_prd_create` calls `ensure_tmux_available`.
  * `cmd_prd_create` generates a unique tmux session name.
  * `cmd_prd_create` spawns `gralph prd run` inside the tmux session.
  * `cmd_prd_create` registers the session in the state store.
  * `cmd_prd_create` prints the session name, log path, and status command hint.
  * The command returns immediately after spawning.
- **Dependencies** PRD-4

- [ ] PRD-5 Refactor cmd_prd_create to spawn background tmux session
### Task PRD-6

- **ID** PRD-6
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** The main command dispatch in `app.rs` routes `PrdCommand::Run` to `cmd_prd_run`.
- **Checklist**
  * The `PrdCommand::Run` match arm calls `prd_init::cmd_prd_run`.
  * The dispatch compiles and routes correctly.
- **Dependencies** PRD-4

- [ ] PRD-6 Wire PrdCommand::Run dispatch in app.rs
### Task PRD-7

- **ID** PRD-7
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Unit tests verify the new background PRD generation flow.
- **Checklist**
  * A test verifies `cmd_prd_create` spawns a tmux session.
  * A test verifies `cmd_prd_run` generates and validates the PRD.
  * A test verifies session state is recorded correctly.
  * All existing PRD-related tests pass or are updated.
  * `cargo test` passes with no failures.
- **Dependencies** PRD-5, PRD-6

- [ ] PRD-7 Add unit tests for background PRD generation
---

## Success Criteria

- Running `gralph prd create --goal "..." --dir .` returns immediately with a session name.
- The PRD generation runs in a background tmux session.
- `gralph status` shows the PRD generation session with appropriate status.
- `gralph logs <session>` shows the PRD generation progress.
- The `--multiline`, `--interactive`, and `--no-interactive` flags are no longer accepted.
- All existing tests pass; new tests cover the background execution path.

---

## Sources

None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
