# Project Requirements Document

## Overview

Add a `gralph init` CLI command that scaffolds the shared markdown metadata files (ARCHITECTURE.md, PROCESS.md, DECISIONS.md, RISK_REGISTER.md, CHANGELOG.md) in a target project directory so gralph loops can inject context reliably. Intended users are developers running gralph in new or existing repos.

## Problem Statement

- Creating the shared context files is manual and error-prone, slowing down first-time setup.
- Missing context files reduce the prompt context injected by `defaults.context_files`, leading to weaker loop performance.

## Solution

Implement a new `gralph init` subcommand in the Rust CLI that reads the configured context file list, creates any missing markdown context documents with ASCII starter templates, skips existing files by default, and reports the results.

---

## Functional Requirements

### FR-1: Init Command and File Selection

- Provide `gralph init` with `--dir` (defaults to current working directory) and `--force` (overwrite existing files) flags, consistent with existing CLI conventions.
- Determine the required context file list from `defaults.context_files` via `Config::load`, falling back to the canonical list documented in README when the config value is empty or missing.
- Only create markdown context files and create parent directories as needed; skip non-markdown entries with a clear message.

### FR-2: File Creation Behavior and Templates

- For each required context file, if missing, write an ASCII-only starter template with a top-level header and minimal sections aligned to the file purpose (architecture overview, process protocol, decisions log, risk register, changelog).
- If the file already exists, leave it unchanged unless `--force` is set; the command must be safe to rerun without data loss.
- Print a concise summary listing created, skipped, and overwritten files, and return a non-zero exit code if any write fails.

---

## Non-Functional Requirements

### NFR-1: Performance

- Completing `gralph init` on a local directory with the default context file list should finish in under 200ms on typical developer hardware.

### NFR-2: Reliability

- File writes must be atomic per file to avoid partial content on interruption.
- The command must be idempotent: rerunning without `--force` does not alter existing files.

---

## Implementation Tasks

### Task INIT-1

- **ID** INIT-1
- **Context Bundle** `src/cli.rs`, `src/main.rs`
- **DoD** The CLI exposes a new `init` subcommand with `--dir` and `--force` options, is included in help text and examples, and routes to a new handler.
- **Checklist**
  * Add `InitArgs` and `Command::Init` to the clap tree.
  * Wire `dispatch` and `cmd_intro` help text to include `gralph init`.
  * Ensure `gralph init --help` renders expected options.
- **Dependencies** None
- [x] INIT-1 Add init subcommand and routing
---

### Task INIT-2

- **ID** INIT-2
- **Context Bundle** `src/main.rs`, `src/config.rs`, `config/default.yaml`, `README.md`
- **DoD** `cmd_init` creates missing markdown context files derived from `defaults.context_files` (with README fallback), writes ASCII templates, skips existing files unless forced, creates parent directories, and prints a summary of actions.
- **Checklist**
  * Resolve required files from config and fall back to README list when empty.
  * Write starter templates for ARCHITECTURE.md, PROCESS.md, DECISIONS.md, RISK_REGISTER.md, and CHANGELOG.md when missing.
  * Preserve existing content unless `--force` is set.
- **Dependencies** INIT-1
- [x] INIT-2 Implement init scaffolding logic and templates
---

### Task INIT-3

- **ID** INIT-3
- **Context Bundle** `src/main.rs`, `src/config.rs`
- **DoD** Unit tests cover init idempotency, force overwrite behavior, invalid directory errors, and config fallback to the canonical file list.
- **Checklist**
  * Add temp-dir tests for created files and rerun with no changes.
  * Add a force overwrite test that rewrites existing files.
  * Add a failure test for missing target directory.
- **Dependencies** INIT-2
- [x] INIT-3 Add init unit tests for idempotency and force behavior
---

### Task INIT-4

- **ID** INIT-4
- **Context Bundle** `README.md`, `CHANGELOG.md`, `completions/gralph.bash`, `completions/gralph.zsh`
- **DoD** Documentation and completions reflect the new command, and the changelog records the addition.
- **Checklist**
  * Document `gralph init` usage and the files it creates in README.
  * Add an Added entry in CHANGELOG for the new command.
  * Update bash and zsh completions to include `init` and its flags.
- **Dependencies** INIT-2
- [ ] INIT-4 Update docs and completions for init
---

## Success Criteria

- Running `gralph init` in an empty project creates the five shared context documents with ASCII starter content.
- Re-running `gralph init` does not alter existing files unless `--force` is provided.
- `gralph --help` and shell completions list the `init` command and its options.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
