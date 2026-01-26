# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- .NET 10 SDK available (`dotnet --version`)
- Native AOT build succeeds (`scripts/publish-aot.sh --rid linux-x64`)
- `./dist/aot/linux-x64/gralph version` returns current version
- `./dist/aot/linux-x64/gralph help` prints CLI usage
- `./dist/aot/linux-x64/gralph status` runs with empty state
- `./dist/aot/linux-x64/gralph backends` lists available backends
- `./dist/aot/linux-x64/gralph config list` shows configuration
- Help output includes new flags: `--no-tmux`, `--backend`, `--webhook`, `--variant`, `--prompt-template`
- Server command available in help

## Command Output

```bash
# Verify toolchain
dotnet --version

# Build AOT binary
scripts/publish-aot.sh --rid linux-x64

# Basic commands
./dist/aot/linux-x64/gralph version
./dist/aot/linux-x64/gralph help
./dist/aot/linux-x64/gralph status

# New commands
./dist/aot/linux-x64/gralph backends
./dist/aot/linux-x64/gralph config list

# Verify new flags in help
./dist/aot/linux-x64/gralph help | grep -E '\-\-(no-tmux|backend|webhook|variant|prompt-template)'
./dist/aot/linux-x64/gralph help | grep 'server'
```

Expected results:
- `dotnet --version` prints a 10.x SDK
- AOT build emits `dist/aot/linux-x64/gralph`
- Version prints `gralph v1.1.0`
- Help output lists commands and options including new flags
- Status reports no sessions found
- Backends lists `claude` as available
- Config list shows default configuration values
- New flags appear in help text
