import { useCallback, useMemo, useState } from 'react';
import { KanbanBoard } from './components/KanbanBoard';
import { SessionDashboard } from './components/SessionDashboard';
import { ThemeToggle } from './components/ThemeToggle';
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

type Tab = 'sessions' | 'tasks';

function App() {
  const token = useMemo(() => getToken(), []);
  const wsUrl = useMemo(() => getWebSocketUrl(), []);
  const baseUrl = useMemo(() => getBaseUrl(), []);

  const [activeTab, setActiveTab] = useState<Tab>('sessions');
  const [selectedSession, setSelectedSession] = useState<string | null>(null);

  const { theme, setTheme } = useTheme();

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
    setActiveTab('tasks');
  }, []);

  const handleUpdateTaskStatus = useCallback(async (taskId: string, newStatus: 'pending' | 'in_progress' | 'completed') => {
    await updateTaskStatus(taskId, newStatus);
  }, [updateTaskStatus]);

  const handleTabChange = useCallback((tab: Tab) => {
    setActiveTab(tab);
    if (tab === 'tasks' && selectedSession) {
      fetchTasks();
    }
  }, [selectedSession, fetchTasks]);

  const error = wsError || fetchError || tasksError;
  const loading = sessionsLoading || tasksLoading;

  // Auto-select first running session if none selected
  const effectiveSelectedSession = selectedSession ??
    sessions.find(s => s.status === 'running')?.name ??
    sessions[0]?.name;

  return (
    <div className="app">
      <header className="header">
        <h1>Gralph Mission Control</h1>
        <div className="header__right">
          <nav className="header__nav" role="tablist" aria-label="Main navigation">
            <button
              role="tab"
              aria-selected={activeTab === 'sessions'}
              aria-controls="panel-sessions"
              className={`header__tab ${activeTab === 'sessions' ? 'header__tab--active' : ''}`}
              onClick={() => handleTabChange('sessions')}
            >
              Sessions
            </button>
            <button
              role="tab"
              aria-selected={activeTab === 'tasks'}
              aria-controls="panel-tasks"
              className={`header__tab ${activeTab === 'tasks' ? 'header__tab--active' : ''}`}
              onClick={() => handleTabChange('tasks')}
              disabled={!effectiveSelectedSession}
            >
              Tasks {effectiveSelectedSession ? `(${effectiveSelectedSession})` : ''}
            </button>
          </nav>
          <ThemeToggle theme={theme} onThemeChange={setTheme} />
        </div>
      </header>
      <main className="main">
        {activeTab === 'sessions' && (
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
        {activeTab === 'tasks' && (
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
      </main>
    </div>
  );
}

export default App;
