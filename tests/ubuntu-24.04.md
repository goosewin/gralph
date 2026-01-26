# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Go toolchain available (`go version`)
- `go test ./...` passes
- `go build -o gralph` produces a binary
- `./gralph version` returns current version
- `./gralph help` prints CLI usage
- `./gralph status` runs with empty state
- `./gralph backends` lists available backends
- `./gralph config list` shows configuration
- Help output includes new flags: `--no-tmux`, `--backend`, `--webhook`, `--variant`, `--prompt-template`
- Server command available in help

## Command Output

```bash
# Verify dependencies
go version

# Tests and build
go test ./...
go build -o gralph

# Basic commands
./gralph version
./gralph help
./gralph status

# New commands
./gralph backends
./gralph config list

# Verify new flags in help
./gralph help | grep -E '\-\-(no-tmux|backend|webhook|variant|prompt-template)'
./gralph help | grep 'server'
```

Expected results:
- Go is installed and tests pass
- Binary builds successfully
- Version prints `gralph v<version>`
- Help output lists commands and options including new flags
- Status reports no sessions found
- Backends lists `claude` as available
- Config list shows default configuration values
- New flags appear in help text
