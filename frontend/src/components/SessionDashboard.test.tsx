import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Session } from '../types/session';
import { SessionDashboard } from './SessionDashboard';

const mockSession: Session = {
  name: 'test-session',
  status: 'running',
  dir: '/home/user/project',
  task_file: 'PRD.md',
  pid: 12345,
  is_alive: true,
  current_remaining: 5,
  iteration: 3,
  max_iterations: 10,
  started_at: new Date().toISOString(),
  last_task_id: 'MC-1',
  last_log_line: 'Processing task...',
};

const mockStoppedSession: Session = {
  name: 'stopped-session',
  status: 'stopped',
  dir: '/home/user/project2',
  is_alive: false,
  current_remaining: 0,
};

const mockFailedSession: Session = {
  name: 'failed-session',
  status: 'failed',
  dir: '/home/user/project3',
  is_alive: false,
  current_remaining: 3,
  last_error: 'Build failed',
};

const mockCompletedSession: Session = {
  name: 'completed-session',
  status: 'completed',
  dir: '/home/user/project4',
  is_alive: false,
  current_remaining: 0,
};

describe('SessionDashboard', () => {
  it('renders empty state when no sessions exist', () => {
    render(
      <SessionDashboard
        sessions={[]}
        connectionState="connected"
        error={null}
      />
    );

    expect(screen.getByText('No sessions found.')).toBeInTheDocument();
    expect(screen.getByText(/gralph start/)).toBeInTheDocument();
  });

  it('renders session cards for each session', () => {
    render(
      <SessionDashboard
        sessions={[mockSession, mockStoppedSession]}
        connectionState="connected"
        error={null}
      />
    );

    expect(screen.getByText('test-session')).toBeInTheDocument();
    expect(screen.getByText('stopped-session')).toBeInTheDocument();
  });

  it('shows connection state indicator', () => {
    render(
      <SessionDashboard
        sessions={[]}
        connectionState="connected"
        error={null}
      />
    );

    expect(screen.getByText('connected')).toBeInTheDocument();
  });

  it('displays error message when error prop is provided', () => {
    render(
      <SessionDashboard
        sessions={[]}
        connectionState="error"
        error="Connection failed"
      />
    );

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('Connection failed')).toBeInTheDocument();
  });

  it('shows reconnect button when disconnected with error', () => {
    const onReconnect = vi.fn();
    render(
      <SessionDashboard
        sessions={[]}
        connectionState="disconnected"
        error="Connection lost"
        onReconnect={onReconnect}
      />
    );

    const reconnectButton = screen.getByRole('button', { name: /reconnect/i });
    expect(reconnectButton).toBeInTheDocument();
  });

  it('displays session statistics', () => {
    render(
      <SessionDashboard
        sessions={[mockSession, mockStoppedSession, mockFailedSession, mockCompletedSession]}
        connectionState="connected"
        error={null}
      />
    );

    // Check that the stats group contains the expected labels
    const statsGroup = screen.getByRole('group', { name: 'Session statistics' });
    expect(statsGroup).toBeInTheDocument();

    // Check for stat labels within the stats group
    expect(within(statsGroup).getByText('Total')).toBeInTheDocument();
    expect(within(statsGroup).getByText('Running')).toBeInTheDocument();
    expect(within(statsGroup).getByText('Stopped')).toBeInTheDocument();
    expect(within(statsGroup).getByText('Failed')).toBeInTheDocument();
    expect(within(statsGroup).getByText('Completed')).toBeInTheDocument();

    // Check that the total is 4
    expect(within(statsGroup).getByText('4')).toBeInTheDocument();

    // Check that there are exactly 4 elements with value "1" within stats (running, stopped, failed, completed each have 1)
    const onesElements = within(statsGroup).getAllByText('1');
    expect(onesElements.length).toBe(4);
  });

  it('shows refresh button when onRefresh is provided', () => {
    const onRefresh = vi.fn();
    render(
      <SessionDashboard
        sessions={[]}
        connectionState="connected"
        error={null}
        onRefresh={onRefresh}
      />
    );

    expect(screen.getByRole('button', { name: /refresh/i })).toBeInTheDocument();
  });

  it('disables refresh button when loading', () => {
    const onRefresh = vi.fn();
    render(
      <SessionDashboard
        sessions={[]}
        connectionState="connected"
        error={null}
        loading={true}
        onRefresh={onRefresh}
      />
    );

    expect(screen.getByRole('button', { name: /refresh/i })).toBeDisabled();
  });

  it('sorts sessions with running first', () => {
    render(
      <SessionDashboard
        sessions={[mockStoppedSession, mockSession, mockFailedSession]}
        connectionState="connected"
        error={null}
      />
    );

    const titles = screen.getAllByRole('heading', { level: 3 });
    expect(titles[0]).toHaveTextContent('test-session'); // running comes first
  });
});
