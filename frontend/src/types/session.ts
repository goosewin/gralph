/** Session status values */
export type SessionStatus = 'running' | 'stopped' | 'failed' | 'completed' | 'stale' | 'unknown';

/** Session data from the API */
export interface Session {
  name: string;
  status: SessionStatus;
  dir?: string;
  task_file?: string;
  pid?: number;
  is_alive: boolean;
  current_remaining: number;
  iteration?: number;
  max_iterations?: number;
  started_at?: string;
  completed_at?: string;
  log_file?: string;
  raw_log_file?: string;
  last_task_id?: string;
  last_log_line?: string;
  last_error?: string;
  tmux_session?: string;
}

/** WebSocket event types */
export type WebSocketEventType =
  | 'initial_state'
  | 'session_update'
  | 'session_created'
  | 'session_deleted'
  | 'sessions_refresh';

/** WebSocket message structure */
export interface WebSocketMessage {
  type: WebSocketEventType;
  session?: string;
  data: Session | Session[] | null;
}

/** API response for status endpoint */
export interface StatusResponse {
  sessions: Session[];
}

/** Connection state for WebSocket */
export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

/** Task status values for Kanban board */
export type TaskStatus = 'pending' | 'in_progress' | 'completed';

/** Task data from PRD file */
export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  context_bundle?: string[];
  definition_of_done?: string;
  checklist?: string[];
  dependencies?: string[];
}

/** API response for tasks endpoint */
export interface TasksResponse {
  tasks: Task[];
  task_file: string;
}

/** Request body for updating task status */
export interface UpdateTaskStatusRequest {
  status: TaskStatus;
}
