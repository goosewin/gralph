# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Rust toolchain installed: `cargo --version`
- Build succeeds: `cargo build --release`
- `./target/release/gralph version` returns current version
- `./target/release/gralph help` prints CLI usage
- `./target/release/gralph status` runs with empty state
- `./target/release/gralph backends` lists available backends
- `./target/release/gralph config list` shows configuration
- Help output includes flags: `--no-tmux`, `--backend`, `--webhook`, `--variant`, `--prompt-template`
- Server command available in help
- If testing a release asset, confirm the tarball matches the host architecture (linux-x86_64 vs linux-aarch64).

## Command Output

```bash
cargo --version
cargo build --release

./target/release/gralph version
./target/release/gralph help
./target/release/gralph status

./target/release/gralph backends
./target/release/gralph config list

./target/release/gralph help | grep -E '\-\-(no-tmux|backend|webhook|variant|prompt-template)'
./target/release/gralph help | grep 'server'
```

Expected results:
- Cargo is available in PATH
- Build completes successfully
- Version prints the current gralph release
- Help output lists commands and options including new flags
- Status reports no sessions found
- Backends lists available backends
- Config list shows default configuration values
- New flags appear in help text
