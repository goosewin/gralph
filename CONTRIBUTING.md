# Contributing

Thanks for taking the time to contribute to gralph.

## Before you start

- Check existing issues and discussions to avoid duplicates.
- For security issues, follow the guidance in `SECURITY.md`.

## Development setup

- Install dependencies: `bash` 4+, `jq`, and `tmux`.
- From a local clone, you can run `./install.sh` to install the CLI.

## Running tests

Run the test suite locally before submitting a PR:

```bash
# Run all tests
./tests/config-test.sh
./tests/state-test.sh
./tests/loop-test.sh
./tests/macos-smoke.sh
```

Test files:
- `tests/config-test.sh` - Tests for config get/set functionality
- `tests/state-test.sh` - Tests for state management (sessions)
- `tests/loop-test.sh` - Tests for core loop functions (prompt rendering, task counting, completion detection)
- `tests/macos-smoke.sh` - Basic smoke tests for CLI commands

## Workflow

- Create a feature branch from `main`.
- Keep changes focused and scoped to a single concern.
- Update docs and task files when behavior changes.

## Pull requests

- Describe the problem and why the change is needed.
- Include any relevant test steps or results.
- Ensure new files and scripts are executable when required.

## Style

- Prefer clear, defensive Bash with explicit error handling.
- Keep scripts portable across macOS and Linux.
