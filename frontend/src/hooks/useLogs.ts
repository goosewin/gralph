import { useCallback, useEffect, useRef, useState } from 'react';
import type { LogsResponse } from '../types/session';

export interface UseLogsOptions {
  baseUrl?: string;
  token?: string;
  sessionName?: string;
  /** Whether to fetch raw log instead of processed log */
  raw?: boolean;
  /** Polling interval in ms (0 to disable) */
  pollInterval?: number;
  /** Auto-fetch logs on mount/session change */
  autoFetch?: boolean;
}

export interface UseLogsResult {
  logs: string[];
  totalLines: number;
  logFile: string | null;
  loading: boolean;
  error: string | null;
  fetchLogs: (options?: { offset?: number; limit?: number }) => Promise<void>;
  clearLogs: () => void;
}

export function useLogs({
  baseUrl = '',
  token,
  sessionName,
  raw = false,
  pollInterval = 0,
  autoFetch = true,
}: UseLogsOptions = {}): UseLogsResult {
  const [logs, setLogs] = useState<string[]>([]);
  const [totalLines, setTotalLines] = useState(0);
  const [logFile, setLogFile] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const getHeaders = useCallback(() => {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
    return headers;
  }, [token]);

  const fetchLogs = useCallback(async (options?: { offset?: number; limit?: number }) => {
    if (!sessionName) {
      setError('No session name provided');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const queryParams = new URLSearchParams();
      if (raw) {
        queryParams.set('raw', 'true');
      }
      if (options?.offset !== undefined) {
        queryParams.set('offset', options.offset.toString());
      }
      if (options?.limit !== undefined) {
        queryParams.set('limit', options.limit.toString());
      }

      const url = `${baseUrl}/logs/${encodeURIComponent(sessionName)}${queryParams.toString() ? '?' + queryParams.toString() : ''}`;
      const response = await fetch(url, {
        headers: getHeaders(),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const data: LogsResponse = await response.json();
      setLogs(data.lines);
      setTotalLines(data.total_lines);
      setLogFile(data.log_file);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch logs');
    } finally {
      setLoading(false);
    }
  }, [baseUrl, sessionName, raw, getHeaders]);

  const clearLogs = useCallback(() => {
    setLogs([]);
    setTotalLines(0);
    setLogFile(null);
    setError(null);
  }, []);

  // Auto-fetch on session change
  useEffect(() => {
    if (autoFetch && sessionName) {
      fetchLogs();
    } else if (!sessionName) {
      clearLogs();
    }
  }, [autoFetch, sessionName, fetchLogs, clearLogs]);

  // Setup polling
  useEffect(() => {
    if (pollInterval > 0 && sessionName) {
      pollTimerRef.current = setInterval(() => {
        fetchLogs();
      }, pollInterval);
    }

    return () => {
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    };
  }, [pollInterval, sessionName, fetchLogs]);

  return {
    logs,
    totalLines,
    logFile,
    loading,
    error,
    fetchLogs,
    clearLogs,
  };
}
