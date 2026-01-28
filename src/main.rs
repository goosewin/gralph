mod cli;

use clap::Parser;
use cli::{
    Cli, Command, ConfigArgs, ConfigCommand, InitArgs, LogsArgs, PrdArgs, PrdCheckArgs, PrdCommand,
    PrdCreateArgs, ResumeArgs, RunLoopArgs, ServerArgs, StartArgs, StopArgs, VerifierArgs,
    WorktreeCommand, WorktreeCreateArgs, WorktreeFinishArgs, ASCII_BANNER,
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
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
        Command::Verifier(args) => cmd_verifier(args),
        Command::Server(args) => cmd_server(args),
        Command::Version => cmd_version(),
        Command::Update => cmd_update(),
    }
}

const DEFAULT_TEST_COMMAND: &str = "cargo test --workspace";
const DEFAULT_COVERAGE_COMMAND: &str = "cargo tarpaulin --workspace --fail-under 60 --exclude-files src/main.rs src/core.rs src/notify.rs src/server.rs src/backend/*";
const DEFAULT_COVERAGE_MIN: f64 = 90.0;
const DEFAULT_PR_BASE: &str = "main";
const DEFAULT_PR_TITLE: &str = "chore: verifier run";
const DEFAULT_STATIC_MAX_COMMENT_LINES: usize = 12;
const DEFAULT_STATIC_MAX_COMMENT_CHARS: usize = 600;
const DEFAULT_STATIC_DUPLICATE_BLOCK_LINES: usize = 8;
const DEFAULT_STATIC_DUPLICATE_MIN_ALNUM_LINES: usize = 4;
const DEFAULT_STATIC_MAX_FILE_BYTES: u64 = 1_000_000;
const DEFAULT_REVIEW_ENABLED: bool = true;
const DEFAULT_REVIEW_REVIEWER: &str = "greptile";
const DEFAULT_REVIEW_MIN_RATING: f64 = 8.0;
const DEFAULT_REVIEW_MAX_ISSUES: usize = 0;
const DEFAULT_REVIEW_POLL_SECONDS: u64 = 20;
const DEFAULT_REVIEW_TIMEOUT_SECONDS: u64 = 1800;
const DEFAULT_REVIEW_REQUIRE_APPROVAL: bool = false;
const DEFAULT_REVIEW_REQUIRE_CHECKS: bool = true;
const DEFAULT_REVIEW_MERGE_METHOD: &str = "merge";
const DEFAULT_VERIFIER_AUTO_RUN: bool = true;

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

fn cmd_verifier(args: VerifierArgs) -> Result<(), CliError> {
    let dir = args
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !dir.is_dir() {
        return Err(CliError::Message(format!(
            "Directory does not exist: {}",
            dir.display()
        )));
    }

    let config = Config::load(Some(&dir)).map_err(|err| CliError::Message(err.to_string()))?;
    run_verifier_pipeline(
        &dir,
        &config,
        args.test_command,
        args.coverage_command,
        args.coverage_min,
    )
}

fn run_verifier_pipeline(
    dir: &Path,
    config: &Config,
    test_command: Option<String>,
    coverage_command: Option<String>,
    coverage_min: Option<f64>,
) -> Result<(), CliError> {
    let test_command = resolve_verifier_command(
        test_command,
        config,
        "verifier.test_command",
        DEFAULT_TEST_COMMAND,
    )?;
    let coverage_command = resolve_verifier_command(
        coverage_command,
        config,
        "verifier.coverage_command",
        DEFAULT_COVERAGE_COMMAND,
    )?;
    let coverage_min = resolve_verifier_coverage_min(coverage_min, config)?;

    println!("Verifier running in {}", dir.display());

    run_verifier_command("Tests", dir, &test_command)?;
    println!("Tests OK.");

    let coverage_output = run_verifier_command("Coverage", dir, &coverage_command)?;
    let coverage = extract_coverage_percent(&coverage_output).ok_or_else(|| {
        CliError::Message("Coverage output missing percentage value.".to_string())
    })?;
    if coverage + f64::EPSILON < coverage_min {
        return Err(CliError::Message(format!(
            "Coverage {:.2}% below required {:.2}%.",
            coverage, coverage_min
        )));
    }
    println!("Coverage OK: {:.2}% (>= {:.2}%)", coverage, coverage_min);

    run_verifier_static_checks(dir, config)?;
    let pr_url = run_verifier_pr_create(dir, config)?;
    run_verifier_review_gate(dir, config, pr_url.as_deref())?;

    Ok(())
}

fn resolve_verifier_command(
    arg_value: Option<String>,
    config: &Config,
    key: &str,
    default: &str,
) -> Result<String, CliError> {
    let from_args = arg_value.filter(|value| !value.trim().is_empty());
    let from_config = config.get(key).filter(|value| !value.trim().is_empty());
    let command = from_args
        .or(from_config)
        .unwrap_or_else(|| default.to_string());
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(CliError::Message(format!(
            "Verifier command for {} is empty.",
            key
        )));
    }
    Ok(trimmed.to_string())
}

fn resolve_verifier_coverage_min(arg_value: Option<f64>, config: &Config) -> Result<f64, CliError> {
    if let Some(value) = arg_value {
        return validate_coverage_min(value);
    }
    if let Some(value) = config.get("verifier.coverage_min") {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(DEFAULT_COVERAGE_MIN);
        }
        let parsed = trimmed.parse::<f64>().map_err(|_| {
            CliError::Message(format!("Invalid verifier.coverage_min: {}", trimmed))
        })?;
        return validate_coverage_min(parsed);
    }
    Ok(DEFAULT_COVERAGE_MIN)
}

fn validate_coverage_min(value: f64) -> Result<f64, CliError> {
    if !(0.0..=100.0).contains(&value) {
        return Err(CliError::Message(format!(
            "Coverage minimum must be between 0 and 100: {}",
            value
        )));
    }
    Ok(value)
}

fn run_verifier_command(label: &str, dir: &Path, command: &str) -> Result<String, CliError> {
    let (program, args) = parse_verifier_command(command)?;
    println!("\n==> {}", label);
    println!("$ {}", command);

    let output = ProcCommand::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(CliError::Io)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        print!("{}", stdout);
        io::stdout().flush().map_err(CliError::Io)?;
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if !output.status.success() {
        return Err(CliError::Message(format!(
            "{} failed with status {}.",
            label, output.status
        )));
    }

    Ok(format!("{}{}", stdout, stderr))
}

