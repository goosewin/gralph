# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Verification Notes

When adding entries under [Unreleased], include a single-line verification note:
Verification: Tests=<command|not-run>; Coverage=<percent> (>= 70%);
CI=<status/link>; PR=<link if final PRD task>

## [Unreleased]

### Added

- MC-22 Add Permission enum with session, task, user, org, and system permission types.
- MC-22 Add UserRole.permissions() returning all permissions granted to Admin, Developer, or Viewer roles.
- MC-22 Add UserRole.has_permission() for checking single permission on a role.
- MC-22 Add UserRole.has_all_permissions() for checking multiple permissions are all present.
- MC-22 Add UserRole.has_any_permission() for checking at least one permission is present.
- MC-22 Add UserRole.can_assign_role() for role assignment authorization (Admin can assign any, Developer can assign Viewer only).
- MC-22 Add RbacError enum with PermissionDenied, InsufficientRole, CannotAssignRole, and MissingPermissions variants.
- MC-22 Add RbacMiddleware with require_permission(), require_all_permissions(), require_any_permission() for permission checks.
- MC-22 Add RbacMiddleware.require_role() for role level checks (Admin > Developer > Viewer).
- MC-22 Add RbacMiddleware.can_assign_role() for validating role assignment authorization.
- MC-22 Add RbacMiddleware convenience methods: require_read_access(), require_write_access(), require_admin_access().
- MC-22 Add 27 new tests for RBAC permission enforcement covering all roles and permission scenarios.
- MC-21 Add user authentication module (`src/auth.rs`) with email/password registration and JWT tokens.
- MC-21 Add User struct with argon2 password hashing and role-based access (Admin, Developer, Viewer).
- MC-21 Add JwtConfig for configurable access token (15 min default) and refresh token (7 days default) expiry.
- MC-21 Add AccessTokenClaims and RefreshTokenClaims for JWT token payloads.
- MC-21 Add TokenPair struct for combined access and refresh token responses.
- MC-21 Add UserStore for in-memory user storage with email uniqueness validation.
- MC-21 Add RefreshTokenStore for refresh token family tracking and rotation support.
- MC-21 Add RateLimiter with configurable max requests, window duration, and lockout period.
- MC-21 Add AuthService combining user store, refresh store, rate limiter, and JWT operations.
- MC-21 Add registration endpoint POST /auth/register with password strength validation.
- MC-21 Add login endpoint POST /auth/login returning JWT access and refresh tokens.
- MC-21 Add refresh endpoint POST /auth/refresh with token rotation (old tokens invalidated).
- MC-21 Add logout endpoint POST /auth/logout revoking entire token family.
- MC-21 Add me endpoint GET /auth/me returning current user info (requires valid access token).
- MC-21 Add check_jwt_auth helper for JWT-based route protection.
- MC-21 Add auth error handling with appropriate HTTP status codes (401, 409, 429).
- MC-21 Add rate limiting on all auth endpoints with Retry-After header on 429 responses.
- MC-21 Add argon2, jsonwebtoken, uuid, and rand dependencies for auth functionality.
- MC-21 Add 48 new tests for authentication module covering password hashing, JWT tokens, rate limiting, and auth flows.
- MC-20 Add AgentOrchestrationDashboard component for real-time agent orchestration visualization.
- MC-20 Add agent nodes display showing id, status, specialization, and current task.
- MC-20 Add task flow visualization with animated transitions for status changes.
- MC-20 Add dependency edge visualization using SVG curves with directional arrows.
- MC-20 Add assignment edges showing agent-to-task connections with active state highlighting.
- MC-20 Add useOrchestration hook for fetching orchestration data with polling support.
- MC-20 Add GET /orchestration API endpoint returning agents, tasks, and queue statistics.
- MC-20 Add Agent and TaskNode TypeScript types for orchestration state.
- MC-20 Add OrchestrationState and OrchestrationResponse types for API responses.
- MC-20 Add 'orchestration' route to sidebar navigation with agent count badge.
- MC-20 Add orchestration dashboard stats showing agent counts (idle/working/failed) and task counts.
- MC-20 Add legend component for agent status and task status color coding.
- MC-20 Add CSS styling for orchestration visualization with dark mode support.
- MC-20 Add reduced motion support for orchestration animations.
- MC-20 Add performance optimization with memoized position calculations scaling to 20 agents.
- MC-20 Add 22 new tests for AgentOrchestrationDashboard component.
- MC-20 Add 12 new tests for useOrchestration hook.
- MC-19 Add WorktreeVerificationResult for individual worktree test and coverage results.
- MC-19 Add AggregatedVerificationResult for combined multi-worktree verification results with is_success(), worktree_count(), and summary() methods.
- MC-19 Add MultiAgentVerifierConfig for configuring multi-agent verifier pipeline with test/coverage commands, thresholds, and PR settings.
- MC-19 Add run_worktree_verification() for running tests and coverage in a single worktree.
- MC-19 Add aggregate_coverage_results() for merging coverage across worktrees by summing covered/total lines.
- MC-19 Add aggregate_test_results() for combining test pass/fail status across worktrees.
- MC-19 Add run_multi_agent_verifier_pipeline() for orchestrating verification across multiple worktrees with aggregated results.
- MC-19 Add merge_worktree_changes() for merging multiple worktree branches into a single PR branch.
- MC-19 Add get_worktree_branch() for retrieving the branch name of a worktree.
- MC-19 Add extract_line_coverage_stats() and extract_fraction_from_line() for parsing coverage line counts.
- MC-19 Add MultiAgentVerifierStats for verification statistics from aggregated results.
- MC-19 Add 30 new tests for multi-agent verifier covering aggregation, result handling, and error cases.
- MC-18 Add RetryConfig for configurable retry behavior with exponential backoff and jitter.
- MC-18 Add RetryPolicy for managing retry attempts with configurable max retries.
- MC-18 Add CircuitBreakerConfig for configurable circuit breaker thresholds and timeouts.
- MC-18 Add CircuitBreaker state machine with Closed, Open, and HalfOpen states.
- MC-18 Add CircuitState enum and state transition logic for cascade failure prevention.
- MC-18 Add ResilientExecutor combining retry policy with circuit breaker for comprehensive failure handling.
- MC-18 Add RetryStats and CircuitBreakerStats for monitoring retry and circuit breaker behavior.
- MC-18 Add exponential backoff with jitter calculation in RetryConfig.calculate_delay().
- MC-18 Add windowed failure counting in CircuitBreaker for accurate threshold detection.
- MC-18 Add 43 new tests for retry policy, circuit breaker, and resilient executor.
- MC-17 Add AgentMessageChannel for inter-agent communication with per-agent inboxes and message queuing.
- MC-17 Add AgentMessage struct with id, from, to, message_type, timestamp, and correlation_id fields.
- MC-17 Add AgentMessageType enum with HandoffRequest, HandoffAck, TaskResult, Ping, Pong, CancelRequest, and Custom variants.
- MC-17 Add MessageChannelConfig for configurable inbox size, delivery timeout, ping timeout, and message TTL.
- MC-17 Add MessageChannelError for agent not found, inbox full, delivery timeout, and channel closed errors.
- MC-17 Add HandoffCoordinator for managing task transitions between specialized agents.
- MC-17 Add PendingHandoff and CompletedHandoff structs for tracking handoff state and history.
- MC-17 Add HandoffResult enum with Accepted, Rejected, TimedOut, and NoAgentAvailable variants.
- MC-17 Add HandoffError for handoff-specific error handling.
- MC-17 Add AgentTimeoutTracker for detecting stuck or unresponsive agents with configurable thresholds.
- MC-17 Add TimeoutState enum (Active, Warning, Stuck) and AgentTimeoutStatus for agent timeout monitoring.
- MC-17 Add 60 new tests for inter-agent communication covering message channel, handoffs, and timeouts.
- MC-16 Add AgentSpecialization enum with General, CodeGen, Testing, Review, and Documentation types.
- MC-16 Add TaskType enum for task classification with infer_from_content() keyword analysis.
- MC-16 Add AgentSpecializationConfig for per-agent specialization settings with fallback options.
- MC-16 Add SpecializationRouter for task routing based on agent specializations.
- MC-16 Add Coordinator.spawn_specialized_agent() for creating agents with specific specializations.
- MC-16 Add Coordinator.spawn_specialized_agent_with_worktree() for specialized agents with worktrees.
- MC-16 Add Coordinator.assign_task_by_specialization() for routing tasks to best matching agent.
- MC-16 Add Coordinator.find_best_agent_for_task_type() for optimal agent selection.
- MC-16 Add Coordinator.get_idle_agents_by_specialization() and specialization_counts() helpers.
- MC-16 Add Agent.with_specialization() builder method.
- MC-16 Add 51 new tests for agent specialization covering routing, fallback, and task inference.
- MC-15 Add ConflictDetector for detecting overlapping changes from parallel agents.
- MC-15 Add ConflictReport struct for aggregated conflict information with severity tracking.
- MC-15 Add FileConflict struct for per-file conflict details with line-level change tracking.
- MC-15 Add ConflictResolutionStrategy enum with Manual, FirstWins, LastWins, and MergeNonOverlapping options.
- MC-15 Add ConflictSeverity enum (Low, Medium, High, Critical) for conflict prioritization.
- MC-15 Add ConflictDetectionConfig for configurable resolution strategy, auto-resolve, and ignore patterns.
- MC-15 Add ConflictDetector.detect_conflicts() for pairwise worktree conflict analysis using git diff.
- MC-15 Add ConflictDetector.detect_conflicts_multi() for detecting conflicts across multiple worktrees.
- MC-15 Add ConflictDetector.resolve() with automatic resolution for non-overlapping conflicts.
- MC-15 Add ConflictDetectionError for git errors, I/O errors, and resolution failures.
- MC-15 Add LineChange and ChangeType for tracking line-level modifications.
- MC-15 Add 34 new tests for conflict detection covering reports, detection config, severity, and resolution.
- MC-14 Add LoadBalancer struct for distributing tasks based on agent capacity and backend availability.
- MC-14 Add LoadBalanceStrategy enum with RoundRobin, Weighted, and LeastLoaded distribution strategies.
- MC-14 Add HealthStatus enum (Healthy, Unhealthy, Unknown) for agent health tracking.
- MC-14 Add AgentMetadata struct tracking weight, health status, consecutive failures, and task metrics.
- MC-14 Add LoadBalancerConfig for configurable strategy, max failures, health check interval, and auto-remove.
- MC-14 Add LoadBalancer.register_agent() and register_agent_with_weight() for weighted agent registration.
- MC-14 Add LoadBalancer.select_agent() implementing strategy-based task distribution.
- MC-14 Add LoadBalancer.health_check() for recording health check results and removing unhealthy agents.
- MC-14 Add LoadBalancer.agents_needing_health_check() for identifying agents due for health checks.
- MC-14 Add LoadBalancerStats for load balancer state metrics.
- MC-14 Add 28 new tests for LoadBalancer covering round-robin, weighted, least-loaded, health checks, and integration scenarios.
- MC-13 Add DependencyGraph struct for task dependency analysis with graph representation.
- MC-13 Add DependencyGraph.from_prd() for extracting task relationships from PRD content.
- MC-13 Add DependencyGraph.get_execution_levels() for identifying parallel task groups using topological sort.
- MC-13 Add DependencyGraph.get_ready_tasks() and schedule_tasks() for concurrent task scheduling.
- MC-13 Add DependencyGraph.validate_no_cycles() for cycle detection with error reporting.
- MC-13 Add DependencyGraph.max_parallelism() and critical_path_length() for scheduling optimization.
- MC-13 Add DependencyGraphStats for graph statistics (task count, root count, parallelism, critical path).
- MC-13 Add 30 new unit tests for DependencyGraph covering edge cases, cycles, and parallel patterns.
- MC-12 Add AgentWorktreeManager for thread-safe creation and cleanup of isolated agent worktrees.
- MC-12 Add Coordinator.with_worktree_manager() for creating coordinators with worktree isolation.
- MC-12 Add Coordinator.spawn_agent_with_isolated_worktree() to spawn agents with unique worktrees.
- MC-12 Add Coordinator.remove_agent() for cleanup of agent and associated worktree.
- MC-12 Add Coordinator.cleanup_all_worktrees() for shutdown cleanup.
- MC-12 Add WorktreeError enum with variants for git, io, already exists, not found, not a repository, and no commits errors.
- MC-12 Add 23 new tests for AgentWorktreeManager and Coordinator worktree integration.
- MC-11 Add multi-agent coordinator module (`src/coordinator.rs`) for parallel task orchestration.
- MC-11 Add Coordinator struct managing agent pool with configurable max_agents limit.
- MC-11 Add WorkQueue for task distribution with dependency-aware scheduling.
- MC-11 Add Agent struct with status tracking (Idle/Working/Failed) and worktree path support.
- MC-11 Add TaskNode struct with id, content, and dependencies for task graph representation.
- MC-11 Add topological_sort for ordering tasks by dependencies with cycle detection.
- MC-11 Add find_independent_tasks to identify parallelizable tasks with satisfied dependencies.
- MC-11 Add tasks_from_prd parser to extract unchecked task blocks with their dependencies.
- MC-11 Add parse_task_dependencies and parse_task_id helpers for PRD content parsing.
- MC-11 Add CoordinatorError for no available agents, task not found, dependency cycles, worktree and agent failures.
- MC-11 Add 31 unit tests for coordinator module covering agent management, task distribution, and dependency logic.
- MC-10 Add SessionLogViewer component with syntax highlighting for log line types (error, warning, success, info).
- MC-10 Add toggleable auto-scroll that follows new log content and disables when user scrolls up.
- MC-10 Add search within logs with keyboard navigation (Enter/Shift+Enter) and highlighted matches.
- MC-10 Add virtualization for performance with 100k+ lines using calculated visible range and overscan.
- MC-10 Add useLogs hook for fetching logs via REST API with pagination support (offset/limit).
- MC-10 Add GET /logs/:session endpoint returning log lines with raw/processed toggle.
- MC-10 Add 'logs' route to sidebar navigation and mobile tab nav for viewing session logs.
- MC-10 Add View Logs button to SessionCard for quick navigation to log viewer.
- MC-10 Add 41 new tests for SessionLogViewer component and useLogs hook.
- MC-9 Add Sidebar component with collapsible sections showing all main navigation items.
- MC-9 Add useLocalStorage hook for persisting collapse state across browser sessions.
- MC-9 Add active route highlighting with visual indicator on sidebar items.
- MC-9 Add smooth transition animations (under 300ms) for collapse/expand and route transitions.
- MC-9 Add Settings page accessible via sidebar navigation.
- MC-9 Add 42 new tests for Sidebar component and useLocalStorage hook.
- MC-8 Add PWA manifest.json with app icons and metadata for home screen installation.
- MC-8 Add service worker (sw.js) for caching static assets with cache-first strategy.
- MC-8 Add OfflineIndicator component displaying banner when user is offline.
- MC-8 Add InstallPrompt component with install button shown on mobile when eligible.
- MC-8 Add useServiceWorker hook for service worker registration and update handling.
- MC-8 Add PWA meta tags in index.html for theme color, mobile web app support, and manifest link.
- MC-8 Add SVG icons (192x192, 512x512, maskable) for PWA installation.
- MC-8 Add 33 new tests for OfflineIndicator, InstallPrompt, and useServiceWorker.
- MC-7 Add responsive mobile-first layout with breakpoints at 640px, 768px, and 1024px.
- MC-7 Add HamburgerMenu component for mobile navigation with animated toggle.
- MC-7 Add useTouchSwipe hook for touch gesture detection on mobile devices.
- MC-7 Add useMediaQuery and useBreakpoints hooks for responsive component logic.
- MC-7 Add touch swipe gestures on Kanban board to move tasks between columns.
- MC-7 Add mobile-optimized CSS with touch-friendly targets and reduced motion support.
- MC-7 Add 35 new tests for HamburgerMenu, useTouchSwipe, and useMediaQuery.
- MC-6 Add dark mode support with theme toggle in header.
- MC-6 Add useTheme hook for theme state management with localStorage persistence.
- MC-6 Add system preference detection (prefers-color-scheme) for automatic theme selection.
- MC-6 Add CSS variables for light/dark theme colors throughout UI.
- MC-6 Add ThemeToggle component with light/dark/system options.
- MC-6 Add 23 new tests for ThemeToggle component and useTheme hook.
- MC-5 Add KanbanBoard component for PRD task visualization with drag-and-drop status transitions.
- MC-5 Add TaskCard component displaying task ID, title, status, and definition of done.
- MC-5 Add KanbanColumn component for grouping tasks by pending/in_progress/completed status.
- MC-5 Add useTasks hook for fetching tasks and updating task status via API.
- MC-5 Add Task TypeScript types and TaskStatus enum for Kanban board.
- MC-5 Add GET /tasks/:session and PUT /tasks/:session/:task_id/status API endpoints.
- MC-5 Add prd_list_tasks and prd_update_task_status functions for PRD task parsing and status updates.
- MC-5 Add keyboard navigation (arrow keys) for moving tasks between columns.
- MC-5 Add screen reader announcements for task movements with aria-live region.
- MC-5 Add 48 new component tests for TaskCard, KanbanColumn, and KanbanBoard.
- MC-4 Add SessionDashboard component displaying all sessions with real-time updates via WebSocket.
- MC-4 Add StatusBadge component with running/stopped/failed/completed/stale status indicators.
- MC-4 Add SessionCard component with session details, progress bar, and stop functionality.
- MC-4 Add useWebSocket hook for real-time WebSocket connection with reconnection handling.
- MC-4 Add useSessions hook for REST API session fetching and stop operations.
- MC-4 Add Session TypeScript types matching backend API response.
- MC-4 Add Vitest testing setup with React Testing Library and 34 component tests.
- MC-3 Initialize TypeScript/React frontend workspace with Vite build system.
- MC-3 Add frontend package.json with React 18, TypeScript 5, and Vite 5 dependencies.
- MC-3 Add TypeScript configuration with strict mode enabled.
- MC-3 Add ESLint configuration with react-hooks and react-refresh plugins.
- MC-3 Add Vite configuration to build frontend to assets directory.
- MC-3 Update CI workflow to build frontend before Rust tests.
- MC-1 Add static file serving to Axum server from embedded assets directory.
- MC-1 Add health check endpoint (`/health`) that returns 200 OK with healthy status.
- MC-1 Add rust-embed dependency for bundling static assets at compile time.
- MC-2 Add WebSocket endpoint (`/ws`) for real-time state broadcasting to connected clients.
- MC-2 Add StateBroadcaster for broadcasting session state changes via tokio broadcast channels.
- MC-2 Add WebSocket authentication via query parameter token validation.
- MC-2 Add tokio-tungstenite and futures-util dependencies for WebSocket support.
- PRD-7 Add retry loop for PRD generation with configurable max retries (`prd_create_max_retries`).
- PRD-7 Add PRD specification document (`docs/PRD_SPEC.md`) defining validation rules for LLM prompt injection.

