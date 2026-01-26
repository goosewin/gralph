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
