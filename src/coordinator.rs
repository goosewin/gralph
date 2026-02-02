use crate::task::{is_unchecked_line, task_blocks_from_contents};
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

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

/// Manages isolated git worktrees for parallel agents.
///
/// Thread-safe: uses RwLock internally to allow concurrent reads and exclusive writes.
/// Each agent gets a unique worktree to operate in isolation from other agents.
#[derive(Debug)]
pub struct AgentWorktreeManager {
    /// Root directory containing all agent worktrees.
    worktrees_root: PathBuf,
    /// Repository root for creating worktrees from.
    repo_root: PathBuf,
    /// Map of agent IDs to their worktree paths.
    agent_worktrees: RwLock<HashMap<AgentId, PathBuf>>,
    /// Counter for generating unique worktree names.
    worktree_counter: AtomicUsize,
    /// Prefix for agent worktree branch names.
    branch_prefix: String,
}

/// Error types specific to worktree operations.
#[derive(Debug)]
pub enum WorktreeError {
    /// Git command failed.
    GitError(String),
    /// IO operation failed.
    IoError(std::io::Error),
    /// Worktree already exists for agent.
    AlreadyExists(AgentId),
    /// No worktree found for agent.
    NotFound(AgentId),
    /// Repository is not a git repository.
    NotARepository,
    /// Repository has no commits.
    NoCommits,
}

impl fmt::Display for WorktreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorktreeError::GitError(msg) => write!(f, "git error: {}", msg),
            WorktreeError::IoError(err) => write!(f, "io error: {}", err),
            WorktreeError::AlreadyExists(id) => {
                write!(f, "worktree already exists for agent {}", id.0)
            }
            WorktreeError::NotFound(id) => write!(f, "no worktree found for agent {}", id.0),
            WorktreeError::NotARepository => write!(f, "not a git repository"),
            WorktreeError::NoCommits => write!(f, "repository has no commits"),
        }
    }
}

impl Error for WorktreeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WorktreeError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WorktreeError {
    fn from(err: std::io::Error) -> Self {
        WorktreeError::IoError(err)
    }
}

impl AgentWorktreeManager {
    /// Creates a new worktree manager for the given repository.
    ///
    /// # Arguments
    /// * `repo_root` - Path to the git repository root
    /// * `worktrees_root` - Optional custom path for worktrees directory (defaults to repo_root/.worktrees/agents)
    /// * `branch_prefix` - Optional prefix for branch names (defaults to "agent")
    pub fn new(
        repo_root: PathBuf,
        worktrees_root: Option<PathBuf>,
        branch_prefix: Option<String>,
    ) -> Result<Self, WorktreeError> {
        // Verify repo_root is a git repository
        let repo_root = Self::resolve_repo_root(&repo_root)?;

        // Check repository has commits
        if !Self::repo_has_commits(&repo_root) {
            return Err(WorktreeError::NoCommits);
        }

        let worktrees_root =
            worktrees_root.unwrap_or_else(|| repo_root.join(".worktrees").join("agents"));

        fs::create_dir_all(&worktrees_root)?;

        Ok(Self {
            worktrees_root,
            repo_root,
            agent_worktrees: RwLock::new(HashMap::new()),
            worktree_counter: AtomicUsize::new(0),
            branch_prefix: branch_prefix.unwrap_or_else(|| "agent".to_string()),
        })
    }

    /// Creates a worktree for an agent with optional timestamp for unique naming.
    ///
    /// Thread-safe: acquires write lock only for the duration of registration.
    pub fn create_worktree_for_agent(
        &self,
        agent_id: AgentId,
        timestamp: Option<&str>,
    ) -> Result<PathBuf, WorktreeError> {
        // Check if worktree already exists for this agent
        {
            let worktrees = self.agent_worktrees.read().unwrap();
            if worktrees.contains_key(&agent_id) {
                return Err(WorktreeError::AlreadyExists(agent_id));
            }
        }

        // Generate unique branch and path names
        let counter = self.worktree_counter.fetch_add(1, Ordering::SeqCst);
        let timestamp_str = timestamp.unwrap_or("");
        let branch_name = if timestamp_str.is_empty() {
            format!("{}-{}-{}", self.branch_prefix, agent_id.0, counter)
        } else {
            format!(
                "{}-{}-{}-{}",
                self.branch_prefix, agent_id.0, counter, timestamp_str
            )
        };

        // Ensure branch name is unique
        let branch_name = self.ensure_unique_branch(&branch_name);
        let worktree_path = self.worktrees_root.join(&branch_name);

        // Create the worktree using git
        self.git_create_worktree(&branch_name, &worktree_path)?;

        // Register the worktree for this agent
        {
            let mut worktrees = self.agent_worktrees.write().unwrap();
            worktrees.insert(agent_id, worktree_path.clone());
        }

        Ok(worktree_path)
    }

    /// Gets the worktree path for an agent if it exists.
    pub fn get_worktree(&self, agent_id: AgentId) -> Option<PathBuf> {
        let worktrees = self.agent_worktrees.read().unwrap();
        worktrees.get(&agent_id).cloned()
    }

