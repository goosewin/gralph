use crate::task::{is_unchecked_line, task_blocks_from_contents};
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

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

// ============================================================================
// Load Balancer Implementation (MC-14)
// ============================================================================

/// Strategy for distributing tasks across agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadBalanceStrategy {
    /// Round-robin distribution: each agent gets tasks in turn.
    #[default]
    RoundRobin,
    /// Weighted distribution: agents with higher weights get more tasks.
    Weighted,
    /// Least-loaded distribution: prefer agents with fewer active tasks.
    LeastLoaded,
}

/// Health status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Agent is healthy and available for work.
    Healthy,
    /// Agent is unhealthy and should not receive new tasks.
    Unhealthy,
    /// Agent health is unknown (no recent checks).
    Unknown,
}

/// Agent metadata for load balancing decisions.
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    /// Agent identifier.
    pub agent_id: AgentId,
    /// Weight for weighted distribution (higher = more tasks). Default is 1.
    pub weight: u32,
    /// Current health status.
    pub health_status: HealthStatus,
    /// Last successful health check timestamp.
    pub last_health_check: Option<Instant>,
    /// Number of consecutive health check failures.
    pub consecutive_failures: u32,
    /// Total tasks completed by this agent.
    pub tasks_completed: u64,
    /// Total tasks failed by this agent.
    pub tasks_failed: u64,
    /// Current active task count.
    pub active_tasks: u32,
}

impl AgentMetadata {
    /// Creates new agent metadata with default values.
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            weight: 1,
            health_status: HealthStatus::Unknown,
            last_health_check: None,
            consecutive_failures: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            active_tasks: 0,
        }
    }

    /// Creates agent metadata with a specific weight.
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight.max(1); // Ensure minimum weight of 1
        self
    }

    /// Marks the agent as healthy after a successful health check.
    pub fn mark_healthy(&mut self) {
        self.health_status = HealthStatus::Healthy;
        self.last_health_check = Some(Instant::now());
        self.consecutive_failures = 0;
    }

    /// Marks the agent as unhealthy after a failed health check.
    pub fn mark_unhealthy(&mut self) {
        self.consecutive_failures += 1;
        self.last_health_check = Some(Instant::now());
        self.health_status = HealthStatus::Unhealthy;
    }

    /// Increments the completed task count.
    pub fn record_task_completed(&mut self) {
        self.tasks_completed += 1;
        self.active_tasks = self.active_tasks.saturating_sub(1);
    }

    /// Increments the failed task count.
    pub fn record_task_failed(&mut self) {
        self.tasks_failed += 1;
        self.active_tasks = self.active_tasks.saturating_sub(1);
    }

    /// Increments the active task count.
    pub fn record_task_assigned(&mut self) {
        self.active_tasks += 1;
    }

    /// Returns true if the agent is available for new tasks.
    pub fn is_available(&self) -> bool {
        self.health_status != HealthStatus::Unhealthy
    }

    /// Returns the effective weight considering health status.
    /// Unhealthy agents have zero effective weight.
    pub fn effective_weight(&self) -> u32 {
        if self.health_status == HealthStatus::Unhealthy {
            0
        } else {
            self.weight
        }
    }
}

/// Configuration for the load balancer.
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy to use.
    pub strategy: LoadBalanceStrategy,
    /// Maximum consecutive health check failures before removing agent.
    pub max_consecutive_failures: u32,
    /// Health check interval duration.
    pub health_check_interval: Duration,
    /// Whether to automatically remove unhealthy agents.
    pub auto_remove_unhealthy: bool,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalanceStrategy::RoundRobin,
            max_consecutive_failures: 3,
            health_check_interval: Duration::from_secs(30),
            auto_remove_unhealthy: true,
        }
    }
}

impl LoadBalancerConfig {
    /// Creates a new config with the specified strategy.
    pub fn with_strategy(mut self, strategy: LoadBalanceStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Sets the maximum consecutive failures before removal.
    pub fn with_max_failures(mut self, max_failures: u32) -> Self {
        self.max_consecutive_failures = max_failures.max(1);
        self
    }

    /// Sets the health check interval.
    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = interval;
        self
    }

    /// Sets whether to auto-remove unhealthy agents.
    pub fn with_auto_remove(mut self, auto_remove: bool) -> Self {
        self.auto_remove_unhealthy = auto_remove;
        self
    }
}

/// Load balancer for distributing tasks across agents.
///
/// The `LoadBalancer` provides multiple strategies for task distribution:
/// - Round-robin: Tasks are distributed evenly in circular order.
/// - Weighted: Agents with higher weights receive proportionally more tasks.
/// - Least-loaded: Prefers agents with fewer active tasks.
///
/// The load balancer also tracks agent health and automatically removes
/// unhealthy agents from the pool.
#[derive(Debug)]
pub struct LoadBalancer {
    /// Configuration for the load balancer.
    config: LoadBalancerConfig,
    /// Agent metadata indexed by agent ID.
    agents: RwLock<HashMap<AgentId, AgentMetadata>>,
    /// Round-robin counter for task distribution.
    round_robin_counter: AtomicUsize,
    /// Weighted distribution accumulator.
    weighted_accumulator: AtomicU64,
    /// Total weight of all healthy agents.
    total_weight: AtomicU32,
    /// List of agents that have been removed due to health failures.
    removed_agents: Mutex<Vec<AgentId>>,
}

impl LoadBalancer {
    /// Creates a new load balancer with the given configuration.
    pub fn new(config: LoadBalancerConfig) -> Self {
        Self {
            config,
            agents: RwLock::new(HashMap::new()),
            round_robin_counter: AtomicUsize::new(0),
            weighted_accumulator: AtomicU64::new(0),
            total_weight: AtomicU32::new(0),
            removed_agents: Mutex::new(Vec::new()),
        }
    }

