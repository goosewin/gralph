# Project Requirements Document: Gralph Mission Control Platform

## Overview

Gralph Mission Control transforms the existing autonomous AI coding loop CLI into the most comprehensive, world-changing development platform ever created. The platform provides a web-based Mission Control UI with real-time dashboards, Kanban task boards, and WebSocket live updates. It introduces multi-agent orchestration for parallel task execution, team collaboration with role-based access control (RBAC), IDE extensions for VS Code, JetBrains, and Neovim, enterprise SSO and billing integrations, AI-powered PRD generation, a community marketplace for extensions and templates, and world-class documentation that makes every developer, AI enthusiast, and newcomer feel empowered to build anything.

The platform maintains full backwards compatibility with the existing CLI while adding a TypeScript/React frontend served by the existing Axum HTTP server infrastructure. All new code requires tests with minimum 90% coverage.

## Vision

Gralph will democratize software development by making autonomous AI-assisted coding accessible to everyone—from seasoned developers to complete beginners. By the end of this transformation, anyone with an idea will be able to turn it into working software using gralph's intuitive interface, intelligent agents, and collaborative features.

## Problem Statement

- Single-user CLI limits adoption in team environments requiring visibility and coordination.
- No real-time monitoring dashboard forces users to rely on terminal logs and polling.
- Sequential task execution prevents parallelization of independent workstreams.
- Lack of IDE integration requires context switching between editor and terminal.
- No enterprise features (SSO, billing, audit logs) blocks adoption by organizations.
- PRD creation requires manual authoring without AI assistance.
- No ecosystem for sharing templates, prompts, or backend adapters.
- Documentation is scattered across markdown files without interactive examples.
- No mobile access for monitoring on the go.
- No analytics to understand loop performance and optimization opportunities.
- No guided onboarding for newcomers to AI-assisted development.

## Solution

Build Mission Control as a layered extension of gralph's existing Axum server infrastructure. The web UI connects via WebSocket for real-time state updates. Multi-agent orchestration spawns parallel backend sessions managed by a coordinator. RBAC enforces permissions at the API layer. IDE extensions communicate via the existing HTTP API. Enterprise integrations plug into authentication and billing middleware. The marketplace indexes community contributions. Documentation uses VitePress with embedded interactive examples. A mobile-responsive PWA enables on-the-go monitoring.

---

## Functional Requirements

### FR-1: Web-Based Mission Control UI

A TypeScript/React single-page application served by the Axum backend provides a real-time dashboard showing all active sessions, their status, iteration counts, and remaining tasks. A Kanban board visualizes PRD tasks as cards with drag-and-drop state transitions. WebSocket connections push state changes to connected clients. The UI is WCAG 2.1 AA accessible with keyboard navigation and screen reader support. Includes dark mode, mobile responsiveness, and customizable layouts.

### FR-2: Multi-Agent Orchestration

A coordinator module manages parallel execution of independent task blocks across multiple backend sessions. Each agent runs in isolation with its own worktree. The coordinator aggregates results, detects conflicts, and schedules dependent tasks after prerequisites complete. Load balancing distributes work across available backend capacity. Agent specialization enables dedicated code-gen, testing, review, and documentation agents.

### FR-3: Team Collaboration with RBAC

User accounts with email/password and OAuth authentication. Role-based access control with roles: Admin, Developer, Viewer. Admins manage users and settings. Developers can start/stop sessions, edit PRDs, and approve merges. Viewers have read-only access. Audit logs track all actions with timestamps and actors. Real-time collaboration features include live cursor presence, commenting, and @mentions.

### FR-4: IDE Extensions

VS Code extension provides sidebar panel showing active sessions, inline task status, and commands to start/stop/attach. JetBrains plugin mirrors functionality for IntelliJ family. Neovim plugin uses Lua with telescope integration for session management. Emacs package via LSP integration. All extensions communicate via the HTTP API with token authentication.

### FR-5: Enterprise SSO and Billing

SAML 2.0 and OIDC SSO integration for enterprise identity providers. Stripe integration for usage-based billing with seat licenses and overage charges. Admin portal for license management, usage dashboards, and invoice history. SOC 2 compliant data handling with encryption at rest and in transit. On-premise deployment option with air-gapped mode support.

### FR-6: AI-Powered PRD Generation

Interactive wizard collects project goals, constraints, and context files. AI generates structured PRD following the template specification. Iterative refinement loop allows user feedback and regeneration. Validation ensures generated PRDs pass all schema checks before saving. Supports all configured backends for generation. Natural language to PRD conversion with smart task breakdown.

### FR-7: Community Marketplace

Registry of community-contributed prompt templates, backend adapters, and PRD examples. Search and filtering by category, rating, and compatibility. One-click installation to local configuration. Contribution workflow with review process and versioning. Hosted at marketplace.gralph.dev. Plugin system for custom backends and integrations.

### FR-8: Documentation Portal

VitePress-based documentation site at docs.gralph.dev. Covers CLI usage, API reference, architecture guides, and tutorials. Interactive code examples with embedded terminal emulator. Versioned documentation matching release tags. Search powered by Algolia DocSearch. Video tutorials with transcripts. Multi-language support (i18n).

### FR-9: Analytics and Insights

Performance analytics dashboard showing loop efficiency, iteration patterns, and cost breakdowns. Historical tracking of success rates. Recommendations for optimization. Cost estimation before runs. Budget alerts and spending limits.

### FR-10: Backwards CLI Compatibility

All existing CLI commands continue to function without breaking changes. New features accessible via new subcommands and flags. Configuration file format remains compatible with schema versioning. State file format migration utilities for new fields.

---

## Non-Functional Requirements

### NFR-1: Performance

- API response times under 100ms for status queries.
- WebSocket message delivery under 50ms latency.
- Dashboard renders within 500ms on 3G connections.
- Multi-agent orchestrator handles 10 concurrent agents per coordinator.
- Frontend bundle size under 500KB gzipped.

### NFR-2: Reliability

- Server graceful shutdown preserves session state.
- WebSocket reconnection with exponential backoff.
- Database transaction rollback on failures.
- Health check endpoints for load balancer integration.
- 99.9% uptime SLA for hosted services.

### NFR-3: Security

- Bearer token authentication for all API endpoints.
- HTTPS required for non-localhost deployments.
- CORS configured per deployment environment.
- Input validation and sanitization on all endpoints.
- Rate limiting on authentication endpoints.
- Secret scanning prevents credential commits.
- CSP headers on all responses.

### NFR-4: Accessibility

- WCAG 2.1 AA compliance for web UI.
- Keyboard navigation for all interactive elements.
- Screen reader compatible with ARIA labels.
- Color contrast ratios meeting AA standards.
- Focus management for modal dialogs.
- Reduced motion support.

### NFR-5: Test Coverage

