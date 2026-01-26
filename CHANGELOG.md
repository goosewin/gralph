# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Initial public release notes for the gralph CLI.
- C-1 Added worktree commands to help output and examples.
- C-2 Added worktree command routing and validation.
- C-3 Added worktree create command to scaffold task branches and worktrees.
- C-4 Added worktree finish command to merge task branches and remove worktrees.
- C-5 Added safety checks for dirty git state and missing worktree paths.
- C-6 Documented worktree workflow in README.
- C-7 Recorded Stage C worktree command rationale in DECISIONS.
- A-1 Added PROCESS.md with protocol steps and guardrails.
- A-2 Added ARCHITECTURE.md skeleton with required sections.
- A-3 Documented module map in ARCHITECTURE.md.
- A-4 Added runtime flow and storage details in ARCHITECTURE.md.
- A-5 Added DECISIONS.md with initial shared docs decision.
- A-6 Added RISK_REGISTER.md with context-loss risks and mitigations.
- P-1 Added task block grouping helper for PRD parsing.
- P-2 Added selector for next unchecked task block.
- P-3 Added task block placeholder to prompt rendering.
- P-4 Injected selected task block into iteration prompts.
- P-5 Added tests for task block parsing and fallback behavior.
- P-6 Documented task block format and legacy fallback behavior.
- B-1 Added defaults.context_files to default config.
- B-2 Read context_files in core loop for prompt rendering.
- B-3 Normalized context file list for prompt injection.
- B-4 Injected context files section into the prompt template.
- B-5 Documented defaults.context_files and env override in README.
- B-6 Recorded Stage B context file injection notes and decision.
- D-3 Added strict PRD validation gate for gralph start.
- D-1 Added PRD validation helpers for task block schema checks.
- D-2 Added gralph prd check command with validation errors.
- D-4 Added PRD validation shell tests for invalid cases.
- D-5 Documented PRD validation and strict mode in README.
- D-6 Recorded Stage D validation changes and strict mode rationale.
