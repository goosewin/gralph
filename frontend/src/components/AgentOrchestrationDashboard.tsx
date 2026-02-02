import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Agent, AgentSpecialization, AgentStatus, ConnectionState, OrchestrationState, TaskNode } from '../types/session';

export interface AgentOrchestrationDashboardProps {
  orchestrationState: OrchestrationState | null;
  connectionState: ConnectionState;
  error: string | null;
  loading?: boolean;
  onRefresh?: () => void;
}

interface NodePosition {
  x: number;
  y: number;
}

interface LayoutConfig {
  agentAreaWidth: number;
  taskAreaWidth: number;
  nodeWidth: number;
  nodeHeight: number;
  verticalGap: number;
  horizontalGap: number;
  padding: number;
}

const DEFAULT_LAYOUT: LayoutConfig = {
  agentAreaWidth: 200,
  taskAreaWidth: 600,
  nodeWidth: 140,
  nodeHeight: 60,
  verticalGap: 20,
  horizontalGap: 40,
  padding: 20,
};

/** Get color class for agent status */
function getAgentStatusColor(status: AgentStatus): string {
  switch (status) {
    case 'idle':
      return 'agent-node--idle';
    case 'working':
      return 'agent-node--working';
    case 'failed':
      return 'agent-node--failed';
    default:
      return '';
  }
}

/** Get icon for agent specialization */
function getSpecializationIcon(spec: AgentSpecialization): string {
  switch (spec) {
    case 'code-gen':
      return '💻';
    case 'testing':
      return '🧪';
    case 'review':
      return '👁️';
    case 'documentation':
      return '📝';
    case 'general':
    default:
      return '🤖';
  }
}

/** Get label for agent specialization */
function getSpecializationLabel(spec: AgentSpecialization): string {
  switch (spec) {
    case 'code-gen':
      return 'Code';
    case 'testing':
      return 'Test';
    case 'review':
      return 'Review';
    case 'documentation':
      return 'Docs';
    case 'general':
    default:
      return 'General';
  }
}

/** Get color class for task status */
function getTaskStatusColor(status: TaskNode['status']): string {
  switch (status) {
    case 'pending':
      return 'task-node--pending';
    case 'in_progress':
      return 'task-node--in-progress';
    case 'completed':
      return 'task-node--completed';
    case 'failed':
      return 'task-node--failed';
    default:
      return '';
  }
}

/** Calculate task node positions using topological sort with level assignment */
function calculateTaskPositions(
  tasks: TaskNode[],
  layout: LayoutConfig,
  startX: number
): Map<string, NodePosition> {
  const positions = new Map<string, NodePosition>();

  if (tasks.length === 0) return positions;

  // Build adjacency and in-degree maps
  const inDegree = new Map<string, number>();
  const dependents = new Map<string, string[]>();

  tasks.forEach((task) => {
    inDegree.set(task.id, 0);
    dependents.set(task.id, []);
  });

  tasks.forEach((task) => {
    task.dependencies.forEach((dep) => {
      if (inDegree.has(dep)) {
        const currentDegree = inDegree.get(task.id) ?? 0;
        inDegree.set(task.id, currentDegree + 1);
        const deps = dependents.get(dep) ?? [];
        deps.push(task.id);
        dependents.set(dep, deps);
      }
    });
  });

  // Assign levels using BFS (topological sort)
  const levels: string[][] = [];
  const taskLevels = new Map<string, number>();

  // Find all root nodes (no dependencies)
  let currentLevel: string[] = [];
  tasks.forEach((task) => {
    if ((inDegree.get(task.id) ?? 0) === 0) {
      currentLevel.push(task.id);
      taskLevels.set(task.id, 0);
    }
  });

  while (currentLevel.length > 0) {
    levels.push([...currentLevel]);
    const nextLevel: string[] = [];

    currentLevel.forEach((taskId) => {
      const deps = dependents.get(taskId) ?? [];
      deps.forEach((depId) => {
        const degree = (inDegree.get(depId) ?? 1) - 1;
        inDegree.set(depId, degree);
        if (degree === 0 && !taskLevels.has(depId)) {
          const level = (taskLevels.get(taskId) ?? 0) + 1;
          taskLevels.set(depId, level);
          nextLevel.push(depId);
        }
      });
    });

    currentLevel = nextLevel;
  }

  // Handle any remaining tasks (in case of cycles or disconnected components)
  tasks.forEach((task) => {
    if (!taskLevels.has(task.id)) {
      const maxLevel = levels.length;
      taskLevels.set(task.id, maxLevel);
      if (!levels[maxLevel]) {
        levels.push([]);
      }
      levels[maxLevel].push(task.id);
    }
  });

  // Assign positions based on levels
  levels.forEach((levelTasks, levelIndex) => {
    const x = startX + levelIndex * (layout.nodeWidth + layout.horizontalGap);

    levelTasks.forEach((taskId, taskIndex) => {
      const y = layout.padding + taskIndex * (layout.nodeHeight + layout.verticalGap);
      positions.set(taskId, { x, y });
    });
  });

  return positions;
}