fn parse_verifier_command(command: &str) -> Result<(String, Vec<String>), CliError> {
    let parts = shell_words::split(command)
        .map_err(|err| CliError::Message(format!("Failed to parse command: {}", err)))?;
    if parts.is_empty() {
        return Err(CliError::Message(
            "Verifier command cannot be empty.".to_string(),
        ));
    }
    let program = parts[0].to_string();
    let args = parts[1..].iter().cloned().collect();
    Ok((program, args))
}

fn extract_coverage_percent(output: &str) -> Option<f64> {
    let mut fallback = None;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("coverage results") {
            if let Some(value) = parse_percent_from_line(line) {
                return Some(value);
            }
        }
        if lower.contains("line coverage") {
            if let Some(value) = parse_percent_from_line(line) {
                fallback = Some(value);
            }
            continue;
        }
        if lower.contains("coverage") {
            if let Some(value) = parse_percent_from_line(line) {
                fallback = Some(value);
            }
        }
    }
    fallback
}

fn parse_percent_from_line(line: &str) -> Option<f64> {
    let bytes = line.as_bytes();
    let mut found = None;
    for (idx, ch) in bytes.iter().enumerate() {
        if *ch != b'%' {
            continue;
        }
        let mut start = idx;
        while start > 0 {
            let c = bytes[start - 1];
            if c.is_ascii_digit() || c == b'.' {
                start -= 1;
            } else {
                break;
            }
        }
        if start == idx {
            continue;
        }
        if let Ok(value) = line[start..idx].parse::<f64>() {
            found = Some(value);
        }
    }
    found
}

#[derive(Debug, Clone)]
struct StaticCheckSettings {
    enabled: bool,
    check_todo: bool,
    check_comments: bool,
    check_duplicates: bool,
    allow_patterns: Vec<String>,
    ignore_patterns: Vec<String>,
    todo_markers: Vec<String>,
    max_comment_lines: usize,
    max_comment_chars: usize,
    duplicate_block_lines: usize,
    duplicate_min_alnum_lines: usize,
    max_file_bytes: u64,
}

#[derive(Debug)]
struct StaticViolation {
    path: PathBuf,
    line: usize,
    message: String,
}

#[derive(Debug)]
struct BlockLocation {
    path: PathBuf,
    line: usize,
}

#[derive(Debug)]
struct FileSnapshot {
    path: PathBuf,
    lines: Vec<String>,
}

#[derive(Clone, Copy)]
struct CommentStyle {
    line_prefixes: &'static [&'static str],
    block_start: Option<&'static str>,
    block_end: Option<&'static str>,
}

fn run_verifier_static_checks(dir: &Path, config: &Config) -> Result<(), CliError> {
    println!("\n==> Static checks");
    let settings = resolve_static_check_settings(config)?;
    if !settings.enabled {
        println!("Static checks skipped (disabled).");
        return Ok(());
    }

    let files = collect_static_check_files(dir, &settings)?;
    if files.is_empty() {
        println!("Static checks OK.");
        return Ok(());
    }

    let mut violations = Vec::new();
    let mut snapshots = Vec::new();

    for path in files {
        let Some(contents) = read_text_file(&path, settings.max_file_bytes)? else {
            continue;
        };
        let lines: Vec<String> = contents.lines().map(|line| line.to_string()).collect();
        if settings.check_todo {
            check_todo_markers(&path, &lines, &settings, &mut violations);
        }
        if settings.check_comments {
            check_verbose_comments(&path, &lines, &settings, &mut violations);
        }
        if settings.check_duplicates && is_duplicate_candidate(&path) {
            snapshots.push(FileSnapshot { path, lines });
        }
    }

    if settings.check_duplicates {
        let mut duplicates = find_duplicate_blocks(&snapshots, &settings);
        violations.append(&mut duplicates);
    }

    if violations.is_empty() {
        println!("Static checks OK.");
        return Ok(());
    }

    violations.sort_by(|left, right| {
        let left_path = left.path.to_string_lossy();
        let right_path = right.path.to_string_lossy();
        match left_path.cmp(&right_path) {
            std::cmp::Ordering::Equal => left.line.cmp(&right.line),
            ordering => ordering,
        }
    });

    eprintln!("Static checks failed ({} issue(s)):", violations.len());
    for violation in &violations {
        let display = format_static_violation_path(dir, &violation.path, violation.line);
        eprintln!("  {} {}", display, violation.message);
    }

    Err(CliError::Message(format!(
        "Static checks failed with {} issue(s).",
        violations.len()
    )))
}

fn run_verifier_pr_create(dir: &Path, config: &Config) -> Result<Option<String>, CliError> {
    println!("\n==> PR creation");

    let repo_root_output = git_output_in_dir(dir, ["rev-parse", "--show-toplevel"])?;
    let repo_root = PathBuf::from(repo_root_output.trim());
    if repo_root.as_os_str().is_empty() {
        return Err(CliError::Message(
            "Unable to resolve git repository root.".to_string(),
        ));
    }

    let branch_output = git_output_in_dir(dir, ["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch_output.trim();
    if branch.is_empty() {
        return Err(CliError::Message(
            "Unable to determine current branch.".to_string(),
        ));
    }
    if branch == "HEAD" {
        return Err(CliError::Message(
            "Cannot create PR from detached HEAD.".to_string(),
        ));
    }

    let template_path = resolve_pr_template_path(&repo_root)?;
    let base = resolve_verifier_pr_base(config, &repo_root)?;
    let title = resolve_verifier_pr_title(config)?;

    ensure_gh_authenticated(&repo_root)?;

    let output = run_gh_pr_create(&repo_root, &template_path, branch, &base, &title)?;
    let pr_url = extract_pr_url(&output);
    if let Some(url) = pr_url.as_deref() {
        println!("PR created: {}", url);
    } else if !output.trim().is_empty() {
        println!("{}", output.trim());
    } else {
        println!("PR created.");
    }

    Ok(pr_url)
}

#[derive(Debug, Clone)]
struct ReviewGateSettings {
    enabled: bool,
    reviewer: String,
    min_rating: f64,
    max_issues: usize,
    poll_seconds: u64,
    timeout_seconds: u64,
    require_approval: bool,
    require_checks: bool,
    merge_method: MergeMethod,
}

#[derive(Debug)]
struct ReviewerReview {
    state: String,
    body: String,
    submitted_at: String,
}

#[derive(Debug)]
struct CheckStatus {
    name: String,
    status: String,
    conclusion: String,
}

#[derive(Debug, Clone, Copy)]
enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    fn as_flag(self) -> &'static str {
        match self {
            MergeMethod::Merge => "--merge",
            MergeMethod::Squash => "--squash",
            MergeMethod::Rebase => "--rebase",
        }
    }
}

