mod cli;

use clap::Parser;
use cli::{
    Cli, Command, ConfigArgs, ConfigCommand, InitArgs, LogsArgs, PrdArgs, PrdCheckArgs, PrdCommand,
    PrdCreateArgs, ResumeArgs, RunLoopArgs, ServerArgs, StartArgs, StopArgs, WorktreeCommand,
    WorktreeCreateArgs, WorktreeFinishArgs, ASCII_BANNER,
};
use gralph_rs::backend::backend_from_name;
use gralph_rs::config::Config;
use gralph_rs::core::{self, LoopStatus};
use gralph_rs::notify;
use gralph_rs::prd;
use gralph_rs::server::{self, ServerConfig};
use gralph_rs::state::{CleanupMode, StateStore};
use gralph_rs::update;
use gralph_rs::version;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcCommand, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        let _ = cmd_intro();
        return;
    };
    if let Err(err) = dispatch(command) {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn dispatch(command: Command) -> Result<(), CliError> {
    match command {
        Command::Start(args) => cmd_start(args),
        Command::RunLoop(args) => cmd_run_loop(args),
        Command::Stop(args) => cmd_stop(args),
        Command::Status => cmd_status(),
        Command::Logs(args) => cmd_logs(args),
        Command::Resume(args) => cmd_resume(args),
        Command::Init(args) => cmd_init(args),
        Command::Prd(args) => cmd_prd(args),
        Command::Worktree(args) => cmd_worktree(args),
        Command::Backends => cmd_backends(),
        Command::Config(args) => cmd_config(args),
        Command::Server(args) => cmd_server(args),
        Command::Version => cmd_version(),
        Command::Update => cmd_update(),
    }
}

#[derive(Debug)]
enum CliError {
    Message(String),
    Io(io::Error),
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Message(message) => write!(f, "{}", message),
            CliError::Io(err) => write!(f, "{}", err),
        }
    }
}

impl From<io::Error> for CliError {
    fn from(value: io::Error) -> Self {
        CliError::Io(value)
    }
}

fn cmd_intro() -> Result<(), CliError> {
    println!("{}", ASCII_BANNER);
    println!("gralph - Autonomous AI coding loops\n");
    println!(
        "gralph reads your PRD tasks and iterates with your chosen backend until tasks complete."
    );
    println!("Run in foreground with --no-tmux (tmux is disabled here).\n");
    println!("Get started:");
    println!("  gralph start . --no-tmux");
    println!("  gralph start /path/to/project --backend opencode --no-tmux\n");
    println!("Common commands:");
    println!("  gralph status");
    println!("  gralph logs <name>");
    println!("  gralph stop <name>");
    println!("  gralph backends");
    println!("  gralph prd create --dir . --output PRD.new.md --goal \"Add a billing dashboard\"");
    println!("  gralph init --dir .");
    println!("  gralph worktree create C-1\n");
    println!("More help:");
    println!("  gralph --help");
    println!("  gralph <command> --help");
    Ok(())
}

fn cmd_version() -> Result<(), CliError> {
    println!("gralph v{}", version::VERSION);
    Ok(())
}

fn cmd_update() -> Result<(), CliError> {
    let outcome = update::install_release().map_err(|err| CliError::Message(err.to_string()))?;
    println!(
        "Installed gralph v{} to {}",
        outcome.version,
        outcome.install_path.display()
    );
    match outcome.resolved_path {
        Some(resolved) if resolved != outcome.install_path => {
            println!("Warning: PATH resolves gralph to {}", resolved.display());
            println!(
                "Run {} or update PATH to prefer {}",
                outcome.install_path.display(),
                outcome.install_dir.display()
            );
        }
        Some(_) => {}
        None => {
            println!(
                "Warning: gralph not found in PATH. Add {} to PATH or run {}",
                outcome.install_dir.display(),
                outcome.install_path.display()
            );
        }
    }
    Ok(())
}

fn maybe_check_for_update() {
    let current_version = version::VERSION;
    match update::check_for_update(current_version) {
        Ok(Some(info)) => {
            println!(
                "Update available: gralph v{} -> v{}. Run `gralph update`.",
                info.current, info.latest
            );
        }
        Ok(None) => {}
        Err(err) => {
            eprintln!("Warning: update check failed: {}", err);
        }
    }
}

