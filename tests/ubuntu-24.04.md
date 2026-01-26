# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Rust toolchain installed: `cargo --version`
- Build succeeds: `cargo build --release --manifest-path gralph-rs/Cargo.toml`
- `./gralph-rs/target/release/gralph-rs version` returns current version
- `./gralph-rs/target/release/gralph-rs help` prints CLI usage
- `./gralph-rs/target/release/gralph-rs status` runs with empty state
- `./gralph-rs/target/release/gralph-rs backends` lists available backends
- `./gralph-rs/target/release/gralph-rs config list` shows configuration
- Help output includes flags: `--no-tmux`, `--backend`, `--webhook`, `--variant`, `--prompt-template`
- Server command available in help

## Command Output

```bash
cargo --version
cargo build --release --manifest-path gralph-rs/Cargo.toml

./gralph-rs/target/release/gralph-rs version
./gralph-rs/target/release/gralph-rs help
./gralph-rs/target/release/gralph-rs status

./gralph-rs/target/release/gralph-rs backends
./gralph-rs/target/release/gralph-rs config list

./gralph-rs/target/release/gralph-rs help | grep -E '\-\-(no-tmux|backend|webhook|variant|prompt-template)'
./gralph-rs/target/release/gralph-rs help | grep 'server'
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