### Changed

### Fixed

### Verification

- Verification: Tests=cargo test --workspace; Coverage=not-run (>= 70%); CI=not-run; PR=not-opened

## [0.2.5]

### Added

- TMUX-3 Add attach command for tmux sessions.
- COV80-WT-1 Add worktree git helper tests for git_output_in_dir, create_worktree_at, cmd_worktree_finish, and validate_task_id.
- COV90-PRD-1 Add PRD validation and sanitize tests covering missing required fields, Open Questions removal, stray checkbox handling, and validation after sanitize with allowed context.
- COV90-VER-1 Add verifier static file inclusion and ignore pattern tests; add coverage parsing tests for tarpaulin output formats including fail-under, verbose, workspace, zero, and hundred percent cases.
- COV90-FINAL-1 Verified coverage target: tests passed (1359 tests), tarpaulin coverage 84.76% (3716/4384 lines covered) with CI threshold (60%) met.

### Changed

- TMUX-1 Remove --no-tmux flag from CLI and completions; document tmux as required.
- TMUX-2 Require tmux sessions for start and persist tmux session names in state.
- WT-1 Run auto worktree loops from `.worktrees/<unique-worktree>`.
- WT-2 Auto-commit dirty repos before auto worktree creation.
- COV-70-1 Add tmux availability and session collision tests.
- COV-70-3 Add auto worktree skip and path mapping tests.
- COV80-LS-3 Add loop_session failure path tests for notification flows and callbacks.
- COV80-LS-4 Add loop_session dry-run and step execution tests.
- COV80-VER-2 Add verifier static check pipeline and PR creation flow tests.
- COV80-VER-3 Add verifier review gate polling tests for timeout, approval, and merge method.
- COV80-APP-1 Add app CLI dispatch tests for error formatting and subcommand routing.
- COV80-APP-3 Add app doctor and init tests for config errors, backend checks, and scaffolding paths.
- COV80-PRD-2 Add PRD sanitize tests for context filtering and Open Questions removal.
- COV80-PRD-3 Add PRD stack detection tests for framework and tool detection.
- COV80-STATE-1 Add state store edge case tests for StateError Display/source and parse_value edge cases.
- COV80-CONFIG-1 Add config loader edge case tests for ConfigError Display/source, get_user merge precedence, and env override handling.
- COV80-UPDATE-1 Add update module tests for UpdateError Display variants, version parsing, install_release success path, and archive extraction errors.
- COV80-FINAL-1 Verified 80% coverage threshold met (84.74% actual, 3715/4384 lines covered, 1319 tests passed).

