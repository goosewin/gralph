# PRD Stage A - Shared Docs Baseline Example

## Overview

Add the shared memory documents used by gralph to preserve context between iterations.

## Problem Statement

- There is no canonical set of shared docs for a new repo.
- Iterations lose context without a persisted process and decision record.

## Solution

Create minimal shared documents that capture process, architecture, decisions, risks, and change history.

---

## Functional Requirements

### FR-1: Shared Docs Baseline

Provide the baseline shared documents with minimal structure and content.

---

## Non-Functional Requirements

### NFR-1: Format

- Documents are stored as ASCII Markdown.

---

## Implementation Tasks

### Task A-1

- **ID** A-1
- **Context Bundle** `PROCESS.md`, `README.md`
- **DoD** Add a process document that defines the work protocol and guardrails.
- **Checklist**
  * Defines the worktree protocol steps.
  * Lists guardrails for task execution.
- **Dependencies** None
- [ ] A-1 Add PROCESS document

### Task A-2

- **ID** A-2
- **Context Bundle** `ARCHITECTURE.md`
- **DoD** Add an architecture document with module and flow summaries.
- **Checklist**
  * Includes a modules section.
  * Includes a runtime flow section.
- **Dependencies** A-1
- [ ] A-2 Add ARCHITECTURE document

### Task A-3

- **ID** A-3
- **Context Bundle** `DECISIONS.md`
- **DoD** Record the initial decision to keep shared docs in-repo.
- **Checklist**
  * Includes decision context and rationale.
  * Notes alternatives considered.
- **Dependencies** A-1
- [ ] A-3 Add DECISIONS document

### Task A-4

- **ID** A-4
- **Context Bundle** `RISK_REGISTER.md`
- **DoD** Add a risk register that captures context-loss risks.
- **Checklist**
  * Adds at least one risk entry.
  * Includes mitigation guidance.
- **Dependencies** A-1
- [ ] A-4 Add RISK_REGISTER document

### Task A-5

- **ID** A-5
- **Context Bundle** `CHANGELOG.md`
- **DoD** Add a changelog file following Keep a Changelog structure.
- **Checklist**
  * Includes an Unreleased section.
  * Includes a link to the standard.
- **Dependencies** A-1
- [ ] A-5 Add CHANGELOG document

---

## Success Criteria

- Shared documents exist with minimal required content.
- The baseline supports tracking process, architecture, decisions, risks, and changes.

---
