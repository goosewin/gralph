# Process

## Worktree Protocol
1) Read: ARCHITECTURE.md, PROCESS.md, DECISIONS.md, CHANGELOG.md, RISK_REGISTER.md.
2) Create worktree: `.worktrees/task-<ID>`.
3) Implement ONLY the assigned task.
4) Update:
   - CHANGELOG.md (include Task ID)
   - DECISIONS.md (if decision made)
   - RISK_REGISTER.md (if new risk found)
   - ARCHITECTURE.md (delta update)
5) Run checklist + verification (go test ./..., gofmt if Go files touched).
6) Merge worktree back and remove it.

## Guardrails
- Any task lacking Context Bundle or DoD is invalid.
- If conflicts occur, record in CHANGELOG.
- New risks must be added to RISK_REGISTER.md with mitigation.