### Fixed

### Verification

- Verification: Tests=CI; Coverage=CI (>= 70%); CI=ran; PR=opened

## [0.2.4]

### Added

- UX-1 Add doctor command with local diagnostics checks.
- UX-2 Add cleanup command for stale sessions.
- UX-3 Record raw log paths and expose raw logs.
- UX-4 Add status JSON fields and last error context.
- UX-5 Add dry-run start and step execution.

### Changed

- DOC-1 Record entrypoint refactor and coverage recovery notes in shared docs.
- MD-2 Extract loop and session command handlers into the app loop session module.
- MD-3 Move PRD and init command handlers into the app PRD module.
- MD-4 Move worktree and git helpers into the app worktree module.
- MD-7 Document the run entrypoint and command module layout.
- MR-2 Document the lib entrypoint helper and main delegation flow.
- LS-1 Extract loop setting helpers and add config precedence tests.
- LS-2 Isolate session lifecycle decisions and add transition tests.
- VER-1 Extract verifier parsing helpers and add coverage parsing tests.
- VER-2 Add review gate and check gate decision tests.
- PRD-1 Split PRD sanitize/validate content handling and add sanitize tests.
- UX-6 Make verifier defaults stack-aware for non-Rust stacks.
- UX-7 Require approval by default for verifier auto-merge and document opt-in.
- UX-8 Add update check opt-out and start log/tmux hints.

