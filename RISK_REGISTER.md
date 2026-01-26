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

## R-003 Stale Background Sessions
- Risk: Background child processes can exit unexpectedly or leave stale PIDs,
  causing status/resume to report incorrect state.
- Impact: Medium
- Mitigation: PID checks in state cleanup, resume command to restart stale
  sessions, and `--no-tmux` foreground runs for debugging.

## R-004 Backend Output Drift
- Risk: Upstream backend CLIs change output formats, breaking JSON parsing or
  completion detection.
- Impact: High
- Mitigation: Keep raw output logs, add backend adapter tests, and update
  parsers when CLI formats change.
