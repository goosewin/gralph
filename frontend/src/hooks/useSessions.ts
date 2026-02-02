import { useCallback, useEffect, useState } from 'react';
import type { Session, StatusResponse } from '../types/session';

export interface UseSessionsOptions {
  baseUrl?: string;
  token?: string;
  autoFetch?: boolean;
}

export interface UseSessionsResult {
  sessions: Session[];
  loading: boolean;
  error: string | null;
  fetchSessions: () => Promise<void>;
  stopSession: (name: string) => Promise<boolean>;
}

export function useSessions({
  baseUrl = '',
  token,
  autoFetch = true,
}: UseSessionsOptions = {}): UseSessionsResult {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getHeaders = useCallback(() => {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
    return headers;
  }, [token]);

  const fetchSessions = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${baseUrl}/status`, {
        headers: getHeaders(),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const data: StatusResponse = await response.json();
      setSessions(data.sessions);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch sessions');
    } finally {
      setLoading(false);
    }
  }, [baseUrl, getHeaders]);

  const stopSession = useCallback(async (name: string): Promise<boolean> => {
    try {
      const response = await fetch(`${baseUrl}/stop/${encodeURIComponent(name)}`, {
        method: 'POST',
        headers: getHeaders(),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      // Update local state
      setSessions((prev) =>
        prev.map((s) =>
          s.name === name ? { ...s, status: 'stopped', is_alive: false } : s
        )
      );

      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop session');
      return false;
    }
  }, [baseUrl, getHeaders]);

  useEffect(() => {
    if (autoFetch) {
      fetchSessions();
    }
  }, [autoFetch, fetchSessions]);

  return {
    sessions,
    loading,
    error,
    fetchSessions,
    stopSession,
  };
}
