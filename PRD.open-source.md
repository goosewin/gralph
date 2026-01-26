# Project Requirements Document (PRD)

## Overview

gralph is a shell-based CLI tool that provides autonomous AI coding loops using multiple AI coding assistants (Claude Code, OpenCode, Gemini CLI, Codex CLI). It spawns fresh AI sessions iteratively until all tasks in a PRD file are complete. The tool is being prepared for open source release to the general public.

## Problem Statement

- The codebase must be audit-ready for public open source release.
- Documentation references models (gemini-3-pro, gpt-5.2-codex, opencode/gpt-5.2-codex) that do not exist and URLs that are invalid or fictional.
- Backend installation hints contain incorrect package names (e.g., @anthropic-ai/gemini-cli for Gemini).
- The README contains duplicate introductory text.
- Shell completion files include fictional model names that should be verified or updated.
- Test coverage may need verification for completeness.

## Solution

Perform a comprehensive audit of the codebase focusing on code quality, documentation accuracy, model references, URL validity, and test coverage. Remove redundant code, fix documentation discrepancies, and ensure all references are accurate for open source release.

---

## Functional Requirements

### FR-1: Documentation Accuracy

All documentation must accurately reflect the current state of the tool, with correct installation instructions, valid URLs, and real model names.

### FR-2: Code Quality

No redundant code, pending refactors, leftover TODOs, or commented-out code blocks that should be removed.

### FR-3: Backend Configuration Accuracy

Backend modules must have accurate installation hints, model names, and CLI flags that match actual tools.

---

## Non-Functional Requirements

### NFR-1: Shell Script Compatibility

- All scripts must remain POSIX-compatible where possible.
- Use bash 4.0+ features consistently.
- Ensure macOS and Linux compatibility.

### NFR-2: Test Coverage

- All library modules should have corresponding test coverage.
- Tests must pass on supported platforms.

---

## Implementation Tasks

### Task OSS-1

- **ID** OSS-1
- **Context Bundle** `README.md`
- **DoD** Remove duplicate introductory text in README.md.
- **Checklist**
  * Lines 3-5 contain duplicate text of lines 1-2.
  * Duplicate text removed.
- **Dependencies** None
- [x] OSS-1 Remove duplicate introductory text from README.md

---

### Task OSS-2

- **ID** OSS-2
- **Context Bundle** `lib/backends/gemini.sh`, `README.md`
- **DoD** Fix gemini backend installation hint to use correct package name.
- **Checklist**
  * backend_get_install_hint returns incorrect package @anthropic-ai/gemini-cli.
  * Update to valid Gemini CLI installation instructions.
  * Update README.md section for Gemini CLI installation.
- **Dependencies** None
- [x] OSS-2 Fix gemini backend installation hint and README documentation

---

### Task OSS-3

- **ID** OSS-3
- **Context Bundle** `lib/backends/gemini.sh`, `lib/backends/codex.sh`, `lib/backends/opencode.sh`, `config/default.yaml`
- **DoD** Replace fictional model names with real or clearly placeholder model names.
- **Checklist**
  * gemini-3-pro, gpt-5.2-codex, opencode/gpt-5.2-codex are not real model names.
  * Replace with actual model names or clearly marked placeholders.
  * Update config/default.yaml comments accordingly.
- **Dependencies** None
- [x] OSS-3 Update model names to real or clearly placeholder values

---

### Task OSS-4

- **ID** OSS-4
- **Context Bundle** `completions/gralph.bash`, `completions/gralph.zsh`
- **DoD** Update shell completions to match corrected model names.
- **Checklist**
  * Model suggestions in completions match backend implementations.
  * Remove fictional model names from completion options.
- **Dependencies** OSS-3
- [ ] OSS-4 Update shell completions with corrected model names

---

### Task OSS-5

- **ID** OSS-5
- **Context Bundle** `README.md`, `lib/backends/codex.sh`
- **DoD** Verify and fix Codex CLI documentation URLs and installation instructions.
- **Checklist**
  * URL https://developers.openai.com/codex/cli/ may not exist.
  * Update with valid documentation URL or remove if Codex CLI is fictional.
- **Dependencies** None
- [ ] OSS-5 Verify and fix Codex CLI documentation URLs

---

### Task OSS-6

- **ID** OSS-6
- **Context Bundle** `README.md`, `lib/backends/opencode.sh`
- **DoD** Verify OpenCode CLI documentation URL and update if needed.
- **Checklist**
  * URL https://opencode.ai/docs/cli/ must be verified.
  * Update installation instructions if URL is invalid.
- **Dependencies** None
- [ ] OSS-6 Verify OpenCode CLI documentation URL and update

---

### Task OSS-7

- **ID** OSS-7
- **Context Bundle** `lib/backends/claude.sh`
- **DoD** Verify Claude Code model name claude-opus-4-5 is accurate.
- **Checklist**
  * Confirm model name matches Anthropic Claude Code CLI expectations.
  * Update if model name format is incorrect.