- Minimum 90% code coverage for all new modules.
- Integration tests for API endpoints.
- End-to-end tests for critical user flows.
- Property-based tests for parsers and validators.
- Visual regression tests for UI components.
- Performance benchmarks with historical tracking.

---

## Implementation Tasks

### Phase 1: Core Infrastructure

### Task MC-1

- **ID** MC-1
- **Context Bundle** `src/server.rs`, `Cargo.toml`
- **DoD** Axum server serves static files from embedded assets directory. Health check endpoint returns 200 OK.
- **Checklist**
  * Static file serving middleware integrated.
  * Health check endpoint responds correctly.
  * Unit tests cover static file routes.
- **Dependencies** None
- [x] MC-1 Add static file serving to Axum server
---

### Task MC-2

- **ID** MC-2
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** WebSocket endpoint broadcasts state changes to connected clients. Connection handling includes authentication.
- **Checklist**
  * WebSocket upgrade handler implemented.
  * State change broadcast mechanism works.
  * Authentication validated on connection.
  * Integration tests verify broadcast behavior.
- **Dependencies** MC-1
- [x] MC-2 Implement WebSocket state broadcasting
---

### Task MC-3

- **ID** MC-3
- **Context Bundle** `Cargo.toml`, `src/lib.rs`
- **DoD** Frontend workspace with React, TypeScript, and Vite initialized. Build outputs to static assets directory.
- **Checklist**
  * package.json with dependencies configured.
  * TypeScript and ESLint configuration present.
  * Vite builds to assets directory.
  * CI builds frontend before Rust.
- **Dependencies** MC-1
- [x] MC-3 Initialize TypeScript/React frontend workspace
---

### Task MC-4

- **ID** MC-4
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Dashboard component displays all sessions with real-time updates via WebSocket. Status indicators show running/stopped/failed states.
- **Checklist**
  * Session list fetches from API.
  * WebSocket updates reflected immediately.
  * Status badges render correctly.
  * Component tests with mock data pass.
- **Dependencies** MC-2, MC-3
- [x] MC-4 Build real-time session dashboard component
---

### Task MC-5

- **ID** MC-5
- **Context Bundle** `src/prd.rs`, `src/task.rs`
- **DoD** Kanban board component renders PRD tasks as draggable cards. State transitions update PRD file via API.
- **Checklist**
  * Task cards display ID, title, status.
  * Drag and drop changes task state.
  * API persists state changes.
  * Keyboard navigation works.
  * Screen reader announces card movements.
- **Dependencies** MC-4
- [x] MC-5 Implement Kanban board for PRD tasks
---

### Task MC-6

- **ID** MC-6
- **Context Bundle** `src/server.rs`
- **DoD** Dark mode toggle persists user preference. CSS variables handle theming throughout UI.
- **Checklist**
  * Theme toggle in settings.
  * Preference stored in localStorage.
  * All components respect theme.
  * System preference detection works.
- **Dependencies** MC-3
- [x] MC-6 Add dark mode support to frontend
---

### Task MC-7

- **ID** MC-7
- **Context Bundle** `src/server.rs`
- **DoD** Responsive layout adapts to mobile, tablet, and desktop viewports. Touch interactions work on mobile.
- **Checklist**
  * Breakpoints at 640px, 768px, 1024px.
  * Navigation collapses to hamburger on mobile.
  * Touch swipe gestures on Kanban.
  * Visual regression tests pass.
- **Dependencies** MC-4, MC-5
- [x] MC-7 Implement responsive mobile-first layout
---

### Task MC-8

- **ID** MC-8
- **Context Bundle** `src/server.rs`
- **DoD** PWA manifest and service worker enable offline access and home screen installation.
- **Checklist**
  * manifest.json with icons.
  * Service worker caches static assets.
  * Offline indicator displayed.
  * Install prompt shown on mobile.
- **Dependencies** MC-3
- [x] MC-8 Add PWA support for offline access
---

### Task MC-9

- **ID** MC-9
- **Context Bundle** `src/server.rs`
- **DoD** Navigation sidebar with collapsible sections. Route transitions animated smoothly.
- **Checklist**
  * Sidebar shows all main sections.
  * Collapse state persisted.
  * Active route highlighted.
  * Transition animations under 300ms.
- **Dependencies** MC-3
- [x] MC-9 Build navigation sidebar component
---

### Task MC-10

- **ID** MC-10
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Session detail view shows full log output with syntax highlighting. Auto-scroll to latest output.
- **Checklist**
  * Log lines syntax highlighted.
  * Auto-scroll toggleable.
  * Search within logs works.
  * Performance handles 100k+ lines.
- **Dependencies** MC-4
- [x] MC-10 Create session log viewer component
---

### Phase 2: Multi-Agent Orchestration

### Task MC-11

- **ID** MC-11
- **Context Bundle** `src/backend/mod.rs`, `src/core.rs`
- **DoD** Agent coordinator spawns multiple backend sessions in parallel. Work queue distributes tasks to available agents.
- **Checklist**
  * Coordinator struct manages agent pool.
  * Task distribution respects dependencies.
  * Agent isolation via separate worktrees.
  * Unit tests verify coordination logic.
- **Dependencies** None
- [x] MC-11 Create multi-agent coordinator module
---

### Task MC-12

- **ID** MC-12
- **Context Bundle** `src/core.rs`, `src/app/worktree.rs`
- **DoD** Parallel agents each operate in isolated git worktrees. Worktree cleanup handles agent failures.
- **Checklist**
  * Each agent gets unique worktree.
  * Worktree creation thread-safe.
  * Cleanup removes worktrees on agent exit.
  * Integration tests verify isolation.
- **Dependencies** MC-11
- [x] MC-12 Implement agent worktree isolation
---

### Task MC-13

- **ID** MC-13
- **Context Bundle** `src/core.rs`, `src/task.rs`
- **DoD** Dependency graph analysis identifies independent task blocks for parallel execution. Dependent tasks wait for prerequisites.
- **Checklist**
  * Dependency parser extracts task relationships.
  * Graph topological sort orders execution.
  * Independent tasks scheduled concurrently.
  * Unit tests cover graph edge cases.
- **Dependencies** MC-11
- [x] MC-13 Add task dependency graph analysis
---

### Task MC-14

- **ID** MC-14
- **Context Bundle** `src/core.rs`, `src/backend/mod.rs`
- **DoD** Load balancer distributes tasks based on agent capacity and backend availability. Health checks remove unhealthy agents.
- **Checklist**
  * Round-robin and weighted distribution.
  * Agent health monitoring.
  * Unhealthy agents removed from pool.
  * Integration tests verify balancing.
- **Dependencies** MC-11
- [x] MC-14 Implement agent load balancing
---

### Task MC-15

- **ID** MC-15
- **Context Bundle** `src/core.rs`
- **DoD** Conflict detection identifies overlapping changes from parallel agents. Resolution strategies configurable.
- **Checklist**
  * Diff analysis detects conflicts.
  * Conflict report generated.
  * Manual or automatic resolution options.
  * Integration tests with conflicting changes.