    /// Creates a new load balancer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(LoadBalancerConfig::default())
    }

    /// Registers an agent with the load balancer.
    pub fn register_agent(&self, agent_id: AgentId) {
        self.register_agent_with_weight(agent_id, 1);
    }

    /// Registers an agent with a specific weight.
    pub fn register_agent_with_weight(&self, agent_id: AgentId, weight: u32) {
        let metadata = AgentMetadata::new(agent_id).with_weight(weight);
        let effective_weight = metadata.effective_weight();

        let mut agents = self.agents.write().unwrap();
        if agents.insert(agent_id, metadata).is_none() {
            // Only add weight if this is a new agent
            self.total_weight.fetch_add(effective_weight, Ordering::SeqCst);
        }
    }

    /// Unregisters an agent from the load balancer.
    pub fn unregister_agent(&self, agent_id: AgentId) {
        let mut agents = self.agents.write().unwrap();
        if let Some(metadata) = agents.remove(&agent_id) {
            let weight = metadata.effective_weight();
            self.total_weight.fetch_sub(weight.min(self.total_weight.load(Ordering::SeqCst)), Ordering::SeqCst);
        }
    }

    /// Returns the number of registered agents.
    pub fn agent_count(&self) -> usize {
        self.agents.read().unwrap().len()
    }

    /// Returns the number of healthy agents available for work.
    pub fn healthy_agent_count(&self) -> usize {
        self.agents
            .read()
            .unwrap()
            .values()
            .filter(|m| m.is_available())
            .count()
    }

    /// Selects the next agent to receive a task based on the configured strategy.
    ///
    /// Returns `None` if no healthy agents are available.
    pub fn select_agent(&self) -> Option<AgentId> {
        let agents = self.agents.read().unwrap();
        let available: Vec<_> = agents.values().filter(|m| m.is_available()).collect();

        if available.is_empty() {
            return None;
        }

        match self.config.strategy {
            LoadBalanceStrategy::RoundRobin => self.select_round_robin(&available),
            LoadBalanceStrategy::Weighted => self.select_weighted(&available),
            LoadBalanceStrategy::LeastLoaded => self.select_least_loaded(&available),
        }
    }

    /// Selects an agent using round-robin strategy.
    fn select_round_robin(&self, available: &[&AgentMetadata]) -> Option<AgentId> {
        if available.is_empty() {
            return None;
        }

        let index = self.round_robin_counter.fetch_add(1, Ordering::SeqCst) % available.len();
        Some(available[index].agent_id)
    }

    /// Selects an agent using weighted strategy.
    fn select_weighted(&self, available: &[&AgentMetadata]) -> Option<AgentId> {
        if available.is_empty() {
            return None;
        }

        let total_weight: u64 = available.iter().map(|m| u64::from(m.effective_weight())).sum();
        if total_weight == 0 {
            // Fall back to round-robin if all weights are zero
            return self.select_round_robin(available);
        }

        // Use accumulator for smooth weighted distribution
        let target = self.weighted_accumulator.fetch_add(1, Ordering::SeqCst) % total_weight;

        let mut cumulative = 0u64;
        for metadata in available {
            cumulative += u64::from(metadata.effective_weight());
            if target < cumulative {
                return Some(metadata.agent_id);
            }
        }

        // Fallback to first available
        Some(available[0].agent_id)
    }

    /// Selects an agent using least-loaded strategy.
    fn select_least_loaded(&self, available: &[&AgentMetadata]) -> Option<AgentId> {
        available
            .iter()
            .min_by_key(|m| m.active_tasks)
            .map(|m| m.agent_id)
    }

    /// Records that a task was assigned to an agent.
    pub fn record_task_assigned(&self, agent_id: AgentId) {
        let mut agents = self.agents.write().unwrap();
        if let Some(metadata) = agents.get_mut(&agent_id) {
            metadata.record_task_assigned();
        }
    }

    /// Records that an agent completed a task successfully.
    pub fn record_task_completed(&self, agent_id: AgentId) {
        let mut agents = self.agents.write().unwrap();
        if let Some(metadata) = agents.get_mut(&agent_id) {
            metadata.record_task_completed();
        }
    }

    /// Records that an agent failed a task.
    pub fn record_task_failed(&self, agent_id: AgentId) {
        let mut agents = self.agents.write().unwrap();
        if let Some(metadata) = agents.get_mut(&agent_id) {
            metadata.record_task_failed();
        }
    }

    /// Performs a health check for an agent.
    ///
    /// The `healthy` parameter indicates whether the health check passed.
    /// If the agent exceeds the maximum consecutive failures and auto-remove
    /// is enabled, the agent will be removed from the pool.
    ///
    /// Returns `true` if the agent was removed due to health failures.
    pub fn health_check(&self, agent_id: AgentId, healthy: bool) -> bool {
        let should_remove = {
            let mut agents = self.agents.write().unwrap();
            if let Some(metadata) = agents.get_mut(&agent_id) {
                if healthy {
                    let was_unhealthy = metadata.health_status == HealthStatus::Unhealthy;
                    metadata.mark_healthy();
                    if was_unhealthy {
                        // Re-add weight when agent becomes healthy
                        self.total_weight.fetch_add(metadata.weight, Ordering::SeqCst);
                    }
                    false
                } else {
                    let was_healthy = metadata.health_status != HealthStatus::Unhealthy;
                    metadata.mark_unhealthy();
                    if was_healthy {
                        // Remove weight when agent becomes unhealthy
                        let weight = metadata.weight;
                        self.total_weight.fetch_sub(weight.min(self.total_weight.load(Ordering::SeqCst)), Ordering::SeqCst);
                    }
                    self.config.auto_remove_unhealthy
                        && metadata.consecutive_failures >= self.config.max_consecutive_failures
                }
            } else {
                false
            }
        };

        if should_remove {
            self.remove_unhealthy_agent(agent_id);
            true
        } else {
            false
        }
    }

    /// Removes an unhealthy agent from the pool.
    fn remove_unhealthy_agent(&self, agent_id: AgentId) {
        let mut agents = self.agents.write().unwrap();
        if agents.remove(&agent_id).is_some() {
            let mut removed = self.removed_agents.lock().unwrap();
            removed.push(agent_id);
        }
    }

    /// Returns a list of agents that have been removed due to health failures.
    pub fn get_removed_agents(&self) -> Vec<AgentId> {
        self.removed_agents.lock().unwrap().clone()
    }

    /// Clears the list of removed agents.
    pub fn clear_removed_agents(&self) {
        self.removed_agents.lock().unwrap().clear();
    }

    /// Returns the health status of an agent.
    pub fn get_agent_health(&self, agent_id: AgentId) -> Option<HealthStatus> {
        self.agents
            .read()
            .unwrap()
            .get(&agent_id)
            .map(|m| m.health_status)
    }

    /// Returns metadata for an agent.
    pub fn get_agent_metadata(&self, agent_id: AgentId) -> Option<AgentMetadata> {
        self.agents.read().unwrap().get(&agent_id).cloned()
    }

    /// Returns metadata for all registered agents.
    pub fn get_all_agents(&self) -> Vec<AgentMetadata> {
        self.agents.read().unwrap().values().cloned().collect()
    }

    /// Returns the current load balancing strategy.
    pub fn strategy(&self) -> LoadBalanceStrategy {
        self.config.strategy
    }

    /// Returns the configuration.
    pub fn config(&self) -> &LoadBalancerConfig {
        &self.config
    }

    /// Checks if any agents need health checks based on the configured interval.
    ///
    /// Returns a list of agent IDs that should be checked.
    pub fn agents_needing_health_check(&self) -> Vec<AgentId> {
        let now = Instant::now();
        let interval = self.config.health_check_interval;

        self.agents
            .read()
            .unwrap()
            .values()
            .filter(|m| {
                m.last_health_check
                    .map(|t| now.duration_since(t) >= interval)
                    .unwrap_or(true) // Check if never checked
            })
            .map(|m| m.agent_id)
            .collect()
    }

    /// Returns statistics about the load balancer.
    pub fn stats(&self) -> LoadBalancerStats {
        let agents = self.agents.read().unwrap();
        let total_agents = agents.len();
        let healthy_agents = agents.values().filter(|m| m.health_status == HealthStatus::Healthy).count();
        let unhealthy_agents = agents.values().filter(|m| m.health_status == HealthStatus::Unhealthy).count();
        let unknown_agents = agents.values().filter(|m| m.health_status == HealthStatus::Unknown).count();
        let total_tasks_completed: u64 = agents.values().map(|m| m.tasks_completed).sum();
        let total_tasks_failed: u64 = agents.values().map(|m| m.tasks_failed).sum();
        let total_active_tasks: u32 = agents.values().map(|m| m.active_tasks).sum();
        let removed_count = self.removed_agents.lock().unwrap().len();

        LoadBalancerStats {
            total_agents,
            healthy_agents,
            unhealthy_agents,
            unknown_agents,
            removed_agents: removed_count,
            total_tasks_completed,
            total_tasks_failed,
            total_active_tasks,
            total_weight: self.total_weight.load(Ordering::SeqCst),
            strategy: self.config.strategy,
        }
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Statistics about the load balancer state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadBalancerStats {
    /// Total number of registered agents.
    pub total_agents: usize,
    /// Number of healthy agents.
    pub healthy_agents: usize,
    /// Number of unhealthy agents.
    pub unhealthy_agents: usize,
    /// Number of agents with unknown health status.
    pub unknown_agents: usize,
    /// Number of agents removed due to health failures.
    pub removed_agents: usize,
    /// Total tasks completed across all agents.
    pub total_tasks_completed: u64,
    /// Total tasks failed across all agents.
    pub total_tasks_failed: u64,
    /// Total active tasks across all agents.
    pub total_active_tasks: u32,
    /// Total effective weight of all agents.
    pub total_weight: u32,
    /// Current load balancing strategy.
    pub strategy: LoadBalanceStrategy,
}

// ============================================================================
// Conflict Detection Implementation (MC-15)
// ============================================================================

/// Strategy for resolving conflicts between parallel agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictResolutionStrategy {
    /// Manual resolution required - user must intervene.
    #[default]
    Manual,
    /// Automatic resolution using the first agent's changes (oldest wins).
    FirstWins,
    /// Automatic resolution using the last agent's changes (newest wins).
    LastWins,
    /// Automatic resolution by merging non-overlapping changes.
    MergeNonOverlapping,
}

/// Severity level for detected conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConflictSeverity {
    /// Low severity: conflicts are in comments or whitespace.
    Low,
    /// Medium severity: conflicts in non-critical code sections.
    Medium,
    /// High severity: conflicts in critical code paths.
    High,
    /// Critical severity: conflicts that may break compilation or tests.
    Critical,
}

