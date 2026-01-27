# CLI Reference

## Commands

```
gralph start <dir>          Start a new loop
gralph stop <name>          Stop a running loop
gralph stop --all           Stop all loops
gralph status               Show all loops
gralph logs <name>          View logs
gralph resume [name]        Resume crashed loops
gralph prd check <file>     Validate PRD
gralph prd create           Generate PRD
gralph worktree create <ID> Create task worktree
gralph worktree finish <ID> Finish task worktree
gralph backends             List backends
gralph config               Manage config
gralph server               Start status server
gralph version              Show version
```

## `gralph start`

```bash
gralph start <directory> [options]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--name` | `-n` | Session name | Directory basename |
| `--max-iterations` | | Max iterations | 30 |
| `--task-file` | `-f` | Task file path | PRD.md |
| `--completion-marker` | | Completion text | COMPLETE |
| `--backend` | `-b` | AI backend | claude |
| `--model` | `-m` | Model | (from config) |
| `--webhook` | | Notification URL | (none) |
| `--no-worktree` | | Disable automatic worktree creation | false |
| `--no-tmux` | | Run in foreground | false |
| `--strict-prd` | | Validate PRD first | false |

By default, `gralph start` creates a git worktree under `.worktrees/` for each PRD run
when the directory is a git repo with at least one commit.

## `gralph stop`

```bash
gralph stop <name>
gralph stop --all
```

## `gralph status`

Shows all sessions with columns: NAME, DIR, ITERATION, STATUS, REMAINING

## `gralph logs`

```bash
gralph logs <name>
gralph logs <name> --follow
```

## `gralph resume`

```bash
gralph resume          # Resume all
gralph resume <name>   # Resume specific
```

## `gralph prd`

```bash
gralph prd check <file>
gralph prd create --goal "description" --output PRD.md
```

## `gralph server`

```bash
gralph server [options]
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--host` | `-H` | Bind address | 127.0.0.1 |
| `--port` | `-p` | Port | 8080 |
| `--token` | `-t` | Auth token | (required for non-localhost) |

API Endpoints:
- `GET /status` - List sessions
- `GET /status/:name` - Get session
- `POST /stop/:name` - Stop session

## `gralph config`

```bash
gralph config              # Show merged config
gralph config get <key>    # Get value
gralph config set <key> <value>  # Set value
```