    /// Removes the worktree for an agent, cleaning up both the directory and git state.
    ///
    /// This should be called when an agent exits (successfully or due to failure).
    /// Thread-safe: acquires write lock only for the duration of deregistration.
    pub fn cleanup_agent_worktree(&self, agent_id: AgentId) -> Result<(), WorktreeError> {
        let worktree_path = {
            let worktrees = self.agent_worktrees.read().unwrap();
            worktrees
                .get(&agent_id)
                .cloned()
                .ok_or(WorktreeError::NotFound(agent_id))?
        };

        // Remove from git first
        self.git_remove_worktree(&worktree_path)?;

        // Clean up any remaining files (git worktree remove --force may leave some)
        if worktree_path.exists() {
            let _ = fs::remove_dir_all(&worktree_path);
        }

        // Remove from our tracking
        {
            let mut worktrees = self.agent_worktrees.write().unwrap();
            worktrees.remove(&agent_id);
        }

        Ok(())
    }

    /// Cleans up all agent worktrees. Called during coordinator shutdown.
    pub fn cleanup_all(&self) -> Vec<(AgentId, Result<(), WorktreeError>)> {
        let agent_ids: Vec<AgentId> = {
            let worktrees = self.agent_worktrees.read().unwrap();
            worktrees.keys().cloned().collect()
        };

        agent_ids
            .into_iter()
            .map(|id| {
                let result = self.cleanup_agent_worktree(id);
                (id, result)
            })
            .collect()
    }

    /// Returns the number of active agent worktrees.
    pub fn active_count(&self) -> usize {
        let worktrees = self.agent_worktrees.read().unwrap();
        worktrees.len()
    }

    /// Returns a list of all active agent worktree paths.
    pub fn list_worktrees(&self) -> Vec<(AgentId, PathBuf)> {
        let worktrees = self.agent_worktrees.read().unwrap();
        worktrees
            .iter()
            .map(|(id, path)| (*id, path.clone()))
            .collect()
    }

