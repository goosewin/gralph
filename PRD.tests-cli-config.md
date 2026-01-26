# PRD: CLI + Config Test Coverage

## Goal
Increase confidence in CLI argument handling and configuration resolution by adding unit tests for parsing, path selection, and PRD template lookup.

## Non-Goals
- No behavioral changes to CLI flags.
- No new runtime dependencies or external command execution.

## Constraints
- Tests must be hermetic (no network).
- Prefer std/test helpers; add new dev-deps only if strictly necessary.

## Success Criteria
- CLI parsing and validation functions are covered with representative inputs.
- Config path precedence is covered, including env overrides.
- PRD template selection chooses project-local template before fallback.

## Tasks

### Task T-CLI-1

- **ID** T-CLI-1
- **Context Bundle** `src/main.rs`, `src/cli.rs`
- **DoD** Unit tests cover `resolve_prd_output`, `invalid_prd_path`, and CLI parse validation edge cases.
- **Checklist**
  * Test relative vs absolute output paths.
  * Test `--force` behavior on existing outputs.
  * Test invalid/missing required args via `Cli::try_parse_from`.
- **Dependencies** None
- [x] T-CLI-1 Add CLI helper unit tests for path + parse validation

### Task T-CONFIG-1

- **ID** T-CONFIG-1
- **Context Bundle** `src/config.rs`, `config/default.yaml`
- **DoD** Unit tests cover config path precedence for env overrides and fallbacks.
- **Checklist**
  * `GRALPH_DEFAULT_CONFIG` overrides bundled default.
  * `GRALPH_CONFIG_DIR` affects global config path.
  * Missing files fall back to `config/default.yaml`.
- **Dependencies** None
- [ ] T-CONFIG-1 Add config path precedence tests

### Task T-CLI-2

- **ID** T-CLI-2
- **Context Bundle** `src/main.rs`, `PRD.template.md`
- **DoD** `read_prd_template` selects local PRD template before fallback, using a temp project directory.
- **Checklist**
  * Temp dir with `PRD.template.md` is selected.
  * Missing file uses embedded default content.
- **Dependencies** T-CLI-1
- [ ] T-CLI-2 Add PRD template lookup tests
