# PRD: Increase Test Coverage to 90%

## Objective

Increase gralph test coverage from 85.59% to 90% by adding targeted unit tests
for modules with the lowest coverage percentages.

## Background

Current coverage stands at 85.59% (4953/5787 lines covered). The target is 90%,
requiring approximately 255 additional lines of coverage. Analysis identifies
these modules as having the largest coverage gaps:

- src/app/loop_session.rs: 64% covered (526/822 lines) - highest priority
- src/app.rs: 74% covered (378/508 lines)
- src/verifier.rs: 86% covered (944/1097 lines)
- src/server.rs: 86% covered (224/260 lines)
- src/backend/claude.rs: 88% covered (81/92 lines)

## Warnings

- No external sources were provided; all tasks are derived from repository files
- Coverage percentages based on cargo tarpaulin output at time of analysis
- Some functions may require mocking external dependencies (tmux, backend CLIs)

## Success Criteria

- Test coverage reaches 90% or higher as measured by cargo tarpaulin
- All existing tests continue to pass
- New tests follow existing test patterns and conventions in the codebase

---

## Implementation Tasks

### Task COV-001

- **ID** COV-001
- **Context Bundle** src/app/loop_session.rs:22-137, src/app.rs:133-181, src/cli.rs
- **DoD** Tests cover cmd_start error paths (non-existent directory), cmd_start_dry_run prompt rendering, and session state initialization.
- **Checklist**
  * Test cmd_start returns error when directory does not exist
  * Test cmd_start_dry_run renders prompt and task block correctly
  * Test cmd_start_dry_run with strict_prd validation
  * Test cmd_start_dry_run with custom prompt_template
  * Test run_loop_args_from_start conversion
- **Dependencies** None
- [ ] COV-001 Add tests for loop_session cmd_start and cmd_start_dry_run

---

### Task COV-002

- **ID** COV-002
- **Context Bundle** src/app/loop_session.rs:161-191, src/app/loop_session.rs:427-455, src/state.rs
- **DoD** Tests cover stop_session logic, stop --all behavior, cleanup with purge flag, and cleanup with remove flag.
- **Checklist**
  * Test cmd_stop with specific session name
  * Test cmd_stop with --all flag stops multiple sessions
  * Test cmd_stop returns error when session not found
  * Test cmd_cleanup with purge flag
  * Test cmd_cleanup with remove flag marks/removes stale sessions
- **Dependencies** None
- [ ] COV-002 Add tests for loop_session cmd_stop and cmd_cleanup

---

### Task COV-003

- **ID** COV-003
- **Context Bundle** src/app/loop_session.rs:193-266, src/app/loop_session.rs:268-349, src/app/loop_session.rs:351-425
- **DoD** Tests cover JSON output format, verbose output, session enrichment with remaining task counts, and log file resolution.
- **Checklist**
  * Test cmd_status with empty sessions returns appropriate message
  * Test cmd_status with --json flag returns valid JSON
  * Test cmd_status with --verbose flag includes extended info
  * Test enrich_status_session adds current_remaining count
  * Test enrich_status_session marks stale sessions correctly
  * Test resolve_status_log_file with empty and non-empty paths
- **Dependencies** None
- [ ] COV-003 Add tests for loop_session cmd_status and enrich_status_session

---

### Task COV-004

- **ID** COV-004
- **Context Bundle** src/app/loop_session.rs:457-485, src/app/loop_session.rs:487-529, src/app/loop_session.rs:531-628
- **DoD** Tests cover log file resolution, follow mode behavior, attach error handling, and resume session state transitions.
- **Checklist**
  * Test cmd_logs returns error when session not found
  * Test cmd_logs returns error when log file does not exist
  * Test cmd_logs with --raw flag uses raw_log_file
  * Test cmd_attach returns error when no tmux session recorded
  * Test cmd_resume identifies sessions needing resume
  * Test should_resume_session helper function
- **Dependencies** None
- [ ] COV-004 Add tests for loop_session cmd_logs, cmd_attach, cmd_resume

---

### Task COV-005

- **ID** COV-005
- **Context Bundle** src/app/loop_session.rs:663-776, src/app/loop_session.rs:778-end
- **DoD** Tests cover configuration resolution, outcome status planning with verifier, and notification decision logic.
- **Checklist**
  * Test resolve_task_file with args, config, and defaults
  * Test resolve_max_iterations with args, config, and defaults
  * Test resolve_completion_marker with args, config, and defaults
  * Test resolve_backend_name with args, config, and defaults
  * Test resolve_model with opencode special case
  * Test outcome_status_plan returns Verify when auto_run_verifier is true
  * Test notification_decision for Complete, Failed, MaxIterations statuses
  * Test should_check_for_update with env var and config