### Verification

- Verification: Tests=not-run; Coverage=not-run (>= 70%); CI=not-run; PR=not-opened

## [0.2.3]

### Added

- PROMPT-1 Require lower-case conventional commits in the default prompt template.
- VER-1 Add verifier command for tests and coverage gates.
- DOC-1 Document verifier workflow, review gate, and commit conventions.
- COV-1 Expand core loop validation, prompt template fallback, and raw output logging coverage.
- COV-2 Expand state store lock acquisition and io error-path coverage.
- COV-2 Expand PRD validation and sanitization coverage.
- COV-2 Expand state store env override coverage.
- COV-2 Add state store cleanup and parse_value edge case tests.
- COV-STATE-1 Add state store lock path, cleanup error-path, and parse_value edge-case tests.
- COV-STATE-1 Expand state store tmp write collision, malformed cleanup, and lock timeout tests.
- COV-3 Add OpenCode backend run_iteration argument and env coverage.
- COV-3 Expand state store edge-case coverage.
- COV-3 Expand PRD validation and sanitization coverage.
- COV-3 Add PRD base override and sanitize proptests.
- COV-3 Add PRD task block parsing and sanitization invariants.
- COV-PRD-1 Expand PRD validation and sanitize invariants.
- COV-PRD-1 Add PRD sanitize fallback and context validation proptests.
- COV-PRD-1 Add PRD sanitize context filtering and stray unchecked invariants.
- COV-PRD-1 Add canonicalize fallback and absolute context sanitize tests.
- COV-4 Expand verifier helper coverage for auto-run defaults, command parsing, PR base resolution, template lookup, static checks, and review gate parsing.
- COV-VER-1 Expand verifier parsing edge cases for empty commands and check rollups.
- COV-VER-1 Add verifier review gate parsing and static check invalid config tests.
- COV-5 Add Codex backend run_iteration flag validation tests.
- COV-5 Expand config normalization and override tests.
- COV-5 Cover config env override conflicts and empty values.
- COV-5 Add config path edge-case and key existence tests.
- COV-5 Expand backend utility PATH and stream coverage.
- COV-6 Expand CLI helper coverage in main.
- COV-6 Expand Claude backend parsing and argument tests.
- COV-7 Expand server auth and CORS error-path coverage.
- COV-7 Expand OpenCode backend env flags, argument ordering, and error-path tests.
- COV-8 Expand notification formatting helper and payload coverage.
- COV-8 Expand Gemini backend tests for headless flags and parse_text errors.
- COV-BACKEND-GEMINI-1 Add Gemini adapter edge-case tests.
- COV-9 Expand backend module utility coverage.
- COV-9 Add notify validation and failure formatting tests.
- COV-9 Cover update version env and archive PATH error cases.
- COV-9 Expand Codex backend tests for flag ordering and error paths.
- COV-10 Add property-based tests for task parsing invariants.
- COV-10 Expand Claude backend parsing and install tests.
- COV-10 Expand Claude backend parsing fallback coverage.
- COV-10 Expand backend helper streaming and PATH scanning coverage.
- COV-10 Expand config env precedence and list rendering coverage.
- COV-11 Expand backend module error formatting coverage.
- COV-11 Expand Claude adapter parsing coverage for invalid stream entries.
- COV-11 Expand OpenCode backend run_iteration tests.
- COV-11 Expand OpenCode backend command ordering and env coverage.
- COV-11 Expand server CORS, auth, and session enrichment tests.
- COV-12 Expand Gemini backend command and error coverage.
- COV-12 Expand Claude adapter error-path coverage.
- COV-BACKEND-CLAUDE-1 Cover Claude adapter parse fallbacks and malformed content.
- COV-BACKEND-CLAUDE-1 Add Claude stream entry tests for missing type handling.
- COV-12 Expand OpenCode adapter coverage.
- COV-12 Expand notification formatting boundary and timeout default tests.
- COV-13 Expand OpenCode adapter error-path coverage.
- COV-13 Expand Codex backend command and error coverage.
- COV-13 Expand Gemini adapter error-path coverage.
- COV-13 Add CLI helper tests for session naming fallbacks.
- COV-14 Expand Gemini adapter error-path coverage.
- COV-14 Expand update/install error-path coverage.
- COV-14 Add property-based tests for task block termination and unchecked parsing.
- COV-14 Expand Codex adapter coverage for parse_text and PATH edge cases.
- COV-15 Expand Codex adapter error-path coverage.
- COV-15 Add verifier command parsing and review gate tests.
- COV-15 Expand task parsing edge-case tests.
- COV-15 Add CRLF and tabbed task parsing invariants.
- COV-15 Add task parsing heading and CRLF unchecked tests.
- COV-15 Expand env_lock contention and panic recovery tests.
- COV-16 Add env_lock safety tests for test_support helpers.
- COV-TEST-SUPPORT-1 Expand env lock contention and poison recovery tests.
- COV-TESTSUPPORT-1 Expand env_lock contention release and restore sequencing tests.
- COV-TEST-SUPPORT-1 Add env_lock sequential drop and panic scope restore tests.
- COV-16 Expand update workflow parsing and install error coverage.
- COV-17 Expand Claude backend parsing and failure path coverage.
- COV-18 Expand Codex backend installation and error coverage.
- COV-19 Expand Gemini backend command and error coverage.
- COV-20 Expand OpenCode backend env and failure coverage.
- COV-21 Expand backend module utility coverage.
- COV-22 Expand config loader and override coverage.
- COV-23 Expand core loop validation and completion coverage.
- COV-24 Expand main CLI helper coverage.
- COV-MAIN-1 Cover CLI and worktree error paths.
- COV-MAIN-1 Add CLI helper tests for auto worktree defaults and timestamp slug.
- COV-MAIN-1 Add CLI helper tests for bool parsing, task ID errors, and branch uniqueness.
- COV-25 Expand notification formatting and HTTP error coverage.
- COV-26 Expand PRD sanitization and stack summary coverage.
- COV-27 Expand server auth, CORS, and session enrichment coverage.
- COV-28 Expand state store normalization coverage.
- COV-29 Expand task parsing edge coverage.
- COV-TASK-1 Add task parsing boundary tests for CRLF/tabbed separators and H2 termination near-misses.
- COV-30 Expand update parsing and extraction coverage.
- COV-CORE-1 Expand core loop prompt template and retention edge coverage.
- COV-CORE-1 Add core loop parse failure coverage and completion invariants.
- COV-CORE-1 Add property tests for prompt rendering and context normalization.
- COV-CORE-1 Cover prompt template placeholder replacement invariants.
- COV-CORE-1 Cover core loop completion edge cases for zero tasks and trailing whitespace.
- COV90-CORE-1 Add core loop tests for error paths, callbacks, and completion invariants.
- COV90-STATE-1 Add state store recovery and parse_value edge-case tests.
- COV90-STATE-1 Expand state store write failure and cleanup coverage.
- COV90-PRD-1 Add property-based PRD validation and sanitize invariants.
- COV90-PRD-1 Cover PRD absolute context acceptance and fallback selection tests.
- COV90-TASK-1 Add property tests for task parsing boundaries.
- COV90-CONFIG-1 Expand config merge precedence, env compat empty override, and list rendering tests.
- COV90-CI-1 Document soft coverage warning target and staged plan.
- COV90-CI-1 Add soft coverage warning target in verifier config and docs.
- COV90-CI-2 Raise soft coverage warning target to 80 percent (warning-only).
- COV-CI-1 Set initial soft coverage warning target to 65 to 70 percent.
- COV-CI-2 Raise soft coverage warning target to 80 percent and update docs.
- COV90-VER-1 Add verifier parsing, review gate, and gh error handling tests.
- COV90-VERIFIER-1 Expand verifier static check parsing and PR creation error coverage.
- COV90-MAIN-1 Add CLI helper tests for session naming and worktree branch formatting.
- COV90-VERSION-1 Add version constant tests.
- COV90-LIB-1 Add lib crate wiring coverage.
- COV90-SERVER-1 Expand server edge-case coverage for CORS origins, stale sessions, and missing stop targets.
- COV90-BACKEND-MOD-1 Add backend module PATH and streaming tests.
- COV90-BACKEND-CLAUDE-1 Expand Claude backend parsing and error tests.
- COV90-BACKEND-CLAUDE-1 Add Claude backend ordering and fallback tests.
- COV90-BACKEND-OPENCODE-1 Expand OpenCode backend env flag ordering and no-flag prompt tests.
- COV90-BACKEND-GEMINI-1 Expand Gemini backend command and error tests.
- COV90-BACKEND-CODEX-1 Expand Codex backend flag ordering, model skipping, and exit propagation tests.
- COV90-NOTIFY-1 Add notify payload formatting, duration edge-case, and HTTP status tests.
- COV90-NOTIFY-1 Add notify_failed unknown and empty reason payload coverage.
- COV90-UPDATE-1 Add update workflow error path tests.
- COV90-UPDATE-1 Expand update workflow coverage for version formats, tar failures, and permission errors.
- COV90-TESTSUPPORT-1 Add env_lock stress and recovery tests.
- COV90-TESTSUPPORT-1 Verify env_lock restore after guard drop.
- COV90-CLI-1 Add CLI parsing tests for run-loop, verifier defaults, and PRD conflicts.
- COV-9 Expand update check and archive error coverage.
- COV-31 Expand verifier parsing and static check coverage.
- COV-VER-1 Add verifier parsing tests for coverage tokens, review gate parsing, and static checks.
- COV-VERIFIER-1 Cover verifier parsing and static checks.
- COV-3 Expand verifier parsing and gate evaluation coverage.
- COV-5 Expand server session enrichment and stop flow coverage.
- COV-CONFIG-1 Add config normalization, env precedence, list rendering, and value_to_string property tests.
- COV-CONFIG-1 Expand config path ordering, env empty override, and normalize_key lookup property coverage.
- COV-SERVER-1 Expand server CORS mismatch and stale session coverage.
- COV-NOTIFY-1 Expand notification timeout defaults, duration boundaries, unknown reason, and webhook type detection tests.
- COV-UPDATE-1 Expand update workflow error-path coverage for release download overrides and unsupported targets.
- COV-UPDATE-1 Add resolve_install_version tests for empty env values and invalid tags.
- COV-UPDATE-1 Add update version parsing and override trim tests.
- COV-UPDATE-1 Add update platform and extract_archive PATH error message tests.
- COV-BACKEND-MOD-1 Expand backend helper streaming, PATH relative-segment handling, and invalid backend name error coverage.
- COV-BACKEND-CLAUDE-1 Add Claude adapter error-path tests.
- COV-BACKEND-CLAUDE-1 Add Claude adapter argument ordering coverage.
- COV-BACKEND-OPENCODE-1 Expand OpenCode adapter coverage for env flags, arg ordering, model/variant trimming, mixed stdout/stderr capture, and invalid UTF-8 parsing.
- COV-BACKEND-CODEX-1 Expand Codex adapter arg ordering, output path, and error coverage.
- COV-CONFIG-1 Add config lookup proptest and nested sequence rendering tests.
- COV-SERVER-1 Add server CORS invalid origin, stale pid stop, and unreadable task file tests.

