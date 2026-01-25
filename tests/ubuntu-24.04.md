# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Dependencies present: `jq`, `tmux`, `bash`
- `./bin/rloop version` returns current version
- `./bin/rloop help` prints CLI usage
- `./bin/rloop status` runs with empty state

## Command Output

```bash
command -v jq && command -v tmux && command -v bash
./bin/rloop version
./bin/rloop help
./bin/rloop status
```

Expected results:
- All dependency commands resolve in PATH
- Version prints `rloop v1.1.0`
- Help output lists commands and options
- Status reports no sessions found