#[derive(Debug)]
enum GateDecision {
    Pending(String),
    Failed(String),
    Passed(String),
}

impl GateDecision {
    fn is_passed(&self) -> bool {
        matches!(self, GateDecision::Passed(_))
    }

    fn is_failed(&self) -> bool {
        matches!(self, GateDecision::Failed(_))
    }

    fn summary(&self) -> &str {
        match self {
            GateDecision::Pending(message) => message,
            GateDecision::Failed(message) => message,
            GateDecision::Passed(message) => message,
        }
    }
}

fn run_verifier_review_gate(
    dir: &Path,
    config: &Config,
    pr_url: Option<&str>,
) -> Result<(), CliError> {
    println!("\n==> Review gate");
    let settings = resolve_review_gate_settings(config)?;
    if !settings.enabled {
        println!("Review gate skipped (disabled).");
        return Ok(());
    }

    let repo_root_output = git_output_in_dir(dir, ["rev-parse", "--show-toplevel"])?;
    let repo_root = PathBuf::from(repo_root_output.trim());
    if repo_root.as_os_str().is_empty() {
        return Err(CliError::Message(
            "Unable to resolve git repository root.".to_string(),
        ));
    }

    ensure_gh_authenticated(&repo_root)?;

    if let Some(url) = pr_url {
        println!("Review gate watching: {}", url);
    }

    let deadline = Instant::now() + Duration::from_secs(settings.timeout_seconds);
    let mut last_status = String::new();

    loop {
        let pr_view = gh_pr_view_json(&repo_root)?;
        let review_decision = evaluate_review_gate(&pr_view, &settings)?;
        let check_decision = evaluate_check_gate(&pr_view, &settings)?;

        if review_decision.is_failed() {
            return Err(CliError::Message(review_decision.summary().to_string()));
        }
        if check_decision.is_failed() {
            return Err(CliError::Message(check_decision.summary().to_string()));
        }
        if review_decision.is_passed() && check_decision.is_passed() {
            run_gh_pr_merge(&repo_root, settings.merge_method)?;
            println!("PR merged.");
            return Ok(());
        }

        let status = format!(
            "review: {} | checks: {}",
            review_decision.summary(),
            check_decision.summary()
        );
        if status != last_status {
            println!("{}", status);
            last_status = status;
        }

        if Instant::now() >= deadline {
            return Err(CliError::Message(format!(
                "Review gate timed out after {}s.",
                settings.timeout_seconds
            )));
        }

        thread::sleep(Duration::from_secs(settings.poll_seconds));
    }
}

fn resolve_review_gate_settings(config: &Config) -> Result<ReviewGateSettings, CliError> {
    let enabled =
        resolve_review_gate_bool(config, "verifier.review.enabled", DEFAULT_REVIEW_ENABLED)?;
    let reviewer = config
        .get("verifier.review.reviewer")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_REVIEW_REVIEWER.to_string());
    let min_rating = resolve_review_gate_rating(
        config.get("verifier.review.min_rating"),
        DEFAULT_REVIEW_MIN_RATING,
    )?;
    let max_issues = resolve_review_gate_usize(
        config.get("verifier.review.max_issues"),
        "verifier.review.max_issues",
        DEFAULT_REVIEW_MAX_ISSUES,
        0,
    )?;
    let poll_seconds = resolve_review_gate_u64(
        config.get("verifier.review.poll_seconds"),
        "verifier.review.poll_seconds",
        DEFAULT_REVIEW_POLL_SECONDS,
        5,
    )?;
    let timeout_seconds = resolve_review_gate_u64(
        config.get("verifier.review.timeout_seconds"),
        "verifier.review.timeout_seconds",
        DEFAULT_REVIEW_TIMEOUT_SECONDS,
        30,
    )?;
    let require_approval = resolve_review_gate_bool(
        config,
        "verifier.review.require_approval",
        DEFAULT_REVIEW_REQUIRE_APPROVAL,
    )?;
    let require_checks = resolve_review_gate_bool(
        config,
        "verifier.review.require_checks",
        DEFAULT_REVIEW_REQUIRE_CHECKS,
    )?;
    let merge_method =
        resolve_review_gate_merge_method(config.get("verifier.review.merge_method"))?;

    Ok(ReviewGateSettings {
        enabled,
        reviewer,
        min_rating,
        max_issues,
        poll_seconds,
        timeout_seconds,
        require_approval,
        require_checks,
        merge_method,
    })
}

fn resolve_review_gate_bool(config: &Config, key: &str, default: bool) -> Result<bool, CliError> {
    let Some(value) = config.get(key) else {
        return Ok(default);
    };
    if value.trim().is_empty() {
        return Ok(default);
    }
    parse_bool_value(&value)
        .ok_or_else(|| CliError::Message(format!("Invalid {}: {}", key, value.trim())))
}

fn resolve_review_gate_u64(
    value: Option<String>,
    key: &str,
    default: u64,
    min: u64,
) -> Result<u64, CliError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let parsed = trimmed
        .parse::<u64>()
        .map_err(|_| CliError::Message(format!("Invalid {}: {}", key, trimmed)))?;
    if parsed < min {
        return Err(CliError::Message(format!(
            "Invalid {}: {} (minimum {})",
            key, parsed, min
        )));
    }
    Ok(parsed)
}

fn resolve_review_gate_usize(
    value: Option<String>,
    key: &str,
    default: usize,
    min: usize,
) -> Result<usize, CliError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let parsed = trimmed
        .parse::<usize>()
        .map_err(|_| CliError::Message(format!("Invalid {}: {}", key, trimmed)))?;
    if parsed < min {
        return Err(CliError::Message(format!(
            "Invalid {}: {} (minimum {})",
            key, parsed, min
        )));
    }
    Ok(parsed)
}

fn resolve_review_gate_rating(value: Option<String>, default: f64) -> Result<f64, CliError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let parsed = trimmed.parse::<f64>().map_err(|_| {
        CliError::Message(format!("Invalid verifier.review.min_rating: {}", trimmed))
    })?;
    if !(0.0..=100.0).contains(&parsed) {
        return Err(CliError::Message(format!(
            "Invalid verifier.review.min_rating: {}",
            parsed
        )));
    }
    if parsed > 10.0 {
        return Ok(parsed / 10.0);
    }
    Ok(parsed)
}