fn cmd_start(args: StartArgs) -> Result<(), CliError> {
    if !args.dir.is_dir() {
        return Err(CliError::Message(format!(
            "Directory does not exist: {}",
            args.dir.display()
        )));
    }
    let no_tmux = args.no_tmux;
    let session_name = session_name(&args.name, &args.dir)?;
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let mut run_args = run_loop_args_from_start(args, session_name)?;
    maybe_create_auto_worktree(&mut run_args, &config)?;
    if no_tmux {
        return run_loop_with_state(run_args);
    }

    let child = spawn_run_loop(&run_args)?;

    let store = StateStore::new_from_env();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let now = chrono::Local::now().to_rfc3339();
    let task_file = run_args
        .task_file
        .clone()
        .unwrap_or_else(|| "PRD.md".to_string());
    let completion_marker = run_args
        .completion_marker
        .clone()
        .unwrap_or_else(|| "COMPLETE".to_string());
    let max_iterations = run_args.max_iterations.unwrap_or(30);
    let remaining = core::count_remaining_tasks(&run_args.dir.join(&task_file));
    let log_file = run_args
        .dir
        .join(".gralph")
        .join(format!("{}.log", run_args.name));

    store
        .set_session(
            &run_args.name,
            &[
                ("dir", &run_args.dir.to_string_lossy()),
                ("task_file", &task_file),
                ("pid", &child.id().to_string()),
                ("tmux_session", ""),
                ("started_at", &now),
                ("iteration", "1"),
                ("max_iterations", &max_iterations.to_string()),
                ("status", "running"),
                ("last_task_count", &remaining.to_string()),
                ("completion_marker", &completion_marker),
                ("log_file", &log_file.to_string_lossy()),
                ("backend", run_args.backend.as_deref().unwrap_or("claude")),
                ("model", run_args.model.as_deref().unwrap_or("")),
                ("variant", run_args.variant.as_deref().unwrap_or("")),
                ("webhook", run_args.webhook.as_deref().unwrap_or("")),
            ],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;

    println!("Gralph loop started in background (PID: {}).", child.id());
    Ok(())
}

fn cmd_run_loop(mut args: RunLoopArgs) -> Result<(), CliError> {
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    maybe_create_auto_worktree(&mut args, &config)?;
    run_loop_with_state(args)
}

fn cmd_stop(args: StopArgs) -> Result<(), CliError> {
    let store = StateStore::new_from_env();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;

    if args.all {
        let sessions = store
            .list_sessions()
            .map_err(|err| CliError::Message(err.to_string()))?;
        for session in sessions {
            if let Some(name) = session.get("name").and_then(|v| v.as_str()) {
                stop_session(&store, name, &session)?;
            }
        }
        println!("Stopped running sessions.");
        return Ok(());
    }

    let name = args
        .name
        .ok_or_else(|| CliError::Message("Session name is required.".to_string()))?;
    let session = store
        .get_session(&name)
        .map_err(|err| CliError::Message(err.to_string()))?
        .ok_or_else(|| CliError::Message(format!("Session not found: {}", name)))?;

    stop_session(&store, &name, &session)?;
    println!("Stopped session: {}", name);
    Ok(())
}

fn cmd_status() -> Result<(), CliError> {
    let store = StateStore::new_from_env();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let _ = store.cleanup_stale(CleanupMode::Mark);

    let sessions = store
        .list_sessions()
        .map_err(|err| CliError::Message(err.to_string()))?;
    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    let mut rows = Vec::new();
    for session in sessions {
        let name = session
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let dir = session.get("dir").and_then(|v| v.as_str()).unwrap_or("");
        let task_file = session
            .get("task_file")
            .and_then(|v| v.as_str())
            .unwrap_or("PRD.md");
        let iteration = session
            .get("iteration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let max_iterations = session
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let status = session
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let remaining = if dir.is_empty() {
            session
                .get("last_task_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        } else {
            core::count_remaining_tasks(&PathBuf::from(dir).join(task_file)) as u64
        };

        rows.push(vec![
            name.to_string(),
            dir.to_string(),
            format!("{}/{}", iteration, max_iterations),
            status.to_string(),
            format!("{}", remaining),
        ]);
    }

    print_table(&["NAME", "DIR", "ITERATION", "STATUS", "REMAINING"], &rows);
    Ok(())
}

fn cmd_logs(args: LogsArgs) -> Result<(), CliError> {
    let store = StateStore::new_from_env();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let session = store
        .get_session(&args.name)
        .map_err(|err| CliError::Message(err.to_string()))?
        .ok_or_else(|| CliError::Message(format!("Session not found: {}", args.name)))?;
    let log_file = resolve_log_file(&args.name, &session)?;
    if !log_file.is_file() {
        return Err(CliError::Message(format!(
            "Log file does not exist: {}",
            log_file.display()
        )));
    }

    if args.follow {
        follow_log(&log_file)?;
    } else {
        print_tail(&log_file, 200)?;
    }
    Ok(())
}

fn cmd_resume(args: ResumeArgs) -> Result<(), CliError> {
    let store = StateStore::new_from_env();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let sessions = store
        .list_sessions()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let target = args.name;

    let mut resumed = 0;
    for session in sessions {
        let name = session.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        if let Some(target) = target.as_deref() {
            if name != target {
                continue;
            }
        }

        let status = session
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let pid = session.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
        let should_resume = status == "stale"
            || status == "stopped"
            || status == "failed"
            || (status == "running" && !is_process_alive(pid));
        if !should_resume {
            continue;
        }

        let dir = session
            .get("dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::Message(format!("Missing dir for session {}", name)))?;
        let task_file = session
            .get("task_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let max_iterations = session
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let completion_marker = session
            .get("completion_marker")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let backend = session
            .get("backend")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let model = session
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let variant = session
            .get("variant")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let webhook = session
            .get("webhook")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let run_args = RunLoopArgs {
            dir: PathBuf::from(dir),
            name: name.to_string(),
            max_iterations,
            task_file,
            completion_marker,
            backend,
            model,
            variant,
            prompt_template: None,
            webhook,
            no_worktree: true,
            strict_prd: false,
        };
        let child = spawn_run_loop(&run_args)?;
        store
            .set_session(
                name,
                &[("pid", &child.id().to_string()), ("status", "running")],
            )
            .map_err(|err| CliError::Message(err.to_string()))?;
        resumed += 1;
    }

    if resumed == 0 {
        println!("No sessions to resume.");
    } else {
        println!("Resumed {} session(s).", resumed);
    }
    Ok(())
}

fn cmd_prd(args: PrdArgs) -> Result<(), CliError> {
    match args.command {
        PrdCommand::Check(args) => cmd_prd_check(args),
        PrdCommand::Create(args) => cmd_prd_create(args),
    }
}

fn cmd_init(args: InitArgs) -> Result<(), CliError> {
    let target_dir = args
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !target_dir.is_dir() {
        return Err(CliError::Message(format!(
            "Directory does not exist: {}",
            target_dir.display()
        )));
    }

    let config =
        Config::load(Some(&target_dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let config_list = config.get("defaults.context_files");
    let entries = resolve_init_context_files(&target_dir, config_list.as_deref());
    if entries.is_empty() {
        println!("No context files configured.");
        return Ok(());
    }

    let mut created = Vec::new();
    let mut overwritten = Vec::new();
    let mut skipped = Vec::new();
    let mut skipped_non_md = Vec::new();

    for entry in entries {
        let path = if Path::new(&entry).is_absolute() {
            PathBuf::from(&entry)
        } else {
            target_dir.join(&entry)
        };
        if !is_markdown_path(&path) {
            println!("Skipping non-markdown entry: {}", entry);
            skipped_non_md.push(entry);
            continue;
        }
        let display = format_display_path(&path, &target_dir);
        let existed = path.exists();
        if existed && !args.force {
            skipped.push(display);
            continue;
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(CliError::Io)?;
        }
        let contents = init_template_for_path(&path);
        write_atomic(&path, &contents, args.force).map_err(CliError::Io)?;

        if existed {
            overwritten.push(display);
        } else {
            created.push(display);
        }
    }

    println!("Init summary:");
    println!("Created ({}): {}", created.len(), join_or_none(&created));
    println!(
        "Overwritten ({}): {}",
        overwritten.len(),
        join_or_none(&overwritten)
    );
    println!("Skipped ({}): {}", skipped.len(), join_or_none(&skipped));
    if !skipped_non_md.is_empty() {
        println!(
            "Non-markdown skipped ({}): {}",
            skipped_non_md.len(),
            join_or_none(&skipped_non_md)
        );
    }
    Ok(())
}

fn cmd_prd_check(args: PrdCheckArgs) -> Result<(), CliError> {
    prd::prd_validate_file(&args.file, args.allow_missing_context, None)
        .map_err(|err| CliError::Message(err.to_string()))?;
    println!("PRD validation passed: {}", args.file.display());
    Ok(())
}

fn cmd_prd_create(args: PrdCreateArgs) -> Result<(), CliError> {
    let target_dir = args
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !target_dir.is_dir() {
        return Err(CliError::Message(format!(
            "Directory does not exist: {}",
            target_dir.display()
        )));
    }

    let goal = args
        .goal
        .clone()
        .ok_or_else(|| CliError::Message("Goal is required. Use --goal.".to_string()))?;

    let constraints = args
        .constraints
        .clone()
        .unwrap_or_else(|| "None.".to_string());

    let output_path = resolve_prd_output(&target_dir, args.output.clone(), args.force)?;

    let config =
        Config::load(Some(&target_dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let backend_name = args
        .backend
        .clone()
        .or_else(|| config.get("defaults.backend"))
        .unwrap_or_else(|| "claude".to_string());
    let mut model = args.model.clone().or_else(|| config.get("defaults.model"));
    if model.as_deref().unwrap_or("").is_empty() && backend_name == "opencode" {
        model = config.get("opencode.default_model");
    }

    let backend = backend_from_name(&backend_name).map_err(CliError::Message)?;
    if !backend.check_installed() {
        return Err(CliError::Message(format!(
            "Backend is not installed: {}",
            backend_name
        )));
    }

    let stack = prd::prd_detect_stack(&target_dir);
    let stack_summary = prd::prd_format_stack_summary(&stack, 2);

    let context_files = build_context_file_list(
        &target_dir,
        args.context.as_deref(),
        config.get("defaults.context_files").as_deref(),
    );
    let context_section = if context_files.is_empty() {
        "None.".to_string()
    } else {
        context_files.join("\n")
    };

    let sources_section = match args.sources.as_deref() {
        Some(value) if !value.trim().is_empty() => normalize_csv(value).join("\n"),
        _ => "None.".to_string(),
    };

    let warnings_section = if sources_section == "None." {
        "No reliable external sources were provided. Verify requirements and stack assumptions before implementation."
            .to_string()
    } else {
        "None.".to_string()
    };

    let template_text = read_prd_template(&target_dir)?;
    let prompt = format!(
        "You are generating a gralph PRD in markdown. The output must be spec-compliant and grounded in the repository.\n\nProject directory: {dir}\n\nGoal:\n{goal}\n\nConstraints:\n{constraints}\n\nDetected stack summary (from repository files):\n{stack_summary}\n\nSources (authoritative URLs or references):\n{sources}\n\nWarnings (only include in the PRD if Sources is empty):\n{warnings}\n\nContext files (read these first if present):\n{context}\n\nRequirements:\n- Output only the PRD markdown with no commentary or code fences.\n- Use ASCII only.\n- Do not include an \"Open Questions\" section.\n- Do not use any checkboxes outside task blocks.\n- Context Bundle entries must be real files in the repo and must be selected from the Context files list above.\n- If a task creates new files, do not list the new files in Context Bundle; cite the closest existing files instead.\n- Use atomic, granular tasks grounded in the repo and context files.\n- Each task block must use a '### Task <ID>' header and include **ID**, **Context Bundle**, **DoD**, **Checklist**, **Dependencies**.\n- Each task block must contain exactly one unchecked task line like '- [ ] <ID> <summary>'.\n- If Sources is empty, include a 'Warnings' section with the warning text above and no checkboxes.\n- Do not invent stack, frameworks, or files not supported by the context files and stack summary.\n\nTemplate:\n{template}\n",
        dir = target_dir.display(),
        goal = goal,
        constraints = constraints,
        stack_summary = stack_summary,
        sources = sources_section,
        warnings = warnings_section,
        context = context_section,
        template = template_text
    );

    let tmp_dir = env::temp_dir();
    let output_file = tmp_dir.join(format!("gralph-prd-{}.tmp", std::process::id()));
    backend
        .run_iteration(
            &prompt,
            model.as_deref(),
            args.variant.as_deref(),
            &output_file,
            &target_dir,
        )
        .map_err(|err| CliError::Message(err.to_string()))?;
    let result = backend
        .parse_text(&output_file)
        .map_err(|err| CliError::Message(err.to_string()))?;
    if result.trim().is_empty() {
        return Err(CliError::Message(
            "PRD generation returned empty output.".to_string(),
        ));
    }

    let temp_prd = tmp_dir.join(format!("gralph-prd-{}.md", std::process::id()));
    fs::write(&temp_prd, result).map_err(CliError::Io)?;

    let allowed_context_file = write_allowed_context(&context_files)?;
    prd::prd_sanitize_generated_file(
        &temp_prd,
        Some(&target_dir),
        allowed_context_file.as_deref(),
    )
    .map_err(|err| CliError::Message(err.to_string()))?;

    if let Err(err) =
        prd::prd_validate_file(&temp_prd, args.allow_missing_context, Some(&target_dir))
    {
        let invalid_path = invalid_prd_path(&output_path, args.force);
        fs::rename(&temp_prd, &invalid_path).map_err(CliError::Io)?;
        return Err(CliError::Message(format!(
            "Generated PRD failed validation. Saved to {}. Details:\n{}",
            invalid_path.display(),
            err
        )));
    }

    fs::rename(&temp_prd, &output_path).map_err(CliError::Io)?;
    println!("PRD created: {}", output_path.display());
    Ok(())
}

fn cmd_worktree(args: cli::WorktreeArgs) -> Result<(), CliError> {
    match args.command {
        WorktreeCommand::Create(args) => cmd_worktree_create(args),
        WorktreeCommand::Finish(args) => cmd_worktree_finish(args),
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

fn cmd_backends() -> Result<(), CliError> {
    let backends = vec![
        (
            "claude",
            backend_from_name("claude").map_err(CliError::Message)?,
            "https://docs.anthropic.com/claude-code",
        ),
        (
            "opencode",
            backend_from_name("opencode").map_err(CliError::Message)?,
            "https://opencode.ai",
        ),
        (
            "gemini",
            backend_from_name("gemini").map_err(CliError::Message)?,
            "https://ai.google.dev",
        ),
        (
            "codex",
            backend_from_name("codex").map_err(CliError::Message)?,
            "https://platform.openai.com/docs",
        ),
    ];

    println!("Available AI backends:\n");
    for (name, backend, hint) in backends {
        if backend.check_installed() {
            println!("  {} (installed)", name);
            println!("      Models: {}", backend.get_models().join(", "));
        } else {
            println!("  {} (not installed)", name);
            println!("      Install: {}", hint);
        }
        println!();
    }
    Ok(())
}

fn cmd_config(args: ConfigArgs) -> Result<(), CliError> {
    match args.command.unwrap_or(ConfigCommand::List) {
        ConfigCommand::Get(args) => cmd_config_get(args),
        ConfigCommand::Set(args) => cmd_config_set(args),
        ConfigCommand::List => cmd_config_list(),
    }
}

fn cmd_config_get(args: cli::ConfigGetArgs) -> Result<(), CliError> {
    let config = Config::load(Some(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    ))
    .map_err(|err| CliError::Message(err.to_string()))?;
    if let Some(value) = config.get(&args.key) {
        println!("{}", value);
        Ok(())
    } else {
        Err(CliError::Message(format!(
            "Config key not found: {}",
            args.key
        )))
    }
}

fn cmd_config_set(args: cli::ConfigSetArgs) -> Result<(), CliError> {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_path = project_config_path(&current_dir);
    let mut root = read_yaml_or_empty(&config_path)?;
    set_yaml_value(&mut root, &args.key, &args.value);
    let rendered = serde_yaml::to_string(&root)
        .map_err(|err| CliError::Message(format!("Failed to serialize config: {}", err)))?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(CliError::Io)?;
    }
    fs::write(&config_path, rendered).map_err(CliError::Io)?;
    println!("Updated config: {}", args.key);
    Ok(())
}

fn cmd_config_list() -> Result<(), CliError> {
    let config = Config::load(Some(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    ))
    .map_err(|err| CliError::Message(err.to_string()))?;
    for (key, value) in config.list() {
        println!("{}={}", key, value);
    }
    Ok(())
}

fn cmd_server(args: ServerArgs) -> Result<(), CliError> {
    let mut config = ServerConfig::from_env();
    if let Some(host) = args.host {
        config.host = host;
    }
    if let Some(port) = args.port {
        config.port = port;
    }
    if let Some(token) = args.token {
        config.token = Some(token);
    }
    if args.open {
        config.open = true;
    }

    let runtime = tokio::runtime::Runtime::new().map_err(CliError::Io)?;
    runtime
        .block_on(server::run_server(config))
        .map_err(|err| CliError::Message(err.to_string()))
}

fn run_loop_with_state(args: RunLoopArgs) -> Result<(), CliError> {
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    maybe_check_for_update();
    let task_file = args
        .task_file
        .clone()
        .or_else(|| config.get("defaults.task_file"))
        .unwrap_or_else(|| "PRD.md".to_string());
    let max_iterations = args
        .max_iterations
        .or_else(|| {
            config
                .get("defaults.max_iterations")
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(30);
    let completion_marker = args
        .completion_marker
        .clone()
        .or_else(|| config.get("defaults.completion_marker"))
        .unwrap_or_else(|| "COMPLETE".to_string());
    let backend_name = args
        .backend
        .clone()
        .or_else(|| config.get("defaults.backend"))
        .unwrap_or_else(|| "claude".to_string());
    let mut model = args.model.clone().or_else(|| config.get("defaults.model"));
    if model.as_deref().unwrap_or("").is_empty() && backend_name == "opencode" {
        model = config.get("opencode.default_model");
    }

    if args.strict_prd {
        prd::prd_validate_file(&args.dir.join(&task_file), false, Some(&args.dir))
            .map_err(|err| CliError::Message(err.to_string()))?;
    }

    let prompt_template = match &args.prompt_template {
        Some(path) => Some(fs::read_to_string(path).map_err(CliError::Io)?),
        None => None,
    };

    let backend = backend_from_name(&backend_name).map_err(CliError::Message)?;
    if !backend.check_installed() {
        return Err(CliError::Message(format!(
            "Backend is not installed: {}",
            backend_name
        )));
    }

    let store = StateStore::new_from_env();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let now = chrono::Local::now().to_rfc3339();
    let remaining = core::count_remaining_tasks(&args.dir.join(&task_file));
    let log_file = args.dir.join(".gralph").join(format!("{}.log", args.name));

    store
        .set_session(
            &args.name,
            &[
                ("dir", &args.dir.to_string_lossy()),
                ("task_file", &task_file),
                ("pid", &std::process::id().to_string()),
                ("tmux_session", ""),
                ("started_at", &now),
                ("iteration", "1"),
                ("max_iterations", &max_iterations.to_string()),
                ("status", "running"),
                ("last_task_count", &remaining.to_string()),
                ("completion_marker", &completion_marker),
                ("log_file", &log_file.to_string_lossy()),
                ("backend", &backend_name),
                ("model", model.as_deref().unwrap_or("")),
                ("variant", args.variant.as_deref().unwrap_or("")),
                ("webhook", args.webhook.as_deref().unwrap_or("")),
            ],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;

    let mut callback =
        |name: Option<&str>, iteration: u32, status: LoopStatus, remaining: usize| {
            let session = name.unwrap_or(&args.name);
            let _ = store.set_session(
                session,
                &[
                    ("iteration", &iteration.to_string()),
                    ("status", status.as_str()),
                    ("last_task_count", &remaining.to_string()),
                ],
            );
        };

    let outcome = core::run_loop(
        &*backend,
        &args.dir,
        Some(&task_file),
        Some(max_iterations),
        Some(&completion_marker),
        model.as_deref(),
        args.variant.as_deref(),
        Some(&args.name),
        prompt_template.as_deref(),
        Some(&config),
        Some(&mut callback),
    )
    .map_err(|err| CliError::Message(err.to_string()))?;

    store
        .set_session(
            &args.name,
            &[
                ("status", outcome.status.as_str()),
                ("last_task_count", &outcome.remaining_tasks.to_string()),
            ],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;

    notify_if_configured(&config, &args, &outcome, max_iterations)?;
    Ok(())
}

fn notify_if_configured(
    config: &Config,
    args: &RunLoopArgs,
    outcome: &core::LoopOutcome,
    max_iterations: u32,
) -> Result<(), CliError> {
    let webhook = args
        .webhook
        .clone()
        .or_else(|| config.get("notifications.webhook"));
    let Some(webhook) = webhook else {
        return Ok(());
    };

    match outcome.status {
        LoopStatus::Complete => {
            let on_complete = config
                .get("notifications.on_complete")
                .map(|v| v == "true")
                .unwrap_or(true);
            if on_complete {
                notify::notify_complete(
                    &args.name,
                    &webhook,
                    Some(&args.dir.to_string_lossy()),
                    Some(outcome.iterations),
                    Some(outcome.duration_secs),
                    None,
                )
                .map_err(|err| CliError::Message(err.to_string()))?;
            }
        }
        LoopStatus::Failed | LoopStatus::MaxIterations => {
            notify::notify_failed(
                &args.name,
                &webhook,
                Some(match outcome.status {
                    LoopStatus::Failed => "error",
                    LoopStatus::MaxIterations => "max_iterations",
                    _ => "error",
                }),
                Some(&args.dir.to_string_lossy()),
                Some(outcome.iterations),
                Some(max_iterations),
                Some(outcome.remaining_tasks as u32),
                Some(outcome.duration_secs),
                None,
            )
            .map_err(|err| CliError::Message(err.to_string()))?;
        }
        LoopStatus::Running => {}
    }

    Ok(())
}

fn run_loop_args_from_start(args: StartArgs, name: String) -> Result<RunLoopArgs, CliError> {
    Ok(RunLoopArgs {
        dir: args.dir,
        name,
        max_iterations: args.max_iterations,
        task_file: args.task_file,
        completion_marker: args.completion_marker,
        backend: args.backend,
        model: args.model,
        variant: args.variant,
        prompt_template: args.prompt_template,
        webhook: args.webhook,
        no_worktree: args.no_worktree,
        strict_prd: args.strict_prd,
    })
}

const DEFAULT_SESSION_NAME: &str = "gralph";

fn session_name(name: &Option<String>, dir: &Path) -> Result<String, CliError> {
    if let Some(name) = name {
        return Ok(sanitize_session_name(name));
    }
    let canonical_name = dir.canonicalize().ok().and_then(|path| {
        path.file_name()
            .and_then(OsStr::to_str)
            .map(|value| value.to_string())
    });
    let raw_name = canonical_name.or_else(|| {
        dir.file_name()
            .and_then(OsStr::to_str)
            .map(|value| value.to_string())
    });
    if let Some(raw_name) = raw_name {
        let sanitized = sanitize_session_name(&raw_name);
        if !sanitized.is_empty() {
            return Ok(sanitized);
        }
    }
    Ok(DEFAULT_SESSION_NAME.to_string())
}

fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn spawn_run_loop(args: &RunLoopArgs) -> Result<std::process::Child, CliError> {
    let exe = env::current_exe().map_err(CliError::Io)?;
    let mut cmd = ProcCommand::new(exe);
    cmd.arg("run-loop")
        .arg(args.dir.to_string_lossy().as_ref())
        .arg("--name")
        .arg(&args.name);

    if let Some(max) = args.max_iterations {
        cmd.arg("--max-iterations").arg(max.to_string());
    }
    if let Some(task_file) = args.task_file.as_deref() {
        cmd.arg("--task-file").arg(task_file);
    }
    if let Some(marker) = args.completion_marker.as_deref() {
        cmd.arg("--completion-marker").arg(marker);
    }
    if let Some(backend) = args.backend.as_deref() {
        cmd.arg("--backend").arg(backend);
    }
    if let Some(model) = args.model.as_deref() {
        cmd.arg("--model").arg(model);
    }
    if let Some(variant) = args.variant.as_deref() {
        cmd.arg("--variant").arg(variant);
    }
    if let Some(template) = args.prompt_template.as_ref() {
        cmd.arg("--prompt-template").arg(template);
    }
    if let Some(webhook) = args.webhook.as_deref() {
        cmd.arg("--webhook").arg(webhook);
    }
    if args.no_worktree {
        cmd.arg("--no-worktree");
    }
    if args.strict_prd {
        cmd.arg("--strict-prd");
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| CliError::Message(format!("Failed to start loop: {}", err)))
}

fn stop_session(
    store: &StateStore,
    name: &str,
    session: &serde_json::Value,
) -> Result<(), CliError> {
    if let Some(tmux) = session.get("tmux_session").and_then(|v| v.as_str()) {
        if !tmux.trim().is_empty() {
            let _ = ProcCommand::new("tmux")
                .arg("kill-session")
                .arg("-t")
                .arg(tmux)
                .status();
        }
    }
    let pid = session.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
    if pid > 0 {
        #[cfg(unix)]
        {
            let _ = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .status();
        }
    }
    store
        .set_session(
            name,
            &[("status", "stopped"), ("pid", "0"), ("tmux_session", "")],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;
    Ok(())
}

fn resolve_log_file(name: &str, session: &serde_json::Value) -> Result<PathBuf, CliError> {
    if let Some(path) = session.get("log_file").and_then(|v| v.as_str()) {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    let dir = session
        .get("dir")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::Message(format!("Missing dir for session {}", name)))?;
    Ok(PathBuf::from(dir)
        .join(".gralph")
        .join(format!("{}.log", name)))
}

fn follow_log(path: &Path) -> Result<(), CliError> {
    let mut file = fs::File::open(path).map_err(CliError::Io)?;
    let mut pos = file.seek(SeekFrom::End(0)).map_err(CliError::Io)?;
    loop {
        let mut buffer = String::new();
        file.seek(SeekFrom::Start(pos)).map_err(CliError::Io)?;
        let bytes = file.read_to_string(&mut buffer).map_err(CliError::Io)?;
        if bytes > 0 {
            print!("{}", buffer);
            io::stdout().flush().map_err(CliError::Io)?;
            pos += bytes as u64;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn print_tail(path: &Path, lines: usize) -> Result<(), CliError> {
    let contents = fs::read_to_string(path).map_err(CliError::Io)?;
    let total: Vec<&str> = contents.lines().collect();
    let start = total.len().saturating_sub(lines);
    for line in &total[start..] {
        println!("{}", line);
    }
    Ok(())
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();
    for row in rows {
        for (index, col) in row.iter().enumerate() {
            if col.len() > widths[index] {
                widths[index] = col.len();
            }
        }
    }
    for (index, header) in headers.iter().enumerate() {
        print!("{:width$}  ", header, width = widths[index]);
    }
    println!();
    for (index, _) in headers.iter().enumerate() {
        print!(
            "{:width$}  ",
            "-".repeat(widths[index]),
            width = widths[index]
        );
    }
    println!();
    for row in rows {
        for (index, col) in row.iter().enumerate() {
            print!("{:width$}  ", col, width = widths[index]);
        }
        println!();
    }
}

fn validate_task_id(id: &str) -> Result<(), CliError> {
    let mut parts = id.split('-');
    let prefix = parts.next().unwrap_or("");
    let number = parts.next().unwrap_or("");
    let valid = !prefix.is_empty()
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

fn git_output_in_dir(
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
    let status = ProcCommand::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .status()
        .map_err(CliError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError::Message("git command failed".to_string()))
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

fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" | "on" => Some(true),
        "false" | "0" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

fn resolve_auto_worktree(config: &Config, no_worktree: bool) -> bool {
    if no_worktree {
        return false;
    }
    config
        .get("defaults.auto_worktree")
        .as_deref()
        .and_then(parse_bool_value)
        .unwrap_or(true)
}

fn worktree_timestamp_slug() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn auto_worktree_branch_name(session_name: &str, timestamp: &str) -> String {
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

fn ensure_unique_worktree_branch(repo_root: &str, worktrees_dir: &Path, base: &str) -> String {
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while git_branch_exists(repo_root, &candidate) || worktrees_dir.join(&candidate).exists() {
        candidate = format!("{}-{}", base, suffix);
        suffix += 1;
    }
    candidate
}

fn create_worktree_at(repo_root: &str, branch: &str, worktree_path: &Path) -> Result<(), CliError> {
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

fn maybe_create_auto_worktree(args: &mut RunLoopArgs, config: &Config) -> Result<(), CliError> {
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

    let timestamp = worktree_timestamp_slug();
    let base_branch = auto_worktree_branch_name(&args.name, &timestamp);
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

fn project_config_path(project_dir: &Path) -> PathBuf {
    let name =
        env::var("GRALPH_PROJECT_CONFIG_NAME").unwrap_or_else(|_| ".gralph.yaml".to_string());
    project_dir.join(name)
}

fn read_yaml_or_empty(path: &Path) -> Result<serde_yaml::Value, CliError> {
    if path.is_file() {
        let contents = fs::read_to_string(path).map_err(CliError::Io)?;
        let value = serde_yaml::from_str(&contents)
            .map_err(|err| CliError::Message(format!("Failed to parse config: {}", err)))?;
        Ok(value)
    } else {
        Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()))
    }
}

fn set_yaml_value(root: &mut serde_yaml::Value, key: &str, value: &str) {
    let parts: Vec<&str> = key.split('.').collect();
    set_yaml_value_parts(root, &parts, parse_yaml_value(value));
}

fn set_yaml_value_parts(root: &mut serde_yaml::Value, parts: &[&str], value: serde_yaml::Value) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        let key = serde_yaml::Value::String(parts[0].to_string());
        ensure_mapping(root).insert(key, value);
        return;
    }

    let key = serde_yaml::Value::String(parts[0].to_string());
    let map = ensure_mapping(root);
    if !map.contains_key(&key) {
        map.insert(
            key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }
    if let Some(child) = map.get_mut(&key) {
        set_yaml_value_parts(child, &parts[1..], value);
    }
}

fn ensure_mapping(value: &mut serde_yaml::Value) -> &mut serde_yaml::Mapping {
    if !matches!(value, serde_yaml::Value::Mapping(_)) {
        *value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
    }
    match value {
        serde_yaml::Value::Mapping(map) => map,
        _ => unreachable!(),
    }
}

fn parse_yaml_value(value: &str) -> serde_yaml::Value {
    if value.eq_ignore_ascii_case("true") {
        return serde_yaml::Value::Bool(true);
    }
    if value.eq_ignore_ascii_case("false") {
        return serde_yaml::Value::Bool(false);
    }
    if let Ok(number) = value.parse::<i64>() {
        return serde_yaml::Value::Number(number.into());
    }
    serde_yaml::Value::String(value.to_string())
}

fn resolve_prd_output(
    dir: &Path,
    output: Option<PathBuf>,
    force: bool,
) -> Result<PathBuf, CliError> {
    let mut output_path = output.unwrap_or_else(|| PathBuf::from("PRD.generated.md"));
    if output_path.is_relative() {
        output_path = dir.join(output_path);
    }
    if output_path.exists() && !force {
        return Err(CliError::Message(format!(
            "Output file exists: {} (use --force to overwrite)",
            output_path.display()
        )));
    }
    Ok(output_path)
}

fn invalid_prd_path(output: &Path, force: bool) -> PathBuf {
    if force {
        return output.to_path_buf();
    }
    if output.extension().and_then(|ext| ext.to_str()) == Some("md") {
        output.with_extension("invalid.md")
    } else {
        output.with_extension("invalid")
    }
}

fn read_prd_template(dir: &Path) -> Result<String, CliError> {
    read_prd_template_with_manifest(dir, Path::new(env!("CARGO_MANIFEST_DIR")))
}

fn read_prd_template_with_manifest(dir: &Path, manifest_dir: &Path) -> Result<String, CliError> {
    let candidates = [
        dir.join("PRD.template.md"),
        manifest_dir.join("PRD.template.md"),
    ];
    for path in candidates {
        if path.is_file() {
            return fs::read_to_string(&path).map_err(CliError::Io);
        }
    }

    Ok(DEFAULT_PRD_TEMPLATE.to_string())
}

fn normalize_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect()
}

fn resolve_init_context_files(target_dir: &Path, config_list: Option<&str>) -> Vec<String> {
    let mut entries = Vec::new();
    let mut seen: BTreeMap<String, bool> = BTreeMap::new();

    let configured = config_list
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(normalize_csv)
        .unwrap_or_default();

    let fallback = if configured.is_empty() {
        let from_readme = read_readme_context_files(target_dir);
        if from_readme.is_empty() {
            default_context_files()
                .iter()
                .map(|item| item.to_string())
                .collect()
        } else {
            from_readme
        }
    } else {
        configured
    };

    for entry in fallback {
        if entry.trim().is_empty() {
            continue;
        }
        if seen.contains_key(&entry) {
            continue;
        }
        seen.insert(entry.clone(), true);
        entries.push(entry);
    }

    entries
}

fn read_readme_context_files(target_dir: &Path) -> Vec<String> {
    let mut entries = Vec::new();
    let mut seen: BTreeMap<String, bool> = BTreeMap::new();
    let readme_path = target_dir.join("README.md");
    let contents = match fs::read_to_string(&readme_path) {
        Ok(contents) => contents,
        Err(_) => return entries,
    };

    let mut in_section = false;
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## ") {
            if in_section {
                break;
            }
            if trimmed.contains("Context Files") {
                in_section = true;
            }
            continue;
        }
        if !in_section {
            continue;
        }

        let mut rest = trimmed;
        while let Some(start) = rest.find('`') {
            let remaining = &rest[start + 1..];
            if let Some(end) = remaining.find('`') {
                let candidate = &remaining[..end];
                if candidate.ends_with(".md") && !candidate.contains(' ') {
                    let value = candidate.to_string();
                    if !seen.contains_key(&value) {
                        seen.insert(value.clone(), true);
                        entries.push(value);
                    }
                }
                rest = &remaining[end + 1..];
            } else {
                break;
            }
        }
    }

    entries
}

fn default_context_files() -> [&'static str; 5] {
    [
        "ARCHITECTURE.md",
        "PROCESS.md",
        "DECISIONS.md",
        "RISK_REGISTER.md",
        "CHANGELOG.md",
    ]
}

fn is_markdown_path(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md") | Some("markdown") => true,
        _ => false,
    }
}

fn format_display_path(path: &Path, base: &Path) -> String {
    if path.starts_with(base) {
        path.strip_prefix(base)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    } else {
        path.to_string_lossy().to_string()
    }
}

fn join_or_none(entries: &[String]) -> String {
    if entries.is_empty() {
        "None".to_string()
    } else {
        entries.join(", ")
    }
}

fn init_template_for_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    match file_name {
        name if name.eq_ignore_ascii_case("ARCHITECTURE.md") => ARCHITECTURE_TEMPLATE.to_string(),
        name if name.eq_ignore_ascii_case("PROCESS.md") => PROCESS_TEMPLATE.to_string(),
        name if name.eq_ignore_ascii_case("DECISIONS.md") => DECISIONS_TEMPLATE.to_string(),
        name if name.eq_ignore_ascii_case("RISK_REGISTER.md") => RISK_REGISTER_TEMPLATE.to_string(),
        name if name.eq_ignore_ascii_case("CHANGELOG.md") => CHANGELOG_TEMPLATE.to_string(),
        _ => generic_markdown_template(path),
    }
}

fn generic_markdown_template(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Context");
    let title = stem.replace('_', " ");
    format!("# {}\n\n## Overview\n\nTBD.\n", title)
}

fn write_atomic(path: &Path, contents: &str, _force: bool) -> Result<(), io::Error> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("context.md");
    let temp_name = format!("{}.tmp-{}", file_name, std::process::id());
    let temp_path = path.with_file_name(temp_name);
    fs::write(&temp_path, contents)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

fn build_context_file_list(
    target_dir: &Path,
    user_list: Option<&str>,
    config_list: Option<&str>,
) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, bool> = BTreeMap::new();

    for raw in [config_list, user_list] {
        if let Some(list) = raw {
            for item in normalize_csv(list) {
                add_context_entry(target_dir, &item, &mut entries, &mut seen);
            }
        }
    }

    for item in [
        "README.md",
        "ARCHITECTURE.md",
        "DECISIONS.md",
        "CHANGELOG.md",
        "RISK_REGISTER.md",
        "PROCESS.md",
        "PRD.template.md",
        "config/default.yaml",
        "opencode.json",
        "completions/gralph.bash",
        "completions/gralph.zsh",
        "Cargo.toml",
        "src/main.rs",
        "src/cli.rs",
        "src/core.rs",
        "src/state.rs",
        "src/config.rs",
        "src/server.rs",
        "src/notify.rs",
        "src/prd.rs",
        "src/lib.rs",
        "src/backend/mod.rs",
        "src/backend/claude.rs",
        "src/backend/opencode.rs",
        "src/backend/gemini.rs",
        "src/backend/codex.rs",
    ] {
        add_context_entry(target_dir, item, &mut entries, &mut seen);
    }

    entries
}

fn add_context_entry(
    target_dir: &Path,
    entry: &str,
    output: &mut Vec<String>,
    seen: &mut BTreeMap<String, bool>,
) {
    if entry.trim().is_empty() {
        return;
    }
    let path = if Path::new(entry).is_absolute() {
        PathBuf::from(entry)
    } else {
        target_dir.join(entry)
    };
    if !path.is_file() {
        return;
    }
    let display = if path.starts_with(target_dir) {
        path.strip_prefix(target_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string()
    } else {
        path.to_string_lossy().to_string()
    };
    if seen.contains_key(&display) {
        return;
    }
    seen.insert(display.clone(), true);
    output.push(display);
}

fn write_allowed_context(entries: &[String]) -> Result<Option<PathBuf>, CliError> {
    if entries.is_empty() {
        return Ok(None);
    }
    let path = env::temp_dir().join(format!("gralph-context-{}.txt", std::process::id()));
    let mut file = fs::File::create(&path).map_err(CliError::Io)?;
    for entry in entries {
        writeln!(file, "{}", entry).map_err(CliError::Io)?;
    }
    Ok(Some(path))
}

fn is_process_alive(pid: i64) -> bool {
    if pid <= 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, 0) };
        if result == 0 {
            return true;
        }
        let err = io::Error::last_os_error();
        return err.kind() == io::ErrorKind::PermissionDenied;
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        clear_env_overrides();
        guard
    }

    fn clear_env_overrides() {
        for key in [
            "GRALPH_DEFAULT_CONFIG",
            "GRALPH_GLOBAL_CONFIG",
            "GRALPH_CONFIG_DIR",
            "GRALPH_PROJECT_CONFIG_NAME",
            "GRALPH_DEFAULTS_AUTO_WORKTREE",
        ] {
            remove_env(key);
        }
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env(key: &str) {
        unsafe {
            env::remove_var(key);
        }
    }

    fn run_loop_args(dir: PathBuf) -> RunLoopArgs {
        RunLoopArgs {
            dir,
            name: "test-session".to_string(),
            max_iterations: None,
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: None,
            webhook: None,
            no_worktree: false,
            strict_prd: false,
        }
    }

    fn git_status_ok(dir: &Path, args: &[&str]) {
        let output = ProcCommand::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_git_repo(dir: &Path) {
        git_status_ok(dir, &["init"]);
        git_status_ok(dir, &["config", "user.email", "test@example.com"]);
        git_status_ok(dir, &["config", "user.name", "Test User"]);
    }

    fn commit_file(dir: &Path, relative: &str, contents: &str) {
        let path = dir.join(relative);
        write_file(&path, contents);
        git_status_ok(dir, &["add", relative]);
        git_status_ok(dir, &["commit", "-m", "init"]);
    }

    #[test]
    fn resolve_prd_output_handles_relative_and_absolute_paths() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();

        let relative = resolve_prd_output(base, Some(PathBuf::from("PRD.out.md")), false).unwrap();
        assert_eq!(relative, base.join("PRD.out.md"));

        let absolute = base.join("PRD.abs.md");
        let resolved = resolve_prd_output(base, Some(absolute.clone()), false).unwrap();
        assert_eq!(resolved, absolute);
    }

    #[test]
    fn resolve_prd_output_respects_force_for_existing_files() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        let output = base.join("PRD.generated.md");
        fs::write(&output, "existing").unwrap();

        let err = resolve_prd_output(base, Some(output.clone()), false).unwrap_err();
        match err {
            CliError::Message(message) => assert!(message.contains("Output file exists")),
            _ => panic!("unexpected error type"),
        }

        let resolved = resolve_prd_output(base, Some(output.clone()), true).unwrap();
        assert_eq!(resolved, output);
    }

    #[test]
    fn invalid_prd_path_handles_extensions_and_force() {
        let output_md = PathBuf::from("PRD.generated.md");
        let invalid_md = invalid_prd_path(&output_md, false);
        assert_eq!(invalid_md, PathBuf::from("PRD.generated.invalid.md"));

        let output_txt = PathBuf::from("PRD.generated.txt");
        let invalid_txt = invalid_prd_path(&output_txt, false);
        assert_eq!(invalid_txt, PathBuf::from("PRD.generated.invalid"));

        let forced = invalid_prd_path(&output_txt, true);
        assert_eq!(forced, output_txt);
    }

    #[test]
    fn cli_parse_reports_missing_required_args() {
        assert!(Cli::try_parse_from(["gralph", "logs"]).is_err());
        assert!(Cli::try_parse_from(["gralph", "worktree", "create"]).is_err());
        assert!(Cli::try_parse_from(["gralph", "prd", "check"]).is_err());
    }

    #[test]
    fn read_prd_template_prefers_project_template() {
        let project = tempfile::tempdir().unwrap();
        let manifest = tempfile::tempdir().unwrap();
        fs::write(project.path().join("PRD.template.md"), "project template").unwrap();
        fs::write(manifest.path().join("PRD.template.md"), "fallback template").unwrap();

        let template = read_prd_template_with_manifest(project.path(), manifest.path()).unwrap();

        assert_eq!(template, "project template");
    }

    #[test]
    fn read_prd_template_falls_back_to_default_content() {
        let project = tempfile::tempdir().unwrap();
        let manifest = tempfile::tempdir().unwrap();

        let template = read_prd_template_with_manifest(project.path(), manifest.path()).unwrap();

        assert_eq!(template, DEFAULT_PRD_TEMPLATE);
    }

    #[test]
    fn init_is_idempotent_without_force() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("default.yaml");

        write_file(
            &config_path,
            "defaults:\n  context_files: ARCHITECTURE.md\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        set_env("GRALPH_GLOBAL_CONFIG", temp.path().join("missing.yaml"));

        let args = InitArgs {
            dir: Some(temp.path().to_path_buf()),
            force: false,
        };
        cmd_init(args.clone()).unwrap();

        let path = temp.path().join("ARCHITECTURE.md");
        let first = fs::read_to_string(&path).unwrap();
        cmd_init(args).unwrap();
        let second = fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
        clear_env_overrides();
    }

    #[test]
    fn init_overwrites_with_force() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("default.yaml");

        write_file(
            &config_path,
            "defaults:\n  context_files: ARCHITECTURE.md\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        set_env("GRALPH_GLOBAL_CONFIG", temp.path().join("missing.yaml"));

        let path = temp.path().join("ARCHITECTURE.md");
        write_file(&path, "custom content");

        let args = InitArgs {
            dir: Some(temp.path().to_path_buf()),
            force: true,
        };
        cmd_init(args).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, ARCHITECTURE_TEMPLATE);
        clear_env_overrides();
    }

    #[test]
    fn init_reports_missing_directory() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");

        let args = InitArgs {
            dir: Some(missing.clone()),
            force: false,
        };
        let err = cmd_init(args).unwrap_err();
        match err {
            CliError::Message(message) => assert!(message.contains("Directory does not exist")),
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn init_falls_back_to_readme_context_files() {
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("README.md"),
            "## Context Files\n\n- `ARCHITECTURE.md`\n- `PROCESS.md`\n",
        );

        let entries = resolve_init_context_files(temp.path(), Some(""));

        assert_eq!(entries, vec!["ARCHITECTURE.md", "PROCESS.md"]);
    }

    #[test]
    fn auto_worktree_branch_name_uses_session_and_timestamp() {
        let name = auto_worktree_branch_name("demo-app", "20260126-120000");
        assert_eq!(name, "prd-demo-app-20260126-120000");

        let empty = auto_worktree_branch_name("", "20260126-120000");
        assert_eq!(empty, "prd-20260126-120000");
    }

    #[test]
    fn session_name_uses_canonical_basename_for_dot() {
        let expected_path = env::current_dir().unwrap().canonicalize().unwrap();
        let expected = expected_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap()
            .to_string();
        let resolved = session_name(&None, Path::new(".")).unwrap();
        assert_eq!(resolved, sanitize_session_name(&expected));
    }

    #[test]
    #[cfg(unix)]
    fn session_name_falls_back_for_root() {
        let resolved = session_name(&None, Path::new("/")).unwrap();
        assert_eq!(resolved, DEFAULT_SESSION_NAME);
    }

    #[test]
    #[cfg(windows)]
    fn session_name_falls_back_for_root() {
        let resolved = session_name(&None, Path::new(r"C:\\")).unwrap();
        assert_eq!(resolved, DEFAULT_SESSION_NAME);
    }

    #[test]
    fn resolve_auto_worktree_defaults_true() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config = Config::load(Some(temp.path())).unwrap();

        assert!(resolve_auto_worktree(&config, false));
    }

    #[test]
    fn resolve_auto_worktree_respects_project_config_and_cli_override() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join(".gralph.yaml"),
            "defaults:\n  auto_worktree: false\n",
        );
        let config = Config::load(Some(temp.path())).unwrap();

        assert!(!resolve_auto_worktree(&config, false));
        assert!(!resolve_auto_worktree(&config, true));
    }

    #[test]
    fn auto_worktree_skips_non_git_directory() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());
        let original = args.dir.clone();

        maybe_create_auto_worktree(&mut args, &config).unwrap();

        assert_eq!(args.dir, original);
        assert!(!args.no_worktree);
    }

    #[test]
    fn auto_worktree_skips_repo_without_commits() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());

        maybe_create_auto_worktree(&mut args, &config).unwrap();

        assert_eq!(args.dir, temp.path());
        assert!(!args.no_worktree);
        assert!(!temp.path().join(".worktrees").exists());
    }

    #[test]
    fn auto_worktree_errors_on_dirty_repo() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        write_file(&temp.path().join("README.md"), "dirty");
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());

        let err = maybe_create_auto_worktree(&mut args, &config).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Git working tree is dirty"));
            }
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn auto_worktree_maps_subdir_to_worktree_path() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        let nested = temp.path().join("nested");
        commit_file(temp.path(), "nested/task.md", "content");
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(nested.clone());

        maybe_create_auto_worktree(&mut args, &config).unwrap();

        let worktrees_dir = temp.path().join(".worktrees");
        let mut entries: Vec<PathBuf> = fs::read_dir(&worktrees_dir)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        assert_eq!(entries.len(), 1);
        let worktree_path = entries.remove(0);
        let expected = fs::canonicalize(worktree_path.join("nested")).unwrap();
        let actual = fs::canonicalize(&args.dir).unwrap();
        assert_eq!(actual, expected);
        assert!(args.no_worktree);
    }

    #[test]
    fn ensure_unique_worktree_branch_handles_collisions() {
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();
        git_status_ok(temp.path(), &["branch", "prd-collision"]);
        fs::create_dir_all(worktrees_dir.join("prd-collision-2")).unwrap();

        let branch = ensure_unique_worktree_branch(
            temp.path().to_str().unwrap(),
            &worktrees_dir,
            "prd-collision",
        );

        assert_eq!(branch, "prd-collision-3");
    }
}

const DEFAULT_PRD_TEMPLATE: &str = "## Overview\n\nBriefly describe the project, goals, and intended users.\n\n## Problem Statement\n\n- What problem does this solve?\n- What pain points exist today?\n\n## Solution\n\nHigh-level solution summary.\n\n---\n\n## Functional Requirements\n\n### FR-1: Core Feature\n\nDescribe the primary user-facing behavior.\n\n### FR-2: Secondary Feature\n\nDescribe supporting behavior.\n\n---\n\n## Non-Functional Requirements\n\n### NFR-1: Performance\n\n- Example: Response times under 200ms for key operations.\n\n### NFR-2: Reliability\n\n- Example: Crash recovery or retries where appropriate.\n\n---\n\n## Implementation Tasks\n\nEach task must use a `### Task <ID>` block header and include the required fields.\nEach task block must contain exactly one unchecked task line.\n\n### Task EX-1\n\n- **ID** EX-1\n- **Context Bundle** `path/to/file`, `path/to/other`\n- **DoD** Define the done criteria for this task.\n- **Checklist**\n  * First verification item.\n  * Second verification item.\n- **Dependencies** None\n- [ ] EX-1 Short task summary\n\n---\n\n## Success Criteria\n\n- Define measurable outcomes that indicate completion.\n\n---\n\n## Sources\n\n- List authoritative URLs used as source of truth.\n\n---\n\n## Warnings\n\n- Only include this section if no reliable sources were found.\n- State what is missing and what must be verified.\n";

const ARCHITECTURE_TEMPLATE: &str = "# Architecture\n\n## Overview\n\nDescribe the system at a high level.\n\n## Modules\n\nList key modules and what they own.\n\n## Runtime Flow\n\nDescribe the primary runtime path.\n\n## Storage\n\nRecord where state or data is stored.\n";

const PROCESS_TEMPLATE: &str = "# Process\n\n## Worktree Protocol\n\n1) Read required context files.\n2) Create a task worktree.\n3) Implement the scoped task.\n4) Update shared docs as needed.\n5) Verify changes.\n6) Finish and merge worktree.\n\n## Guardrails\n\n- Keep changes scoped to the assigned task.\n- Update CHANGELOG with the task ID.\n- Record new decisions and risks.\n";

const DECISIONS_TEMPLATE: &str = "# Decisions\n\n## D-001 Decision Title\n- Date: YYYY-MM-DD\n- Status: Proposed\n\n### Context\n\nWhy this decision is needed.\n\n### Decision\n\nWhat was decided.\n\n### Rationale\n\nWhy this choice was made.\n\n### Alternatives\n\nOther options considered.\n";

const RISK_REGISTER_TEMPLATE: &str = "# Risk Register\n\n## R-001 Risk Title\n- Risk: Describe the risk.\n- Impact: Low/Medium/High\n- Mitigation: How to reduce or monitor it.\n";

const CHANGELOG_TEMPLATE: &str = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nThe format is based on Keep a Changelog.\n\n## [Unreleased]\n\n### Added\n\n### Fixed\n";
