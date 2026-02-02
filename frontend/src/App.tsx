import { useCallback, useEffect, useMemo, useState } from 'react';
import { AgentOrchestrationDashboard } from './components/AgentOrchestrationDashboard';
import { HamburgerMenu } from './components/HamburgerMenu';
import { InstallPrompt } from './components/InstallPrompt';
import { KanbanBoard } from './components/KanbanBoard';
import { OfflineIndicator } from './components/OfflineIndicator';
import { SessionDashboard } from './components/SessionDashboard';
import { SessionLogViewer } from './components/SessionLogViewer';
import { Sidebar, type Route, type SidebarSection } from './components/Sidebar';
import { ThemeToggle } from './components/ThemeToggle';
import { useBreakpoints } from './hooks/useMediaQuery';
import { useLogs } from './hooks/useLogs';
import { useOrchestration } from './hooks/useOrchestration';
import { useServiceWorker } from './hooks/useServiceWorker';
import { useSessions } from './hooks/useSessions';
import { useTasks } from './hooks/useTasks';
import { useTheme } from './hooks/useTheme';
import { useWebSocket } from './hooks/useWebSocket';

function getWebSocketUrl(): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}/ws`;
}

function getBaseUrl(): string {
  return '';
}

function getToken(): string | undefined {
  // Token can be passed via URL query parameter for development
  const params = new URLSearchParams(window.location.search);
  return params.get('token') ?? undefined;
}

function App() {
  const token = useMemo(() => getToken(), []);
  const wsUrl = useMemo(() => getWebSocketUrl(), []);
  const baseUrl = useMemo(() => getBaseUrl(), []);

  const [activeRoute, setActiveRoute] = useState<Route>('sessions');
  const [selectedSession, setSelectedSession] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [showRawLogs, setShowRawLogs] = useState(false);

  const { theme, setTheme } = useTheme();
  const { isMobile } = useBreakpoints();

  // Register service worker for PWA support
  useServiceWorker();

  // Close sidebar when switching to tablet/desktop view (sidebar becomes permanent)
  useEffect(() => {
    if (!isMobile) {
      setSidebarOpen(false);
    }
  }, [isMobile]);

  const toggleSidebar = useCallback(() => {
    setSidebarOpen((prev) => !prev);
  }, []);

  const closeSidebar = useCallback(() => {
    setSidebarOpen(false);
  }, []);

  const {
    loading: sessionsLoading,
    error: fetchError,
    fetchSessions,
    stopSession,
  } = useSessions({
    baseUrl,
    token,
    autoFetch: false, // WebSocket provides initial state
  });

  const {
    connectionState,
    sessions,
    error: wsError,
    reconnect,
  } = useWebSocket({
    url: wsUrl,
    token,
  });

  // Auto-select first running session if none selected
  const effectiveSelectedSession = useMemo(() => {
    return selectedSession ??
      sessions.find(s => s.status === 'running')?.name ??
      sessions[0]?.name;
  }, [selectedSession, sessions]);

  const {
    tasks,
    loading: tasksLoading,
    error: tasksError,
    fetchTasks,
    updateTaskStatus,
  } = useTasks({
    baseUrl,
    token,
    sessionName: selectedSession ?? undefined,
    autoFetch: false,
  });

  const {
    logs,
    totalLines,
    logFile,
    loading: logsLoading,
    error: logsError,
    fetchLogs,
  } = useLogs({
    baseUrl,
    token,
    sessionName: effectiveSelectedSession ?? undefined,
    raw: showRawLogs,
    autoFetch: false,
    pollInterval: 0, // Disable polling, use manual refresh
  });

  const {
    state: orchestrationState,
    loading: orchestrationLoading,
    error: orchestrationError,
    fetchOrchestration,
  } = useOrchestration({
    baseUrl,
    token,
    autoFetch: activeRoute === 'orchestration',
    pollInterval: activeRoute === 'orchestration' ? 2000 : 0,
  });

  // Fetch logs when switching to logs route or when session changes
  useEffect(() => {
    if (activeRoute === 'logs' && effectiveSelectedSession) {
      fetchLogs();
    }
  }, [activeRoute, effectiveSelectedSession, fetchLogs]);

  // Fetch orchestration when switching to orchestration route
  useEffect(() => {
    if (activeRoute === 'orchestration') {
      fetchOrchestration();
    }
  }, [activeRoute, fetchOrchestration]);

  const handleStopSession = useCallback(async (name: string) => {
    await stopSession(name);
  }, [stopSession]);

  const handleSessionSelect = useCallback((sessionName: string) => {
    setSelectedSession(sessionName);
    setActiveRoute('tasks');
  }, []);

  const handleViewLogs = useCallback((sessionName: string) => {
    setSelectedSession(sessionName);
    setActiveRoute('logs');
  }, []);

  const handleUpdateTaskStatus = useCallback(async (taskId: string, newStatus: 'pending' | 'in_progress' | 'completed') => {
    await updateTaskStatus(taskId, newStatus);
  }, [updateTaskStatus]);

  const handleRouteChange = useCallback((route: Route) => {
    setActiveRoute(route);
    if (route === 'tasks' && selectedSession) {
      fetchTasks();
    }
    if (route === 'logs' && effectiveSelectedSession) {
      fetchLogs();
    }
  }, [selectedSession, effectiveSelectedSession, fetchTasks, fetchLogs]);

  const handleToggleRawLogs = useCallback((raw: boolean) => {
    setShowRawLogs(raw);
  }, []);

  // Refetch logs when raw toggle changes
  useEffect(() => {
    if (activeRoute === 'logs' && effectiveSelectedSession) {
      fetchLogs();
    }
  }, [showRawLogs]); // eslint-disable-line react-hooks/exhaustive-deps

  const error = wsError || fetchError || tasksError || orchestrationError;
  const loading = sessionsLoading || tasksLoading || orchestrationLoading;

  // Count sessions by status for badges
  const runningCount = useMemo(
    () => sessions.filter((s) => s.status === 'running').length,
    [sessions]
  );

  const taskCount = useMemo(() => tasks.length, [tasks]);

  // Count agents for orchestration badge
  const agentCount = useMemo(
    () => orchestrationState?.agents.length ?? 0,
    [orchestrationState]
  );

  // Build sidebar sections
  const sidebarSections: SidebarSection[] = useMemo(
    () => [
      {
        id: 'main',
        title: 'Dashboard',
        icon: '📊',
        items: [
          {
            id: 'sessions',
            label: 'Sessions',
            route: 'sessions' as Route,
            icon: '🖥️',
            badge: runningCount > 0 ? runningCount : undefined,
          },
          {
            id: 'tasks',
            label: effectiveSelectedSession
              ? `Tasks (${effectiveSelectedSession})`
              : 'Tasks',
            route: 'tasks' as Route,
            icon: '📋',
            badge: taskCount > 0 ? taskCount : undefined,
          },
          {
            id: 'logs',
            label: effectiveSelectedSession
              ? `Logs (${effectiveSelectedSession})`
              : 'Logs',
            route: 'logs' as Route,
            icon: '📜',
          },
          {
            id: 'orchestration',
            label: 'Orchestration',
            route: 'orchestration' as Route,
            icon: '🔄',
            badge: agentCount > 0 ? agentCount : undefined,
          },
        ],
      },
      {
        id: 'preferences',
        title: 'Preferences',
        icon: '⚙️',
        items: [
          {
            id: 'settings',
            label: 'Settings',
            route: 'settings' as Route,
            icon: '🔧',
          },
        ],
      },
    ],
    [runningCount, taskCount, agentCount, effectiveSelectedSession]
  );

  // Determine if we should show the sidebar (tablet and larger)
  const showPermanentSidebar = !isMobile;

  return (
    <div className={`app ${showPermanentSidebar ? 'app--with-sidebar' : ''}`}>
      {/* PWA Components */}
      <OfflineIndicator />
      <InstallPrompt />

      {/* Sidebar */}
      {showPermanentSidebar ? (
        <Sidebar
          sections={sidebarSections}
          activeRoute={activeRoute}
          onRouteChange={handleRouteChange}
          isOpen={true}
        />
      ) : (
        <>
          {/* Mobile sidebar overlay */}
          {sidebarOpen && (
            <div
              className="sidebar-overlay sidebar-overlay--visible"
              onClick={closeSidebar}
              aria-hidden="true"
            />
          )}
          <Sidebar
            sections={sidebarSections}
            activeRoute={activeRoute}
            onRouteChange={handleRouteChange}
            isOpen={sidebarOpen}
            onClose={closeSidebar}
          />
        </>
      )}

      <div className="app__content">
        <header className="header">
          <div className="header__mobile-controls">
            <h1>Gralph Mission Control</h1>
            {isMobile && (
              <HamburgerMenu
                isOpen={sidebarOpen}
                onToggle={toggleSidebar}
                aria-controls="sidebar-nav"
              />
            )}
          </div>
          <div className="header__right">
            {/* Mobile-only tab nav (kept for backwards compatibility) */}
            {isMobile && !sidebarOpen && (
              <nav
                id="main-nav"
                className="header__nav"
                role="tablist"
                aria-label="Quick navigation"
              >
                <button
                  role="tab"
                  aria-selected={activeRoute === 'sessions'}
                  aria-controls="panel-sessions"
                  className={`header__tab ${activeRoute === 'sessions' ? 'header__tab--active' : ''}`}
                  onClick={() => handleRouteChange('sessions')}
                >
                  Sessions
                </button>
                <button
                  role="tab"
                  aria-selected={activeRoute === 'tasks'}
                  aria-controls="panel-tasks"
                  className={`header__tab ${activeRoute === 'tasks' ? 'header__tab--active' : ''}`}
                  onClick={() => handleRouteChange('tasks')}
                  disabled={!effectiveSelectedSession}
                >
                  Tasks
                </button>
                <button
                  role="tab"
                  aria-selected={activeRoute === 'logs'}
                  aria-controls="panel-logs"
                  className={`header__tab ${activeRoute === 'logs' ? 'header__tab--active' : ''}`}
                  onClick={() => handleRouteChange('logs')}
                  disabled={!effectiveSelectedSession}
                >
                  Logs
                </button>
              </nav>
            )}
            <ThemeToggle theme={theme} onThemeChange={setTheme} />
          </div>
        </header>
        <main className="main">
          {activeRoute === 'sessions' && (
            <div id="panel-sessions" role="tabpanel" aria-labelledby="tab-sessions">
              <SessionDashboard
                sessions={sessions}
                connectionState={connectionState}
                error={error}
                loading={loading}
                onStopSession={handleStopSession}
                onReconnect={reconnect}
                onRefresh={fetchSessions}
                onSessionSelect={handleSessionSelect}
                onViewLogs={handleViewLogs}
              />
            </div>
          )}
          {activeRoute === 'tasks' && (
            <div id="panel-tasks" role="tabpanel" aria-labelledby="tab-tasks">
              <KanbanBoard
                tasks={tasks}
                connectionState={connectionState}
                error={error}
                loading={loading}
                onUpdateTaskStatus={handleUpdateTaskStatus}
                onRefresh={fetchTasks}
              />
            </div>
          )}
          {activeRoute === 'logs' && (
            <div id="panel-logs" role="tabpanel" aria-labelledby="tab-logs">
              <SessionLogViewer
                logs={logs}
                totalLines={totalLines}
                sessionName={effectiveSelectedSession}
                logFile={logFile}
                loading={logsLoading}
                error={logsError}
                onRefresh={fetchLogs}
                onToggleRaw={handleToggleRawLogs}
                isRaw={showRawLogs}
              />
            </div>
          )}
          {activeRoute === 'orchestration' && (
            <div id="panel-orchestration" role="tabpanel" aria-labelledby="tab-orchestration">
              <AgentOrchestrationDashboard
                orchestrationState={orchestrationState}
                connectionState={connectionState}
                error={orchestrationError}
                loading={orchestrationLoading}
                onRefresh={fetchOrchestration}
              />
            </div>
          )}
          {activeRoute === 'settings' && (
            <div id="panel-settings" role="tabpanel" aria-labelledby="tab-settings">
              <div className="settings-page">
                <h2 className="settings-page__title">Settings</h2>
                <div className="settings-page__section">
                  <h3 className="settings-page__section-title">Theme</h3>
                  <p className="settings-page__description">
                    Choose your preferred theme for the Mission Control interface.
                  </p>
                  <ThemeToggle theme={theme} onThemeChange={setTheme} />
                </div>
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