fn resolve_review_gate_merge_method(value: Option<String>) -> Result<MergeMethod, CliError> {
    let method = value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_REVIEW_MERGE_METHOD);
    match method.to_ascii_lowercase().as_str() {
        "merge" => Ok(MergeMethod::Merge),
        "squash" => Ok(MergeMethod::Squash),
        "rebase" => Ok(MergeMethod::Rebase),
        _ => Err(CliError::Message(format!(
            "Invalid verifier.review.merge_method: {}",
            method
        ))),
    }
}

fn gh_pr_view_json(repo_root: &Path) -> Result<serde_json::Value, CliError> {
    let output = ProcCommand::new("gh")
        .arg("pr")
        .arg("view")
        .arg("--json")
        .arg("url,number,reviews,reviewDecision,statusCheckRollup")
        .current_dir(repo_root)
        .output()
        .map_err(map_gh_error)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let combined = format!("{}{}", stdout, stderr);
        let trimmed = combined.trim();
        return Err(CliError::Message(if trimmed.is_empty() {
            "gh pr view failed.".to_string()
        } else {
            format!("gh pr view failed: {}", trimmed)
        }));
    }
    serde_json::from_str(stdout.trim())
        .map_err(|err| CliError::Message(format!("Unable to parse gh pr view output: {}", err)))
}

fn evaluate_review_gate(
    pr_view: &serde_json::Value,
    settings: &ReviewGateSettings,
) -> Result<GateDecision, CliError> {
    let reviews = pr_view
        .get("reviews")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let review = find_reviewer_review(&reviews, &settings.reviewer);
    let Some(review) = review else {
        return Ok(GateDecision::Pending(format!(
            "waiting for {} review",
            settings.reviewer
        )));
    };

    if review.state.eq_ignore_ascii_case("CHANGES_REQUESTED") {
        return Ok(GateDecision::Failed(format!(
            "{} requested changes",
            settings.reviewer
        )));
    }

    if settings.require_approval && !review.state.eq_ignore_ascii_case("APPROVED") {
        return Ok(GateDecision::Pending(format!(
            "waiting for {} approval",
            settings.reviewer
        )));
    }

    let rating = parse_review_rating(&review.body);
    if let Some(rating) = rating {
        if rating + f64::EPSILON < settings.min_rating {
            return Ok(GateDecision::Failed(format!(
                "{} rating {:.2} below {:.2}",
                settings.reviewer, rating, settings.min_rating
            )));
        }
    } else {
        return Ok(GateDecision::Pending(format!(
            "waiting for {} rating",
            settings.reviewer
        )));
    }

    if let Some(issue_count) = parse_review_issue_count(&review.body) {
        if issue_count > settings.max_issues {
            return Ok(GateDecision::Failed(format!(
                "{} flagged {} issue(s)",
                settings.reviewer, issue_count
            )));
        }
    }

    Ok(GateDecision::Passed(format!(
        "{} review ok",
        settings.reviewer
    )))
}

fn evaluate_check_gate(
    pr_view: &serde_json::Value,
    settings: &ReviewGateSettings,
) -> Result<GateDecision, CliError> {
    if !settings.require_checks {
        return Ok(GateDecision::Passed("checks skipped".to_string()));
    }

    let checks = extract_check_rollup(pr_view);
    if checks.is_empty() {
        return Ok(GateDecision::Passed("no checks".to_string()));
    }

    let mut pending = Vec::new();
    let mut failed = Vec::new();

    for check in checks {
        let status = check.status.to_ascii_uppercase();
        let conclusion = check.conclusion.to_ascii_uppercase();
        if status.is_empty() || status == "PENDING" || status == "IN_PROGRESS" || status == "QUEUED"
        {
            pending.push(check.name);
            continue;
        }
        if conclusion.is_empty() {
            pending.push(check.name);
            continue;
        }
        if matches!(
            conclusion.as_str(),
            "FAILURE" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "STALE"
        ) {
            failed.push(check.name);
        }
    }

    if !failed.is_empty() {
        return Ok(GateDecision::Failed(format!(
            "checks failed: {}",
            join_or_none(&failed)
        )));
    }
    if !pending.is_empty() {
        return Ok(GateDecision::Pending(format!(
            "checks pending: {}",
            join_or_none(&pending)
        )));
    }

    Ok(GateDecision::Passed("checks ok".to_string()))
}

