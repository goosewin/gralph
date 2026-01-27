# PRD: Core Loop + State Test Coverage

## Goal
Expand unit coverage of task parsing, loop control, completion detection, and state persistence.

## Non-Goals
- No changes to loop semantics or state file format.
- No external backend CLI calls.

## Constraints
- Use fake backend implementations in tests.
- Use temp directories for state files.

## Success Criteria
- Task parsing covers block boundaries and fallback rules.
- Loop logic covers success, failure, and max-iteration paths.
- State helpers cover corrupted/missing state error paths.

## Tasks

### Task T-CORE-1

- **ID** T-CORE-1
- **Context Bundle** `src/core.rs`, `README.md`
- **DoD** Tests cover task block parsing edge cases and fallback behavior.
- **Checklist**
  * Block termination on `---` and new section headings.
  * Fallback to single unchecked task when no blocks exist.
  * Ignore stray unchecked items outside blocks.
- **Dependencies** None
- [x] T-CORE-1 Add task parsing edge-case tests

### Task T-CORE-2

- **ID** T-CORE-2
- **Context Bundle** `src/core.rs`, `src/backend/mod.rs`, `src/state.rs`
- **DoD** Loop execution tests using a fake backend cover completion, error, and max-iteration paths.
- **Checklist**
  * Fake backend returns completion promise.
  * Loop stops on max iterations with failure status.
  * State updates reflect iterations and status.
- **Dependencies** None
- [x] T-CORE-2 Add loop execution tests with fake backend

### Task T-STATE-1

- **ID** T-STATE-1
- **Context Bundle** `src/state.rs`
- **DoD** State handling tests cover corrupted JSON and missing files.
- **Checklist**
  * Corrupted state file yields recovery behavior (per current logic).
  * Missing state file initializes cleanly.
- **Dependencies** None
- [x] T-STATE-1 Add state error-path tests