/** Agent node component */
function AgentNode({ agent, position }: { agent: Agent; position: NodePosition }) {
  return (
    <g
      className={`agent-node ${getAgentStatusColor(agent.status)}`}
      transform={`translate(${position.x}, ${position.y})`}
      role="listitem"
      aria-label={`Agent ${agent.id}: ${agent.status}, ${agent.specialization}`}
    >
      <rect
        className="agent-node__bg"
        width={DEFAULT_LAYOUT.nodeWidth}
        height={DEFAULT_LAYOUT.nodeHeight}
        rx="8"
        ry="8"
      />
      <text className="agent-node__icon" x="12" y="28" fontSize="18">
        {getSpecializationIcon(agent.specialization)}
      </text>
      <text className="agent-node__id" x="38" y="22" fontSize="12">
        Agent {agent.id}
      </text>
      <text className="agent-node__spec" x="38" y="38" fontSize="10">
        {getSpecializationLabel(agent.specialization)}
      </text>
      <text className="agent-node__status" x="38" y="52" fontSize="9">
        {agent.status}
      </text>
      {/* Status indicator */}
      <circle
        className="agent-node__indicator"
        cx={DEFAULT_LAYOUT.nodeWidth - 12}
        cy="12"
        r="6"
      />
    </g>
  );
}

/** Task node component */
function TaskNodeComponent({
  task,
  position,
  isAnimating,
}: {
  task: TaskNode;
  position: NodePosition;
  isAnimating: boolean;
}) {
  const truncatedId = task.id.length > 12 ? `${task.id.slice(0, 12)}...` : task.id;

  return (
    <g
      className={`task-node ${getTaskStatusColor(task.status)} ${isAnimating ? 'task-node--animating' : ''}`}
      transform={`translate(${position.x}, ${position.y})`}
      role="listitem"
      aria-label={`Task ${task.id}: ${task.status}`}
    >
      <rect
        className="task-node__bg"
        width={DEFAULT_LAYOUT.nodeWidth}
        height={DEFAULT_LAYOUT.nodeHeight}
        rx="6"
        ry="6"
      />
      <text className="task-node__id" x="10" y="22" fontSize="11" fontWeight="600">
        {truncatedId}
      </text>
      <text className="task-node__status" x="10" y="40" fontSize="9">
        {task.status.replace('_', ' ')}
      </text>
      {task.assigned_agent !== undefined && (
        <text className="task-node__agent" x="10" y="52" fontSize="8">
          Agent {task.assigned_agent}
        </text>
      )}
      {/* Progress indicator for in-progress tasks */}
      {task.status === 'in_progress' && (
        <rect
          className="task-node__progress"
          x="0"
          y={DEFAULT_LAYOUT.nodeHeight - 4}
          width={DEFAULT_LAYOUT.nodeWidth}
          height="4"
          rx="0"
          ry="0"
        />
      )}
    </g>
  );
}

