import { useCallback, useEffect, useRef, useState } from 'react';
import type { ConnectionState, Session, WebSocketMessage } from '../types/session';

export interface UseWebSocketOptions {
  url: string;
  token?: string;
  onMessage?: (message: WebSocketMessage) => void;
  onSessionsUpdate?: (sessions: Session[]) => void;
  reconnectDelay?: number;
  maxReconnectAttempts?: number;
}

export interface UseWebSocketResult {
  connectionState: ConnectionState;
  sessions: Session[];
  error: string | null;
  reconnect: () => void;
}

export function useWebSocket({
  url,
  token,
  onMessage,
  onSessionsUpdate,
  reconnectDelay = 2000,
  maxReconnectAttempts = 5,
}: UseWebSocketOptions): UseWebSocketResult {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [sessions, setSessions] = useState<Session[]>([]);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimeoutRef = useRef<number | null>(null);

  const buildUrl = useCallback(() => {
    const wsUrl = new URL(url);
    if (token) {
      wsUrl.searchParams.set('token', token);
    }
    return wsUrl.toString();
  }, [url, token]);

  const handleMessage = useCallback((event: MessageEvent) => {
    try {
      const message = JSON.parse(event.data) as WebSocketMessage;
      onMessage?.(message);

      switch (message.type) {
        case 'initial_state':
          if (Array.isArray(message.data)) {
            setSessions(message.data);
            onSessionsUpdate?.(message.data);
          }
          break;

        case 'session_update':
          if (message.session && message.data && !Array.isArray(message.data)) {
            setSessions((prev) => {
              const updated = prev.map((s) =>
                s.name === message.session ? (message.data as Session) : s
              );
              onSessionsUpdate?.(updated);
              return updated;
            });
          }
          break;

        case 'session_created':
          if (message.data && !Array.isArray(message.data)) {
            setSessions((prev) => {
              const updated = [...prev, message.data as Session];
              onSessionsUpdate?.(updated);
              return updated;
            });
          }
          break;

        case 'session_deleted':
          if (message.session) {
            setSessions((prev) => {
              const updated = prev.filter((s) => s.name !== message.session);
              onSessionsUpdate?.(updated);
              return updated;
            });
          }
          break;

        case 'sessions_refresh':
          if (Array.isArray(message.data)) {
            setSessions(message.data);
            onSessionsUpdate?.(message.data);
          }
          break;
      }
    } catch (err) {
      console.error('Failed to parse WebSocket message:', err);
    }
  }, [onMessage, onSessionsUpdate]);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    setConnectionState('connecting');
    setError(null);

    try {
      const ws = new WebSocket(buildUrl());
      wsRef.current = ws;

      ws.onopen = () => {
        setConnectionState('connected');
        setError(null);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = handleMessage;

      ws.onerror = () => {
        setConnectionState('error');
        setError('WebSocket connection error');
      };

      ws.onclose = () => {
        setConnectionState('disconnected');
        wsRef.current = null;

        // Attempt to reconnect with exponential backoff
        if (reconnectAttemptsRef.current < maxReconnectAttempts) {
          const delay = reconnectDelay * Math.pow(2, reconnectAttemptsRef.current);
          reconnectAttemptsRef.current++;

          reconnectTimeoutRef.current = window.setTimeout(() => {
            connect();
          }, delay);
        } else {
          setError('Max reconnection attempts reached');
        }
      };
    } catch (err) {
      setConnectionState('error');
      setError(err instanceof Error ? err.message : 'Failed to connect');
    }
  }, [buildUrl, handleMessage, maxReconnectAttempts, reconnectDelay]);

  const reconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      window.clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    reconnectAttemptsRef.current = 0;

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    connect();
  }, [connect]);

  useEffect(() => {
    connect();

    return () => {
      if (reconnectTimeoutRef.current) {
        window.clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [connect]);

  return {
    connectionState,
    sessions,
    error,
    reconnect,
  };
}
