import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Session } from '../types/session';
import { SessionCard } from './SessionCard';

const mockRunningSession: Session = {
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
  last_error: 'Build failed: missing dependency',
};

describe('SessionCard', () => {
  it('renders session name', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('test-session')).toBeInTheDocument();
  });

  it('renders status badge', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('Running')).toBeInTheDocument();
  });

  it('renders directory path', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('/home/user/project')).toBeInTheDocument();
  });

  it('renders task file', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('PRD.md')).toBeInTheDocument();
  });

  it('renders tasks remaining', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('renders iteration count with max', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('3 / 10')).toBeInTheDocument();
  });

  it('renders current task ID', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('MC-1')).toBeInTheDocument();
  });

  it('renders last log line', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByText('Processing task...')).toBeInTheDocument();
  });

  it('renders last error for failed sessions', () => {
    render(<SessionCard session={mockFailedSession} />);
    expect(screen.getByText('Build failed: missing dependency')).toBeInTheDocument();
  });

  it('shows progress bar when max_iterations is set', () => {
    render(<SessionCard session={mockRunningSession} />);
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.getByText('30%')).toBeInTheDocument(); // 3/10 = 30%
  });

  it('shows stop button for running sessions', () => {
    const onStop = vi.fn();
    render(<SessionCard session={mockRunningSession} onStop={onStop} />);
    const stopButton = screen.getByRole('button', { name: /stop session test-session/i });
    expect(stopButton).toBeInTheDocument();
  });

  it('calls onStop when stop button is clicked', () => {
    const onStop = vi.fn();
    render(<SessionCard session={mockRunningSession} onStop={onStop} />);
    const stopButton = screen.getByRole('button', { name: /stop session test-session/i });
    fireEvent.click(stopButton);
    expect(onStop).toHaveBeenCalledWith('test-session');
  });

  it('does not show stop button for stopped sessions', () => {
    const onStop = vi.fn();
    render(<SessionCard session={mockStoppedSession} onStop={onStop} />);
    expect(screen.queryByRole('button', { name: /stop/i })).not.toBeInTheDocument();
  });

  it('does not show stop button when is_alive is false', () => {
    const onStop = vi.fn();
    const staleSession: Session = { ...mockRunningSession, is_alive: false };
    render(<SessionCard session={staleSession} onStop={onStop} />);
    expect(screen.queryByRole('button', { name: /stop/i })).not.toBeInTheDocument();
  });

  it('has accessible article structure', () => {
    render(<SessionCard session={mockRunningSession} />);
    const article = screen.getByRole('article');
    expect(article).toHaveAttribute('aria-labelledby', 'session-test-session-title');
  });
});