/** Dependency edge component */
function DependencyEdge({
  from,
  to,
  fromTask,
  toTask,
}: {
  from: NodePosition;
  to: NodePosition;
  fromTask: TaskNode;
  toTask: TaskNode;
}) {
  // Calculate edge points (from right side of 'from' node to left side of 'to' node)
  const startX = from.x + DEFAULT_LAYOUT.nodeWidth;
  const startY = from.y + DEFAULT_LAYOUT.nodeHeight / 2;
  const endX = to.x;
  const endY = to.y + DEFAULT_LAYOUT.nodeHeight / 2;

  // Create a curved path
  const controlOffset = Math.abs(endX - startX) / 2;
  const path = `M ${startX} ${startY} C ${startX + controlOffset} ${startY}, ${endX - controlOffset} ${endY}, ${endX} ${endY}`;

  // Determine edge status based on tasks
  let edgeClass = 'dependency-edge';
  if (fromTask.status === 'completed' && toTask.status === 'in_progress') {
    edgeClass += ' dependency-edge--active';
  } else if (fromTask.status === 'completed') {
    edgeClass += ' dependency-edge--completed';
  }

  return (
    <g className={edgeClass}>
      <path
        className="dependency-edge__line"
        d={path}
        fill="none"
        strokeWidth="2"
        markerEnd="url(#arrowhead)"
      />
    </g>
  );
}

/** Assignment edge component (agent to task) */
function AssignmentEdge({
  agentPos,
  taskPos,
  isActive,
}: {
  agentPos: NodePosition;
  taskPos: NodePosition;
  isActive: boolean;
}) {
  const startX = agentPos.x + DEFAULT_LAYOUT.nodeWidth;
  const startY = agentPos.y + DEFAULT_LAYOUT.nodeHeight / 2;
  const endX = taskPos.x;
  const endY = taskPos.y + DEFAULT_LAYOUT.nodeHeight / 2;

  const path = `M ${startX} ${startY} L ${endX} ${endY}`;

  return (
    <g className={`assignment-edge ${isActive ? 'assignment-edge--active' : ''}`}>
      <path
        className="assignment-edge__line"
        d={path}
        fill="none"
        strokeWidth="2"
        strokeDasharray={isActive ? '0' : '4,4'}
      />
    </g>
  );
}