/// Represents a single file conflict between two agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConflict {
    /// Path to the conflicting file relative to repo root.
    pub file_path: String,
    /// Agent that made the first change.
    pub agent_a: AgentId,
    /// Agent that made the conflicting change.
    pub agent_b: AgentId,
    /// Lines changed by agent A.
    pub lines_a: Vec<LineChange>,
    /// Lines changed by agent B.
    pub lines_b: Vec<LineChange>,
    /// Severity of the conflict.
    pub severity: ConflictSeverity,
    /// Whether the conflict is a true overlap (same lines) or adjacent changes.
    pub is_overlapping: bool,
}

/// Represents a change to a specific line or range of lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineChange {
    /// Starting line number (1-based).
    pub start_line: usize,
    /// Ending line number (inclusive, 1-based).
    pub end_line: usize,
    /// Type of change.
    pub change_type: ChangeType,
    /// Content of the changed lines (for additions/modifications).
    pub content: Option<String>,
}

/// Type of change made to lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Lines were added.
    Added,
    /// Lines were deleted.
    Deleted,
    /// Lines were modified.
    Modified,
}

/// Aggregated conflict report for multiple agents and files.
#[derive(Debug, Clone)]
pub struct ConflictReport {
    /// Individual file conflicts.
    pub conflicts: Vec<FileConflict>,
    /// Total number of conflicting files.
    pub total_files: usize,
    /// Total number of conflicting lines.
    pub total_lines: usize,
    /// Highest severity among all conflicts.
    pub max_severity: ConflictSeverity,
    /// Resolution strategy to apply.
    pub resolution_strategy: ConflictResolutionStrategy,
    /// Whether automatic resolution is possible.
    pub can_auto_resolve: bool,
    /// Suggested resolution actions.
    pub suggestions: Vec<String>,
}

impl ConflictReport {
    /// Creates an empty conflict report.
    pub fn new() -> Self {
        Self {
            conflicts: Vec::new(),
            total_files: 0,
            total_lines: 0,
            max_severity: ConflictSeverity::Low,
            resolution_strategy: ConflictResolutionStrategy::Manual,
            can_auto_resolve: true,
            suggestions: Vec::new(),
        }
    }

    /// Adds a conflict to the report.
    pub fn add_conflict(&mut self, conflict: FileConflict) {
        if conflict.severity > self.max_severity {
            self.max_severity = conflict.severity;
        }
        if conflict.is_overlapping {
            self.can_auto_resolve = false;
        }
        self.total_lines += conflict.lines_a.len() + conflict.lines_b.len();
        self.conflicts.push(conflict);
        self.total_files = self.conflicts.iter().map(|c| &c.file_path).collect::<std::collections::HashSet<_>>().len();
    }

    /// Returns true if any conflicts were detected.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Returns the number of high or critical severity conflicts.
    pub fn critical_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|c| c.severity >= ConflictSeverity::High)
            .count()
    }

    /// Sets the resolution strategy.
    pub fn with_strategy(mut self, strategy: ConflictResolutionStrategy) -> Self {
        self.resolution_strategy = strategy;
        self
    }

    /// Adds a suggestion for conflict resolution.
    pub fn add_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }

    /// Generates a summary of the conflict report.
    pub fn summary(&self) -> String {
        if !self.has_conflicts() {
            return "No conflicts detected.".to_string();
        }

        let mut summary = format!(
            "Detected {} conflict(s) in {} file(s) ({} lines affected).\n",
            self.conflicts.len(),
            self.total_files,
            self.total_lines
        );
        summary.push_str(&format!("Max severity: {:?}\n", self.max_severity));
        summary.push_str(&format!(
            "Auto-resolve possible: {}\n",
            if self.can_auto_resolve { "yes" } else { "no" }
        ));

        if !self.suggestions.is_empty() {
            summary.push_str("\nSuggestions:\n");
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                summary.push_str(&format!("  {}. {}\n", i + 1, suggestion));
            }
        }

        summary
    }
}

impl Default for ConflictReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for conflict detection.
#[derive(Debug, Clone)]
pub struct ConflictDetectionConfig {
    /// Default resolution strategy.
    pub default_strategy: ConflictResolutionStrategy,
    /// Whether to auto-resolve when possible.
    pub auto_resolve: bool,
    /// Number of context lines around changes for conflict detection.
    pub context_lines: usize,
    /// File patterns to ignore when detecting conflicts.
    pub ignore_patterns: Vec<String>,
    /// Whether to treat adjacent (non-overlapping) changes as conflicts.
    pub strict_mode: bool,
}

impl Default for ConflictDetectionConfig {
    fn default() -> Self {
        Self {
            default_strategy: ConflictResolutionStrategy::Manual,
            auto_resolve: false,
            context_lines: 3,
            ignore_patterns: vec![
                "*.lock".to_string(),
                "*.log".to_string(),
                ".gitignore".to_string(),
            ],
            strict_mode: false,
        }
    }
}

impl ConflictDetectionConfig {
    /// Sets the default resolution strategy.
    pub fn with_strategy(mut self, strategy: ConflictResolutionStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    /// Enables or disables auto-resolution.
    pub fn with_auto_resolve(mut self, auto_resolve: bool) -> Self {
        self.auto_resolve = auto_resolve;
        self
    }

    /// Sets the number of context lines.
    pub fn with_context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    /// Adds patterns to ignore.
    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    /// Enables or disables strict mode.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }
}

/// Detects and analyzes conflicts between changes made by parallel agents.
///
/// The `ConflictDetector` compares changes from multiple agent worktrees
/// against a common base and identifies overlapping modifications.
#[derive(Debug)]
pub struct ConflictDetector {
    /// Configuration for conflict detection.
    config: ConflictDetectionConfig,
    /// Repository root path.
    repo_root: PathBuf,
    /// Base commit or branch to compare against.
    base_ref: String,
}

impl ConflictDetector {
    /// Creates a new conflict detector for the given repository.
    ///
    /// # Arguments
    /// * `repo_root` - Path to the git repository root
    /// * `base_ref` - Git reference (commit, branch, tag) to use as the base for comparison
    pub fn new(repo_root: PathBuf, base_ref: String) -> Self {
        Self {
            config: ConflictDetectionConfig::default(),
            repo_root,
            base_ref,
        }
    }

