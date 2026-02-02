import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Task } from '../types/session';
import { KanbanBoard } from './KanbanBoard';

const mockTasks: Task[] = [
  {
    id: 'MC-1',
    title: 'MC-1 Pending task',
    status: 'pending',
    checklist: [],
    dependencies: [],
  },
  {
    id: 'MC-2',
    title: 'MC-2 In progress task',
    status: 'in_progress',
    checklist: [],
    dependencies: ['MC-1'],
  },
  {
    id: 'MC-3',
    title: 'MC-3 Completed task',
    status: 'completed',
    checklist: [],
    dependencies: [],
  },
];

describe('KanbanBoard', () => {
  it('renders board title', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    expect(screen.getByText('PRD Tasks')).toBeInTheDocument();
  });

  it('renders all three columns', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    expect(screen.getByRole('heading', { name: 'Pending', level: 3 })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'In Progress', level: 3 })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Completed', level: 3 })).toBeInTheDocument();
  });

  it('groups tasks by status', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    expect(screen.getByText('MC-1 Pending task')).toBeInTheDocument();
    expect(screen.getByText('MC-2 In progress task')).toBeInTheDocument();
    expect(screen.getByText('MC-3 Completed task')).toBeInTheDocument();
  });

  it('renders connection state', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    expect(screen.getByLabelText('Connection: connected')).toBeInTheDocument();
  });

  it('renders error message', () => {
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="error"
        error="Failed to connect"
      />
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to connect');
  });

  it('renders refresh button when onRefresh is provided', () => {
    const onRefresh = vi.fn();
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onRefresh={onRefresh}
      />
    );
    expect(screen.getByRole('button', { name: /refresh/i })).toBeInTheDocument();
  });

  it('calls onRefresh when refresh button is clicked', () => {
    const onRefresh = vi.fn();
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onRefresh={onRefresh}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /refresh/i }));
    expect(onRefresh).toHaveBeenCalled();
  });

  it('disables refresh button when loading', () => {
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        loading={true}
        onRefresh={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /refresh/i })).toBeDisabled();
  });

  it('renders empty state when no tasks', () => {
    render(<KanbanBoard tasks={[]} connectionState="connected" error={null} />);
    expect(screen.getByText('No tasks found in the PRD file.')).toBeInTheDocument();
  });

  it('renders keyboard instructions', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    expect(screen.getByText(/use arrow keys/i)).toBeInTheDocument();
  });

  it('has screen reader announcement region', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    // There are multiple status roles (task badges and announcement region)
    const statusElements = screen.getAllByRole('status');
    // At least one should be for announcements (has aria-live)
    expect(statusElements.some(el => el.getAttribute('aria-live') === 'polite')).toBe(true);
  });

  it('calls onUpdateTaskStatus when task is dropped on different column', async () => {
    const onUpdateTaskStatus = vi.fn().mockResolvedValue(undefined);
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onUpdateTaskStatus={onUpdateTaskStatus}
      />
    );

    // Find the pending task
    const pendingTask = screen.getByText('MC-1 Pending task').closest('[data-task-id]');
    expect(pendingTask).toBeInTheDocument();

    // Start drag
    fireEvent.dragStart(pendingTask!, {
      dataTransfer: {
        setData: vi.fn(),
        getData: () => 'MC-1',
        effectAllowed: '',
      },
    });

    // Find the in_progress column
    const inProgressColumn = screen.getByLabelText(/in progress tasks/i);

    // Drop on in_progress column
    fireEvent.drop(inProgressColumn, {
      dataTransfer: {
        getData: () => 'MC-1',
      },
    });

    await waitFor(() => {
      expect(onUpdateTaskStatus).toHaveBeenCalledWith('MC-1', 'in_progress');
    });
  });

  it('does not call onUpdateTaskStatus when dropped on same column', async () => {
    const onUpdateTaskStatus = vi.fn().mockResolvedValue(undefined);
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onUpdateTaskStatus={onUpdateTaskStatus}
      />
    );

    const pendingColumn = screen.getByLabelText(/pending tasks/i);

    fireEvent.drop(pendingColumn, {
      dataTransfer: {
        getData: () => 'MC-1',
      },
    });

    // Should not call because task is already pending
    expect(onUpdateTaskStatus).not.toHaveBeenCalled();
  });

  it('moves task with keyboard ArrowRight', async () => {
    const onUpdateTaskStatus = vi.fn().mockResolvedValue(undefined);
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onUpdateTaskStatus={onUpdateTaskStatus}
      />
    );

    const pendingTask = screen.getByText('MC-1 Pending task').closest('[data-task-id]');
    fireEvent.keyDown(pendingTask!, { key: 'ArrowRight' });

    await waitFor(() => {
      expect(onUpdateTaskStatus).toHaveBeenCalledWith('MC-1', 'in_progress');
    });
  });

  it('moves task with keyboard ArrowLeft', async () => {
    const onUpdateTaskStatus = vi.fn().mockResolvedValue(undefined);
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onUpdateTaskStatus={onUpdateTaskStatus}
      />
    );

    const inProgressTask = screen.getByText('MC-2 In progress task').closest('[data-task-id]');
    fireEvent.keyDown(inProgressTask!, { key: 'ArrowLeft' });

    await waitFor(() => {
      expect(onUpdateTaskStatus).toHaveBeenCalledWith('MC-2', 'pending');
    });
  });

  it('does not move task past first column', async () => {
    const onUpdateTaskStatus = vi.fn().mockResolvedValue(undefined);
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onUpdateTaskStatus={onUpdateTaskStatus}
      />
    );

    const pendingTask = screen.getByText('MC-1 Pending task').closest('[data-task-id]');
    fireEvent.keyDown(pendingTask!, { key: 'ArrowLeft' });

    // Should not call because already in first column
    expect(onUpdateTaskStatus).not.toHaveBeenCalled();
  });

  it('does not move task past last column', async () => {
    const onUpdateTaskStatus = vi.fn().mockResolvedValue(undefined);
    render(
      <KanbanBoard
        tasks={mockTasks}
        connectionState="connected"
        error={null}
        onUpdateTaskStatus={onUpdateTaskStatus}
      />
    );

    const completedTask = screen.getByText('MC-3 Completed task').closest('[data-task-id]');
    fireEvent.keyDown(completedTask!, { key: 'ArrowRight' });

    // Should not call because already in last column
    expect(onUpdateTaskStatus).not.toHaveBeenCalled();
  });

  it('has accessible section with aria-label', () => {
    render(<KanbanBoard tasks={mockTasks} connectionState="connected" error={null} />);
    expect(screen.getByLabelText('Kanban Board')).toBeInTheDocument();
  });
});
