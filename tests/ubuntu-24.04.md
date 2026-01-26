# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Dependencies present: `jq`, `tmux`, `bash`
- `./bin/gralph version` returns current version
- `./bin/gralph help` prints CLI usage
- `./bin/gralph status` runs with empty state
- `./bin/gralph backends` lists available backends
- `./bin/gralph config list` shows configuration
- Help output includes new flags: `--no-tmux`, `--backend`, `--webhook`, `--variant`, `--prompt-template`
- Server command available in help

## Command Output

```bash
# Verify dependencies
command -v jq && command -v tmux && command -v bash

# Basic commands
./bin/gralph version
./bin/gralph help
./bin/gralph status

# New commands
./bin/gralph backends
./bin/gralph config list

# Verify new flags in help
./bin/gralph help | grep -E '\-\-(no-tmux|backend|webhook|variant|prompt-template)'
./bin/gralph help | grep 'server'
```

Expected results:
- All dependency commands resolve in PATH
- Version prints `gralph v1.1.0`
- Help output lists commands and options including new flags
- Status reports no sessions found
- Backends lists `claude` as available
- Config list shows default configuration values
- New flags appear in help text