    /// Creates a conflict detector with custom configuration.
    pub fn with_config(mut self, config: ConflictDetectionConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns the repository root path.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &ConflictDetectionConfig {
        &self.config
    }

    /// Detects conflicts between changes from two worktrees.
    ///
    /// Compares the changes each worktree made relative to the base reference
    /// and identifies any overlapping or conflicting modifications.
    pub fn detect_conflicts(
        &self,
        worktree_a: &Path,
        agent_a: AgentId,
        worktree_b: &Path,
        agent_b: AgentId,
    ) -> Result<ConflictReport, ConflictDetectionError> {
        let changes_a = self.get_changes(worktree_a)?;
        let changes_b = self.get_changes(worktree_b)?;

        let mut report = ConflictReport::new();
        report.resolution_strategy = self.config.default_strategy;

        // Find files changed by both agents
        let files_a: HashSet<_> = changes_a.keys().collect();
        let files_b: HashSet<_> = changes_b.keys().collect();
        let common_files: Vec<_> = files_a.intersection(&files_b).collect();

        for file_path in common_files {
            if self.should_ignore(file_path) {
                continue;
            }

            let lines_a = changes_a.get(*file_path).unwrap();
            let lines_b = changes_b.get(*file_path).unwrap();

            if let Some(conflict) = self.analyze_file_conflict(
                file_path,
                agent_a,
                agent_b,
                lines_a,
                lines_b,
            ) {
                report.add_conflict(conflict);
            }
        }

        // Add suggestions based on conflict analysis
        self.add_resolution_suggestions(&mut report);

        Ok(report)
    }

    /// Detects conflicts across multiple worktrees.
    ///
    /// Performs pairwise comparison of all worktrees and aggregates conflicts.
    pub fn detect_conflicts_multi(
        &self,
        worktrees: &[(AgentId, &Path)],
    ) -> Result<ConflictReport, ConflictDetectionError> {
        let mut report = ConflictReport::new();
        report.resolution_strategy = self.config.default_strategy;

        // Compare each pair of worktrees
        for i in 0..worktrees.len() {
            for j in (i + 1)..worktrees.len() {
                let (agent_a, path_a) = worktrees[i];
                let (agent_b, path_b) = worktrees[j];

                let pair_report = self.detect_conflicts(path_a, agent_a, path_b, agent_b)?;
                for conflict in pair_report.conflicts {
                    report.add_conflict(conflict);
                }
            }
        }

        self.add_resolution_suggestions(&mut report);
        Ok(report)
    }

    /// Gets the list of changed files and their modified lines in a worktree.
    fn get_changes(&self, worktree: &Path) -> Result<HashMap<String, Vec<LineChange>>, ConflictDetectionError> {
        // Get the diff between base and worktree HEAD
        let diff_output = self.git_diff(worktree, &self.base_ref, "HEAD")?;
        self.parse_diff_output(&diff_output)
    }

    /// Runs git diff between two refs in a worktree.
    fn git_diff(&self, worktree: &Path, from: &str, to: &str) -> Result<String, ConflictDetectionError> {
        let output = ProcCommand::new("git")
            .arg("-C")
            .arg(worktree)
            .args(["diff", "--unified=0", from, to])
            .output()
            .map_err(|e| ConflictDetectionError::GitError(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ConflictDetectionError::GitError(stderr.to_string()))
        }
    }

    /// Parses git diff output into a map of file paths to line changes.
    fn parse_diff_output(&self, diff: &str) -> Result<HashMap<String, Vec<LineChange>>, ConflictDetectionError> {
        let mut changes: HashMap<String, Vec<LineChange>> = HashMap::new();
        let mut current_file: Option<String> = None;

        for line in diff.lines() {
            // Parse file headers: "diff --git a/path b/path" or "+++ b/path"
            if let Some(rest) = line.strip_prefix("+++ b/") {
                current_file = Some(rest.to_string());
                if !changes.contains_key(rest) {
                    changes.insert(rest.to_string(), Vec::new());
                }
                continue;
            }

            // Parse hunk headers: "@@ -start,count +start,count @@"
            if line.starts_with("@@") {
                if let Some(file) = &current_file {
                    if let Some(line_change) = self.parse_hunk_header(line) {
                        changes.get_mut(file).unwrap().push(line_change);
                    }
                }
            }
        }

        Ok(changes)
    }

    /// Parses a git diff hunk header into a LineChange.
    fn parse_hunk_header(&self, header: &str) -> Option<LineChange> {
        // Format: @@ -old_start,old_count +new_start,new_count @@
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let new_range = parts[2]; // +start,count or +start
        let new_range = new_range.strip_prefix('+')?;

        let (start, count) = if let Some(idx) = new_range.find(',') {
            let start: usize = new_range[..idx].parse().ok()?;
            let count: usize = new_range[idx + 1..].parse().ok()?;
            (start, count)
        } else {
            let start: usize = new_range.parse().ok()?;
            (start, 1)
        };

        if count == 0 {
            return None; // Pure deletion
        }

        let change_type = if parts[1].contains(",0") {
            ChangeType::Added
        } else {
            ChangeType::Modified
        };

        Some(LineChange {
            start_line: start.max(1),
            end_line: (start + count).saturating_sub(1).max(1),
            change_type,
            content: None,
        })
    }

    /// Analyzes whether changes to a file from two agents conflict.
    fn analyze_file_conflict(
        &self,
        file_path: &str,
        agent_a: AgentId,
        agent_b: AgentId,
        lines_a: &[LineChange],
        lines_b: &[LineChange],
    ) -> Option<FileConflict> {
        let mut overlapping = false;
        let context = self.config.context_lines;

        // Check for overlapping line ranges
        for change_a in lines_a {
            for change_b in lines_b {
                let range_a = (
                    change_a.start_line.saturating_sub(context),
                    change_a.end_line + context,
                );
                let range_b = (
                    change_b.start_line.saturating_sub(context),
                    change_b.end_line + context,
                );

                // Check if ranges overlap
                if range_a.0 <= range_b.1 && range_b.0 <= range_a.1 {
                    // True overlap (same lines) or adjacent (within context)
                    let true_overlap = change_a.start_line <= change_b.end_line
                        && change_b.start_line <= change_a.end_line;

                    if true_overlap || self.config.strict_mode {
                        overlapping = true;
                        break;
                    }
                }
            }
            if overlapping {
                break;
            }
        }

        // If no overlap and not in strict mode, no conflict
        if !overlapping && !self.config.strict_mode {
            return None;
        }

        // Determine severity based on file type and change extent
        let severity = self.assess_severity(file_path, lines_a, lines_b);

        Some(FileConflict {
            file_path: file_path.to_string(),
            agent_a,
            agent_b,
            lines_a: lines_a.to_vec(),
            lines_b: lines_b.to_vec(),
            severity,
            is_overlapping: overlapping,
        })
    }

    /// Assesses the severity of a conflict based on file type and extent.
    fn assess_severity(
        &self,
        file_path: &str,
        lines_a: &[LineChange],
        lines_b: &[LineChange],
    ) -> ConflictSeverity {
        // Critical files
        let critical_patterns = ["Cargo.toml", "Cargo.lock", "package.json", "go.mod", "pyproject.toml"];
        if critical_patterns.iter().any(|p| file_path.ends_with(p)) {
            return ConflictSeverity::Critical;
        }

        // Configuration files
        let config_patterns = [".yml", ".yaml", ".json", ".toml", ".ini", ".env"];
        if config_patterns.iter().any(|p| file_path.ends_with(p)) {
            return ConflictSeverity::High;
        }

        // Assess based on number of conflicting lines
        let total_lines = lines_a.len() + lines_b.len();
        if total_lines > 20 {
            return ConflictSeverity::High;
        }
        if total_lines > 5 {
            return ConflictSeverity::Medium;
        }

        ConflictSeverity::Low
    }

    /// Checks if a file should be ignored based on configured patterns.
    ///
    /// Supports glob-like patterns:
    /// - `*.ext` matches any file ending with `.ext`
    /// - `*-lock*` matches files containing `-lock` (e.g., `package-lock.json`)
    /// - Exact matches for filenames
    fn should_ignore(&self, file_path: &str) -> bool {
        let filename = file_path.rsplit('/').next().unwrap_or(file_path);

        for pattern in &self.config.ignore_patterns {
            if pattern.starts_with('*') && pattern.ends_with('*') {
                // Pattern like *-lock* - check if filename contains the middle part
                let middle = &pattern[1..pattern.len() - 1];
                if filename.contains(middle) {
                    return true;
                }
            } else if pattern.starts_with('*') {
                // Pattern like *.ext - check suffix
                let suffix = &pattern[1..];
                if filename.ends_with(suffix) {
                    return true;
                }
            } else if file_path == pattern || filename == pattern {
                return true;
            }
        }
        false
    }

    /// Adds resolution suggestions to a conflict report.
    fn add_resolution_suggestions(&self, report: &mut ConflictReport) {
        if !report.has_conflicts() {
            return;
        }

        if report.can_auto_resolve && self.config.auto_resolve {
            report.add_suggestion(format!(
                "Conflicts can be auto-resolved using {:?} strategy.",
                self.config.default_strategy
            ));
        }

        if report.max_severity >= ConflictSeverity::High {
            report.add_suggestion("High severity conflicts detected. Manual review recommended.".to_string());
        }

        if report.critical_count() > 0 {
            report.add_suggestion(format!(
                "{} critical conflict(s) require immediate attention.",
                report.critical_count()
            ));
        }

        // Suggest based on overlap patterns
        let overlapping_count = report.conflicts.iter().filter(|c| c.is_overlapping).count();
        if overlapping_count == 0 && report.has_conflicts() {
            report.add_suggestion(
                "All conflicts are in adjacent (non-overlapping) regions. \
                Consider using MergeNonOverlapping strategy.".to_string()
            );
        }
    }

    /// Attempts to auto-resolve conflicts using the configured strategy.
    ///
    /// Returns the resolved content for each conflicting file, or an error
    /// if auto-resolution is not possible.
    pub fn resolve(
        &self,
        report: &ConflictReport,
        worktree_a: &Path,
        worktree_b: &Path,
    ) -> Result<HashMap<String, String>, ConflictDetectionError> {
        if !report.can_auto_resolve && report.resolution_strategy != ConflictResolutionStrategy::Manual {
            return Err(ConflictDetectionError::CannotAutoResolve);
        }

        match report.resolution_strategy {
            ConflictResolutionStrategy::Manual => {
                Err(ConflictDetectionError::ManualResolutionRequired)
            }
            ConflictResolutionStrategy::FirstWins => {
                self.resolve_with_preference(report, worktree_a)
            }
            ConflictResolutionStrategy::LastWins => {
                self.resolve_with_preference(report, worktree_b)
            }
            ConflictResolutionStrategy::MergeNonOverlapping => {
                self.resolve_merge_non_overlapping(report, worktree_a, worktree_b)
            }
        }
    }

    /// Resolves by taking all content from the preferred worktree.
    fn resolve_with_preference(
        &self,
        report: &ConflictReport,
        preferred: &Path,
    ) -> Result<HashMap<String, String>, ConflictDetectionError> {
        let mut resolved = HashMap::new();

        for conflict in &report.conflicts {
            let file_path = preferred.join(&conflict.file_path);
            let content = fs::read_to_string(&file_path)
                .map_err(|e| ConflictDetectionError::IoError(e.to_string()))?;
            resolved.insert(conflict.file_path.clone(), content);
        }

        Ok(resolved)
    }

    /// Resolves by merging non-overlapping changes from both worktrees.
    fn resolve_merge_non_overlapping(
        &self,
        report: &ConflictReport,
        _worktree_a: &Path,
        _worktree_b: &Path,
    ) -> Result<HashMap<String, String>, ConflictDetectionError> {
        // Check if merge is possible
        let has_overlapping = report.conflicts.iter().any(|c| c.is_overlapping);
        if has_overlapping {
            return Err(ConflictDetectionError::CannotAutoResolve);
        }

        // For now, return an error indicating this strategy requires implementation
        // In a full implementation, this would:
        // 1. Read base file content
        // 2. Apply non-overlapping changes from both worktrees
        // 3. Return merged content
        Err(ConflictDetectionError::CannotAutoResolve)
    }
}

/// Errors that can occur during conflict detection.
#[derive(Debug)]
pub enum ConflictDetectionError {
    /// Git command failed.
    GitError(String),
    /// I/O error reading files.
    IoError(String),
    /// Conflicts cannot be automatically resolved.
    CannotAutoResolve,
    /// Manual resolution is required.
    ManualResolutionRequired,
    /// Invalid worktree path.
    InvalidWorktree(String),
}

impl fmt::Display for ConflictDetectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictDetectionError::GitError(msg) => write!(f, "git error: {}", msg),
            ConflictDetectionError::IoError(msg) => write!(f, "i/o error: {}", msg),
            ConflictDetectionError::CannotAutoResolve => {
                write!(f, "conflicts cannot be automatically resolved")
            }
            ConflictDetectionError::ManualResolutionRequired => {
                write!(f, "manual resolution is required")
            }
            ConflictDetectionError::InvalidWorktree(path) => {
                write!(f, "invalid worktree path: {}", path)
            }
        }
    }
}