- **Dependencies** MC-12
- [x] MC-15 Implement agent conflict detection
---

### Task MC-16

- **ID** MC-16
- **Context Bundle** `src/core.rs`, `src/backend/mod.rs`
- **DoD** Agent specialization allows dedicated agents for code-gen, testing, review, and documentation tasks.
- **Checklist**
  * Specialization config per agent.
  * Task routing by specialization.
  * Fallback to general agents.
  * Unit tests verify routing.
- **Dependencies** MC-11
- [x] MC-16 Add agent specialization support
---

### Task MC-17

- **ID** MC-17
- **Context Bundle** `src/core.rs`
- **DoD** Agent communication protocol enables message passing between specialized agents for handoffs.
- **Checklist**
  * Message queue between agents.
  * Handoff triggers on task completion.
  * Timeout handling for stuck agents.
  * Integration tests verify messaging.
- **Dependencies** MC-16
- [x] MC-17 Implement inter-agent communication
---

### Task MC-18

- **ID** MC-18
- **Context Bundle** `src/core.rs`, `src/backend/mod.rs`
- **DoD** Automatic retry with exponential backoff recovers from transient failures. Circuit breaker prevents cascade failures.
- **Checklist**
  * Exponential backoff with jitter.
  * Max retries configurable.
  * Circuit breaker state machine.
  * Unit tests verify retry behavior.
- **Dependencies** MC-11
- [x] MC-18 Add retry and circuit breaker patterns
---

### Task MC-19

- **ID** MC-19
- **Context Bundle** `src/verifier.rs`, `src/core.rs`
- **DoD** Verifier pipeline extended for multi-agent results. Aggregated coverage and test results.
- **Checklist**
  * Coverage merged across worktrees.
  * Test results aggregated.
  * Single PR created for all changes.
  * Integration tests verify aggregation.
- **Dependencies** MC-11, MC-12
- [x] MC-19 Extend verifier for multi-agent results
---

### Task MC-20

- **ID** MC-20
- **Context Bundle** `src/server.rs`
- **DoD** Dashboard shows real-time agent orchestration visualization with task flow diagram.
- **Checklist**
  * Agent nodes displayed.
  * Task flow animated.
  * Dependency edges shown.
  * Performance scales to 20 agents.
- **Dependencies** MC-4, MC-11
- [x] MC-20 Build agent orchestration dashboard
---

### Phase 3: Authentication and RBAC

### Task MC-21

- **ID** MC-21
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** User authentication with email/password and JWT tokens. Session management with refresh tokens.
- **Checklist**
  * Registration endpoint with password hashing.
  * Login endpoint returns JWT.
  * Token validation middleware.
  * Refresh token rotation works.
  * Rate limiting on auth endpoints.
- **Dependencies** MC-1
- [x] MC-21 Implement user authentication system
---

### Task MC-22

- **ID** MC-22
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** RBAC middleware enforces role-based permissions on API endpoints. Roles: Admin, Developer, Viewer.
- **Checklist**
  * Role definitions in database.
  * Permission checks on protected routes.
  * Admin can manage all resources.
  * Developer can execute sessions.
  * Viewer has read-only access.
  * Unit tests verify permission enforcement.
- **Dependencies** MC-21
- [x] MC-22 Add role-based access control middleware
---

### Task MC-23

- **ID** MC-23
- **Context Bundle** `src/server.rs`, `src/notify.rs`
- **DoD** Audit log records all authenticated actions with timestamp, actor, and resource. Logs queryable via API.
- **Checklist**
  * Audit entries written on each action.
  * Query endpoint with filtering.
  * Log retention policy configurable.
  * Integration tests verify logging.
- **Dependencies** MC-22
- [ ] MC-23 Create audit logging system
---

### Task MC-24

- **ID** MC-24
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** SAML 2.0 SSO integration with configurable identity provider. Session binding after SAML assertion validation.
- **Checklist**
  * SAML metadata endpoint.
  * Assertion consumer service.
  * User provisioning from claims.
  * Session creation after validation.
  * Integration tests with mock IdP.
- **Dependencies** MC-21
- [ ] MC-24 Implement SAML 2.0 SSO
---

### Task MC-25

- **ID** MC-25
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** OIDC integration with configurable providers. Token exchange and user info retrieval.
- **Checklist**
  * OIDC discovery endpoint handling.
  * Authorization code flow.
  * Token validation.
  * User info mapped to local account.
  * Integration tests with mock provider.
- **Dependencies** MC-21
- [ ] MC-25 Implement OIDC integration
---

### Task MC-26

- **ID** MC-26
- **Context Bundle** `src/server.rs`
- **DoD** OAuth2 social login for GitHub, Google, and GitLab. Account linking for existing users.
- **Checklist**
  * GitHub OAuth flow.
  * Google OAuth flow.
  * GitLab OAuth flow.
  * Account linking UI.
  * Integration tests for each provider.
- **Dependencies** MC-21
- [ ] MC-26 Add OAuth2 social login providers
---

### Task MC-27

- **ID** MC-27
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** Rate limiting middleware protects authentication and API endpoints. Configurable limits per endpoint.
- **Checklist**
  * Token bucket or sliding window algorithm.
  * Limits configurable in config file.
  * 429 responses with retry-after header.
  * Unit tests verify rate limiting.
- **Dependencies** MC-21
- [ ] MC-27 Add rate limiting middleware
---

### Task MC-28

- **ID** MC-28
- **Context Bundle** `src/server.rs`
- **DoD** Two-factor authentication with TOTP and backup codes. Recovery flow for lost devices.
- **Checklist**
  * TOTP setup with QR code.
  * Backup codes generated.
  * Recovery via email.
  * Rate limiting on 2FA attempts.
- **Dependencies** MC-21
- [ ] MC-28 Implement two-factor authentication
---

### Task MC-29

- **ID** MC-29
- **Context Bundle** `src/server.rs`
- **DoD** User settings page with profile, password change, and linked accounts management.
- **Checklist**
  * Profile edit form.
  * Password change with old password verification.
  * Linked accounts list with unlink option.
  * Component tests pass.
- **Dependencies** MC-21, MC-3
- [ ] MC-29 Build user settings page
---

### Task MC-30

- **ID** MC-30
- **Context Bundle** `src/server.rs`
- **DoD** Admin user management page with list, search, and role assignment. Bulk operations supported.
- **Checklist**
  * User list with pagination.
  * Search by email/name.
  * Role dropdown per user.
  * Bulk role assignment.
  * Admin-only access enforced.
- **Dependencies** MC-22, MC-3
- [ ] MC-30 Build admin user management page
---

### Phase 4: Team Collaboration

### Task MC-31