fn find_reviewer_review(reviews: &[serde_json::Value], reviewer: &str) -> Option<ReviewerReview> {
    let mut latest: Option<ReviewerReview> = None;
    for review in reviews {
        let author = review
            .get("author")
            .and_then(|value| value.get("login"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if !author.eq_ignore_ascii_case(reviewer) {
            continue;
        }
        let state = review
            .get("state")
            .and_then(|value| value.as_str())
            .unwrap_or("COMMENTED")
            .to_string();
        let body = review
            .get("body")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let submitted_at = review
            .get("submittedAt")
            .or_else(|| review.get("createdAt"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let candidate = ReviewerReview {
            state,
            body,
            submitted_at: submitted_at.clone(),
        };
        let replace = latest
            .as_ref()
            .map(|existing| submitted_at >= existing.submitted_at)
            .unwrap_or(true);
        if replace {
            latest = Some(candidate);
        }
    }
    latest
}

fn parse_review_rating(body: &str) -> Option<f64> {
    let rating_from_fraction = parse_fraction_rating(body);
    if rating_from_fraction.is_some() {
        return rating_from_fraction;
    }
    let lower = body.to_ascii_lowercase();
    for line in lower.lines() {
        if line.contains("rating") || line.contains("score") || line.contains("quality") {
            if let Some(value) = parse_first_number(line) {
                return Some(scale_rating_value(value, line));
            }
        }
    }
    None
}

fn scale_rating_value(value: f64, line: &str) -> f64 {
    if line.contains('%') {
        return value / 10.0;
    }
    if value <= 1.0 {
        return value * 10.0;
    }
    if value > 10.0 {
        return value / 10.0;
    }
    value
}

fn parse_fraction_rating(body: &str) -> Option<f64> {
    let bytes = body.as_bytes();
    for (idx, ch) in bytes.iter().enumerate() {
        if *ch != b'/' {
            continue;
        }
        let mut right = idx + 1;
        while right < bytes.len() && bytes[right].is_ascii_whitespace() {
            right += 1;
        }
        let Some((denom, _)) = parse_number_forward(bytes, right) else {
            continue;
        };
        if denom <= 0.0 {
            continue;
        }
        let mut left = idx;
        while left > 0 && bytes[left - 1].is_ascii_whitespace() {
            left -= 1;
        }
        let Some(numerator) = parse_number_backward(bytes, left) else {
            continue;
        };
        if denom == 5.0 || denom == 10.0 || denom == 100.0 {
            return Some(numerator * 10.0 / denom);
        }
    }
    None
}

fn parse_review_issue_count(body: &str) -> Option<usize> {
    let lower = body.to_ascii_lowercase();
    if lower.contains("no issues") || lower.contains("no issue") {
        return Some(0);
    }
    for line in lower.lines() {
        if line.contains("issue") {
            if let Some(value) = parse_first_usize(line) {
                return Some(value);
            }
        }
    }
    None
}

fn parse_number_forward(bytes: &[u8], start: usize) -> Option<(f64, usize)> {
    let mut end = start;
    while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
        end += 1;
    }
    if end == start {
        return None;
    }
    let value = std::str::from_utf8(&bytes[start..end])
        .ok()?
        .parse::<f64>()
        .ok()?;
    Some((value, end))
}

fn parse_number_backward(bytes: &[u8], end: usize) -> Option<f64> {
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
        start -= 1;
    }
    if start == end {
        return None;
    }
    std::str::from_utf8(&bytes[start..end])
        .ok()?
        .parse::<f64>()
        .ok()
}

fn parse_first_number(line: &str) -> Option<f64> {
    let bytes = line.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx].is_ascii_digit() {
            let (value, _) = parse_number_forward(bytes, idx)?;
            return Some(value);
        }
        idx += 1;
    }
    None
}

fn parse_first_usize(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx].is_ascii_digit() {
            let mut end = idx + 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let value = std::str::from_utf8(&bytes[idx..end])
                .ok()?
                .parse::<usize>()
                .ok()?;
            return Some(value);
        }
        idx += 1;
    }
    None
}

