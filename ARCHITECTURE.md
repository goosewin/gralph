# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/main.rs` is the thin CLI entrypoint. It delegates to `cli_entrypoint` re-exported from `src/lib.rs` and returns its `ExitCode`.
`src/entrypoint.rs` owns the CLI entrypoint helper that parses args, builds real deps, runs the app, and maps results to exit codes.
`src/lib.rs` exposes `run`, `Deps`, and the entrypoint helper for external callers.
`src/app.rs` owns the `run` entrypoint, dependency seams, command dispatch, and
doctor diagnostics.
`src/app/loop_session.rs` implements start/run-loop/stop/status/logs/resume handlers with `Deps`.
`src/app/prd_init.rs` implements `gralph prd` and `gralph init` plus PRD/template helpers.
`src/app/worktree.rs` implements worktree commands and auto-worktree flow.
`src/cli.rs` defines the clap command tree and options; `build.rs` generates bash/zsh completions during build.

`src/core.rs` owns the execution loop for iteration execution, task counting, completion checks, and loop orchestration.
`src/state.rs` manages persistent session state with file locking and atomic writes.
`src/server.rs` implements the HTTP status server, CORS handling, bearer auth, and WebSocket state broadcasting.
`src/config.rs` loads default/global/project YAML config with env overrides.
`src/prd.rs` provides PRD validation, sanitization, and stack detection utilities.
`src/task.rs` centralizes task block parsing helpers shared by core and PRD validation.
`src/verifier.rs` implements the verifier pipeline helpers for tests, coverage, static checks, PR creation, and review gating.
`src/update.rs` handles release update checks and installs.
`src/version.rs` defines the CLI version constants.

`src/backend` defines the backend trait and CLI-backed implementations (`backend/mod.rs` plus `backend/claude.rs`, `backend/opencode.rs`, `backend/gemini.rs`, `backend/codex.rs`).
`src/coordinator.rs` manages multi-agent orchestration for parallel task execution. The `Coordinator` struct maintains an agent pool with configurable capacity, a `WorkQueue` for dependency-aware task distribution, and lifecycle management (spawn, assign, complete, fail). `TaskNode` represents tasks with their dependencies, `topological_sort` orders tasks respecting dependencies with cycle detection, and `find_independent_tasks` identifies parallelizable work. `AgentWorktreeManager` provides thread-safe creation and cleanup of isolated git worktrees for parallel agents, ensuring each agent operates in its own worktree without interference. `DependencyGraph` provides task dependency analysis with execution level computation via `get_execution_levels()` for parallel scheduling, `get_ready_tasks()` and `schedule_tasks()` for concurrent task scheduling respecting capacity limits, cycle detection via `validate_no_cycles()`, and graph statistics including `max_parallelism()` and `critical_path_length()`. `LoadBalancer` distributes tasks based on agent capacity and backend availability using configurable strategies: `RoundRobin` for even distribution, `Weighted` for proportional allocation based on agent weights, and `LeastLoaded` for preferring agents with fewer active tasks. `AgentMetadata` tracks agent health status, weight, consecutive failures, and task metrics. Health checks via `LoadBalancer.health_check()` mark agents as healthy or unhealthy, and agents exceeding `max_consecutive_failures` are automatically removed from the pool when `auto_remove_unhealthy` is enabled. `ConflictDetector` identifies overlapping changes from parallel agents by analyzing git diffs between worktrees and a common base reference. `ConflictReport` aggregates detected conflicts with severity tracking (Low, Medium, High, Critical), resolution strategy options (Manual, FirstWins, LastWins, MergeNonOverlapping), and suggestions for resolution. `FileConflict` captures per-file conflict details with line-level change tracking via `LineChange`. `ConflictDetectionConfig` allows configurable context lines, strict mode for adjacent change detection, and file ignore patterns. `AgentSpecialization` defines agent types (General, CodeGen, Testing, Review, Documentation) for task routing. `TaskType` classifies tasks by their nature, with `infer_from_content()` using keyword analysis to determine the most likely type. `SpecializationRouter` manages task routing based on agent specializations, routing tasks to specialized agents when available and falling back to general agents otherwise. The `Coordinator` provides specialization-aware methods: `spawn_specialized_agent()` creates agents with specific specializations, `assign_task_by_specialization()` routes tasks to the best matching agent, and `find_best_agent_for_task_type()` identifies the optimal agent for a given task type. `AgentMessageChannel` provides inter-agent communication with per-agent message inboxes, configurable size limits, delivery timeout, and message TTL. `AgentMessage` carries id, from, to, message_type, timestamp, and optional correlation_id for request-response patterns. `AgentMessageType` includes HandoffRequest, HandoffAck, TaskResult, Ping, Pong, CancelRequest, and Custom variants. `HandoffCoordinator` manages task transitions between agents with `initiate_handoff()`, `accept_handoff()`, and `reject_handoff()` methods, tracking pending and completed handoffs. `AgentTimeoutTracker` monitors agent activity and identifies stuck agents that exceed configurable timeout thresholds, supporting warning and stuck states. `RetryConfig` provides exponential backoff with jitter for transient failure recovery, configurable via `max_retries`, `initial_delay`, `max_delay`, `backoff_multiplier`, and `jitter_factor`. `RetryPolicy` manages retry attempts with automatic delay calculation via `record_failure()` returning `Some(delay)` for retries or `None` when exhausted. `CircuitBreakerConfig` configures failure detection with `failure_threshold`, `success_threshold` for half-open recovery, `reset_timeout` for automatic transition to half-open, and `sampling_duration` for windowed failure counting. `CircuitBreaker` implements a state machine with Closed (normal operation), Open (blocking requests), and HalfOpen (testing recovery) states. State transitions occur when failures exceed threshold (Closed->Open), after reset timeout (Open->HalfOpen), after success threshold in half-open (HalfOpen->Closed), or on any failure in half-open (HalfOpen->Open). `ResilientExecutor` combines `RetryPolicy` and `CircuitBreaker` for comprehensive failure handling: retries handle transient failures while the circuit breaker prevents cascade failures by stopping retries when the circuit opens.
`src/notify.rs` formats and sends webhook notifications via reqwest.