export function AgentOrchestrationDashboard({
  orchestrationState,
  connectionState,
  error,
  loading = false,
  onRefresh,
}: AgentOrchestrationDashboardProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [animatingTasks, setAnimatingTasks] = useState<Set<string>>(new Set());
  const prevTaskStatuses = useRef<Map<string, TaskNode['status']>>(new Map());

  // Track task status changes for animations
  useEffect(() => {
    if (!orchestrationState) return;

    const newAnimating = new Set<string>();
    orchestrationState.tasks.forEach((task) => {
      const prevStatus = prevTaskStatuses.current.get(task.id);
      if (prevStatus && prevStatus !== task.status) {
        newAnimating.add(task.id);
      }
      prevTaskStatuses.current.set(task.id, task.status);
    });

    if (newAnimating.size > 0) {
      setAnimatingTasks(newAnimating);
      // Clear animation after duration
      const timer = setTimeout(() => {
        setAnimatingTasks(new Set());
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [orchestrationState]);

  // Calculate positions
  const { agentPositions, taskPositions, svgDimensions } = useMemo(() => {
    if (!orchestrationState) {
      return {
        agentPositions: new Map<number, NodePosition>(),
        taskPositions: new Map<string, NodePosition>(),
        svgDimensions: { width: 800, height: 400 },
      };
    }

    const layout = DEFAULT_LAYOUT;
    const agents = orchestrationState.agents;
    const tasks = orchestrationState.tasks;

    // Calculate agent positions (left column)
    const agentPos = new Map<number, NodePosition>();
    agents.forEach((agent, index) => {
      agentPos.set(agent.id, {
        x: layout.padding,
        y: layout.padding + index * (layout.nodeHeight + layout.verticalGap),
      });
    });

    // Calculate task positions (right side, in levels)
    const taskStartX = layout.padding + layout.agentAreaWidth + layout.horizontalGap;
    const taskPos = calculateTaskPositions(tasks, layout, taskStartX);

    // Calculate SVG dimensions
    const maxAgentY = agents.length > 0
      ? layout.padding + agents.length * (layout.nodeHeight + layout.verticalGap)
      : layout.padding;

    let maxTaskY = layout.padding;
    let maxTaskX = taskStartX;
    taskPos.forEach((pos) => {
      maxTaskY = Math.max(maxTaskY, pos.y + layout.nodeHeight + layout.verticalGap);
      maxTaskX = Math.max(maxTaskX, pos.x + layout.nodeWidth + layout.horizontalGap);
    });

    const width = Math.max(maxTaskX + layout.padding, 800);
    const height = Math.max(maxAgentY, maxTaskY) + layout.padding;

    return {
      agentPositions: agentPos,
      taskPositions: taskPos,
      svgDimensions: { width, height: Math.max(height, 300) },
    };
  }, [orchestrationState]);

  // Build task map for quick lookup
  const taskMap = useMemo(() => {
    const map = new Map<string, TaskNode>();
    orchestrationState?.tasks.forEach((task) => {
      map.set(task.id, task);
    });
    return map;
  }, [orchestrationState]);

  // Stats computation
  const stats = useMemo(() => {
    if (!orchestrationState) {
      return {
        totalAgents: 0,
        idleAgents: 0,
        workingAgents: 0,
        failedAgents: 0,
        totalTasks: 0,
        pendingTasks: 0,
        inProgressTasks: 0,
        completedTasks: 0,
      };
    }

    return {
      totalAgents: orchestrationState.agents.length,
      idleAgents: orchestrationState.agents.filter((a) => a.status === 'idle').length,
      workingAgents: orchestrationState.agents.filter((a) => a.status === 'working').length,
      failedAgents: orchestrationState.agents.filter((a) => a.status === 'failed').length,
      totalTasks: orchestrationState.tasks.length,
      pendingTasks: orchestrationState.queue_stats.pending_count,
      inProgressTasks: orchestrationState.queue_stats.in_progress_count,
      completedTasks: orchestrationState.queue_stats.completed_count,
    };
  }, [orchestrationState]);

  const handleRefresh = useCallback(() => {
    onRefresh?.();
  }, [onRefresh]);

  return (
    <section className="orchestration-dashboard" aria-label="Agent Orchestration Dashboard">
      <header className="orchestration-dashboard__header">
        <h2 className="orchestration-dashboard__title">Agent Orchestration</h2>
        <div className="orchestration-dashboard__connection">
          <span
            className={`connection-indicator connection-indicator--${connectionState}`}
            aria-label={`Connection: ${connectionState}`}
          />
          <span className="connection-label">{connectionState}</span>
        </div>
      </header>

      {error && (
        <div className="orchestration-dashboard__error" role="alert">
          <span className="orchestration-dashboard__error-message">{error}</span>
        </div>
      )}

      <div className="orchestration-dashboard__stats" role="group" aria-label="Orchestration statistics">
        <div className="stat-group">
          <h3 className="stat-group__title">Agents</h3>
          <div className="stat-group__items">
            <div className="stat-item">
              <span className="stat-value">{stats.totalAgents}</span>
              <span className="stat-label">Total</span>
            </div>
            <div className="stat-item stat-item--idle">
              <span className="stat-value">{stats.idleAgents}</span>
              <span className="stat-label">Idle</span>
            </div>
            <div className="stat-item stat-item--working">
              <span className="stat-value">{stats.workingAgents}</span>
              <span className="stat-label">Working</span>
            </div>
            <div className="stat-item stat-item--failed">
              <span className="stat-value">{stats.failedAgents}</span>
              <span className="stat-label">Failed</span>
            </div>
          </div>
        </div>
        <div className="stat-group">
          <h3 className="stat-group__title">Tasks</h3>
          <div className="stat-group__items">
            <div className="stat-item">
              <span className="stat-value">{stats.totalTasks}</span>
              <span className="stat-label">Total</span>
            </div>
            <div className="stat-item stat-item--pending">
              <span className="stat-value">{stats.pendingTasks}</span>
              <span className="stat-label">Pending</span>
            </div>
            <div className="stat-item stat-item--in-progress">
              <span className="stat-value">{stats.inProgressTasks}</span>
              <span className="stat-label">In Progress</span>
            </div>
            <div className="stat-item stat-item--completed">
              <span className="stat-value">{stats.completedTasks}</span>
              <span className="stat-label">Completed</span>
            </div>
          </div>
        </div>
      </div>

      <div className="orchestration-dashboard__actions">
        {onRefresh && (
          <button
            className="orchestration-dashboard__refresh"
            onClick={handleRefresh}
            disabled={loading}
            aria-label="Refresh orchestration state"
          >
            {loading ? 'Loading...' : 'Refresh'}
          </button>
        )}
      </div>

      <div className="orchestration-dashboard__visualization" role="img" aria-label="Task flow diagram showing agents and task dependencies">
        {orchestrationState && orchestrationState.agents.length === 0 && orchestrationState.tasks.length === 0 ? (
          <div className="orchestration-dashboard__empty">
            <p>No active orchestration.</p>
            <p className="orchestration-dashboard__empty-hint">
              Start a multi-agent session to see orchestration visualization.
            </p>
          </div>
        ) : (
          <svg
            ref={svgRef}
            className="orchestration-svg"
            width={svgDimensions.width}
            height={svgDimensions.height}
            viewBox={`0 0 ${svgDimensions.width} ${svgDimensions.height}`}
          >
            {/* Defs for markers */}
            <defs>
              <marker
                id="arrowhead"
                markerWidth="10"
                markerHeight="7"
                refX="9"
                refY="3.5"
                orient="auto"
              >
                <polygon
                  points="0 0, 10 3.5, 0 7"
                  className="arrowhead-fill"
                />
              </marker>
            </defs>

            {/* Dependency edges (draw first, behind nodes) */}
            <g className="dependency-edges" role="presentation">
              {orchestrationState?.tasks.map((task) =>
                task.dependencies.map((depId) => {
                  const fromPos = taskPositions.get(depId);
                  const toPos = taskPositions.get(task.id);
                  const fromTask = taskMap.get(depId);
                  const toTask = taskMap.get(task.id);

                  if (fromPos && toPos && fromTask && toTask) {
                    return (
                      <DependencyEdge
                        key={`${depId}->${task.id}`}
                        from={fromPos}
                        to={toPos}
                        fromTask={fromTask}
                        toTask={toTask}
                      />
                    );
                  }
                  return null;
                })
              )}
            </g>

            {/* Assignment edges (agent to task) */}
            <g className="assignment-edges" role="presentation">
              {orchestrationState?.agents
                .filter((agent) => agent.current_task)
                .map((agent) => {
                  const agentPos = agentPositions.get(agent.id);
                  const taskPos = taskPositions.get(agent.current_task!);
                  if (agentPos && taskPos) {
                    return (
                      <AssignmentEdge
                        key={`agent-${agent.id}->${agent.current_task}`}
                        agentPos={agentPos}
                        taskPos={taskPos}
                        isActive={agent.status === 'working'}
                      />
                    );
                  }
                  return null;
                })}
            </g>

            {/* Agent nodes */}
            <g className="agent-nodes" role="list" aria-label="Agents">
              {orchestrationState?.agents.map((agent) => {
                const pos = agentPositions.get(agent.id);
                if (!pos) return null;
                return <AgentNode key={agent.id} agent={agent} position={pos} />;
              })}
            </g>

            {/* Task nodes */}
            <g className="task-nodes" role="list" aria-label="Tasks">
              {orchestrationState?.tasks.map((task) => {
                const pos = taskPositions.get(task.id);
                if (!pos) return null;
                return (
                  <TaskNodeComponent
                    key={task.id}
                    task={task}
                    position={pos}
                    isAnimating={animatingTasks.has(task.id)}
                  />
                );
              })}
            </g>
          </svg>
        )}
      </div>

      {/* Legend */}
      <div className="orchestration-dashboard__legend" role="group" aria-label="Legend">
        <div className="legend-section">
          <h4 className="legend-section__title">Agent Status</h4>
          <div className="legend-items">
            <div className="legend-item">
              <span className="legend-color legend-color--idle" />
              <span className="legend-label">Idle</span>
            </div>
            <div className="legend-item">
              <span className="legend-color legend-color--working" />
              <span className="legend-label">Working</span>
            </div>
            <div className="legend-item">
              <span className="legend-color legend-color--failed" />
              <span className="legend-label">Failed</span>
            </div>
          </div>
        </div>
        <div className="legend-section">
          <h4 className="legend-section__title">Task Status</h4>
          <div className="legend-items">
            <div className="legend-item">
              <span className="legend-color legend-color--pending" />
              <span className="legend-label">Pending</span>
            </div>
            <div className="legend-item">
              <span className="legend-color legend-color--in-progress" />
              <span className="legend-label">In Progress</span>
            </div>
            <div className="legend-item">
              <span className="legend-color legend-color--completed" />
              <span className="legend-label">Completed</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
