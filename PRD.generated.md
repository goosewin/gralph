# Project Requirements Document

## Overview

This PRD describes adding support for Gemini CLI and Codex CLI as new backend types in gralph. Gralph is an autonomous AI coding loop tool that spawns fresh AI coding sessions iteratively until all tasks in a PRD are complete. Currently it supports Claude Code and OpenCode backends. This enhancement adds two additional backends: Gemini CLI (Google's AI coding CLI) and Codex CLI (OpenAI's CLI). The intended users are developers who want flexibility in choosing their preferred AI coding assistant.

## Problem Statement

- Gralph currently supports only Claude Code and OpenCode as AI backends.
- Users who prefer Gemini CLI or Codex CLI cannot use gralph with their preferred tools.
- The backend system is extensible but lacks implementations for Gemini and Codex.

## Solution

Implement two new backend modules following the existing backend interface pattern defined in `lib/backends/common.sh`. Each backend will implement the required functions: `backend_name`, `backend_check_installed`, `backend_get_install_hint`, `backend_run_iteration`, `backend_parse_text`, `backend_get_models`, and `backend_get_default_model`. Update configuration, documentation, and add comprehensive tests for each new backend.

---

## Functional Requirements

### FR-1: Gemini CLI Backend

Implement a new backend module `lib/backends/gemini.sh` that integrates with Gemini CLI. The backend must support headless execution mode, model selection, and streaming output. It should detect the `gemini` CLI binary and provide installation hints when not found.

### FR-2: Codex CLI Backend

Implement a new backend module `lib/backends/codex.sh` that integrates with OpenAI Codex CLI. The backend must support non-interactive execution, model selection, and capture output for parsing. It should detect the `codex` CLI binary and provide installation hints when not found.

### FR-3: Configuration Support

Extend `config/default.yaml` with configuration sections for gemini and codex backends, including default models and any backend-specific flags or environment variables.

### FR-4: CLI Integration

Ensure the `gralph backends` command lists the new backends with correct installation status. The `--backend` flag on `gralph start` must accept `gemini` and `codex` as valid values.

### FR-5: Documentation

Update README.md with usage examples, installation instructions, and model formats for both new backends.

---

## Non-Functional Requirements

### NFR-1: Consistency

All backends must adhere to the same interface defined in `lib/backends/common.sh`. Output parsing and iteration behavior should be consistent across backends.

### NFR-2: Testability

Each backend must have dedicated test coverage verifying installation detection, iteration execution mocking, and output parsing.

---

## Implementation Tasks

### Task G-1

- **ID** G-1
- **Context Bundle** `lib/backends/common.sh`, `lib/backends/claude.sh`, `lib/backends/opencode.sh`
- **DoD** Create `lib/backends/gemini.sh` implementing all required backend interface functions for Gemini CLI headless mode.
- **Checklist**
  * File `lib/backends/gemini.sh` exists and is executable.
  * Functions `backend_name`, `backend_check_installed`, `backend_get_install_hint`, `backend_run_iteration`, `backend_parse_text`, `backend_get_models`, `backend_get_default_model` are implemented.
  * `backend_run_iteration` uses Gemini CLI headless mode with `--headless` flag.
  * Output is captured to the specified output file.
- **Dependencies** None
- [x] G-1 Implement Gemini CLI backend module

---

### Task G-2

- **ID** G-2
- **Context Bundle** `lib/backends/common.sh`, `lib/backends/claude.sh`, `lib/backends/opencode.sh`
- **DoD** Create `lib/backends/codex.sh` implementing all required backend interface functions for Codex CLI.
- **Checklist**
  * File `lib/backends/codex.sh` exists and is executable.
  * Functions `backend_name`, `backend_check_installed`, `backend_get_install_hint`, `backend_run_iteration`, `backend_parse_text`, `backend_get_models`, `backend_get_default_model` are implemented.
  * `backend_run_iteration` uses Codex CLI with appropriate non-interactive flags.
  * Output is captured to the specified output file.
- **Dependencies** None
- [x] G-2 Implement Codex CLI backend module

---

### Task G-3

