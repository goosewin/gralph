# PRD: Increase Test Coverage to 90%

## Overview

Increase the test coverage of the gralph-rs codebase to 90% by adding targeted unit tests for uncovered code paths in core modules.

## Stack

- Rust 2024 edition
- Cargo (build system)
- tempfile (test fixtures)
- proptest (property-based testing)

## Context Files

- src/lib.rs
- src/core.rs
- src/prd.rs
- src/app.rs
- src/verifier.rs
- src/cli.rs
- src/server.rs
- src/notify.rs
- src/state.rs
- src/config.rs
- src/task.rs
- src/update.rs
- src/backend/mod.rs
- src/backend/claude.rs
- src/backend/gemini.rs
- src/backend/opencode.rs
- src/backend/codex.rs
- src/app/loop_session.rs
- src/app/worktree.rs
- src/app/prd_init.rs
- src/test_support.rs

## Warnings

No reliable external sources were provided. Verify requirements and stack assumptions before implementation.

## Tasks

### Task T-1

- **ID** T-1
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for CoreError Display and Error trait implementations including all variants.
- **Checklist**
  * Test CoreError::Io display format
  * Test CoreError::Backend display format
  * Test CoreError::InvalidInput display format
  * Test Error::source for each variant
- **Dependencies** None

- [ ] T-1 Add CoreError trait implementation tests

### Task T-2

- **ID** T-2
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for LoopStatus::as_str method covering all enum variants.
- **Checklist**
  * Test LoopStatus::Running as_str
  * Test LoopStatus::Failed as_str
  * Test LoopStatus::Complete as_str
  * Test LoopStatus::MaxIterations as_str
- **Dependencies** None

- [ ] T-2 Add LoopStatus as_str tests

### Task T-3

- **ID** T-3
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for render_iteration_prompt edge cases including missing directories and invalid inputs.
- **Checklist**
  * Test empty project_dir error
  * Test zero iteration error
  * Test zero max_iterations error
  * Test non-existent project_dir error
  * Test missing task file error
- **Dependencies** None

- [ ] T-3 Add render_iteration_prompt validation tests

### Task T-4

- **ID** T-4
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for run_iteration_with_clock covering backend not installed and empty output scenarios.
- **Checklist**
  * Test backend not installed error
  * Test empty backend output error
  * Test empty parsed result error
- **Dependencies** None

- [ ] T-4 Add run_iteration_with_clock error path tests

### Task T-5

- **ID** T-5
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for count_remaining_tasks with various PRD formats.
- **Checklist**
  * Test empty file returns zero
  * Test file with task headers
  * Test file without task headers
  * Test non-existent file returns zero
- **Dependencies** None

- [ ] T-5 Add count_remaining_tasks tests

### Task T-6

- **ID** T-6
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for check_completion covering all completion scenarios.
- **Checklist**
  * Test empty task_file error
  * Test empty result returns false
  * Test remaining tasks returns false
  * Test negated promise returns false
  * Test valid completion returns true
- **Dependencies** None

- [ ] T-6 Add check_completion tests

### Task T-7

- **ID** T-7
- **Context Bundle** `src/core.rs`
- **DoD** Add tests for run_loop_with_clock input validation.
- **Checklist**
  * Test empty project_dir error
  * Test zero max_iterations error
  * Test non-existent project_dir error
  * Test missing task file error
- **Dependencies** None

- [ ] T-7 Add run_loop_with_clock validation tests

### Task T-8

- **ID** T-8
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for PrdError and PrdValidationError Display implementations.
- **Checklist**
  * Test PrdError::Io display
  * Test PrdError::Validation display
  * Test PrdValidationError display
- **Dependencies** None

- [ ] T-8 Add PRD error Display tests

### Task T-9

- **ID** T-9
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for prd_detect_stack with various project types.
- **Checklist**
  * Test Rust project detection
  * Test Node.js project detection
  * Test Python project detection
  * Test empty directory handling
- **Dependencies** None

- [ ] T-9 Add prd_detect_stack tests

### Task T-10

- **ID** T-10
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for prd_validate_file with invalid PRD formats.
- **Checklist**
  * Test missing task ID
  * Test missing Context Bundle
  * Test missing DoD
  * Test missing unchecked task line
  * Test invalid dependencies reference
- **Dependencies** None

- [ ] T-10 Add prd_validate_file error tests

### Task T-11

- **ID** T-11
- **Context Bundle** `src/prd.rs`
- **DoD** Add tests for prd_sanitize_generated_file edge cases.
- **Checklist**
  * Test removal of code fences
  * Test ASCII normalization
  * Test context file validation
- **Dependencies** None

- [ ] T-11 Add prd_sanitize_generated_file tests

### Task T-12

- **ID** T-12
- **Context Bundle** `src/app.rs`
- **DoD** Add tests for CliError Display implementations.
- **Checklist**
  * Test CliError::Message display
  * Test CliError::Io display
- **Dependencies** None

- [ ] T-12 Add CliError Display tests

