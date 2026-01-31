# Gralph CLI

Autonomous AI coding loops. Spawns fresh AI sessions iteratively until all tasks in a PRD are complete.

## Quick Start

```bash
# Install (defaults to ~/.local/bin, no sudo needed)
curl -fsSL https://raw.githubusercontent.com/goosewin/gralph/main/install.sh | bash

# Create a task file
echo "- [ ] Build the CLI" > PRD.md

# Start the loop
gralph start .

# Check progress
gralph status
```

**Windows:**
```powershell
irm https://raw.githubusercontent.com/goosewin/gralph/main/install.ps1 | iex
```

## Requirements

- One AI backend CLI:
  - `claude` (Claude Code) - `npm install -g @anthropic-ai/claude-code`
  - `opencode` - `npm install -g opencode-ai`
  - `gemini` - `npm install -g @google/gemini-cli`
  - `codex` - `npm install -g @openai/codex`
- `tmux` for background sessions (optional with `--no-tmux`)

## Basic Commands

```bash
gralph start .                    # Start loop in current directory
gralph start . --backend opencode # Use different backend
gralph start . --no-worktree      # Skip auto worktree creation
gralph start . --dry-run          # Print next task block and resolved prompt
gralph step .                     # Run exactly one iteration
gralph verifier                   # Run verifier pipeline
gralph init .                     # Scaffold shared context files
gralph status                     # Check all running loops
gralph logs myapp --follow        # Watch logs
gralph logs myapp --raw           # Show raw backend output
gralph doctor                     # Run local diagnostics
gralph cleanup                    # Mark stale sessions (state cleanup)
gralph stop myapp                 # Stop a loop
gralph resume                     # Resume after crash
gralph update                     # Install latest release to ~/.local/bin
```

On session start, gralph performs a best-effort update check and prints a notice
if a newer release is available (it never blocks startup). Disable it with
`defaults.check_updates: false` or `GRALPH_NO_UPDATE_CHECK=1`.

By default, `gralph start` creates a git worktree under `.worktrees/` for each PRD run
when the target directory is inside a git repo with at least one commit and the
repo is clean. Subdirectory runs are preserved, so `gralph start path/to/subdir`
continues the loop from the matching subdirectory inside the worktree.

Auto worktree creation is skipped when the target directory is not inside a git
repo, the repo has no commits, or the repo is dirty. In those cases the loop runs
in the target directory. Disable auto worktrees with `--no-worktree` or set
`defaults.auto_worktree: false`.

When stacking with Graphite, run `gt` inside the worktree created for the task
so the stack is attached to the correct checkout and branch.

## Logs

Session logs are written to `.gralph/<session>.log` under the project directory.
Raw backend output is saved to `.gralph/<session>.raw.log` and can be viewed with
`gralph logs <name> --raw`.
When running in the background (no `--no-tmux`), follow logs with
`gralph logs <name> --follow` or `tail -f .gralph/<session>.log`.

## Dry-run and Step

`gralph start --dry-run` prints the next task block and the resolved prompt template
without running a backend or creating tmux sessions.

`gralph step` runs exactly one iteration using the same prompt rendering and strict
PRD validation behavior as the loop. It does not auto-run the verifier.

## How It Works

1. Reads task file (`PRD.md` by default) - [see example PRDs](examples/)
2. Finds unchecked tasks (`- [ ]` lines)
3. Spawns AI to complete one task
4. Repeats until all tasks done or max iterations hit

## Doctor

`gralph doctor` runs local checks for backend CLIs, gh install/auth, git clean
state, config readability, and state store access. It prints per-check status,
actionable hints, and exits non-zero when required items are missing.

Common failure hints:
- Missing backend CLI: install the CLI listed under Requirements or use `gralph backends`.
- Missing gh or auth: install https://cli.github.com and run `gh auth login`.
- Dirty git repo: `git status`, then commit or stash changes.
- Config parse error: fix YAML in `~/.config/gralph/config.yaml` or project `.gralph.yaml`.
- State store error: check `GRALPH_STATE_DIR` and permissions for `~/.config/gralph`.

