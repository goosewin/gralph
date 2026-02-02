# PRD: Reliable PRD Generation

## Overview

This project enhances the `gralph prd create` command to improve generation reliability. Currently, LLM-generated PRDs fail validation approximately 50% of the time due to format violations. The solution introduces specification clarifications and an automated retry-recovery system that validates output and asks the LLM to fix issues iteratively until all validation passes.

## Problem Statement

- LLM backends frequently produce PRDs that fail strict validation checks.
- The PRD template and specification lack explicit constraints, leading to ambiguous outputs.
- Users must manually re-run generation or fix files, slowing workflow.

## Solution

Tighten the PRD specification with explicit format rules, embed these rules into the system prompt sent to LLM backends, and implement a post-generation retry loop that validates output and prompts the LLM to fix violations until validation succeeds or a retry limit is reached.

---

## Functional Requirements

### FR-1: Explicit PRD Specification Document

Create a machine-readable specification file that enumerates all validation rules. This document serves as the single source of truth referenced by both the validator and the LLM prompt.

### FR-2: Enhanced System Prompt for LLM Backends

Inject the specification rules directly into the system prompt sent during `gralph prd create` so the LLM understands exact formatting requirements before generation.

### FR-3: Post-Generation Validation Retry Loop

After initial generation, run `prd_validate_contents` on the output. If errors exist, send the PRD and error messages back to the LLM with a request to fix violations. Repeat until validation passes or a configurable retry limit is reached.

### FR-4: Retry Configuration

Expose `prd_create_max_retries` in `config/default.yaml` with a default value of 3. Allow users to override via environment variable or CLI flag.

---

## Non-Functional Requirements

### NFR-1: Reliability

- Generation success rate must reach at least 95% when retries are enabled.

### NFR-2: Performance

- Retry loop must not exceed 60 seconds total wall-clock time under normal network conditions.

### NFR-3: Observability

- Each retry attempt logs the validation errors and the corrective prompt sent.

---

## Implementation Tasks

Each task must use a `### Task <ID>` block header and include the required fields.
Each task block must contain exactly one unchecked task line.

### Task PRD-1

- **ID** PRD-1
- **Context Bundle** `PRD.template.md`, `src/prd.rs`
- **DoD** A new file `docs/PRD_SPEC.md` exists containing all validation rules extracted from `src/prd.rs`, written in a format suitable for inclusion in LLM prompts.
- **Checklist**
  * All rules from `prd_validate_contents` documented.
  * Rules for task block fields documented.
  * Rules for forbidden sections documented.
  * Rules for stray checkboxes documented.
- **Dependencies** None
- [x] PRD-1 Create PRD specification document
---

### Task PRD-2

- **ID** PRD-2
- **Context Bundle** `src/prd.rs`, `src/backend/mod.rs`
- **DoD** The `prd_create` function reads `docs/PRD_SPEC.md` and injects its contents into the system prompt sent to the backend.
- **Checklist**
  * Specification loaded at generation time.
  * Specification appended to system prompt.
  * Unit test verifies prompt contains specification text.
- **Dependencies** PRD-1
- [ ] PRD-2 Inject specification into LLM system prompt
---

### Task PRD-3

- **ID** PRD-3
- **Context Bundle** `src/prd.rs`, `src/backend/mod.rs`
- **DoD** A new function `prd_create_with_retry` wraps the existing generation logic, validates output, and re-prompts the LLM with error messages until validation passes or retries exhausted.
- **Checklist**
  * Retry loop implemented with configurable max retries.
  * Each iteration sends previous PRD and validation errors to LLM.
  * Loop exits on successful validation.
  * Loop exits when retry limit reached, returning best attempt.
- **Dependencies** PRD-2
- [ ] PRD-3 Implement retry loop for PRD generation
---

### Task PRD-4

- **ID** PRD-4
- **Context Bundle** `config/default.yaml`, `src/config.rs`, `src/cli.rs`
- **DoD** A new configuration key `prd_create_max_retries` is recognized in config, environment, and CLI, defaulting to 3.
- **Checklist**
  * Key added to `config/default.yaml`.
  * `Config` struct includes field.
  * CLI flag `--max-retries` added to `PrdCreateArgs`.
  * Environment variable `GRALPH_PRD_CREATE_MAX_RETRIES` recognized.
- **Dependencies** None
- [ ] PRD-4 Add retry configuration option
---

### Task PRD-5

- **ID** PRD-5
- **Context Bundle** `src/prd.rs`
- **DoD** Integration tests verify that a deliberately malformed PRD triggers retries and that a valid PRD is produced within the retry limit when the backend cooperates.
- **Checklist**
  * Test with mock backend returning invalid PRD first, valid second.
  * Test retry limit exhaustion returns best attempt.
  * Test valid first attempt returns immediately without retry.
- **Dependencies** PRD-3, PRD-4
- [ ] PRD-5 Add integration tests for retry loop
---

### Task PRD-6

- **ID** PRD-6
- **Context Bundle** `src/prd.rs`
- **DoD** Each retry attempt logs the iteration number, validation errors encountered, and length of corrective prompt sent.
- **Checklist**
  * Log messages at info level for retry start.
  * Log messages at debug level for error details.
  * No sensitive data logged.
- **Dependencies** PRD-3
- [ ] PRD-6 Add logging for retry attempts
---

### Task PRD-7

- **ID** PRD-7
- **Context Bundle** `CHANGELOG.md`
- **DoD** CHANGELOG updated with Unreleased entry describing the retry feature and specification document.
- **Checklist**
  * Added section under Unreleased.
  * Describes retry loop feature.
  * References PRD_SPEC.md.
- **Dependencies** PRD-5
- [ ] PRD-7 Update CHANGELOG with retry feature
---

## Success Criteria

- Running `gralph prd create` with retries enabled produces a valid PRD at least 95% of the time in automated tests with a cooperating mock backend.
- All existing `cargo test` suites pass.
- Coverage remains at or above 90% for `src/prd.rs`.

---

## Warnings

- No external authoritative sources were consulted for this PRD.
- Retry behavior with live LLM backends may vary; integration tests use mocks.
- Actual success rates depend on backend model quality and prompt adherence.
