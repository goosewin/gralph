# Contributing

Thanks for taking the time to contribute to gralph.

## Before you start

- Check existing issues and discussions to avoid duplicates.
- For security issues, follow the guidance in `SECURITY.md`.

## Development setup

- Install dependencies: .NET 10 SDK and at least one backend CLI (claude/opencode/gemini/codex).
- Build or run locally with `dotnet run --project src/Gralph -- --help`.

## Running tests

Run the test suite locally before submitting a PR:

```bash
# Run all tests
dotnet test tests/Gralph.Tests/Gralph.Tests.csproj
```

Test files:
- `tests/Gralph.Tests/ConfigServiceTests.cs` - Tests for config merging and overrides
- `tests/Gralph.Tests/StateStoreTests.cs` - Tests for state persistence
- `tests/Gralph.Tests/CoreLoopIntegrationTests.cs` - Loop integration checks
- `tests/Gralph.Tests/CliArgumentParsingTests.cs` - CLI option parsing coverage

## Workflow

- Create a feature branch from `main`.
- Keep changes focused and scoped to a single concern.
- Update docs and task files when behavior changes.

## Pull requests

- Describe the problem and why the change is needed.
- Include any relevant test steps or results.
- Ensure new files and scripts are executable when required.

## Style

- Follow .NET conventions and keep code formatted.
- Prefer small, testable classes with clear error messages.
