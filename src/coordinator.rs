use crate::task::{is_unchecked_line, task_blocks_from_contents};
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Working,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: AgentId,
    pub status: AgentStatus,
    pub worktree_path: Option<PathBuf>,
    pub current_task: Option<String>,
}

impl Agent {
    pub fn new(id: AgentId) -> Self {
        Self {
            id,
            status: AgentStatus::Idle,
            worktree_path: None,
            current_task: None,
        }
    }

    pub fn with_worktree(mut self, path: PathBuf) -> Self {
        self.worktree_path = Some(path);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNode {
    pub id: String,
    pub content: String,
    pub dependencies: Vec<String>,
}

impl TaskNode {
    pub fn new(id: String, content: String) -> Self {
        Self {
            id,
            content,
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

#[derive(Debug)]
pub enum CoordinatorError {
    NoAvailableAgents,
    TaskNotFound(String),
    DependencyCycle(Vec<String>),
    WorktreeError(String),
    AgentFailed { agent_id: AgentId, reason: String },
}

impl fmt::Display for CoordinatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoordinatorError::NoAvailableAgents => write!(f, "no available agents in pool"),
            CoordinatorError::TaskNotFound(id) => write!(f, "task not found: {}", id),
            CoordinatorError::DependencyCycle(cycle) => {
                write!(f, "dependency cycle detected: {}", cycle.join(" -> "))
            }
            CoordinatorError::WorktreeError(msg) => write!(f, "worktree error: {}", msg),
            CoordinatorError::AgentFailed { agent_id, reason } => {
                write!(f, "agent {} failed: {}", agent_id.0, reason)
            }
        }
    }
}

impl Error for CoordinatorError {}

#[derive(Debug, Clone)]
pub struct WorkQueue {
    pending: VecDeque<TaskNode>,
    in_progress: HashMap<String, AgentId>,
    completed: HashSet<String>,
}

impl WorkQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            in_progress: HashMap::new(),
            completed: HashSet::new(),
        }
    }

    pub fn add_task(&mut self, task: TaskNode) {
        if !self.completed.contains(&task.id) && !self.in_progress.contains_key(&task.id) {
            let already_pending = self.pending.iter().any(|t| t.id == task.id);
            if !already_pending {
                self.pending.push_back(task);
            }
        }
    }

    pub fn get_ready_task(&mut self) -> Option<TaskNode> {
        let ready_index = self.pending.iter().position(|task| {
            task.dependencies
                .iter()
                .all(|dep| self.completed.contains(dep))
        });

        ready_index.and_then(|index| self.pending.remove(index))
    }

    pub fn mark_in_progress(&mut self, task_id: &str, agent_id: AgentId) {
        self.in_progress.insert(task_id.to_string(), agent_id);
    }

    pub fn mark_completed(&mut self, task_id: &str) {
        self.in_progress.remove(task_id);
        self.completed.insert(task_id.to_string());
    }

    pub fn mark_failed(&mut self, task_id: &str) {
        self.in_progress.remove(task_id);
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn in_progress_count(&self) -> usize {
        self.in_progress.len()
    }

    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.in_progress.is_empty()
    }

    pub fn all_complete(&self) -> bool {
        self.pending.is_empty() && self.in_progress.is_empty()
    }
}

impl Default for WorkQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Coordinator {
    agents: Vec<Agent>,
    work_queue: Arc<Mutex<WorkQueue>>,
    max_agents: usize,
    next_agent_id: AtomicUsize,
    shutdown: AtomicBool,
}

impl Coordinator {
    pub fn new(max_agents: usize) -> Self {
        Self {
            agents: Vec::new(),
            work_queue: Arc::new(Mutex::new(WorkQueue::new())),
            max_agents,
            next_agent_id: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
        }
    }

    pub fn spawn_agent(&mut self) -> Option<AgentId> {
        if self.agents.len() >= self.max_agents {
            return None;
        }

        let id = AgentId(self.next_agent_id.fetch_add(1, Ordering::SeqCst));
        let agent = Agent::new(id);
        self.agents.push(agent);
        Some(id)
    }

    pub fn spawn_agent_with_worktree(&mut self, worktree_path: PathBuf) -> Option<AgentId> {
        if self.agents.len() >= self.max_agents {
            return None;
        }

        let id = AgentId(self.next_agent_id.fetch_add(1, Ordering::SeqCst));
        let agent = Agent::new(id).with_worktree(worktree_path);
        self.agents.push(agent);
        Some(id)
    }