impl Error for ConflictDetectionError {}

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

    // ========================================================================
    // Load Balancer Tests (MC-14)
    // ========================================================================

    #[test]
    fn load_balancer_round_robin_distributes_evenly() {
        let lb = LoadBalancer::new(
            LoadBalancerConfig::default().with_strategy(LoadBalanceStrategy::RoundRobin),
        );

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));
        lb.register_agent(AgentId(2));

        // Mark all agents as healthy
        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), true);
        lb.health_check(AgentId(2), true);

        let mut counts = HashMap::new();
        for _ in 0..30 {
            if let Some(id) = lb.select_agent() {
                *counts.entry(id.0).or_insert(0) += 1;
            }
        }

        // Each agent should get roughly equal tasks (10 each for 30 tasks)
        assert_eq!(counts.get(&0), Some(&10));
        assert_eq!(counts.get(&1), Some(&10));
        assert_eq!(counts.get(&2), Some(&10));
    }

    #[test]
    fn load_balancer_weighted_respects_weights() {
        let lb = LoadBalancer::new(
            LoadBalancerConfig::default().with_strategy(LoadBalanceStrategy::Weighted),
        );

        lb.register_agent_with_weight(AgentId(0), 1);
        lb.register_agent_with_weight(AgentId(1), 2);
        lb.register_agent_with_weight(AgentId(2), 3);

        // Mark all agents as healthy
        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), true);
        lb.health_check(AgentId(2), true);

        let mut counts = HashMap::new();
        for _ in 0..60 {
            if let Some(id) = lb.select_agent() {
                *counts.entry(id.0).or_insert(0) += 1;
            }
        }

        // Agent 0 (weight 1): ~10 tasks
        // Agent 1 (weight 2): ~20 tasks
        // Agent 2 (weight 3): ~30 tasks
        assert_eq!(counts.get(&0), Some(&10));
        assert_eq!(counts.get(&1), Some(&20));
        assert_eq!(counts.get(&2), Some(&30));
    }

    #[test]
    fn load_balancer_least_loaded_prefers_idle_agents() {
        let lb = LoadBalancer::new(
            LoadBalancerConfig::default().with_strategy(LoadBalanceStrategy::LeastLoaded),
        );

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));
        lb.register_agent(AgentId(2));

        // Mark all agents as healthy
        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), true);
        lb.health_check(AgentId(2), true);

        // Assign tasks to agents 0 and 1
        lb.record_task_assigned(AgentId(0));
        lb.record_task_assigned(AgentId(0));
        lb.record_task_assigned(AgentId(1));

        // Agent 2 should be selected (least loaded - 0 active tasks)
        let selected = lb.select_agent();
        assert_eq!(selected, Some(AgentId(2)));
    }

    #[test]
    fn load_balancer_health_check_marks_agent_healthy() {
        let lb = LoadBalancer::with_defaults();
        lb.register_agent(AgentId(0));

        assert_eq!(lb.get_agent_health(AgentId(0)), Some(HealthStatus::Unknown));

        lb.health_check(AgentId(0), true);
        assert_eq!(lb.get_agent_health(AgentId(0)), Some(HealthStatus::Healthy));
    }

    #[test]
    fn load_balancer_health_check_marks_agent_unhealthy() {
        let lb = LoadBalancer::with_defaults();
        lb.register_agent(AgentId(0));

        lb.health_check(AgentId(0), false);
        assert_eq!(lb.get_agent_health(AgentId(0)), Some(HealthStatus::Unhealthy));
    }

    #[test]
    fn load_balancer_removes_unhealthy_agent_after_max_failures() {
        let config = LoadBalancerConfig::default()
            .with_max_failures(3)
            .with_auto_remove(true);
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));

        // Fail agent 0 three times
        assert!(!lb.health_check(AgentId(0), false)); // 1st failure
        assert!(!lb.health_check(AgentId(0), false)); // 2nd failure
        assert!(lb.health_check(AgentId(0), false));  // 3rd failure - removed

        // Agent 0 should be removed
        assert_eq!(lb.agent_count(), 1);
        assert_eq!(lb.get_removed_agents(), vec![AgentId(0)]);

        // Only agent 1 should be selectable
        lb.health_check(AgentId(1), true);
        assert_eq!(lb.select_agent(), Some(AgentId(1)));
    }

    #[test]
    fn load_balancer_unhealthy_agents_not_selected() {
        let config = LoadBalancerConfig::default().with_auto_remove(false);
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));

        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), false); // Mark agent 1 as unhealthy

        // Only agent 0 should be selected
        for _ in 0..10 {
            assert_eq!(lb.select_agent(), Some(AgentId(0)));
        }
    }

    #[test]
    fn load_balancer_no_agents_returns_none() {
        let lb = LoadBalancer::with_defaults();
        assert!(lb.select_agent().is_none());
    }

    #[test]
    fn load_balancer_all_unhealthy_returns_none() {
        let config = LoadBalancerConfig::default().with_auto_remove(false);
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));

        lb.health_check(AgentId(0), false);
        lb.health_check(AgentId(1), false);

        assert!(lb.select_agent().is_none());
    }

    #[test]
    fn load_balancer_tracks_task_metrics() {
        let lb = LoadBalancer::with_defaults();
        lb.register_agent(AgentId(0));
        lb.health_check(AgentId(0), true);

        lb.record_task_assigned(AgentId(0));
        lb.record_task_assigned(AgentId(0));
        lb.record_task_completed(AgentId(0));
        lb.record_task_failed(AgentId(0));

        let metadata = lb.get_agent_metadata(AgentId(0)).unwrap();
        assert_eq!(metadata.tasks_completed, 1);
        assert_eq!(metadata.tasks_failed, 1);
        assert_eq!(metadata.active_tasks, 0);
    }

    #[test]
    fn load_balancer_stats_reflect_state() {
        let lb = LoadBalancer::new(
            LoadBalancerConfig::default().with_strategy(LoadBalanceStrategy::Weighted),
        );

        lb.register_agent_with_weight(AgentId(0), 2);
        lb.register_agent_with_weight(AgentId(1), 3);

        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), false);

        let stats = lb.stats();
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.healthy_agents, 1);
        assert_eq!(stats.unhealthy_agents, 1);
        assert_eq!(stats.strategy, LoadBalanceStrategy::Weighted);
    }

    #[test]
    fn load_balancer_unregister_agent_removes_weight() {
        let lb = LoadBalancer::new(
            LoadBalancerConfig::default().with_strategy(LoadBalanceStrategy::Weighted),
        );

        lb.register_agent_with_weight(AgentId(0), 5);
        lb.register_agent_with_weight(AgentId(1), 3);
        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), true);

        assert_eq!(lb.agent_count(), 2);

        lb.unregister_agent(AgentId(0));
        assert_eq!(lb.agent_count(), 1);
        assert_eq!(lb.select_agent(), Some(AgentId(1)));
    }

    #[test]
    fn load_balancer_recovery_from_unhealthy() {
        let config = LoadBalancerConfig::default().with_auto_remove(false);
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.health_check(AgentId(0), false);
        assert_eq!(lb.get_agent_health(AgentId(0)), Some(HealthStatus::Unhealthy));
        assert!(lb.select_agent().is_none());

        // Agent recovers
        lb.health_check(AgentId(0), true);
        assert_eq!(lb.get_agent_health(AgentId(0)), Some(HealthStatus::Healthy));
        assert_eq!(lb.select_agent(), Some(AgentId(0)));
    }

    #[test]
    fn load_balancer_config_builder() {
        let config = LoadBalancerConfig::default()
            .with_strategy(LoadBalanceStrategy::LeastLoaded)
            .with_max_failures(5)
            .with_health_check_interval(Duration::from_secs(60))
            .with_auto_remove(false);

        assert_eq!(config.strategy, LoadBalanceStrategy::LeastLoaded);
        assert_eq!(config.max_consecutive_failures, 5);
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
        assert!(!config.auto_remove_unhealthy);
    }

    #[test]
    fn agent_metadata_effective_weight_zero_when_unhealthy() {
        let mut metadata = AgentMetadata::new(AgentId(0)).with_weight(10);
        assert_eq!(metadata.effective_weight(), 10);

        metadata.mark_unhealthy();
        assert_eq!(metadata.effective_weight(), 0);

        metadata.mark_healthy();
        assert_eq!(metadata.effective_weight(), 10);
    }

    #[test]
    fn agent_metadata_tracks_consecutive_failures() {
        let mut metadata = AgentMetadata::new(AgentId(0));

        metadata.mark_unhealthy();
        assert_eq!(metadata.consecutive_failures, 1);

        metadata.mark_unhealthy();
        assert_eq!(metadata.consecutive_failures, 2);

        metadata.mark_healthy();
        assert_eq!(metadata.consecutive_failures, 0);
    }

    #[test]
    fn load_balancer_agents_needing_health_check() {
        let config = LoadBalancerConfig::default()
            .with_health_check_interval(Duration::from_millis(10));
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));

        // Initially, all agents need health check (never checked)
        let needing_check = lb.agents_needing_health_check();
        assert_eq!(needing_check.len(), 2);

        // After checking agent 0, only agent 1 needs check
        lb.health_check(AgentId(0), true);
        std::thread::sleep(Duration::from_millis(15));

        // Now agent 0 should need a check again
        let needing_check = lb.agents_needing_health_check();
        assert!(needing_check.contains(&AgentId(0)));
    }

    #[test]
    fn load_balancer_clear_removed_agents() {
        let config = LoadBalancerConfig::default()
            .with_max_failures(1)
            .with_auto_remove(true);
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.health_check(AgentId(0), false);

        assert_eq!(lb.get_removed_agents().len(), 1);

        lb.clear_removed_agents();
        assert!(lb.get_removed_agents().is_empty());
    }

    #[test]
    fn load_balancer_get_all_agents() {
        let lb = LoadBalancer::with_defaults();

        lb.register_agent(AgentId(0));
        lb.register_agent_with_weight(AgentId(1), 5);

        let agents = lb.get_all_agents();
        assert_eq!(agents.len(), 2);

        let agent0 = agents.iter().find(|m| m.agent_id == AgentId(0)).unwrap();
        let agent1 = agents.iter().find(|m| m.agent_id == AgentId(1)).unwrap();

        assert_eq!(agent0.weight, 1);
        assert_eq!(agent1.weight, 5);
    }

    #[test]
    fn load_balancer_default_creates_with_defaults() {
        let lb = LoadBalancer::default();
        assert_eq!(lb.strategy(), LoadBalanceStrategy::RoundRobin);
        assert_eq!(lb.agent_count(), 0);
    }

    #[test]
    fn load_balance_strategy_default() {
        let strategy = LoadBalanceStrategy::default();
        assert_eq!(strategy, LoadBalanceStrategy::RoundRobin);
    }

    #[test]
    fn load_balancer_healthy_agent_count() {
        let config = LoadBalancerConfig::default().with_auto_remove(false);
        let lb = LoadBalancer::new(config);

        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));
        lb.register_agent(AgentId(2));

        // All unknown = all available (conservative approach)
        assert_eq!(lb.healthy_agent_count(), 3);

        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), false);
        // Agent 2 still unknown

        // Healthy + Unknown = available
        assert_eq!(lb.healthy_agent_count(), 2);
    }

    #[test]
    fn load_balancer_integration_round_robin_with_health_checks() {
        let config = LoadBalancerConfig::default()
            .with_strategy(LoadBalanceStrategy::RoundRobin)
            .with_max_failures(2)
            .with_auto_remove(true);
        let lb = LoadBalancer::new(config);

        // Register 3 agents
        lb.register_agent(AgentId(0));
        lb.register_agent(AgentId(1));
        lb.register_agent(AgentId(2));

        // Mark all healthy
        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), true);
        lb.health_check(AgentId(2), true);

        // Simulate task distribution
        for _ in 0..6 {
            if let Some(agent_id) = lb.select_agent() {
                lb.record_task_assigned(agent_id);
            }
        }

        // Agent 1 starts failing health checks
        lb.health_check(AgentId(1), false);
        lb.health_check(AgentId(1), false); // Removed after 2 failures

        // Verify agent 1 was removed
        assert_eq!(lb.agent_count(), 2);
        assert!(lb.get_removed_agents().contains(&AgentId(1)));

        // Remaining agents should still be selectable
        let mut selected_ids = HashSet::new();
        for _ in 0..4 {
            if let Some(id) = lb.select_agent() {
                selected_ids.insert(id);
            }
        }
        assert!(selected_ids.contains(&AgentId(0)));
        assert!(selected_ids.contains(&AgentId(2)));
        assert!(!selected_ids.contains(&AgentId(1)));
    }

    #[test]
    fn load_balancer_integration_weighted_distribution() {
        let config = LoadBalancerConfig::default()
            .with_strategy(LoadBalanceStrategy::Weighted);
        let lb = LoadBalancer::new(config);

        // Agent 0: weight 1, Agent 1: weight 4 (4x more likely)
        lb.register_agent_with_weight(AgentId(0), 1);
        lb.register_agent_with_weight(AgentId(1), 4);

        lb.health_check(AgentId(0), true);
        lb.health_check(AgentId(1), true);

        let mut count_0 = 0;
        let mut count_1 = 0;

        for _ in 0..50 {
            match lb.select_agent() {
                Some(AgentId(0)) => count_0 += 1,
                Some(AgentId(1)) => count_1 += 1,
                _ => {}
            }
        }

        // Agent 1 should get ~4x more tasks than agent 0
        // With weights 1:4 and 50 tasks, expect ~10 for agent 0 and ~40 for agent 1
        assert_eq!(count_0, 10);
        assert_eq!(count_1, 40);
    }

    // =========================================================================
    // Conflict Detection Tests (MC-15)
    // =========================================================================

    #[test]
    fn conflict_report_new_is_empty() {
        let report = ConflictReport::new();
        assert!(!report.has_conflicts());
        assert_eq!(report.total_files, 0);
        assert_eq!(report.total_lines, 0);
        assert!(report.can_auto_resolve);
    }

    #[test]
    fn conflict_report_add_conflict_updates_stats() {
        let mut report = ConflictReport::new();

        let conflict = FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![LineChange {
                start_line: 10,
                end_line: 15,
                change_type: ChangeType::Modified,
                content: None,
            }],
            lines_b: vec![LineChange {
                start_line: 12,
                end_line: 18,
                change_type: ChangeType::Modified,
                content: None,
            }],
            severity: ConflictSeverity::Medium,
            is_overlapping: true,
        };

        report.add_conflict(conflict);

        assert!(report.has_conflicts());
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.total_files, 1);
        assert_eq!(report.total_lines, 2);
        assert!(!report.can_auto_resolve); // Overlapping conflicts can't be auto-resolved
    }

    #[test]
    fn conflict_report_tracks_max_severity() {
        let mut report = ConflictReport::new();

        let low_conflict = FileConflict {
            file_path: "README.md".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: false,
        };

        let high_conflict = FileConflict {
            file_path: "src/lib.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::High,
            is_overlapping: false,
        };

        report.add_conflict(low_conflict);
        assert_eq!(report.max_severity, ConflictSeverity::Low);

        report.add_conflict(high_conflict);
        assert_eq!(report.max_severity, ConflictSeverity::High);
    }

    #[test]
    fn conflict_report_critical_count() {
        let mut report = ConflictReport::new();

        // Add one high severity conflict
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::High,
            is_overlapping: false,
        });

        // Add one critical severity conflict
        report.add_conflict(FileConflict {
            file_path: "Cargo.toml".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Critical,
            is_overlapping: false,
        });

        // Add one low severity conflict
        report.add_conflict(FileConflict {
            file_path: "README.md".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: false,
        });

        assert_eq!(report.critical_count(), 2); // High + Critical
    }

    #[test]
    fn conflict_report_summary_no_conflicts() {
        let report = ConflictReport::new();
        let summary = report.summary();
        assert_eq!(summary, "No conflicts detected.");
    }

    #[test]
    fn conflict_report_summary_with_conflicts() {
        let mut report = ConflictReport::new();
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![LineChange {
                start_line: 10,
                end_line: 10,
                change_type: ChangeType::Modified,
                content: None,
            }],
            lines_b: vec![],
            severity: ConflictSeverity::Medium,
            is_overlapping: false,
        });

        let summary = report.summary();
        assert!(summary.contains("1 conflict(s)"));
        assert!(summary.contains("1 file(s)"));
        assert!(summary.contains("Medium"));
    }

    #[test]
    fn conflict_report_with_strategy() {
        let report = ConflictReport::new()
            .with_strategy(ConflictResolutionStrategy::FirstWins);
        assert_eq!(report.resolution_strategy, ConflictResolutionStrategy::FirstWins);
    }

    #[test]
    fn conflict_report_add_suggestion() {
        let mut report = ConflictReport::new();
        report.add_suggestion("Test suggestion".to_string());
        assert_eq!(report.suggestions.len(), 1);
        assert_eq!(report.suggestions[0], "Test suggestion");
    }

    #[test]
    fn conflict_detection_config_defaults() {
        let config = ConflictDetectionConfig::default();
        assert_eq!(config.default_strategy, ConflictResolutionStrategy::Manual);
        assert!(!config.auto_resolve);
        assert_eq!(config.context_lines, 3);
        assert!(!config.strict_mode);
        assert!(!config.ignore_patterns.is_empty());
    }

    #[test]
    fn conflict_detection_config_builder() {
        let config = ConflictDetectionConfig::default()
            .with_strategy(ConflictResolutionStrategy::LastWins)
            .with_auto_resolve(true)
            .with_context_lines(5)
            .with_strict_mode(true)
            .with_ignore_patterns(vec!["*.bak".to_string()]);

        assert_eq!(config.default_strategy, ConflictResolutionStrategy::LastWins);
        assert!(config.auto_resolve);
        assert_eq!(config.context_lines, 5);
        assert!(config.strict_mode);
        assert_eq!(config.ignore_patterns, vec!["*.bak".to_string()]);
    }

    #[test]
    fn conflict_resolution_strategy_default() {
        let strategy = ConflictResolutionStrategy::default();
        assert_eq!(strategy, ConflictResolutionStrategy::Manual);
    }

    #[test]
    fn conflict_severity_ordering() {
        assert!(ConflictSeverity::Low < ConflictSeverity::Medium);
        assert!(ConflictSeverity::Medium < ConflictSeverity::High);
        assert!(ConflictSeverity::High < ConflictSeverity::Critical);
    }

    #[test]
    fn line_change_types() {
        let added = LineChange {
            start_line: 1,
            end_line: 5,
            change_type: ChangeType::Added,
            content: Some("new content".to_string()),
        };

        let deleted = LineChange {
            start_line: 10,
            end_line: 15,
            change_type: ChangeType::Deleted,
            content: None,
        };

        let modified = LineChange {
            start_line: 20,
            end_line: 25,
            change_type: ChangeType::Modified,
            content: Some("modified content".to_string()),
        };

        assert_eq!(added.change_type, ChangeType::Added);
        assert_eq!(deleted.change_type, ChangeType::Deleted);
        assert_eq!(modified.change_type, ChangeType::Modified);
    }

    #[test]
    fn file_conflict_structure() {
        let conflict = FileConflict {
            file_path: "src/lib.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![LineChange {
                start_line: 10,
                end_line: 20,
                change_type: ChangeType::Modified,
                content: None,
            }],
            lines_b: vec![LineChange {
                start_line: 15,
                end_line: 25,
                change_type: ChangeType::Modified,
                content: None,
            }],
            severity: ConflictSeverity::High,
            is_overlapping: true,
        };

        assert_eq!(conflict.file_path, "src/lib.rs");
        assert_eq!(conflict.agent_a, AgentId(0));
        assert_eq!(conflict.agent_b, AgentId(1));
        assert!(conflict.is_overlapping);
    }

    #[test]
    fn conflict_detection_error_display() {
        let git_err = ConflictDetectionError::GitError("command failed".to_string());
        assert!(git_err.to_string().contains("git error"));

        let io_err = ConflictDetectionError::IoError("file not found".to_string());
        assert!(io_err.to_string().contains("i/o error"));

        let auto_err = ConflictDetectionError::CannotAutoResolve;
        assert!(auto_err.to_string().contains("cannot be automatically resolved"));

        let manual_err = ConflictDetectionError::ManualResolutionRequired;
        assert!(manual_err.to_string().contains("manual resolution"));

        let worktree_err = ConflictDetectionError::InvalidWorktree("/bad/path".to_string());
        assert!(worktree_err.to_string().contains("invalid worktree"));
    }

    #[test]
    fn conflict_detector_config() {
        let config = ConflictDetectionConfig::default()
            .with_strategy(ConflictResolutionStrategy::MergeNonOverlapping);

        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        ).with_config(config);

        assert_eq!(
            detector.config().default_strategy,
            ConflictResolutionStrategy::MergeNonOverlapping
        );
    }

    #[test]
    fn conflict_report_non_overlapping_can_auto_resolve() {
        let mut report = ConflictReport::new();

        // Add a non-overlapping conflict
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: false,
        });

        assert!(report.can_auto_resolve);
    }

    #[test]
    fn conflict_report_overlapping_cannot_auto_resolve() {
        let mut report = ConflictReport::new();

        // Add an overlapping conflict
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: true,
        });

        assert!(!report.can_auto_resolve);
    }

    #[test]
    fn conflict_report_tracks_unique_files() {
        let mut report = ConflictReport::new();

        // Add two conflicts in the same file
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: false,
        });

        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(2),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: false,
        });

        // Add one conflict in a different file
        report.add_conflict(FileConflict {
            file_path: "src/lib.rs".to_string(),
            agent_a: AgentId(1),
            agent_b: AgentId(2),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Low,
            is_overlapping: false,
        });

        assert_eq!(report.conflicts.len(), 3);
        assert_eq!(report.total_files, 2); // Only 2 unique files
    }

    #[test]
    fn conflict_report_default_is_new() {
        let report = ConflictReport::default();
        assert!(!report.has_conflicts());
        assert!(report.suggestions.is_empty());
    }

    #[test]
    fn conflict_report_summary_includes_suggestions() {
        let mut report = ConflictReport::new();
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::Medium,
            is_overlapping: false,
        });
        report.add_suggestion("First suggestion".to_string());
        report.add_suggestion("Second suggestion".to_string());

        let summary = report.summary();
        assert!(summary.contains("Suggestions:"));
        assert!(summary.contains("First suggestion"));
        assert!(summary.contains("Second suggestion"));
    }

    #[test]
    fn conflict_detector_parse_hunk_header_basic() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        // Test "@@ -10,5 +15,7 @@" format
        let result = detector.parse_hunk_header("@@ -10,5 +15,7 @@");
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.start_line, 15);
        assert_eq!(change.end_line, 21); // 15 + 7 - 1
    }

    #[test]
    fn conflict_detector_parse_hunk_header_single_line() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        // Test "@@ -10,1 +15 @@" format (single line addition)
        let result = detector.parse_hunk_header("@@ -10,1 +15 @@");
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.start_line, 15);
        assert_eq!(change.end_line, 15);
    }

    #[test]
    fn conflict_detector_parse_hunk_header_addition() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        // Test "@@ -10,0 +15,3 @@" format (pure addition)
        let result = detector.parse_hunk_header("@@ -10,0 +15,3 @@");
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.change_type, ChangeType::Added);
    }

    #[test]
    fn conflict_detector_parse_hunk_header_deletion() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        // Test "@@ -10,3 +10,0 @@" format (pure deletion)
        let result = detector.parse_hunk_header("@@ -10,3 +10,0 @@");
        assert!(result.is_none()); // Deletions have count 0, so None
    }

    #[test]
    fn conflict_detector_should_ignore_patterns() {
        let config = ConflictDetectionConfig::default()
            .with_ignore_patterns(vec![
                "*.lock".to_string(),
                "*-lock*".to_string(),
                ".gitignore".to_string(),
            ]);

        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        ).with_config(config);

        assert!(detector.should_ignore("Cargo.lock"));
        assert!(detector.should_ignore("package-lock.json"));
        assert!(detector.should_ignore(".gitignore"));
        assert!(!detector.should_ignore("src/main.rs"));
        assert!(!detector.should_ignore("README.md"));
    }

    #[test]
    fn conflict_detector_assess_severity_critical_files() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let severity = detector.assess_severity("Cargo.toml", &[], &[]);
        assert_eq!(severity, ConflictSeverity::Critical);

        let severity = detector.assess_severity("package.json", &[], &[]);
        assert_eq!(severity, ConflictSeverity::Critical);

        let severity = detector.assess_severity("go.mod", &[], &[]);
        assert_eq!(severity, ConflictSeverity::Critical);
    }

    #[test]
    fn conflict_detector_assess_severity_config_files() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let severity = detector.assess_severity("config.yml", &[], &[]);
        assert_eq!(severity, ConflictSeverity::High);

        let severity = detector.assess_severity("settings.json", &[], &[]);
        assert_eq!(severity, ConflictSeverity::High);
    }

    #[test]
    fn conflict_detector_assess_severity_by_line_count() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        // Few lines = Low severity
        let lines_a = vec![LineChange {
            start_line: 1,
            end_line: 1,
            change_type: ChangeType::Modified,
            content: None,
        }];
        let severity = detector.assess_severity("src/main.rs", &lines_a, &[]);
        assert_eq!(severity, ConflictSeverity::Low);

        // Many lines = High severity
        let lines_a: Vec<_> = (0..15).map(|i| LineChange {
            start_line: i + 1,
            end_line: i + 1,
            change_type: ChangeType::Modified,
            content: None,
        }).collect();
        let lines_b: Vec<_> = (0..10).map(|i| LineChange {
            start_line: i + 20,
            end_line: i + 20,
            change_type: ChangeType::Modified,
            content: None,
        }).collect();
        let severity = detector.assess_severity("src/main.rs", &lines_a, &lines_b);
        assert_eq!(severity, ConflictSeverity::High);
    }

    #[test]
    fn conflict_detector_analyze_file_conflict_overlapping() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let lines_a = vec![LineChange {
            start_line: 10,
            end_line: 20,
            change_type: ChangeType::Modified,
            content: None,
        }];

        let lines_b = vec![LineChange {
            start_line: 15,
            end_line: 25,
            change_type: ChangeType::Modified,
            content: None,
        }];

        let conflict = detector.analyze_file_conflict(
            "src/main.rs",
            AgentId(0),
            AgentId(1),
            &lines_a,
            &lines_b,
        );

        assert!(conflict.is_some());
        let conflict = conflict.unwrap();
        assert!(conflict.is_overlapping);
    }

    #[test]
    fn conflict_detector_analyze_file_conflict_non_overlapping() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let lines_a = vec![LineChange {
            start_line: 10,
            end_line: 15,
            change_type: ChangeType::Modified,
            content: None,
        }];

        let lines_b = vec![LineChange {
            start_line: 100,
            end_line: 110,
            change_type: ChangeType::Modified,
            content: None,
        }];

        // In non-strict mode, non-overlapping changes should not be conflicts
        let conflict = detector.analyze_file_conflict(
            "src/main.rs",
            AgentId(0),
            AgentId(1),
            &lines_a,
            &lines_b,
        );

        assert!(conflict.is_none());
    }

    #[test]
    fn conflict_detector_analyze_file_conflict_strict_mode() {
        let config = ConflictDetectionConfig::default()
            .with_strict_mode(true);

        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        ).with_config(config);

        let lines_a = vec![LineChange {
            start_line: 10,
            end_line: 15,
            change_type: ChangeType::Modified,
            content: None,
        }];

        let lines_b = vec![LineChange {
            start_line: 100,
            end_line: 110,
            change_type: ChangeType::Modified,
            content: None,
        }];

        // In strict mode, any changes to the same file are conflicts
        let conflict = detector.analyze_file_conflict(
            "src/main.rs",
            AgentId(0),
            AgentId(1),
            &lines_a,
            &lines_b,
        );

        assert!(conflict.is_some());
        assert!(!conflict.unwrap().is_overlapping);
    }

    #[test]
    fn conflict_detector_resolve_manual_returns_error() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let report = ConflictReport::new()
            .with_strategy(ConflictResolutionStrategy::Manual);

        let result = detector.resolve(
            &report,
            Path::new("/tmp/worktree_a"),
            Path::new("/tmp/worktree_b"),
        );

        assert!(matches!(result, Err(ConflictDetectionError::ManualResolutionRequired)));
    }

    #[test]
    fn conflict_detector_resolve_cannot_auto_resolve() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let mut report = ConflictReport::new()
            .with_strategy(ConflictResolutionStrategy::FirstWins);

        // Add an overlapping conflict (can't auto-resolve)
        report.add_conflict(FileConflict {
            file_path: "src/main.rs".to_string(),
            agent_a: AgentId(0),
            agent_b: AgentId(1),
            lines_a: vec![],
            lines_b: vec![],
            severity: ConflictSeverity::High,
            is_overlapping: true,
        });

        let result = detector.resolve(
            &report,
            Path::new("/tmp/worktree_a"),
            Path::new("/tmp/worktree_b"),
        );

        assert!(matches!(result, Err(ConflictDetectionError::CannotAutoResolve)));
    }

    #[test]
    fn conflict_detector_parse_diff_output_empty() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let result = detector.parse_diff_output("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn conflict_detector_parse_diff_output_single_file() {
        let detector = ConflictDetector::new(
            PathBuf::from("/tmp/repo"),
            "main".to_string(),
        );

        let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,0 +11,3 @@ fn main() {
+    let x = 1;
+    let y = 2;
+    let z = 3;
"#;

        let result = detector.parse_diff_output(diff);
        assert!(result.is_ok());
        let changes = result.unwrap();
        assert!(changes.contains_key("src/main.rs"));
        assert!(!changes.get("src/main.rs").unwrap().is_empty());
    }
}
