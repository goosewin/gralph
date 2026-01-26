# PRD Stage P Example - Task Block Parsing

## Overview
Demonstrate the task block schema with a minimal PRD focused on parsing and prompt wiring.

## Problem Statement
The project needs a compact example that shows how task blocks are structured in the new format.

## Solution
Provide a small PRD with two atomic tasks that exercise parsing and documentation updates.

---

## Functional Requirements

### FR-1: Task block selection
Select the next unchecked task block from a PRD file for prompt injection.

### FR-2: Documentation guidance
Expose the task block format in documentation to reduce confusion.

---

## Non-Functional Requirements

### NFR-1: Clarity
- The example must be short and easy to skim.

### NFR-2: Compatibility
- The format must align with the template schema.

---

## Implementation Tasks

### Task P-EX-1

- **ID** P-EX-1
- **Context Bundle** `gralph/lib/core.sh`, `gralph/lib/config.sh`
- **DoD** Select the next unchecked task block or fall back to legacy single-line tasks.
- **Checklist**
  * Selector returns the first unchecked task block.
  * Legacy single-line task parsing still works.
- **Dependencies** None
- [x] P-EX-1 Select next task block

### Task P-EX-2

- **ID** P-EX-2
- **Context Bundle** `gralph/README.md`, `gralph/PRD.template.md`
- **DoD** Document the task block format in README and align the template.
- **Checklist**
  * README includes a task block example.
  * Template reflects the documented format.
- **Dependencies** P-EX-1
- [ ] P-EX-2 Document task block format

---

## Success Criteria
- The PRD validates with exactly one unchecked task line per task block.
- The example remains minimal and focused.

---

## Open Questions
None.
