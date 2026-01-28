# Project Requirements Document (Template)

## Overview

Refactor the gralph Rust CLI codebase to improve maintainability, reduce duplication, and clarify module boundaries without changing user-facing behavior. The intended users are developers running gralph loops, PRD generation, and verifier workflows from the CLI.

## Problem Statement

- Core logic, PRD parsing, and backend adapters duplicate parsing and IO patterns across multiple files.
- Large functions in CLI entrypoints make changes risky and harder to reason about.
- Configuration, server responses, and notification formatting have repeated logic that is error-prone to maintain.

## Solution

Refactor shared utilities, extract cohesive helpers, and enforce consistent patterns across backends, PRD parsing, configuration, server handling, and verifier flow. Keep CLI behavior and outputs stable while documenting the refactor in shared context files.

---

## Functional Requirements

### FR-1: Behavior Parity

Refactor internal modules while preserving existing CLI commands, flags, output formats, and PRD generation behavior.

### FR-2: Consistent Internal APIs

Expose shared helpers for backend execution, task parsing, configuration lookup, and server response handling to reduce duplication and simplify future changes.

---

## Non-Functional Requirements

### NFR-1: Performance

Maintain current runtime behavior with no new per-iteration process spawns or network calls beyond existing backend invocations.

### NFR-2: Reliability

Preserve file locking, atomic writes, and error handling semantics across state, PRD validation, and verifier workflows.

---

## Implementation Tasks

### Task REF-1

- **ID** REF-1
- **Context Bundle** `src/backend/mod.rs`, `src/backend/opencode.rs`, `src/backend/gemini.rs`, `src/backend/codex.rs`
- **DoD** Shared backend helpers encapsulate command lookup and output streaming, with each backend delegating to them and maintaining identical outputs.
- **Checklist**
  * Backend helpers cover command lookup and line streaming.
  * Existing backend tests continue to pass without output changes.
- **Dependencies** None
- [x] REF-1 Consolidate shared backend execution helpers
---

### Task REF-2

- **ID** REF-2
- **Context Bundle** `src/core.rs`, `src/prd.rs`, `src/lib.rs`
- **DoD** Task block parsing and unchecked line detection are centralized and reused by core and PRD validation with no behavioral changes.
- **Checklist**
  * Shared parsing utilities replace duplicated logic.
  * Core and PRD tests remain green with identical semantics.
- **Dependencies** None
- [x] REF-2 Unify task block parsing utilities across core and PRD
---

### Task REF-3

- **ID** REF-3
- **Context Bundle** `src/config.rs`, `config/default.yaml`, `src/cli.rs`
- **DoD** Configuration lookup and environment overrides are simplified and tested for precedence and key normalization without altering external config behavior.
- **Checklist**
  * Config merge and override logic is centralized and documented in code.
  * Existing config tests pass and new edge cases are covered if needed.
- **Dependencies** None
- [x] REF-3 Simplify configuration merging and override handling
---

### Task REF-4

- **ID** REF-4
- **Context Bundle** `src/server.rs`, `src/state.rs`, `src/core.rs`
- **DoD** Server handlers share common auth and error response helpers, reducing duplication while keeping response schemas and status codes unchanged.
- **Checklist**
  * Auth failure responses are centralized and consistent.
  * Server tests pass with identical response payloads.
- **Dependencies** None
- [x] REF-4 Refactor server handler flow for shared auth and error responses
---

### Task REF-5

- **ID** REF-5
- **Context Bundle** `src/notify.rs`
- **DoD** Notification payload formatting reuses shared builders for common fields without changing JSON structure or content.
- **Checklist**
  * Discord, Slack, and generic payloads remain byte-for-byte equivalent where possible.
  * Notification tests pass without updates to expected payloads.
- **Dependencies** None
- [x] REF-5 Reduce duplication in notification payload formatting
---

### Task REF-6

- **ID** REF-6
- **Context Bundle** `src/main.rs`, `src/lib.rs`, `src/config.rs`
- **DoD** Verifier pipeline logic is modularized into focused helpers or a new module while keeping CLI behavior identical.
- **Checklist**
  * CLI command dispatch remains unchanged.
  * Verifier pipeline outputs and error messages remain stable.
- **Dependencies** None
- [x] REF-6 Modularize verifier pipeline logic out of main.rs
---

### Task REF-7

- **ID** REF-7
- **Context Bundle** `ARCHITECTURE.md`, `DECISIONS.md`, `CHANGELOG.md`, `PROCESS.md`, `RISK_REGISTER.md`
- **DoD** Documentation reflects the refactor scope, decisions, and verification status per process requirements.
- **Checklist**
  * ARCHITECTURE.md is updated with any module boundary changes.
  * CHANGELOG.md includes refactor task IDs and verification note.
- **Dependencies** REF-1, REF-2, REF-3, REF-4, REF-5, REF-6
- [ ] REF-7 Update shared docs and changelog for refactor outcomes
---

## Success Criteria

- All existing CLI commands and outputs behave the same as before the refactor.
- `cargo test --workspace` passes with coverage at or above 90 percent.
- Duplicate logic in backend execution, task parsing, config handling, server responses, and notifications is eliminated or centralized.

---

## Sources

- None.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
