# Gralph Architecture

This document captures the high-level structure of gralph. It is a living summary of modules and flow.

## Modules

`src/main.rs` is the thin CLI entrypoint. It delegates to `cli_entrypoint` re-exported from `src/lib.rs` and returns its `ExitCode`.
`src/entrypoint.rs` owns the CLI entrypoint helper that parses args, builds real deps, runs the app, and maps results to exit codes.
`src/lib.rs` exposes `run`, `Deps`, and the entrypoint helper for external callers.
`src/app.rs` owns the `run` entrypoint, dependency seams, command dispatch, and
doctor diagnostics.
`src/auth.rs` implements user authentication with email/password and JWT tokens, plus role-based access control (RBAC). `User` stores user data with argon2-hashed passwords and role (Admin, Developer, Viewer). `UserStore` provides in-memory user storage with email uniqueness. `JwtConfig` configures token expiry times. `AccessTokenClaims` and `RefreshTokenClaims` define JWT payloads. `RefreshTokenStore` tracks token families for rotation, detecting token reuse attacks. `RateLimiter` protects auth endpoints with configurable request limits and lockout periods. `AuthService` combines all auth functionality: registration, login, token refresh with rotation, logout, and token validation. `Permission` enum defines granular permissions for session, task, user, org, and system resources. `UserRole` provides `permissions()`, `has_permission()`, `has_all_permissions()`, `has_any_permission()`, and `can_assign_role()` methods for role-based authorization. `RbacMiddleware` provides static methods `require_permission()`, `require_all_permissions()`, `require_any_permission()`, `require_role()`, and convenience methods `require_read_access()`, `require_write_access()`, `require_admin_access()` for protecting API endpoints. Admin has full access, Developer can execute sessions and modify resources, Viewer has read-only access.
`src/audit.rs` implements audit logging for all authenticated actions. `AuditAction` enum defines 22 action types covering auth (Login, Logout, Register, TokenRefresh, TokenRevoke), sessions (SessionCreate, SessionStart, SessionStop, SessionDelete, SessionResume, SessionView, SessionLogsView), tasks (TaskCreate, TaskUpdate, TaskDelete, TaskStatusChange, TaskView), and admin operations (UserCreate, UserUpdate, UserDelete, RoleChange, PermissionChange). `AuditOutcome` enum tracks Success, Failure, and Denied results. `AuditEntry` records id, timestamp, actor_id, actor_email, client_ip, action, resource_type, resource_id, outcome, details, and user_agent. `AuditLogConfig` configures max_entries and max_age_secs for retention policy. `AuditQuery` supports filtering by actor_id, action, resource_type, resource_id, outcome, time range (from_timestamp/to_timestamp), and pagination (limit/offset). `AuditLog` provides thread-safe in-memory storage using `Arc<RwLock<VecDeque>>` with automatic cleanup of old entries on insert. Configuration via environment variables: `GRALPH_AUDIT_MAX_ENTRIES` (default 10000) and `GRALPH_AUDIT_RETENTION_DAYS` (default 30).
`src/oidc.rs` implements OpenID Connect (OIDC) integration for enterprise identity providers. `OidcConfig` stores client credentials (client_id, client_secret), issuer URL, redirect_uri, and endpoint URLs (authorization, token, userinfo, jwks). `OidcClaimMapping` maps OIDC standard claims (sub, email, email_verified, given_name, family_name, name, preferred_username, groups) to user profile fields with configurable group-to-role mapping. `OidcDiscoveryDocument` represents the OIDC provider metadata from the .well-known/openid-configuration endpoint. `OidcAuthRequest` tracks authorization requests with state parameter for CSRF protection, nonce for ID token validation, and optional PKCE code_verifier. `IdTokenClaims` represents decoded JWT claims including iss, sub, aud (supporting single or array), exp, iat, nonce, and standard profile claims. `OidcTokenResponse` contains access_token, id_token, refresh_token, expires_in, and scope from the token endpoint. `OidcUserInfo` represents the userinfo endpoint response. `OidcUserProfile` extracts user data mapped to local account fields. `OidcClient` implements the authorization code flow with PKCE support (S256 challenge method), state/nonce validation, ID token validation (issuer, audience, expiration, nonce), and user profile extraction. `TokenRequestParams` builds form-encoded token exchange requests. `OidcAuthService` combines OidcClient with UserStore for automatic user provisioning from OIDC claims and JWT session creation after successful authentication. Configuration via environment variables: `GRALPH_OIDC_CLIENT_ID`, `GRALPH_OIDC_CLIENT_SECRET`, `GRALPH_OIDC_ISSUER`, `GRALPH_OIDC_REDIRECT_URI`.
`src/rate_limit.rs` implements token bucket rate limiting middleware for protecting authentication and API endpoints. `TokenBucketConfig` defines capacity (max tokens) and refill_rate (tokens per second) for configurable rate limits. `TokenBucket` implements the token bucket algorithm with `try_acquire()` for atomic token consumption and automatic refill based on elapsed time. `EndpointGroup` enum categorizes endpoints into Auth (stricter limits for authentication), Api (standard API limits), WebSocket (connection limits), and Status (health check exemptions). `endpoint_group_for_path()` routes requests to appropriate rate limit buckets based on URL path. `RateLimitMiddlewareConfig` provides global settings including enabled flag, trust_proxy for X-Forwarded-For header support, and per-group `TokenBucketConfig`. `RateLimitMiddleware` maintains per-client token buckets using `Arc<RwLock<HashMap<BucketKey, TokenBucket>>>` where `BucketKey` combines client IP and endpoint group. `check_rate_limit()` returns `RateLimitResult` with allowed flag, remaining tokens, reset timestamp, and retry_after seconds for 429 responses. Rate-limited responses include standard headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset, and Retry-After. Configuration via environment variables: `GRALPH_RATE_LIMIT_ENABLED` (default true), `GRALPH_RATE_LIMIT_AUTH_CAPACITY` (default 10), `GRALPH_RATE_LIMIT_AUTH_REFILL_RATE` (default 1.0), `GRALPH_RATE_LIMIT_API_CAPACITY` (default 100), `GRALPH_RATE_LIMIT_API_REFILL_RATE` (default 10.0), `GRALPH_RATE_LIMIT_TRUST_PROXY` (default false).
`src/oauth2.rs` implements OAuth2 social login for GitHub, Google, and GitLab providers with account linking support. `OAuth2Provider` enum defines supported providers (GitHub, Google, GitLab) with `Display` and `FromStr` implementations. `OAuth2ProviderConfig` stores provider-specific settings (client_id, client_secret, redirect_uri, authorization/token/profile endpoints, scopes) with factory methods `github()`, `google()`, `gitlab()`, and `gitlab_with_host()` for self-hosted GitLab. `OAuth2Config` manages multi-provider configuration with `with_provider()` builder, `from_env()` for environment-based setup. Configuration via environment variables: `GRALPH_OAUTH2_GITHUB_CLIENT_ID`, `GRALPH_OAUTH2_GITHUB_CLIENT_SECRET`, `GRALPH_OAUTH2_GITHUB_REDIRECT_URI`, `GRALPH_OAUTH2_GOOGLE_*`, `GRALPH_OAUTH2_GITLAB_*`, `GRALPH_OAUTH2_GITLAB_HOST`. `OAuth2AuthRequest` tracks authorization requests with state parameter, provider type, optional redirect URL, and optional `link_to_user_id` for account linking flows. `OAuth2UserProfile` provides unified profile extraction from provider-specific responses (`GitHubUserProfile`, `GoogleUserProfile`, `GitLabUserProfile`) with conversion methods `from_github()`, `from_google()`, `from_gitlab()`. `LinkedAccount` stores linked provider account information (provider, provider_user_id, email, username, linked_at). `LinkedAccountStore` manages account linking with `link_account()`, `unlink_account()`, `find_user_by_provider()`, and `is_linked()` methods, maintaining both user-to-accounts and provider-to-user indices for bidirectional lookup. `OAuth2Client` implements the authorization code flow with `create_authorization_url()`, `create_linking_url()`, `validate_callback()`, and `build_token_request()` methods. `OAuth2TokenRequestParams` builds form-encoded token exchange requests. `OAuth2AuthService` combines OAuth2Client with UserStore and LinkedAccountStore for complete authentication with `start_auth()`, `start_linking()`, `complete_auth()`, `unlink_account()`, and `get_linked_accounts()` methods. Authentication flow supports three scenarios: login via linked account, automatic linking for existing email, and new user provisioning with account linking.
`src/totp.rs` implements two-factor authentication with TOTP (RFC 6238) and backup codes. `TwoFactorStatus` enum tracks Disabled, Pending (awaiting first code verification), and Enabled states. `TwoFactorData` stores per-user 2FA configuration including TOTP secret (base32 encoded), hashed backup codes, recovery email, and enabled timestamp. `TwoFactorSetupResult` returns the base32 secret, otpauth:// QR code URL for authenticator apps, and plaintext backup codes (shown only once). `TwoFactorStore` provides thread-safe in-memory storage for 2FA data and recovery tokens using `Arc<RwLock<HashMap>>`. `RecoveryToken` enables email-based recovery with token, user_id, expiration (15 minutes), and used flag. `TwoFactorRateLimiter` protects 2FA verification with configurable max attempts (default 5), window duration (default 5 minutes), and lockout period (default 15 minutes). `TwoFactorService` combines all 2FA functionality: `setup()` generates TOTP secret and backup codes, `confirm_setup()` verifies first code to enable 2FA, `verify_code()` validates TOTP codes with ±1 period tolerance for clock skew, `verify_backup_code()` validates and consumes backup codes, `regenerate_backup_codes()` creates new codes (requires valid TOTP), `initiate_recovery()` generates email recovery token, `complete_recovery()` disables 2FA via token, `disable()` turns off 2FA (requires valid code), and `update_recovery_email()` changes recovery address. TOTP implementation uses HMAC-SHA256 with 6-digit codes and 30-second periods. Backup codes are 8 alphanumeric characters (excluding ambiguous I/O/0/1), SHA256 hashed for storage. `TwoFactorError` enum covers NotEnabled, AlreadyEnabled, SetupNotPending, InvalidCode, InvalidBackupCode, RateLimited, InvalidRecoveryToken, RecoveryTokenUsed, NoRecoveryEmail, and InternalError cases.
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
`src/verifier.rs` implements the verifier pipeline helpers for tests, coverage, static checks, PR creation, and review gating. Multi-agent verification is supported via `WorktreeVerificationResult` for individual worktree outcomes, `AggregatedVerificationResult` for combined results, and `run_multi_agent_verifier_pipeline()` for orchestrating verification across multiple worktrees. Coverage is merged across worktrees by summing covered and total lines via `aggregate_coverage_results()`. Test results are aggregated via `aggregate_test_results()`. The `merge_worktree_changes()` function combines multiple worktree branches into a single PR branch for unified review.
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
`frontend/src/components/AgentOrchestrationDashboard.tsx` displays real-time agent orchestration visualization with task flow diagram, agent nodes, dependency edges, and animated status transitions. The dashboard shows agent status (idle/working/failed), specialization (general/code-gen/testing/review/documentation), current task assignments, and task dependencies. Layout uses SVG with topological sort for task positioning and supports up to 20 agents with performant rendering.
`frontend/src/components/AdminUserManagement.tsx` provides admin user management UI with paginated user list, search by email, role dropdown per user, bulk role assignment, and delete functionality. Admin-only access enforced via backend RBAC checks.
`frontend/src/hooks/useLocalStorage.ts` syncs React state with localStorage for persistence across sessions.
`frontend/src/hooks/useAdminUsers.ts` provides REST API hooks for admin user management including user list, search, role updates, bulk role updates, and user deletion.
`frontend/src/hooks/useLogs.ts` provides REST API hooks for fetching session logs with pagination support.
`frontend/src/hooks/useOrchestration.ts` provides REST API hooks for fetching agent orchestration state with polling support and auto-fetch options.
`frontend/src/types/session.ts` defines TypeScript types matching backend API responses, including Task, LogsResponse, Agent, TaskNode, OrchestrationState, and OrchestrationResponse types.
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