### Task T-13

- **ID** T-13
- **Context Bundle** `src/app.rs`
- **DoD** Add tests for exit_code_for function mapping Result to ExitCode.
- **Checklist**
  * Test Ok result returns SUCCESS
  * Test Err result returns FAILURE
- **Dependencies** None

- [ ] T-13 Add exit_code_for tests

### Task T-14

- **ID** T-14
- **Context Bundle** `src/app.rs`
- **DoD** Add tests for session_name resolution with various inputs.
- **Checklist**
  * Test explicit name used
  * Test directory-based name fallback
  * Test sanitization of special characters
- **Dependencies** None

- [ ] T-14 Add session_name tests

### Task T-15

- **ID** T-15
- **Context Bundle** `src/app.rs`
- **DoD** Add tests for parse_bool_value utility function.
- **Checklist**
  * Test true variants
  * Test false variants
  * Test invalid input returns None
- **Dependencies** None

- [ ] T-15 Add parse_bool_value tests

### Task T-16

- **ID** T-16
- **Context Bundle** `src/app/loop_session.rs`
- **DoD** Add tests for cmd_start_dry_run output generation.
- **Checklist**
  * Test prompt rendering
  * Test task block extraction
  * Test PRD validation in dry run
- **Dependencies** None

- [ ] T-16 Add cmd_start_dry_run tests

### Task T-17

- **ID** T-17
- **Context Bundle** `src/app/loop_session.rs`
- **DoD** Add tests for cmd_stop with various session states.
- **Checklist**
  * Test stop named session
  * Test stop all sessions
  * Test stop missing session error
- **Dependencies** None

- [ ] T-17 Add cmd_stop tests

### Task T-18

- **ID** T-18
- **Context Bundle** `src/app/loop_session.rs`
- **DoD** Add tests for cmd_status output formatting.
- **Checklist**
  * Test JSON output format
  * Test table output format
  * Test verbose output
  * Test empty sessions
- **Dependencies** None

- [ ] T-18 Add cmd_status tests

### Task T-19

- **ID** T-19
- **Context Bundle** `src/app/worktree.rs`
- **DoD** Add tests for validate_task_id with various formats.
- **Checklist**
  * Test valid task ID format
  * Test invalid prefix
  * Test invalid number
  * Test extra segments
- **Dependencies** None

- [ ] T-19 Add validate_task_id tests

### Task T-20

- **ID** T-20
- **Context Bundle** `src/app/worktree.rs`
- **DoD** Add tests for auto_worktree_branch_name generation.
- **Checklist**
  * Test normal session name
  * Test empty session name
  * Test special characters sanitization
- **Dependencies** None

- [ ] T-20 Add auto_worktree_branch_name tests

### Task T-21

- **ID** T-21
- **Context Bundle** `src/app/prd_init.rs`
- **DoD** Add tests for resolve_prd_output path handling.
- **Checklist**
  * Test relative path resolution
  * Test absolute path handling
  * Test existing file without force error
  * Test existing file with force succeeds
- **Dependencies** None

- [ ] T-21 Add resolve_prd_output tests

### Task T-22

- **ID** T-22
- **Context Bundle** `src/app/prd_init.rs`
- **DoD** Add tests for invalid_prd_path generation.
- **Checklist**
  * Test md extension handling
  * Test non-md extension handling
  * Test force flag behavior
- **Dependencies** None

- [ ] T-22 Add invalid_prd_path tests

### Task T-23

- **ID** T-23
- **Context Bundle** `src/app/prd_init.rs`
- **DoD** Add tests for read_prd_template_with_manifest template loading.
- **Checklist**
  * Test project template found
  * Test manifest template found
  * Test default template fallback
- **Dependencies** None

- [ ] T-23 Add read_prd_template_with_manifest tests

### Task T-24

- **ID** T-24
- **Context Bundle** `src/verifier.rs`
- **DoD** Add tests for extract_coverage_percent parsing.
- **Checklist**
  * Test coverage results line parsing
  * Test line coverage fallback
  * Test generic coverage fallback
  * Test missing percentage returns None
- **Dependencies** None

- [ ] T-24 Add extract_coverage_percent tests

### Task T-25

- **ID** T-25
- **Context Bundle** `src/verifier.rs`
- **DoD** Add tests for validate_coverage_min boundary conditions.
- **Checklist**
  * Test zero is valid
  * Test 100 is valid
  * Test negative value error
  * Test over 100 error
- **Dependencies** None

- [ ] T-25 Add validate_coverage_min tests

### Task T-26

- **ID** T-26
- **Context Bundle** `src/verifier.rs`
- **DoD** Add tests for coverage_warn_message generation.
- **Checklist**
  * Test warning below threshold
  * Test no warning at threshold
  * Test no warning above threshold
- **Dependencies** None

- [ ] T-26 Add coverage_warn_message tests

### Task T-27

