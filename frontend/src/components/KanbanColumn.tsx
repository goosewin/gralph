import type { Task, TaskStatus } from '../types/session';
import { TaskCard } from './TaskCard';

export interface KanbanColumnProps {
  status: TaskStatus;
  title: string;
  tasks: Task[];
  draggingTaskId: string | null;
  selectedTaskId?: string | null;
  onDragStart: (taskId: string, event: React.DragEvent) => void;
  onDragEnd: (event: React.DragEvent) => void;
  onDragOver: (status: TaskStatus, event: React.DragEvent) => void;
  onDrop: (status: TaskStatus, event: React.DragEvent) => void;
  onTaskKeyDown: (taskId: string, event: React.KeyboardEvent) => void;
  onTaskTouch?: (taskId: string) => void;
}

const COLUMN_CONFIG: Record<TaskStatus, { className: string }> = {
  pending: { className: 'kanban-column--pending' },
  in_progress: { className: 'kanban-column--in-progress' },
  completed: { className: 'kanban-column--completed' },
};

export function KanbanColumn({
  status,
  title,
  tasks,
  draggingTaskId,
  selectedTaskId,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDrop,
  onTaskKeyDown,
  onTaskTouch,
}: KanbanColumnProps) {
  const config = COLUMN_CONFIG[status] || COLUMN_CONFIG.pending;

  const handleDragOver = (event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    onDragOver(status, event);
  };

  const handleDrop = (event: React.DragEvent) => {
    event.preventDefault();
    onDrop(status, event);
  };

  return (
    <section
      className={`kanban-column ${config.className}`}
      aria-labelledby={`column-${status}-title`}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      <header className="kanban-column__header">
        <h3 id={`column-${status}-title`} className="kanban-column__title">
          {title}
        </h3>
        <span className="kanban-column__count" aria-label={`${tasks.length} tasks`}>
          {tasks.length}
        </span>
      </header>

      <div
        className="kanban-column__content"
        role="list"
        aria-label={`${title} tasks`}
      >
        {tasks.length === 0 ? (
          <div className="kanban-column__empty" role="listitem">
            <p>No tasks</p>
          </div>
        ) : (
          tasks.map((task, index) => (
            <div key={task.id} role="listitem">
              <TaskCard
                task={task}
                isDragging={draggingTaskId === task.id}
                isSelected={selectedTaskId === task.id}
                onDragStart={onDragStart}
                onDragEnd={onDragEnd}
                onKeyDown={onTaskKeyDown}
                onTouch={onTaskTouch}
                tabIndex={index === 0 ? 0 : -1}
              />
            </div>
          ))
        )}
      </div>
    </section>
  );
}
