# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Verification Notes
When adding entries under [Unreleased], include a single-line verification note:
Verification: Tests=<command|not-run>; Coverage=<percent> (>= 90%);
CI=<status/link>; PR=<link if final PRD task>

## [Unreleased]

### Added
- PROMPT-1 Require lower-case conventional commits in the default prompt template.
- VER-1 Add verifier command for tests and coverage gates.
- DOC-1 Document verifier workflow, review gate, and commit conventions.
- COV-1 Expand core loop error-path and prompt rendering coverage.
- COV-2 Expand PRD validation and sanitization coverage.
- COV-3 Add OpenCode backend run_iteration argument and env coverage.
- COV-3 Expand state store edge-case coverage.
- COV-4 Expand verifier parsing and gate evaluation tests.
- COV-5 Add Codex backend run_iteration flag validation tests.
- COV-5 Expand config normalization and override tests.
- COV-6 Expand CLI helper coverage in main.
- COV-7 Expand server auth and CORS error-path coverage.
- COV-8 Expand notification formatting and HTTP error coverage.
- COV-9 Expand backend module utility coverage.
- COV-9 Add notify validation and failure formatting tests.
- COV-10 Add property-based tests for task parsing invariants.
- COV-10 Expand Claude backend parsing and install tests.
- COV-11 Expand backend module error formatting coverage.
- COV-12 Expand Claude adapter error-path coverage.
- COV-13 Expand OpenCode adapter error-path coverage.
- COV-14 Expand Gemini adapter error-path coverage.
- COV-15 Expand Codex adapter error-path coverage.
- COV-15 Add verifier command parsing and review gate tests.
- COV-16 Add verifier static check and duplicate detection tests.
- COV-17 Expand Claude backend parsing and failure path coverage.
- COV-18 Expand Codex backend installation and error coverage.
- COV-19 Expand Gemini backend command and error coverage.
- COV-20 Expand OpenCode backend env and failure coverage.
- COV-21 Expand backend module utility coverage.
- COV-22 Expand config loader and override coverage.
- COV-23 Expand core loop validation and completion coverage.
- COV-24 Expand main CLI helper coverage.
- COV-25 Expand notification formatting and HTTP error coverage.
- COV-26 Expand PRD sanitization and stack summary coverage.
- COV-27 Expand server auth, CORS, and session enrichment coverage.
- COV-28 Expand state store normalization coverage.
- COV-29 Expand task parsing edge coverage.
- COV-30 Expand update parsing and extraction coverage.
- COV-9 Expand update check and archive error coverage.
- COV-31 Expand verifier parsing and static check coverage.
- COV-3 Expand verifier parsing and gate evaluation coverage.
- COV-5 Expand server session enrichment and stop flow coverage.

### Fixed
- WT-1 Skip auto worktree creation on dirty repos and emit explicit skip reasons.
- REF-1 Consolidate shared backend execution helpers.
- REF-2 Unify task block parsing helpers across core and PRD validation.
- REF-3 Centralize config merge precedence and normalize override lookup.
- REF-4 Centralize server auth and error responses.
- REF-5 Reduce duplication in notification payload formatting.
- REF-6 Modularize verifier pipeline helpers into a dedicated module.
- REF-7 Update shared docs and module map for refactor outcomes.
- COV-5 Align verifier coverage command with the 90 percent gate.
- COV-6 Normalize absolute context path comparisons and isolate config env override tests.

### Verification
- Verification: Tests=cargo test --workspace; Coverage=43.50% via cargo tarpaulin --workspace --fail-under 90 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/* (>= 90%); CI=not-run; PR=not-opened

## [0.2.2]

### Fixed
- Installer: add PATH auto-update for local installs.
- Windows installer: fix Join-Path usage when piping to iex.

### Verification
- Verification: Tests=not-run; Coverage=not-run (>= 90%); CI=not-run; PR=not-opened

## [0.2.1]

### Added
- AW-2 Added auto worktree edge case tests for skip behavior, subdir mapping, and collisions.
- AW-3 Documented auto worktree UX, skip reasons, and Graphite stacking guidance.
- WT-1 Auto-create worktrees for PRD runs with config and CLI controls.
- INIT-1 Added init CLI subcommand and routing.
- UPD-1 Added session-start update check with version parsing.
- UPD-2 Added update subcommand to install release binaries.
- DOC-1 Documented update command, update notice, and regenerated completions.

### Fixed
- START-1 Added session name fallback for dot and root paths.
- INST-1 Hardened installer cleanup and PATH-aware verification.
- LOG-1 Format loop start/finish timestamps and human-readable durations.

### Verification
- Verification: Tests=CI; Coverage=CI (>= 90%); CI=green; PR=not-required

## [0.1.0]

### Added
- INIT-4 Documented init command and updated shell completions.
- Added multi-arch release assets for Linux and macOS.
- Initial public release notes for the gralph CLI.
- T-SERVER-1 Added status endpoint auth and response tests.
- T-SERVER-2 Added stop endpoint behavior tests.
- T-SERVER-3 Added CORS and error response tests.
- T-NOTIFY-2 Added send_webhook HTTP delivery tests for headers and response handling.
- T-NOTIFY-1 Added webhook payload formatting tests for Discord, Slack, and generic webhooks.
- T-BACKEND-3 Added backend registry tests for selection and model listing.
- T-BACKEND-2 Added run-iteration success/failure tests for backend adapters.
- T-BACKEND-1 Added reusable fake CLI helper for backend adapter tests.
- T-STATE-1 Added state error-path tests for corrupted/missing files.
- T-CORE-2 Added core loop execution tests for completion, failure, and max-iteration paths.
- T-CORE-1 Added core task parsing edge-case tests.
- RS-1 Scaffolded Rust project with clap CLI skeleton.
- RS-2 Added Rust config loader with serde_yaml merging and env overrides.
- RS-3 Added Rust state module with file locking and atomic JSON writes.
- RS-4 Added backend trait with Claude CLI implementation and tests.
- RS-5 Added OpenCode, Gemini, and Codex Rust backends with integration test stubs.
- RS-6 Added Rust core loop module with iteration execution and completion checks.
- RS-7 Added Rust webhook notifications with Discord, Slack, and generic payloads via reqwest.
- RS-8 Added Rust PRD validation, sanitization, and stack detection utilities with tests.
- RS-9 Added Rust HTTP status server with bearer auth and CORS handling.
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
- RS-10 Wired Rust CLI subcommands with build-time shell completions.
- RS-11 Added Rust tests coverage and CI workflow with coverage threshold.
- RS-12 Documented Rust build/install steps and migration notes.
- RS-13 Updated release workflow to package Rust binaries and completions.
- T-CLI-1 Added CLI unit tests for PRD output resolution and parse validation.
- T-CLI-2 Added tests for PRD template selection and fallback behavior.
- T-CONFIG-1 Added config path precedence tests.

### Fixed
- AW-1 Resolve auto worktree repo roots from target dirs and preserve subdir runs.
- Aligned the Cargo-installed binary name with release assets (`gralph`).
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
- OSS-19 Removed legacy shell artifacts and aligned context defaults with Rust sources.
