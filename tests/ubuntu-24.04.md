# Ubuntu 24.04 Smoke Test

Date: 2026-01-25
Environment: Ubuntu 24.04.3 LTS

## Checks

- Rust toolchain installed: `cargo --version`
- Build succeeds: `cargo build --release --manifest-path gralph-rs/Cargo.toml`
- `./gralph-rs/target/release/gralph version` returns current version
- `./gralph-rs/target/release/gralph help` prints CLI usage
- `./gralph-rs/target/release/gralph status` runs with empty state
- `./gralph-rs/target/release/gralph backends` lists available backends
- `./gralph-rs/target/release/gralph config list` shows configuration
- Help output includes flags: `--no-tmux`, `--backend`, `--webhook`, `--variant`, `--prompt-template`
- Server command available in help
- If testing a release asset, confirm the tarball matches the host architecture (linux-x86_64 vs linux-aarch64).

## Command Output

```bash
cargo --version
cargo build --release --manifest-path gralph-rs/Cargo.toml

./gralph-rs/target/release/gralph version
./gralph-rs/target/release/gralph help
./gralph-rs/target/release/gralph status

./gralph-rs/target/release/gralph backends
./gralph-rs/target/release/gralph config list

./gralph-rs/target/release/gralph help | grep -E '\-\-(no-tmux|backend|webhook|variant|prompt-template)'
./gralph-rs/target/release/gralph help | grep 'server'
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
