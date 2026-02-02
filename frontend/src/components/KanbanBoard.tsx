import { useCallback, useMemo, useRef, useState } from 'react';
import { useBreakpoints } from '../hooks/useMediaQuery';
import { useTouchSwipe } from '../hooks/useTouchSwipe';
import type { Task, TaskStatus, ConnectionState } from '../types/session';
import { KanbanColumn } from './KanbanColumn';

export interface KanbanBoardProps {
  tasks: Task[];
  connectionState: ConnectionState;
  error: string | null;
  loading?: boolean;
  onUpdateTaskStatus?: (taskId: string, newStatus: TaskStatus) => Promise<void>;
  onRefresh?: () => void;
}

interface ColumnConfig {
  status: TaskStatus;
  title: string;
}

const COLUMNS: ColumnConfig[] = [
  { status: 'pending', title: 'Pending' },
  { status: 'in_progress', title: 'In Progress' },
  { status: 'completed', title: 'Completed' },
];

const STATUS_ORDER: TaskStatus[] = ['pending', 'in_progress', 'completed'];

export function KanbanBoard({
  tasks,
  connectionState,
  error,
  loading = false,
  onUpdateTaskStatus,
  onRefresh,
}: KanbanBoardProps) {
  const [draggingTaskId, setDraggingTaskId] = useState<string | null>(null);
  const [announcement, setAnnouncement] = useState<string>('');
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const announcementTimeoutRef = useRef<number | null>(null);
  const { isTouchDevice, isMobile } = useBreakpoints();

  // Group tasks by status
  const tasksByStatus = useMemo(() => {
    const grouped: Record<TaskStatus, Task[]> = {
      pending: [],
      in_progress: [],
      completed: [],
    };
    for (const task of tasks) {
      if (grouped[task.status]) {
        grouped[task.status].push(task);
      } else {
        grouped.pending.push(task);
      }
    }
    return grouped;
  }, [tasks]);

  // Announce to screen readers
  const announce = useCallback((message: string) => {
    if (announcementTimeoutRef.current) {
      clearTimeout(announcementTimeoutRef.current);
    }
    setAnnouncement(message);
    announcementTimeoutRef.current = window.setTimeout(() => {
      setAnnouncement('');
    }, 1000);
  }, []);

  // Handle swipe to move selected task
  const handleSwipeMove = useCallback(
    async (direction: 'left' | 'right') => {
      if (!selectedTaskId || !onUpdateTaskStatus) return;

      const task = tasks.find((t) => t.id === selectedTaskId);
      if (!task) return;

      const currentIndex = STATUS_ORDER.indexOf(task.status);
      let newIndex: number;

      if (direction === 'left') {
        newIndex = Math.max(0, currentIndex - 1);
      } else {
        newIndex = Math.min(STATUS_ORDER.length - 1, currentIndex + 1);
      }

      if (newIndex !== currentIndex) {
        const newStatus = STATUS_ORDER[newIndex];
        const columnTitle = COLUMNS.find((c) => c.status === newStatus)?.title || newStatus;
        try {
          await onUpdateTaskStatus(selectedTaskId, newStatus);
          announce(`Moved task ${task.id} to ${columnTitle}`);
        } catch (err) {
          announce(`Failed to move task ${task.id}`);
        }
      }
    },
    [selectedTaskId, tasks, onUpdateTaskStatus, announce]
  );

  // Touch swipe gesture support
  const { handlers: swipeHandlers, swiping } = useTouchSwipe({
    enabled: isTouchDevice && !!selectedTaskId,
    minSwipeDistance: 50,
    onSwipeLeft: () => handleSwipeMove('right'), // Swipe left moves task right (forward)
    onSwipeRight: () => handleSwipeMove('left'), // Swipe right moves task left (backward)
  });

  // Handle task selection for touch swipe
  const handleTaskTouch = useCallback((taskId: string) => {
    setSelectedTaskId((prev) => (prev === taskId ? null : taskId));
  }, []);

  // Drag and drop handlers
  const handleDragStart = useCallback((taskId: string, _event: React.DragEvent) => {
    setDraggingTaskId(taskId);
    const task = tasks.find((t) => t.id === taskId);
    if (task) {
      announce(`Picked up task ${task.id}: ${task.title}`);
    }
  }, [tasks, announce]);

  const handleDragEnd = useCallback((_event: React.DragEvent) => {
    setDraggingTaskId(null);
    announce('Dropped');
  }, [announce]);

  const handleDragOver = useCallback((_status: TaskStatus, _event: React.DragEvent) => {
    // Handled in column component
  }, []);

  const handleDrop = useCallback(async (targetStatus: TaskStatus, event: React.DragEvent) => {
    event.preventDefault();
    const taskId = event.dataTransfer.getData('text/plain');
    const task = tasks.find((t) => t.id === taskId);

    if (!task || task.status === targetStatus) {
      setDraggingTaskId(null);
      return;
    }

    const columnTitle = COLUMNS.find((c) => c.status === targetStatus)?.title || targetStatus;
    announce(`Moved task ${task.id} to ${columnTitle}`);

    if (onUpdateTaskStatus) {
      try {
        await onUpdateTaskStatus(taskId, targetStatus);
      } catch (err) {
        announce(`Failed to move task ${task.id}`);
      }
    }

    setDraggingTaskId(null);
  }, [tasks, onUpdateTaskStatus, announce]);

  // Keyboard navigation
  const handleTaskKeyDown = useCallback(async (taskId: string, event: React.KeyboardEvent) => {
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    // Arrow left/right to move between columns
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      const currentIndex = STATUS_ORDER.indexOf(task.status);
      let newIndex: number;

      if (event.key === 'ArrowLeft') {
        newIndex = Math.max(0, currentIndex - 1);
      } else {
        newIndex = Math.min(STATUS_ORDER.length - 1, currentIndex + 1);
      }

      if (newIndex !== currentIndex && onUpdateTaskStatus) {
        const newStatus = STATUS_ORDER[newIndex];
        const columnTitle = COLUMNS.find((c) => c.status === newStatus)?.title || newStatus;
        announce(`Moving task ${task.id} to ${columnTitle}`);
        try {
          await onUpdateTaskStatus(taskId, newStatus);
          announce(`Moved task ${task.id} to ${columnTitle}`);
        } catch (err) {
          announce(`Failed to move task ${task.id}`);
        }
      }
      return;
    }

    // Arrow up/down to move focus within column
    if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      const columnTasks = tasksByStatus[task.status];
      const currentTaskIndex = columnTasks.findIndex((t) => t.id === taskId);
      let newTaskIndex: number;

      if (event.key === 'ArrowUp') {
        newTaskIndex = Math.max(0, currentTaskIndex - 1);
      } else {
        newTaskIndex = Math.min(columnTasks.length - 1, currentTaskIndex + 1);
      }

      if (newTaskIndex !== currentTaskIndex) {
        const nextTask = columnTasks[newTaskIndex];
        const nextElement = document.querySelector(`[data-task-id="${nextTask.id}"]`) as HTMLElement;
        nextElement?.focus();
      }
      return;
    }

    // Space or Enter to announce current task
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      const columnTitle = COLUMNS.find((c) => c.status === task.status)?.title || task.status;
      announce(`Task ${task.id}: ${task.title}. Currently in ${columnTitle}. Use arrow keys to move.`);
    }
  }, [tasks, tasksByStatus, onUpdateTaskStatus, announce]);

  return (
    <section className="kanban-board" aria-label="Kanban Board">
      {/* Screen reader announcement region */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="kanban-board__announcement sr-only"
      >
        {announcement}
      </div>

      <header className="kanban-board__header">
        <h2 className="kanban-board__title">PRD Tasks</h2>
        <div className="kanban-board__connection">
          <span
            className={`connection-indicator connection-indicator--${connectionState}`}
            aria-label={`Connection: ${connectionState}`}
          />
          <span className="connection-label">{connectionState}</span>
        </div>
      </header>

      {error && (
        <div className="kanban-board__error" role="alert">
          <span className="kanban-board__error-message">{error}</span>
        </div>
      )}

      <div className="kanban-board__actions">
        {onRefresh && (
          <button
            className="kanban-board__refresh"
            onClick={onRefresh}
            disabled={loading}
            aria-label="Refresh tasks"
          >
            {loading ? 'Loading...' : 'Refresh'}
          </button>
        )}
      </div>

      <div className="kanban-board__instructions" role="note">
        <p>
          <strong>Keyboard:</strong> Use arrow keys to navigate. Left/Right moves tasks between columns.
        </p>
        {isTouchDevice && (
          <p>
            <strong>Touch:</strong> Tap a task to select it, then swipe left/right to move between columns.
          </p>
        )}
      </div>

      {/* Swipe hint for mobile */}
      {isMobile && tasks.length > 0 && (
        <div className="kanban-board__swipe-hint" aria-hidden="true">
          <span className="kanban-board__swipe-indicator">👆</span>
          <span>Tap to select, swipe to move</span>
        </div>
      )}

      <div
        className={`kanban-board__columns ${swiping ? 'kanban-board__columns--swiping' : ''}`}
        {...swipeHandlers}
      >
        {COLUMNS.map((column) => (
          <KanbanColumn
            key={column.status}
            status={column.status}
            title={column.title}
            tasks={tasksByStatus[column.status]}
            draggingTaskId={draggingTaskId}
            selectedTaskId={selectedTaskId}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
            onDragOver={handleDragOver}
            onDrop={handleDrop}
            onTaskKeyDown={handleTaskKeyDown}
            onTaskTouch={handleTaskTouch}
          />
        ))}
      </div>

      {tasks.length === 0 && !loading && (
        <div className="kanban-board__empty">
          <p>No tasks found in the PRD file.</p>
        </div>
      )}
    </section>
  );
}
