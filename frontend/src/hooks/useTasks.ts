import { useCallback, useEffect, useState } from 'react';
import type { Task, TaskStatus, TasksResponse } from '../types/session';

export interface UseTasksOptions {
  baseUrl?: string;
  token?: string;
  sessionName?: string;
  autoFetch?: boolean;
}

export interface UseTasksResult {
  tasks: Task[];
  taskFile: string | null;
  loading: boolean;
  error: string | null;
  fetchTasks: () => Promise<void>;
  updateTaskStatus: (taskId: string, status: TaskStatus) => Promise<void>;
}

export function useTasks({
  baseUrl = '',
  token,
  sessionName,
  autoFetch = true,
}: UseTasksOptions = {}): UseTasksResult {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [taskFile, setTaskFile] = useState<string | null>(null);
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

  const fetchTasks = useCallback(async () => {
    if (!sessionName) {
      setError('No session selected');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(
        `${baseUrl}/tasks/${encodeURIComponent(sessionName)}`,
        { headers: getHeaders() }
      );

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const data: TasksResponse = await response.json();
      setTasks(data.tasks);
      setTaskFile(data.task_file);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch tasks');
    } finally {
      setLoading(false);
    }
  }, [baseUrl, sessionName, getHeaders]);

  const updateTaskStatus = useCallback(
    async (taskId: string, status: TaskStatus) => {
      if (!sessionName) {
        throw new Error('No session selected');
      }

      const response = await fetch(
        `${baseUrl}/tasks/${encodeURIComponent(sessionName)}/${encodeURIComponent(taskId)}/status`,
        {
          method: 'PUT',
          headers: getHeaders(),
          body: JSON.stringify({ status }),
        }
      );

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      // Optimistically update local state
      setTasks((prev) =>
        prev.map((t) => (t.id === taskId ? { ...t, status } : t))
      );
    },
    [baseUrl, sessionName, getHeaders]
  );

  useEffect(() => {
    if (autoFetch && sessionName) {
      fetchTasks();
    }
  }, [autoFetch, sessionName, fetchTasks]);

  return {
    tasks,
    taskFile,
    loading,
    error,
    fetchTasks,
    updateTaskStatus,
  };
}
