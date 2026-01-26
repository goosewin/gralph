# Gralph CLI (`gralph`)

Autonomous AI coding loops using Claude Code, OpenCode, Gemini, or Codex. Spawns fresh AI coding sessions iteratively until all tasks in a PRD are complete.

## Features

- **Multi-backend support** - Use Claude Code, OpenCode, Gemini CLI, or Codex CLI
- **Robust completion detection** - Requires zero unchecked tasks and the completion promise
- **Multi-project support** - Run multiple concurrent loops
- **State persistence** - Resume after crash/reboot
- **Remote monitoring** - Built-in status server for checking progress remotely
- **Notifications** - Webhooks for Discord/Slack on completion or failure
- **Cross-platform .NET** - Runs on macOS, Linux, and Windows without bash/jq/tmux dependencies
- **Self-contained builds** - Publish single-file binaries per platform

## Requirements

- At least one AI backend:
  - `claude` CLI (Claude Code) - `npm install -g @anthropic-ai/claude-code`
  - `opencode` CLI (OpenCode) - See https://opencode.ai/docs/cli/
  - `gemini` CLI (Gemini CLI) - `npm install -g @google/gemini-cli` (optional)
  - `codex` CLI (Codex CLI) - `npm install -g @openai/codex` (optional)
- .NET 10 SDK (only required when building or running from source)

### Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux | ✅ Supported | Primary development platform |
| macOS 12+ | ✅ Supported | Intel and Apple Silicon |
| Windows 10/11 | ✅ Supported | Self-contained or .NET 10 runtime |

## Installation

### Quick Install (release binary)

Download the matching release asset and place `gralph` in your PATH. If your platform is missing a release asset, build from source.

```bash
chmod +x gralph
mkdir -p ~/.local/bin
mv gralph ~/.local/bin/
```

### From Source (git clone)

```bash
git clone git@github.com:goosewin/gralph.git
cd gralph
dotnet build src/Gralph/Gralph.csproj
dotnet run --project src/Gralph -- help
```

### Manual Installation

If you prefer not to use a release asset:

1. Build a self-contained binary with `dotnet publish` (see below).
2. Copy the `gralph` binary to a directory in your PATH (e.g., `~/.local/bin/`).
3. Copy `config/default.yaml` to `~/.config/gralph/config/default.yaml` if you want a local default.

### Build (.NET)

To produce self-contained, single-file executables, use the publish profiles:

```bash
# Windows x64
dotnet publish src/Gralph/Gralph.csproj -c Release -p:PublishProfile=PublishWinX64

# macOS x64
dotnet publish src/Gralph/Gralph.csproj -c Release -p:PublishProfile=PublishOsxX64

# macOS arm64
dotnet publish src/Gralph/Gralph.csproj -c Release -p:PublishProfile=PublishOsxArm64

# Linux x64
dotnet publish src/Gralph/Gralph.csproj -c Release -p:PublishProfile=PublishLinuxX64
```

Outputs land under `src/Gralph/bin/Release/net10.0/<RID>/publish/`.

### Uninstalling

Remove the `gralph` binary from your PATH and delete local data if desired.

**What to remove:**

- `~/.config/gralph/config.yaml`
- `~/.config/gralph/state.json`
- `~/.config/gralph/state.lock` (or `state.lock.dir`)
- Project logs under `.gralph/` directories

On Windows, the config path resolves under `%USERPROFILE%\.config\gralph`.

## Backends

gralph supports multiple AI coding assistants through a pluggable backend system.

### Claude Code (Default)