## Frontend

`frontend/` contains the TypeScript/React Mission Control web UI.
`frontend/src/main.tsx` is the React entry point that renders the App component.
`frontend/src/App.tsx` is the root React component integrating SessionDashboard and KanbanBoard with tabs.
`frontend/src/components/SessionDashboard.tsx` displays all sessions with real-time updates.
`frontend/src/components/SessionCard.tsx` renders individual session details with status and progress.
`frontend/src/components/StatusBadge.tsx` shows status indicators (running/stopped/failed/completed/stale).
`frontend/src/components/KanbanBoard.tsx` displays PRD tasks as a Kanban board with drag-and-drop.
`frontend/src/components/KanbanColumn.tsx` renders a column of tasks grouped by status.
`frontend/src/components/TaskCard.tsx` displays task ID, title, status, and definition of done.
`frontend/src/hooks/useWebSocket.ts` manages WebSocket connection with reconnection handling.
`frontend/src/hooks/useSessions.ts` provides REST API hooks for fetching and stopping sessions.
`frontend/src/hooks/useTasks.ts` provides REST API hooks for fetching tasks and updating task status.
`frontend/src/hooks/useTheme.ts` manages theme state with localStorage persistence and system preference detection.
`frontend/src/hooks/useMediaQuery.ts` detects media query matches and provides useBreakpoints for responsive logic.
`frontend/src/hooks/useTouchSwipe.ts` detects touch swipe gestures for mobile task movement.
`frontend/src/hooks/useServiceWorker.ts` manages service worker registration and update notifications.
`frontend/src/components/ThemeToggle.tsx` provides light/dark/system theme selection buttons.
`frontend/src/components/HamburgerMenu.tsx` provides mobile navigation toggle with animated icon.
`frontend/src/components/OfflineIndicator.tsx` displays banner when user is offline.
`frontend/src/components/InstallPrompt.tsx` shows install button on mobile when PWA installation is available.
`frontend/src/components/Sidebar.tsx` provides navigation sidebar with collapsible sections and route highlighting.
`frontend/src/components/SessionLogViewer.tsx` displays full session log output with syntax highlighting, virtualized scrolling, search, and auto-scroll toggle.
`frontend/src/hooks/useLocalStorage.ts` syncs React state with localStorage for persistence across sessions.
`frontend/src/hooks/useLogs.ts` provides REST API hooks for fetching session logs with pagination support.
`frontend/src/types/session.ts` defines TypeScript types matching backend API responses, including Task and LogsResponse types.
`frontend/public/manifest.json` defines PWA metadata including name, icons, and display mode.
`frontend/public/sw.js` is the service worker that caches static assets for offline access.
`frontend/public/icons/` contains SVG icons for PWA installation (192x192, 512x512, maskable).
`frontend/vite.config.ts` configures Vite to build production assets to `assets/`.
The Axum server serves these static files via rust-embed at compile time.

## Runtime Flow

`src/main.rs` calls `cli_entrypoint` in `src/lib.rs`, which parses CLI
arguments, builds real dependencies, and calls `app::run`. The `run`
entrypoint dispatches to command handlers. The
start/run-loop paths optionally create a worktree, load configuration,
validate PRDs (when strict), and invoke `core::run_loop_with_clock`.
Each iteration builds the prompt, invokes the backend, parses the result,
checks for completion promises, and updates state callbacks with remaining
task counts. On completion or failure, the loop records duration, writes
final status to logs, and optionally sends notifications. When completion
succeeds and `verifier.auto_run` is true, the verifier pipeline runs in the
active worktree to execute tests, coverage, and static checks, open a PR
via `gh`, wait for the configured review gate, and merge after approvals.
By default the review gate requires explicit approval (`verifier.review.require_approval: true`);
disable it to allow auto-merge without an approval requirement.
Verifier defaults are stack-aware: Rust/Cargo keeps the default auto-run and
command settings, while non-Rust or unknown stacks default `verifier.auto_run`
to false and require explicit verifier commands.

## WebSocket Broadcasting

The server provides a WebSocket endpoint at `/ws` for real-time state updates.
Clients connect with optional token authentication via query parameter (`/ws?token=...`).
On connection, clients receive an initial state snapshot with all sessions.
The `StateBroadcaster` uses tokio broadcast channels to push state changes to
all connected clients. Supported event types: `initial_state`, `session_update`,
`session_created`, `session_deleted`, and `sessions_refresh`.

## Storage

Session state is stored in `~/.config/gralph/state.json` with a lock file
at `~/.config/gralph/state.lock` (or a lock dir fallback). Loop logs are
written to `.gralph/<session>.log` inside the target project directory.

## Quality Gates

CI workflows live in `.github/workflows/` (notably `ci.yml`). The `frontend`
job runs first, building the React app and uploading assets as an artifact.
The `test` and `coverage` jobs depend on `frontend` and download the built
assets before running Rust tests. Tests and coverage checks are required
before merge; coverage must remain at or above 90%. CI runs
`cargo test --workspace` and `cargo tarpaulin --workspace --fail-under 60
--exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*`.
Release and smoke workflows assume CI is green. The verifier mirrors these
gates, adds static checks, and enforces the review gate before merge. The
verifier can also emit a non-blocking warning when coverage falls below the
soft target configured in `verifier.coverage_warn`.