    pub fn get_agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn get_agent_mut(&mut self, id: AgentId) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    pub fn get_idle_agent(&self) -> Option<&Agent> {
        self.agents
            .iter()
            .find(|a| a.status == AgentStatus::Idle)
    }

    pub fn get_idle_agent_id(&self) -> Option<AgentId> {
        self.get_idle_agent().map(|a| a.id)
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn idle_agent_count(&self) -> usize {
        self.agents
            .iter()
            .filter(|a| a.status == AgentStatus::Idle)
            .count()
    }

    pub fn working_agent_count(&self) -> usize {
        self.agents
            .iter()
            .filter(|a| a.status == AgentStatus::Working)
            .count()
    }

    pub fn add_task(&self, task: TaskNode) {
        let mut queue = self.work_queue.lock().unwrap();
        queue.add_task(task);
    }

    pub fn add_tasks(&self, tasks: Vec<TaskNode>) {
        let mut queue = self.work_queue.lock().unwrap();
        for task in tasks {
            queue.add_task(task);
        }
    }

    pub fn assign_next_task(&mut self) -> Option<(AgentId, TaskNode)> {
        let agent_id = self.get_idle_agent_id()?;

        let task = {
            let mut queue = self.work_queue.lock().unwrap();
            let task = queue.get_ready_task()?;
            queue.mark_in_progress(&task.id, agent_id);
            task
        };

        if let Some(agent) = self.get_agent_mut(agent_id) {
            agent.status = AgentStatus::Working;
            agent.current_task = Some(task.id.clone());
        }

        Some((agent_id, task))
    }

    pub fn complete_task(&mut self, agent_id: AgentId, task_id: &str) {
        {
            let mut queue = self.work_queue.lock().unwrap();
            queue.mark_completed(task_id);
        }

        if let Some(agent) = self.get_agent_mut(agent_id) {
            agent.status = AgentStatus::Idle;
            agent.current_task = None;
        }
    }

    pub fn fail_task(&mut self, agent_id: AgentId, task_id: &str) {
        {
            let mut queue = self.work_queue.lock().unwrap();
            queue.mark_failed(task_id);
        }

        if let Some(agent) = self.get_agent_mut(agent_id) {
            agent.status = AgentStatus::Failed;
            agent.current_task = None;
        }
    }

    pub fn pending_task_count(&self) -> usize {
        self.work_queue.lock().unwrap().pending_count()
    }

    pub fn in_progress_task_count(&self) -> usize {
        self.work_queue.lock().unwrap().in_progress_count()
    }

    pub fn completed_task_count(&self) -> usize {
        self.work_queue.lock().unwrap().completed_count()
    }

    pub fn all_tasks_complete(&self) -> bool {
        self.work_queue.lock().unwrap().all_complete()
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

pub fn parse_task_dependencies(task_content: &str) -> Vec<String> {
    for line in task_content.lines() {
        let trimmed = line.trim();
        if let Some(deps_str) = trimmed.strip_prefix("- **Dependencies**") {
            let deps_part = deps_str.trim().trim_start_matches(':').trim();
            if deps_part.eq_ignore_ascii_case("none") || deps_part.is_empty() {
                return Vec::new();
            }
            return deps_part
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("none"))
                .collect();
        }
    }
    Vec::new()
}

pub fn parse_task_id(task_content: &str) -> Option<String> {
    for line in task_content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("### Task ") {
            let id = rest.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
        if let Some(rest) = trimmed.strip_prefix("- **ID**") {
            let id = rest.trim().trim_start_matches(':').trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

pub fn tasks_from_prd(prd_content: &str) -> Vec<TaskNode> {
    let blocks = task_blocks_from_contents(prd_content);
    let mut tasks = Vec::new();

    for block in blocks {
        let has_unchecked = block.lines().any(is_unchecked_line);
        if !has_unchecked {
            continue;
        }

        if let Some(id) = parse_task_id(&block) {
            let deps = parse_task_dependencies(&block);
            let task = TaskNode::new(id, block).with_dependencies(deps);
            tasks.push(task);
        }
    }

    tasks
}

pub fn build_dependency_graph(tasks: &[TaskNode]) -> HashMap<String, Vec<String>> {
    let mut graph = HashMap::new();
    for task in tasks {
        graph.insert(task.id.clone(), task.dependencies.clone());
    }
    graph
}

pub fn topological_sort(tasks: &[TaskNode]) -> Result<Vec<String>, CoordinatorError> {
    let task_ids: HashSet<_> = tasks.iter().map(|t| t.id.clone()).collect();

    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for id in &task_ids {
        in_degree.insert(id.clone(), 0);
    }

    for task in tasks {
        for dep in &task.dependencies {
            if task_ids.contains(dep) {
                *in_degree.get_mut(&task.id).unwrap() += 1;
            }
        }
    }

    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|&(_, degree)| *degree == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut sorted = Vec::new();

    while let Some(id) = queue.pop_front() {
        sorted.push(id.clone());

        for task in tasks {
            if task.dependencies.contains(&id) {
                if let Some(degree) = in_degree.get_mut(&task.id) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        queue.push_back(task.id.clone());
                    }
                }
            }
        }
    }

    if sorted.len() != task_ids.len() {
        let remaining: Vec<String> = task_ids
            .iter()
            .filter(|id| !sorted.contains(id))
            .cloned()
            .collect();
        return Err(CoordinatorError::DependencyCycle(remaining));
    }

    Ok(sorted)
}

pub fn find_independent_tasks<'a>(tasks: &'a [TaskNode], completed: &HashSet<String>) -> Vec<&'a TaskNode> {
    tasks
        .iter()
        .filter(|task| {
            !completed.contains(&task.id)
                && task.dependencies.iter().all(|dep| completed.contains(dep))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinator_spawns_agents_up_to_max() {
        let mut coordinator = Coordinator::new(3);

        assert!(coordinator.spawn_agent().is_some());
        assert!(coordinator.spawn_agent().is_some());
        assert!(coordinator.spawn_agent().is_some());
        assert!(coordinator.spawn_agent().is_none());

        assert_eq!(coordinator.agent_count(), 3);
    }

    #[test]
    fn coordinator_spawns_agent_with_worktree() {
        let mut coordinator = Coordinator::new(2);
        let path = PathBuf::from("/tmp/worktree-1");

        let id = coordinator.spawn_agent_with_worktree(path.clone()).unwrap();
        let agent = coordinator.get_agent(id).unwrap();

        assert_eq!(agent.worktree_path, Some(path));
        assert_eq!(agent.status, AgentStatus::Idle);
    }

    #[test]
    fn coordinator_tracks_agent_status() {
        let mut coordinator = Coordinator::new(5);

        coordinator.spawn_agent();
        coordinator.spawn_agent();

        assert_eq!(coordinator.idle_agent_count(), 2);
        assert_eq!(coordinator.working_agent_count(), 0);
    }

    #[test]
    fn work_queue_adds_and_retrieves_tasks() {
        let mut queue = WorkQueue::new();

        let task1 = TaskNode::new("T-1".to_string(), "content1".to_string());
        let task2 = TaskNode::new("T-2".to_string(), "content2".to_string());

        queue.add_task(task1);
        queue.add_task(task2);

        assert_eq!(queue.pending_count(), 2);

        let retrieved = queue.get_ready_task().unwrap();
        assert_eq!(retrieved.id, "T-1");
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn work_queue_respects_dependencies() {
        let mut queue = WorkQueue::new();

        let task1 = TaskNode::new("T-1".to_string(), "content1".to_string());
        let task2 =
            TaskNode::new("T-2".to_string(), "content2".to_string()).with_dependencies(vec!["T-1".to_string()]);

        queue.add_task(task2.clone());
        queue.add_task(task1);

        let first = queue.get_ready_task().unwrap();
        assert_eq!(first.id, "T-1");

        let second = queue.get_ready_task();
        assert!(second.is_none());

        queue.mark_completed("T-1");

        let third = queue.get_ready_task().unwrap();
        assert_eq!(third.id, "T-2");
    }

    #[test]
    fn work_queue_prevents_duplicate_tasks() {
        let mut queue = WorkQueue::new();

        let task1 = TaskNode::new("T-1".to_string(), "content1".to_string());
        let task1_dup = TaskNode::new("T-1".to_string(), "content1".to_string());

        queue.add_task(task1);
        queue.add_task(task1_dup);

        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn work_queue_tracks_completion_status() {
        let mut queue = WorkQueue::new();
        let agent_id = AgentId(0);

        let task = TaskNode::new("T-1".to_string(), "content".to_string());
        queue.add_task(task);

        let retrieved = queue.get_ready_task().unwrap();
        queue.mark_in_progress(&retrieved.id, agent_id);

        assert_eq!(queue.in_progress_count(), 1);
        assert_eq!(queue.pending_count(), 0);

        queue.mark_completed("T-1");

        assert_eq!(queue.completed_count(), 1);
        assert_eq!(queue.in_progress_count(), 0);
        assert!(queue.all_complete());
    }

    #[test]
    fn coordinator_assigns_tasks_to_idle_agents() {
        let mut coordinator = Coordinator::new(2);
        coordinator.spawn_agent();

        let task = TaskNode::new("T-1".to_string(), "content".to_string());
        coordinator.add_task(task);

        let (agent_id, assigned_task) = coordinator.assign_next_task().unwrap();

        assert_eq!(assigned_task.id, "T-1");

        let agent = coordinator.get_agent(agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Working);
        assert_eq!(agent.current_task, Some("T-1".to_string()));
    }

    #[test]
    fn coordinator_completes_tasks() {
        let mut coordinator = Coordinator::new(2);
        let agent_id = coordinator.spawn_agent().unwrap();

        let task = TaskNode::new("T-1".to_string(), "content".to_string());
        coordinator.add_task(task);

        coordinator.assign_next_task();
        coordinator.complete_task(agent_id, "T-1");

        assert_eq!(coordinator.completed_task_count(), 1);
        assert_eq!(coordinator.idle_agent_count(), 1);
    }

    #[test]
    fn coordinator_handles_task_failure() {
        let mut coordinator = Coordinator::new(2);
        let agent_id = coordinator.spawn_agent().unwrap();

        let task = TaskNode::new("T-1".to_string(), "content".to_string());
        coordinator.add_task(task);

        coordinator.assign_next_task();
        coordinator.fail_task(agent_id, "T-1");

        let agent = coordinator.get_agent(agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Failed);
        assert_eq!(coordinator.pending_task_count(), 0);
        assert_eq!(coordinator.in_progress_task_count(), 0);
    }

    #[test]
    fn parse_task_dependencies_extracts_deps() {
        let content = "### Task T-2\n- **ID** T-2\n- **Dependencies** T-1\n";
        let deps = parse_task_dependencies(content);
        assert_eq!(deps, vec!["T-1".to_string()]);
    }

    #[test]
    fn parse_task_dependencies_handles_multiple_deps() {
        let content = "### Task T-3\n- **ID** T-3\n- **Dependencies** T-1, T-2\n";
        let deps = parse_task_dependencies(content);
        assert_eq!(deps, vec!["T-1".to_string(), "T-2".to_string()]);
    }

    #[test]
    fn parse_task_dependencies_handles_none() {
        let content = "### Task T-1\n- **ID** T-1\n- **Dependencies** None\n";
        let deps = parse_task_dependencies(content);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_task_dependencies_handles_missing() {
        let content = "### Task T-1\n- **ID** T-1\n";
        let deps = parse_task_dependencies(content);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_task_id_extracts_from_header() {
        let content = "### Task MC-11\n- **ID** MC-11\n";
        let id = parse_task_id(content);
        assert_eq!(id, Some("MC-11".to_string()));
    }

    #[test]
    fn parse_task_id_extracts_from_id_line() {
        let content = "Some other text\n- **ID** T-1\n- **Dependencies** None\n";
        let id = parse_task_id(content);
        assert_eq!(id, Some("T-1".to_string()));
    }

    #[test]
    fn tasks_from_prd_parses_unchecked_tasks() {
        let prd = "# PRD\n\n### Task T-1\n- **ID** T-1\n- **Dependencies** None\n- [ ] T-1 Do something\n---\n### Task T-2\n- **ID** T-2\n- **Dependencies** T-1\n- [ ] T-2 Do another\n---\n### Task T-3\n- **ID** T-3\n- **Dependencies** None\n- [x] T-3 Already done\n";

        let tasks = tasks_from_prd(prd);

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "T-1");
        assert!(tasks[0].dependencies.is_empty());
        assert_eq!(tasks[1].id, "T-2");
        assert_eq!(tasks[1].dependencies, vec!["T-1".to_string()]);
    }

    #[test]
    fn topological_sort_orders_tasks_by_dependencies() {
        let tasks = vec![
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
        ];

        let sorted = topological_sort(&tasks).unwrap();

        let t1_pos = sorted.iter().position(|id| id == "T-1").unwrap();
        let t2_pos = sorted.iter().position(|id| id == "T-2").unwrap();
        let t3_pos = sorted.iter().position(|id| id == "T-3").unwrap();

        assert!(t1_pos < t2_pos);
        assert!(t2_pos < t3_pos);
    }

    #[test]
    fn topological_sort_detects_cycle() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
        ];

        let result = topological_sort(&tasks);
        assert!(matches!(result, Err(CoordinatorError::DependencyCycle(_))));
    }

    #[test]
    fn find_independent_tasks_returns_tasks_with_satisfied_deps() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string()),
        ];

        let completed = HashSet::new();
        let independent = find_independent_tasks(&tasks, &completed);

        assert_eq!(independent.len(), 2);
        let ids: Vec<_> = independent.iter().map(|t| &t.id).collect();
        assert!(ids.contains(&&"T-1".to_string()));
        assert!(ids.contains(&&"T-3".to_string()));
    }

    #[test]
    fn find_independent_tasks_excludes_completed() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
        ];