- **ID** MC-31
- **Context Bundle** `src/server.rs`, `src/state.rs`
- **DoD** Organization model groups users into teams with shared projects. Org-scoped resources.
- **Checklist**
  * Organization CRUD endpoints.
  * User membership management.
  * Project association with org.
  * Integration tests for org operations.
- **Dependencies** MC-22
- [ ] MC-31 Implement organization model
---

### Task MC-32

- **ID** MC-32
- **Context Bundle** `src/server.rs`
- **DoD** Real-time presence shows which team members are viewing a session or PRD.
- **Checklist**
  * Presence tracked via WebSocket.
  * Cursor positions broadcast.
  * User avatars displayed.
  * Graceful disconnect handling.
- **Dependencies** MC-2, MC-31
- [ ] MC-32 Add real-time presence indicators
---

### Task MC-33

- **ID** MC-33
- **Context Bundle** `src/server.rs`
- **DoD** Commenting system allows inline comments on PRD tasks with threaded replies.
- **Checklist**
  * Comment creation with line association.
  * Threaded reply support.
  * Edit and delete own comments.
  * Notification on mention.
- **Dependencies** MC-31
- [ ] MC-33 Build inline commenting system
---

### Task MC-34

- **ID** MC-34
- **Context Bundle** `src/server.rs`, `src/notify.rs`
- **DoD** @mentions in comments trigger notifications to mentioned users.
- **Checklist**
  * Mention autocomplete.
  * Notification created on mention.
  * Link to comment context.
  * Integration tests verify delivery.
- **Dependencies** MC-33
- [ ] MC-34 Implement @mention notifications
---

### Task MC-35

- **ID** MC-35
- **Context Bundle** `src/server.rs`
- **DoD** Activity feed shows recent actions across the organization with filtering options.
- **Checklist**
  * Chronological event list.
  * Filter by user, project, action type.
  * Pagination with infinite scroll.
  * Real-time updates via WebSocket.
- **Dependencies** MC-23, MC-31
- [ ] MC-35 Create organization activity feed
---

### Task MC-36

- **ID** MC-36
- **Context Bundle** `src/server.rs`, `src/notify.rs`
- **DoD** Notification preferences allow users to choose email, in-app, or both for each event type.
- **Checklist**
  * Preferences stored per user.
  * Event type granularity.
  * Digest email option.
  * Settings UI component.
- **Dependencies** MC-34
- [ ] MC-36 Add notification preferences
---

### Task MC-37

- **ID** MC-37
- **Context Bundle** `src/server.rs`
- **DoD** Team workspace dashboard shows shared sessions, PRDs, and team activity.
- **Checklist**
  * Session list scoped to org.
  * PRD list with shared editing.
  * Team member list.
  * Quick actions for common tasks.
- **Dependencies** MC-31, MC-4
- [ ] MC-37 Build team workspace dashboard
---

### Task MC-38

- **ID** MC-38
- **Context Bundle** `src/server.rs`
- **DoD** Invitation system allows admins to invite users via email with role assignment.
- **Checklist**
  * Invitation email sent.
  * Invite link with expiration.
  * Role assigned on acceptance.
  * Pending invites list for admins.
- **Dependencies** MC-31
- [ ] MC-38 Implement team invitation system
---

### Phase 5: IDE Extensions

### Task MC-39

- **ID** MC-39
- **Context Bundle** `src/cli.rs`, `Cargo.toml`
- **DoD** VS Code extension package with sidebar panel showing sessions. Commands to start, stop, and attach.
- **Checklist**
  * Extension activates on workspace.
  * Session list populates from API.
  * Start command triggers session.
  * Stop command halts session.
  * Attach opens terminal view.
  * Extension tests pass.
- **Dependencies** MC-1, MC-21
- [ ] MC-39 Build VS Code extension
---

### Task MC-40

- **ID** MC-40
- **Context Bundle** `src/cli.rs`
- **DoD** VS Code extension shows inline task status decorations in PRD files.
- **Checklist**
  * Task checkboxes have status icons.
  * Hover shows task details.
  * Click navigates to Kanban.
  * Real-time sync with server.
- **Dependencies** MC-39
- [ ] MC-40 Add VS Code inline task decorations
---

### Task MC-41

- **ID** MC-41
- **Context Bundle** `src/cli.rs`
- **DoD** VS Code extension provides PRD authoring assistance with snippets and validation.
- **Checklist**
  * Snippets for task blocks.
  * Validation diagnostics shown.
  * Quick fixes for common errors.
  * Syntax highlighting for PRD format.
- **Dependencies** MC-39
- [ ] MC-41 Add VS Code PRD authoring features
---

### Task MC-42

- **ID** MC-42
- **Context Bundle** `src/cli.rs`, `src/server.rs`
- **DoD** JetBrains plugin with tool window displaying sessions. Actions mirror VS Code functionality.
- **Checklist**
  * Plugin descriptor configured.
  * Tool window shows session list.
  * Actions invoke API correctly.
  * Plugin tests pass.
- **Dependencies** MC-1, MC-21
- [ ] MC-42 Build JetBrains plugin
---

### Task MC-43

- **ID** MC-43
- **Context Bundle** `src/cli.rs`
- **DoD** JetBrains plugin provides inline task annotations in PRD files.
- **Checklist**
  * Gutter icons for task status.
  * Intention actions for tasks.
  * Navigation to related code.
  * Tests verify annotations.
- **Dependencies** MC-42
- [ ] MC-43 Add JetBrains inline task annotations
---

### Task MC-44

- **ID** MC-44
- **Context Bundle** `src/cli.rs`, `src/server.rs`
- **DoD** Neovim plugin with Lua API and telescope picker for sessions. Commands for session management.
- **Checklist**
  * Lua module structure correct.
  * Telescope picker lists sessions.
  * Commands invoke API.
  * Integration tests with headless Neovim.
- **Dependencies** MC-1, MC-21
- [ ] MC-44 Build Neovim plugin
---

### Task MC-45

- **ID** MC-45
- **Context Bundle** `src/cli.rs`
- **DoD** Neovim plugin shows virtual text for task status in PRD buffers.
- **Checklist**
  * Virtual text displays status.
  * Extmarks handle updates.
  * Performance tested on large PRDs.
  * Integration tests pass.
- **Dependencies** MC-44
- [ ] MC-45 Add Neovim virtual text task status
---

### Task MC-46

- **ID** MC-46
- **Context Bundle** `src/cli.rs`
- **DoD** Emacs package via LSP provides session management and PRD editing features.
- **Checklist**
  * LSP server responds to requests.
  * Emacs client package created.
  * Session commands work.
  * Tests verify LSP protocol.
- **Dependencies** MC-1, MC-21
- [ ] MC-46 Build Emacs LSP package
---

### Phase 6: Enterprise Features

### Task MC-47

- **ID** MC-47
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** Stripe integration for subscription billing. Webhook handling for payment events.
- **Checklist**
  * Checkout session creation.
  * Subscription lifecycle events handled.
  * Usage metering for overage.
  * Invoice history API.
  * Webhook signature verification.
