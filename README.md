# Gralph CLI (`gralph`)

Autonomous AI coding loops using Claude Code or OpenCode. Spawns fresh AI coding sessions iteratively until all tasks in a PRD are complete.

Autonomous AI coding loops using Claude Code or OpenCode.

## Features

- **Multi-backend support** - Use Claude Code or OpenCode as your AI coding assistant
- **Robust completion detection** - Only detects genuine completion, not mentions of the completion promise
- **Multi-project support** - Run multiple concurrent loops
- **State persistence** - Resume after crash/reboot
- **Remote monitoring** - Status server for checking progress remotely
- **Notifications** - Webhooks for Discord/Slack on completion

## Requirements

- At least one AI backend:
  - `claude` CLI (Claude Code) - `npm install -g @anthropic-ai/claude-code`
  - `opencode` CLI (OpenCode) - See https://opencode.ai/docs/cli/
- `jq` for JSON parsing
- `tmux` for session management
- `bash` 4.0+
- `curl` (optional, for notifications)
- `socat` or `nc` (optional, required for `gralph server`)
- `flock` (optional, for safer concurrent state access - built-in on Linux, available via Homebrew on macOS)

### Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux | ✅ Fully supported | Primary development platform |
| macOS 12+ | ✅ Supported | Requires Homebrew dependencies |
| WSL2 | ⚠️ Best effort | Should work like Linux |

**macOS Users:** Install dependencies via Homebrew:
```bash
brew install bash jq tmux
```

See `tests/macos-compatibility.md` for detailed platform notes.

## Installation

### Quick Install (curl)

```bash
curl -fsSL https://raw.githubusercontent.com/goosewin/gralph/main/install.sh | bash
```

### From Source

```bash
git clone git@github.com:goosewin/gralph.git
cd gralph
./install.sh
```

### Manual Installation

1. Clone this repository
2. Copy `bin/gralph` to a directory in your PATH (e.g., `~/.local/bin/` or `/usr/local/bin/`)
3. Copy `lib/` to `~/.config/gralph/lib/`
4. Create config directory: `mkdir -p ~/.config/gralph`

## Backends

gralph supports multiple AI coding assistants through a pluggable backend system.

### Claude Code (Default)