- **Dependencies** COV-001
- [ ] COV-005 Add tests for loop_session run_loop_with_state and iteration logic

---

### Task COV-006

- **ID** COV-006
- **Context Bundle** src/app.rs:201-223, src/app.rs:302-338, src/app.rs:340-398
- **DoD** Tests cover command dispatch routing, doctor check execution with various config states, and backends listing.
- **Checklist**
  * Test dispatch routes commands to correct handlers
  * Test cmd_doctor with valid config
  * Test cmd_doctor with invalid config shows failure
  * Test cmd_doctor with missing directory returns error
  * Test cmd_backends lists installed and not-installed backends
  * Test DoctorStatus as_str conversion
- **Dependencies** None
- [ ] COV-006 Add tests for app.rs command dispatch and doctor checks

---

### Task COV-007

- **ID** COV-007
- **Context Bundle** src/verifier.rs:246-298, src/verifier.rs:307-359, src/verifier.rs:361-400
- **DoD** Tests cover static check configuration parsing, command parsing with shell words, and coverage percentage extraction from various output formats.
- **Checklist**
  * Test parse_verifier_command with simple and complex commands
  * Test parse_verifier_command_tokens with quoted arguments
  * Test extract_coverage_percent with tarpaulin output
  * Test extract_coverage_percent with line coverage format
  * Test coverage_percent_from_line with various formats
  * Test parse_percent_from_line edge cases
- **Dependencies** None
- [ ] COV-007 Add tests for verifier static checks

---

### Task COV-008

- **ID** COV-008
- **Context Bundle** src/verifier.rs:160-244, src/verifier.rs:79-158
- **DoD** Tests cover PR creation logic, review gate polling, and configuration resolution from args and config.
- **Checklist**
  * Test resolve_verifier_command with explicit and default values
  * Test resolve_verifier_command requires explicit for non-Rust stacks
  * Test resolve_verifier_coverage_min validation
  * Test resolve_verifier_coverage_warn validation
  * Test coverage_warn_message with values above and below threshold
  * Test validate_coverage_min boundary conditions
- **Dependencies** COV-007
- [ ] COV-008 Add tests for verifier PR and review gate functions

---

### Task COV-009

- **ID** COV-009
- **Context Bundle** src/server.rs:54-74, src/server.rs:258-282, src/server.rs:125-143
- **DoD** Tests cover bearer token authentication, CORS header handling, and server configuration validation.
- **Checklist**
  * Test check_auth returns None when no token configured
  * Test check_auth returns unauthorized when token missing
  * Test check_auth returns unauthorized when token invalid
  * Test check_auth returns None when token matches
  * Test ServerConfig validate requires token for non-localhost
  * Test ServerConfig addr parsing with valid and invalid addresses
- **Dependencies** None
- [ ] COV-009 Add tests for server authentication and CORS

---

### Task COV-010

- **ID** COV-010
- **Context Bundle** src/server.rs:145-256, src/server.rs:284-end
- **DoD** Tests cover HTTP handler responses, session enrichment with task counts, and error response formatting.
- **Checklist**
  * Test root_handler returns ok status
  * Test status_handler returns sessions list
  * Test status_name_handler returns 404 for missing session
  * Test stop_handler updates session status
  * Test fallback_handler returns 404 for unknown endpoints
  * Test enrich_session adds remaining task count
  * Test json_response and error_response formatting
- **Dependencies** COV-009
- [ ] COV-010 Add tests for server handlers and session enrichment

---

### Task COV-011

- **ID** COV-011
- **Context Bundle** src/backend/claude.rs:49-119, src/backend/claude.rs:121-178
- **DoD** Tests cover run_iteration error handling, parse_text with malformed JSON, and text extraction from various response structures.
- **Checklist**
  * Test run_iteration returns error for empty prompt
  * Test parse_text with empty file
  * Test parse_text with non-JSON content
  * Test extract_assistant_texts with non-assistant messages
  * Test extract_assistant_texts with missing content array
  * Test extract_result_text with non-result messages
- **Dependencies** None
- [ ] COV-011 Add tests for backend claude.rs edge cases

---

### Task COV-012

- **ID** COV-012
- **Context Bundle** Cargo.toml, src/lib.rs
- **DoD** Run cargo tarpaulin to verify 90% coverage. Identify any remaining gaps and add targeted tests to close them.
- **Checklist**
  * Run cargo tarpaulin --workspace and verify 90% threshold
  * Identify any remaining uncovered lines in priority modules
  * Add tests for remaining gaps if coverage below 90%
  * Ensure all tests pass with cargo test --workspace
  * Document any intentionally uncovered code paths
- **Dependencies** COV-001, COV-002, COV-003, COV-004, COV-005, COV-006, COV-007, COV-008, COV-009, COV-010, COV-011
- [ ] COV-012 Final coverage verification and gap closure