- **Dependencies** MC-21
- [ ] MC-47 Integrate Stripe billing
---

### Task MC-48

- **ID** MC-48
- **Context Bundle** `src/server.rs`
- **DoD** Usage dashboard shows API calls, agent minutes, and storage by org and project.
- **Checklist**
  * Metrics aggregated per time period.
  * Charts show trends.
  * Export to CSV.
  * Admin and org-scoped views.
- **Dependencies** MC-47, MC-31
- [ ] MC-48 Build usage analytics dashboard
---

### Task MC-49

- **ID** MC-49
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** Budget alerts notify admins when spending approaches or exceeds thresholds.
- **Checklist**
  * Threshold configuration per org.
  * Email and in-app alerts.
  * Automatic pause option.
  * Tests verify alert triggers.
- **Dependencies** MC-48
- [ ] MC-49 Implement budget alerts
---

### Task MC-50

- **ID** MC-50
- **Context Bundle** `src/server.rs`
- **DoD** Cost estimation API predicts run cost before execution based on task complexity.
- **Checklist**
  * Token estimation algorithm.
  * Cost model per backend.
  * Estimation shown in UI before start.
  * Unit tests verify accuracy.
- **Dependencies** MC-48
- [ ] MC-50 Add pre-run cost estimation
---

### Task MC-51

- **ID** MC-51
- **Context Bundle** `src/server.rs`
- **DoD** License management portal for enterprise customers with seat allocation.
- **Checklist**
  * Seat count displayed.
  * Add/remove seats.
  * License key management.
  * Compliance export for audits.
- **Dependencies** MC-47, MC-31
- [ ] MC-51 Build license management portal
---

### Task MC-52

- **ID** MC-52
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** On-premise deployment guide and Docker Compose configuration.
- **Checklist**
  * Docker images published.
  * Compose file with all services.
  * Configuration documentation.
  * Health checks for all containers.
- **Dependencies** MC-1
- [ ] MC-52 Create on-premise deployment package
---

### Task MC-53

- **ID** MC-53
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** Air-gapped mode allows operation without internet connectivity for secure environments.
- **Checklist**
  * Offline license validation.
  * Local backend support only.
  * Update via manual package.
  * Documentation for setup.
- **Dependencies** MC-52
- [ ] MC-53 Implement air-gapped mode
---

### Task MC-54

- **ID** MC-54
- **Context Bundle** `src/server.rs`
- **DoD** GDPR compliance features include data export, deletion requests, and consent management.
- **Checklist**
  * User data export endpoint.
  * Account deletion with cascade.
  * Consent tracking.
  * Audit log for compliance actions.
- **Dependencies** MC-23
- [ ] MC-54 Add GDPR compliance features
---

### Task MC-55

- **ID** MC-55
- **Context Bundle** `src/server.rs`
- **DoD** SOC 2 compliance documentation and controls implemented.
- **Checklist**
  * Access control documentation.
  * Encryption verification.
  * Logging review procedures.
  * Penetration test preparation.
- **Dependencies** MC-23, MC-27
- [ ] MC-55 Implement SOC 2 compliance controls
---

### Phase 7: AI and Intelligence

### Task MC-56

- **ID** MC-56
- **Context Bundle** `src/app/prd_init.rs`, `src/prd.rs`
- **DoD** Interactive PRD wizard collects inputs and generates structured PRD. Multi-turn refinement supported.
- **Checklist**
  * Wizard steps collect goal, constraints, context.
  * AI generates PRD from inputs.
  * User can request changes.
  * Final PRD passes validation.
  * Unit tests cover wizard flow.
- **Dependencies** None
- [ ] MC-56 Enhance PRD generation wizard
---

### Task MC-57

- **ID** MC-57
- **Context Bundle** `src/prd.rs`, `src/backend/mod.rs`
- **DoD** PRD validation with detailed error messages and auto-fix suggestions. Schema versioning for forward compatibility.
- **Checklist**
  * Validator reports specific errors.
  * Auto-fix for common issues.
  * Schema version in PRD header.
  * Migration for older formats.
  * Property tests for parser.
- **Dependencies** MC-56
- [ ] MC-57 Add PRD validation enhancements
---

### Task MC-58

- **ID** MC-58
- **Context Bundle** `src/prd.rs`
- **DoD** Natural language to PRD conversion accepts plain English descriptions and generates compliant PRD.
- **Checklist**
  * Free-text input accepted.
  * AI extracts tasks and structure.
  * User reviews before save.
  * Handles ambiguous input gracefully.
- **Dependencies** MC-56
- [ ] MC-58 Implement natural language PRD generation
---

### Task MC-59

- **ID** MC-59
- **Context Bundle** `src/core.rs`
- **DoD** Smart task breakdown optimizes task granularity for parallel execution efficiency.
- **Checklist**
  * Task size analysis.
  * Split suggestions for large tasks.
  * Merge suggestions for trivial tasks.
  * User approval required.
- **Dependencies** MC-58
- [ ] MC-59 Add smart task breakdown optimization
---

### Task MC-60

- **ID** MC-60
- **Context Bundle** `src/core.rs`, `src/backend/mod.rs`
- **DoD** Automatic backend selection chooses optimal model based on task complexity and cost.
- **Checklist**
  * Complexity scoring algorithm.
  * Cost/performance tradeoff model.
  * Override option for users.
  * Logging of selection rationale.
- **Dependencies** MC-50
- [ ] MC-60 Implement automatic backend selection
---

### Task MC-61

- **ID** MC-61
- **Context Bundle** `src/core.rs`
- **DoD** Learning system improves prompts based on successful loop patterns (opt-in, privacy-preserving).
- **Checklist**
  * Success pattern extraction.
  * Prompt improvement suggestions.
  * Opt-in only with clear consent.
  * Local-only processing option.
- **Dependencies** MC-56
- [ ] MC-61 Add prompt improvement learning
---

### Task MC-62

- **ID** MC-62
- **Context Bundle** `src/core.rs`
- **DoD** Anomaly detection identifies stuck or looping agents and suggests interventions.
- **Checklist**
  * Pattern detection for stuck states.
  * Alert generated on anomaly.
  * Suggested interventions shown.
  * Auto-restart option.
- **Dependencies** MC-11
- [ ] MC-62 Implement agent anomaly detection
---

### Task MC-63

- **ID** MC-63
- **Context Bundle** `src/server.rs`
- **DoD** AI assistant in UI answers questions about gralph usage and troubleshooting.
- **Checklist**
  * Chat interface component.
  * Context-aware responses.
  * Links to documentation.
  * Escalation to support option.
- **Dependencies** MC-3
- [ ] MC-63 Build in-app AI assistant
---

### Phase 8: Marketplace and Community

### Task MC-64