    // Helper: resolve repository root from a path
    fn resolve_repo_root(path: &Path) -> Result<PathBuf, WorktreeError> {
        let output = ProcCommand::new("git")
            .arg("-C")
            .arg(path)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| WorktreeError::IoError(e))?;

        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(PathBuf::from(root))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.to_lowercase().contains("not a git repository") {
                Err(WorktreeError::NotARepository)
            } else {
                Err(WorktreeError::GitError(stderr.to_string()))
            }
        }
    }

    // Helper: check if repository has commits
    fn repo_has_commits(repo_root: &Path) -> bool {
        Self::git_cmd_in_dir(repo_root, ["rev-parse", "--verify", "HEAD"]).is_ok()
    }

    // Helper: run git command in directory
    fn git_cmd_in_dir(
        dir: &Path,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<String, WorktreeError> {
        let output = ProcCommand::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .map_err(|e| WorktreeError::IoError(e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(WorktreeError::GitError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    // Helper: check if branch exists
    fn branch_exists(&self, branch: &str) -> bool {
        Self::git_cmd_in_dir(
            &self.repo_root,
            [
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", branch),
            ],
        )
        .is_ok()
    }

    // Helper: ensure unique branch name
    fn ensure_unique_branch(&self, base: &str) -> String {
        let mut candidate = base.to_string();
        let mut suffix = 2;
        while self.branch_exists(&candidate) || self.worktrees_root.join(&candidate).exists() {
            candidate = format!("{}-{}", base, suffix);
            suffix += 1;
        }
        candidate
    }

    // Helper: create git worktree
    fn git_create_worktree(&self, branch: &str, path: &Path) -> Result<(), WorktreeError> {
        Self::git_cmd_in_dir(
            &self.repo_root,
            [
                "worktree",
                "add",
                "-b",
                branch,
                path.to_string_lossy().as_ref(),
            ],
        )?;
        Ok(())
    }

    // Helper: remove git worktree
    fn git_remove_worktree(&self, path: &Path) -> Result<(), WorktreeError> {
        // Try force removal to handle locked/incomplete worktrees
        let result = Self::git_cmd_in_dir(
            &self.repo_root,
            ["worktree", "remove", "--force", path.to_string_lossy().as_ref()],
        );

        // If worktree is already gone or doesn't exist, that's fine
        if let Err(WorktreeError::GitError(msg)) = &result {
            if msg.contains("is not a working tree") || msg.contains("does not exist") {
                return Ok(());
            }
        }

        result.map(|_| ())
    }
}

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
    /// Optional worktree manager for agent isolation.
    worktree_manager: Option<Arc<AgentWorktreeManager>>,
}

impl Coordinator {
    pub fn new(max_agents: usize) -> Self {
        Self {
            agents: Vec::new(),
            work_queue: Arc::new(Mutex::new(WorkQueue::new())),
            max_agents,
            next_agent_id: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
            worktree_manager: None,
        }
    }

    /// Creates a new coordinator with an attached worktree manager for agent isolation.
    pub fn with_worktree_manager(
        max_agents: usize,
        worktree_manager: AgentWorktreeManager,
    ) -> Self {
        Self {
            agents: Vec::new(),
            work_queue: Arc::new(Mutex::new(WorkQueue::new())),
            max_agents,
            next_agent_id: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
            worktree_manager: Some(Arc::new(worktree_manager)),
        }
    }

    /// Returns a reference to the worktree manager if configured.
    pub fn worktree_manager(&self) -> Option<&AgentWorktreeManager> {
        self.worktree_manager.as_ref().map(|arc| arc.as_ref())
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

    /// Spawns a new agent with an automatically created isolated worktree.
    ///
    /// Requires a worktree manager to be configured. Returns the agent ID and worktree path.
    pub fn spawn_agent_with_isolated_worktree(
        &mut self,
        timestamp: Option<&str>,
    ) -> Result<(AgentId, PathBuf), CoordinatorError> {
        if self.agents.len() >= self.max_agents {
            return Err(CoordinatorError::NoAvailableAgents);
        }

        let manager = self
            .worktree_manager
            .as_ref()
            .ok_or_else(|| CoordinatorError::WorktreeError("no worktree manager configured".to_string()))?;

        let id = AgentId(self.next_agent_id.fetch_add(1, Ordering::SeqCst));

        let worktree_path = manager
            .create_worktree_for_agent(id, timestamp)
            .map_err(|e| CoordinatorError::WorktreeError(e.to_string()))?;

        let agent = Agent::new(id).with_worktree(worktree_path.clone());
        self.agents.push(agent);

        Ok((id, worktree_path))
    }

    /// Removes an agent and cleans up its worktree if one was allocated.
    pub fn remove_agent(&mut self, agent_id: AgentId) -> Result<(), CoordinatorError> {
        // Find and remove the agent
        let agent_index = self.agents.iter().position(|a| a.id == agent_id);
        if agent_index.is_none() {
            return Err(CoordinatorError::AgentFailed {
                agent_id,
                reason: "agent not found".to_string(),
            });
        }
        let agent = self.agents.remove(agent_index.unwrap());

        // Clean up worktree if manager is configured and agent had a worktree
        if agent.worktree_path.is_some() {
            if let Some(manager) = &self.worktree_manager {
                let _ = manager.cleanup_agent_worktree(agent_id);
            }
        }

        Ok(())
    }

    /// Cleans up all agent worktrees during shutdown.
    pub fn cleanup_all_worktrees(&self) {
        if let Some(manager) = &self.worktree_manager {
            manager.cleanup_all();
        }
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

/// Represents the dependency graph for task scheduling and parallel execution analysis.
///
/// The `DependencyGraph` provides methods to analyze task dependencies, identify
/// independent task blocks that can run in parallel, and order execution based
/// on dependency relationships.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// All tasks in the graph indexed by their ID.
    tasks: HashMap<String, TaskNode>,
    /// Adjacency list: task ID -> list of tasks that depend on it (reverse edges).
    dependents: HashMap<String, Vec<String>>,
    /// Number of unresolved dependencies for each task.
    in_degree: HashMap<String, usize>,
}

impl DependencyGraph {
    /// Creates a new dependency graph from a list of task nodes.
    ///
    /// Only includes dependencies that reference tasks within the provided list.
    /// External dependencies (referencing tasks not in the list) are ignored.
    pub fn new(tasks: Vec<TaskNode>) -> Self {
        let task_ids: HashSet<_> = tasks.iter().map(|t| t.id.clone()).collect();

        let mut tasks_map = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize all tasks with zero in-degree and empty dependents
        for task in &tasks {
            in_degree.insert(task.id.clone(), 0);
            dependents.insert(task.id.clone(), Vec::new());
        }

        // Build the graph
        for task in &tasks {
            let mut valid_dep_count = 0;
            for dep in &task.dependencies {
                // Only count dependencies that are part of the task set
                if task_ids.contains(dep) {
                    valid_dep_count += 1;
                    dependents
                        .get_mut(dep)
                        .unwrap()
                        .push(task.id.clone());
                }
            }
            *in_degree.get_mut(&task.id).unwrap() = valid_dep_count;
            tasks_map.insert(task.id.clone(), task.clone());
        }

        Self {
            tasks: tasks_map,
            dependents,
            in_degree,
        }
    }

    /// Creates a dependency graph from PRD content.
    ///
    /// Parses the PRD to extract unchecked tasks with their dependencies.
    pub fn from_prd(prd_content: &str) -> Self {
        let tasks = tasks_from_prd(prd_content);
        Self::new(tasks)
    }

    /// Returns the number of tasks in the graph.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Returns a reference to a task by its ID.
    pub fn get_task(&self, id: &str) -> Option<&TaskNode> {
        self.tasks.get(id)
    }

    /// Returns all task IDs in the graph.
    pub fn task_ids(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    /// Checks if the graph contains a dependency cycle.
    ///
    /// Returns `Ok(())` if no cycle exists, or `Err(CoordinatorError::DependencyCycle)`
    /// with the tasks involved in the cycle.
    pub fn validate_no_cycles(&self) -> Result<(), CoordinatorError> {
        let tasks: Vec<_> = self.tasks.values().cloned().collect();
        topological_sort(&tasks).map(|_| ())
    }

    /// Returns the in-degree (number of unresolved dependencies) for a task.
    pub fn get_in_degree(&self, task_id: &str) -> usize {
        self.in_degree.get(task_id).copied().unwrap_or(0)
    }

    /// Returns a list of task IDs that directly depend on the given task.
    pub fn get_dependents(&self, task_id: &str) -> Vec<String> {
        self.dependents.get(task_id).cloned().unwrap_or_default()
    }

    /// Returns all tasks with no dependencies (in-degree of 0).
    ///
    /// These tasks can be executed immediately without waiting for any prerequisites.
    pub fn get_root_tasks(&self) -> Vec<&TaskNode> {
        self.tasks
            .values()
            .filter(|task| self.in_degree.get(&task.id).copied().unwrap_or(0) == 0)
            .collect()
    }

    /// Computes execution levels for parallel scheduling.
    ///
    /// Returns a vector of task ID groups, where each group represents tasks
    /// that can be executed in parallel. Groups are ordered by execution level:
    /// - Level 0: Tasks with no dependencies (can start immediately)
    /// - Level 1: Tasks whose dependencies are all in level 0
    /// - Level N: Tasks whose dependencies are all in levels 0..N-1
    ///
    /// Returns an error if a cycle is detected.
    pub fn get_execution_levels(&self) -> Result<Vec<Vec<String>>, CoordinatorError> {
        if self.tasks.is_empty() {
            return Ok(Vec::new());
        }

        // Validate no cycles first
        self.validate_no_cycles()?;

        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut remaining_in_degree = self.in_degree.clone();
        let mut scheduled: HashSet<String> = HashSet::new();

        loop {
            // Find all tasks with in-degree 0 that haven't been scheduled
            let ready: Vec<String> = remaining_in_degree
                .iter()
                .filter(|(id, degree)| **degree == 0 && !scheduled.contains(*id))
                .map(|(id, _)| id.clone())
                .collect();

            if ready.is_empty() {
                break;
            }

            // Mark all ready tasks as scheduled
            for task_id in &ready {
                scheduled.insert(task_id.clone());

                // Decrement in-degree of all dependents
                if let Some(deps) = self.dependents.get(task_id) {
                    for dep_id in deps {
                        if let Some(degree) = remaining_in_degree.get_mut(dep_id) {
                            *degree = degree.saturating_sub(1);
                        }
                    }
                }
            }

            levels.push(ready);
        }

        Ok(levels)
    }

    /// Returns the maximum parallelism achievable at any execution level.
    ///
    /// This is the maximum number of tasks that can run concurrently.
    pub fn max_parallelism(&self) -> Result<usize, CoordinatorError> {
        let levels = self.get_execution_levels()?;
        Ok(levels.iter().map(|level| level.len()).max().unwrap_or(0))
    }

    /// Identifies independent task blocks for parallel execution.
    ///
    /// Given a set of already-completed tasks, returns tasks that can be
    /// scheduled concurrently because all their dependencies are satisfied.
    pub fn get_ready_tasks(&self, completed: &HashSet<String>) -> Vec<&TaskNode> {
        self.tasks
            .values()
            .filter(|task| {
                !completed.contains(&task.id)
                    && task.dependencies.iter().all(|dep| {
                        // Dependency is satisfied if:
                        // 1. It's completed, OR
                        // 2. It's not part of this graph (external dependency)
                        completed.contains(dep) || !self.tasks.contains_key(dep)
                    })
            })
            .collect()
    }

    /// Schedules tasks for execution, respecting dependencies and capacity limits.
    ///
    /// Returns up to `max_concurrent` tasks that can be executed in parallel,
    /// given the set of already-completed tasks and currently in-progress tasks.
    pub fn schedule_tasks(
        &self,
        completed: &HashSet<String>,
        in_progress: &HashSet<String>,
        max_concurrent: usize,
    ) -> Vec<&TaskNode> {
        if max_concurrent == 0 {
            return Vec::new();
        }

        let available_slots = max_concurrent.saturating_sub(in_progress.len());
        if available_slots == 0 {
            return Vec::new();
        }

        let ready = self.get_ready_tasks(completed);
        ready
            .into_iter()
            .filter(|task| !in_progress.contains(&task.id))
            .take(available_slots)
            .collect()
    }

    /// Computes the critical path length (longest dependency chain).
    ///
    /// This represents the minimum number of sequential execution steps required,
    /// assuming unlimited parallelism.
    pub fn critical_path_length(&self) -> Result<usize, CoordinatorError> {
        let levels = self.get_execution_levels()?;
        Ok(levels.len())
    }

    /// Returns statistics about the dependency graph.
    pub fn stats(&self) -> DependencyGraphStats {
        let task_count = self.tasks.len();
        let root_count = self.get_root_tasks().len();
        let max_parallelism = self.max_parallelism().unwrap_or(0);
        let critical_path = self.critical_path_length().unwrap_or(0);

        let total_edges: usize = self.tasks.values().map(|t| t.dependencies.len()).sum();

        DependencyGraphStats {
            task_count,
            root_count,
            max_parallelism,
            critical_path_length: critical_path,
            total_dependency_edges: total_edges,
        }
    }
}

/// Statistics about a dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraphStats {
    /// Total number of tasks in the graph.
    pub task_count: usize,
    /// Number of tasks with no dependencies (can start immediately).
    pub root_count: usize,
    /// Maximum number of tasks that can run concurrently at any level.
    pub max_parallelism: usize,
    /// Minimum number of sequential steps required (critical path).
    pub critical_path_length: usize,
    /// Total number of dependency edges in the graph.
    pub total_dependency_edges: usize,
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

    #[test]
    fn worktree_error_display() {
        let err = WorktreeError::GitError("failed".to_string());
        assert_eq!(err.to_string(), "git error: failed");

        let err = WorktreeError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        assert!(err.to_string().contains("io error:"));

        let err = WorktreeError::AlreadyExists(AgentId(1));
        assert_eq!(err.to_string(), "worktree already exists for agent 1");

        let err = WorktreeError::NotFound(AgentId(2));
        assert_eq!(err.to_string(), "no worktree found for agent 2");

        let err = WorktreeError::NotARepository;
        assert_eq!(err.to_string(), "not a git repository");

        let err = WorktreeError::NoCommits;
        assert_eq!(err.to_string(), "repository has no commits");
    }

    #[test]
    fn worktree_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = WorktreeError::IoError(io_err);
        assert!(err.source().is_some());

        let err = WorktreeError::GitError("failed".to_string());
        assert!(err.source().is_none());
    }

    #[test]
    fn worktree_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: WorktreeError = io_err.into();
        assert!(matches!(err, WorktreeError::IoError(_)));
    }

    #[test]
    fn coordinator_with_worktree_manager_returns_manager() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let coordinator = Coordinator::with_worktree_manager(5, manager);

        assert!(coordinator.worktree_manager().is_some());
    }

    #[test]
    fn coordinator_without_worktree_manager() {
        let coordinator = Coordinator::new(5);
        assert!(coordinator.worktree_manager().is_none());
    }

    #[test]
    fn coordinator_spawn_isolated_worktree_fails_without_manager() {
        let mut coordinator = Coordinator::new(5);
        let result = coordinator.spawn_agent_with_isolated_worktree(None);

        assert!(result.is_err());
        match result.unwrap_err() {
            CoordinatorError::WorktreeError(msg) => {
                assert!(msg.contains("no worktree manager"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn coordinator_spawn_isolated_worktree_creates_worktree() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let mut coordinator = Coordinator::with_worktree_manager(5, manager);

        let result = coordinator.spawn_agent_with_isolated_worktree(Some("20260201-120000"));
        assert!(result.is_ok());

        let (agent_id, worktree_path) = result.unwrap();
        assert!(worktree_path.exists());

        // Verify agent has worktree path set
        let agent = coordinator.get_agent(agent_id).unwrap();
        assert_eq!(agent.worktree_path, Some(worktree_path.clone()));

        // Cleanup
        coordinator.cleanup_all_worktrees();
    }

    #[test]
    fn coordinator_remove_agent_cleans_up_worktree() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let mut coordinator = Coordinator::with_worktree_manager(5, manager);

        let (agent_id, worktree_path) = coordinator
            .spawn_agent_with_isolated_worktree(None)
            .unwrap();

        assert!(worktree_path.exists());

        // Remove agent should cleanup worktree
        coordinator.remove_agent(agent_id).unwrap();

        // Worktree should be removed
        assert!(!worktree_path.exists());
        assert!(coordinator.get_agent(agent_id).is_none());
    }

    #[test]
    fn coordinator_remove_agent_not_found() {
        let mut coordinator = Coordinator::new(5);
        let result = coordinator.remove_agent(AgentId(999));

        assert!(result.is_err());
        match result.unwrap_err() {
            CoordinatorError::AgentFailed { agent_id, .. } => {
                assert_eq!(agent_id, AgentId(999));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn coordinator_cleanup_all_worktrees_cleans_up() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let mut coordinator = Coordinator::with_worktree_manager(5, manager);

        // Spawn multiple agents with worktrees
        let (_, path1) = coordinator
            .spawn_agent_with_isolated_worktree(Some("t1"))
            .unwrap();
        let (_, path2) = coordinator
            .spawn_agent_with_isolated_worktree(Some("t2"))
            .unwrap();

        assert!(path1.exists());
        assert!(path2.exists());

        // Cleanup all
        coordinator.cleanup_all_worktrees();

        // Both should be removed
        assert!(!path1.exists());
        assert!(!path2.exists());
    }

    // Agent worktree manager tests

    #[test]
    fn agent_worktree_manager_requires_git_repo() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        // Don't initialize as git repo

        let result = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            WorktreeError::NotARepository => {}
            other => panic!("expected NotARepository, got: {:?}", other),
        }
    }

    #[test]
    fn agent_worktree_manager_requires_commits() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        // Initialize repo but no commits
        run_git(temp.path(), &["init"]);

        let result = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            WorktreeError::NoCommits => {}
            other => panic!("expected NoCommits, got: {:?}", other),
        }
    }

    #[test]
    fn agent_worktree_manager_creates_worktrees_root() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let worktrees_root = temp.path().join("custom-worktrees");
        let manager = AgentWorktreeManager::new(
            temp.path().to_path_buf(),
            Some(worktrees_root.clone()),
            None,
        )
        .unwrap();

        assert!(worktrees_root.exists());
        drop(manager);
    }

    #[test]
    fn agent_worktree_manager_creates_unique_worktrees() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();

        let agent1 = AgentId(0);
        let agent2 = AgentId(1);

        let path1 = manager.create_worktree_for_agent(agent1, None).unwrap();
        let path2 = manager.create_worktree_for_agent(agent2, None).unwrap();

        // Paths should be different
        assert_ne!(path1, path2);
        assert!(path1.exists());
        assert!(path2.exists());

        // Both should be tracked
        assert_eq!(manager.active_count(), 2);

        // Cleanup
        manager.cleanup_all();
    }

    #[test]
    fn agent_worktree_manager_prevents_duplicate_worktrees() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let agent_id = AgentId(0);

        manager.create_worktree_for_agent(agent_id, None).unwrap();

        // Second attempt should fail
        let result = manager.create_worktree_for_agent(agent_id, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            WorktreeError::AlreadyExists(id) => {
                assert_eq!(id, agent_id);
            }
            other => panic!("expected AlreadyExists, got: {:?}", other),
        }

        // Cleanup
        manager.cleanup_all();
    }

    #[test]
    fn agent_worktree_manager_get_worktree() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let agent_id = AgentId(0);

        assert!(manager.get_worktree(agent_id).is_none());

        let path = manager.create_worktree_for_agent(agent_id, None).unwrap();
        assert_eq!(manager.get_worktree(agent_id), Some(path));

        // Cleanup
        manager.cleanup_all();
    }

    #[test]
    fn agent_worktree_manager_cleanup_removes_worktree() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let agent_id = AgentId(0);

        let path = manager.create_worktree_for_agent(agent_id, None).unwrap();
        assert!(path.exists());
        assert_eq!(manager.active_count(), 1);

        manager.cleanup_agent_worktree(agent_id).unwrap();

        assert!(!path.exists());
        assert_eq!(manager.active_count(), 0);
        assert!(manager.get_worktree(agent_id).is_none());
    }

    #[test]
    fn agent_worktree_manager_cleanup_not_found() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();
        let result = manager.cleanup_agent_worktree(AgentId(999));

        assert!(result.is_err());
        match result.unwrap_err() {
            WorktreeError::NotFound(id) => {
                assert_eq!(id, AgentId(999));
            }
            other => panic!("expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn agent_worktree_manager_list_worktrees() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();

        let agent1 = AgentId(0);
        let agent2 = AgentId(1);

        let path1 = manager.create_worktree_for_agent(agent1, None).unwrap();
        let path2 = manager.create_worktree_for_agent(agent2, None).unwrap();

        let list = manager.list_worktrees();
        assert_eq!(list.len(), 2);

        let ids: Vec<_> = list.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&agent1));
        assert!(ids.contains(&agent2));

        let paths: Vec<_> = list.iter().map(|(_, p)| p.clone()).collect();
        assert!(paths.contains(&path1));
        assert!(paths.contains(&path2));

        // Cleanup
        manager.cleanup_all();
    }

    #[test]
    fn agent_worktree_manager_cleanup_all() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();

        let path1 = manager.create_worktree_for_agent(AgentId(0), None).unwrap();
        let path2 = manager.create_worktree_for_agent(AgentId(1), None).unwrap();

        assert_eq!(manager.active_count(), 2);

        let results = manager.cleanup_all();
        assert_eq!(results.len(), 2);

        // All should succeed
        for (_, result) in &results {
            assert!(result.is_ok());
        }

        assert_eq!(manager.active_count(), 0);
        assert!(!path1.exists());
        assert!(!path2.exists());
    }

    #[test]
    fn agent_worktree_manager_custom_branch_prefix() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(
            temp.path().to_path_buf(),
            None,
            Some("worker".to_string()),
        )
        .unwrap();

        let path = manager.create_worktree_for_agent(AgentId(0), None).unwrap();

        // Path should include the custom prefix
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("worker-0"));

        // Cleanup
        manager.cleanup_all();
    }

    #[test]
    fn agent_worktree_manager_with_timestamp() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager = AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap();

        let path = manager
            .create_worktree_for_agent(AgentId(0), Some("20260201-120000"))
            .unwrap();

        // Path should include timestamp
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("20260201-120000"));

        // Cleanup
        manager.cleanup_all();
    }

    #[test]
    fn agent_worktree_manager_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_test_repo(temp.path());

        let manager =
            Arc::new(AgentWorktreeManager::new(temp.path().to_path_buf(), None, None).unwrap());

        let mut handles = vec![];

        // Spawn multiple threads that create worktrees
        for i in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let agent_id = AgentId(i);
                manager_clone.create_worktree_for_agent(agent_id, None)
            });
            handles.push(handle);
        }

        // Collect results
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should succeed
        for result in &results {
            assert!(result.is_ok());
        }

        // Should have 5 active worktrees
        assert_eq!(manager.active_count(), 5);

        // All paths should be unique
        let paths: Vec<_> = results.iter().filter_map(|r| r.as_ref().ok()).collect();
        let unique_paths: HashSet<_> = paths.iter().collect();
        assert_eq!(unique_paths.len(), 5);

        // Cleanup
        manager.cleanup_all();
    }

    // Helper functions for tests

    fn run_git(dir: &Path, args: &[&str]) {
        let output = ProcCommand::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_test_repo(dir: &Path) {
        run_git(dir, &["init"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test User"]);
        fs::write(dir.join("README.md"), "init\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "-m", "init"]);
    }

    // DependencyGraph tests

    #[test]
    fn dependency_graph_empty() {
        let graph = DependencyGraph::new(Vec::new());
        assert_eq!(graph.task_count(), 0);
        assert!(graph.get_root_tasks().is_empty());
        assert_eq!(graph.get_execution_levels().unwrap(), Vec::<Vec<String>>::new());
        assert_eq!(graph.max_parallelism().unwrap(), 0);
        assert_eq!(graph.critical_path_length().unwrap(), 0);
    }

    #[test]
    fn dependency_graph_single_task_no_deps() {
        let tasks = vec![TaskNode::new("T-1".to_string(), "content".to_string())];
        let graph = DependencyGraph::new(tasks);

        assert_eq!(graph.task_count(), 1);
        assert_eq!(graph.get_root_tasks().len(), 1);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0], vec!["T-1".to_string()]);

        assert_eq!(graph.max_parallelism().unwrap(), 1);
        assert_eq!(graph.critical_path_length().unwrap(), 1);
    }

    #[test]
    fn dependency_graph_multiple_independent_tasks() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
            TaskNode::new("T-3".to_string(), "".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        assert_eq!(graph.task_count(), 3);
        assert_eq!(graph.get_root_tasks().len(), 3);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 3);

        assert_eq!(graph.max_parallelism().unwrap(), 3);
        assert_eq!(graph.critical_path_length().unwrap(), 1);
    }

    #[test]
    fn dependency_graph_linear_chain() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        assert_eq!(graph.task_count(), 3);
        assert_eq!(graph.get_root_tasks().len(), 1);
        assert_eq!(graph.get_root_tasks()[0].id, "T-1");

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["T-1".to_string()]);
        assert_eq!(levels[1], vec!["T-2".to_string()]);
        assert_eq!(levels[2], vec!["T-3".to_string()]);

        assert_eq!(graph.max_parallelism().unwrap(), 1);
        assert_eq!(graph.critical_path_length().unwrap(), 3);
    }

    #[test]
    fn dependency_graph_diamond_pattern() {
        // T-1 -> T-2, T-3 -> T-4
        //        T-2 --|
        //              +-> T-4
        //        T-3 --|
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-4".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string(), "T-3".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        assert_eq!(graph.task_count(), 4);
        assert_eq!(graph.get_root_tasks().len(), 1);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["T-1".to_string()]);
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&"T-2".to_string()));
        assert!(levels[1].contains(&"T-3".to_string()));
        assert_eq!(levels[2], vec!["T-4".to_string()]);

        assert_eq!(graph.max_parallelism().unwrap(), 2);
        assert_eq!(graph.critical_path_length().unwrap(), 3);
    }

    #[test]
    fn dependency_graph_detects_cycle() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        assert!(graph.validate_no_cycles().is_err());
        assert!(graph.get_execution_levels().is_err());
    }

    #[test]
    fn dependency_graph_ignores_external_deps() {
        // T-2 depends on T-1, but T-1 is not in the graph
        let tasks = vec![TaskNode::new("T-2".to_string(), "".to_string())
            .with_dependencies(vec!["T-1".to_string()])];
        let graph = DependencyGraph::new(tasks);

        assert_eq!(graph.task_count(), 1);
        // T-2 should be a root since T-1 is external
        assert_eq!(graph.get_root_tasks().len(), 1);
        assert_eq!(graph.get_in_degree("T-2"), 0);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 1);
    }

    #[test]
    fn dependency_graph_get_dependents() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        let deps = graph.get_dependents("T-1");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"T-2".to_string()));
        assert!(deps.contains(&"T-3".to_string()));

        assert!(graph.get_dependents("T-2").is_empty());
        assert!(graph.get_dependents("T-3").is_empty());
        assert!(graph.get_dependents("nonexistent").is_empty());
    }

    #[test]
    fn dependency_graph_get_ready_tasks() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        let completed = HashSet::new();
        let ready = graph.get_ready_tasks(&completed);
        let ready_ids: Vec<_> = ready.iter().map(|t| &t.id).collect();
        assert_eq!(ready.len(), 2);
        assert!(ready_ids.contains(&&"T-1".to_string()));
        assert!(ready_ids.contains(&&"T-3".to_string()));

        let mut completed = HashSet::new();
        completed.insert("T-1".to_string());
        let ready = graph.get_ready_tasks(&completed);
        let ready_ids: Vec<_> = ready.iter().map(|t| &t.id).collect();
        assert_eq!(ready.len(), 2);
        assert!(ready_ids.contains(&&"T-2".to_string()));
        assert!(ready_ids.contains(&&"T-3".to_string()));
    }

    #[test]
    fn dependency_graph_schedule_tasks_respects_capacity() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
            TaskNode::new("T-3".to_string(), "".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        let completed = HashSet::new();
        let in_progress = HashSet::new();

        // Request at most 2
        let scheduled = graph.schedule_tasks(&completed, &in_progress, 2);
        assert_eq!(scheduled.len(), 2);

        // Request 0
        let scheduled = graph.schedule_tasks(&completed, &in_progress, 0);
        assert!(scheduled.is_empty());
    }

    #[test]
    fn dependency_graph_schedule_tasks_excludes_in_progress() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        let completed = HashSet::new();
        let mut in_progress = HashSet::new();
        in_progress.insert("T-1".to_string());

        let scheduled = graph.schedule_tasks(&completed, &in_progress, 5);
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].id, "T-2");
    }

    #[test]
    fn dependency_graph_schedule_tasks_respects_slots() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
            TaskNode::new("T-3".to_string(), "".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        let completed = HashSet::new();
        let mut in_progress = HashSet::new();
        in_progress.insert("T-1".to_string());

        // max_concurrent=2, 1 in progress, so only 1 slot available
        let scheduled = graph.schedule_tasks(&completed, &in_progress, 2);
        assert_eq!(scheduled.len(), 1);
    }

    #[test]
    fn dependency_graph_stats() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-4".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string(), "T-3".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        let stats = graph.stats();
        assert_eq!(stats.task_count, 4);
        assert_eq!(stats.root_count, 1);
        assert_eq!(stats.max_parallelism, 2);
        assert_eq!(stats.critical_path_length, 3);
        assert_eq!(stats.total_dependency_edges, 4); // T-2->T-1, T-3->T-1, T-4->T-2, T-4->T-3
    }

    #[test]
    fn dependency_graph_from_prd() {
        let prd = r#"# PRD

### Task T-1
- **ID** T-1
- **Dependencies** None
- [ ] T-1 First task
---
### Task T-2
- **ID** T-2
- **Dependencies** T-1
- [ ] T-2 Second task
---
### Task T-3
- **ID** T-3
- **Dependencies** None
- [ ] T-3 Third task
---
### Task T-4
- **ID** T-4
- **Dependencies** T-2, T-3
- [ ] T-4 Fourth task
"#;

        let graph = DependencyGraph::from_prd(prd);
        assert_eq!(graph.task_count(), 4);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 3);

        // Level 0: T-1 and T-3 (no deps)
        assert_eq!(levels[0].len(), 2);
        assert!(levels[0].contains(&"T-1".to_string()));
        assert!(levels[0].contains(&"T-3".to_string()));

        // Level 1: T-2 (depends on T-1)
        assert_eq!(levels[1], vec!["T-2".to_string()]);

        // Level 2: T-4 (depends on T-2, T-3)
        assert_eq!(levels[2], vec!["T-4".to_string()]);
    }

    #[test]
    fn dependency_graph_complex_parallel() {
        // Complex graph with multiple parallel branches
        // T-1 ---> T-2 ---> T-5
        // T-3 ---> T-4 --/
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string()),
            TaskNode::new("T-4".to_string(), "".to_string())
                .with_dependencies(vec!["T-3".to_string()]),
            TaskNode::new("T-5".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string(), "T-4".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels.len(), 3);

        // Level 0: T-1, T-3
        assert_eq!(levels[0].len(), 2);
        assert!(levels[0].contains(&"T-1".to_string()));
        assert!(levels[0].contains(&"T-3".to_string()));

        // Level 1: T-2, T-4
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&"T-2".to_string()));
        assert!(levels[1].contains(&"T-4".to_string()));

        // Level 2: T-5
        assert_eq!(levels[2], vec!["T-5".to_string()]);

        assert_eq!(graph.max_parallelism().unwrap(), 2);
    }

    #[test]
    fn dependency_graph_get_task() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "content1".to_string()),
            TaskNode::new("T-2".to_string(), "content2".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        let task = graph.get_task("T-1").unwrap();
        assert_eq!(task.id, "T-1");
        assert_eq!(task.content, "content1");

        assert!(graph.get_task("nonexistent").is_none());
    }

    #[test]
    fn dependency_graph_task_ids() {
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string()),
        ];
        let graph = DependencyGraph::new(tasks);

        let ids = graph.task_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"T-1".to_string()));
        assert!(ids.contains(&"T-2".to_string()));
    }

    #[test]
    fn dependency_graph_three_level_cycle() {
        // T-1 -> T-2 -> T-3 -> T-1 (cycle)
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string())
                .with_dependencies(vec!["T-3".to_string()]),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        let result = graph.validate_no_cycles();
        assert!(matches!(result, Err(CoordinatorError::DependencyCycle(_))));
    }

    #[test]
    fn dependency_graph_partial_cycle() {
        // T-1 (no deps), T-2 -> T-3 -> T-2 (cycle)
        let tasks = vec![
            TaskNode::new("T-1".to_string(), "".to_string()),
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-3".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        // T-1 is a root and can be scheduled
        assert_eq!(graph.get_root_tasks().len(), 1);
        assert_eq!(graph.get_root_tasks()[0].id, "T-1");

        // But validation should fail due to cycle in T-2, T-3
        let result = graph.validate_no_cycles();
        assert!(matches!(result, Err(CoordinatorError::DependencyCycle(_))));
    }

    #[test]
    fn dependency_graph_wide_parallel() {
        // All tasks at level 0, then all depend on all at level 1
        let tasks = vec![
            TaskNode::new("A-1".to_string(), "".to_string()),
            TaskNode::new("A-2".to_string(), "".to_string()),
            TaskNode::new("A-3".to_string(), "".to_string()),
            TaskNode::new("A-4".to_string(), "".to_string()),
            TaskNode::new("A-5".to_string(), "".to_string()),
            TaskNode::new("B-1".to_string(), "".to_string())
                .with_dependencies(vec!["A-1".to_string(), "A-2".to_string(), "A-3".to_string(), "A-4".to_string(), "A-5".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        assert_eq!(graph.get_root_tasks().len(), 5);
        assert_eq!(graph.max_parallelism().unwrap(), 5);
        assert_eq!(graph.critical_path_length().unwrap(), 2);

        let levels = graph.get_execution_levels().unwrap();
        assert_eq!(levels[0].len(), 5);
        assert_eq!(levels[1].len(), 1);
    }

    #[test]
    fn dependency_graph_stats_empty() {
        let graph = DependencyGraph::new(Vec::new());
        let stats = graph.stats();

        assert_eq!(stats.task_count, 0);
        assert_eq!(stats.root_count, 0);
        assert_eq!(stats.max_parallelism, 0);
        assert_eq!(stats.critical_path_length, 0);
        assert_eq!(stats.total_dependency_edges, 0);
    }

    #[test]
    fn dependency_graph_with_completed_external_deps() {
        // T-2 depends on T-1 (external), T-3 depends on T-2
        let tasks = vec![
            TaskNode::new("T-2".to_string(), "".to_string())
                .with_dependencies(vec!["T-1".to_string()]),
            TaskNode::new("T-3".to_string(), "".to_string())
                .with_dependencies(vec!["T-2".to_string()]),
        ];
        let graph = DependencyGraph::new(tasks);

        // T-2 should be ready since T-1 is external
        let completed = HashSet::new();
        let ready = graph.get_ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "T-2");

        // After completing T-2, T-3 should be ready
        let mut completed = HashSet::new();
        completed.insert("T-2".to_string());
        let ready = graph.get_ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "T-3");
    }
}
