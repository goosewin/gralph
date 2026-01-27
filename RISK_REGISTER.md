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

## R-004 Coverage Regression Below 90%
- Risk: Coverage drops below the 90% bar, weakening confidence and causing CI failures.
- Impact: High
- Mitigation: Enforce coverage checks locally and in CI; block merges below 90%.

## R-005 Final PRD Task Skips PR/CI Gate
- Risk: The last PRD task is merged without a PR or green CI, increasing the
  likelihood of undetected defects.
- Impact: High
- Mitigation: Require a new task branch/worktree, run tests with coverage >= 90%,
  and open a PR before merge.