- **Dependencies** None
- [ ] OSS-7 Verify Claude Code model name accuracy

---

### Task OSS-8

- **ID** OSS-8
- **Context Bundle** `README.md`
- **DoD** Remove references to non-existent files and update file paths.
- **Checklist**
  * Reference to tests/macos-compatibility.md on line 43 - file does not exist.
  * Reference to uninstall.sh in uninstall section - verify file exists.
  * Update or remove invalid references.
- **Dependencies** None
- [ ] OSS-8 Fix invalid file references in README

---

### Task OSS-9

- **ID** OSS-9
- **Context Bundle** `lib/notify.sh`
- **DoD** Audit notification module for completeness and remove any dead code.
- **Checklist**
  * All functions are used and documented.
  * No dead code paths.
  * Error handling is consistent.
- **Dependencies** None
- [ ] OSS-9 Audit notification module for dead code

---

### Task OSS-10

- **ID** OSS-10
- **Context Bundle** `lib/server.sh`
- **DoD** Audit HTTP server module for security and completeness.
- **Checklist**
  * CORS headers are appropriate for the use case.
  * No security vulnerabilities in request handling.
  * All endpoints documented.
- **Dependencies** None
- [ ] OSS-10 Audit HTTP server module for security

---

### Task OSS-11

- **ID** OSS-11
- **Context Bundle** `tests/backend-codex-test.sh`, `tests/backend-gemini-test.sh`
- **DoD** Verify test files work correctly and update assertions for corrected model names.
- **Checklist**
  * Tests pass with updated model names.
  * Test coverage is adequate.
- **Dependencies** OSS-3
- [ ] OSS-11 Update backend tests for corrected model names

---

### Task OSS-12

- **ID** OSS-12
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Audit installer for correctness and platform compatibility.
- **Checklist**
  * All installation paths are correct.
  * macOS and Linux installation works.
  * Error handling is robust.
- **Dependencies** None
- [ ] OSS-12 Audit installer for correctness

---

### Task OSS-13

- **ID** OSS-13
- **Context Bundle** `bin/gralph`
- **DoD** Audit main CLI entry point for code quality.
- **Checklist**
  * No dead code paths.
  * Help text is accurate and complete.
  * Error messages are clear.
- **Dependencies** None
- [ ] OSS-13 Audit main CLI entry point

---

### Task OSS-14

- **ID** OSS-14
- **Context Bundle** `lib/state.sh`
- **DoD** Audit state management module for race conditions and edge cases.
- **Checklist**
  * Locking mechanism works correctly.
  * State file corruption is handled.
  * All functions are documented.
- **Dependencies** None
- [ ] OSS-14 Audit state management module

---

### Task OSS-15

- **ID** OSS-15
- **Context Bundle** `lib/config.sh`
- **DoD** Audit configuration module for YAML parsing edge cases.
- **Checklist**
  * YAML parser handles documented feature set.
  * Environment variable overrides work correctly.
  * Configuration precedence is documented and implemented.
- **Dependencies** None
- [ ] OSS-15 Audit configuration module

---

### Task OSS-16

- **ID** OSS-16
- **Context Bundle** `lib/core.sh`
- **DoD** Audit core loop logic for robustness.
- **Checklist**
  * Completion detection is reliable.
  * Task block parsing handles edge cases.
  * Error handling is comprehensive.
- **Dependencies** None
- [ ] OSS-16 Audit core loop logic

---

### Task OSS-17

- **ID** OSS-17
- **Context Bundle** `lib/prd.sh`
- **DoD** Audit PRD validation module for completeness.
- **Checklist**
  * All validation rules are documented.
  * Stack detection is accurate.
  * Sanitization handles edge cases.
- **Dependencies** None
- [ ] OSS-17 Audit PRD validation module

---

### Task OSS-18

- **ID** OSS-18
- **Context Bundle** `README.md`
- **DoD** Final documentation review for consistency and accuracy.
- **Checklist**
  * All code examples work.
  * CLI reference matches implementation.
  * No broken links.
  * Troubleshooting section is complete.
- **Dependencies** OSS-1, OSS-2, OSS-3, OSS-5, OSS-6, OSS-7, OSS-8
- [ ] OSS-18 Final documentation review

---

## Success Criteria

- All tasks completed with no remaining unchecked items.
- All tests pass on macOS and Linux.
- Documentation accurately reflects implementation.
- No fictional URLs, model names, or file references remain.
- Code is ready for public open source release.

---

## Sources

- http://www.w3.org/TR/html4/loose.dtd

---

## Warnings

- The provided source URL (http://www.w3.org/TR/html4/loose.dtd) is not relevant to this project and appears to be a placeholder.
- Model names (gemini-3-pro, gpt-5.2-codex, opencode/gpt-5.2-codex) and documentation URLs should be verified against actual product documentation before finalizing changes.
- OpenCode, Gemini CLI, and Codex CLI backends may reference fictional or unreleased products that need verification.