fn extract_check_rollup(pr_view: &serde_json::Value) -> Vec<CheckStatus> {
    let mut checks = Vec::new();
    let Some(items) = pr_view
        .get("statusCheckRollup")
        .and_then(|value| value.as_array())
    else {
        return checks;
    };

    for item in items {
        let name = item
            .get("name")
            .or_else(|| item.get("context"))
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        let status = item
            .get("status")
            .or_else(|| item.get("state"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let conclusion = item
            .get("conclusion")
            .or_else(|| item.get("result"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        checks.push(CheckStatus {
            name,
            status,
            conclusion,
        });
    }
    checks
}

fn run_gh_pr_merge(repo_root: &Path, method: MergeMethod) -> Result<(), CliError> {
    println!("$ gh pr merge {}", method.as_flag());
    let output = ProcCommand::new("gh")
        .arg("pr")
        .arg("merge")
        .arg(method.as_flag())
        .current_dir(repo_root)
        .output()
        .map_err(map_gh_error)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let combined = format!("{}{}", stdout, stderr);
        let trimmed = combined.trim();
        let message = if trimmed.is_empty() {
            "gh pr merge failed.".to_string()
        } else {
            format!("gh pr merge failed: {}", trimmed)
        };
        return Err(CliError::Message(message));
    }
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }
    if !stderr.trim().is_empty() {
        eprintln!("{}", stderr.trim());
    }
    Ok(())
}

fn resolve_verifier_pr_base(config: &Config, repo_root: &Path) -> Result<String, CliError> {
    let from_config = config
        .get("verifier.pr.base")
        .filter(|value| !value.trim().is_empty());
    if let Some(value) = from_config {
        return Ok(value.trim().to_string());
    }
    if let Some(value) = detect_default_base_branch(repo_root) {
        return Ok(value);
    }
    Ok(DEFAULT_PR_BASE.to_string())
}

fn detect_default_base_branch(repo_root: &Path) -> Option<String> {
    let output = git_output_in_dir(
        repo_root,
        ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .ok()?;
    let trimmed = output.trim();
    if let Some(stripped) = trimmed.strip_prefix("origin/") {
        if !stripped.is_empty() {
            return Some(stripped.to_string());
        }
    }
    None
}

fn resolve_verifier_pr_title(config: &Config) -> Result<String, CliError> {
    let title = config
        .get("verifier.pr.title")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PR_TITLE.to_string());
    Ok(title.trim().to_string())
}

fn resolve_pr_template_path(repo_root: &Path) -> Result<PathBuf, CliError> {
    let candidates = [
        repo_root.join(".github").join("pull_request_template.md"),
        repo_root.join(".github").join("PULL_REQUEST_TEMPLATE.md"),
        repo_root.join("pull_request_template.md"),
        repo_root.join("PULL_REQUEST_TEMPLATE.md"),
    ];
    for path in candidates {
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(CliError::Message(
        "PR template not found in repository.".to_string(),
    ))
}

fn ensure_gh_authenticated(dir: &Path) -> Result<(), CliError> {
    let output = ProcCommand::new("gh")
        .arg("auth")
        .arg("status")
        .current_dir(dir)
        .output()
        .map_err(map_gh_error)?;
    if output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    let trimmed = combined.trim();
    if trimmed.is_empty() {
        Err(CliError::Message(
            "gh auth status failed. Run `gh auth login`.".to_string(),
        ))
    } else {
        Err(CliError::Message(format!(
            "gh auth status failed: {}. Run `gh auth login`.",
            trimmed
        )))
    }
}

fn run_gh_pr_create(
    repo_root: &Path,
    template_path: &Path,
    head: &str,
    base: &str,
    title: &str,
) -> Result<String, CliError> {
    println!(
        "$ gh pr create --base {} --head {} --title {} --body-file {}",
        base,
        head,
        title,
        template_path.display()
    );
    let output = ProcCommand::new("gh")
        .arg("pr")
        .arg("create")
        .arg("--base")
        .arg(base)
        .arg("--head")
        .arg(head)
        .arg("--title")
        .arg(title)
        .arg("--body-file")
        .arg(template_path)
        .current_dir(repo_root)
        .output()
        .map_err(map_gh_error)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let combined = format!("{}{}", stdout, stderr);
        let trimmed = combined.trim();
        let message = if trimmed.is_empty() {
            "gh pr create failed.".to_string()
        } else {
            format!("gh pr create failed: {}", trimmed)
        };
        return Err(CliError::Message(message));
    }
    Ok(format!("{}{}", stdout, stderr))
}

fn map_gh_error(err: io::Error) -> CliError {
    if err.kind() == io::ErrorKind::NotFound {
        CliError::Message("gh CLI not found. Install from https://cli.github.com/.".to_string())
    } else {
        CliError::Io(err)
    }
}

fn extract_pr_url(output: &str) -> Option<String> {
    for token in output.split_whitespace() {
        if token.starts_with("https://") || token.starts_with("http://") {
            let trimmed = token.trim_matches(|c: char| c == ')' || c == ',' || c == ';');
            return Some(trimmed.to_string());
        }
    }
    None
}

fn resolve_static_check_settings(config: &Config) -> Result<StaticCheckSettings, CliError> {
    let enabled = resolve_static_check_bool(config, "verifier.static_checks.enabled", true)?;
    let check_todo = resolve_static_check_bool(config, "verifier.static_checks.todo", true)?;
    let check_comments =
        resolve_static_check_bool(config, "verifier.static_checks.comments", true)?;
    let check_duplicates =
        resolve_static_check_bool(config, "verifier.static_checks.duplicate", true)?;
    let allow_patterns = resolve_static_check_patterns(
        config.get("verifier.static_checks.allow"),
        default_static_allow_patterns(),
    );
    let ignore_patterns = resolve_static_check_patterns(
        config.get("verifier.static_checks.ignore"),
        default_static_ignore_patterns(),
    );
    let todo_markers = resolve_static_check_markers(
        config.get("verifier.static_checks.todo_markers"),
        vec!["TODO".to_string(), "FIXME".to_string()],
    );
    let max_comment_lines = resolve_static_check_usize(
        config.get("verifier.static_checks.max_comment_lines"),
        "verifier.static_checks.max_comment_lines",
        DEFAULT_STATIC_MAX_COMMENT_LINES,
        1,
    )?;
    let max_comment_chars = resolve_static_check_usize(
        config.get("verifier.static_checks.max_comment_chars"),
        "verifier.static_checks.max_comment_chars",
        DEFAULT_STATIC_MAX_COMMENT_CHARS,
        1,
    )?;
    let duplicate_block_lines = resolve_static_check_usize(
        config.get("verifier.static_checks.duplicate_block_lines"),
        "verifier.static_checks.duplicate_block_lines",
        DEFAULT_STATIC_DUPLICATE_BLOCK_LINES,
        2,
    )?;
    let duplicate_min_alnum_lines = resolve_static_check_usize(
        config.get("verifier.static_checks.duplicate_min_alnum_lines"),
        "verifier.static_checks.duplicate_min_alnum_lines",
        DEFAULT_STATIC_DUPLICATE_MIN_ALNUM_LINES,
        1,
    )?;
    let max_file_bytes = resolve_static_check_u64(
        config.get("verifier.static_checks.max_file_bytes"),
        "verifier.static_checks.max_file_bytes",
        DEFAULT_STATIC_MAX_FILE_BYTES,
        64,
    )?;

    Ok(StaticCheckSettings {
        enabled,
        check_todo,
        check_comments,
        check_duplicates,
        allow_patterns,
        ignore_patterns,
        todo_markers,
        max_comment_lines,
        max_comment_chars,
        duplicate_block_lines,
        duplicate_min_alnum_lines,
        max_file_bytes,
    })
}

fn resolve_static_check_bool(config: &Config, key: &str, default: bool) -> Result<bool, CliError> {
    let Some(value) = config.get(key) else {
        return Ok(default);
    };
    if value.trim().is_empty() {
        return Ok(default);
    }
    parse_bool_value(&value)
        .ok_or_else(|| CliError::Message(format!("Invalid {}: {}", key, value.trim())))
}

fn resolve_static_check_usize(
    value: Option<String>,
    key: &str,
    default: usize,
    min: usize,
) -> Result<usize, CliError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let parsed = trimmed
        .parse::<usize>()
        .map_err(|_| CliError::Message(format!("Invalid {}: {}", key, trimmed)))?;
    if parsed < min {
        return Err(CliError::Message(format!(
            "Invalid {}: {} (minimum {})",
            key, parsed, min
        )));
    }
    Ok(parsed)
}

fn resolve_static_check_u64(
    value: Option<String>,
    key: &str,
    default: u64,
    min: u64,
) -> Result<u64, CliError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let parsed = trimmed
        .parse::<u64>()
        .map_err(|_| CliError::Message(format!("Invalid {}: {}", key, trimmed)))?;
    if parsed < min {
        return Err(CliError::Message(format!(
            "Invalid {}: {} (minimum {})",
            key, parsed, min
        )));
    }
    Ok(parsed)
}

fn resolve_static_check_patterns(value: Option<String>, default: Vec<String>) -> Vec<String> {
    let parsed = value
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(normalize_csv)
        .unwrap_or_default();
    if parsed.is_empty() {
        default
            .into_iter()
            .map(|pattern| normalize_pattern(&pattern))
            .collect()
    } else {
        parsed
            .into_iter()
            .map(|pattern| normalize_pattern(&pattern))
            .collect()
    }
}

fn resolve_static_check_markers(value: Option<String>, default: Vec<String>) -> Vec<String> {
    let parsed = value
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(normalize_csv)
        .unwrap_or_default();
    let markers = if parsed.is_empty() { default } else { parsed };
    let mut unique = BTreeMap::new();
    for marker in markers {
        let trimmed = marker.trim();
        if trimmed.is_empty() {
            continue;
        }
        unique.insert(trimmed.to_ascii_uppercase(), true);
    }
    unique.keys().cloned().collect()
}

fn default_static_allow_patterns() -> Vec<String> {
    vec![
        "**/*.rs".to_string(),
        "**/*.md".to_string(),
        "**/*.toml".to_string(),
        "**/*.yaml".to_string(),
        "**/*.yml".to_string(),
        "**/*.json".to_string(),
        "**/*.js".to_string(),
        "**/*.ts".to_string(),
        "**/*.tsx".to_string(),
        "**/*.jsx".to_string(),
        "**/*.py".to_string(),
        "**/*.go".to_string(),
        "**/*.java".to_string(),
        "**/*.c".to_string(),
        "**/*.h".to_string(),
        "**/*.hpp".to_string(),
        "**/*.cpp".to_string(),
        "**/*.cc".to_string(),
        "**/*.cs".to_string(),
        "**/*.sh".to_string(),
        "**/*.ps1".to_string(),
        "**/*.txt".to_string(),
        "**/Dockerfile".to_string(),
        "**/Makefile".to_string(),
    ]
}

