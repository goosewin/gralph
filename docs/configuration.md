# Configuration Reference

## Config File Locations

- **Global**: `~/.config/gralph/config.yaml`
- **Project**: `.gralph.yaml` in project directory

## Section: `defaults`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_iterations` | integer | `30` | Maximum loop iterations |
| `task_file` | string | `PRD.md` | Task file path |
| `completion_marker` | string | `COMPLETE` | Completion signal text |
| `context_files` | string | `ARCHITECTURE.md, DECISIONS.md, ...` | Context files to inject |
| `backend` | string | `claude` | AI backend (`claude`, `opencode`, `gemini`, `codex`) |
| `model` | string | (none) | Model override |

## Section: `claude`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `flags` | array | `["--dangerously-skip-permissions"]` | CLI flags |
| `env` | object | `{ IS_SANDBOX: "1" }` | Environment variables |

## Section: `opencode`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_model` | string | `opencode/example-code-model` | Default model |

## Section: `gemini`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_model` | string | `gemini-1.5-pro` | Default model |
| `flags` | array | `["--headless"]` | CLI flags |

## Section: `codex`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_model` | string | `example-codex-model` | Default model |
| `flags` | array | `["--quiet", "--auto-approve"]` | CLI flags |

## Section: `notifications`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `on_complete` | boolean | `true` | Notify on completion |
| `webhook` | string | (none) | Webhook URL |

## Section: `logging`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `level` | string | `info` | Log level (`debug`, `info`, `warn`, `error`) |
| `retain_days` | integer | `7` | Days to keep logs |

## Environment Variables

All config keys can be overridden with `GRALPH_` prefix:

| Config Key | Environment Variable |
|------------|---------------------|
| `defaults.max_iterations` | `GRALPH_DEFAULTS_MAX_ITERATIONS` |
| `defaults.backend` | `GRALPH_DEFAULTS_BACKEND` |
| `notifications.webhook` | `GRALPH_NOTIFICATIONS_WEBHOOK` |

## Precedence

1. Default config
2. Global config (`~/.config/gralph/config.yaml`)
3. Project config (`.gralph.yaml`)
4. Environment variables
5. CLI arguments

## Example

```yaml
defaults:
  max_iterations: 50
  backend: claude
  model: claude-opus-4-5

claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

notifications:
  webhook: https://discord.com/api/webhooks/123/abc

logging:
  level: info
  retain_days: 7
```
