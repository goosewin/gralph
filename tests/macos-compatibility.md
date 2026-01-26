# macOS Compatibility Test Report

**Test Date:** 2026-01-26
**Tested By:** Automated review
**Target Platforms:** macOS 12+ (Monterey and later)

## Summary

The gralph CLI is now a .NET 10 Native AOT binary. This review focuses on
binary execution, config paths, and backend CLI dependencies. No bash/jq/tmux
dependencies remain.

## Automation

macOS smoke tests run in GitHub Actions on `macos-14` using `dotnet test`.

## Compatibility Checklist

### ✅ Passed: Core Dependencies

| Dependency | macOS Status | Notes |
|------------|--------------|-------|
| .NET 10 SDK | ✅ Available | Required for building from source only |
| claude CLI | ✅ Available | `npm install -g @anthropic-ai/claude-code` |
| opencode CLI | ✅ Available | See https://opencode.ai/docs/cli/ |
| gemini CLI | ✅ Available | `npm install -g @google/gemini-cli` |
| codex CLI | ✅ Available | `npm install -g @openai/codex` |
| git | ✅ Built-in | Required for worktree commands |

### ✅ Passed: Config and State Paths

Defaults use `~/.config/gralph/` for configuration and state, which matches
macOS user-space conventions.

### ✅ Passed: Background Worker

Background loops spawn a child process (no tmux or shell utilities needed).

### ✅ Passed: Shell Completions (Optional)

Completion scripts can be installed manually for bash/zsh if desired.

### ⚠️ Note: Gatekeeper and Unsigned Binaries

macOS may quarantine downloaded binaries. If execution is blocked, remove
the quarantine attribute:

```bash
xattr -d com.apple.quarantine gralph
```

## Platform-Specific Notes

### Apple Silicon vs Intel

Use the correct RID when building AOT binaries:
- Apple Silicon: `osx-arm64`
- Intel: `osx-x64`

### SIP (System Integrity Protection)

No issues expected. gralph only writes to user-space directories:
- `~/.config/gralph/`
- `~/.local/bin/` (or another user-owned PATH)

## Test Scenarios

### Scenario 1: Build and run AOT binary

```bash
scripts/publish-aot.sh --rid osx-arm64
./dist/aot/osx-arm64/gralph version
```

### Scenario 2: Basic loop execution

```bash
mkdir ~/test-project && cd ~/test-project
echo '- [ ] Test task 1' > PRD.md
gralph start . --no-tmux --max-iterations 1
```

### Scenario 3: Status server

```bash
gralph server --host 127.0.0.1 --port 8080
curl http://127.0.0.1:8080/status
```

## Recommendations

1. Codesign and notarize release assets for smoother Gatekeeper handling.
2. Publish both `osx-arm64` and `osx-x64` binaries when possible.
3. Keep backend CLI install instructions current in README.

## Conclusion

The gralph CLI is compatible with macOS 12+ using Native AOT binaries and
standard user-space paths. No shell dependencies remain.
