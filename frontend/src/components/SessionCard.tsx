import type { Session } from '../types/session';
import { StatusBadge } from './StatusBadge';

export interface SessionCardProps {
  session: Session;
  onStop?: (name: string) => void;
  onSelect?: (name: string) => void;
}

function formatRelativeTime(dateString?: string): string {
  if (!dateString) return 'N/A';

  try {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    const diffMin = Math.floor(diffSec / 60);
    const diffHour = Math.floor(diffMin / 60);
    const diffDay = Math.floor(diffHour / 24);

    if (diffSec < 60) return `${diffSec}s ago`;
    if (diffMin < 60) return `${diffMin}m ago`;
    if (diffHour < 24) return `${diffHour}h ago`;
    return `${diffDay}d ago`;
  } catch {
    return 'N/A';
  }
}

export function SessionCard({ session, onStop, onSelect }: SessionCardProps) {
  const canStop = session.status === 'running' && session.is_alive;
  const progress =
    session.max_iterations && session.iteration
      ? Math.round((session.iteration / session.max_iterations) * 100)
      : null;

  return (
    <article className="session-card" aria-labelledby={`session-${session.name}-title`}>
      <header className="session-card__header">
        <h3 id={`session-${session.name}-title`} className="session-card__title">
          {session.name}
        </h3>
        <StatusBadge status={session.status} isAlive={session.is_alive} />
      </header>

      <div className="session-card__body">
        <dl className="session-card__details">
          {session.dir && (
            <>
              <dt>Directory</dt>
              <dd className="session-card__path" title={session.dir}>
                {session.dir}
              </dd>
            </>
          )}

          {session.task_file && (
            <>
              <dt>Task File</dt>
              <dd>{session.task_file}</dd>
            </>
          )}

          <dt>Tasks Remaining</dt>
          <dd>{session.current_remaining}</dd>

          {session.iteration !== undefined && (
            <>
              <dt>Iteration</dt>
              <dd>
                {session.iteration}
                {session.max_iterations ? ` / ${session.max_iterations}` : ''}
              </dd>
            </>
          )}

          {session.last_task_id && (
            <>
              <dt>Current Task</dt>
              <dd>{session.last_task_id}</dd>
            </>
          )}

          {session.started_at && (
            <>
              <dt>Started</dt>
              <dd>{formatRelativeTime(session.started_at)}</dd>
            </>
          )}
        </dl>

        {progress !== null && (
          <div className="session-card__progress">
            <div className="session-card__progress-bar">
              <div
                className="session-card__progress-fill"
                style={{ width: `${progress}%` }}
                role="progressbar"
                aria-valuenow={progress}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label={`Progress: ${progress}%`}
              />
            </div>
            <span className="session-card__progress-label">{progress}%</span>
          </div>
        )}

        {session.last_log_line && (
          <div className="session-card__log">
            <span className="session-card__log-label">Last log:</span>
            <code className="session-card__log-content">{session.last_log_line}</code>
          </div>
        )}

        {session.last_error && (
          <div className="session-card__error">
            <span className="session-card__error-label">Error:</span>
            <code className="session-card__error-content">{session.last_error}</code>
          </div>
        )}
      </div>

      <footer className="session-card__footer">
        {onSelect && (
          <button
            className="session-card__button session-card__button--tasks"
            onClick={() => onSelect(session.name)}
            aria-label={`View tasks for session ${session.name}`}
          >
            View Tasks
          </button>
        )}
        {canStop && (
          <button
            className="session-card__button session-card__button--stop"
            onClick={() => onStop?.(session.name)}
            aria-label={`Stop session ${session.name}`}
          >
            Stop
          </button>
        )}
      </footer>
    </article>
  );
}
