# macOS Compatibility Test Report

**Test Date:** 2026-01-23
**Tested By:** Automated code analysis
**Target Platforms:** macOS 12+ (Monterey and later)

## Summary

The rloop CLI has been analyzed for macOS compatibility. Several potential issues were identified and documented below, along with their resolutions.

## Automation

macOS smoke tests run in GitHub Actions on `macos-14` using `tests/macos-smoke.sh`.

## Compatibility Checklist

### ✅ Passed: Core Dependencies

| Dependency | macOS Status | Notes |
|------------|--------------|-------|
| bash 4.0+ | ✅ Available | macOS ships with bash 3.x, but bash 4/5 available via Homebrew |
| jq | ✅ Available | `brew install jq` |
| tmux | ✅ Available | `brew install tmux` |
| curl | ✅ Built-in | Pre-installed on macOS |
| claude CLI | ✅ Available | `npm install -g @anthropic-ai/claude-code` |

### ✅ Passed: Package Manager Detection

The `install.sh` script correctly detects Homebrew on macOS:

```bash
# From install.sh:get_package_manager()
elif check_command brew; then
    echo "brew"
```

All dependency installation hints include Homebrew commands.

### ✅ Passed: Shell Completions

The installer handles both bash and zsh completion paths:

- **Bash**: `/etc/bash_completion.d/` or `~/.local/share/bash-completion/completions/`
- **Zsh**: `/usr/local/share/zsh/site-functions/` or `~/.zsh/completions/`

Zsh is the default shell on modern macOS, and the installer correctly installs zsh completions.

### ⚠️ Note: `flock` Command

**Issue:** The `flock` command used in `lib/state.sh` for file locking is not built into macOS.

**Impact:** Low - flock is available via Homebrew: `brew install flock`

**Resolution:** The install script should check for and offer to install flock on macOS. However, the tool will still function if flock fails - it will just display a warning about concurrent access.

**Current Behavior:** If flock is unavailable, state operations may have race conditions with concurrent access, but this is unlikely in typical single-user scenarios.

### ⚠️ Note: `date -Iseconds` Format

**Issue:** BSD date (macOS) has different syntax than GNU date (Linux).

**GNU date:** `date -Iseconds`
**BSD date:** `date -u +%Y-%m-%dT%H:%M:%S%z`

**Impact:** Low - Timestamp formatting for logs and state file.

**Current Code Analysis:**
- `lib/core.sh:330` - Uses `date -Iseconds` for logging
- `bin/rloop:385,459` - Uses `date -Iseconds` for session timestamps

**Resolution:** The `-Iseconds` flag is supported on macOS 12+ (Monterey and later) as Apple has updated their date command. For older macOS versions, users can install GNU coreutils: `brew install coreutils` and use `gdate`.

### ✅ Passed: Path Handling

All path handling uses POSIX-compliant methods:
- `$HOME` for user directory
- Standard config paths (`~/.config/rloop/`)
- No hardcoded Linux-specific paths

### ✅ Passed: Install Paths

The installer correctly determines install paths for macOS:

1. `~/.local/bin` - User-local, no sudo needed
2. `/usr/local/bin` - System-wide (common on macOS with Homebrew)

Both paths are standard for macOS.

### ✅ Passed: Shell Compatibility

All scripts use `#!/bin/bash` and bash-specific features that are available in bash 4.0+:
- `${BASH_REMATCH}` for regex matching
- Associative arrays (not used, but would work)
- `set -e` for error handling

**Recommendation:** Users on macOS should install modern bash via Homebrew:
```bash
brew install bash
```

And optionally add it to their allowed shells:
```bash
sudo bash -c 'echo /opt/homebrew/bin/bash >> /etc/shells'
```

## Platform-Specific Notes

### Homebrew Installation on Apple Silicon (M1/M2/M3)

Homebrew installs to `/opt/homebrew/` on Apple Silicon Macs instead of `/usr/local/`. The install script handles this correctly as it uses `command -v` to find executables rather than hardcoded paths.

### SIP (System Integrity Protection)

No issues expected - rloop only modifies user-space directories:
- `~/.config/rloop/`
- `~/.local/bin/`

### Gatekeeper

If downloading the script directly, users may need to allow execution:
```bash
chmod +x install.sh
xattr -d com.apple.quarantine install.sh  # If downloaded from browser
```

## Test Scenarios

### Scenario 1: Fresh Installation

```bash
# Install prerequisites
brew install bash jq tmux

# Clone and install
git clone git@github.com:USER/ralph-cli.git
cd ralph-cli
./install.sh

# Verify
rloop version
```

### Scenario 2: Basic Loop Execution

```bash
# Create test project
mkdir ~/test-project && cd ~/test-project
echo '- [ ] Test task 1' > PRD.md

# Start loop (foreground for testing)
rloop start . --no-tmux --max-iterations 1
```

### Scenario 3: tmux Session Management

```bash
# Start in background
rloop start ~/test-project --name test

# Check status
rloop status

# View logs
rloop logs test

# Stop
rloop stop test
```

## Recommendations

1. **Add flock to recommended dependencies** - Update README to mention flock on macOS
2. **Test on actual macOS hardware** - This analysis is code-based; real-world testing recommended
3. **Consider fallback for date formatting** - Add compatibility shim for older macOS versions

## Conclusion

The rloop CLI is compatible with macOS 12+ (Monterey and later) with standard Homebrew dependencies. No code changes are required for basic functionality. Users should ensure they have:

1. Homebrew installed
2. bash 4.0+ (via Homebrew)
3. jq, tmux (via Homebrew)
4. Optionally: flock for safer concurrent access

The tool follows POSIX standards and macOS conventions for installation paths and configuration directories.
