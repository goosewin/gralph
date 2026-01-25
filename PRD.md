# Ralph Loop CLI - Product Requirements Document

## Overview

**Ralph Loop CLI** (`rloop`) is a standalone utility for running autonomous AI coding loops using Claude Code. It spawns fresh Claude Code sessions iteratively until all tasks in a PRD are complete, designed for remote/AFK execution on VPS or local machines.

Named after the "Ralph Wiggum" technique - persistent iteration despite setbacks.

## Problem Statement

1. **Premature termination**: Current implementations terminate when LLM *mentions* the completion promise without actually completing (e.g., "I cannot respond with `<promise>COMPLETE</promise>` yet")
2. **No easy installation**: Scripts are copy-pasted, not version-controlled or installable
3. **Poor observability**: Hard to check status of multiple running loops remotely
4. **No persistence**: Session state lost on crash/reboot
5. **Single-project**: No easy way to manage multiple concurrent loops

## Solution

A installable CLI tool with:
- Robust completion detection (regex matching actual completion, not mentions)
- npm/shell-based installation from private GitHub repo
- Built-in status server for remote monitoring
- State persistence and crash recovery
- Multi-project orchestration

---

## Functional Requirements

### FR-1: Core Loop Execution

```bash
# Basic usage
rloop start ./my-project

# With options
rloop start ./my-project \
  --max-iterations 50 \
  --task-file PRD.md \
  --completion-marker "ALL_DONE" \
  --model claude-sonnet-4-20250514
```

**Loop logic:**
1. Read task file (PRD.md by default)
2. Count unchecked tasks (`- [ ]`)
3. If count > 0, spawn Claude Code with task prompt
4. Wait for Claude to exit
5. Re-count unchecked tasks
6. If count decreased OR Claude output contains `<promise>MARKER</promise>` at end of response (not mentioned mid-text), continue
7. If count == 0, mark complete
8. Repeat until complete or max iterations

### FR-2: Completion Detection (Bug Fix)

**Current bug**: Script detects `<promise>COMPLETE</promise>` anywhere in output, including:
```
"I cannot output <promise>COMPLETE</promise> because tasks remain"
```

**Fix**: Only detect completion when:
1. Zero `- [ ]` patterns remain in task file, AND
2. Promise appears as final statement (last 200 chars of output), OR
3. Promise appears on its own line with no negating context

**Regex pattern:**
```bash
# WRONG - matches mentions
[[ "$result" == *"<promise>$PROMISE</promise>"* ]]

# RIGHT - matches only standalone completion signals  
echo "$result" | tail -c 500 | grep -qE "^<promise>$PROMISE</promise>\s*$"
```

**Additional safeguard**: Always verify task file has zero `- [ ]` before accepting completion.

### FR-3: Installation

```bash
# From GitHub (private repo)
curl -fsSL https://raw.githubusercontent.com/USER/ralph-cli/main/install.sh | bash

# Or clone and install
git clone git@github.com:USER/ralph-cli.git
cd ralph-cli
./install.sh

# Or npm (if packaged)
npm install -g @user/ralph-cli
```

**Install script responsibilities:**
- Check dependencies (claude, jq, tmux)
- Copy `rloop` to `/usr/local/bin/` or `~/.local/bin/`
- Create config directory `~/.config/rloop/`
- Set up shell completions (optional)

### FR-4: Multi-Project Management

```bash
# Start multiple projects
rloop start ~/projects/app1 --name app1
rloop start ~/projects/app2 --name app2
rloop start ~/projects/app3 --name app3

# List all running
rloop status

# Output:
# NAME          DIR                      ITERATION  STATUS     REMAINING
# app1          ~/projects/app1          5/30       running    12 tasks
# app2          ~/projects/app2          3/30       running    8 tasks  
# app3          ~/projects/app3          15/30      complete   0 tasks

# Stop one
rloop stop app1

# Stop all
rloop stop --all

# View logs
rloop logs app1
rloop logs app1 --follow
```

### FR-5: State Persistence

State file: `~/.config/rloop/state.json`

```json
{
  "sessions": {
    "app1": {
      "name": "app1",
      "dir": "/root/projects/app1",
      "task_file": "PRD.md",
      "pid": 12345,
      "tmux_session": "rloop-app1",
      "started_at": "2026-01-23T10:00:00Z",
      "iteration": 5,
      "max_iterations": 30,
      "status": "running",
      "last_task_count": 12,
      "completion_marker": "COMPLETE",
      "log_file": "/root/projects/app1/.rloop/ralph.log"
    }
  }
}
```

### FR-6: Crash Recovery

