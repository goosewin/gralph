# Contributing

Thanks for taking the time to contribute to gralph.

## Before you start

- Check existing issues and discussions to avoid duplicates.
- For security issues, follow the guidance in `SECURITY.md`.

## Development setup

- Install the .NET 10 SDK.
- Install at least one backend CLI if you want to run loops locally.
- Build from source: `dotnet build src/Gralph/Gralph.csproj`.
- Run from source: `dotnet run --project src/Gralph -- help`.

## Running tests

Run the test suite locally before submitting a PR:

```bash
# Run unit tests
dotnet test tests/Gralph.Tests
```

## Workflow

- Create a feature branch from `main`.
- Keep changes focused and scoped to a single concern.
- Update docs and task files when behavior changes.

## Pull requests

- Describe the problem and why the change is needed.
- Include any relevant test steps or results.
- Avoid committing build outputs (bin/obj).

## Style

- Follow existing C# naming and formatting conventions.
- Keep behavior cross-platform and avoid OS-specific paths.