- **ID** MC-64
- **Context Bundle** `src/server.rs`, `src/config.rs`
- **DoD** Marketplace API for listing, searching, and installing community contributions. Categories and ratings supported.
- **Checklist**
  * List endpoint with pagination.
  * Search with filters.
  * Install downloads and configures.
  * Rating submission and aggregation.
  * Integration tests cover CRUD.
- **Dependencies** MC-1, MC-21
- [ ] MC-64 Build marketplace API
---

### Task MC-65

- **ID** MC-65
- **Context Bundle** `src/server.rs`
- **DoD** Marketplace frontend with browse, search, and install UI. User ratings and reviews displayed.
- **Checklist**
  * Grid view of contributions.
  * Search filters work.
  * Install triggers API call.
  * Rating stars display.
  * Accessible navigation.
- **Dependencies** MC-3, MC-64
- [ ] MC-65 Build marketplace frontend
---

### Task MC-66

- **ID** MC-66
- **Context Bundle** `src/config.rs`
- **DoD** Plugin system allows custom backend adapters to be loaded from local or marketplace.
- **Checklist**
  * Plugin manifest format defined.
  * Local plugin loading.
  * Marketplace plugin installation.
  * Plugin isolation sandbox.
- **Dependencies** MC-64
- [ ] MC-66 Implement plugin system
---

### Task MC-67

- **ID** MC-67
- **Context Bundle** `src/server.rs`
- **DoD** Contribution submission workflow with validation, review queue, and publishing.
- **Checklist**
  * Submission form with metadata.
  * Automated validation checks.
  * Admin review queue.
  * Version management.
- **Dependencies** MC-64
- [ ] MC-67 Build contribution submission workflow
---

### Task MC-68

- **ID** MC-68
- **Context Bundle** `src/server.rs`
- **DoD** PRD template gallery showcases community examples with one-click import.
- **Checklist**
  * Template preview UI.
  * Import to local project.
  * Category browsing.
  * Featured templates section.
- **Dependencies** MC-65
- [ ] MC-68 Create PRD template gallery
---

### Task MC-69

- **ID** MC-69
- **Context Bundle** `src/notify.rs`
- **DoD** Discord bot sends notifications for session events to configured channels.
- **Checklist**
  * Bot registration instructions.
  * Channel configuration.
  * Event messages formatted.
  * Rich embeds for status.
- **Dependencies** MC-36
- [ ] MC-69 Build Discord notification bot
---

### Task MC-70

- **ID** MC-70
- **Context Bundle** `src/notify.rs`
- **DoD** Slack app integration for notifications with interactive buttons.
- **Checklist**
  * Slack app manifest.
  * OAuth installation flow.
  * Interactive messages.
  * Thread replies for updates.
- **Dependencies** MC-36
- [ ] MC-70 Build Slack app integration
---

### Task MC-71

- **ID** MC-71
- **Context Bundle** `src/notify.rs`
- **DoD** Microsoft Teams connector sends notifications to configured channels.
- **Checklist**
  * Connector configuration.
  * Adaptive cards for messages.
  * Action buttons.
  * Tests with mock Teams API.
- **Dependencies** MC-36
- [ ] MC-71 Build Microsoft Teams connector
---

### Phase 9: Documentation and Onboarding

### Task MC-72

- **ID** MC-72
- **Context Bundle** `README.md`, `ARCHITECTURE.md`
- **DoD** VitePress documentation site with CLI reference, API docs, and tutorials. Deployed to docs.gralph.dev.
- **Checklist**
  * VitePress configured.
  * CLI commands documented.
  * API endpoints documented.
  * Tutorial guides written.
  * Search integration works.
- **Dependencies** None
- [ ] MC-72 Create VitePress documentation site
---

### Task MC-73

- **ID** MC-73
- **Context Bundle** `README.md`, `ARCHITECTURE.md`
- **DoD** Interactive code examples embedded in documentation. Terminal emulator shows live command output.
- **Checklist**
  * Embedded terminal component.
  * Examples executable in browser.
  * Output matches real CLI.
  * Fallback for no-JS browsers.
- **Dependencies** MC-72
- [ ] MC-73 Add interactive documentation examples
---

### Task MC-74

- **ID** MC-74
- **Context Bundle** `README.md`
- **DoD** Video tutorials for common workflows with transcripts and captions.
- **Checklist**
  * Videos hosted on YouTube/platform.
  * Transcripts on doc site.
  * Closed captions.
  * Playlist organization.
- **Dependencies** MC-72
- [ ] MC-74 Create video tutorial series
---

### Task MC-75

- **ID** MC-75
- **Context Bundle** `src/server.rs`
- **DoD** Guided first-run experience walks new users through initial setup and first loop.
- **Checklist**
  * Welcome modal on first login.
  * Step-by-step setup wizard.
  * Sample project creation.
  * First loop completion celebration.
- **Dependencies** MC-3, MC-21
- [ ] MC-75 Build guided first-run experience
---

### Task MC-76

- **ID** MC-76
- **Context Bundle** `src/server.rs`
- **DoD** Interactive PRD builder wizard with guided steps and live preview.
- **Checklist**
  * Form-based task creation.
  * Live PRD preview.
  * Validation feedback.
  * Export to file.
- **Dependencies** MC-56, MC-3
- [ ] MC-76 Create interactive PRD builder wizard
---

### Task MC-77

- **ID** MC-77
- **Context Bundle** `README.md`
- **DoD** Multi-language documentation with at least English, Spanish, and Chinese versions.
- **Checklist**
  * Translation workflow established.
  * Language switcher in UI.
  * Core docs translated.
  * Community contribution guide for translations.
- **Dependencies** MC-72
- [ ] MC-77 Add multi-language documentation
---

### Task MC-78

- **ID** MC-78
- **Context Bundle** `README.md`
- **DoD** Troubleshooting chatbot answers common questions using documentation knowledge base.
- **Checklist**
  * Chat widget on docs site.
  * Documentation indexed.
  * Common questions handled.
  * Escalation to GitHub issues.
- **Dependencies** MC-72
- [ ] MC-78 Build troubleshooting chatbot
---

### Task MC-79

- **ID** MC-79
- **Context Bundle** `README.md`
- **DoD** Example project gallery with different tech stacks and complexity levels.
- **Checklist**
  * Gallery page on docs site.
  * Categorization by stack/complexity.
  * One-click clone to local.
  * Success metrics displayed.
- **Dependencies** MC-72
- [ ] MC-79 Create example project gallery
---

### Phase 10: Testing and Quality

### Task MC-80

- **ID** MC-80
- **Context Bundle** `.github/workflows/ci.yml`
- **DoD** CI pipeline includes frontend build, Rust tests, coverage checks, and accessibility tests. All gates required for merge.
- **Checklist**
  * Frontend build step added.
  * Coverage threshold enforced.
  * Accessibility tests run.
  * All jobs required for merge.
- **Dependencies** MC-3
- [ ] MC-80 Update CI for full-stack testing
---

