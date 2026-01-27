# Troubleshooting

## Loop terminates immediately

1. **Task file not found** - Check `PRD.md` exists or use `--task-file`
2. **No unchecked tasks** - Run `grep '- \[ \]' PRD.md`
3. **Backend auth issues** - Test with `claude --version`

## Loop never completes tasks

1. Tasks too complex - Break into smaller items
2. Ambiguous descriptions - Make tasks specific
3. Check logs: `gralph logs <name>`

## "Session already exists"

```bash
gralph stop myapp
# or
gralph start . --name myapp-v2
```

## tmux session not found

```bash
# Install tmux
brew install tmux    # macOS
sudo apt install tmux  # Linux

# Or resume crashed session
gralph resume myapp
```

## Failed to acquire state lock

```bash
# Check running processes
gralph status

# Remove stale lock
rm ~/.config/gralph/state.lock
```

## Webhooks not firing

```bash
# Test webhook manually
curl -X POST "https://discord.com/api/webhooks/..." \
  -H "Content-Type: application/json" \
  -d '{"content": "Test"}'
```

## Debugging

```bash
# Verbose logging
GRALPH_LOGGING_LEVEL=debug gralph start .

# Run in foreground
gralph start . --no-tmux

# View state
cat ~/.config/gralph/state.json

# Check tmux sessions
tmux list-sessions | grep gralph
```

## Clean up stale state

```bash
gralph stop --all
rm ~/.config/gralph/state.json
```
