# Contributing

Thanks for taking the time to contribute to gralph.

## Before you start

- Check existing issues and discussions to avoid duplicates.
- For security issues, follow the guidance in `SECURITY.md`.

## Development setup

- Install the Rust toolchain (rustup + Cargo).
- Install `tmux` if you want to exercise background sessions.
- Install at least one backend CLI if you want to run integration flows.

## Running tests

Run the test suite locally before submitting a PR:

```bash
cargo test --manifest-path gralph-rs/Cargo.toml --workspace
```

## Workflow

- Create a feature branch from `main`.
- Keep changes focused and scoped to a single concern.
- Update docs and task files when behavior changes.

## Pull requests

- Describe the problem and why the change is needed.
- Include any relevant test steps or results.
- Ensure new files and scripts are executable when required.

## Style

- Format Rust code with `cargo fmt` and keep changes idiomatic.
- Keep CLI behavior consistent across macOS and Linux.
