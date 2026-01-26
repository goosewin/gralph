# AOT Build Metrics

Metrics recorded for Native AOT builds of `gralph`. Use `scripts/verify-aot.sh`
to regenerate and update this file on each platform.

## Environment
- Date: 2026-01-26
- OS: macOS 15.5 (Darwin)
- Arch: arm64
- .NET SDK: 10.0.101
- Command: `scripts/verify-aot.sh --runs 15`
- Startup probe: `gralph --version`

## Metrics
| RID | Binary | Size (MiB) | Startup median (ms) | Startup p95 (ms) | Notes |
| --- | ------ | ---------- | ------------------- | ---------------- | ----- |
| osx-arm64 | `dist/aot/osx-arm64/gralph` | 7.96 | 4.5 | 5.2 | Release AOT build |
