import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Task } from '../types/session';
import { TaskCard } from './TaskCard';

const mockPendingTask: Task = {
  id: 'MC-1',
  title: 'MC-1 Add feature',
  status: 'pending',
  context_bundle: ['src/main.rs'],
  definition_of_done: 'Feature is implemented and tested.',
  checklist: ['Implement feature', 'Write tests'],
  dependencies: [],
};

const mockInProgressTask: Task = {
  id: 'MC-2',
  title: 'MC-2 Fix bug',
  status: 'in_progress',
  context_bundle: ['src/lib.rs', 'src/core.rs'],
  definition_of_done: 'Bug is fixed.',
  checklist: ['Debug issue', 'Apply fix', 'Add regression test'],
  dependencies: ['MC-1'],
};

const mockCompletedTask: Task = {
  id: 'MC-3',
  title: 'MC-3 Setup project',
  status: 'completed',
  context_bundle: [],
  checklist: [],
  dependencies: [],
};

describe('TaskCard', () => {
  it('renders task ID', () => {
    render(<TaskCard task={mockPendingTask} />);
    expect(screen.getByText('MC-1')).toBeInTheDocument();
  });

  it('renders task title', () => {
    render(<TaskCard task={mockPendingTask} />);
    expect(screen.getByText('MC-1 Add feature')).toBeInTheDocument();
  });

  it('renders pending status badge', () => {
    render(<TaskCard task={mockPendingTask} />);
    expect(screen.getByText('Pending')).toBeInTheDocument();
  });

  it('renders in progress status badge', () => {
    render(<TaskCard task={mockInProgressTask} />);
    expect(screen.getByText('In Progress')).toBeInTheDocument();
  });

  it('renders completed status badge', () => {
    render(<TaskCard task={mockCompletedTask} />);
    expect(screen.getByText('Completed')).toBeInTheDocument();
  });

  it('renders definition of done', () => {
    render(<TaskCard task={mockPendingTask} />);
    expect(screen.getByText('Feature is implemented and tested.')).toBeInTheDocument();
  });

  it('truncates long definition of done', () => {
    const longDoD =
      'A'.repeat(150);
    const taskWithLongDoD: Task = {
      ...mockPendingTask,
      definition_of_done: longDoD,
    };
    render(<TaskCard task={taskWithLongDoD} />);
    expect(screen.getByText(/^A{100}\.\.\.$/)).toBeInTheDocument();
  });

  it('renders dependencies', () => {
    render(<TaskCard task={mockInProgressTask} />);
    expect(screen.getByText('Depends on:')).toBeInTheDocument();
    expect(screen.getByText('MC-1')).toBeInTheDocument();
  });

  it('does not render dependencies section when empty', () => {
    render(<TaskCard task={mockPendingTask} />);
    expect(screen.queryByText('Depends on:')).not.toBeInTheDocument();
  });

  it('renders checklist count', () => {
    render(<TaskCard task={mockPendingTask} />);
    expect(screen.getByText('2 checklist items')).toBeInTheDocument();
  });

  it('renders singular checklist count', () => {
    const taskWithOneItem: Task = {
      ...mockPendingTask,
      checklist: ['Single item'],
    };
    render(<TaskCard task={taskWithOneItem} />);
    expect(screen.getByText('1 checklist item')).toBeInTheDocument();
  });

  it('does not render checklist summary when empty', () => {
    render(<TaskCard task={mockCompletedTask} />);
    expect(screen.queryByText(/checklist item/)).not.toBeInTheDocument();
  });

  it('is draggable', () => {
    render(<TaskCard task={mockPendingTask} />);
    const card = screen.getByRole('article');
    expect(card).toHaveAttribute('draggable', 'true');
  });

  it('calls onDragStart when dragging starts', () => {
    const onDragStart = vi.fn();
    render(<TaskCard task={mockPendingTask} onDragStart={onDragStart} />);
    const card = screen.getByRole('article');
    fireEvent.dragStart(card, {
      dataTransfer: { setData: vi.fn(), effectAllowed: '' },
    });
    expect(onDragStart).toHaveBeenCalledWith('MC-1', expect.any(Object));
  });

  it('calls onDragEnd when dragging ends', () => {
    const onDragEnd = vi.fn();
    render(<TaskCard task={mockPendingTask} onDragEnd={onDragEnd} />);
    const card = screen.getByRole('article');
    fireEvent.dragEnd(card);
    expect(onDragEnd).toHaveBeenCalled();
  });

  it('calls onKeyDown when key is pressed', () => {
    const onKeyDown = vi.fn();
    render(<TaskCard task={mockPendingTask} onKeyDown={onKeyDown} />);
    const card = screen.getByRole('article');
    fireEvent.keyDown(card, { key: 'ArrowRight' });
    expect(onKeyDown).toHaveBeenCalledWith('MC-1', expect.any(Object));
  });

  it('applies dragging class when isDragging is true', () => {
    render(<TaskCard task={mockPendingTask} isDragging={true} />);
    const card = screen.getByRole('article');
    expect(card).toHaveClass('task-card--dragging');
  });

  it('has accessible aria-label', () => {
    render(<TaskCard task={mockPendingTask} />);
    const card = screen.getByRole('article');
    expect(card).toHaveAttribute(
      'aria-label',
      'Task MC-1: MC-1 Add feature. Status: Pending'
    );
  });

  it('has data-task-id attribute', () => {
    render(<TaskCard task={mockPendingTask} />);
    const card = screen.getByRole('article');
    expect(card).toHaveAttribute('data-task-id', 'MC-1');
  });
});
