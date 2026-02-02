# Project Requirements Document: Harden Loop Completion for Auto Worktrees

## Overview

Harden gralph loop completion so auto worktrees end clean by adding pre-PR verification steps, task PRD deletion from the final commit, cargo fmt and test enforcement, reliable PR creation via gh CLI, and PRD content posting as a PR comment. This ensures automated loop runs in worktrees conclude in a reviewable, mergeable state without manual intervention.

## Problem Statement

- Auto worktree loops may complete with unstaged changes left in the working tree, leading to dirty state before PR creation.
- The task PRD file remains in the final commit, cluttering the target branch with ephemeral task definitions.
- `cargo fmt` is not enforced before PR creation, risking inconsistent formatting in merged code.
- Tests may not run reliably before PR creation when the verifier pipeline is invoked.
- PR creation via `gh` can fail silently or produce unreliable results without actionable error messages.
- Reviewers lack context because the PRD content is not posted as a PR comment for reference.

## Solution

Extend the verifier pipeline in `src/verifier.rs` and loop completion logic in `src/app/loop_session.rs` to:

1. Check git status for unstaged/uncommitted changes before PR creation and fail with an actionable error when dirty.
2. Resolve the task PRD path from CLI args or config defaults (not assuming `PRD.md` only) and delete it before the final commit.
3. Run `cargo fmt --check` (or `cargo fmt` with commit) before tests to enforce formatting for Rust stacks.
4. Ensure tests run via the existing verifier test command before PR creation.
5. Improve `gh pr create` invocation to reliably return the PR URL or fail with a clear error.
6. Post the resolved PRD content as a PR comment using `gh pr comment` for review context.
7. Add config options for enabling/disabling PRD deletion and PR comment posting.

---

## Functional Requirements

### FR-1: Clean Git Status Enforcement

Before PR creation, the verifier must check `git status --porcelain` in the worktree and abort with an actionable error message if any unstaged or uncommitted changes exist. The error must list the dirty paths and suggest commit or stash actions.

### FR-2: Task PRD Deletion from Final Commit

Resolve the task PRD path from `--task-file` CLI arg, `defaults.task_file` config, or the default `PRD.md`. Before the final commit (prior to PR creation), delete the PRD file from the worktree and stage the deletion. If the file does not exist, skip silently.

### FR-3: Cargo Fmt Enforcement

For Rust stacks (detected via existing `is_rust_stack` helper), run `cargo fmt --check` before tests. If it fails, run `cargo fmt` and commit the formatting changes with a conventional commit message. Non-Rust stacks skip this step.

### FR-4: Reliable PR Creation

Improve `run_gh_pr_create` to capture stdout/stderr, parse the PR URL from output, and return an actionable error when `gh pr create` fails. The PR URL must be reliably extracted or the function must fail explicitly.

### FR-5: PRD Content as PR Comment

After successful PR creation, read the resolved PRD file content and post it as a PR comment using `gh pr comment <url> --body <content>`. Truncate or summarize if the content exceeds GitHub comment limits. Failures are warnings, not fatal errors.

### FR-6: Config Options for New Behaviors

Add optional config keys under `verifier.*`:
- `verifier.delete_prd_on_complete` (default: true) - delete task PRD from final commit.
- `verifier.post_prd_comment` (default: true) - post PRD content as PR comment.
- `verifier.fmt_command` (default: `cargo fmt --check` for Rust) - formatting command.

---

## Non-Functional Requirements

### NFR-1: ASCII and Non-Interactive Output

All outputs must be ASCII-only and non-interactive. No prompts, spinners, or ANSI escape codes in verifier pipeline output.

### NFR-2: Backward Compatibility

Preserve existing behavior for non-Rust stacks by requiring explicit commands. Do not break existing verifier invocations or config formats.

### NFR-3: Actionable Error Messages

All failure conditions must emit clear, actionable error messages that describe what failed and suggest remediation steps.

---

## Implementation Tasks

### Task CLEAN-1

- **ID** CLEAN-1
- **Context Bundle** `src/verifier.rs`, `src/app/worktree.rs`
- **DoD** Verifier checks `git status --porcelain` before PR creation and fails with dirty file paths listed when not clean.
- **Checklist**
  * Add `ensure_git_clean_for_pr` helper in verifier.rs that calls git status and returns CliError with file list.
  * Call helper at the start of `run_verifier_pr_create`.
  * Error message includes "Commit or stash changes before PR creation" hint.
  * Add unit test for dirty detection with mock git output.
- **Dependencies** None
- [x] CLEAN-1 Add pre-PR git clean status check with actionable error
### Task PRD-DEL-1

