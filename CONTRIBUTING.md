# Contributing

Thanks for taking the time to contribute to gralph.

## Before you start

- Check existing issues and discussions to avoid duplicates.
- For security issues, follow the guidance in `SECURITY.md`.

## Development setup

- Install Go 1.24+.
- Install `tmux` if you want to test background sessions (optional).
- Install at least one backend CLI (claude/opencode/gemini/codex) for live runs.

## Running tests

Run the test suite locally before submitting a PR:

```bash
# Run all Go tests
go test ./...
```

Tests live alongside packages under `internal/` and `cmd/`.

## Workflow

- Create a feature branch from `main`.
- Keep changes focused and scoped to a single concern.
- Update docs and task files when behavior changes.

## Pull requests

- Describe the problem and why the change is needed.
- Include any relevant test steps or results.
- Ensure new files and scripts are executable when required.

## Style

- Prefer idiomatic Go, keep functions small, and run `gofmt` on changed files.
- Keep CLI output stable and update docs when flags or behavior change.
