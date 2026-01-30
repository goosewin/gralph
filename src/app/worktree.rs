use super::{parse_bool_value, sanitize_session_name, CliError};
use crate::cli::{self, RunLoopArgs, WorktreeCommand, WorktreeCreateArgs, WorktreeFinishArgs};
use crate::config::Config;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;

#[derive(Default)]
pub(crate) struct Worktree;

impl Worktree {
    pub(crate) fn cmd_worktree(&self, args: cli::WorktreeArgs) -> Result<(), CliError> {
        match args.command {
            WorktreeCommand::Create(args) => cmd_worktree_create(args),
            WorktreeCommand::Finish(args) => cmd_worktree_finish(args),
        }
    }

    pub(crate) fn maybe_create_auto_worktree(
        &self,
        args: &mut RunLoopArgs,
        config: &Config,
    ) -> Result<(), CliError> {
        maybe_create_auto_worktree(args, config)
    }
}

fn cmd_worktree_create(args: WorktreeCreateArgs) -> Result<(), CliError> {
    validate_task_id(&args.id)?;
    let repo_root = git_output(["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string();
    if !git_has_commits(&repo_root) {
        return Err(CliError::Message(
            "Repository has no commits; cannot create worktree.".to_string(),
        ));
    }
    ensure_git_clean(&repo_root)?;

    let worktrees_dir = PathBuf::from(&repo_root).join(".worktrees");
    fs::create_dir_all(&worktrees_dir).map_err(CliError::Io)?;

    let branch = format!("task-{}", args.id);
    let worktree_path = worktrees_dir.join(&branch);
    create_worktree_at(&repo_root, &branch, &worktree_path)?;

    println!(
        "Created worktree {} on branch {}",
        worktree_path.display(),
        branch
    );
    Ok(())
}

fn cmd_worktree_finish(args: WorktreeFinishArgs) -> Result<(), CliError> {
    validate_task_id(&args.id)?;
    let repo_root = git_output(["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string();
    if !git_has_commits(&repo_root) {
        return Err(CliError::Message(
            "Repository has no commits; cannot finish worktree.".to_string(),
        ));
    }
    ensure_git_clean(&repo_root)?;

    let branch = format!("task-{}", args.id);
    let worktrees_dir = PathBuf::from(&repo_root).join(".worktrees");
    let worktree_path = worktrees_dir.join(&branch);

    if git_status_in_repo(
        &repo_root,
        [
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{}", branch),
        ],
    )
    .is_err()
    {
        return Err(CliError::Message(format!(
            "Branch does not exist: {}",
            branch
        )));
    }
    if !worktree_path.is_dir() {
        return Err(CliError::Message(format!(
            "Worktree path is missing: {}",
            worktree_path.display()
        )));
    }

    let current_branch = git_output(["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();
    if current_branch == branch {
        return Err(CliError::Message(format!(
            "Cannot finish while on branch {}",
            branch
        )));
    }

    git_status_in_repo(&repo_root, ["merge", "--no-ff", &branch])
        .map_err(|err| CliError::Message(format!("Failed to merge branch: {}", err)))?;
    git_status_in_repo(
        &repo_root,
        [
            "worktree",
            "remove",
            worktree_path.to_string_lossy().as_ref(),
        ],
    )
    .map_err(|err| CliError::Message(format!("Failed to remove worktree: {}", err)))?;

    println!(
        "Finished worktree {} and merged {}",
        worktree_path.display(),
        branch
    );
    Ok(())
}

pub(super) fn validate_task_id(id: &str) -> Result<(), CliError> {
    let mut parts = id.split('-');
    let prefix = parts.next().unwrap_or("");
    let number = parts.next().unwrap_or("");
    let valid = !prefix.is_empty()
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
        && prefix.chars().all(|c| c.is_ascii_alphabetic())
        && parts.next().is_none();
    if !valid {
        return Err(CliError::Message(format!(
            "Invalid task ID format: {} (expected like A-1)",
            id
        )));
    }
    Ok(())
}

fn git_output(args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Result<String, CliError> {
    let output = ProcCommand::new("git")
        .args(args)
        .output()
        .map_err(CliError::Io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(CliError::Message(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

pub(crate) fn git_output_in_dir(
    dir: &Path,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<String, CliError> {
    let output = ProcCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(CliError::Io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(CliError::Message(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

fn git_status_in_repo(
    repo_root: &str,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<(), CliError> {
    let output = ProcCommand::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(CliError::Io)?;
    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);
        let trimmed = combined.trim();
        if trimmed.is_empty() {
            Err(CliError::Message("git command failed".to_string()))
        } else {
            Err(CliError::Message(trimmed.to_string()))
        }
    }
}

fn git_is_clean(repo_root: &str) -> Result<bool, CliError> {
    let output = ProcCommand::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .map_err(CliError::Io)?;
    if !output.status.success() {
        return Err(CliError::Message("Unable to check git status".to_string()));
    }
    Ok(output.stdout.is_empty())
}

fn ensure_git_clean(repo_root: &str) -> Result<(), CliError> {
    if git_is_clean(repo_root)? {
        Ok(())
    } else {
        Err(CliError::Message(
            "Git working tree is dirty. Commit or stash changes before running worktree commands."
                .to_string(),
        ))
    }
}

pub(super) fn resolve_auto_worktree(config: &Config, no_worktree: bool) -> bool {
    if no_worktree {
        return false;
    }
    config
        .get("defaults.auto_worktree")
        .as_deref()
        .and_then(parse_bool_value)
        .unwrap_or(true)
}

pub(super) fn worktree_timestamp_slug() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub(super) fn auto_worktree_branch_name(session_name: &str, timestamp: &str) -> String {
    let sanitized = sanitize_session_name(session_name);
    if sanitized.is_empty() {
        format!("prd-{}", timestamp)
    } else {
        format!("prd-{}-{}", sanitized, timestamp)
    }
}

fn git_branch_exists(repo_root: &str, branch: &str) -> bool {
    git_status_in_repo(
        repo_root,
        [
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{}", branch),
        ],
    )
    .is_ok()
}

fn git_has_commits(repo_root: &str) -> bool {
    git_status_in_repo(repo_root, ["rev-parse", "--verify", "HEAD"]).is_ok()
}

pub(super) fn ensure_unique_worktree_branch(
    repo_root: &str,
    worktrees_dir: &Path,
    base: &str,
) -> String {
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while git_branch_exists(repo_root, &candidate) || worktrees_dir.join(&candidate).exists() {
        candidate = format!("{}-{}", base, suffix);
        suffix += 1;
    }
    candidate
}

pub(super) fn create_worktree_at(
    repo_root: &str,
    branch: &str,
    worktree_path: &Path,
) -> Result<(), CliError> {
    if git_branch_exists(repo_root, branch) {
        return Err(CliError::Message(format!(
            "Branch already exists: {}",
            branch
        )));
    }
    if worktree_path.exists() {
        return Err(CliError::Message(format!(
            "Worktree path already exists: {}",
            worktree_path.display()
        )));
    }

    git_status_in_repo(
        repo_root,
        [
            "worktree",
            "add",
            "-b",
            branch,
            worktree_path.to_string_lossy().as_ref(),
        ],
    )
    .map_err(|err| CliError::Message(format!("Failed to create worktree: {}", err)))?;
    Ok(())
}

pub(super) fn maybe_create_auto_worktree(
    args: &mut RunLoopArgs,
    config: &Config,
) -> Result<(), CliError> {
    let timestamp = worktree_timestamp_slug();
    maybe_create_auto_worktree_with_timestamp(args, config, &timestamp)
}

pub(super) fn maybe_create_auto_worktree_with_timestamp(
    args: &mut RunLoopArgs,
    config: &Config,
    timestamp: &str,
) -> Result<(), CliError> {
    if !resolve_auto_worktree(config, args.no_worktree) {
        return Ok(());
    }

    let target_dir = args.dir.clone();
    let target_display = target_dir.display();
    let repo_root = match git_output_in_dir(&target_dir, ["rev-parse", "--show-toplevel"]) {
        Ok(output) => output.trim().to_string(),
        Err(CliError::Message(message)) => {
            if message.to_lowercase().contains("not a git repository") {
                println!(
                    "Auto worktree skipped for {}: not a git repository.",
                    target_display
                );
                return Ok(());
            }
            return Err(CliError::Message(message));
        }
        Err(CliError::Io(err)) => {
            println!(
                "Auto worktree skipped for {}: git unavailable ({}).",
                target_display, err
            );
            return Ok(());
        }
    };
    if !git_has_commits(&repo_root) {
        println!(
            "Auto worktree skipped for {}: repository has no commits.",
            target_display
        );
        return Ok(());
    }
    let clean = match git_is_clean(&repo_root) {
        Ok(value) => value,
        Err(err) => {
            println!(
                "Auto worktree skipped for {}: unable to check git status ({}).",
                target_display, err
            );
            return Ok(());
        }
    };
    if !clean {
        println!(
            "Auto worktree skipped for {}: repository is dirty.",
            target_display
        );
        return Ok(());
    }

    let worktrees_dir = PathBuf::from(&repo_root).join(".worktrees");
    fs::create_dir_all(&worktrees_dir).map_err(CliError::Io)?;

    let target_dir = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.clone());
    let repo_root_path = PathBuf::from(&repo_root);
    let repo_root_path = repo_root_path
        .canonicalize()
        .unwrap_or_else(|_| repo_root_path.clone());
    let relative_target = target_dir
        .strip_prefix(&repo_root_path)
        .unwrap_or_else(|_| Path::new(""))
        .to_path_buf();

    let base_branch = auto_worktree_branch_name(&args.name, timestamp);
    let branch = ensure_unique_worktree_branch(&repo_root, &worktrees_dir, &base_branch);
    let worktree_path = worktrees_dir.join(&branch);

    create_worktree_at(&repo_root, &branch, &worktree_path)?;
    println!(
        "Auto worktree created: {} (branch {})",
        worktree_path.display(),
        branch
    );

    args.dir = if relative_target.as_os_str().is_empty() {
        worktree_path
    } else {
        worktree_path.join(relative_target)
    };
    args.no_worktree = true;
    Ok(())
}