[Claude Code](https://claude.ai/claude-code) is Anthropic's official CLI for Claude.

```bash
# Install Claude Code
npm install -g @anthropic-ai/claude-code

# Use Claude Code (default)
gralph start .
```

**Models:**
- `claude-opus-4-5`

### OpenCode

[OpenCode](https://opencode.ai) is an open-source AI coding CLI that supports multiple providers.

```bash
# Install OpenCode (see https://opencode.ai/docs/cli/)

# Use OpenCode
gralph start . --backend opencode

# OpenCode models use provider/model format
gralph start . --backend opencode --model opencode/example-code-model
gralph start . --backend opencode --model google/gemini-1.5-pro
```

**Models (format: provider/model):**
- `opencode/example-code-model` (default for opencode)
- `anthropic/claude-opus-4-5`
- `google/gemini-1.5-pro`

### Gemini CLI

[Gemini CLI](https://geminicli.com/docs/) is Google's AI coding CLI that runs in headless mode.

```bash
# Install Gemini CLI
npm install -g @google/gemini-cli

# Use Gemini CLI
gralph start . --backend gemini
```

**Models:**
- `gemini-1.5-pro` (default)

### Codex CLI

[Codex CLI](https://developers.openai.com/codex/cli) is OpenAI's AI coding CLI.

```bash
# Install Codex CLI
npm install -g @openai/codex

# Use Codex CLI
gralph start . --backend codex
```

**Models:**
- `example-codex-model` (default)

### Setting Default Backend

Set the default backend in your config file:

```yaml
# ~/.config/gralph/config.yaml
defaults:
  backend: opencode
  model: opencode/example-code-model
```

Or via environment variable:

```bash
export GRALPH_DEFAULTS_BACKEND=opencode
```

## Usage

Examples assume `gralph` is on your PATH. When running from source, prefix commands with:

```bash
dotnet run --project src/Gralph -- <command>
```

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
# Basic usage - start loop in current directory (background child process)
gralph start .

# Start with options
gralph start ~/projects/myapp \
  --name myapp \
  --max-iterations 50 \
  --task-file PRD.md \
  --completion-marker "COMPLETE"

# Run in the foreground for debugging
gralph start . --no-tmux
```

### Check Status

```bash
# List all running loops
gralph status
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

### Worktree Workflow

Use worktrees to isolate task branches under `.worktrees/task-<ID>`.
Task IDs use the `LETTER-NUMBER` format (for example, `C-6`).

Steps:
1. Create the worktree for the task.
2. Work inside `.worktrees/task-<ID>`.
3. Finish the task to merge and remove it.

```bash
# Create a task worktree (IDs look like C-6)
gralph worktree create C-6

# Work inside the new worktree directory
cd .worktrees/task-C-6

# Finish the task (merge and remove worktree)
gralph worktree finish C-6
```

## Configuration

### Global Configuration

Location: `~/.config/gralph/config.yaml`

```yaml
defaults:
  max_iterations: 30
  task_file: PRD.md
  completion_marker: COMPLETE
  context_files: ARCHITECTURE.md, DECISIONS.md, CHANGELOG.md, RISK_REGISTER.md, PROCESS.md
  backend: claude
  model: claude-opus-4-5

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

notifications:
  on_complete: true
  on_fail: false
  webhook: https://hooks.example.com/notify

logging:
  level: info
  retain_days: 7
```

### Project Configuration

Create `.gralph.yaml` in your project directory to override global settings.

### Environment Variables

Legacy overrides (still supported):
- `GRALPH_MAX_ITERATIONS`
- `GRALPH_TASK_FILE`
- `GRALPH_COMPLETION_MARKER`
- `GRALPH_BACKEND`
- `GRALPH_MODEL`

Additional configuration paths:
- `GRALPH_CONFIG_DIR`
- `GRALPH_GLOBAL_CONFIG`
- `GRALPH_DEFAULT_CONFIG`
- `GRALPH_PROJECT_CONFIG_NAME`

## Configuration Options Reference

### Section: `defaults`

Default values for loop behavior.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `defaults.max_iterations` | integer | `30` | Maximum number of loop iterations before giving up. Prevents infinite loops. |
| `defaults.task_file` | string | `PRD.md` | Path to the task file relative to project directory. |
| `defaults.completion_marker` | string | `COMPLETE` | The text used in `<promise>MARKER</promise>` to signal completion. |
| `defaults.context_files` | string | `ARCHITECTURE.md, DECISIONS.md, CHANGELOG.md, RISK_REGISTER.md, PROCESS.md` | Comma-separated list of shared context files to inject into the prompt. |
| `defaults.backend` | string | `claude` | AI backend to use: `claude`, `opencode`, `gemini`, or `codex`. |
| `defaults.model` | string | (none) | Model to use. Format depends on backend (see Backends section). |

### Section: `claude`

Settings for the Claude Code backend.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `claude.flags` | array | `["--dangerously-skip-permissions"]` | CLI flags passed to `claude` command. |
| `claude.env` | object | `{ IS_SANDBOX: "1" }` | Environment variables set when running Claude. |

### Section: `opencode`

Settings for the OpenCode backend.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `opencode.default_model` | string | `opencode/example-code-model` | Default model in provider/model format. |

### Section: `gemini`

Settings for the Gemini CLI backend.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `gemini.default_model` | string | `gemini-1.5-pro` | Default model (gemini-1.5-pro). |
| `gemini.flags` | array | `["--headless"]` | CLI flags passed to `gemini` command. |

### Section: `codex`

Settings for the Codex CLI backend.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `codex.default_model` | string | `example-codex-model` | Default model (example-codex-model). |
| `codex.flags` | array | `["--quiet", "--auto-approve"]` | CLI flags passed to `codex` command. |

### Section: `notifications`

Notification settings for loop events.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `notifications.on_complete` | boolean | `true` | Send notification when loop completes successfully. |
| `notifications.on_fail` | boolean | `false` | Send notification when loop fails. |
| `notifications.webhook` | string | (none) | Webhook URL for notifications (Discord, Slack, or generic JSON POST). |

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
| `defaults.context_files` | `GRALPH_DEFAULTS_CONTEXT_FILES` |
| `defaults.backend` | `GRALPH_DEFAULTS_BACKEND` |
| `notifications.webhook` | `GRALPH_NOTIFICATIONS_WEBHOOK` |
| `logging.level` | `GRALPH_LOGGING_LEVEL` |

### Configuration Precedence

Configuration values are loaded in the following order (later sources override earlier):

1. **Default config** (`config/default.yaml` near the binary, or `~/.config/gralph/config/default.yaml`)
2. **Global config** (`~/.config/gralph/config.yaml`)
3. **Project config** (`.gralph.yaml` in project directory)
4. **Environment variables** (`GRALPH_*`)
5. **CLI arguments** (e.g., `--max-iterations`)

### Supported YAML Features

Gralph loads YAML using YamlDotNet and flattens nested keys into dotted paths.
Sequence values are converted to comma-separated strings.

**Example:**

```yaml
claude:
  flags:
    - --dangerously-skip-permissions
```

Becomes:

```
claude.flags=--dangerously-skip-permissions
```

## Notifications

Gralph can send webhook notifications when loops complete or fail.

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
- `max_iterations`
- `error`
- `manual_stop`

### Supported Webhook Platforms

Gralph auto-detects the webhook platform from the URL and formats payloads accordingly:

| Platform | URL Pattern | Format |
|----------|-------------|--------|
| Discord | `discord.com/api/webhooks/` | Discord embed with colored status |
| Slack | `hooks.slack.com/services/` | Slack block kit attachment |
| Generic | Any other URL | JSON POST |

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
  max_iterations: 50
  task_file: PRD.md
  completion_marker: COMPLETE
  context_files: ARCHITECTURE.md,DECISIONS.md,CHANGELOG.md,RISK_REGISTER.md,PROCESS.md
  backend: claude
  model: claude-opus-4-5

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

opencode:
  default_model: opencode/example-code-model

gemini:
  default_model: gemini-1.5-pro
  flags:
    - --headless

codex:
  default_model: example-codex-model
  flags:
    - --quiet
    - --auto-approve

notifications:
  on_complete: true
  on_fail: false
  webhook: https://discord.com/api/webhooks/123/abc

logging:
  level: info
  retain_days: 7
```

```yaml
# ~/projects/myapp/.gralph.yaml - Project-specific overrides

defaults:
  max_iterations: 100
  task_file: TASKS.md
  backend: opencode
  model: google/gemini-1.5-pro

notifications:
  webhook: https://hooks.slack.com/services/T00/B00/xxx
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
  prd check <file>    Validate PRD task blocks
  prd create          Generate a spec-compliant PRD
  worktree create <ID> Create task worktree
  worktree finish <ID> Finish task worktree
  backends            List available AI backends
  config              Manage configuration
  server              Start status API server
  version             Show version
  help                Show this help message
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
| `--backend` | `-b` | AI backend to use (claude, opencode, gemini, codex) | claude |
| `--model` | `-m` | Model to use (format depends on backend) | (from config) |
| `--variant` | | Model variant override (backend-specific) | (none) |
| `--prompt-template` | | Path to custom prompt template file | (none) |
| `--webhook` | | Notification webhook URL | (none) |
| `--no-tmux` | | Run in foreground (blocks) | false |
| `--strict-prd` | | Validate PRD before starting the loop | false |

**Examples:**
```bash
# Start with defaults (uses Claude)
gralph start .

# Start with custom name and iterations
gralph start ~/projects/myapp --name myapp --max-iterations 50

# Use different task file
gralph start . --task-file TODO.md

# Use OpenCode backend
gralph start . --backend opencode

# Use OpenCode with specific model
gralph start . --backend opencode --model opencode/example-code-model

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

### Command: `resume`

Resume crashed or stopped loops.

```bash
gralph resume [name]
```

**Arguments:**
- `[name]` - Optional specific session to resume. If omitted, resumes all resumable sessions.

### Command: `prd`

Generate or validate PRDs.

```bash
gralph prd create [options]
gralph prd check <file> [options]
```

**Options (create):**
| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--dir` | | Project directory | current |
| `--output` | `-o` | Output PRD file path | PRD.generated.md |
| `--goal` | | Short description of what to build | (required) |
| `--constraints` | | Constraints or non-functional requirements | (optional) |
| `--context` | | Extra context files (comma-separated) | (none) |
| `--sources` | | External URLs or references (comma-separated) | (none) |
| `--backend` | `-b` | Backend for PRD generation | (from config) |
| `--model` | `-m` | Model override for PRD generation | (from config) |
| `--allow-missing-context` | | Allow missing Context Bundle paths | false |
| `--multiline` | | Enable multiline prompts (interactive) | false |
| `--interactive` | | Force interactive prompts | auto |
| `--no-interactive` | | Disable interactive prompts | auto |
| `--force` | | Overwrite existing output file | false |

**Options (check):**
| Option | Description |
|--------|-------------|
| `--allow-missing-context` | Allow missing Context Bundle paths |

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

**Security note:** The server defaults to localhost-only binding. When binding to non-localhost addresses (e.g., `0.0.0.0`), a `--token` is required for security. Use `--open` to explicitly disable this requirement (not recommended). Always restrict network access via firewall/VPN when exposing the server.

### Command: `backends`

List available AI backends and their installation status.

```bash
gralph backends
```

### Command: `config`

Manage gralph configuration values.

```bash
gralph config
gralph config list
gralph config get <key>
gralph config set <key> <value>
```

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

## How It Works

1. Reads the task file (PRD.md by default)
2. Counts unchecked tasks inside task blocks (or `- [ ]` lines when no blocks exist)
3. Invokes the configured backend with the task prompt
4. Waits for the backend to complete one task and exit
5. Re-counts unchecked tasks
6. Repeats until all tasks complete or max iterations reached

### Completion Detection

The loop only terminates when:
1. Zero unchecked task lines remain in task blocks (or the whole file when no blocks exist), AND
2. The completion promise appears as the final output (not just mentioned mid-text)

This prevents premature termination when the backend mentions the promise without actually completing.

### Task Block Format

Task files can group related context using task blocks. A task block starts with a header like
`### Task P-1` and includes the task metadata plus checklist items. When blocks exist, gralph
selects the first block that contains an unchecked line and injects the full block into the
prompt. Task blocks end at the next `### Task` header or at a section divider like `---` or a
new `##` section heading. If no task blocks are present, gralph falls back to the first
unchecked `- [ ]` line.

**Example:**
```markdown
### Task P-1

- **ID** P-1
- **Context Bundle** `src/Gralph/Core/CoreLoop.cs`
- **DoD** Add task block parsing
- **Checklist**
  * Parser extracts task blocks from PRD.
  * Fallback to single-line tasks when no blocks exist.
- **Dependencies** None
- [ ] P-1 Implement parser
```

### PRD Validation

Validate PRD task blocks for required fields and consistent checklists.

```bash
# Check a PRD file for schema errors
gralph prd check PRD.md
```

Use strict mode to block loop start on invalid PRDs:

```bash
gralph start . --strict-prd
```

**Common validation failures:**
- Missing required fields in a task block: **ID**, **Context Bundle**, **DoD**, **Checklist**, or **Dependencies**
- Multiple unchecked task lines within a single task block
- No unchecked task line in a block that should be actionable
- Unchecked task lines outside task blocks
- Open Questions sections (not allowed)
- Context Bundle paths that do not exist (use `--allow-missing-context` to bypass)
- Context Bundle entries outside the repo root (absolute paths must stay within the project)
- Context Bundle with no file paths

### PRD Sanitization (Generated Files)

`gralph prd create` sanitizes generated output before writing to disk:
- Drops any Open Questions section (and any text before the first heading)
- Keeps only Context Bundle entries that exist in the repo and, when provided, appear in the allowed-context list
- Removes extra unchecked task lines beyond the first inside a block
- Strips stray unchecked task lines that appear outside task blocks
- Falls back to `README.md` as Context Bundle when no valid entries remain

### Stack Detection

`gralph prd create` infers a stack summary by scanning the project root for common
ecosystem files and dependency hints (for example `package.json`, `go.mod`,
`pyproject.toml`, `Cargo.toml`, `Gemfile`, `pom.xml`, `build.gradle`, `.csproj`,
`Dockerfile`, or `terraform` files). It can report multiple stacks when the repo
contains more than one ecosystem and records the evidence file paths.

## Shared Memory System

Gralph runs stateless iterations, so shared documents provide durable context between runs and keep execution aligned with the protocol.

- [PROCESS.md](PROCESS.md) - Worktree protocol steps and guardrails.
- [ARCHITECTURE.md](ARCHITECTURE.md) - System modules, runtime flow, and storage map.
- [DECISIONS.md](DECISIONS.md) - Recorded architectural choices with rationale.
- [RISK_REGISTER.md](RISK_REGISTER.md) - Risks and mitigations for context loss and process drift.

## Using gralph to build gralph

The repo includes a minimal self-hosting example set that exercises the PRD schema.

- Example PRDs: `examples/README.md` (see `examples/PRD-Stage-P-Example.md` and `examples/PRD-Stage-A-Example.md`).

Run the example flow from the repo root:

```bash
gralph start . --task-file examples/PRD-Stage-P-Example.md --no-tmux --backend claude --model claude-opus-4-5 && \
  gralph start . --task-file examples/PRD-Stage-A-Example.md --no-tmux --backend claude --model claude-opus-4-5
```

## Usage Examples

### Example 0: Generate a PRD

Generate a spec-compliant PRD interactively (step-by-step prompts and confirmation):

```bash
gralph prd create --dir . --output PRD.generated.md --goal "Add a billing dashboard"
```

Non-interactive with explicit inputs:

```bash
gralph prd create --dir . --output PRD.generated.md \
  --goal "Add a billing dashboard" \
  --constraints "Use existing auth and billing tables" \
  --context "README.md,ARCHITECTURE.md" \
  --sources "https://stripe.com/docs,https://nextjs.org/docs" \
  --no-interactive
```

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
```

### Example 3: Custom Task File and Completion Marker

Use a different task file or completion marker:

```bash
# Use TODO.md instead of PRD.md
gralph start . --task-file TODO.md

# Use a custom completion marker
gralph start . --completion-marker "ALL_DONE"
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
```

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
gralph start . --no-tmux
```

### Example 8: Model Override

Use a different Claude model:

```bash
# Use Claude Opus for more complex tasks
gralph start . --model claude-opus-4-5

# Or set as default in config
gralph config set defaults.model claude-opus-4-5
```

### Example 9: Using OpenCode Backend

Use OpenCode instead of Claude Code:

```bash
# Use OpenCode with default model
gralph start . --backend opencode

# Use OpenCode with example model
gralph start . --backend opencode --model opencode/example-code-model
```

### Example 10: Using Gemini CLI Backend

Use Gemini CLI for Google's AI models:

```bash
# Use Gemini CLI with default model (gemini-1.5-pro)
gralph start . --backend gemini

# Set Gemini as default in config
gralph config set defaults.backend gemini
gralph config set defaults.model gemini-1.5-pro
```

### Example 11: Using Codex CLI Backend

Use Codex CLI for OpenAI's coding models:

```bash
# Use Codex CLI with default model (example-codex-model)
gralph start . --backend codex
```

## Troubleshooting

### Common Issues

#### Loop terminates immediately without doing work

**Symptom:** The loop exits after the first iteration without completing any tasks.

**Causes and solutions:**

1. **Task file not found**
   ```bash
   ls -la PRD.md
   gralph start . --task-file TASKS.md
   ```

2. **No unchecked tasks in file**
   ```bash
   grep -c '^\s*- \[ \]' PRD.md
   ```

3. **Backend CLI not installed**
   ```bash
   gralph backends
   ```

#### Background loop not running

**Symptom:** `gralph status` shows a running session but no progress.

**Solutions:**
1. Run in foreground to see errors:
   ```bash
   gralph start . --no-tmux
   ```
2. Check logs:
   ```bash
   gralph logs <session-name>
   ```

#### Failed to acquire state lock

**Symptom:** `Error: Failed to acquire state lock within 10s`

**Solutions:**
1. Ensure no other gralph process is running:
   ```bash
   gralph status
   ```
2. Remove a stale lock file if no process is running:
   ```bash
   rm ~/.config/gralph/state.lock
   ```

#### Status server not responding

**Symptom:** `curl: (7) Failed to connect` when querying status server

**Solutions:**
1. Verify the server is running with the right host/port
2. Ensure you are passing the correct `Authorization` header when token auth is enabled

#### Webhooks not firing

**Symptom:** No notifications received on completion

**Solutions:**
1. Verify webhook URL is correct
2. Check network connectivity from the host running gralph

### Debugging Tips

#### Run in foreground mode

```bash
gralph start . --no-tmux
```

#### Inspect state file

```bash
cat ~/.config/gralph/state.json
```

### Getting Help

If you continue to experience issues:
1. Check the logs with `gralph logs <name>`
2. Verify backend CLI installation with `gralph backends`
3. Open an issue with logs and system info

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
