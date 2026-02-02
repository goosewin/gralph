import { renderHook, waitFor, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useOrchestration } from './useOrchestration';

declare const global: typeof globalThis;

const mockOrchestrationState = {
  state: {
    agents: [
      { id: 0, status: 'idle', specialization: 'general' },
      { id: 1, status: 'working', specialization: 'code-gen', current_task: 'task-1' },
    ],
    tasks: [
      { id: 'task-1', content: 'Test task', dependencies: [], status: 'in_progress', assigned_agent: 1 },
      { id: 'task-2', content: 'Pending task', dependencies: ['task-1'], status: 'pending' },
    ],
    queue_stats: {
      pending_count: 1,
      in_progress_count: 1,
      completed_count: 0,
    },
    max_agents: 10,
  },
};

describe('useOrchestration', () => {
  const mockFetch = vi.fn();
  const originalFetch = global.fetch;

  beforeEach(() => {
    global.fetch = mockFetch;
    mockFetch.mockReset();
  });

  afterEach(() => {
    global.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  describe('initial state', () => {
    it('initializes with null state', () => {
      const { result } = renderHook(() =>
        useOrchestration({ autoFetch: false, pollInterval: 0 })
      );

      expect(result.current.state).toBeNull();
      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
    });
  });

  describe('fetching', () => {
    it('fetches orchestration state on mount when autoFetch is true', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockOrchestrationState),
      });

      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: true, pollInterval: 0 })
      );

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
        expect(result.current.state).toEqual(mockOrchestrationState.state);
      });

      expect(mockFetch).toHaveBeenCalledWith('/orchestration', expect.any(Object));
      expect(result.current.error).toBeNull();
    });

    it('does not fetch on mount when autoFetch is false', async () => {
      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: false, pollInterval: 0 })
      );

      // State should remain null
      expect(result.current.state).toBeNull();
      expect(mockFetch).not.toHaveBeenCalled();
    });

    it('includes auth token in request headers', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockOrchestrationState),
      });

      const { result } = renderHook(() =>
        useOrchestration({
          baseUrl: '',
          token: 'test-token',
          autoFetch: true,
          pollInterval: 0,
        })
      );

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      expect(mockFetch).toHaveBeenCalledWith('/orchestration', {
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer test-token',
        },
      });
    });
  });

  describe('error handling', () => {
    it('sets error on fetch failure', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: true, pollInterval: 0 })
      );

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      expect(result.current.error).toBe('Network error');
    });

    it('handles 404 response gracefully with default state', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
      });

      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: true, pollInterval: 0 })
      );

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      // Should fall back to default state for 404
      expect(result.current.error).toBeNull();
      expect(result.current.state).not.toBeNull();
    });

    it('handles non-200 response as error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
      });

      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: true, pollInterval: 0 })
      );

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      expect(result.current.error).toContain('500');
    });
  });

  describe('manual fetch', () => {
    it('fetchOrchestration triggers a new fetch', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockOrchestrationState),
      });

      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: false, pollInterval: 0 })
      );

      expect(mockFetch).not.toHaveBeenCalled();

      // Manually trigger fetch
      await act(async () => {
        await result.current.fetchOrchestration();
      });

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledTimes(1);
        expect(result.current.state).toEqual(mockOrchestrationState.state);
      });
    });
  });

  describe('loading state', () => {
    it('sets loading to true during fetch', async () => {
      let resolveFetch: (value: unknown) => void;
      const fetchPromise = new Promise((resolve) => {
        resolveFetch = resolve;
      });

      mockFetch.mockReturnValueOnce(fetchPromise);

      const { result } = renderHook(() =>
        useOrchestration({ baseUrl: '', autoFetch: true, pollInterval: 0 })
      );

      // Loading should be true while fetching
      expect(result.current.loading).toBe(true);

      // Resolve the fetch
      await act(async () => {
        resolveFetch!({
          ok: true,
          json: () => Promise.resolve(mockOrchestrationState),
        });
      });

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });
    });
  });
});