        let mut completed = HashSet::new();
        completed.insert("T-1".to_string());

        let independent = find_independent_tasks(&tasks, &completed);

        assert_eq!(independent.len(), 1);
        assert_eq!(independent[0].id, "T-2");
    }

    #[test]
    fn coordinator_error_display() {
        let err = CoordinatorError::NoAvailableAgents;
        assert_eq!(err.to_string(), "no available agents in pool");

        let err = CoordinatorError::TaskNotFound("T-1".to_string());
        assert_eq!(err.to_string(), "task not found: T-1");

        let err = CoordinatorError::DependencyCycle(vec!["T-1".to_string(), "T-2".to_string()]);
        assert_eq!(err.to_string(), "dependency cycle detected: T-1 -> T-2");

        let err = CoordinatorError::WorktreeError("failed".to_string());
        assert_eq!(err.to_string(), "worktree error: failed");

        let err = CoordinatorError::AgentFailed {
            agent_id: AgentId(0),
            reason: "timeout".to_string(),
        };
        assert_eq!(err.to_string(), "agent 0 failed: timeout");
    }

    #[test]
    fn agent_new_creates_idle_agent() {
        let agent = Agent::new(AgentId(0));
        assert_eq!(agent.status, AgentStatus::Idle);
        assert!(agent.worktree_path.is_none());
        assert!(agent.current_task.is_none());
    }

    #[test]
    fn task_node_with_dependencies() {
        let task = TaskNode::new("T-1".to_string(), "content".to_string())
            .with_dependencies(vec!["T-0".to_string()]);

        assert_eq!(task.id, "T-1");
        assert_eq!(task.content, "content");
        assert_eq!(task.dependencies, vec!["T-0".to_string()]);
    }

    #[test]
    fn coordinator_shutdown() {
        let coordinator = Coordinator::new(2);

        assert!(!coordinator.is_shutdown());
        coordinator.shutdown();
        assert!(coordinator.is_shutdown());
    }

    #[test]
    fn work_queue_default() {
        let queue = WorkQueue::default();
        assert!(queue.is_empty());
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn coordinator_add_multiple_tasks() {
        let coordinator = Coordinator::new(2);

        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
        ];
        coordinator.add_tasks(tasks);

        assert_eq!(coordinator.pending_task_count(), 2);
    }

    #[test]
    fn coordinator_all_tasks_complete() {
        let mut coordinator = Coordinator::new(2);
        let agent_id = coordinator.spawn_agent().unwrap();

        assert!(coordinator.all_tasks_complete());

        let task = TaskNode::new("T-1".to_string(), "".to_string());
        coordinator.add_task(task);

        assert!(!coordinator.all_tasks_complete());

        coordinator.assign_next_task();
        coordinator.complete_task(agent_id, "T-1");

        assert!(coordinator.all_tasks_complete());
    }

    #[test]
    fn build_dependency_graph_creates_correct_structure() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
        ];

        let graph = build_dependency_graph(&tasks);

        assert!(graph.get("T-1").unwrap().is_empty());
        assert_eq!(graph.get("T-2").unwrap(), &vec!["T-1".to_string()]);
    }

    #[test]
    fn topological_sort_handles_no_dependencies() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
            TaskNode::new("T-3".to_string(), "".to_string()),
        ];

        let sorted = topological_sort(&tasks).unwrap();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn topological_sort_handles_external_dependencies() {
        let tasks = vec![
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
        ];

        let sorted = topological_sort(&tasks).unwrap();
        assert_eq!(sorted, vec!["T-2".to_string()]);
    }
}