```bash
# On machine reboot, resume all previously running sessions
rloop resume

# Resume specific
rloop resume app1
```

Reads state file, restarts any sessions marked "running" that don't have active PIDs.

### FR-7: Configuration

Global config: `~/.config/rloop/config.yaml`

```yaml
defaults:
  max_iterations: 30
  task_file: PRD.md
  completion_marker: COMPLETE
  model: claude-sonnet-4-20250514
  
claude:
  flags:
    - --dangerously-skip-permissions
  env:
    IS_SANDBOX: "1"

notifications:
  on_complete: true
  webhook: https://hooks.example.com/notify  # optional
  
logging:
  level: info
  retain_days: 7
```

Project-level override: `.rloop.yaml` in project directory.

### FR-8: Notifications

```bash
# Discord/Slack webhook on completion
rloop start ./project --webhook "https://discord.com/api/webhooks/..."

# Or configure globally
rloop config set notifications.webhook "https://..."
```

### FR-9: Remote Status Server (Optional)

```bash
# Start status API server
rloop server --port 8080 --token SECRET

# Query remotely
curl -H "Authorization: Bearer SECRET" http://vps:8080/status
```

---

## Non-Functional Requirements

### NFR-1: Dependencies
- `claude` CLI (Claude Code)
- `jq` for JSON parsing
- `tmux` for session management
- `bash` 4.0+
- Optional: `curl` for notifications

### NFR-2: Platforms
- Linux (primary - VPS use case)
- macOS (secondary)
- WSL2 (best effort)

### NFR-3: Performance
- Minimal overhead between iterations (<5s)
- Log rotation to prevent disk fill
- Memory-efficient (no daemon, uses tmux)

---

## CLI Reference

```
rloop - Autonomous AI coding loops

USAGE:
  rloop <command> [options]

COMMANDS:
  start <dir>         Start a new ralph loop
  stop <name>         Stop a running loop  
  stop --all          Stop all loops
  status              Show status of all loops
  logs <name>         View logs for a loop
  resume              Resume crashed/stopped loops
  config              Manage configuration
  server              Start status API server
  version             Show version

START OPTIONS:
  --name, -n          Session name (default: directory name)
  --max-iterations    Max iterations before giving up (default: 30)
  --task-file, -f     Task file path (default: PRD.md)
  --completion-marker Completion promise text (default: COMPLETE)
  --model, -m         Claude model override
  --webhook           Notification webhook URL
  --no-tmux           Run in foreground (blocks)

EXAMPLES:
  rloop start .
  rloop start ~/project --name myapp --max-iterations 50
  rloop status
  rloop logs myapp --follow
  rloop stop myapp
```

---

## File Structure

```
ralph-cli/
├── install.sh              # Installation script
├── uninstall.sh            # Removal script
├── bin/
│   └── rloop               # Main executable (bash)
├── lib/
│   ├── core.sh             # Core loop logic
│   ├── state.sh            # State management
│   ├── status.sh           # Status display
│   ├── notify.sh           # Notifications
│   └── utils.sh            # Helpers
├── completions/
│   ├── rloop.bash          # Bash completions
│   └── rloop.zsh           # Zsh completions
├── config/
│   └── default.yaml        # Default configuration
├── README.md
├── LICENSE
└── .github/
    └── workflows/
        └── release.yml     # Auto-release on tag
```

---

## Implementation Tasks

### Unit 01: Project Scaffolding
- [ ] Initialize git repository
- [ ] Create directory structure per File Structure section
- [ ] Create README.md with installation instructions
- [ ] Create LICENSE (MIT)
- [ ] Create .gitignore

### Unit 02: Core Loop Engine (`lib/core.sh`)
- [ ] Implement `run_iteration()` function
  - [ ] Spawn Claude Code with proper flags
  - [ ] Capture output to temp file
  - [ ] Stream output to log and stdout
  - [ ] Extract result from JSON stream
- [x] Implement `check_completion()` function
  - [ ] Count `- [ ]` in task file
  - [ ] Verify promise appears at END of output (not mentioned)
  - [ ] Return true only if both conditions met
- [ ] Implement `run_loop()` function
  - [ ] Initialize iteration counter
  - [ ] Call run_iteration in loop
  - [ ] Check completion after each
  - [ ] Handle max iterations
  - [ ] Update state after each iteration
- [x] Add configurable prompt template
- [ ] Add model override support

