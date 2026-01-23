# Ralph Loop CLI (`rloop`)

Autonomous AI coding loops using Claude Code. Spawns fresh Claude Code sessions iteratively until all tasks in a PRD are complete.

Named after the "Ralph Wiggum" technique - persistent iteration despite setbacks.

## Features

- **Robust completion detection** - Only detects genuine completion, not mentions of the completion promise
- **Multi-project support** - Run multiple concurrent loops
- **State persistence** - Resume after crash/reboot
- **Remote monitoring** - Status server for checking progress remotely
- **Notifications** - Webhooks for Discord/Slack on completion

## Requirements

- `claude` CLI (Claude Code)
- `jq` for JSON parsing
- `tmux` for session management
- `bash` 4.0+
- `curl` (optional, for notifications)

## Installation

### Quick Install (curl)

```bash
curl -fsSL https://raw.githubusercontent.com/USER/ralph-cli/main/install.sh | bash
```

### From Source

```bash
git clone git@github.com:USER/ralph-cli.git
cd ralph-cli
./install.sh
```

### Manual Installation

1. Clone this repository
2. Copy `bin/rloop` to a directory in your PATH (e.g., `~/.local/bin/` or `/usr/local/bin/`)
3. Copy `lib/` to `~/.config/rloop/lib/`
4. Create config directory: `mkdir -p ~/.config/rloop`

## Usage

### Start a Loop

```bash
# Basic usage - start loop in current directory
rloop start .

# Start with options
rloop start ~/projects/myapp \
  --name myapp \
  --max-iterations 50 \
  --task-file PRD.md \
  --completion-marker "COMPLETE"
```

### Check Status

```bash
# List all running loops
rloop status

# Output:
# NAME          DIR                      ITERATION  STATUS     REMAINING
# app1          ~/projects/app1          5/30       running    12 tasks
# app2          ~/projects/app2          3/30       running    8 tasks
# app3          ~/projects/app3          15/30      complete   0 tasks
```

### View Logs

```bash
# View logs for a specific loop
rloop logs myapp

# Follow logs in real-time
rloop logs myapp --follow
```

### Stop a Loop

```bash
# Stop specific loop
rloop stop myapp

# Stop all loops
rloop stop --all
```

### Resume After Crash

```bash
# Resume all previously running sessions
rloop resume

# Resume specific session
rloop resume myapp
```

## Configuration

### Global Configuration

Location: `~/.config/rloop/config.yaml`

```yaml
defaults:
  max_iterations: 30
  task_file: PRD.md
  completion_marker: COMPLETE
  model: claude-sonnet-4-20250514

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

notifications:
  on_complete: true
  webhook: https://hooks.example.com/notify

logging:
  level: info
  retain_days: 7
```

### Project Configuration

Create `.rloop.yaml` in your project directory to override global settings.

### Environment Variables

- `RLOOP_MAX_ITERATIONS` - Override max iterations
- `RLOOP_TASK_FILE` - Override task file path
- `RLOOP_COMPLETION_MARKER` - Override completion marker

## CLI Reference

```
rloop - Autonomous AI coding loops

USAGE:
  rloop <command> [options]

COMMANDS:
  start <dir>         Start a new ralph loop
  stop <name>         Stop a running loop
  stop --all          Stop all loops
  status              Show status of all loops
  logs <name>         View logs for a loop
  resume              Resume crashed/stopped loops
  config              Manage configuration
  server              Start status API server
  version             Show version

START OPTIONS:
  --name, -n          Session name (default: directory name)
  --max-iterations    Max iterations before giving up (default: 30)
  --task-file, -f     Task file path (default: PRD.md)
  --completion-marker Completion promise text (default: COMPLETE)
  --model, -m         Claude model override
  --webhook           Notification webhook URL
  --no-tmux           Run in foreground (blocks)
```

## How It Works

1. Reads the task file (PRD.md by default)
2. Counts unchecked tasks (`- [ ]` pattern)
3. If tasks remain, spawns Claude Code with the task prompt
4. Waits for Claude to complete one task and exit
5. Re-counts unchecked tasks
6. Repeats until all tasks complete or max iterations reached

### Completion Detection

The loop only terminates when:
1. Zero `- [ ]` patterns remain in the task file, AND
2. The completion promise appears as the final output (not just mentioned mid-text)

This prevents premature termination when Claude mentions the promise without actually completing.

## Usage Examples

### Example 1: Basic Single Project Loop

Start a loop on a project with a PRD file:

```bash
# Navigate to your project
cd ~/projects/my-webapp

# Start the loop (uses PRD.md by default)
rloop start .

# Check progress
rloop status

# View live logs
rloop logs my-webapp --follow
```

### Example 2: Multiple Concurrent Projects

Run several projects simultaneously on a VPS:

```bash
# Start multiple projects with custom names
rloop start ~/projects/backend --name api-server --max-iterations 50
rloop start ~/projects/frontend --name web-ui --max-iterations 30
rloop start ~/projects/mobile --name mobile-app --max-iterations 40

# Check all at once
rloop status

# Output:
# NAME          DIR                      ITERATION  STATUS     REMAINING
# api-server    ~/projects/backend       12/50      running    8 tasks
# web-ui        ~/projects/frontend      5/30       running    15 tasks
# mobile-app    ~/projects/mobile        3/40       running    22 tasks
```

### Example 3: Custom Task File and Completion Marker

Use a different task file or completion marker:

```bash
# Use TODO.md instead of PRD.md
rloop start . --task-file TODO.md

# Use a custom completion marker
rloop start . --completion-marker "ALL_DONE"

# Combined
rloop start ~/projects/app \
  --name myapp \
  --task-file TASKS.md \
  --completion-marker "FINISHED" \
  --max-iterations 100
```

### Example 4: Remote VPS Monitoring

Run loops on a VPS and monitor from anywhere:

```bash
# On your VPS: start the status server
rloop server --port 8080 --token "your-secret-token"

# On your VPS: start your loops
rloop start ~/projects/app1 --name app1
rloop start ~/projects/app2 --name app2

# From your laptop or phone: check status
curl -H "Authorization: Bearer your-secret-token" \
  http://your-vps-ip:8080/status

# Stop a session remotely
curl -X POST -H "Authorization: Bearer your-secret-token" \
  http://your-vps-ip:8080/stop/app1
```

### Example 5: Webhook Notifications

Get notified when loops complete:

```bash
# Discord webhook
rloop start . --webhook "https://discord.com/api/webhooks/123/abc"

# Slack webhook
rloop start . --webhook "https://hooks.slack.com/services/T00/B00/xxx"

# Or set globally
rloop config set notifications.webhook "https://discord.com/api/webhooks/123/abc"
```

### Example 6: Recovery After Reboot

Resume loops after a server restart:

```bash
# After reboot, check what was running
rloop status

# Resume all previously running sessions
rloop resume

# Or resume specific session
rloop resume myapp
```

### Example 7: Foreground Mode (No tmux)

Run in foreground for debugging or CI/CD:

```bash
# Run without tmux (blocks until complete)
rloop start . --no-tmux

# Useful for:
# - Debugging loop behavior
# - CI/CD pipelines
# - Single-iteration testing
```

### Example 8: Model Override

Use a different Claude model:

```bash
# Use Claude Opus for more complex tasks
rloop start . --model claude-opus-4-20250514

# Or set as default in config
rloop config set defaults.model claude-opus-4-20250514
```

## License

MIT
