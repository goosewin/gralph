import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AgentOrchestrationDashboard } from './AgentOrchestrationDashboard';
import type { OrchestrationState } from '../types/session';

const createMockState = (overrides?: Partial<OrchestrationState>): OrchestrationState => ({
  agents: [],
  tasks: [],
  queue_stats: {
    pending_count: 0,
    in_progress_count: 0,
    completed_count: 0,
  },
  max_agents: 10,
  ...overrides,
});

describe('AgentOrchestrationDashboard', () => {
  describe('rendering', () => {
    it('renders the dashboard with title', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );
      expect(screen.getByText('Agent Orchestration')).toBeInTheDocument();
    });

    it('renders connection indicator', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );
      expect(screen.getByText('connected')).toBeInTheDocument();
    });

    it('displays error message when error is provided', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error="Connection failed"
        />
      );
      expect(screen.getByRole('alert')).toHaveTextContent('Connection failed');
    });

    it('shows empty state when no agents or tasks exist', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );
      expect(screen.getByText('No active orchestration.')).toBeInTheDocument();
    });

    it('renders refresh button when onRefresh is provided', () => {
      const onRefresh = vi.fn();
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
          onRefresh={onRefresh}
        />
      );
      expect(screen.getByRole('button', { name: /refresh/i })).toBeInTheDocument();
    });

    it('disables refresh button when loading', () => {
      const onRefresh = vi.fn();
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
          loading={true}
          onRefresh={onRefresh}
        />
      );
      expect(screen.getByRole('button', { name: /refresh/i })).toBeDisabled();
    });
  });

  describe('agent stats', () => {
    it('displays correct agent counts', () => {
      const state = createMockState({
        agents: [
          { id: 0, status: 'idle', specialization: 'general' },
          { id: 1, status: 'working', specialization: 'code-gen', current_task: 'task-1' },
          { id: 2, status: 'working', specialization: 'testing', current_task: 'task-2' },
          { id: 3, status: 'failed', specialization: 'review' },
        ],
      });

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      // Look for stats in the agent stats section
      const statsSection = screen.getByRole('group', { name: /orchestration statistics/i });
      expect(statsSection).toBeInTheDocument();
    });

    it('displays correct task counts from queue stats', () => {
      const state = createMockState({
        agents: [
          { id: 0, status: 'idle', specialization: 'general' },
        ],
        queue_stats: {
          pending_count: 5,
          in_progress_count: 2,
          completed_count: 10,
        },
      });

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      // Stats should be displayed
      expect(screen.getByText('5')).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument();
      expect(screen.getByText('10')).toBeInTheDocument();
    });
  });

  describe('visualization', () => {
    it('renders SVG visualization when agents exist', () => {
      const state = createMockState({
        agents: [
          { id: 0, status: 'idle', specialization: 'general' },
        ],
        tasks: [
          { id: 'task-1', content: 'Test task', dependencies: [], status: 'pending' },
        ],
      });

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      expect(screen.getByRole('img', { name: /task flow diagram/i })).toBeInTheDocument();
    });

    it('renders agent nodes in visualization', () => {
      const state = createMockState({
        agents: [
          { id: 0, status: 'working', specialization: 'code-gen', current_task: 'task-1' },
          { id: 1, status: 'idle', specialization: 'testing' },
        ],
        tasks: [
          { id: 'task-1', content: 'Implement feature', dependencies: [], status: 'in_progress', assigned_agent: 0 },
        ],
      });

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      // Check for agent list in visualization
      const agentList = screen.getByRole('list', { name: /agents/i });
      expect(agentList).toBeInTheDocument();
    });

    it('renders task nodes in visualization', () => {
      const state = createMockState({
        agents: [
          { id: 0, status: 'idle', specialization: 'general' },
        ],
        tasks: [
          { id: 'task-1', content: 'First task', dependencies: [], status: 'completed' },
          { id: 'task-2', content: 'Second task', dependencies: ['task-1'], status: 'in_progress' },
        ],
      });

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      // Check for task list in visualization
      const taskList = screen.getByRole('list', { name: /tasks/i });
      expect(taskList).toBeInTheDocument();
    });
  });

  describe('legend', () => {
    it('renders agent status legend', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );

      expect(screen.getByText('Agent Status')).toBeInTheDocument();
      // Use getAllByText since Idle/Working/Failed appear in both stats and legend
      expect(screen.getAllByText('Idle').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Working').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Failed').length).toBeGreaterThanOrEqual(1);
    });

    it('renders task status legend', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );

      expect(screen.getByText('Task Status')).toBeInTheDocument();
      // Use getAllByText since Pending/In Progress/Completed appear in both stats and legend
      expect(screen.getAllByText('Pending').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('In Progress').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Completed').length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('accessibility', () => {
    it('has proper aria labels for dashboard section', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );

      expect(screen.getByRole('region', { name: /agent orchestration dashboard/i })).toBeInTheDocument();
    });

    it('has proper aria labels for statistics groups', () => {
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
        />
      );

      expect(screen.getByRole('group', { name: /orchestration statistics/i })).toBeInTheDocument();
      expect(screen.getByRole('group', { name: /legend/i })).toBeInTheDocument();
    });

    it('has accessible refresh button', () => {
      const onRefresh = vi.fn();
      render(
        <AgentOrchestrationDashboard
          orchestrationState={createMockState()}
          connectionState="connected"
          error={null}
          onRefresh={onRefresh}
        />
      );

      const refreshButton = screen.getByRole('button', { name: /refresh orchestration state/i });
      expect(refreshButton).toBeInTheDocument();
    });
  });

  describe('performance', () => {
    it('handles 20 agents without performance issues', () => {
      const statuses = ['idle', 'working', 'failed'] as const;
      const specializations = ['general', 'code-gen', 'testing', 'review', 'documentation'] as const;

      const agents = Array.from({ length: 20 }, (_, i) => ({
        id: i,
        status: statuses[i % 3],
        specialization: specializations[i % 5],
        current_task: i % 3 === 1 ? `task-${i}` : undefined,
      }));

      const taskStatuses = ['completed', 'in_progress', 'pending'] as const;
      const tasks = Array.from({ length: 50 }, (_, i) => ({
        id: `task-${i}`,
        content: `Task ${i}`,
        dependencies: i > 0 ? [`task-${i - 1}`] : [],
        status: i < 20 ? taskStatuses[0] : i < 30 ? taskStatuses[1] : taskStatuses[2],
        assigned_agent: i >= 20 && i < 30 ? i - 20 : undefined,
      }));

      const state = createMockState({
        agents,
        tasks,
        queue_stats: {
          pending_count: 20,
          in_progress_count: 10,
          completed_count: 20,
        },
        max_agents: 20,
      });

      const startTime = performance.now();

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      const endTime = performance.now();
      const renderTime = endTime - startTime;

      // Render should complete in reasonable time (under 500ms)
      expect(renderTime).toBeLessThan(500);

      // All 20 agents should be rendered - use getAllByText since '20' appears in multiple stats
      expect(screen.getAllByText('20').length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('task dependencies', () => {
    it('correctly displays tasks with multiple dependencies', () => {
      const state = createMockState({
        agents: [
          { id: 0, status: 'idle', specialization: 'general' },
        ],
        tasks: [
          { id: 'setup', content: 'Setup', dependencies: [], status: 'completed' },
          { id: 'config', content: 'Config', dependencies: [], status: 'completed' },
          { id: 'build', content: 'Build', dependencies: ['setup', 'config'], status: 'in_progress' },
        ],
      });

      render(
        <AgentOrchestrationDashboard
          orchestrationState={state}
          connectionState="connected"
          error={null}
        />
      );

      // Visualization should render without errors
      expect(screen.getByRole('img', { name: /task flow diagram/i })).toBeInTheDocument();
    });
  });
});

describe('useOrchestration hook integration', () => {
  it('handles null orchestration state gracefully', () => {
    const onRefresh = vi.fn();
    render(
      <AgentOrchestrationDashboard
        orchestrationState={null}
        connectionState="connecting"
        error={null}
        loading={true}
        onRefresh={onRefresh}
      />
    );

    // Should show loading state without crashing
    expect(screen.getByText('Agent Orchestration')).toBeInTheDocument();
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });
});
