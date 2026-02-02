import type { Task, TaskStatus } from '../types/session';

export interface TaskCardProps {
  task: Task;
  isDragging?: boolean;
  isSelected?: boolean;
  onDragStart?: (taskId: string, event: React.DragEvent) => void;
  onDragEnd?: (event: React.DragEvent) => void;
  onKeyDown?: (taskId: string, event: React.KeyboardEvent) => void;
  onTouch?: (taskId: string) => void;
  tabIndex?: number;
}

const STATUS_CONFIG: Record<TaskStatus, { label: string; className: string }> = {
  pending: { label: 'Pending', className: 'task-card--pending' },
  in_progress: { label: 'In Progress', className: 'task-card--in-progress' },
  completed: { label: 'Completed', className: 'task-card--completed' },
};

export function TaskCard({
  task,
  isDragging = false,
  isSelected = false,
  onDragStart,
  onDragEnd,
  onKeyDown,
  onTouch,
  tabIndex = 0,
}: TaskCardProps) {
  const statusConfig = STATUS_CONFIG[task.status] || STATUS_CONFIG.pending;

  const handleDragStart = (event: React.DragEvent) => {
    event.dataTransfer.setData('text/plain', task.id);
    event.dataTransfer.effectAllowed = 'move';
    onDragStart?.(task.id, event);
  };

  const handleDragEnd = (event: React.DragEvent) => {
    onDragEnd?.(event);
  };

  const handleKeyDown = (event: React.KeyboardEvent) => {
    onKeyDown?.(task.id, event);
  };

  const handleClick = () => {
    onTouch?.(task.id);
  };

  const classNames = [
    'task-card',
    statusConfig.className,
    isDragging ? 'task-card--dragging' : '',
    isSelected ? 'task-card--selected' : '',
  ].filter(Boolean).join(' ');

  return (
    <article
      className={classNames}
      draggable
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onKeyDown={handleKeyDown}
      onClick={handleClick}
      tabIndex={tabIndex}
      role="article"
      aria-label={`Task ${task.id}: ${task.title}. Status: ${statusConfig.label}${isSelected ? '. Selected for swipe movement' : ''}`}
      aria-grabbed={isDragging}
      aria-selected={isSelected}
      data-task-id={task.id}
    >
      <header className="task-card__header">
        <span className="task-card__id">{task.id}</span>
        <span
          className={`task-card__status-badge ${statusConfig.className}`}
          role="status"
          aria-label={`Status: ${statusConfig.label}`}
        >
          {statusConfig.label}
        </span>
      </header>

      <h4 className="task-card__title">{task.title}</h4>

      {task.definition_of_done && (
        <p className="task-card__dod" title={task.definition_of_done}>
          {task.definition_of_done.length > 100
            ? `${task.definition_of_done.substring(0, 100)}...`
            : task.definition_of_done}
        </p>
      )}

      {task.dependencies && task.dependencies.length > 0 && (
        <div className="task-card__deps">
          <span className="task-card__deps-label">Depends on:</span>
          <span className="task-card__deps-list">
            {task.dependencies.join(', ')}
          </span>
        </div>
      )}

      {task.checklist && task.checklist.length > 0 && (
        <div className="task-card__checklist-summary">
          <span className="task-card__checklist-count">
            {task.checklist.length} checklist item{task.checklist.length !== 1 ? 's' : ''}
          </span>
        </div>
      )}
    </article>
  );
}
