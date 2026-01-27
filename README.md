# Gralph CLI

Autonomous AI coding loops. Spawns fresh AI sessions iteratively until all tasks in a PRD are complete.

## Quick Start

```bash
# Install
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
  - `opencode` - See [opencode.ai](https://opencode.ai/docs/cli/)
  - `gemini` - `npm install -g @google/gemini-cli`
  - `codex` - `npm install -g @openai/codex`
- `tmux` for background sessions (optional with `--no-tmux`)

## Basic Commands

```bash
gralph start .                    # Start loop in current directory
gralph start . --backend opencode # Use different backend
gralph status                     # Check all running loops
gralph logs myapp --follow        # Watch logs
gralph stop myapp                 # Stop a loop
gralph resume                     # Resume after crash
```

## How It Works

1. Reads task file (`PRD.md` by default)
2. Finds unchecked tasks (`- [ ]` lines)
3. Spawns AI to complete one task
4. Repeats until all tasks done or max iterations hit

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

- [Configuration Reference](docs/configuration.md) - All config options
- [CLI Reference](docs/cli.md) - Full command documentation  
- [Backends](docs/backends.md) - Backend setup and models
- [Notifications](docs/notifications.md) - Webhook setup
- [Troubleshooting](docs/troubleshooting.md) - Common issues
- [PRD Format](docs/prd-format.md) - Task file structure

## License

MIT - Dan Goosewin