### Unit 03: State Management (`lib/state.sh`)
- [ ] Implement `init_state()` - create state file if missing
- [ ] Implement `get_session()` - read session by name
- [ ] Implement `set_session()` - upsert session
- [ ] Implement `delete_session()` - remove session
- [ ] Implement `list_sessions()` - get all sessions
- [ ] Implement `cleanup_stale()` - remove sessions with dead PIDs
- [ ] Use file locking for concurrent access

### Unit 04: CLI Commands (`bin/rloop`)
- [ ] Implement argument parsing
- [ ] Implement `cmd_start()`
  - [ ] Validate directory exists
  - [ ] Validate task file exists
  - [ ] Check for existing session with same name
  - [ ] Start tmux session with loop
  - [ ] Save state
- [ ] Implement `cmd_stop()`
  - [ ] Find session by name
  - [ ] Kill tmux session
  - [ ] Update state
- [x] Implement `cmd_status()`
  - [ ] List all sessions
  - [ ] Show table with iteration/status/remaining
  - [ ] Color coding (green=complete, yellow=running, red=failed)
- [x] Implement `cmd_logs()`
  - [ ] Tail log file
  - [ ] Support --follow flag
- [ ] Implement `cmd_resume()`
  - [ ] Find sessions marked running with dead PIDs
  - [ ] Restart their tmux sessions
- [x] Implement `cmd_version()`

### Unit 05: Configuration (`lib/config.sh`)
 - [x] Implement `load_config()` - merge default + global + project configs
 - [x] Implement `get_config()` - get specific key
- [ ] Implement `set_config()` - set global config value
- [x] Create default.yaml with sensible defaults
- [x] Support environment variable overrides (RLOOP_MAX_ITERATIONS, etc.)

### Unit 06: Installation Script (`install.sh`)
- [ ] Check for required dependencies (claude, jq, tmux)
- [ ] Prompt to install missing (apt/brew)
- [ ] Determine install path (~/.local/bin or /usr/local/bin)
- [ ] Copy bin/rloop to install path
- [ ] Copy lib/ to ~/.config/rloop/lib/
- [ ] Create default config
- [ ] Install shell completions
- [ ] Print success message with usage examples

### Unit 07: Notifications (`lib/notify.sh`)
- [x] Implement `send_webhook()` - POST to webhook URL
- [x] Implement `notify_complete()` - format completion message
- [x] Implement `notify_failed()` - format failure message
- [x] Support Discord webhook format
- [x] Support Slack webhook format
- [x] Support generic JSON POST

### Unit 08: Status Server (Optional, `lib/server.sh`)
- [ ] Implement simple HTTP server using netcat/socat
- [ ] GET /status - return JSON of all sessions
- [ ] GET /status/:name - return specific session
- [ ] POST /stop/:name - stop a session
- [ ] Bearer token authentication
- [ ] CORS headers for browser access

### Unit 09: Testing & Documentation
- [ ] Write usage examples in README
- [ ] Document all CLI commands
- [ ] Document configuration options
- [ ] Add troubleshooting section
- [x] Create example PRD.md template
- [x] Test on fresh Ubuntu 24.04
- [ ] Test on macOS

### Unit 10: Release Automation
- [ ] Create GitHub Actions workflow
- [ ] Auto-create release on version tag
- [ ] Attach install script to release
 - [x] Update version in rloop script

---

## Prompt Template

Default prompt used for each iteration:

```
You are completing tasks from a PRD file. 

TASK FILE: {task_file}

INSTRUCTIONS:
1. Read the task file carefully
2. Find any task marked with '- [ ]' (unchecked checkbox)
3. If you find unchecked tasks:
   - Pick ONE task to complete
   - Implement it fully with tests if applicable
   - Mark it '- [ ]' in the task file
   - Commit changes with descriptive message
   - Exit normally (do NOT output the completion promise)
4. If ALL tasks are marked '- [ ]' (zero unchecked remain):
   - Verify by searching for '- [ ]' pattern
   - If truly zero remain, output ONLY: <promise>{completion_marker}</promise>

CRITICAL: 
- Do NOT mention <promise>{completion_marker}</promise> unless outputting it as completion signal
- Do NOT output the promise if ANY '- [ ]' remain
- Complete ONE task per session, then exit

Current iteration: {iteration}/{max_iterations}
```

---

## Success Criteria

1. **Reliable completion**: Never terminates early due to LLM mentioning promise
2. **Easy install**: Single curl command installs on fresh VPS
3. **Multi-project**: Can run 5+ concurrent loops without issues
4. **Crash recovery**: `rloop resume` restores all sessions after reboot
5. **Observable**: Can check status of all loops from phone via webhook/API
6. **Zero maintenance**: Runs AFK for hours without intervention

---

