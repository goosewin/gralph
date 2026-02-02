import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Task } from '../types/session';
import { KanbanColumn } from './KanbanColumn';

const mockTasks: Task[] = [
  {
    id: 'MC-1',
    title: 'MC-1 First task',
    status: 'pending',
    checklist: [],
    dependencies: [],
  },
  {
    id: 'MC-2',
    title: 'MC-2 Second task',
    status: 'pending',
    checklist: [],
    dependencies: [],
  },
];

const defaultProps = {
  status: 'pending' as const,
  title: 'Pending',
  tasks: mockTasks,
  draggingTaskId: null,
  onDragStart: vi.fn(),
  onDragEnd: vi.fn(),
  onDragOver: vi.fn(),
  onDrop: vi.fn(),
  onTaskKeyDown: vi.fn(),
};

describe('KanbanColumn', () => {
  it('renders column title', () => {
    render(<KanbanColumn {...defaultProps} />);
    expect(screen.getByRole('heading', { name: 'Pending', level: 3 })).toBeInTheDocument();
  });

  it('renders task count', () => {
    render(<KanbanColumn {...defaultProps} />);
    expect(screen.getByLabelText('2 tasks')).toBeInTheDocument();
  });

  it('renders all tasks', () => {
    render(<KanbanColumn {...defaultProps} />);
    expect(screen.getByText('MC-1 First task')).toBeInTheDocument();
    expect(screen.getByText('MC-2 Second task')).toBeInTheDocument();
  });

  it('renders empty message when no tasks', () => {
    render(<KanbanColumn {...defaultProps} tasks={[]} />);
    expect(screen.getByText('No tasks')).toBeInTheDocument();
  });

  it('calls onDragOver when dragging over', () => {
    const onDragOver = vi.fn();
    render(<KanbanColumn {...defaultProps} onDragOver={onDragOver} />);
    const column = screen.getByRole('region');
    fireEvent.dragOver(column, {
      dataTransfer: { dropEffect: '' },
    });
    expect(onDragOver).toHaveBeenCalledWith('pending', expect.any(Object));
  });

  it('calls onDrop when dropping', () => {
    const onDrop = vi.fn();
    render(<KanbanColumn {...defaultProps} onDrop={onDrop} />);
    const column = screen.getByRole('region');
    fireEvent.drop(column);
    expect(onDrop).toHaveBeenCalledWith('pending', expect.any(Object));
  });

  it('has accessible region with labelledby', () => {
    render(<KanbanColumn {...defaultProps} />);
    const column = screen.getByRole('region');
    expect(column).toHaveAttribute('aria-labelledby', 'column-pending-title');
  });

  it('has list role for task container', () => {
    render(<KanbanColumn {...defaultProps} />);
    expect(screen.getByRole('list')).toBeInTheDocument();
  });

  it('passes draggingTaskId to TaskCards', () => {
    render(<KanbanColumn {...defaultProps} draggingTaskId="MC-1" />);
    const cards = screen.getAllByRole('article');
    expect(cards[0]).toHaveClass('task-card--dragging');
    expect(cards[1]).not.toHaveClass('task-card--dragging');
  });

  it('applies correct class for in_progress column', () => {
    render(
      <KanbanColumn
        {...defaultProps}
        status="in_progress"
        title="In Progress"
      />
    );
    const column = screen.getByRole('region');
    expect(column).toHaveClass('kanban-column--in-progress');
  });

  it('applies correct class for completed column', () => {
    render(
      <KanbanColumn
        {...defaultProps}
        status="completed"
        title="Completed"
      />
    );
    const column = screen.getByRole('region');
    expect(column).toHaveClass('kanban-column--completed');
  });
});
