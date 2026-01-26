# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed
- Updated documentation and workflows to reflect the .NET 10 CLI and AOT distribution model.

### Removed
- Removed the legacy bash CLI implementation, installers, and shell-based tests.

### Added
- AOT-1 Added .NET 10 solution and bootstrap console app with Native AOT publish profile.
- AOT-2 Added .NET configuration loader with YAML parsing, merging, and env overrides.
- AOT-3 Added .NET state persistence with JSON storage, file locking, and stale cleanup.
- AOT-4 Added backend abstraction layer and Claude backend with JSON stream parsing.
- AOT-5 Added OpenCode, Gemini, and Codex backends.
- AOT-6 Added .NET PRD task block parsing and validation helpers.
- AOT-7 Added .NET core execution loop with prompt rendering, iteration control, and completion checks.
- AOT-8 Implemented .NET CLI entry point with command routing and option parsing.
- AOT-9 Implemented start command with foreground/background session management.
- AOT-10 Implemented stop, status, logs, and resume commands.
- AOT-11 Implemented prd check/create commands with stack detection and sanitization.
- AOT-12 Implemented worktree create/finish commands with git safety checks.
- AOT-13 Implemented HTTP status server with bearer authentication.
- AOT-14 Implemented webhook notifications with Slack/Discord/generic payloads for completion and failure events.
- AOT-15 Implemented config list/get/set commands and backend status output.
- AOT-16 Added Native AOT publish/verify scripts and documented build metrics.
- AOT-17 Added .NET unit tests for config, state, and PRD validation modules.
- AOT-18 Added integration tests for backend parsing, core loop completion, and CLI argument handling.
- Initial public release notes for the gralph CLI.
- G-1 Added interactive PRD generator via `gralph prd create`.
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
- E-1 Added example README for self-hosting PRDs.
- E-2 Added Stage P example PRD.
- E-3 Added Stage A example PRD.
- E-4 Added release runner script for example stages.
- E-5 Documented self-hosting workflow in README.
- E-6 Recorded Stage E example and runner rationale in DECISIONS.
- P-EX-2 Updated README task block example to include all required fields.

### Fixed
- OSS-1 Removed duplicate introductory text from README.
- OSS-2 Corrected Gemini CLI install hints and README instructions.
- OSS-3 Updated backend model names to real or placeholder values.
- OSS-4 Updated shell completions to match backend model names.
- OSS-5 Verified Codex CLI docs URL and install reference.
- OSS-6 Verified OpenCode CLI docs URL and install reference.
- OSS-7 Verified Claude Code model alias claude-opus-4-5.
- OSS-8 Removed stale platform notes reference in README.
- OSS-9 Removed unused notification helpers and variables.
- OSS-10 Hardened server request handling and tightened CORS defaults.
- OSS-11 Updated backend tests to assert corrected model names.
- OSS-12 Hardened installer path handling and bootstrap checks.
- OSS-13 Aligned CLI backend validation and help text.
- OSS-14 Hardened state locking, atomic writes, and corruption recovery.
- OSS-15 Added simple YAML array parsing and documented supported config lists.
- OSS-16 Hardened core loop completion detection and parse error handling.
- OSS-17 Documented PRD validation rules, sanitization behavior, and stack detection heuristics.
- OSS-18 Reviewed README for CLI reference, model names, and doc accuracy.