### Fixed

- MD-8 Fix verifier config import, public CLI error type, and file reader trait alias.
- WT-1 Skip auto worktree creation on dirty repos and emit explicit skip reasons.
- REF-1 Consolidate shared backend execution helpers.
- REF-2 Unify task block parsing helpers across core and PRD validation.
- REF-3 Centralize config merge precedence and normalize override lookup.
- REF-4 Centralize server auth and error responses.
- REF-5 Reduce duplication in notification payload formatting.
- REF-6 Modularize verifier pipeline helpers into a dedicated module.
- REF-7 Update shared docs and module map for refactor outcomes.
- COV-5 Align verifier coverage command with the 90 percent gate.
- COV-6 Normalize absolute context path comparisons and isolate config env override tests.

### Verification

- Verification: Tests=cargo test --workspace; Coverage=not-run (>= 70%); CI=not-run; PR=not-opened

## [0.2.2]

### Fixed

- Installer: add PATH auto-update for local installs.
- Windows installer: fix Join-Path usage when piping to iex.

### Verification

- Verification: Tests=cargo test --workspace; Coverage=65.49% (>= 70%); CI=not-run; PR=not-opened

## [0.2.1]

### Added

- AW-2 Added auto worktree edge case tests for skip behavior, subdir mapping, and collisions.
- AW-3 Documented auto worktree UX, skip reasons, and Graphite stacking guidance.
- WT-1 Auto-create worktrees for PRD runs with config and CLI controls.
- INIT-1 Added init CLI subcommand and routing.
- UPD-1 Added session-start update check with version parsing.
- UPD-2 Added update subcommand to install release binaries.
- DOC-1 Documented update command, update notice, and regenerated completions.

