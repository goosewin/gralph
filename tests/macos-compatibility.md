# macOS Compatibility Test Report

**Test Date:** 2026-01-23
**Tested By:** Automated code analysis
**Target Platforms:** macOS 12+ (Monterey and later)

## Summary

The Rust CLI has been reviewed for macOS compatibility. The runtime dependencies
are limited to optional `tmux` for background sessions and external backend CLIs.

## Automation

macOS smoke tests run in GitHub Actions on `macos-14` using:

```bash
cargo test --workspace
```

## Compatibility Checklist

### ✅ Passed: Core Dependencies

| Dependency | macOS Status | Notes |
|------------|--------------|-------|
| Rust toolchain | ✅ Available | Install via rustup |
| tmux | ✅ Available | `brew install tmux` (optional) |
| claude CLI | ✅ Available | `npm install -g @anthropic-ai/claude-code` |

### ✅ Passed: Shell Completions

Builds generate bash/zsh completions under `completions/`.

### ✅ Passed: Path Handling

All paths use standard macOS locations:
- `~/.config/gralph/` for config and state
- `.gralph/` for per-project logs

### ✅ Passed: Install Paths

The binary can be placed in either:
1. `~/.local/bin` (user-local)
2. `/usr/local/bin` (system-wide)

## Test Scenarios

### Scenario 1: Build from source

```bash
brew install tmux
curl https://sh.rustup.rs -sSf | sh
cargo build --release
./target/release/gralph version
```

### Scenario 2: Basic Loop Execution

```bash
mkdir ~/test-project && cd ~/test-project
echo '- [ ] Test task 1' > PRD.md
gralph start . --no-worktree --no-tmux --max-iterations 1
```

### Scenario 3: tmux Session Management

```bash
gralph start ~/test-project --name test
gralph status
gralph logs test
gralph stop test
```

## Recommendations

1. Publish macOS release artifacts (arm64 + x86_64).
2. Keep README requirements in sync with release packaging.

## Conclusion

The Rust CLI is compatible with macOS 12+ using standard Homebrew dependencies.
