# PRD: Server Test Coverage

## Goal
Validate the status API server for auth, endpoint behavior, and error handling.

## Non-Goals
- No changes to routes or response schemas.
- No real network bindings required.

## Constraints
- Use in-process axum router tests (tower ServiceExt).
- Use temp state files where needed.

## Success Criteria
- Auth required and enforced for protected endpoints.
- Status and stop endpoints return expected JSON and error responses.

## Tasks

### Task T-SERVER-1

- **ID** T-SERVER-1
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Tests cover `/status` and `/status/:name` with valid and invalid auth.
- **Checklist**
  * Missing token returns 401.
  * Valid token returns JSON with sessions list.
  * Unknown session returns not-found response.
- **Dependencies** None
- [x] T-SERVER-1 Add status endpoint auth + response tests

### Task T-SERVER-2

- **ID** T-SERVER-2
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Tests cover `/stop/:name` behavior and state updates.
- **Checklist**
  * Valid token stops existing session.
  * Missing session returns expected error.
- **Dependencies** None
- [x] T-SERVER-2 Add stop endpoint behavior tests

### Task T-SERVER-3

- **ID** T-SERVER-3
- **Context Bundle** `src/server.rs`
- **DoD** CORS and error responses are validated.
- **Checklist**
  * CORS headers present on status endpoint.
  * Error body matches expected schema.
- **Dependencies** T-SERVER-1
- [x] T-SERVER-3 Add CORS + error response tests