- **ID** T-27
- **Context Bundle** `src/verifier.rs`
- **DoD** Add tests for parse_verifier_command tokenization.
- **Checklist**
  * Test simple command
  * Test command with arguments
  * Test quoted arguments
  * Test empty command error
- **Dependencies** None

- [ ] T-27 Add parse_verifier_command tests

### Task T-28

- **ID** T-28
- **Context Bundle** `src/cli.rs`
- **DoD** Add tests for CLI argument parsing edge cases.
- **Checklist**
  * Test default values
  * Test flag combinations
  * Test invalid argument errors
- **Dependencies** None

- [ ] T-28 Add CLI argument parsing tests

### Task T-29

- **ID** T-29
- **Context Bundle** `src/server.rs`
- **DoD** Add tests for ServerConfig::from_env environment parsing.
- **Checklist**
  * Test default port
  * Test custom port from env
  * Test invalid port handling
- **Dependencies** None

- [ ] T-29 Add ServerConfig::from_env tests

### Task T-30

- **ID** T-30
- **Context Bundle** `src/notify.rs`
- **DoD** Add tests for notification functions with mocked notifier.
- **Checklist**
  * Test notify_failed message
  * Test notify_complete message
  * Test notification disabled
- **Dependencies** None

- [ ] T-30 Add notification function tests

### Task T-31

- **ID** T-31
- **Context Bundle** `src/state.rs`
- **DoD** Add tests for StateStore locking and concurrent access.
- **Checklist**
  * Test lock acquisition
  * Test lock timeout
  * Test concurrent read safety
- **Dependencies** None

- [ ] T-31 Add StateStore locking tests

### Task T-32

- **ID** T-32
- **Context Bundle** `src/state.rs`
- **DoD** Add tests for cleanup_stale with various cleanup modes.
- **Checklist**
  * Test CleanupMode::Mark behavior
  * Test CleanupMode::Remove behavior
  * Test stale session detection
- **Dependencies** None

- [ ] T-32 Add cleanup_stale tests

### Task T-33

- **ID** T-33
- **Context Bundle** `src/config.rs`
- **DoD** Add tests for Config::load with layered configuration.
- **Checklist**
  * Test default config loading
  * Test global config override
  * Test project config override
  * Test missing config handling
- **Dependencies** None

- [ ] T-33 Add Config::load layering tests

### Task T-34

- **ID** T-34
- **Context Bundle** `src/config.rs`
- **DoD** Add tests for config get and get_user methods.
- **Checklist**
  * Test nested key access
  * Test missing key returns None
  * Test user-only config isolation
- **Dependencies** None

- [ ] T-34 Add config accessor tests

### Task T-35

- **ID** T-35
- **Context Bundle** `src/task.rs`
- **DoD** Add tests for task_blocks_from_contents with various PRD formats.
- **Checklist**
  * Test single task block extraction
  * Test multiple task blocks
  * Test nested content handling
  * Test malformed PRD handling
- **Dependencies** None

- [ ] T-35 Add task_blocks_from_contents tests

### Task T-36

- **ID** T-36
- **Context Bundle** `src/task.rs`
- **DoD** Add tests for is_unchecked_line with various checkbox formats.
- **Checklist**
  * Test standard unchecked format
  * Test checked line returns false
  * Test non-checkbox line returns false
  * Test whitespace variations
- **Dependencies** None

- [ ] T-36 Add is_unchecked_line tests

### Task T-37

- **ID** T-37
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for backend_from_name with all supported backends.
- **Checklist**
  * Test claude backend
  * Test opencode backend
  * Test gemini backend
  * Test codex backend
  * Test unknown backend error
- **Dependencies** None

- [ ] T-37 Add backend_from_name tests

### Task T-38

- **ID** T-38
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for spawn_with_retry retry logic.
- **Checklist**
  * Test successful spawn
  * Test retry on transient failure
  * Test max retries exceeded
- **Dependencies** None

- [ ] T-38 Add spawn_with_retry tests

### Task T-39

- **ID** T-39
- **Context Bundle** `src/backend/mod.rs`
- **DoD** Add tests for stream_command_output line processing.
- **Checklist**
  * Test stdout streaming
  * Test stderr handling
  * Test callback invocation
  * Test non-zero exit code error
- **Dependencies** None

- [ ] T-39 Add stream_command_output tests

### Task T-40

- **ID** T-40
- **Context Bundle** `src/lib.rs`
- **DoD** Run cargo tarpaulin and verify 90% coverage threshold is met.
- **Checklist**
  * Run coverage report
  * Verify 90% minimum
  * Document any exclusions
- **Dependencies** T-1, T-2, T-3, T-4, T-5, T-6, T-7, T-8, T-9, T-10, T-11, T-12, T-13, T-14, T-15, T-16, T-17, T-18, T-19, T-20, T-21, T-22, T-23, T-24, T-25, T-26, T-27, T-28, T-29, T-30, T-31, T-32, T-33, T-34, T-35, T-36, T-37, T-38, T-39

- [ ] T-40 Verify 90% coverage threshold met
