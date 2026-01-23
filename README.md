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

## Configuration Options Reference

This section documents all available configuration options in detail.

### Section: `defaults`

Default values for loop behavior.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `defaults.max_iterations` | integer | `30` | Maximum number of loop iterations before giving up. Prevents infinite loops. |
| `defaults.task_file` | string | `PRD.md` | Path to the task file relative to project directory. |
| `defaults.completion_marker` | string | `COMPLETE` | The text used in `<promise>MARKER</promise>` to signal completion. |
| `defaults.model` | string | `claude-sonnet-4-20250514` | Claude model to use for each iteration. |

### Section: `claude`

Settings passed to the Claude CLI.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `claude.flags` | array | `["--dangerously-skip-permissions"]` | CLI flags passed to `claude` command. |
| `claude.env` | object | `{ IS_SANDBOX: "1" }` | Environment variables set when running Claude. |

**Example:**

```yaml
claude:
  flags:
    - --dangerously-skip-permissions
    - --verbose
  env:
    IS_SANDBOX: "1"
    CUSTOM_VAR: "value"
```

### Section: `notifications`

Notification settings for loop events.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `notifications.on_complete` | boolean | `true` | Send notification when loop completes successfully. |
| `notifications.webhook` | string | (none) | Webhook URL for notifications (Discord, Slack, or generic JSON POST). |

**Webhook formats supported:**
- **Discord**: URLs containing `discord.com/api/webhooks/`
- **Slack**: URLs containing `hooks.slack.com/services/`
- **Generic**: Any other URL receives a JSON POST

**Example:**

```yaml
notifications:
  on_complete: true
  webhook: https://discord.com/api/webhooks/123456/abcdef
```

### Section: `logging`

Log file management settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `logging.level` | string | `info` | Log verbosity level (`debug`, `info`, `warn`, `error`). |
| `logging.retain_days` | integer | `7` | Number of days to keep old log files before cleanup. |

### Environment Variable Overrides

All configuration keys can be overridden using environment variables. The conversion rule is:

1. Take the full dotted key (e.g., `defaults.max_iterations`)
2. Prefix with `RLOOP_`
3. Convert to uppercase
4. Replace dots with underscores

**Examples:**

| Config Key | Environment Variable |
|------------|---------------------|
| `defaults.max_iterations` | `RLOOP_DEFAULTS_MAX_ITERATIONS` |
| `defaults.task_file` | `RLOOP_DEFAULTS_TASK_FILE` |
| `defaults.completion_marker` | `RLOOP_DEFAULTS_COMPLETION_MARKER` |
| `notifications.webhook` | `RLOOP_NOTIFICATIONS_WEBHOOK` |
| `logging.level` | `RLOOP_LOGGING_LEVEL` |

**Usage:**

```bash
# Override max iterations for a single run
RLOOP_DEFAULTS_MAX_ITERATIONS=100 rloop start .

# Export for all runs in this shell
export RLOOP_NOTIFICATIONS_WEBHOOK="https://hooks.slack.com/services/xxx"
rloop start .
```

### Configuration Precedence

Configuration values are loaded in the following order (later sources override earlier):

1. **Default config** (`~/.config/rloop/config/default.yaml` or bundled default)
2. **Global config** (`~/.config/rloop/config.yaml`)
3. **Project config** (`.rloop.yaml` in project directory)
4. **Environment variables** (`RLOOP_*`)
5. **CLI arguments** (e.g., `--max-iterations`)

### Complete Configuration Example

```yaml
# ~/.config/rloop/config.yaml - Global configuration

defaults:
  max_iterations: 50          # Allow more iterations
  task_file: PRD.md           # Default task file
  completion_marker: COMPLETE # Completion signal
  model: claude-sonnet-4-20250514

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

notifications:
  on_complete: true
  webhook: https://discord.com/api/webhooks/123/abc

logging:
  level: info
  retain_days: 7
```

```yaml
# ~/projects/myapp/.rloop.yaml - Project-specific overrides

defaults:
  max_iterations: 100         # This project needs more iterations
  task_file: TASKS.md         # Use different task file
  model: claude-opus-4-20250514  # Use Opus for this complex project

notifications:
  webhook: https://hooks.slack.com/services/T00/B00/xxx  # Different webhook
```

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
  resume [name]       Resume crashed/stopped loops
  config              Manage configuration
  server              Start status API server
  version             Show version
  help                Show help message
