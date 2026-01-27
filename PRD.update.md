# Project Requirements Document (Template)

## Overview

Improve the gralph CLI release readiness by fixing installer cleanup and version verification, adding update awareness and an update command, and preparing the 0.2.1 release. Intended users are developers running gralph loops locally who need reliable installation and update flows.

## Problem Statement

- Installer cleanup can fail under `set -u` because `tmp_dir` is scoped to `main`, and PATH-based version checks can report a different binary than the one installed into `$INSTALL_DIR`.
- The CLI lacks a built-in update check and update command, so users can run stale versions without notice.
- `gralph start .` can fail with “Invalid directory name” when the path has no basename.

## Solution

Harden `install.sh` to safely clean temp directories and verify `$INSTALL_DIR/gralph` directly. Add a Rust update helper that checks for newer releases and prints a non-blocking message at session start. Introduce `gralph update`, refresh docs and shell completions, update the changelog, and bump the Cargo version to 0.2.1.

---

## Functional Requirements

### FR-1: Installer Reliability

`install.sh` must avoid unbound `tmp_dir` errors on exit and verify the binary installed in `$INSTALL_DIR`, with accurate PATH messaging.

### FR-2: Update Awareness on Session Start

`gralph start` and `gralph run-loop` must emit a best-effort update notice when a newer release exists, without blocking session startup.

### FR-3: CLI Update Command

`gralph update` must install the latest release (or the version specified via environment overrides) and return clear errors on failure.

### FR-4: Session Name Fallback

`gralph start` must accept `.` and paths without a basename by falling back to a safe session name derived from the canonical path (or a default).

---

## Non-Functional Requirements

### NFR-1: Reliability

- Update checks are best-effort with a short timeout and never prevent session start.
- Installer cleanup must not error under `set -u`.

### NFR-2: Quality Gates

- Tests remain green and coverage stays >= 90%.

---

## Implementation Tasks

### Task INST-1

- **ID** INST-1
- **Context Bundle** `README.md`, `CHANGELOG.md`
- **DoD** `install.sh` cleans temp directories safely under `set -u` and verifies `$INSTALL_DIR/gralph` with accurate PATH warnings.
- **Checklist**
  * Trap handles unset `tmp_dir` without aborting.
  * Verification uses `$INSTALL_DIR/gralph --version` and warns when `command -v gralph` resolves elsewhere.
- **Dependencies** None
- [x] INST-1 Harden install.sh temp cleanup and PATH verification
### Task START-1

- **ID** START-1
- **Context Bundle** `src/main.rs`
- **DoD** `gralph start .` no longer errors with “Invalid directory name” and uses a stable fallback session name; unit tests cover the fallback behavior.
- **Checklist**
  * Resolve the basename from a canonicalized path when possible.
  * Fall back to a default session name when no basename exists.
  * Add tests for `.` and root-path cases.
- **Dependencies** None
- [x] START-1 Add session name fallback for dot and root paths
### Task UPD-1

- **ID** UPD-1
- **Context Bundle** `src/main.rs`, `src/lib.rs`
- **DoD** A new update helper checks the latest release version, compares to `CARGO_PKG_VERSION`, and prints a non-blocking update message on session start; unit tests cover version comparison and release parsing.
- **Checklist**
  * Update check uses a short timeout and logs warnings on failure.
  * Session start path calls the update check once per run.
  * Tests validate newer-version detection and parsing errors.
- **Dependencies** None
- [ ] UPD-1 Add update check message on session start
### Task UPD-2

- **ID** UPD-2
- **Context Bundle** `src/cli.rs`, `src/main.rs`, `src/lib.rs`
- **DoD** `gralph update` is wired in the CLI and uses the update helper to install the latest release, respecting `GRALPH_INSTALL_DIR` and `GRALPH_VERSION`; errors return non-zero exit status.
- **Checklist**
  * `Command` enum and help output include `update`.
  * Dispatch path runs update flow and surfaces failures.
  * Tests confirm CLI parsing accepts `update`.
- **Dependencies** UPD-1
- [ ] UPD-2 Add gralph update subcommand for release installs
### Task DOC-1

- **ID** DOC-1
- **Context Bundle** `README.md`, `completions/gralph.bash`, `completions/gralph.zsh`, `src/cli.rs`
- **DoD** Docs mention `gralph update` and the session-start update notice; shell completions are regenerated to include the new command.
- **Checklist**
  * `README.md` command list includes `gralph update` and update check note.
  * `docs/cli.md` documents `gralph update` usage.
  * Completions are regenerated from `src/cli.rs` and committed.
- **Dependencies** UPD-2
- [ ] DOC-1 Update docs and completions for update feature
### Task REL-1

- **ID** REL-1
- **Context Bundle** `CHANGELOG.md`, `Cargo.toml`
- **DoD** `Cargo.toml` version is 0.2.1 and `CHANGELOG.md` includes Added/Fixed entries for this work plus an updated verification line.
- **Checklist**
  * Version bumped to 0.2.1 in `Cargo.toml`.
  * Unreleased entries reference INST-1, START-1, UPD-1, UPD-2, DOC-1.
  * Verification line updated with test and coverage status.
- **Dependencies** INST-1, UPD-1, UPD-2, DOC-1
- [ ] REL-1 Prep 0.2.1 version and changelog
---

## Success Criteria

- `install.sh` exits cleanly under `set -u` and verifies `$INSTALL_DIR/gralph`.
- `gralph start .` runs without a session name error.
- Session start prints an update notice when a newer release exists without blocking.
- `gralph update` installs the latest release successfully and updates `gralph --version`.
- Docs and completions include the new command, and `CHANGELOG.md` reflects the changes.
- Tests pass with coverage >= 90%.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
