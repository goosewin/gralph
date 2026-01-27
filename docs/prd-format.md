# PRD Format

## Basic Format

Simple task list with checkbox items:

```markdown
- [ ] Build the CLI
- [ ] Add tests
- [ ] Write docs
```

## Task Block Format

For complex projects, use structured task blocks:

```markdown
### Task P-1

- **ID** P-1
- **Context Bundle** `src/core.rs`
- **DoD** Add task block parsing
- **Checklist**
  * Parser extracts task blocks from PRD
  * Fallback to single-line tasks when no blocks exist
- **Dependencies** None
- [ ] P-1 Implement parser
```

Task blocks end at the next `### Task` header, `---`, or `##` section.

## Validation

```bash
# Check PRD for errors
gralph prd check PRD.md

# Start with strict validation
gralph start . --strict-prd
```

**Validation rules:**
- Every task block needs: ID, Context Bundle, DoD, Checklist, Dependencies
- Each block has exactly one unchecked `- [ ]` line
- Context Bundle paths must exist in repo

## Generate PRD

```bash
gralph prd create --goal "Add billing dashboard" --output PRD.md
```

Options:
- `--constraints` - Non-functional requirements
- `--context` - Context files (comma-separated)
- `--sources` - External URLs
- `--no-interactive` - Skip prompts

## Completion Detection

Loop terminates when:
1. Zero unchecked tasks remain
2. Completion promise appears in output

This prevents false positives when AI mentions completion without actually finishing.