## Open Questions

1. Should we support non-Claude backends (OpenCode, Cursor)?
2. Should state be SQLite instead of JSON for better concurrency?
3. Should we add cost tracking per session?
4. Should we support GitHub Issues as task source?

---

## Appendix: Fixed Ralph Script

Reference implementation with completion detection fix:

```bash
#!/bin/bash
set -e

PROJECT_DIR="$1"
MAX_ITERATIONS="${2:-30}"
COMPLETION_PROMISE="${3:-COMPLETE}"

if [ -z "$PROJECT_DIR" ]; then
  echo "Usage: $0 <project_dir> [max_iterations] [completion_promise]"
  exit 1
fi

cd "$PROJECT_DIR"
mkdir -p .rloop

TASK_FILE="PRD.md"
LOG_FILE=".rloop/ralph.log"

stream_text='select(.type == "assistant").message.content[]? | select(.type == "text").text // empty | gsub("\n"; "\r\n") | . + "\r\n\n"'
final_result='select(.type == "result").result // empty'

count_remaining() {
  grep -c '^\s*- \[ \]' "$TASK_FILE" 2>/dev/null || echo "0"
}

check_genuine_completion() {
  local result="$1"
  local remaining=$(count_remaining)
  
  # Must have zero remaining tasks
  if [ "$remaining" -gt 0 ]; then
    return 1
  fi
  
  # Promise must appear at the end (last 500 chars), not just mentioned
  local tail_result=$(echo "$result" | tail -c 500)
  if echo "$tail_result" | grep -qE "<promise>$COMPLETION_PROMISE</promise>"; then
    # Verify it's not negated (common patterns)
    if echo "$tail_result" | grep -qE "(cannot|can't|won't|will not|do not|don't).*<promise>"; then
      return 1
    fi
    return 0
  fi
  
  return 1
}

echo "Starting ralph loop in $PROJECT_DIR" | tee "$LOG_FILE"
echo "Max iterations: $MAX_ITERATIONS" | tee -a "$LOG_FILE"
echo "Started at: $(date -Iseconds)" | tee -a "$LOG_FILE"
echo "Initial remaining tasks: $(count_remaining)" | tee -a "$LOG_FILE"

for ((i=1; i<=$MAX_ITERATIONS; i++)); do
  remaining=$(count_remaining)
  echo "" | tee -a "$LOG_FILE"
  echo "=== Iteration $i/$MAX_ITERATIONS (Remaining: $remaining) ===" | tee -a "$LOG_FILE"
  
  if [ "$remaining" -eq 0 ]; then
    echo "Zero tasks remaining before iteration, checking completion..." | tee -a "$LOG_FILE"
  fi
  
  tmpfile=$(mktemp)
  trap "rm -f $tmpfile" EXIT

  IS_SANDBOX=1 claude \
    --dangerously-skip-permissions \
    --verbose \
    --print \
    --output-format stream-json \
    -p "Read $TASK_FILE carefully. Find any task marked '- [ ]' (unchecked).

If unchecked tasks exist:
- Complete ONE task fully
- Mark it '- [ ]' in $TASK_FILE  
- Commit changes
- Exit normally (do NOT output completion promise)

If ZERO '- [ ]' remain (all complete):
- Verify by searching the file
- Output ONLY: <promise>$COMPLETION_PROMISE</promise>

CRITICAL: Never mention the promise unless outputting it as the completion signal.

Iteration: $i/$MAX_ITERATIONS" \
  2>&1 | grep --line-buffered '^{' \
  | tee "$tmpfile" \
  | jq --unbuffered -rj "$stream_text" \
  | tee -a "$LOG_FILE"

  result=$(jq -r "$final_result" "$tmpfile" 2>/dev/null || cat "$tmpfile")

  if check_genuine_completion "$result"; then
    echo "" | tee -a "$LOG_FILE"
    echo "✅ Ralph complete after $i iterations." | tee -a "$LOG_FILE"
    echo "FINISHED: $(date -Iseconds)" | tee -a "$LOG_FILE"
    exit 0
  fi
  
  new_remaining=$(count_remaining)
  echo "Tasks remaining after iteration: $new_remaining" | tee -a "$LOG_FILE"
  
  sleep 2
done

echo "" | tee -a "$LOG_FILE"
echo "⚠️ Hit max iterations ($MAX_ITERATIONS)" | tee -a "$LOG_FILE"
echo "Remaining tasks: $(count_remaining)" | tee -a "$LOG_FILE"
echo "FINISHED: $(date -Iseconds)" | tee -a "$LOG_FILE"
exit 1
```