- **ID** PRD-DEL-1
- **Context Bundle** `src/verifier.rs`, `src/config.rs`, `src/cli.rs`
- **DoD** Verifier resolves task PRD path and deletes it from worktree before final commit when enabled.
- **Checklist**
  * Add `verifier.delete_prd_on_complete` config key with default true.
  * Add `resolve_task_prd_path` helper that checks CLI/config/default for task file path.
  * Delete PRD file and stage deletion with `git rm` if file exists.
  * Skip silently if file does not exist or config is false.
  * Add test for PRD deletion flow.
- **Dependencies** CLEAN-1
- [x] PRD-DEL-1 Delete task PRD from final commit before PR creation
### Task FMT-1

- **ID** FMT-1
- **Context Bundle** `src/verifier.rs`, `src/prd.rs`, `config/default.yaml`
- **DoD** Verifier runs cargo fmt check for Rust stacks and auto-commits formatting fixes if needed.
- **Checklist**
  * Add `verifier.fmt_command` config key with default `cargo fmt --check` for Rust.
  * Add `run_verifier_fmt_check` helper that runs fmt command and detects failure.
  * On failure, run `cargo fmt` without `--check`, commit with `style: format code`, and continue.
  * Skip for non-Rust stacks.
  * Add test for fmt check and auto-fix flow.
- **Dependencies** PRD-DEL-1
- [x] FMT-1 Add cargo fmt enforcement with auto-commit for Rust stacks
### Task PRURL-1

- **ID** PRURL-1
- **Context Bundle** `src/verifier.rs`
- **DoD** PR creation reliably returns PR URL or fails with actionable error including gh stderr.
- **Checklist**
  * Modify `run_gh_pr_create` to capture both stdout and stderr.
  * Parse PR URL from stdout using existing `extract_pr_url` helper.
  * If URL missing and exit code non-zero, return error with stderr content.
  * If URL missing but exit code zero, return error stating URL not found.
  * Add test for various gh output scenarios.
- **Dependencies** FMT-1
- [ ] PRURL-1 Improve PR creation to reliably return URL or actionable error
### Task PRCOMMENT-1

- **ID** PRCOMMENT-1
- **Context Bundle** `src/verifier.rs`, `src/config.rs`
- **DoD** Verifier posts PRD content as PR comment after successful PR creation.
- **Checklist**
  * Add `verifier.post_prd_comment` config key with default true.
  * Add `post_prd_as_pr_comment` helper that reads PRD content and runs `gh pr comment`.
  * Truncate content to 65000 chars with truncation notice if needed.
  * Log warning but do not fail if comment posting fails.
  * Add test for comment posting flow.
- **Dependencies** PRURL-1
- [ ] PRCOMMENT-1 Post PRD content as PR comment for review context
### Task CONFIG-1

- **ID** CONFIG-1
- **Context Bundle** `config/default.yaml`, `src/verifier.rs`, `README.md`
- **DoD** New config keys documented in default.yaml and README verifier section.
- **Checklist**
  * Add `delete_prd_on_complete`, `post_prd_comment`, and `fmt_command` to default.yaml with comments.
  * Update README verifier pipeline section with new config options.
  * Verify config loading works with new keys.
- **Dependencies** PRCOMMENT-1
- [ ] CONFIG-1 Document new verifier config options in default.yaml and README
### Task TEST-1

- **ID** TEST-1
- **Context Bundle** `src/verifier.rs`, `.github/workflows/ci.yml`
- **DoD** Unit tests cover all new verifier helpers with mocked git and gh commands.
- **Checklist**
  * Add tests for `ensure_git_clean_for_pr` with clean and dirty scenarios.
  * Add tests for `resolve_task_prd_path` with CLI, config, and default sources.
  * Add tests for `run_verifier_fmt_check` with pass and fail scenarios.
  * Add tests for `post_prd_as_pr_comment` with success and truncation.
  * Verify existing tests still pass.
- **Dependencies** CONFIG-1
- [ ] TEST-1 Add unit tests for all new verifier helpers
---

## Success Criteria

- Auto worktree loops complete with clean git status before PR creation.
- Task PRD file is deleted from the final commit in the PR branch.
- Rust projects have consistent formatting enforced before PR.
- PR URL is reliably returned or failure includes actionable error.
- PRD content appears as first PR comment for reviewer context.
- All new config options have defaults and documentation.
- Test coverage remains at or above 60% CI threshold.
- Existing non-Rust verifier behavior is preserved.

---

## Sources

- None provided.

---

## Warnings

- No reliable external sources were provided. Verify requirements and stack assumptions before implementation.
- Ensure `gh` CLI authentication is configured in target environments.
- Test PRD deletion and comment posting with real GitHub repositories before deployment.