### Task MC-81

- **ID** MC-81
- **Context Bundle** `tests/`
- **DoD** End-to-end tests cover critical user flows using Playwright.
- **Checklist**
  * Playwright setup complete.
  * Login flow tested.
  * Session start/stop tested.
  * Kanban drag-drop tested.
  * CI runs E2E tests.
- **Dependencies** MC-3, MC-21
- [ ] MC-81 Add Playwright E2E tests
---

### Task MC-82

- **ID** MC-82
- **Context Bundle** `tests/`
- **DoD** Visual regression tests detect unintended UI changes using Percy or similar.
- **Checklist**
  * Visual testing tool configured.
  * Baseline screenshots captured.
  * PR checks compare to baseline.
  * Approval workflow for changes.
- **Dependencies** MC-3
- [ ] MC-82 Add visual regression tests
---

### Task MC-83

- **ID** MC-83
- **Context Bundle** `tests/`
- **DoD** Performance benchmarks with historical tracking and regression detection.
- **Checklist**
  * Benchmark suite created.
  * Results stored with history.
  * Regression alerts triggered.
  * Dashboard shows trends.
- **Dependencies** MC-1
- [ ] MC-83 Implement performance benchmarks
---

### Task MC-84

- **ID** MC-84
- **Context Bundle** `tests/`
- **DoD** Fuzz testing for parser robustness using cargo-fuzz.
- **Checklist**
  * Fuzz targets defined.
  * Continuous fuzzing setup.
  * Crash reproduction documented.
  * Coverage-guided fuzzing enabled.
- **Dependencies** None
- [ ] MC-84 Add fuzz testing for parsers
---

### Task MC-85

- **ID** MC-85
- **Context Bundle** `tests/`
- **DoD** Mutation testing verifies test effectiveness using cargo-mutants.
- **Checklist**
  * Mutation testing configured.
  * Baseline mutation score recorded.
  * Tests improved based on survivors.
  * CI mutation testing gate.
- **Dependencies** None
- [ ] MC-85 Add mutation testing
---

### Task MC-86

- **ID** MC-86
- **Context Bundle** `tests/`
- **DoD** Load testing infrastructure validates performance under concurrent users.
- **Checklist**
  * Load test scenarios defined.
  * k6 or similar tool configured.
  * Thresholds for pass/fail.
  * Results dashboard.
- **Dependencies** MC-1
- [ ] MC-86 Create load testing infrastructure
---

### Task MC-87

- **ID** MC-87
- **Context Bundle** `tests/`
- **DoD** Contract testing ensures API stability between frontend and backend.
- **Checklist**
  * Pact or similar tool setup.
  * Consumer contracts defined.
  * Provider verification.
  * CI contract checks.
- **Dependencies** MC-1, MC-3
- [ ] MC-87 Add API contract testing
---

### Task MC-88

- **ID** MC-88
- **Context Bundle** `src/server.rs`
- **DoD** WCAG 2.1 AA accessibility audit passes for all UI components. Keyboard navigation and screen reader support verified.
- **Checklist**
  * Color contrast meets AA.
  * Focus indicators visible.
  * ARIA labels complete.
  * Keyboard shortcuts documented.
  * Automated accessibility tests pass.
- **Dependencies** MC-4, MC-5, MC-65
- [ ] MC-88 Complete accessibility audit and fixes
---

### Phase 11: Analytics and Monitoring

### Task MC-89

- **ID** MC-89
- **Context Bundle** `src/server.rs`
- **DoD** Performance analytics dashboard shows loop efficiency, iteration patterns, and success rates.
- **Checklist**
  * Metrics collection implemented.
  * Charts for key metrics.
  * Time period filtering.
  * Export capability.
- **Dependencies** MC-3
- [ ] MC-89 Build performance analytics dashboard
---

### Task MC-90

- **ID** MC-90
- **Context Bundle** `src/server.rs`
- **DoD** Session replay allows reviewing agent decisions and outputs after completion.
- **Checklist**
  * Full session recording.
  * Playback UI component.
  * Scrubbing and search.
  * Privacy-respecting storage.
- **Dependencies** MC-10
- [ ] MC-90 Implement session replay
---

### Task MC-91

- **ID** MC-91
- **Context Bundle** `src/server.rs`
- **DoD** Error tracking with Sentry or similar captures and reports frontend and backend errors.
- **Checklist**
  * Sentry SDK integrated.
  * Source maps uploaded.
  * Error grouping configured.
  * Alerts for new errors.
- **Dependencies** MC-1, MC-3
- [ ] MC-91 Add error tracking integration
---

### Task MC-92

- **ID** MC-92
- **Context Bundle** `src/server.rs`
- **DoD** Prometheus metrics endpoint exposes server health and performance metrics.
- **Checklist**
  * Metrics endpoint /metrics.
  * Standard metrics (requests, latency).
  * Custom business metrics.
  * Grafana dashboard template.
- **Dependencies** MC-1
- [ ] MC-92 Add Prometheus metrics endpoint
---

### Task MC-93

- **ID** MC-93
- **Context Bundle** `src/server.rs`
- **DoD** Distributed tracing with OpenTelemetry tracks requests across services.
- **Checklist**
  * OTLP exporter configured.
  * Trace context propagation.
  * Span attributes populated.
  * Jaeger/Tempo integration.
- **Dependencies** MC-1
- [ ] MC-93 Implement OpenTelemetry tracing
---

### Phase 12: CLI Enhancements

### Task MC-94

- **ID** MC-94
- **Context Bundle** `src/cli.rs`, `src/config.rs`
- **DoD** New CLI subcommands for multi-agent and team features. Backwards compatible with existing commands.
- **Checklist**
  * New subcommands added to clap tree.
  * Existing commands unchanged.
  * Help text covers new features.
  * Integration tests for new commands.
- **Dependencies** MC-11, MC-22
- [ ] MC-94 Add CLI subcommands for new features
---

### Task MC-95

- **ID** MC-95
- **Context Bundle** `src/state.rs`, `src/config.rs`
- **DoD** State file migration utilities handle schema changes. Automatic migration on load with backup.
- **Checklist**
  * Version field in state file.
  * Migration functions for each version.
  * Backup created before migration.
  * Migration tests cover all versions.
- **Dependencies** None
- [ ] MC-95 Implement state file migrations
---

### Task MC-96

- **ID** MC-96
- **Context Bundle** `src/cli.rs`
- **DoD** Shell completions generated for bash, zsh, fish, and PowerShell.
- **Checklist**
  * clap_complete integration.
  * Completions bundled in release.
  * Installation instructions.
  * Tests verify completion scripts.
- **Dependencies** None
- [ ] MC-96 Generate shell completions
---

### Task MC-97

- **ID** MC-97
- **Context Bundle** `src/cli.rs`
- **DoD** Interactive TUI mode provides dashboard and controls without leaving terminal.
- **Checklist**
  * Ratatui-based TUI.
  * Session list and status.
  * Log viewer panel.
  * Keyboard shortcuts.
