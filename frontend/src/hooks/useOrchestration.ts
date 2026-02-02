import { useCallback, useEffect, useState } from 'react';
import type { OrchestrationState } from '../types/session';

export interface UseOrchestrationOptions {
  baseUrl?: string;
  token?: string;
  autoFetch?: boolean;
  pollInterval?: number;
}

export interface UseOrchestrationResult {
  state: OrchestrationState | null;
  loading: boolean;
  error: string | null;
  fetchOrchestration: () => Promise<void>;
}

const defaultState: OrchestrationState = {
  agents: [],
  tasks: [],
  queue_stats: {
    pending_count: 0,
    in_progress_count: 0,
    completed_count: 0,
  },
  max_agents: 10,
};

export function useOrchestration({
  baseUrl = '',
  token,
  autoFetch = true,
  pollInterval = 2000,
}: UseOrchestrationOptions = {}): UseOrchestrationResult {
  const [state, setState] = useState<OrchestrationState | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchOrchestration = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const headers: HeadersInit = {
        'Content-Type': 'application/json',
      };
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch(`${baseUrl}/orchestration`, { headers });

      if (!response.ok) {
        // If endpoint doesn't exist yet, return mock data for development
        if (response.status === 404) {
          setState(defaultState);
          return;
        }
        throw new Error(`Failed to fetch orchestration: ${response.status}`);
      }

      const data = await response.json();
      setState(data.state || defaultState);
    } catch (err) {
      // Gracefully handle missing endpoint during development
      if (err instanceof TypeError && err.message.includes('fetch')) {
        setState(defaultState);
        return;
      }
      setError(err instanceof Error ? err.message : 'Unknown error');
      // Fall back to default state on error
      setState(defaultState);
    } finally {
      setLoading(false);
    }
  }, [baseUrl, token]);

  // Auto-fetch on mount
  useEffect(() => {
    if (autoFetch) {
      fetchOrchestration();
    }
  }, [autoFetch, fetchOrchestration]);

  // Poll for updates
  useEffect(() => {
    if (pollInterval <= 0) return;

    const interval = setInterval(() => {
      fetchOrchestration();
    }, pollInterval);

    return () => clearInterval(interval);
  }, [pollInterval, fetchOrchestration]);

  return {
    state,
    loading,
    error,
    fetchOrchestration,
  };
}
