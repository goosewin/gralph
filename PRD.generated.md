# Project Requirements Document: Coverage Target 80%

## Overview

Increase test coverage of the gralph Rust CLI codebase from the current 70.69% to 80% or higher. The goal is to harden the codebase by adding tests for uncovered paths in the core modules, ensuring reliability for autonomous AI coding loops.

## Problem Statement

- Current test coverage is 70.69% (3089/4370 lines covered).
- The soft coverage warning target is 80% but not yet met.
- Uncovered lines concentrate in src/app/loop_session.rs (427 uncovered), src/verifier.rs (289 uncovered), src/app.rs (250 uncovered), and src/prd.rs (173 uncovered).
- Without additional tests, edge cases and error paths remain unverified.

## Solution

Add targeted unit tests for the highest-impact uncovered lines across core modules. Prioritize error paths, edge cases, and helper functions that are invoked during loop execution, verifier pipelines, and PRD processing. Organize tasks by module to allow parallel execution and incremental progress.

---

## Functional Requirements

### FR-1: Loop Session Coverage

Add tests for uncovered paths in loop_session.rs including session lifecycle management, tmux command assembly, iteration state transitions, and failure notification flows.

### FR-2: Verifier Coverage

Add tests for uncovered verifier paths including static check parsing, PR creation error handling, review gate polling, and coverage extraction edge cases.

### FR-3: App Module Coverage

Add tests for uncovered app.rs paths including CLI argument validation, session name fallbacks, worktree error handling, and command dispatch flows.

### FR-4: PRD Module Coverage

Add tests for uncovered prd.rs paths including stack detection helpers, sanitization fallbacks, and validation error formatting.

---

## Non-Functional Requirements

### NFR-1: Coverage Threshold

- Final coverage must be at least 80% as measured by cargo tarpaulin with the standard exclusions.

### NFR-2: Test Isolation

- Tests must not require network access, external services, or filesystem side effects outside tempdir.
- Environment variable tests must use the env_lock helper.

---

## Implementation Tasks

### Task COV80-LS-1

- **ID** COV80-LS-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests covering loop_session session lifecycle helpers including start_session, stop_session, and iteration state tracking.
- **Checklist**
  * Tests cover lines 22-82 session initialization paths.
  * Tests cover lines 139-177 iteration state transitions.
  * Tests verify notification callback invocations.
- **Dependencies** None
- [x] COV80-LS-1 Add loop_session session lifecycle tests
---

### Task COV80-LS-2

- **ID** COV80-LS-2
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for loop_session tmux command assembly and session collision detection.
- **Checklist**
  * Tests cover lines 487-526 tmux spawn helpers.
  * Tests cover lines 570-599 session name collision handling.
  * Tests verify tmux unavailable error paths.
- **Dependencies** COV80-LS-1
- [ ] COV80-LS-2 Add loop_session tmux command and collision tests
---

### Task COV80-LS-3

- **ID** COV80-LS-3
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for loop_session failure paths including max iteration exit, error callbacks, and notification dispatch.
- **Checklist**
  * Tests cover lines 612-665 failure notification flows.
  * Tests cover lines 778-892 max iteration and error exits.
  * Tests verify completion callback invocations.
- **Dependencies** COV80-LS-1
- [ ] COV80-LS-3 Add loop_session failure path tests
---

### Task COV80-LS-4

- **ID** COV80-LS-4
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for loop_session dry-run mode, step execution, and prompt rendering paths.
- **Checklist**
  * Tests cover lines 905-999 dry-run print paths.
  * Tests cover lines 1001-1064 step iteration flow.
  * Tests verify prompt template injection.
- **Dependencies** COV80-LS-2
- [ ] COV80-LS-4 Add loop_session dry-run and step tests
---

### Task COV80-VER-1

- **ID** COV80-VER-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for verifier run_verifier_pipeline entry paths including stack detection and command resolution.
- **Checklist**
  * Tests cover lines 56-139 pipeline entry and command defaults.
  * Tests verify Rust vs non-Rust stack command selection.
  * Tests cover coverage extraction from tarpaulin output.
- **Dependencies** None
- [ ] COV80-VER-1 Add verifier pipeline entry tests
---

### Task COV80-VER-2

- **ID** COV80-VER-2
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for verifier static check helpers including file collection, violation detection, and duplicate block scanning.
- **Checklist**
  * Tests cover lines 403-465 static check pipeline.
  * Tests cover lines 467-510 PR creation flow.
  * Tests verify violation sorting and formatting.
- **Dependencies** COV80-VER-1
- [ ] COV80-VER-2 Add verifier static check tests
---

### Task COV80-VER-3

- **ID** COV80-VER-3
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for verifier review gate polling including timeout, approval detection, and check status aggregation.
- **Checklist**
  * Tests cover lines 548-645 review gate polling.
  * Tests verify timeout and retry behavior.
  * Tests cover merge method selection.
- **Dependencies** COV80-VER-2
- [ ] COV80-VER-3 Add verifier review gate tests
---

### Task COV80-APP-1

- **ID** COV80-APP-1
- **Context Bundle** `src/cli.rs`
- **DoD** Add tests for app module CLI dispatch paths including argument parsing, subcommand routing, and error formatting.
- **Checklist**
  * Tests cover lines 55-123 CLI argument handling.
  * Tests cover lines 142-221 subcommand dispatch.
  * Tests verify error message formatting.
- **Dependencies** None
- [ ] COV80-APP-1 Add app CLI dispatch tests
---

