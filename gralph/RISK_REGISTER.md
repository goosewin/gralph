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