### Fixed

- START-1 Added session name fallback for dot and root paths.
- INST-1 Hardened installer cleanup and PATH-aware verification.
- LOG-1 Format loop start/finish timestamps and human-readable durations.

### Verification

- Verification: Tests=CI; Coverage=CI (>= 70%); CI=green; PR=not-required

## [0.1.0]

### Added

- INIT-4 Documented init command and updated shell completions.
- Added multi-arch release assets for Linux and macOS.
- Initial public release notes for the gralph CLI.
- T-SERVER-1 Added status endpoint auth and response tests.
- T-SERVER-2 Added stop endpoint behavior tests.
- T-SERVER-3 Added CORS and error response tests.
- T-NOTIFY-2 Added send_webhook HTTP delivery tests for headers and response handling.
- T-NOTIFY-1 Added webhook payload formatting tests for Discord, Slack, and generic webhooks.
- T-BACKEND-3 Added backend registry tests for selection and model listing.
- T-BACKEND-2 Added run-iteration success/failure tests for backend adapters.
- T-BACKEND-1 Added reusable fake CLI helper for backend adapter tests.
- T-STATE-1 Added state error-path tests for corrupted/missing files.
- T-CORE-2 Added core loop execution tests for completion, failure, and max-iteration paths.
- T-CORE-1 Added core task parsing edge-case tests.
- RS-1 Scaffolded Rust project with clap CLI skeleton.
- RS-2 Added Rust config loader with serde_yaml merging and env overrides.
- RS-3 Added Rust state module with file locking and atomic JSON writes.
- RS-4 Added backend trait with Claude CLI implementation and tests.
- RS-5 Added OpenCode, Gemini, and Codex Rust backends with integration test stubs.
- RS-6 Added Rust core loop module with iteration execution and completion checks.
- RS-7 Added Rust webhook notifications with Discord, Slack, and generic payloads via reqwest.
- RS-8 Added Rust PRD validation, sanitization, and stack detection utilities with tests.
- RS-9 Added Rust HTTP status server with bearer auth and CORS handling.
- G-1 Added interactive PRD generator via `gralph prd create`.
- C-1 Added worktree commands to help output and examples.
- C-2 Added worktree command routing and validation.
- C-3 Added worktree create command to scaffold task branches and worktrees.
- C-4 Added worktree finish command to merge task branches and remove worktrees.
- C-5 Added safety checks for dirty git state and missing worktree paths.
- C-6 Documented worktree workflow in README.
- C-7 Recorded Stage C worktree command rationale in DECISIONS.
- A-1 Added PROCESS.md with protocol steps and guardrails.
- A-2 Added ARCHITECTURE.md skeleton with required sections.
- A-3 Documented module map in ARCHITECTURE.md.
- A-4 Added runtime flow and storage details in ARCHITECTURE.md.
- A-5 Added DECISIONS.md with initial shared docs decision.
- A-6 Added RISK_REGISTER.md with context-loss risks and mitigations.
- P-1 Added task block grouping helper for PRD parsing.
- P-2 Added selector for next unchecked task block.
- P-3 Added task block placeholder to prompt rendering.
- P-4 Injected selected task block into iteration prompts.
- P-5 Added tests for task block parsing and fallback behavior.
- P-6 Documented task block format and legacy fallback behavior.
- B-1 Added defaults.context_files to default config.
- B-2 Read context_files in core loop for prompt rendering.
- B-3 Normalized context file list for prompt injection.
- B-4 Injected context files section into the prompt template.
- B-5 Documented defaults.context_files and env override in README.
- B-6 Recorded Stage B context file injection notes and decision.
- D-3 Added strict PRD validation gate for gralph start.
- D-1 Added PRD validation helpers for task block schema checks.
- D-2 Added gralph prd check command with validation errors.
- D-4 Added PRD validation shell tests for invalid cases.
- D-5 Documented PRD validation and strict mode in README.
- D-6 Recorded Stage D validation changes and strict mode rationale.
- E-1 Added example README for self-hosting PRDs.
- E-2 Added Stage P example PRD.
- E-3 Added Stage A example PRD.
- E-4 Added release runner script for example stages.
- E-5 Documented self-hosting workflow in README.
- E-6 Recorded Stage E example and runner rationale in DECISIONS.
- P-EX-2 Updated README task block example to include all required fields.
- RS-10 Wired Rust CLI subcommands with build-time shell completions.
- RS-11 Added Rust tests coverage and CI workflow with coverage threshold.
- RS-12 Documented Rust build/install steps and migration notes.
- RS-13 Updated release workflow to package Rust binaries and completions.
- T-CLI-1 Added CLI unit tests for PRD output resolution and parse validation.
- T-CLI-2 Added tests for PRD template selection and fallback behavior.
- T-CONFIG-1 Added config path precedence tests.

