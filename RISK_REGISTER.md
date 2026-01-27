# Risk Register

## R-001 Context Loss Between Iterations
- Risk: Stateless runs drop intent, causing rework and inconsistent outputs.
- Impact: Medium
- Mitigation: Maintain shared docs (PROCESS, ARCHITECTURE, DECISIONS, CHANGELOG,
  RISK_REGISTER) and keep the Rust module map current.

## R-002 Process Drift From Protocol
- Risk: Tasks bypass required steps, leading to incomplete or invalid outputs.
- Impact: Medium
- Mitigation: Enforce the Worktree Protocol checklist and add a changelog entry
  for every task ID.

## R-003 Legacy Shell Drift
- Risk: Shell-era references linger or reappear, causing confusion and broken automation.
- Impact: Low
- Mitigation: Keep workflows, docs, and packaging aligned with the Rust-only CLI.
