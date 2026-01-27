# Process

## Worktree Protocol
1) Read: ARCHITECTURE.md, PROCESS.md, DECISIONS.md, CHANGELOG.md, RISK_REGISTER.md.
2) Create worktree: `.worktrees/task-<ID>` (new `task-<ID>` branch).
3) Implement ONLY the assigned task in the Rust codebase (`src/`) or scoped docs.
4) Update:
   - CHANGELOG.md (include Task ID)
   - DECISIONS.md (if decision made)
   - RISK_REGISTER.md (if new risk found)
   - ARCHITECTURE.md (delta update)
5) Run checklist + verification:
   - `cargo test --workspace`
   - `cargo tarpaulin --workspace --fail-under 60 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`
     (confirm coverage >= 90%)
   - CI/CD preflight matches `.github/workflows/ci.yml`
   - Worktree is clean
6) Merge worktree back and remove it.

## Last PRD Todo Gate
- Ensure you are on a new task branch/worktree; never finish the last PRD task
  on main.
- Run `cargo test --workspace` and confirm coverage >= 90% with:
  `cargo tarpaulin --workspace --fail-under 60 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`
- Ensure CI/CD will pass (run the same checks as `.github/workflows/ci.yml` or
  confirm a green CI run).
- Open a PR before merging the final task.

## Guardrails
- Any task lacking Context Bundle or DoD is invalid.
- If conflicts occur, record in CHANGELOG.
- New risks must be added to RISK_REGISTER.md with mitigation.
- Rust CLI is the source of truth; do not reintroduce shell scripts.
- Test coverage must remain >= 90%.
- Final PRD task requires a PR and green CI before merge.
