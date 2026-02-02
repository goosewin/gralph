import type { SessionStatus } from '../types/session';

export interface StatusBadgeProps {
  status: SessionStatus;
  isAlive?: boolean;
  className?: string;
}

const STATUS_CONFIG: Record<SessionStatus, { label: string; className: string }> = {
  running: { label: 'Running', className: 'status-badge--running' },
  stopped: { label: 'Stopped', className: 'status-badge--stopped' },
  failed: { label: 'Failed', className: 'status-badge--failed' },
  completed: { label: 'Completed', className: 'status-badge--completed' },
  stale: { label: 'Stale', className: 'status-badge--stale' },
  unknown: { label: 'Unknown', className: 'status-badge--unknown' },
};

export function StatusBadge({ status, isAlive, className = '' }: StatusBadgeProps) {
  const displayStatus = status === 'running' && isAlive === false ? 'stale' : status;
  const displayConfig = STATUS_CONFIG[displayStatus] || STATUS_CONFIG.unknown;

  return (
    <span
      className={`status-badge ${displayConfig.className} ${className}`}
      role="status"
      aria-label={`Status: ${displayConfig.label}`}
    >
      <span className="status-badge__indicator" aria-hidden="true" />
      <span className="status-badge__label">{displayConfig.label}</span>
    </span>
  );
}
