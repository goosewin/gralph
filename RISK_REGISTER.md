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

## R-003 AOT Artifact + Config Drift
- Risk: The AOT binary can be moved without the bundled config folder, leading to
  missing defaults or confusing config resolution.
- Impact: Medium
- Mitigation: Ship `config/default.yaml` alongside release assets and document the
  `GRALPH_DEFAULT_CONFIG` override.

## R-004 Backend CLI Output Drift
- Risk: External backend CLIs can change output formats or flags, breaking parsing
  and completion detection.
- Impact: Medium
- Mitigation: Track backend versions in tests, surface parse errors in logs, and
  update adapters promptly when upstream changes.
