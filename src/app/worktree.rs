use super::{CliError, parse_bool_value, sanitize_session_name};
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
                print_auto_worktree_hint();
                return Ok(());
            }
            return Err(CliError::Message(message));
        }
        Err(CliError::Io(err)) => {
            println!(
                "Auto worktree skipped for {}: git unavailable ({}).",
                target_display, err
            );
            print_auto_worktree_hint();
            return Ok(());
        }
    };
    if !git_has_commits(&repo_root) {
        println!(
            "Auto worktree skipped for {}: repository has no commits.",
            target_display
        );
        print_auto_worktree_hint();
        return Ok(());
    }
    let clean = match git_is_clean(&repo_root) {
        Ok(value) => value,
        Err(err) => {
            println!(
                "Auto worktree skipped for {}: unable to check git status ({}).",
                target_display, err
            );
            print_auto_worktree_hint();
            return Ok(());
        }
    };
    if !clean {
        println!(
            "Auto worktree preparing for {}: repository is dirty; committing changes.",
            target_display
        );
        auto_commit_dirty_repo(&repo_root, timestamp)?;
        let clean = git_is_clean(&repo_root)?;
        if !clean {
            return Err(CliError::Message(
                "Auto worktree requires a clean repo after auto-commit.".to_string(),
            ));
        }
    }

    let worktree_root = PathBuf::from(&repo_root).join(".worktrees");
    fs::create_dir_all(&worktree_root).map_err(CliError::Io)?;

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
    let branch = ensure_unique_worktree_branch(&repo_root, &worktree_root, &base_branch);
    let worktree_path = worktree_root.join(&branch);

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

fn print_auto_worktree_hint() {
    println!(
        "Hint: use --no-worktree or set defaults.auto_worktree: false to disable auto worktrees."
    );
}

