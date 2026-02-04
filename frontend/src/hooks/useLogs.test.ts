import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useLogs } from './useLogs';

describe('useLogs', () => {
  const mockFetch = vi.fn();
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    globalThis.fetch = mockFetch as typeof fetch;
    mockFetch.mockReset();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.useRealTimers();
  });

  const mockLogsResponse = {
    session: 'test-session',
    raw: false,
    total_lines: 100,
    offset: 0,
    limit: 100,
    lines: ['Line 1', 'Line 2', 'Line 3'],
    log_file: '/path/to/logs/test.log',
  };

  it('initializes with empty state', () => {
    const { result } = renderHook(() => useLogs({ autoFetch: false }));

    expect(result.current.logs).toEqual([]);
    expect(result.current.totalLines).toBe(0);
    expect(result.current.logFile).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('fetches logs successfully', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    const { result } = renderHook(() =>
      useLogs({ sessionName: 'test-session', autoFetch: true })
    );

    await waitFor(() => {
      expect(result.current.logs).toEqual(['Line 1', 'Line 2', 'Line 3']);
    });

    expect(result.current.totalLines).toBe(100);
    expect(result.current.logFile).toBe('/path/to/logs/test.log');
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('handles fetch error', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 404,
      json: () => Promise.resolve({ error: 'Session not found' }),
    });

    const { result } = renderHook(() =>
      useLogs({ sessionName: 'unknown-session', autoFetch: true })
    );

    await waitFor(() => {
      expect(result.current.error).toBe('Session not found');
    });

    expect(result.current.logs).toEqual([]);
    expect(result.current.loading).toBe(false);
  });

  it('handles network error', async () => {
    mockFetch.mockRejectedValueOnce(new Error('Network error'));

    const { result } = renderHook(() =>
      useLogs({ sessionName: 'test-session', autoFetch: true })
    );

    await waitFor(() => {
      expect(result.current.error).toBe('Network error');
    });
  });

  it('includes raw parameter when raw is true', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve({ ...mockLogsResponse, raw: true }),
    });

    renderHook(() => useLogs({ sessionName: 'test-session', raw: true, autoFetch: true }));

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    const fetchUrl = mockFetch.mock.calls[0][0];
    expect(fetchUrl).toContain('raw=true');
  });

  it('includes offset and limit in fetch', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    const { result } = renderHook(() =>
      useLogs({ sessionName: 'test-session', autoFetch: false })
    );

    await act(async () => {
      await result.current.fetchLogs({ offset: 50, limit: 25 });
    });

    const fetchUrl = mockFetch.mock.calls[0][0];
    expect(fetchUrl).toContain('offset=50');
    expect(fetchUrl).toContain('limit=25');
  });

  it('includes authorization header when token provided', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    renderHook(() => useLogs({ sessionName: 'test-session', token: 'my-token', autoFetch: true }));

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    const fetchOptions = mockFetch.mock.calls[0][1];
    expect(fetchOptions.headers['Authorization']).toBe('Bearer my-token');
  });

  it('clears logs on clearLogs call', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    const { result } = renderHook(() =>
      useLogs({ sessionName: 'test-session', autoFetch: true })
    );

    await waitFor(() => {
      expect(result.current.logs.length).toBeGreaterThan(0);
    });

    act(() => {
      result.current.clearLogs();
    });

    expect(result.current.logs).toEqual([]);
    expect(result.current.totalLines).toBe(0);
    expect(result.current.logFile).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('sets error when no session name provided', async () => {
    const { result } = renderHook(() => useLogs({ autoFetch: false }));

    await act(async () => {
      await result.current.fetchLogs();
    });

    expect(result.current.error).toBe('No session name provided');
  });

  it('uses base URL correctly', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    renderHook(() =>
      useLogs({
        baseUrl: 'http://localhost:8080',
        sessionName: 'test-session',
        autoFetch: true,
      })
    );

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    const fetchUrl = mockFetch.mock.calls[0][0] as string;
    expect(fetchUrl.startsWith('http://localhost:8080/logs/')).toBe(true);
  });

  it('encodes session name in URL', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    renderHook(() => useLogs({ sessionName: 'session with spaces', autoFetch: true }));

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    const fetchUrl = mockFetch.mock.calls[0][0];
    expect(fetchUrl).toContain('session%20with%20spaces');
  });

  it('clears logs when session changes to null', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockLogsResponse),
    });

    const { result, rerender } = renderHook(
      ({ sessionName }) => useLogs({ sessionName, autoFetch: true }),
      { initialProps: { sessionName: 'test-session' } }
    );

    await waitFor(() => {
      expect(result.current.logs.length).toBeGreaterThan(0);
    });

    rerender({ sessionName: undefined as unknown as string });

    expect(result.current.logs).toEqual([]);
  });
});
