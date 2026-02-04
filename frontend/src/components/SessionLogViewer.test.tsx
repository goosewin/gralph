import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SessionLogViewer } from './SessionLogViewer';

describe('SessionLogViewer', () => {
  const defaultProps = {
    logs: [],
    totalLines: 0,
  };

  beforeEach(() => {
    // Mock ResizeObserver
    (globalThis as typeof globalThis & { ResizeObserver: typeof ResizeObserver }).ResizeObserver =
      vi.fn().mockImplementation(() => ({
        observe: vi.fn(),
        unobserve: vi.fn(),
        disconnect: vi.fn(),
      }));
  });

  it('renders empty state when no logs', () => {
    render(<SessionLogViewer {...defaultProps} />);
    expect(screen.getByText('No log content available')).toBeInTheDocument();
  });

  it('renders session name in title', () => {
    render(<SessionLogViewer {...defaultProps} sessionName="test-session" />);
    expect(screen.getByText('Session Logs: test-session')).toBeInTheDocument();
  });

  it('renders log file name', () => {
    render(
      <SessionLogViewer
        {...defaultProps}
        logFile="/path/to/logs/session.log"
      />
    );
    expect(screen.getByText('session.log')).toBeInTheDocument();
  });

  it('renders total line count', () => {
    render(<SessionLogViewer {...defaultProps} totalLines={12345} />);
    expect(screen.getByText('12,345 lines')).toBeInTheDocument();
  });

  it('renders log lines with line numbers', () => {
    const logs = ['First line', 'Second line', 'Third line'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={3} />);

    expect(screen.getByText('First line')).toBeInTheDocument();
    expect(screen.getByText('Second line')).toBeInTheDocument();
    expect(screen.getByText('Third line')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('renders loading state', () => {
    render(<SessionLogViewer {...defaultProps} loading={true} />);
    expect(screen.getByText('Loading logs...')).toBeInTheDocument();
  });

  it('renders error message', () => {
    render(<SessionLogViewer {...defaultProps} error="Failed to load logs" />);
    expect(screen.getByText('Failed to load logs')).toBeInTheDocument();
  });

  it('calls onRefresh when refresh button clicked', () => {
    const onRefresh = vi.fn();
    render(<SessionLogViewer {...defaultProps} onRefresh={onRefresh} />);

    fireEvent.click(screen.getByText('Refresh'));
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it('disables refresh button when loading', () => {
    const onRefresh = vi.fn();
    render(
      <SessionLogViewer
        {...defaultProps}
        onRefresh={onRefresh}
        loading={true}
      />
    );

    const button = screen.getByText('Loading...');
    expect(button).toBeDisabled();
  });

  it('renders auto-scroll toggle button', () => {
    render(<SessionLogViewer {...defaultProps} />);
    expect(screen.getByText('Auto-scroll')).toBeInTheDocument();
  });

  it('toggles auto-scroll state when clicked', () => {
    render(<SessionLogViewer {...defaultProps} />);
    const button = screen.getByText('Auto-scroll');

    // Initially active
    expect(button).toHaveClass('log-viewer__control-btn--active');

    fireEvent.click(button);

    // Should become inactive
    expect(button).not.toHaveClass('log-viewer__control-btn--active');
  });

  it('renders raw toggle when onToggleRaw provided', () => {
    const onToggleRaw = vi.fn();
    render(<SessionLogViewer {...defaultProps} onToggleRaw={onToggleRaw} />);
    expect(screen.getByText('Raw')).toBeInTheDocument();
  });

  it('calls onToggleRaw when raw button clicked', () => {
    const onToggleRaw = vi.fn();
    render(
      <SessionLogViewer
        {...defaultProps}
        onToggleRaw={onToggleRaw}
        isRaw={false}
      />
    );

    fireEvent.click(screen.getByText('Raw'));
    expect(onToggleRaw).toHaveBeenCalledWith(true);
  });

  it('shows search input', () => {
    render(<SessionLogViewer {...defaultProps} />);
    expect(
      screen.getByPlaceholderText('Search logs... (Enter for next, Shift+Enter for prev)')
    ).toBeInTheDocument();
  });

  it('shows search results count when searching', () => {
    const logs = ['Line with error', 'Normal line', 'Another error here'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={3} />);

    const searchInput = screen.getByPlaceholderText(
      'Search logs... (Enter for next, Shift+Enter for prev)'
    );
    fireEvent.change(searchInput, { target: { value: 'error' } });

    expect(screen.getByText('1 / 2')).toBeInTheDocument();
  });

  it('shows no results message when search has no matches', () => {
    const logs = ['First line', 'Second line'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={2} />);

    const searchInput = screen.getByPlaceholderText(
      'Search logs... (Enter for next, Shift+Enter for prev)'
    );
    fireEvent.change(searchInput, { target: { value: 'nonexistent' } });

    expect(screen.getByText('No results')).toBeInTheDocument();
  });

  it('highlights error lines', () => {
    const logs = ['Error: Something went wrong'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={1} />);

    const line = screen.getByText('Error: Something went wrong').closest('.log-viewer__line');
    expect(line).toHaveClass('log-viewer__line--error');
  });

  it('highlights warning lines', () => {
    const logs = ['Warning: This is a warning'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={1} />);

    const line = screen.getByText('Warning: This is a warning').closest('.log-viewer__line');
    expect(line).toHaveClass('log-viewer__line--warning');
  });

  it('highlights success lines', () => {
    const logs = ['Task completed successfully'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={1} />);

    const line = screen.getByText('Task completed successfully').closest('.log-viewer__line');
    expect(line).toHaveClass('log-viewer__line--success');
  });

  it('highlights info lines', () => {
    const logs = ['Iteration 1 starting'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={1} />);

    const line = screen.getByText('Iteration 1 starting').closest('.log-viewer__line');
    expect(line).toHaveClass('log-viewer__line--info');
  });

  it('renders scroll to top button', () => {
    render(<SessionLogViewer {...defaultProps} />);
    expect(screen.getByText('Top')).toBeInTheDocument();
  });

  it('renders scroll to bottom button', () => {
    render(<SessionLogViewer {...defaultProps} />);
    expect(screen.getByText('Bottom')).toBeInTheDocument();
  });

  it('has correct aria labels for accessibility', () => {
    render(<SessionLogViewer {...defaultProps} logs={['test']} totalLines={1} />);

    expect(screen.getByRole('log')).toHaveAttribute(
      'aria-label',
      'Session log output'
    );
    expect(screen.getByLabelText('Search logs')).toBeInTheDocument();
  });

  it('clears search on Escape key', () => {
    const logs = ['Line with text'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={1} />);

    const searchInput = screen.getByPlaceholderText(
      'Search logs... (Enter for next, Shift+Enter for prev)'
    );

    fireEvent.change(searchInput, { target: { value: 'text' } });
    expect(searchInput).toHaveValue('text');

    fireEvent.keyDown(searchInput, { key: 'Escape' });
    expect(searchInput).toHaveValue('');
  });

  it('navigates to next search result on Enter', () => {
    const logs = ['error one', 'normal', 'error two', 'error three'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={4} />);

    const searchInput = screen.getByPlaceholderText(
      'Search logs... (Enter for next, Shift+Enter for prev)'
    );
    fireEvent.change(searchInput, { target: { value: 'error' } });

    // Initial: 1 / 3
    expect(screen.getByText('1 / 3')).toBeInTheDocument();

    // Press Enter to go to next
    fireEvent.keyDown(searchInput, { key: 'Enter' });
    expect(screen.getByText('2 / 3')).toBeInTheDocument();
  });

  it('navigates to previous search result on Shift+Enter', () => {
    const logs = ['error one', 'normal', 'error two'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={3} />);

    const searchInput = screen.getByPlaceholderText(
      'Search logs... (Enter for next, Shift+Enter for prev)'
    );
    fireEvent.change(searchInput, { target: { value: 'error' } });

    // Initial: 1 / 2
    expect(screen.getByText('1 / 2')).toBeInTheDocument();

    // Press Shift+Enter to go to previous (wraps to last)
    fireEvent.keyDown(searchInput, { key: 'Enter', shiftKey: true });
    expect(screen.getByText('2 / 2')).toBeInTheDocument();
  });

  it('shows search navigation buttons when results exist', () => {
    const logs = ['error one', 'error two'];
    render(<SessionLogViewer {...defaultProps} logs={logs} totalLines={2} />);

    const searchInput = screen.getByPlaceholderText(
      'Search logs... (Enter for next, Shift+Enter for prev)'
    );
    fireEvent.change(searchInput, { target: { value: 'error' } });

    expect(screen.getByLabelText('Previous result')).toBeInTheDocument();
    expect(screen.getByLabelText('Next result')).toBeInTheDocument();
  });

  it('shows auto-scroll indicator when enabled', () => {
    render(<SessionLogViewer {...defaultProps} />);
    expect(screen.getByText('Auto-scrolling enabled')).toBeInTheDocument();
  });

  it('hides auto-scroll indicator when disabled', () => {
    render(<SessionLogViewer {...defaultProps} />);

    // Click to disable auto-scroll
    fireEvent.click(screen.getByText('Auto-scroll'));

    expect(screen.queryByText('Auto-scrolling enabled')).not.toBeInTheDocument();
  });
});
