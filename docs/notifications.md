# Notifications

Gralph can send webhook notifications when loops complete or fail.

## Setup

```bash
# CLI flag
gralph start . --webhook "https://discord.com/api/webhooks/123/abc"

# Config file
gralph config set notifications.webhook "https://hooks.slack.com/services/..."

# Environment
export GRALPH_NOTIFICATIONS_WEBHOOK="https://example.com/webhook"
```

## Events

| Event | Trigger | Fields |
|-------|---------|--------|
| **Complete** | All tasks done | session, project, iterations, duration |
| **Failed** | Loop stopped | session, project, reason, iterations, remaining_tasks |

**Failure reasons:** `max_iterations`, `error`, `manual_stop`

## Supported Platforms

| Platform | URL Pattern |
|----------|-------------|
| Discord | `discord.com/api/webhooks/` |
| Slack | `hooks.slack.com/services/` |
| Generic | Any other URL |

## Payload Examples

**Discord:**
```json
{
  "embeds": [{
    "title": "✅ Gralph Complete",
    "description": "Session **myapp** has finished.",
    "color": 5763719,
    "fields": [
      {"name": "Project", "value": "`/path/to/project`"},
      {"name": "Iterations", "value": "15"}
    ]
  }]
}
```

**Generic:**
```json
{
  "event": "complete",
  "status": "success",
  "session": "myapp",
  "project": "/path/to/project",
  "iterations": "15",
  "duration": "2h 15m 30s"
}
```