- **ID** G-3
- **Context Bundle** `config/default.yaml`, `lib/config.sh`
- **DoD** Add gemini and codex configuration sections to `config/default.yaml` with default models and backend-specific settings.
- **Checklist**
  * `gemini:` section exists with `default_model` key.
  * `codex:` section exists with `default_model` key.
  * Configuration follows existing patterns from claude and opencode sections.
- **Dependencies** G-1, G-2
- [x] G-3 Add backend configuration sections for gemini and codex

---

### Task G-4

- **ID** G-4
- **Context Bundle** `tests/config-test.sh`, `tests/task-block-test.sh`, `lib/backends/common.sh`
- **DoD** Create test file `tests/backend-gemini-test.sh` with unit tests for the gemini backend.
- **Checklist**
  * Test file exists and passes when gemini backend is sourced.
  * Tests verify `backend_name` returns "gemini".
  * Tests verify `backend_get_models` returns expected model list.
  * Tests verify `backend_get_default_model` returns a valid default.
  * Tests verify `backend_check_installed` returns correct status based on CLI presence.
- **Dependencies** G-1
- [x] G-4 Add unit tests for gemini backend

---

### Task G-5

- **ID** G-5
- **Context Bundle** `tests/config-test.sh`, `tests/task-block-test.sh`, `lib/backends/common.sh`
- **DoD** Create test file `tests/backend-codex-test.sh` with unit tests for the codex backend.
- **Checklist**
  * Test file exists and passes when codex backend is sourced.
  * Tests verify `backend_name` returns "codex".
  * Tests verify `backend_get_models` returns expected model list.
  * Tests verify `backend_get_default_model` returns a valid default.
  * Tests verify `backend_check_installed` returns correct status based on CLI presence.
- **Dependencies** G-2
- [x] G-5 Add unit tests for codex backend

---

### Task G-6

- **ID** G-6
- **Context Bundle** `README.md`, `config/default.yaml`
- **DoD** Update README.md with documentation for gemini and codex backends including installation, usage examples, and model formats.
- **Checklist**
  * Requirements section lists gemini and codex CLI as optional dependencies.
  * Backends section documents gemini backend with installation command and model list.
  * Backends section documents codex backend with installation command and model list.
  * Configuration examples include gemini and codex sections.
  * Usage examples show `--backend gemini` and `--backend codex` flags.
- **Dependencies** G-1, G-2, G-3
- [x] G-6 Update README with gemini and codex backend documentation

---

### Task G-7

- **ID** G-7
- **Context Bundle** `bin/gralph`, `lib/backends/common.sh`
- **DoD** Verify `gralph backends` command correctly lists gemini and codex with installation status.
- **Checklist**
  * Running `gralph backends` shows gemini in the list.
  * Running `gralph backends` shows codex in the list.
  * Installation status (installed/not installed) is correct for each.
  * Install hints are displayed for backends that are not installed.
- **Dependencies** G-1, G-2
- [ ] G-7 Verify gralph backends command integration

---

### Task G-8

- **ID** G-8
- **Context Bundle** `tests/config-test.sh`, `lib/backends/common.sh`, `lib/core.sh`
- **DoD** Create integration test `tests/backend-integration-test.sh` that verifies all four backends can be loaded and validated.
- **Checklist**
  * Test file exists and is executable.
  * Test loads claude backend and validates required functions exist.
  * Test loads opencode backend and validates required functions exist.
  * Test loads gemini backend and validates required functions exist.
  * Test loads codex backend and validates required functions exist.
  * All tests pass.
- **Dependencies** G-1, G-2
- [ ] G-8 Add integration test for all backend modules

---

## Success Criteria

- All four backends (claude, opencode, gemini, codex) are listed by `gralph backends`.
- Each backend can be selected via `gralph start --backend <name>`.
- Configuration for each backend can be set and retrieved via `gralph config`.
- All unit tests pass for each backend.
- Integration test validates all backends implement the required interface.
- README documents all backends with installation and usage instructions.

---

## Sources

- https://geminicli.com/docs/
- https://geminicli.com/docs/get-started/
- https://geminicli.com/docs/cli/headless/
- https://developers.openai.com/codex/cli/features
- https://developers.openai.com/codex/cli/reference
