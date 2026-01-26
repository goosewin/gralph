# Risk Register

## R-001 Context Loss Between Iterations
- Risk: Stateless runs drop intent, causing rework and inconsistent outputs.
- Impact: Medium
- Mitigation: Maintain shared docs (PROCESS, ARCHITECTURE, DECISIONS, CHANGELOG,
  RISK_REGISTER) and keep them updated each iteration.

## R-002 Process Drift From Protocol
- Risk: Tasks bypass required steps, leading to incomplete or invalid outputs.
- Impact: Medium
- Mitigation: Enforce the Worktree Protocol checklist and add a changelog entry
  for every task ID.

## R-003 Backend CLI Compatibility Drift
- Risk: External backend CLIs change flags or output formats, breaking loop runs.
- Impact: Medium
- Mitigation: Document supported versions, add integration coverage in CI, and
  validate backend execution with real CLIs before release tags.

## R-004 Release Artifact Packaging Gaps
- Risk: Release archives omit config defaults or completions, causing confusing
  behavior after install.
- Impact: Low
- Mitigation: Bundle `config/default.yaml` and completion files in release
  archives and keep README install steps aligned with releases.