fn auto_commit_dirty_repo(repo_root: &str, timestamp: &str) -> Result<(), CliError> {
    git_status_in_repo(repo_root, ["add", "-A"]).map_err(|err| {
        CliError::Message(format!(
            "Failed to stage changes for auto worktree: {}",
            err
        ))
    })?;
    let message = format!("chore: auto-commit for worktree {}", timestamp);
    git_status_in_repo(repo_root, ["commit", "-m", message.as_str()]).map_err(|err| {
        CliError::Message(format!(
            "Failed to commit changes for auto worktree: {}",
            err
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    struct CurrentDirGuard {
        original: PathBuf,
    }

    impl CurrentDirGuard {
        fn set(path: &Path) -> Self {
            let original = env::current_dir().unwrap();
            env::set_current_dir(path).unwrap();
            Self { original }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

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

    fn init_repo(dir: &Path) {
        run_git(dir, &["init"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test User"]);
        fs::write(dir.join("README.md"), "init\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "-m", "init"]);
    }

    #[test]
    fn cmd_worktree_create_creates_branch_and_path() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        let _dir_guard = CurrentDirGuard::set(temp.path());

        cmd_worktree_create(WorktreeCreateArgs {
            id: "C-1".to_string(),
        })
        .unwrap();

        let worktree_path = temp.path().join(".worktrees").join("task-C-1");
        assert!(worktree_path.is_dir());
        run_git(
            temp.path(),
            &["show-ref", "--verify", "--quiet", "refs/heads/task-C-1"],
        );
    }

    #[test]
    fn cmd_worktree_create_rejects_dirty_repo() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        fs::write(temp.path().join("README.md"), "dirty\n").unwrap();
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_create(WorktreeCreateArgs {
            id: "C-2".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Git working tree is dirty"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_finish_reports_missing_branch() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_finish(WorktreeFinishArgs {
            id: "C-3".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Branch does not exist"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_finish_reports_missing_path() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        run_git(temp.path(), &["branch", "task-C-4"]);
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_finish(WorktreeFinishArgs {
            id: "C-4".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Worktree path is missing"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_finish_rejects_current_branch() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        run_git(temp.path(), &["checkout", "-b", "task-C-5"]);
        let worktree_path = temp.path().join(".worktrees").join("task-C-5");
        fs::create_dir_all(&worktree_path).unwrap();
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_finish(WorktreeFinishArgs {
            id: "C-5".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Cannot finish while on branch"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn git_output_in_dir_success() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());

        let result = git_output_in_dir(temp.path(), ["rev-parse", "--show-toplevel"]);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.trim().is_empty());
    }

    #[test]
    fn git_output_in_dir_error_on_invalid_dir() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        let nonexistent = temp.path().join("nonexistent");

        let result = git_output_in_dir(&nonexistent, ["status"]);
        assert!(result.is_err());
    }

    #[test]
    fn git_output_in_dir_error_on_non_repo() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        // Don't initialize as git repo

        let result = git_output_in_dir(temp.path(), ["rev-parse", "--show-toplevel"]);
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(
                    msg.to_lowercase().contains("not a git repository") || msg.contains("fatal"),
                    "Expected git error message, got: {}",
                    msg
                );
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn create_worktree_at_rejects_existing_branch() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        run_git(temp.path(), &["branch", "existing-branch"]);

        let worktree_path = temp.path().join(".worktrees").join("existing-branch");
        let result = create_worktree_at(
            temp.path().to_str().unwrap(),
            "existing-branch",
            &worktree_path,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(msg.contains("Branch already exists"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn create_worktree_at_rejects_existing_path() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());

        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();
        let worktree_path = worktrees_dir.join("new-branch");
        fs::create_dir_all(&worktree_path).unwrap();

        let result =
            create_worktree_at(temp.path().to_str().unwrap(), "new-branch", &worktree_path);
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(msg.contains("Worktree path already exists"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_create_rejects_repo_without_commits() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        // Initialize repo but don't commit
        run_git(temp.path(), &["init"]);
        run_git(temp.path(), &["config", "user.email", "test@example.com"]);
        run_git(temp.path(), &["config", "user.name", "Test User"]);
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_create(WorktreeCreateArgs {
            id: "C-6".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Repository has no commits"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_finish_rejects_repo_without_commits() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        // Initialize repo but don't commit
        run_git(temp.path(), &["init"]);
        run_git(temp.path(), &["config", "user.email", "test@example.com"]);
        run_git(temp.path(), &["config", "user.name", "Test User"]);
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_finish(WorktreeFinishArgs {
            id: "C-7".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Repository has no commits"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_finish_rejects_dirty_repo() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        run_git(temp.path(), &["branch", "task-C-8"]);
        let worktree_path = temp.path().join(".worktrees").join("task-C-8");
        fs::create_dir_all(&worktree_path).unwrap();
        // Make the repo dirty
        fs::write(temp.path().join("README.md"), "dirty\n").unwrap();
        let _dir_guard = CurrentDirGuard::set(temp.path());

        let err = cmd_worktree_finish(WorktreeFinishArgs {
            id: "C-8".to_string(),
        })
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Git working tree is dirty"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_worktree_finish_success() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        let _dir_guard = CurrentDirGuard::set(temp.path());

        // Create worktree manually using git directly
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();
        let worktree_path = worktrees_dir.join("task-C-9");
        run_git(
            temp.path(),
            &[
                "worktree",
                "add",
                "-b",
                "task-C-9",
                worktree_path.to_str().unwrap(),
            ],
        );

        // Add .worktrees to .gitignore so the repo stays clean
        fs::write(temp.path().join(".gitignore"), ".worktrees/\n").unwrap();
        run_git(temp.path(), &["add", ".gitignore"]);
        run_git(temp.path(), &["commit", "-m", "add gitignore"]);

        // Add a commit in the worktree
        assert!(worktree_path.is_dir());
        fs::write(worktree_path.join("new_file.txt"), "content\n").unwrap();
        run_git(&worktree_path, &["add", "."]);
        run_git(&worktree_path, &["commit", "-m", "add new file"]);

        // Finish the worktree (merges and removes)
        cmd_worktree_finish(WorktreeFinishArgs {
            id: "C-9".to_string(),
        })
        .unwrap();

        // Verify worktree is removed
        assert!(!worktree_path.exists());
        // Verify the new file exists in main repo after merge
        assert!(temp.path().join("new_file.txt").exists());
    }

    #[test]
    fn validate_task_id_rejects_empty_prefix() {
        let result = validate_task_id("-1");
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(msg.contains("Invalid task ID format"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn validate_task_id_rejects_empty_number() {
        let result = validate_task_id("ABC-");
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(msg.contains("Invalid task ID format"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn validate_task_id_rejects_non_numeric_suffix() {
        let result = validate_task_id("ABC-xyz");
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(msg.contains("Invalid task ID format"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn validate_task_id_rejects_extra_segments() {
        let result = validate_task_id("ABC-1-2");
        assert!(result.is_err());
        match result.unwrap_err() {
            CliError::Message(msg) => {
                assert!(msg.contains("Invalid task ID format"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn validate_task_id_accepts_valid_ids() {
        assert!(validate_task_id("A-1").is_ok());
        assert!(validate_task_id("ABC-123").is_ok());
        assert!(validate_task_id("COV-80").is_ok());
    }

    #[test]
    fn git_is_clean_detects_dirty_repo() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        fs::write(temp.path().join("README.md"), "dirty\n").unwrap();

        let result = git_is_clean(temp.path().to_str().unwrap());
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn git_is_clean_detects_clean_repo() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());

        let result = git_is_clean(temp.path().to_str().unwrap());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn ensure_unique_worktree_branch_increments_suffix() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        init_repo(temp.path());
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();

        // Create base branch and worktree path
        run_git(temp.path(), &["branch", "test-branch"]);
        fs::create_dir_all(worktrees_dir.join("test-branch-2")).unwrap();

        let unique = ensure_unique_worktree_branch(
            temp.path().to_str().unwrap(),
            &worktrees_dir,
            "test-branch",
        );
        // Should skip test-branch (exists as branch) and test-branch-2 (exists as path)
        assert_eq!(unique, "test-branch-3");
    }

    #[test]
    fn auto_worktree_branch_name_handles_empty_session() {
        let result = auto_worktree_branch_name("", "20260101-120000");
        assert_eq!(result, "prd-20260101-120000");
    }

    #[test]
    fn auto_worktree_branch_name_includes_session() {
        let result = auto_worktree_branch_name("my-session", "20260101-120000");
        assert_eq!(result, "prd-my-session-20260101-120000");
    }
}
