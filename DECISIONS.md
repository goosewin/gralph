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

## D-004 Worktree Command Design
- Date: 2026-01-25
- Status: Accepted

### Context
Worktree automation needs a small, predictable interface that enforces naming
conventions and keeps operations safe on dirty repos.

### Decision
Use `gralph worktree create <ID>` and `gralph worktree finish <ID>` commands
that create `task-<ID>` branches with `.worktrees/task-<ID>` paths and refuse to
run when `git status` is dirty.

### Rationale
- Keeps a minimal two-command surface aligned with the protocol steps.
- Encodes task ID naming and worktree location to reduce manual mistakes.
- Fails fast on dirty state to prevent partial merges or orphaned worktrees.

### Alternatives
- Expose low-level `add`/`remove` wrappers without task ID conventions.
- Allow custom worktree paths and branch names per invocation.
- Attempt auto-stash or auto-clean instead of refusing dirty state.

## D-005 Strict PRD Validation Gate
- Date: 2026-01-25
- Status: Accepted

### Context
PRD task blocks can be malformed, which leads to incomplete or incorrect loops.
We need a guardrail that blocks execution when validation fails, while preserving
the existing behavior for users who are not ready to enforce strict checks.

### Decision
Add a `--strict-prd` flag to `gralph start` that validates PRDs and aborts on
schema errors. Keep the default behavior unchanged when strict mode is not set.

### Rationale
- Prevents running loops on invalid or ambiguous task definitions.
- Makes validation explicit and opt-in to avoid breaking existing workflows.
- Surfaces actionable error output before work begins.

### Alternatives
- Always enforce strict validation and break existing flows.
- Emit warnings only and allow the loop to proceed.
- Require manual pre-checks without CLI support.

## D-006 Stage E Self-Hosting Examples and Runner
- Date: 2026-01-26
- Status: Accepted

### Context
Stage E needs a canonical, minimal example set plus a repeatable way to run it
so the self-hosting flow can be validated end-to-end.

### Decision
Add example PRDs for Stage P and Stage A, plus a release runner script that
executes them in order with a safe, re-runnable command line.

### Rationale
- Provides a concrete schema reference for new users and reviewers.
- Ensures the self-hosting flow can be executed consistently in CI or locally.
- Keeps the workflow minimal while still proving the end-to-end path.

### Alternatives
- Rely on ad-hoc internal PRDs without published examples.
- Document the steps but skip the scripted runner.
- Bundle a single large example instead of small stage-specific PRDs.

## D-007 Interactive PRD Generator
- Date: 2026-01-26
- Status: Accepted

### Context
Creating spec-compliant PRDs manually is slow and error-prone. We want a
first-class way to capture user intent, inspect the repo if needed, and
generate a valid PRD that can immediately drive a gralph loop.

### Decision
Add a `gralph prd create` command that prompts for goals and constraints,
uses the configured backend to generate a PRD from the template, validates
the result, and prints the exact loop command to run next.

### Rationale
- Reduces friction and errors when starting a new build.
- Produces consistent task blocks that meet the schema.
- Encourages context-first execution by surfacing key docs.

### Alternatives
- Maintain only a static PRD template and manual editing.
- Build an external generator script outside the CLI.
- Skip validation and rely on runtime failures.

## D-008 Go Port Architecture Alignment
- Date: 2026-01-26
- Status: Accepted

### Context
The Go implementation needs a clear mapping from bash modules to Go packages so
contributors can navigate the codebase and reason about dependencies.

### Decision
Align Go package structure with existing module boundaries and document the
package dependency flow in ARCHITECTURE.

### Rationale
- Preserves functional parity by mirroring known module responsibilities.
- Keeps dependency direction explicit for the core loop and CLI commands.
- Reduces onboarding time when moving between bash and Go implementations.

### Alternatives
- Reorganize packages by technical layer (handlers/services) without mapping to
  bash modules.
- Defer documentation and let structure emerge organically.
