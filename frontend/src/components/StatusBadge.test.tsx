import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { StatusBadge } from './StatusBadge';

describe('StatusBadge', () => {
  it('renders running status correctly', () => {
    render(<StatusBadge status="running" />);
    expect(screen.getByText('Running')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--running');
  });

  it('renders stopped status correctly', () => {
    render(<StatusBadge status="stopped" />);
    expect(screen.getByText('Stopped')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--stopped');
  });

  it('renders failed status correctly', () => {
    render(<StatusBadge status="failed" />);
    expect(screen.getByText('Failed')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--failed');
  });

  it('renders completed status correctly', () => {
    render(<StatusBadge status="completed" />);
    expect(screen.getByText('Completed')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--completed');
  });

  it('renders stale status correctly', () => {
    render(<StatusBadge status="stale" />);
    expect(screen.getByText('Stale')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--stale');
  });

  it('renders unknown status correctly', () => {
    render(<StatusBadge status="unknown" />);
    expect(screen.getByText('Unknown')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--unknown');
  });

  it('shows stale when running but not alive', () => {
    render(<StatusBadge status="running" isAlive={false} />);
    expect(screen.getByText('Stale')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--stale');
  });

  it('shows running when running and alive', () => {
    render(<StatusBadge status="running" isAlive={true} />);
    expect(screen.getByText('Running')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('status-badge--running');
  });

  it('applies custom className', () => {
    render(<StatusBadge status="running" className="custom-class" />);
    expect(screen.getByRole('status')).toHaveClass('custom-class');
  });

  it('has accessible label', () => {
    render(<StatusBadge status="running" />);
    expect(screen.getByRole('status')).toHaveAttribute('aria-label', 'Status: Running');
  });
});