### Task COV80-APP-2

- **ID** COV80-APP-2
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for app module worktree and session name resolution helpers.
- **Checklist**
  * Tests cover lines 274-299 worktree path resolution.
  * Tests cover lines 348-463 session name fallbacks.
  * Tests verify branch uniqueness checks.
- **Dependencies** COV80-APP-1
- [ ] COV80-APP-2 Add app worktree and naming tests
---

### Task COV80-APP-3

- **ID** COV80-APP-3
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for app module PRD init and doctor command paths.
- **Checklist**
  * Tests cover lines 467-570 doctor checks.
  * Tests cover lines 574-621 init scaffolding.
  * Tests verify error path formatting.
- **Dependencies** COV80-APP-1
- [ ] COV80-APP-3 Add app doctor and init tests
---

### Task COV80-PRD-1

- **ID** COV80-PRD-1
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for prd module error types, validation helpers, and display formatting.
- **Checklist**
  * Tests cover lines 44-62 PrdError Display and source.
  * Tests cover lines 89-127 prd_validate_file entry.
  * Tests verify error message formatting.
- **Dependencies** None
- [ ] COV80-PRD-1 Add PRD error and validation tests
---

### Task COV80-PRD-2

- **ID** COV80-PRD-2
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for prd module sanitize helpers including context filtering and block sanitization.
- **Checklist**
  * Tests cover lines 172-279 prd_sanitize_generated_file and prd_sanitize_contents.
  * Tests verify allowed context filtering.
  * Tests cover Open Questions section removal.
- **Dependencies** COV80-PRD-1
- [ ] COV80-PRD-2 Add PRD sanitize tests
---

### Task COV80-PRD-3

- **ID** COV80-PRD-3
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for prd module stack detection helpers including framework and tool detection.
- **Checklist**
  * Tests cover lines 281-555 prd_detect_stack.
  * Tests verify package.json dependency parsing.
  * Tests cover Gemfile, mix.exs, and composer.json detection.
- **Dependencies** COV80-PRD-1
- [ ] COV80-PRD-3 Add PRD stack detection tests
---

### Task COV80-STATE-1

- **ID** COV80-STATE-1
- **Context Bundle** `src/state.rs`, `src/config.rs`
- **DoD** Add tests for state module uncovered paths including lock acquisition, cleanup, and parse_value edge cases.
- **Checklist**
  * Tests cover lines 35-52 StateError Display and source.
  * Tests cover lines 277-278 cleanup edge cases.
  * Tests cover lines 355-422 parse_value edge cases.
- **Dependencies** None
- [ ] COV80-STATE-1 Add state store edge case tests
---

### Task COV80-CONFIG-1

- **ID** COV80-CONFIG-1
- **Context Bundle** `src/config.rs`, `src/state.rs`
- **DoD** Add tests for config module uncovered paths including loader edge cases and env override conflicts.
- **Checklist**
  * Tests cover lines 22-43 Config creation paths.
  * Tests cover lines 84-90 merge precedence.
  * Tests cover lines 157-230 env override handling.
- **Dependencies** None
- [ ] COV80-CONFIG-1 Add config loader edge case tests
---

### Task COV80-WT-1

- **ID** COV80-WT-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for worktree module uncovered paths including git output parsing and error handling.
- **Checklist**
  * Tests cover lines 13-36 git_output_in_dir.
  * Tests cover lines 62-122 worktree creation.
  * Tests cover lines 151-210 worktree finish.
- **Dependencies** None
- [ ] COV80-WT-1 Add worktree git helper tests
---

### Task COV80-PI-1

- **ID** COV80-PI-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for prd_init module uncovered paths including template scaffolding and file write error handling.
- **Checklist**
  * Tests cover lines 35-53 init entry paths.
  * Tests cover lines 84-139 template rendering.
  * Tests cover lines 152-221 file write flows.
- **Dependencies** None
- [ ] COV80-PI-1 Add prd_init template and error tests
---

### Task COV80-UPDATE-1

- **ID** COV80-UPDATE-1
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add tests for update module uncovered paths including version parsing and archive extraction errors.
- **Checklist**
  * Tests cover lines 64-101 version resolution.
  * Tests cover lines 173-225 archive extraction.
  * Tests verify permission error handling.
- **Dependencies** None
- [ ] COV80-UPDATE-1 Add update version and extract tests
---

### Task COV80-FINAL-1

- **ID** COV80-FINAL-1
- **Context Bundle** `CHANGELOG.md`, `PROCESS.md`
- **DoD** Run cargo tarpaulin with standard exclusions and verify coverage is at least 80%. Update CHANGELOG with verification note.
- **Checklist**
  * Coverage output shows at least 80%.
  * All tests pass.
  * CHANGELOG entry added with verification note.
- **Dependencies** COV80-LS-4, COV80-VER-3, COV80-APP-3, COV80-PRD-3, COV80-STATE-1, COV80-CONFIG-1, COV80-WT-1, COV80-PI-1, COV80-UPDATE-1
- [ ] COV80-FINAL-1 Verify 80% coverage threshold met
---

## Success Criteria

- Test coverage measured by cargo tarpaulin with standard exclusions reaches at least 80%.
- All 900+ existing tests continue to pass.
- No regressions in CI pipeline.
- CHANGELOG updated with verification note.

---

## Sources

- None provided.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
- Coverage percentages are based on current tarpaulin output and may shift as code changes.