fn default_static_ignore_patterns() -> Vec<String> {
    vec![
        "**/.git/**".to_string(),
        "**/.worktrees/**".to_string(),
        "**/.gralph/**".to_string(),
        "**/target/**".to_string(),
        "**/node_modules/**".to_string(),
        "**/dist/**".to_string(),
        "**/build/**".to_string(),
    ]
}

fn normalize_pattern(pattern: &str) -> String {
    let mut value = pattern.trim().replace('\\', "/");
    while value.starts_with("./") {
        value = value.trim_start_matches("./").to_string();
    }
    while value.starts_with('/') {
        value = value.trim_start_matches('/').to_string();
    }
    value
}

fn normalize_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_static_check_files(
    root: &Path,
    settings: &StaticCheckSettings,
) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(CliError::Io)?;
        for entry in entries {
            let entry = entry.map_err(CliError::Io)?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(CliError::Io)?;
            if file_type.is_symlink() {
                continue;
            }
            let rel = normalize_relative_path(root, &path);
            if file_type.is_dir() {
                if path_is_ignored(&rel, true, &settings.ignore_patterns) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            if !path_is_allowed(&rel, &settings.allow_patterns) {
                continue;
            }
            if path_is_ignored(&rel, false, &settings.ignore_patterns) {
                continue;
            }
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn path_is_allowed(path: &str, allow_patterns: &[String]) -> bool {
    if allow_patterns.is_empty() {
        return true;
    }
    path_matches_any(path, allow_patterns)
}

fn path_is_ignored(path: &str, is_dir: bool, ignore_patterns: &[String]) -> bool {
    if path_matches_any(path, ignore_patterns) {
        return true;
    }
    if is_dir {
        let with_slash = format!("{}/", path);
        if path_matches_any(&with_slash, ignore_patterns) {
            return true;
        }
    }
    false
}

fn path_matches_any(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if wildcard_match(pattern, path) {
            return true;
        }
        if let Some(stripped) = pattern.strip_prefix("**/") {
            if wildcard_match(stripped, path) {
                return true;
            }
        }
    }
    false
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let mut pi = 0;
    let mut ti = 0;
    let mut star = None;
    let mut match_index = 0;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            match_index = ti;
            pi += 1;
        } else if let Some(star_idx) = star {
            pi = star_idx + 1;
            match_index += 1;
            ti = match_index;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

fn read_text_file(path: &Path, max_bytes: u64) -> Result<Option<String>, CliError> {
    let metadata = fs::metadata(path).map_err(CliError::Io)?;
    if metadata.len() > max_bytes {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(CliError::Io)?;
    if bytes.len() as u64 > max_bytes {
        return Ok(None);
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

fn check_todo_markers(
    path: &Path,
    lines: &[String],
    settings: &StaticCheckSettings,
    violations: &mut Vec<StaticViolation>,
) {
    for (index, line) in lines.iter().enumerate() {
        if let Some(marker) = line_contains_marker(line, &settings.todo_markers) {
            violations.push(StaticViolation {
                path: path.to_path_buf(),
                line: index + 1,
                message: format!("Found {} marker.", marker),
            });
        }
    }
}

fn line_contains_marker(line: &str, markers: &[String]) -> Option<String> {
    if markers.is_empty() {
        return None;
    }
    let upper = line.to_ascii_uppercase();
    for marker in markers {
        let mut offset = 0;
        while let Some(pos) = upper[offset..].find(marker) {
            let start = offset + pos;
            let end = start + marker.len();
            let before = upper[..start].chars().last();
            let after = upper[end..].chars().next();
            let before_ok = before
                .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(true);
            let after_ok = after
                .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(true);
            if before_ok && after_ok {
                return Some(marker.clone());
            }
            offset = end;
        }
    }
    None
}

fn check_verbose_comments(
    path: &Path,
    lines: &[String],
    settings: &StaticCheckSettings,
    violations: &mut Vec<StaticViolation>,
) {
    let Some(style) = comment_style_for_path(path) else {
        return;
    };
    let mut in_block = false;
    let mut block_start_line = 0;
    let mut block_lines = 0;
    let mut block_chars = 0;

    for (index, line) in lines.iter().enumerate() {
        let line_no = index + 1;
        let trimmed = line.trim_start();
        let mut is_comment = false;
        if in_block {
            is_comment = true;
            if let Some(end) = style.block_end {
                if trimmed.contains(end) {
                    in_block = false;
                }
            }
        } else if let Some(start) = style.block_start {
            if trimmed.starts_with(start) {
                is_comment = true;
                if let Some(end) = style.block_end {
                    if !trimmed.contains(end) {
                        in_block = true;
                    }
                }
            }
        }

        if !is_comment
            && style
                .line_prefixes
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
        {
            is_comment = true;
        }

        if is_comment {
            if block_lines == 0 {
                block_start_line = line_no;
            }
            block_lines += 1;
            block_chars += comment_text_len(trimmed, &style);
        } else if block_lines > 0 {
            record_verbose_comment(
                path,
                block_start_line,
                block_lines,
                block_chars,
                settings,
                violations,
            );
            block_lines = 0;
            block_chars = 0;
        }
    }

    if block_lines > 0 {
        record_verbose_comment(
            path,
            block_start_line,
            block_lines,
            block_chars,
            settings,
            violations,
        );
    }
}

fn record_verbose_comment(
    path: &Path,
    start_line: usize,
    block_lines: usize,
    block_chars: usize,
    settings: &StaticCheckSettings,
    violations: &mut Vec<StaticViolation>,
) {
    if block_lines <= settings.max_comment_lines && block_chars <= settings.max_comment_chars {
        return;
    }
    violations.push(StaticViolation {
        path: path.to_path_buf(),
        line: start_line,
        message: format!(
            "Verbose comment block ({} lines, {} chars) exceeds limits ({} lines, {} chars).",
            block_lines, block_chars, settings.max_comment_lines, settings.max_comment_chars
        ),
    });
}

fn comment_style_for_path(path: &Path) -> Option<CommentStyle> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    match ext {
        "rs" | "js" | "ts" | "tsx" | "jsx" | "c" | "cc" | "cpp" | "h" | "hpp" | "java" | "go"
        | "cs" => Some(CommentStyle {
            line_prefixes: &["//"],
            block_start: Some("/*"),
            block_end: Some("*/"),
        }),
        "py" | "rb" | "sh" | "bash" | "zsh" | "yaml" | "yml" | "toml" | "ini" | "ps1" => {
            Some(CommentStyle {
                line_prefixes: &["#"],
                block_start: None,
                block_end: None,
            })
        }
        "sql" => Some(CommentStyle {
            line_prefixes: &["--"],
            block_start: Some("/*"),
            block_end: Some("*/"),
        }),
        _ => None,
    }
}

fn comment_text_len(line: &str, style: &CommentStyle) -> usize {
    let trimmed = line.trim_start();
    for prefix in style.line_prefixes {
        if trimmed.starts_with(prefix) {
            return trimmed[prefix.len()..].trim_start().len();
        }
    }
    if let Some(start) = style.block_start {
        if trimmed.starts_with(start) {
            return trimmed[start.len()..].trim_start().len();
        }
    }
    if let Some(end) = style.block_end {
        if trimmed.starts_with(end) {
            return trimmed[end.len()..].trim_start().len();
        }
    }
    if trimmed.starts_with('*') {
        return trimmed[1..].trim_start().len();
    }
    trimmed.len()
}

fn is_duplicate_candidate(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    matches!(
        ext,
        "rs" | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "py"
            | "go"
            | "java"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
    )
}

fn find_duplicate_blocks(
    snapshots: &[FileSnapshot],
    settings: &StaticCheckSettings,
) -> Vec<StaticViolation> {
    let mut seen: HashMap<String, BlockLocation> = HashMap::new();
    let mut violations = Vec::new();

    for snapshot in snapshots {
        for (start_line, block) in split_nonempty_blocks(&snapshot.lines) {
            if block.len() < settings.duplicate_block_lines {
                continue;
            }
            if !block_is_substantive(&block, settings.duplicate_min_alnum_lines) {
                continue;
            }
            let normalized: Vec<String> = block
                .iter()
                .map(|line| normalize_line_for_duplicate(line))
                .collect();
            let key = normalized.join("\n");
            if key.trim().is_empty() {
                continue;
            }
            if let Some(existing) = seen.get(&key) {
                violations.push(StaticViolation {
                    path: snapshot.path.to_path_buf(),
                    line: start_line,
                    message: format!(
                        "Duplicate block matches {}:{}.",
                        existing.path.display(),
                        existing.line
                    ),
                });
            } else {
                seen.insert(
                    key,
                    BlockLocation {
                        path: snapshot.path.to_path_buf(),
                        line: start_line,
                    },
                );
            }
        }
    }

    violations
}

fn split_nonempty_blocks(lines: &[String]) -> Vec<(usize, Vec<String>)> {
    let mut blocks = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut start_line = 0;

    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push((start_line, current));
                current = Vec::new();
            }
            continue;
        }
        if current.is_empty() {
            start_line = index + 1;
        }
        current.push(line.to_string());
    }

    if !current.is_empty() {
        blocks.push((start_line, current));
    }

    blocks
}

fn block_is_substantive(lines: &[String], min_alnum_lines: usize) -> bool {
    let mut count = 0;
    for line in lines {
        if line.chars().any(|ch| ch.is_ascii_alphanumeric()) {
            count += 1;
        }
    }
    count >= min_alnum_lines
}

fn normalize_line_for_duplicate(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_static_violation_path(root: &Path, path: &Path, line: usize) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    format!("{}:{}", rel.display(), line)
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

    if outcome.status == LoopStatus::Complete && resolve_verifier_auto_run(&config) {
        store
            .set_session(
                &args.name,
                &[("status", "verifying"), ("last_task_count", "0")],
            )
            .map_err(|err| CliError::Message(err.to_string()))?;
        if let Err(err) = run_verifier_pipeline(&args.dir, &config, None, None, None) {
            let _ = store.set_session(&args.name, &[("status", "verify-failed")]);
            return Err(err);
        }
        store
            .set_session(
                &args.name,
                &[("status", "verified"), ("last_task_count", "0")],
            )
            .map_err(|err| CliError::Message(err.to_string()))?;
    }

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

fn resolve_verifier_auto_run(config: &Config) -> bool {
    config
        .get("verifier.auto_run")
        .as_deref()
        .and_then(parse_bool_value)
        .unwrap_or(DEFAULT_VERIFIER_AUTO_RUN)
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
    let timestamp = worktree_timestamp_slug();
    maybe_create_auto_worktree_with_timestamp(args, config, &timestamp)
}

fn maybe_create_auto_worktree_with_timestamp(
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
    fn auto_worktree_skips_dirty_repo() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        write_file(&temp.path().join("README.md"), "dirty");
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());
        let original = args.dir.clone();

        maybe_create_auto_worktree(&mut args, &config).unwrap();

        assert_eq!(args.dir, original);
        assert!(!args.no_worktree);
        assert!(!temp.path().join(".worktrees").exists());
    }

    #[test]
    fn auto_worktree_creates_worktree_for_clean_repo() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());

        maybe_create_auto_worktree(&mut args, &config).unwrap();

        let worktrees_dir = temp.path().join(".worktrees");
        let mut entries: Vec<PathBuf> = fs::read_dir(&worktrees_dir)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        assert_eq!(entries.len(), 1);
        let worktree_path = entries.remove(0);
        let expected = fs::canonicalize(&worktree_path).unwrap();
        let actual = fs::canonicalize(&args.dir).unwrap();
        assert_eq!(actual, expected);
        assert!(args.no_worktree);
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
    fn auto_worktree_handles_branch_and_path_collisions() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();

        let timestamp = "20260126-120000";
        let base_branch = auto_worktree_branch_name(&args.name, timestamp);
        git_status_ok(temp.path(), &["branch", base_branch.as_str()]);
        fs::create_dir_all(worktrees_dir.join(&base_branch)).unwrap();
        fs::create_dir_all(worktrees_dir.join(format!("{}-2", base_branch))).unwrap();

        maybe_create_auto_worktree_with_timestamp(&mut args, &config, timestamp).unwrap();

        let expected_branch = format!("{}-3", base_branch);
        let expected_path = worktrees_dir.join(&expected_branch);
        let expected = fs::canonicalize(&expected_path).unwrap();
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
