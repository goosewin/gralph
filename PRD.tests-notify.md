# PRD: Notification Test Coverage

## Goal
Validate webhook type detection, payload formatting, and HTTP delivery behavior without external services.

## Non-Goals
- No changes to payload schemas or webhook formatting.
- No external network calls.

## Constraints
- Use local test HTTP server to capture requests.
- Keep tests deterministic.

## Success Criteria
- Payload formatting is verified for Discord, Slack, and generic webhooks.
- HTTP request behavior is verified for success and failure cases.

## Tasks

### Task T-NOTIFY-1

- **ID** T-NOTIFY-1
- **Context Bundle** `src/notify.rs`
- **DoD** Unit tests cover webhook type detection and payload shape formatting.
- **Checklist**
  * Discord payload fields and color values.
  * Slack block structure and text formatting.
  * Generic JSON payload fields.
- **Dependencies** None
- [x] T-NOTIFY-1 Add webhook type and payload formatting tests

### Task T-NOTIFY-2

- **ID** T-NOTIFY-2
- **Context Bundle** `src/notify.rs`
- **DoD** HTTP delivery tests verify headers, body, and response handling.
- **Checklist**
  * Local server captures request body.
  * Non-2xx response is handled per current logic.
- **Dependencies** T-NOTIFY-1
- [x] T-NOTIFY-2 Add send_webhook request/response tests
