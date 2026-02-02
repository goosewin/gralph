import { useCallback, useEffect, useMemo, useState } from 'react';
import { HamburgerMenu } from './components/HamburgerMenu';
import { InstallPrompt } from './components/InstallPrompt';
import { KanbanBoard } from './components/KanbanBoard';
import { OfflineIndicator } from './components/OfflineIndicator';
import { SessionDashboard } from './components/SessionDashboard';
import { Sidebar, type Route, type SidebarSection } from './components/Sidebar';
import { ThemeToggle } from './components/ThemeToggle';
import { useBreakpoints } from './hooks/useMediaQuery';
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

  const handleStopSession = useCallback(async (name: string) => {
    await stopSession(name);
  }, [stopSession]);

  const handleSessionSelect = useCallback((sessionName: string) => {
    setSelectedSession(sessionName);
    setActiveRoute('tasks');
  }, []);

  const handleUpdateTaskStatus = useCallback(async (taskId: string, newStatus: 'pending' | 'in_progress' | 'completed') => {
    await updateTaskStatus(taskId, newStatus);
  }, [updateTaskStatus]);

  const handleRouteChange = useCallback((route: Route) => {
    setActiveRoute(route);
    if (route === 'tasks' && selectedSession) {
      fetchTasks();
    }
  }, [selectedSession, fetchTasks]);

  const error = wsError || fetchError || tasksError;
  const loading = sessionsLoading || tasksLoading;

  // Auto-select first running session if none selected
  const effectiveSelectedSession = selectedSession ??
    sessions.find(s => s.status === 'running')?.name ??
    sessions[0]?.name;

  // Count sessions by status for badges
  const runningCount = useMemo(
    () => sessions.filter((s) => s.status === 'running').length,
    [sessions]
  );

  const taskCount = useMemo(() => tasks.length, [tasks]);

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
    [runningCount, taskCount, effectiveSelectedSession]
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