### Fixed

- AW-1 Resolve auto worktree repo roots from target dirs and preserve subdir runs.
- Aligned the Cargo-installed binary name with release assets (`gralph`).
- OSS-1 Removed duplicate introductory text from README.
- OSS-2 Corrected Gemini CLI install hints and README instructions.
- OSS-3 Updated backend model names to real or placeholder values.
- OSS-4 Updated shell completions to match backend model names.
- OSS-5 Verified Codex CLI docs URL and install reference.
- OSS-6 Verified OpenCode CLI docs URL and install reference.
- OSS-7 Verified Claude Code model alias claude-opus-4-5.
- OSS-8 Removed stale platform notes reference in README.
- OSS-9 Removed unused notification helpers and variables.
- OSS-10 Hardened server request handling and tightened CORS defaults.
- OSS-11 Updated backend tests to assert corrected model names.
- OSS-12 Hardened installer path handling and bootstrap checks.
- OSS-13 Aligned CLI backend validation and help text.
- OSS-14 Hardened state locking, atomic writes, and corruption recovery.
- OSS-15 Added simple YAML array parsing and documented supported config lists.
- OSS-16 Hardened core loop completion detection and parse error handling.
- OSS-17 Documented PRD validation rules, sanitization behavior, and stack detection heuristics.
- OSS-18 Reviewed README for CLI reference, model names, and doc accuracy.
- OSS-19 Removed legacy shell artifacts and aligned context defaults with Rust sources.