```

### Command: `start`

Start a new ralph loop in a specified directory.

```bash
rloop start <directory> [options]
```

**Arguments:**
- `<directory>` - Path to the project directory (required)

**Options:**
| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--name` | `-n` | Session name for tracking | Directory basename |
| `--max-iterations` | | Maximum loop iterations | 30 |
| `--task-file` | `-f` | Path to task file relative to project | PRD.md |
| `--completion-marker` | | Completion promise text | COMPLETE |
| `--model` | `-m` | Claude model to use | (from config) |
| `--webhook` | | Notification webhook URL | (none) |
| `--no-tmux` | | Run in foreground (blocking) | false |

**Examples:**
```bash
# Start with defaults
rloop start .

# Start with custom name and iterations
rloop start ~/projects/myapp --name myapp --max-iterations 50

# Use different task file
rloop start . --task-file TODO.md

# Run in foreground (for debugging)
rloop start . --no-tmux
```

### Command: `stop`

Stop a running ralph loop.

```bash
rloop stop <name>
rloop stop --all
```

**Arguments:**
- `<name>` - Session name to stop (required unless using --all)

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--all` | `-a` | Stop all running sessions |

**Examples:**
```bash
# Stop specific session
rloop stop myapp

# Stop all sessions
rloop stop --all
```

### Command: `status`

Show status of all ralph loop sessions.

```bash
rloop status
```

**Output columns:**
- **NAME** - Session name
- **DIR** - Project directory path
- **ITERATION** - Current iteration / max iterations
- **STATUS** - running (yellow), complete (green), failed/stopped (red)
- **REMAINING** - Number of unchecked tasks

**Example output:**
```
NAME          DIR                      ITERATION  STATUS     REMAINING
--------      -----------------------  ---------- ---------- ----------
api-server    ~/projects/backend       12/50      running    8 tasks
web-ui        ~/projects/frontend      30/30      complete   0 tasks
mobile-app    ~/projects/mobile        15/40      failed     5 tasks
```

### Command: `logs`

View logs for a ralph loop session.

```bash
rloop logs <name> [options]
```

**Arguments:**
- `<name>` - Session name (required)

**Options:**
| Option | Description |
|--------|-------------|
| `--follow` | Continuously stream new log entries (like `tail -f`) |

**Examples:**
```bash
# View last 100 log lines
rloop logs myapp

# Follow logs in real-time
rloop logs myapp --follow
```

### Command: `resume`

Resume crashed or stopped loops. Finds sessions that were marked as running but whose processes are no longer alive, and restarts them.

```bash
rloop resume [name]
```

**Arguments:**
- `[name]` - Optional specific session to resume. If omitted, resumes all resumable sessions.

**Resumable states:**
- Sessions marked "running" with dead PIDs
- Sessions marked "stale"
- Sessions marked "stopped"

**Examples:**
```bash
# Resume all crashed sessions
rloop resume

# Resume specific session
rloop resume myapp
```

### Command: `server`

Start an HTTP status server for remote monitoring.

```bash
rloop server [options]
```

**Options:**
| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--port` | `-p` | Port number to listen on | 8080 |
| `--token` | `-t` | Bearer token for authentication | (none, open access) |

**API Endpoints:**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/status` | List all sessions |
| GET | `/status/:name` | Get specific session |
| POST | `/stop/:name` | Stop a session |

**Examples:**
```bash
# Start server on default port
rloop server

# Start with authentication
rloop server --port 8080 --token "my-secret-token"

# Query from remote
curl -H "Authorization: Bearer my-secret-token" http://server:8080/status
```

### Command: `config`

Manage rloop configuration (planned feature).

```bash
rloop config get <key>
rloop config set <key> <value>
```

### Command: `version`

Show rloop version.

```bash
rloop version
```

**Aliases:** `--version`, `-v`

### Command: `help`

Show help message with usage information.

```bash
rloop help
```

**Aliases:** `--help`, `-h`

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