## Cleanup

`gralph cleanup` marks stale sessions by default (running sessions with dead PIDs)
using the state store. Use `--remove` to delete only stale sessions from state.
Use `--purge` to delete all sessions from state (explicit opt-in). Output lists
affected sessions when available, otherwise it prints a count.

## Verifier Pipeline

`gralph verifier` runs tests, coverage, and static checks, creates a PR via `gh`,
waits for review criteria (greptile by default), and merges only when reviews and
checks meet thresholds. When `verifier.auto_run` is true, it runs automatically
after loop completion; otherwise run `gralph verifier` manually. Configure the
review gate under `verifier.review.*` and ensure `gh auth login` is complete.
Auto-merge requires explicit approval by default. To allow auto-merge without an
approval requirement, set `verifier.review.require_approval: false` in config.
Verifier defaults are stack-aware: Rust/Cargo keeps the default cargo commands and
auto-run behavior, while non-Rust or unknown stacks default `verifier.auto_run`
to false and require explicit `verifier.test_command` and
`verifier.coverage_command` (or CLI flags).
Soft coverage warning target is controlled by `verifier.coverage_warn` (default 80).
It is warning-only, does not change `verifier.coverage_min`, and never blocks
merges. The target was staged from 65 to 70 percent during ramp-up, then raised
to 80 percent after coverage stayed stable for at least two consecutive cycles.

## Commit Conventions

Use lower-case conventional commits for all loop work and verifier-generated
commits (for example: `feat: add verifier pipeline`, `fix: handle dirty repo`).

## Context Files (Shared Memory)

Gralph agents are stateless - each iteration starts fresh with no memory of previous runs. To prevent context loss and rework, maintain these files in your project root:

Use `gralph init --dir .` to scaffold the shared context files when missing. Pass `--force` to overwrite existing files.

| File | Purpose | Example |
|------|---------|---------|
| `ARCHITECTURE.md` | Module map, runtime flow, storage locations. Agents read this to understand where code lives and how components connect. | [see example](ARCHITECTURE.md) |
| `PROCESS.md` | Step-by-step protocol agents must follow. Defines guardrails like "update CHANGELOG after each task" or "reject tasks without Context Bundle." | [see example](PROCESS.md) |
| `DECISIONS.md` | Architectural decisions with context, rationale, and rejected alternatives. Prevents agents from revisiting settled debates. | [see example](DECISIONS.md) |
| `RISK_REGISTER.md` | Known risks (e.g., "context loss between iterations") with mitigations. Agents add new risks they discover. | [see example](RISK_REGISTER.md) |
| `CHANGELOG.md` | Record of what each agent accomplished, tagged by task ID. Next agent sees what was done and builds on it. | [see example](CHANGELOG.md) |

For PRD task files, see [example PRDs](examples/).

Gralph injects these into every prompt, so each agent:
- Knows the codebase structure without exploring
- Follows established conventions and protocols
- Understands why past decisions were made
- Sees what previous agents completed
- Adds to the shared memory for future agents

**This is how stateless agents maintain continuity** - the context lives in the repo, not in memory.

## Configuration

Create `~/.config/gralph/config.yaml`:

```yaml
defaults:
  backend: claude
  max_iterations: 30
  task_file: PRD.md

notifications:
  webhook: https://discord.com/api/webhooks/...
```

Or use environment variables: `GRALPH_DEFAULTS_BACKEND=opencode`

## Documentation

- [Example PRDs](examples/) - See what good task files look like
- [PRD Format](docs/prd-format.md) - Task file structure
- [CLI Reference](docs/cli.md) - Full command documentation  
- [Configuration Reference](docs/configuration.md) - All config options
- [Backends](docs/backends.md) - Backend setup and models
- [Notifications](docs/notifications.md) - Webhook setup
- [Troubleshooting](docs/troubleshooting.md) - Common issues

## License

MIT © Dan Goosewin