- **Dependencies** None
- [ ] MC-97 Build terminal UI mode
---

### Task MC-98

- **ID** MC-98
- **Context Bundle** `src/cli.rs`
- **DoD** gralph login command authenticates CLI with server for team features.
- **Checklist**
  * OAuth device flow.
  * Token stored securely.
  * Token refresh handled.
  * Logout command clears token.
- **Dependencies** MC-21
- [ ] MC-98 Add CLI authentication command
---

### Task MC-99

- **ID** MC-99
- **Context Bundle** `src/cli.rs`
- **DoD** gralph sync command uploads local sessions to cloud for team visibility.
- **Checklist**
  * Session upload endpoint.
  * Incremental sync.
  * Conflict resolution.
  * Offline queue.
- **Dependencies** MC-31, MC-98
- [ ] MC-99 Add CLI sync command
---

### Task MC-100

- **ID** MC-100
- **Context Bundle** `src/cli.rs`
- **DoD** gralph deploy command packages and deploys completed work to configured targets.
- **Checklist**
  * Deploy target configuration.
  * Package creation.
  * Target-specific adapters.
  * Rollback support.
- **Dependencies** None
- [ ] MC-100 Add CLI deploy command
---

### Phase 13: GitHub Integration

### Task MC-101

- **ID** MC-101
- **Context Bundle** `src/server.rs`
- **DoD** GitHub App integration automatically creates issues from PRD tasks.
- **Checklist**
  * GitHub App manifest.
  * Issue creation from tasks.
  * Bidirectional sync.
  * Label mapping.
- **Dependencies** MC-21
- [ ] MC-101 Build GitHub App for issue sync
---

### Task MC-102

- **ID** MC-102
- **Context Bundle** `src/server.rs`
- **DoD** GitHub Actions workflow template for gralph CI integration.
- **Checklist**
  * Reusable workflow published.
  * PRD validation step.
  * Loop execution step.
  * Status reporting.
- **Dependencies** None
- [ ] MC-102 Create GitHub Actions workflow
---

### Task MC-103

- **ID** MC-103
- **Context Bundle** `src/server.rs`
- **DoD** PR status checks from gralph verifier integrated with GitHub.
- **Checklist**
  * Check run creation.
  * Status updates.
  * Annotation for failures.
  * Re-run triggers.
- **Dependencies** MC-101
- [ ] MC-103 Add GitHub PR status checks
---

### Task MC-104

- **ID** MC-104
- **Context Bundle** `src/server.rs`
- **DoD** GitLab integration provides equivalent functionality for GitLab users.
- **Checklist**
  * GitLab OAuth integration.
  * Issue sync.
  * Pipeline integration.
  * MR status reporting.
- **Dependencies** MC-21
- [ ] MC-104 Build GitLab integration
---

### Task MC-105

- **ID** MC-105
- **Context Bundle** `src/server.rs`
- **DoD** Bitbucket integration provides equivalent functionality for Bitbucket users.
- **Checklist**
  * Bitbucket OAuth integration.
  * Issue tracking sync.
  * Pipeline integration.
  * PR status reporting.
- **Dependencies** MC-21
- [ ] MC-105 Build Bitbucket integration
---

### Phase 14: Final Polish

### Task MC-106

- **ID** MC-106
- **Context Bundle** `src/server.rs`
- **DoD** Keyboard shortcuts for all common UI actions with discoverable help modal.
- **Checklist**
  * Shortcut definitions.
  * Help modal (? key).
  * No conflicts with browser.
  * Customization option.
- **Dependencies** MC-3
- [ ] MC-106 Add comprehensive keyboard shortcuts
---

### Task MC-107

- **ID** MC-107
- **Context Bundle** `src/server.rs`
- **DoD** Toast notification system for async operation feedback.
- **Checklist**
  * Toast component created.
  * Auto-dismiss with configurable duration.
  * Action buttons in toasts.
  * Queue management for multiple.
- **Dependencies** MC-3
- [ ] MC-107 Build toast notification system
---

### Task MC-108

- **ID** MC-108
- **Context Bundle** `src/server.rs`
- **DoD** Command palette (cmd+k) for quick navigation and actions.
- **Checklist**
  * Palette component.
  * Fuzzy search.
  * Recent commands.
  * Extensible action registration.
- **Dependencies** MC-3
- [ ] MC-108 Implement command palette
---

### Task MC-109

- **ID** MC-109
- **Context Bundle** `src/server.rs`
- **DoD** Export functionality for sessions, PRDs, and analytics in various formats.
- **Checklist**
  * Export to JSON, CSV, PDF.
  * Bulk export option.
  * Scheduled exports.
  * Format customization.
- **Dependencies** MC-4, MC-89
- [ ] MC-109 Add export functionality
---

### Task MC-110

- **ID** MC-110
- **Context Bundle** `src/server.rs`
- **DoD** Import functionality for PRDs and configurations from other tools.
- **Checklist**
  * Import from common formats.
  * Mapping UI for fields.
  * Validation before import.
  * Undo capability.
- **Dependencies** MC-56
- [ ] MC-110 Add import functionality
---

---

## Success Criteria

- Dashboard displays real-time session status within 100ms of state change.
- Kanban board drag-and-drop updates PRD file within 500ms.
- Multi-agent orchestrator executes 5+ independent tasks in parallel.
- RBAC prevents unauthorized actions with 100% enforcement.
- VS Code, JetBrains, Neovim, and Emacs extensions connect and control sessions.
- SAML and OIDC SSO complete authentication flow end-to-end.
- Stripe billing processes subscription and usage events.
- Marketplace lists and installs 100+ community contributions.
- Documentation site has search, interactive examples, and video tutorials.
- All new code maintains 90%+ test coverage.
- Accessibility audit passes WCAG 2.1 AA criteria.
- CLI backwards compatibility verified by existing integration tests.
- Performance benchmarks show no regressions.
- E2E tests pass for all critical flows.
- User satisfaction score above 4.5/5 in beta feedback.

---

## Risk Mitigation

- R-001: Context loss between agents mitigated by shared worktree state and communication protocol.
- R-002: WebSocket scalability addressed via connection pooling and horizontal scaling.
- R-003: Plugin security enforced via sandboxing and review process.
- R-004: Coverage regression blocked by CI gates at 90% threshold.
- R-005: Breaking changes prevented by API versioning and deprecation policy.
- R-006: Enterprise security validated via penetration testing and compliance audits.

---

## Warnings

- External service integrations (Stripe, SAML, OIDC, GitHub) require API documentation review during implementation.
- Accessibility compliance requires manual audit with assistive technology testing.
- Multi-agent orchestration performance targets must be validated under real workloads.
- Plugin system security requires careful sandboxing design.
- International compliance (GDPR, SOC 2) requires legal review.