[Claude Code](https://claude.ai/claude-code) is Anthropic's official CLI for Claude.

```bash
# Install Claude Code
npm install -g @anthropic-ai/claude-code

# Use Claude Code (default)
gralph start .

# Or explicitly specify
gralph start . --backend claude
```

**Models:**
- `claude-opus-4.5`

### OpenCode

[OpenCode](https://opencode.ai) is an open-source AI coding CLI that supports multiple providers.

```bash
# Install OpenCode (see https://opencode.ai/docs/cli/)

# Use OpenCode
gralph start . --backend opencode

# OpenCode models use provider/model format
gralph start . --backend opencode --model opencode/gpt-5.2-codex
gralph start . --backend opencode --model google/gemini-3-pro
```

**Models (format: provider/model):**
- `opencode/gpt-5.2-codex` (default for opencode)
- `anthropic/claude-opus-4.5`
- `google/gemini-3-pro`

### Setting Default Backend

Set the default backend in your config file:

```yaml
# ~/.config/gralph/config.yaml
defaults:
  backend: opencode
  model: opencode/gpt-5.2-codex
```

Or via environment variable:

```bash
export GRALPH_DEFAULTS_BACKEND=opencode
```

## Usage

### Quickstart

Use `PRD.template.md` as a starting point for your task file.

```bash
# Create a PRD with one task
echo "- [ ] Build the CLI" > PRD.md

# Start the loop in the current directory
gralph start .

# Watch progress
gralph status
```

### Start a Loop

```bash
# Basic usage - start loop in current directory
gralph start .

# Start with options
gralph start ~/projects/myapp \
  --name myapp \
  --max-iterations 50 \
  --task-file PRD.md \
  --completion-marker "COMPLETE"
```

### Check Status

```bash
# List all running loops
gralph status

# Output:
# NAME          DIR                      ITERATION  STATUS     REMAINING
# app1          ~/projects/app1          5/30       running    12 tasks
# app2          ~/projects/app2          3/30       running    8 tasks
# app3          ~/projects/app3          15/30      complete   0 tasks
```

### View Logs

```bash
# View logs for a specific loop
gralph logs myapp

# Follow logs in real-time
gralph logs myapp --follow
```

### Stop a Loop

```bash
# Stop specific loop
gralph stop myapp

# Stop all loops
gralph stop --all
```

### Resume After Crash

```bash
# Resume all previously running sessions
gralph resume

# Resume specific session
gralph resume myapp
```

## Configuration

### Global Configuration

Location: `~/.config/gralph/config.yaml`

```yaml
defaults:
  max_iterations: 30
  task_file: PRD.md
  completion_marker: COMPLETE
  backend: opencode
  model: opencode/gpt-5.2-codex

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

Create `.gralph.yaml` in your project directory to override global settings.

### Environment Variables

- `GRALPH_MAX_ITERATIONS` - Override max iterations
- `GRALPH_TASK_FILE` - Override task file path
- `GRALPH_COMPLETION_MARKER` - Override completion marker

## Configuration Options Reference

This section documents all available configuration options in detail.

### Section: `defaults`

Default values for loop behavior.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `defaults.max_iterations` | integer | `30` | Maximum number of loop iterations before giving up. Prevents infinite loops. |
| `defaults.task_file` | string | `PRD.md` | Path to the task file relative to project directory. |
| `defaults.completion_marker` | string | `COMPLETE` | The text used in `<promise>MARKER</promise>` to signal completion. |
| `defaults.backend` | string | `opencode` | AI backend to use: `claude` or `opencode`. |
| `defaults.model` | string | `opencode/gpt-5.2-codex` | Model to use. Format depends on backend (see Backends section). |

### Section: `claude`

Settings for the Claude Code backend.

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

### Section: `opencode`

Settings for the OpenCode backend.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `opencode.default_model` | string | `opencode/gpt-5.2-codex` | Default model in provider/model format. |

**Example:**

```yaml
opencode:
  default_model: opencode/gpt-5.2-codex
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
2. Prefix with `GRALPH_`
3. Convert to uppercase
4. Replace dots with underscores

**Examples:**

| Config Key | Environment Variable |
|------------|---------------------|
| `defaults.max_iterations` | `GRALPH_DEFAULTS_MAX_ITERATIONS` |
| `defaults.task_file` | `GRALPH_DEFAULTS_TASK_FILE` |
| `defaults.completion_marker` | `GRALPH_DEFAULTS_COMPLETION_MARKER` |
| `defaults.backend` | `GRALPH_DEFAULTS_BACKEND` |
| `notifications.webhook` | `GRALPH_NOTIFICATIONS_WEBHOOK` |
| `logging.level` | `GRALPH_LOGGING_LEVEL` |

**Usage:**

```bash
# Override max iterations for a single run
GRALPH_DEFAULTS_MAX_ITERATIONS=100 gralph start .

# Export for all runs in this shell
export GRALPH_NOTIFICATIONS_WEBHOOK="https://hooks.slack.com/services/xxx"
gralph start .
```

### Configuration Precedence

Configuration values are loaded in the following order (later sources override earlier):

1. **Default config** (`~/.config/gralph/config/default.yaml` or bundled default)
2. **Global config** (`~/.config/gralph/config.yaml`)
3. **Project config** (`.gralph.yaml` in project directory)
4. **Environment variables** (`GRALPH_*`)
5. **CLI arguments** (e.g., `--max-iterations`)

### Supported YAML Features

Gralph uses a lightweight built-in YAML parser (no external dependencies). The following YAML features are supported:

**Supported:**
- Simple key-value pairs: `key: value`
- Nested objects (up to 2 levels): `parent:\n  child: value`
- String values (with or without quotes): `name: "quoted"` or `name: unquoted`
- Numbers and booleans: `max: 30`, `enabled: true`
- Comments: `# this is a comment`
- Inline comments: `key: value  # comment`

**Not Supported:**
- Arrays/lists: `items:\n  - item1\n  - item2`
- Multi-line strings (block scalars): `description: |`
- Anchors and aliases: `&anchor`, `*alias`
- Complex nested structures (3+ levels deep)
- Flow style: `{key: value}` or `[item1, item2]`

**Example of supported config:**
```yaml
# Global settings
defaults:
  max_iterations: 50
  task_file: PRD.md
  backend: claude

notifications:
  webhook: https://hooks.example.com/notify

logging:
  level: info
```

For complex configuration needs (arrays, deep nesting), use environment variable overrides or CLI arguments instead.

## Notifications

Gralph can send webhook notifications when loops complete or fail. This is useful for monitoring long-running loops on remote servers.

### Enabling Notifications

Set a webhook URL via CLI flag, config file, or environment variable:

```bash
# Via CLI flag (per-session)
gralph start . --webhook "https://discord.com/api/webhooks/123/abc"

# Via config file (global)
gralph config set notifications.webhook "https://hooks.slack.com/services/T00/B00/xxx"

# Via environment variable
export GRALPH_NOTIFICATIONS_WEBHOOK="https://example.com/webhook"
```

### Notification Events

Gralph sends notifications for two types of events:

| Event | Trigger | Information Included |
|-------|---------|---------------------|
| **Complete** | Loop finishes with all tasks done | Session name, project, iterations, duration |
| **Failed** | Loop stops before completion | Session name, project, reason, iterations, max iterations, remaining tasks, duration |

**Failure reasons:**
- `max_iterations` - Loop hit the maximum iteration limit
- `error` - Loop encountered an error
- `manual_stop` - User stopped the loop with `gralph stop`

### Supported Webhook Platforms

Gralph auto-detects the webhook platform from the URL and formats payloads accordingly:

| Platform | URL Pattern | Format |
|----------|-------------|--------|
| Discord | `discord.com/api/webhooks/` | Discord embed with colored status |
| Slack | `hooks.slack.com/services/` | Slack block kit attachment |
| Generic | Any other URL | JSON POST (see below) |

### Webhook Payload Formats

**Discord** notifications use embeds with green (success) or red (failure) colors:

```json
{
  "embeds": [{
    "title": "✅ Gralph Complete",
    "description": "Session **myapp** has finished all tasks successfully.",
    "color": 5763719,
    "fields": [
      {"name": "Project", "value": "`/path/to/project`"},
      {"name": "Iterations", "value": "15"},
      {"name": "Duration", "value": "2h 15m 30s"}
    ]
  }]
}
```

**Slack** notifications use attachments with block kit:

```json
{
  "attachments": [{
    "color": "#57F287",
    "blocks": [
      {"type": "header", "text": {"type": "plain_text", "text": "✅ Gralph Complete"}},
      {"type": "section", "text": {"type": "mrkdwn", "text": "Session *myapp* has finished."}}
    ]
  }]
}
```

**Generic** webhooks receive a simple JSON POST:

```json
{
  "event": "complete",
  "status": "success",
  "session": "myapp",
  "project": "/path/to/project",
  "iterations": "15",
  "duration": "2h 15m 30s",
  "timestamp": "2024-01-15T10:30:00-05:00",
  "message": "Gralph loop 'myapp' completed successfully after 15 iterations (2h 15m 30s)"
}
```

For failures, the generic payload includes additional fields:

```json
{
  "event": "failed",
  "status": "failure",
  "session": "myapp",
  "project": "/path/to/project",
  "reason": "max_iterations",
  "iterations": "30",
  "max_iterations": "30",
  "remaining_tasks": "5",
  "duration": "1h 45m 20s",
  "timestamp": "2024-01-15T12:00:00-05:00",
  "message": "Gralph loop 'myapp' failed: hit max iterations (30/30) with 5 tasks remaining"
}
```

### Complete Configuration Example

```yaml
# ~/.config/gralph/config.yaml - Global configuration

defaults:
  max_iterations: 50          # Allow more iterations
  task_file: PRD.md           # Default task file
  completion_marker: COMPLETE # Completion signal
  backend: opencode           # AI backend (claude or opencode)
  model: opencode/gpt-5.2-codex

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

opencode:
  default_model: opencode/gpt-5.2-codex

notifications:
  on_complete: true
  webhook: https://discord.com/api/webhooks/123/abc

logging:
  level: info
  retain_days: 7
```

```yaml
# ~/projects/myapp/.gralph.yaml - Project-specific overrides

defaults:
  max_iterations: 100         # This project needs more iterations
  task_file: TASKS.md         # Use different task file
  backend: opencode           # Use OpenCode for this project
  model: google/gemini-3-pro  # Use Gemini 3 Pro via OpenCode

notifications:
  webhook: https://hooks.slack.com/services/T00/B00/xxx  # Different webhook
```

## CLI Reference

```
gralph - Autonomous AI coding loops

USAGE:
  gralph <command> [options]

COMMANDS:
  start <dir>         Start a new gralph loop
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

Start a new gralph loop in a specified directory.

```bash
gralph start <directory> [options]
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
| `--backend` | `-b` | AI backend to use (claude or opencode) | opencode |
| `--model` | `-m` | Model to use (format depends on backend) | (from config) |
| `--webhook` | | Notification webhook URL | (none) |
| `--no-tmux` | | Run in foreground (blocking) | false |

**Examples:**
```bash
# Start with defaults (uses OpenCode)
gralph start .

# Start with custom name and iterations
gralph start ~/projects/myapp --name myapp --max-iterations 50

# Use different task file
gralph start . --task-file TODO.md

# Use OpenCode backend
gralph start . --backend opencode

# Use OpenCode with specific model
gralph start . --backend opencode --model opencode/gpt-5.2-codex

# Run in foreground (for debugging)
gralph start . --no-tmux
```

### Command: `stop`

Stop a running gralph loop.

```bash
gralph stop <name>
gralph stop --all
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
gralph stop myapp

# Stop all sessions
gralph stop --all
```

### Command: `status`

Show status of all gralph loop sessions.

```bash
gralph status
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

View logs for a gralph loop session.

```bash
gralph logs <name> [options]
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
gralph logs myapp

# Follow logs in real-time
gralph logs myapp --follow
```

### Command: `resume`

Resume crashed or stopped loops. Finds sessions that were marked as running but whose processes are no longer alive, and restarts them.

```bash
gralph resume [name]
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
gralph resume

# Resume specific session
gralph resume myapp
```

### Command: `server`

Start an HTTP status server for remote monitoring.

```bash
gralph server [options]
```

**Options:**
| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--host` | `-H` | Host/IP to bind to | 127.0.0.1 |
| `--port` | `-p` | Port number to listen on | 8080 |
| `--token` | `-t` | Bearer token for authentication | (required for non-localhost) |
| `--open` | | Disable token requirement (not recommended) | false |

**API Endpoints:**
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/status` | List all sessions |
| GET | `/status/:name` | Get specific session |
| POST | `/stop/:name` | Stop a session |

**Dependencies:** `socat` is preferred; `nc` (netcat) is used as a fallback.

**Security note:** The server defaults to localhost-only binding. When binding to non-localhost addresses (e.g., `0.0.0.0`), a `--token` is required for security. Use `--open` to explicitly disable this requirement (not recommended). Always restrict network access via firewall/VPN when exposing the server.

**Examples:**
```bash
# Start server on localhost (no token required)
gralph server

# Expose to network (token required)
gralph server --host 0.0.0.0 --port 8080 --token "my-secret-token"

# Query from remote
curl -H "Authorization: Bearer my-secret-token" http://server:8080/status
```

### Command: `backends`

List available AI backends and their installation status.

```bash
gralph backends
```

**Output example:**
```
Available AI backends:

  claude (installed)
      Models: claude-opus-4.5

  opencode (not installed)
      Install: See https://opencode.ai/docs/cli/ for installation

Usage: gralph start <dir> --backend <name>
```

### Command: `config`

Manage gralph configuration values.

```bash
gralph config
gralph config list
gralph config get <key>
gralph config set <key> <value>
```

**Notes:**
- `gralph config` and `gralph config list` print the merged configuration (default + global + project).
- `gralph config set` writes to the global config file at `~/.config/gralph/config.yaml`.

### Command: `version`

Show gralph version.

```bash
gralph version
```

**Aliases:** `--version`, `-v`

### Command: `help`

Show top-level help message with usage information.

```bash
gralph help
gralph --help
gralph -h
gralph
```

**Notes:**
- `--help` and `-h` are global flags; they always print the top-level usage.
- Running `gralph` with no arguments prints the same usage.

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
gralph start .

# Check progress
gralph status

# View live logs
gralph logs my-webapp --follow
```

### Example 2: Multiple Concurrent Projects

Run several projects simultaneously on a VPS:

```bash
# Start multiple projects with custom names
gralph start ~/projects/backend --name api-server --max-iterations 50
gralph start ~/projects/frontend --name web-ui --max-iterations 30
gralph start ~/projects/mobile --name mobile-app --max-iterations 40

# Check all at once
gralph status

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
gralph start . --task-file TODO.md

# Use a custom completion marker
gralph start . --completion-marker "ALL_DONE"

# Combined
gralph start ~/projects/app \
  --name myapp \
  --task-file TASKS.md \
  --completion-marker "FINISHED" \
  --max-iterations 100
```

### Example 4: Remote VPS Monitoring

Run loops on a VPS and monitor from anywhere:

```bash
# On your VPS: start the status server
gralph server --port 8080 --token "your-secret-token"

# On your VPS: start your loops
gralph start ~/projects/app1 --name app1
gralph start ~/projects/app2 --name app2

# From your laptop or phone: check status
curl -H "Authorization: Bearer your-secret-token" \
  http://your-vps-ip:8080/status

# Stop a session remotely
curl -X POST -H "Authorization: Bearer your-secret-token" \
  http://your-vps-ip:8080/stop/app1
```

### Example 5: Webhook Notifications

Get notified when loops complete or fail:

```bash
# Discord webhook
gralph start . --webhook "https://discord.com/api/webhooks/123/abc"

# Slack webhook
gralph start . --webhook "https://hooks.slack.com/services/T00/B00/xxx"

# Or set globally
gralph config set notifications.webhook "https://discord.com/api/webhooks/123/abc"
```

See [Notifications](#notifications) for details on webhook formats and notification events.

### Example 6: Recovery After Reboot

Resume loops after a server restart:

```bash
# After reboot, check what was running
gralph status

# Resume all previously running sessions
gralph resume

# Or resume specific session
gralph resume myapp
```

### Example 7: Foreground Mode (No tmux)

Run in foreground for debugging or CI/CD:

```bash
# Run without tmux (blocks until complete)
gralph start . --no-tmux

# Useful for:
# - Debugging loop behavior
# - CI/CD pipelines
# - Single-iteration testing
```

### Example 8: Model Override

Use a different Claude model:

```bash
# Use Claude Opus for more complex tasks
gralph start . --model claude-opus-4.5

# Or set as default in config
gralph config set defaults.model claude-opus-4.5
```

### Example 9: Using OpenCode Backend

Use OpenCode instead of Claude Code:

```bash
# Use OpenCode with default model
gralph start . --backend opencode

# Use OpenCode with GPT-5.2 Codex
gralph start . --backend opencode --model opencode/gpt-5.2-codex

# Use OpenCode with Gemini 3 Pro
gralph start . --backend opencode --model google/gemini-3-pro

# Set OpenCode as default in config
gralph config set defaults.backend opencode
gralph config set defaults.model opencode/gpt-5.2-codex
```

## Troubleshooting

### Common Issues

#### Loop terminates immediately without doing work

**Symptom:** The loop exits after the first iteration without completing any tasks.

**Causes and solutions:**

1. **Task file not found**
   ```bash
   # Verify task file exists
   ls -la PRD.md

   # Or specify correct path
   gralph start . --task-file TASKS.md
   ```

2. **No unchecked tasks in file**
   ```bash
   # Check for unchecked tasks
   grep -c '^\s*- \[ \]' PRD.md

   # Should return > 0 if tasks remain
   ```

3. **Claude API authentication issues**
   ```bash
   # Verify claude CLI works
   claude --version
   claude --print -p "hello"
   ```

#### Loop keeps running but never completes tasks

**Symptom:** Iterations increase but task count stays the same.

**Causes and solutions:**

1. **Tasks are too complex** - Break tasks into smaller, more specific items
2. **Ambiguous task descriptions** - Make tasks clear and actionable
3. **Missing dependencies** - Ensure required files/packages exist
4. **Check logs for errors:**
   ```bash
   gralph logs <session-name>
   ```

#### "Session already exists" error

**Symptom:** `Error: Session 'myapp' already exists`

**Solution:**
```bash
# Stop the existing session first
gralph stop myapp

# Or use a different name
gralph start . --name myapp-v2
```

#### tmux session not found

**Symptom:** `Error: tmux session not found` when checking status

**Causes:**

1. **tmux not installed:**
   ```bash
   # Install tmux
   sudo apt install tmux  # Ubuntu/Debian
   brew install tmux      # macOS
   ```

2. **Session crashed** - Use resume to restart:
   ```bash
   gralph resume myapp
   ```

#### Failed to acquire state lock

**Symptom:** `Error: Failed to acquire state lock within 10s`

**Causes and solutions:**

1. **Another gralph process is still running**
   ```bash
   gralph status
   ```

2. **Stale lock file after a crash**
   ```bash
   rm ~/.config/gralph/state.lock
   ```

3. **Missing flock on macOS**
   ```bash
   brew install flock
   ```

#### Status server not responding

**Symptom:** `curl: (7) Failed to connect` when querying status server

**Solutions:**

1. **Check if server is running:**
   ```bash
   ps aux | grep "gralph server"
   ```

2. **Check port availability:**
   ```bash
   # Is port in use?
   lsof -i :8080

   # Try different port
   gralph server --port 9090
   ```

3. **Firewall blocking connections:**
   ```bash
   # Allow port through firewall
   sudo ufw allow 8080/tcp
   ```

#### Webhooks not firing

**Symptom:** No notifications received on completion

**Solutions:**

1. **Verify webhook URL is correct:**
   ```bash
   # Test webhook manually
   curl -X POST "https://discord.com/api/webhooks/..." \
     -H "Content-Type: application/json" \
     -d '{"content": "Test message"}'
   ```

2. **Check curl is installed:**
   ```bash
   which curl || sudo apt install curl
   ```

3. **Check network connectivity from server**

#### Permission denied errors

**Symptom:** `Permission denied` when starting loop

**Solutions:**

1. **Check gralph is executable:**
   ```bash
   chmod +x ~/.local/bin/gralph
   ```

2. **Check lib files are readable:**
   ```bash
   chmod -R 755 ~/.config/gralph/lib/
   ```

3. **Check project directory is writable:**
   ```bash
   ls -la ~/projects/myapp/
   ```

#### JSON parse errors in logs

**Symptom:** `jq: parse error` messages in output

**Causes:**

1. **jq not installed:**
   ```bash
   sudo apt install jq  # Ubuntu/Debian
   brew install jq      # macOS
   ```

2. **Claude output contains invalid JSON** - Usually transient, loop will retry

#### Loop stuck at max iterations

**Symptom:** Loop hits max iterations without completing

**Solutions:**

1. **Increase max iterations:**
   ```bash
   gralph start . --max-iterations 100
   ```

2. **Check remaining task complexity:**
   ```bash
   grep '^\s*- \[ \]' PRD.md
   ```

3. **Review logs for repeated errors:**
   ```bash
   gralph logs myapp | grep -i error
   ```

### Debugging Tips

#### Enable verbose logging

```bash
# Set debug log level in config
GRALPH_LOGGING_LEVEL=debug gralph start .
```

#### Run in foreground mode

For easier debugging, run without tmux:

```bash
gralph start . --no-tmux
```

#### Inspect state file

```bash
# View current state
cat ~/.config/gralph/state.json | jq .

# Find specific session
jq '.sessions.myapp' ~/.config/gralph/state.json
```

#### Check tmux session directly

```bash
# List gralph tmux sessions
tmux list-sessions | grep gralph

# Attach to session
tmux attach -t gralph-myapp
```

#### Clean up stale state

If state file becomes corrupted or out of sync:

```bash
# Backup current state
cp ~/.config/gralph/state.json ~/.config/gralph/state.json.bak

# Remove specific session from state
jq 'del(.sessions.myapp)' ~/.config/gralph/state.json > tmp.json && \
  mv tmp.json ~/.config/gralph/state.json

# Or reset entirely (stops all sessions first)
gralph stop --all
rm ~/.config/gralph/state.json
```

### Getting Help

If you continue to experience issues:

1. **Check the logs** - Most issues are visible in `gralph logs <name>`
2. **Verify dependencies** - Run `./install.sh` to check all requirements
3. **Open an issue** - Include logs and system info (OS, bash version, etc.)

## Security

Please report vulnerabilities via the private advisory workflow in
[SECURITY.md](SECURITY.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and PR guidelines.

## Code of Conduct

Please read and follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes.

## License

MIT
