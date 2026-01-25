# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Dependencies present: `jq`, `tmux`, `bash`
- `./bin/gralph version` returns current version
- `./bin/gralph help` prints CLI usage
- `./bin/gralph status` runs with empty state

## Command Output

```bash
command -v jq && command -v tmux && command -v bash
./bin/gralph version
./bin/gralph help
./bin/gralph status
```

Expected results:
- All dependency commands resolve in PATH
- Version prints `gralph v1.1.0`
- Help output lists commands and options
- Status reports no sessions found
