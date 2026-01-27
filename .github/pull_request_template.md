# Summary
-

# Verification
- Tests: `cargo test --workspace`
- Coverage: `cargo tarpaulin --workspace --fail-under 60 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`
- Coverage result: __ (must be >= 90%)
- CI: __ (link or status)

# Checklist
- [ ] CHANGELOG entry includes a Verification note
- [ ] New task branch/worktree used (required for final PRD task)
- [ ] Final PRD task: do not merge without CI green and coverage >= 90%
