# PRD Specification

This document defines the validation rules for PRD (Project Requirements Document) files used by gralph. These rules are enforced by `prd_validate_contents` in `src/prd.rs` and should be followed when generating PRDs.

---

## Document-Level Rules

### 1. Non-Empty Content

The PRD file must not be empty or contain only whitespace.

**Error:** `Task file is empty`

### 2. Forbidden Sections

The following sections are not allowed in validated PRDs:

- **Open Questions** (`## Open Questions`)

This section indicates unresolved decisions and must be addressed before the PRD can be validated.

**Error:** `Open Questions section is not allowed`

### 3. Stray Checkboxes

Unchecked task lines (`- [ ]`) appearing outside of task blocks are invalid. All unchecked task items must be contained within a properly formatted task block.

**Error:** `Unchecked task line outside task block` (includes line number)

---

## Task Block Rules

Each task must be defined in a task block starting with a `### Task <ID>` header.

### Task Block Structure

A valid task block:
1. Starts with `### Task <ID>` (exactly three `#`, one space, "Task", one space, then the ID)
2. Ends at a `---` separator or an H2 heading (`## <Title>`)
3. Contains all required fields
4. Contains exactly one unchecked task line

### Task Header Format

```
### Task <ID>
```

- Must start with `### Task ` (three hashes, space, "Task", space)
- Leading whitespace (spaces or tabs) before `###` is allowed
- The ID follows after the space (e.g., `PRD-1`, `EX-1`)
- Tabs within the header sequence are rejected (e.g., `###\tTask` is invalid)
- Double spaces are rejected (e.g., `###  Task` is invalid)

**Valid examples:**
- `### Task PRD-1`
- `  ### Task COV-13`

**Invalid examples:**
- `## Task PRD-1` (H2 instead of H3)
- `#### Task PRD-1` (H4 instead of H3)
- `###Task PRD-1` (missing space after `###`)
- `### Tasks PRD-1` (plural "Tasks")
- `### Task` (missing ID)

### Task Block Termination

A task block ends when one of the following is encountered:
- A horizontal rule: `---`
- An H2 heading: `## <Title>` (must have space after `##` and non-empty title)

---

## Required Task Block Fields

Every task block must contain these five fields:

### 1. ID

```
- **ID** <task-id>
```

The unique identifier for the task, matching the task header.

### 2. Context Bundle

```
- **Context Bundle** `<path>`, `<path>`, ...
```

A list of file paths (in backticks) that provide context for the task.

**Validation rules:**
- Must include at least one file path
- Relative paths are resolved from the PRD file's directory or the specified base directory
- Absolute paths must be inside the repository root
- All listed files must exist (unless `allow_missing_context` is enabled)

**Errors:**
- `Context Bundle must include at least one file path`
- `Context Bundle path not found: <path>`
- `Context Bundle path outside repo: <path>`

### 3. DoD (Definition of Done)

```
- **DoD** <description>
```

A clear statement of what constitutes completion for this task.

### 4. Checklist

```
- **Checklist**
  * <item>
  * <item>
```

A list of verification items for the task.

### 5. Dependencies

```
- **Dependencies** <list or "None">
```

Other task IDs that must be completed before this task, or "None" if no dependencies.

**Error for missing fields:** `Missing required field: <field-name>`

---

## Unchecked Task Line Rules

Each task block must contain exactly one unchecked task line.

### Format

```
- [ ] <ID> <description>
```

- Must start with `- [ ]` (dash, space, open bracket, space, close bracket)
- Leading whitespace (spaces or tabs) is allowed
- Typically includes the task ID and a short description

**Valid examples:**
- `- [ ] PRD-1 Create specification document`
- `   - [ ] Edge case task`

**Invalid examples:**
- `-[ ] Task` (missing space after dash)
- `- [] Task` (missing space in brackets)
- `- [x] Task` (checked, not unchecked)

### Count Validation

- **Zero unchecked lines:** `Missing unchecked task line`
- **Multiple unchecked lines:** `Multiple unchecked task lines (<count>)`

---

## Complete Task Block Example

```markdown
### Task PRD-1

- **ID** PRD-1
- **Context Bundle** `PRD.template.md`, `src/prd.rs`
- **DoD** A new file `docs/PRD_SPEC.md` exists containing all validation rules.
- **Checklist**
  * All rules from `prd_validate_contents` documented.
  * Rules for task block fields documented.
  * Rules for forbidden sections documented.
  * Rules for stray checkboxes documented.
- **Dependencies** None
- [ ] PRD-1 Create PRD specification document
---
```

---

## Summary of Validation Errors

| Rule | Error Message |
|------|---------------|
| Empty file | `Task file is empty` |
| Open Questions section | `Open Questions section is not allowed` |
| Stray unchecked line | `Unchecked task line outside task block` |
| Missing required field | `Missing required field: <field>` |
| No unchecked line in block | `Missing unchecked task line` |
| Multiple unchecked lines | `Multiple unchecked task lines (<count>)` |
| Empty context bundle | `Context Bundle must include at least one file path` |
| Missing context file | `Context Bundle path not found: <path>` |
| Context outside repo | `Context Bundle path outside repo: <path>` |

---

## Notes for LLM Generation

When generating PRDs:

1. **Never include an Open Questions section** - resolve all questions before generation
2. **Place all `- [ ]` lines inside task blocks** - never outside
3. **Include all five required fields** in every task block
4. **Use exactly one unchecked line per task block**
5. **Ensure context bundle paths exist** and are relative to the PRD location
6. **Terminate each task block** with `---` before the next section
7. **Use the exact field format** with `- **FieldName**` syntax
