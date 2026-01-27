# PRD: Backend Adapter Test Coverage

## Goal
Validate backend CLI adapters for command construction, env usage, output parsing, and error behavior without calling real CLIs.

## Non-Goals
- No changes to backend protocol or CLI flags.
- No external network or real CLI binaries.

## Constraints
- Use temp dir + fake executables on PATH.
- Keep tests deterministic and OS-portable.

## Success Criteria
- Each backend has tests for success and non-zero exit.
- Registry selection and model listing are covered.

## Tasks

### Task T-BACKEND-1

- **ID** T-BACKEND-1
- **Context Bundle** `src/backend/claude.rs`, `src/backend/opencode.rs`, `src/backend/gemini.rs`, `src/backend/codex.rs`
- **DoD** Shared test helper can create a fake CLI binary and inject into PATH for adapter tests.
- **Checklist**
  * Fake binary can emit stdout and stderr.
  * Exit code is configurable per test.
- **Dependencies** None
- [x] T-BACKEND-1 Add reusable fake-CLI helper for backend tests

### Task T-BACKEND-2

- **ID** T-BACKEND-2
- **Context Bundle** `src/backend/claude.rs`, `src/backend/opencode.rs`, `src/backend/gemini.rs`, `src/backend/codex.rs`
- **DoD** Each backend has tests for successful run and failure exit.
- **Checklist**
  * Verify command arguments and env are passed.
  * Verify parse_text behavior on expected output.
  * Verify error mapping on non-zero exit.
- **Dependencies** T-BACKEND-1
- [ ] T-BACKEND-2 Add backend run_iteration success/failure tests

### Task T-BACKEND-3

- **ID** T-BACKEND-3
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Registry tests cover backend selection and model listing.
- **Checklist**
  * Backend selection returns expected type.
  * Model listing is non-empty and stable.
- **Dependencies** None
- [ ] T-BACKEND-3 Add backend registry tests
