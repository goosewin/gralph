import { useMemo } from 'react';
import type { ConnectionState, Session } from '../types/session';
import { SessionCard } from './SessionCard';

export interface SessionDashboardProps {
  sessions: Session[];
  connectionState: ConnectionState;
  error: string | null;
  loading?: boolean;
  onStopSession?: (name: string) => void;
  onReconnect?: () => void;
  onRefresh?: () => void;
  onSessionSelect?: (name: string) => void;
  onViewLogs?: (name: string) => void;
}

export function SessionDashboard({
  sessions,
  connectionState,
  error,
  loading = false,
  onStopSession,
  onReconnect,
  onRefresh,
  onSessionSelect,
  onViewLogs,
}: SessionDashboardProps) {
  const sortedSessions = useMemo(() => {
    return [...sessions].sort((a, b) => {
      // Running sessions first
      if (a.status === 'running' && b.status !== 'running') return -1;
      if (b.status === 'running' && a.status !== 'running') return 1;
      // Then by name
      return a.name.localeCompare(b.name);
    });
  }, [sessions]);

  const stats = useMemo(() => {
    return {
      total: sessions.length,
      running: sessions.filter((s) => s.status === 'running' && s.is_alive).length,
      stopped: sessions.filter((s) => s.status === 'stopped').length,
      failed: sessions.filter((s) => s.status === 'failed').length,
      completed: sessions.filter((s) => s.status === 'completed').length,
    };
  }, [sessions]);

  return (
    <section className="session-dashboard" aria-label="Session Dashboard">
      <header className="session-dashboard__header">
        <h2 className="session-dashboard__title">Sessions</h2>
        <div className="session-dashboard__connection">
          <span
            className={`connection-indicator connection-indicator--${connectionState}`}
            aria-label={`Connection: ${connectionState}`}
          />
          <span className="connection-label">{connectionState}</span>
        </div>
      </header>

      {error && (
        <div className="session-dashboard__error" role="alert">
          <span className="session-dashboard__error-message">{error}</span>
          {connectionState === 'disconnected' && onReconnect && (
            <button
              className="session-dashboard__error-action"
              onClick={onReconnect}
              aria-label="Reconnect to server"
            >
              Reconnect
            </button>
          )}
        </div>
      )}

      <div className="session-dashboard__stats" role="group" aria-label="Session statistics">
        <div className="stat-item">
          <span className="stat-value">{stats.total}</span>
          <span className="stat-label">Total</span>
        </div>
        <div className="stat-item stat-item--running">
          <span className="stat-value">{stats.running}</span>
          <span className="stat-label">Running</span>
        </div>
        <div className="stat-item stat-item--completed">
          <span className="stat-value">{stats.completed}</span>
          <span className="stat-label">Completed</span>
        </div>
        <div className="stat-item stat-item--stopped">
          <span className="stat-value">{stats.stopped}</span>
          <span className="stat-label">Stopped</span>
        </div>
        <div className="stat-item stat-item--failed">
          <span className="stat-value">{stats.failed}</span>
          <span className="stat-label">Failed</span>
        </div>
      </div>

      <div className="session-dashboard__actions">
        {onRefresh && (
          <button
            className="session-dashboard__refresh"
            onClick={onRefresh}
            disabled={loading}
            aria-label="Refresh sessions"
          >
            {loading ? 'Loading...' : 'Refresh'}
          </button>
        )}
      </div>

      <div className="session-dashboard__content">
        {sortedSessions.length === 0 ? (
          <div className="session-dashboard__empty">
            <p>No sessions found.</p>
            <p className="session-dashboard__empty-hint">
              Start a new session with <code>gralph start</code>
            </p>
          </div>
        ) : (
          <div className="session-dashboard__grid">
            {sortedSessions.map((session) => (
              <SessionCard
                key={session.name}
                session={session}
                onStop={onStopSession}
                onSelect={onSessionSelect}
                onViewLogs={onViewLogs}
              />
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
