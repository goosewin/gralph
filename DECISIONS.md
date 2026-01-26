# Decisions

## D-001 Shared Memory Documents
- Date: 2026-01-25
- Status: Accepted

### Context
Stateless iterations lose context unless key artifacts are stored in the repo.
We need a small set of shared documents to preserve intent, process, and risks.

### Decision
Add shared memory documents to the repository (PROCESS, ARCHITECTURE, DECISIONS,
RISK_REGISTER, and CHANGELOG) and keep them in ASCII.

### Rationale
- Keeps critical context close to the code and versioned with changes.
- Reduces drift between protocol expectations and actual implementation.
- Makes handoff and review deterministic across iterations.

### Alternatives
- Store documents externally (wiki or drive) with a link in the repo.
- Encode context in code comments only.
- Use a single monolithic context file instead of separate documents.

## D-002 Inject Task Blocks into Prompts
- Date: 2026-01-25
- Status: Accepted

### Context
Task execution needs more than a single checkbox line. Without surrounding
context, iterations can miss scope, assumptions, and dependencies captured in
the task block.

### Decision
Inject the full task block into the iteration prompt when available. When task
blocks are absent, fall back to the single unchecked line to keep legacy PRDs
working.

### Rationale
- Ensures the agent sees full task intent and constraints.
- Preserves compatibility with existing single-line task formats.
- Reduces mis-scoped work caused by missing local context.

### Alternatives
- Only include the unchecked line and rely on the agent to look up context.
- Require task blocks and break legacy PRDs.
- Attach the entire PRD instead of a scoped task block.

## D-003 Inject Context Files into Prompts
- Date: 2026-01-25
- Status: Accepted

### Context
Even with shared docs present, agents can skip reading them without explicit
instructions. We need a configurable list of shared context files that is
visible in each iteration prompt.

### Decision
Add a configurable context file list and inject it into the prompt with a
clear instruction to read those files first. Keep the list optional so existing
behavior remains when unset.

### Rationale
- Ensures shared docs are surfaced at the start of each iteration.
- Keeps context file selection configurable without code changes.
- Preserves backward compatibility when no list is provided.

### Alternatives
- Rely on the agent to discover shared docs without explicit instructions.
- Hardcode a fixed list of context files in the prompt.
- Inject the entire repository contents into the prompt.
