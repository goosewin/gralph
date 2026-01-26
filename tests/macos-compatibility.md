# macOS Compatibility Test Report

**Test Date:** 2026-01-23
**Tested By:** Automated code analysis
**Target Platforms:** macOS 12+ (Monterey and later)

## Summary

The Go-based gralph CLI has been reviewed for macOS compatibility. The binary is
self-contained and relies only on backend CLIs and optional tmux.

## Automation

macOS smoke tests run in GitHub Actions on `macos-14` using `go test ./...`.

## Compatibility Checklist

### ✅ Passed: Core Dependencies

| Dependency | macOS Status | Notes |
|------------|--------------|-------|
| Go 1.24+ | ✅ Available | Required only for building from source |
| tmux | ✅ Available | Optional for background sessions (`brew install tmux`) |
| claude CLI | ✅ Available | `npm install -g @anthropic-ai/claude-code` |
| opencode CLI | ✅ Available | See https://opencode.ai/docs/cli/ |
| gemini CLI | ✅ Available | `npm install -g @google/gemini-cli` (optional) |
| codex CLI | ✅ Available | `npm install -g @openai/codex` (optional) |

### ✅ Passed: File Locking

State locking uses Go's syscall-based file locks with a directory lock fallback.
No external `flock` binary is required.

### ✅ Passed: Timestamp Formatting

Timestamps are formatted with RFC3339 via Go's time package; no GNU/BSD `date`
compatibility issues apply.

### ✅ Passed: Path Handling

Paths use `~/.config/gralph/` and `.gralph/` directories with no hardcoded
Linux-specific paths.

### ✅ Passed: Install Paths

Binaries can be installed to `~/.local/bin` or `/usr/local/bin` depending on
user preference.

## Platform-Specific Notes

### Homebrew on Apple Silicon

Homebrew installs to `/opt/homebrew/` on Apple Silicon. Use `brew --prefix`
if you need explicit paths.

### Gatekeeper

If downloading a release binary from a browser, macOS may quarantine the file.
Remove quarantine if needed:

```bash
xattr -d com.apple.quarantine gralph
```

## Test Scenarios

### Scenario 1: Build from Source

```bash
go test ./...
go build -o gralph
./gralph version
```

### Scenario 2: Basic Loop Execution

```bash
mkdir ~/test-project && cd ~/test-project
echo '- [ ] Test task 1' > PRD.md
gralph start . --no-tmux --max-iterations 1
```

### Scenario 3: tmux Session Management

```bash
gralph start ~/test-project --name test
gralph status
gralph logs test
gralph stop test
```

## Recommendations

1. Keep macOS CI coverage for Go builds and tests.
2. Validate tmux-based background loops on macOS hardware periodically.
3. Update backend CLI install links if upstream changes.

## Conclusion

The Go-based gralph CLI is compatible with macOS 12+ with standard tooling.
Users should ensure they have a backend CLI installed and optional tmux for
background sessions.
