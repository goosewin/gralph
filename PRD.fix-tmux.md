# Project Requirements Document (Template)

## Overview

Update gralph (Rust CLI) to require tmux for loop starts, always launch PRD runs inside a uniquely named tmux session, and run the loop from a .worktree/<unique-worktree> directory unless --no-worktree is set.

## Problem Statement

- tmux is currently optional via --no-tmux, which allows foreground runs that cannot be attached later and leads to inconsistent session management.
- PRD starts do not guarantee a stable, attachable tmux session name or a consistent worktree run directory, making recovery and collaboration harder.

## Solution

Make tmux a required dependency for start runs, remove --no-tmux, create a unique tmux session for every PRD start, persist that session name in state for attach and stop, add a gralph CLI attach flow, and ensure auto worktree runs always cd into .worktree/<unique-worktree> (unless --no-worktree).

---

## Functional Requirements

### FR-1: Required tmux session on start

Every gralph start must fail fast if tmux is unavailable and must launch the loop in a unique tmux session whose name is stored in state and shown for attach.

### FR-2: Attach via tmux CLI and gralph CLI

Users must be able to attach to the session at any time with tmux and with a gralph CLI command that resolves the session name from state.

### FR-3: Worktree run directory

When auto worktree is enabled, start must cd into .worktree/<unique-worktree> (preserving subdirectory runs) and run the loop there; when --no-worktree is set, it must run in the original directory.

---

## Non-Functional Requirements

### NFR-1: Performance

Start overhead for tmux session creation should remain minimal and not noticeably delay loop startup.

### NFR-2: Reliability

Start must either create and record a tmux session successfully or exit with a clear error; state must always contain the tmux session name for running loops.

---

## Implementation Tasks

### Task TMUX-1

- **ID** TMUX-1
- **Context Bundle** `src/cli.rs`, `README.md`, `completions/gralph.bash`, `completions/gralph.zsh`
- **DoD** Remove --no-tmux from CLI flags/help and completions, and update README Requirements and Logs wording to state tmux is required.
- **Checklist**
  * CLI help no longer lists --no-tmux.
  * Shell completions do not offer --no-tmux.
  * README Requirements and Logs sections reflect tmux as required.
- **Dependencies** None
- [x] TMUX-1 Remove no-tmux flag and docs
### Task TMUX-2

- **ID** TMUX-2
- **Context Bundle** `ARCHITECTURE.md`, `src/state.rs`, `src/server.rs`, `src/lib.rs`
- **DoD** Start validates tmux availability, creates a unique tmux session name, records it in state (tmux_session), and ensures stop uses that value when present.
- **Checklist**
  * Start fails with a clear error when tmux is missing.
  * State entries include tmux_session for running loops.
  * Stop uses tmux_session to terminate the session when present.
- **Dependencies** TMUX-1
- [ ] TMUX-2 Require tmux session and state
### Task TMUX-3

- **ID** TMUX-3
- **Context Bundle** `src/cli.rs`, `src/state.rs`, `README.md`, `completions/gralph.bash`
- **DoD** Add a gralph CLI attach command that attaches to the tmux session by name (from state) and document the command.
- **Checklist**
  * New CLI command attaches to the tmux session by name.
  * Errors are clear when no session or tmux_session is found.
  * README includes the attach command usage.
- **Dependencies** TMUX-2
- [ ] TMUX-3 Add attach command
### Task WT-1

- **ID** WT-1
- **Context Bundle** `PROCESS.md`, `README.md`, `ARCHITECTURE.md`, `config/default.yaml`
- **DoD** Auto worktree runs use .worktree/<unique-worktree> as the working directory (unless --no-worktree) and docs reference the updated path.
- **Checklist**
  * Worktree path in docs uses .worktree/<unique-worktree>.
  * Start runs inside the worktree when auto worktree is enabled.
  * --no-worktree keeps the original working directory.
- **Dependencies** TMUX-2
- [ ] WT-1 Worktree run directory
---

## Success Criteria

- gralph start always creates a unique tmux session, records it in state, and provides a reliable attach path.
- tmux is required for start runs; --no-tmux is removed from CLI and docs.
- Auto worktree runs execute from .worktree/<unique-worktree> unless --no-worktree.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
