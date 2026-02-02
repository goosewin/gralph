import { useCallback, useMemo } from 'react';
import { SessionDashboard } from './components/SessionDashboard';
import { useSessions } from './hooks/useSessions';
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

  const {
    loading,
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

  const handleStopSession = useCallback(async (name: string) => {
    await stopSession(name);
  }, [stopSession]);

  const error = wsError || fetchError;

  return (
    <div className="app">
      <header className="header">
        <h1>Gralph Mission Control</h1>
      </header>
      <main className="main">
        <SessionDashboard
          sessions={sessions}
          connectionState={connectionState}
          error={error}
          loading={loading}
          onStopSession={handleStopSession}
          onReconnect={reconnect}
          onRefresh={fetchSessions}
        />
      </main>
    </div>
  );
}

export default App;
