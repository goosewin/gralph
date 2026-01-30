use crate::backend::backend_from_name;
use crate::cli::{
    self, Cli, Command, ConfigArgs, ConfigCommand, InitArgs, PrdArgs, PrdCheckArgs, PrdCommand,
    PrdCreateArgs, RunLoopArgs, ServerArgs, VerifierArgs, WorktreeCommand, WorktreeCreateArgs,
    WorktreeFinishArgs, ASCII_BANNER,
};
use crate::config::Config;
use crate::prd;
use crate::server::{self, ServerConfig};
use crate::state::StateStore;
use crate::update;
use crate::verifier;
use crate::version;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;
use std::process::ExitCode;

mod loop_session;

#[derive(Default)]
pub struct Deps;

impl Deps {
    pub fn real() -> Self {
        Self
    }

    pub fn state_store(&self) -> StateStore {
        StateStore::new_from_env()
    }
}

pub fn run(cli: Cli, deps: &Deps) -> Result<(), CliError> {
    let Some(command) = cli.command else {
        cmd_intro()?;
        return Ok(());
    };
    dispatch(command, deps)
}

pub fn exit_code_for(result: Result<(), CliError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {}", err);
            ExitCode::FAILURE
        }
    }
}

fn dispatch(command: Command, deps: &Deps) -> Result<(), CliError> {
    match command {
        Command::Start(args) => loop_session::cmd_start(args, deps),
        Command::RunLoop(args) => loop_session::cmd_run_loop(args, deps),
        Command::Stop(args) => loop_session::cmd_stop(args, deps),
        Command::Status => loop_session::cmd_status(deps),
        Command::Logs(args) => loop_session::cmd_logs(args, deps),
        Command::Resume(args) => loop_session::cmd_resume(args, deps),
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

#[derive(Debug)]
pub(crate) enum CliError {
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
    verifier::run_verifier_pipeline(
        &dir,
        &config,
        args.test_command,
        args.coverage_command,
        args.coverage_min,
    )
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

const DEFAULT_SESSION_NAME: &str = "gralph";

fn session_name(name: &Option<String>, dir: &Path) -> Result<String, CliError> {
    if let Some(name) = name {
        let sanitized = sanitize_session_name(name);
        if sanitized.is_empty() {
            return Ok(DEFAULT_SESSION_NAME.to_string());
        }
        return Ok(sanitized);
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

fn validate_task_id(id: &str) -> Result<(), CliError> {
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

pub(crate) fn parse_bool_value(value: &str) -> Option<bool> {
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

pub(crate) fn normalize_csv(input: &str) -> Vec<String> {
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

pub(crate) fn join_or_none(entries: &[String]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serde_json::json;
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
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT",
        ] {
            remove_env(key);
        }
    }

    #[test]
    fn exit_code_for_ok_maps_success() {
        let code = exit_code_for(Ok(()));
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn exit_code_for_err_maps_failure() {
        let err = CliError::Message("nope".to_string());
        let code = exit_code_for(Err(err));
        assert_eq!(code, ExitCode::FAILURE);
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

    fn set_state_env(root: &Path) -> PathBuf {
        let state_dir = root.join("state");
        set_env("GRALPH_STATE_DIR", &state_dir);
        set_env("GRALPH_STATE_FILE", state_dir.join("state.json"));
        set_env("GRALPH_LOCK_FILE", state_dir.join("state.lock"));
        state_dir
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

    fn is_semver(value: &str) -> bool {
        let (core, build) = match value.split_once('+') {
            Some((left, right)) => (left, Some(right)),
            None => (value, None),
        };
        let (core, pre) = match core.split_once('-') {
            Some((left, right)) => (left, Some(right)),
            None => (core, None),
        };
        let mut parts = core.split('.');
        let major = parts.next().unwrap_or("");
        let minor = parts.next().unwrap_or("");
        let patch = parts.next().unwrap_or("");
        if parts.next().is_some() {
            return false;
        }
        for part in [major, minor, patch] {
            if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
                return false;
            }
        }
        if let Some(pre) = pre {
            if pre.is_empty() {
                return false;
            }
            if !pre.split('.').all(|ident| {
                !ident.is_empty()
                    && ident
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            }) {
                return false;
            }
        }
        if let Some(build) = build {
            if build.is_empty() {
                return false;
            }
            if !build.split('.').all(|ident| {
                !ident.is_empty()
                    && ident
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            }) {
                return false;
            }
        }
        true
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
    fn cmd_config_set_writes_nested_keys_and_preserves_mappings() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.yaml");

        write_file(
            &config_path,
            "defaults:\n  backend: claude\nlogging:\n  format:\n    color: true\n",
        );
        set_env("GRALPH_PROJECT_CONFIG_NAME", &config_path);

        let args = cli::ConfigSetArgs {
            key: "logging.level".to_string(),
            value: "info".to_string(),
        };
        cmd_config_set(args).unwrap();

        let args = cli::ConfigSetArgs {
            key: "notifications.webhook".to_string(),
            value: "https://example.test".to_string(),
        };
        cmd_config_set(args).unwrap();

        let contents = fs::read_to_string(&config_path).unwrap();
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents).unwrap();
        let root = yaml.as_mapping().unwrap();

        let defaults = root
            .get(&serde_yaml::Value::String("defaults".to_string()))
            .unwrap();
        let defaults_map = defaults.as_mapping().unwrap();
        assert_eq!(
            defaults_map.get(&serde_yaml::Value::String("backend".to_string())),
            Some(&serde_yaml::Value::String("claude".to_string()))
        );

        let logging = root
            .get(&serde_yaml::Value::String("logging".to_string()))
            .unwrap();
        let logging_map = logging.as_mapping().unwrap();
        assert_eq!(
            logging_map.get(&serde_yaml::Value::String("level".to_string())),
            Some(&serde_yaml::Value::String("info".to_string()))
        );
        let format = logging_map
            .get(&serde_yaml::Value::String("format".to_string()))
            .unwrap();
        let format_map = format.as_mapping().unwrap();
        assert_eq!(
            format_map.get(&serde_yaml::Value::String("color".to_string())),
            Some(&serde_yaml::Value::Bool(true))
        );

        let notifications = root
            .get(&serde_yaml::Value::String("notifications".to_string()))
            .unwrap();
        let notifications_map = notifications.as_mapping().unwrap();
        assert_eq!(
            notifications_map.get(&serde_yaml::Value::String("webhook".to_string())),
            Some(&serde_yaml::Value::String(
                "https://example.test".to_string()
            ))
        );

        clear_env_overrides();
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
    fn parse_bool_value_accepts_true_false_and_invalid() {
        for value in ["true", "True", "1", "yes", "Y", "on", "  ON  "] {
            assert_eq!(parse_bool_value(value), Some(true));
        }
        for value in ["false", "False", "0", "no", "N", "off", "  off  "] {
            assert_eq!(parse_bool_value(value), Some(false));
        }
        for value in ["", "maybe", "truthy", "2"] {
            assert_eq!(parse_bool_value(value), None);
        }
    }

    #[test]
    fn parse_bool_value_accepts_mixed_case_with_whitespace() {
        assert_eq!(parse_bool_value("\tYeS\n"), Some(true));
        assert_eq!(parse_bool_value("  oFf\t"), Some(false));
    }

    #[test]
    fn version_constants_match_package() {
        assert_eq!(version::VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(version::VERSION_TAG, format!("v{}", version::VERSION));
    }

    #[test]
    fn version_constant_parses_as_semver() {
        assert!(is_semver(version::VERSION));
    }

    #[test]
    fn validate_task_id_accepts_valid_formats() {
        for value in ["A-1", "COV-24", "cov-2", "Build-99"] {
            assert!(validate_task_id(value).is_ok(), "expected valid: {value}");
        }
    }

    #[test]
    fn validate_task_id_rejects_invalid_formats() {
        for value in [
            "", "A", "A-", "-1", "1-2", "1A-2", "A--1", "A-1b", "A-1-2", "A_1",
        ] {
            assert!(
                validate_task_id(value).is_err(),
                "expected invalid: {value}"
            );
        }
    }

    #[test]
    fn validate_task_id_reports_expected_error() {
        let err = validate_task_id("A-1b").unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Invalid task ID format"));
                assert!(message.contains("A-1b"));
                assert!(message.contains("expected like A-1"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn sanitize_session_name_replaces_invalid_chars() {
        assert_eq!(
            sanitize_session_name("My Session@2026!"),
            "My-Session-2026-"
        );
        assert_eq!(sanitize_session_name("dev_env-1"), "dev_env-1");
    }

    #[test]
    fn sanitize_session_name_handles_empty_and_whitespace() {
        assert_eq!(sanitize_session_name(""), "");
        assert_eq!(sanitize_session_name("   "), "---");
        assert_eq!(sanitize_session_name("\t"), "-");
        assert_eq!(sanitize_session_name("!!!"), "---");
    }

    #[test]
    fn session_name_uses_explicit_name_and_sanitizes() {
        let temp = tempfile::tempdir().unwrap();
        let resolved = session_name(&Some("My Session@2026!".to_string()), temp.path()).unwrap();
        assert_eq!(resolved, "My-Session-2026-");
    }

    #[test]
    fn session_name_uses_whitespace_override() {
        let temp = tempfile::tempdir().unwrap();
        let resolved = session_name(&Some("   ".to_string()), temp.path()).unwrap();
        assert_eq!(resolved, "---");
    }

    #[test]
    fn session_name_uses_directory_basename() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("My Session@2026!");
        fs::create_dir_all(&dir).unwrap();
        let resolved = session_name(&None, &dir).unwrap();
        assert_eq!(resolved, "My-Session-2026-");
    }

    #[test]
    fn session_name_uses_raw_basename_when_canonicalize_fails() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("Missing Dir@2026!");
        let resolved = session_name(&None, &dir).unwrap();
        assert_eq!(resolved, "Missing-Dir-2026-");
    }

    #[test]
    fn session_name_falls_back_for_empty_override() {
        let temp = tempfile::tempdir().unwrap();
        let resolved = session_name(&Some("".to_string()), temp.path()).unwrap();
        assert_eq!(resolved, DEFAULT_SESSION_NAME);
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
    fn read_readme_context_files_parses_section_and_dedupes() {
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("README.md"),
            "## Intro\n\nNothing here.\n\n## Context Files\n- `ARCHITECTURE.md` and `PROCESS.md`\n- `NOTES.txt`\n- `ARCHITECTURE.md`\n## Usage\n- `README.md`\n",
        );

        let entries = read_readme_context_files(temp.path());

        assert_eq!(entries, vec!["ARCHITECTURE.md", "PROCESS.md"]);
    }

    #[test]
    fn read_readme_context_files_skips_non_md_and_spaced_entries() {
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("README.md"),
            "## Context Files\n- `ARCHITECTURE.md`\n- `NOTES.txt`\n- `Team Notes.md`\n- `PROCESS.md`\n",
        );

        let entries = read_readme_context_files(temp.path());

        assert_eq!(entries, vec!["ARCHITECTURE.md", "PROCESS.md"]);
    }

    #[test]
    fn resolve_init_context_files_uses_config_list_and_dedupes() {
        let temp = tempfile::tempdir().unwrap();
        let entries = resolve_init_context_files(
            temp.path(),
            Some("ARCHITECTURE.md, ,PROCESS.md,ARCHITECTURE.md"),
        );

        assert_eq!(entries, vec!["ARCHITECTURE.md", "PROCESS.md"]);
    }

    #[test]
    fn resolve_init_context_files_falls_back_to_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let entries = resolve_init_context_files(temp.path(), None);

        let expected = default_context_files()
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        assert_eq!(entries, expected);
    }

    #[test]
    fn build_context_file_list_includes_config_user_and_defaults() {
        let temp = tempfile::tempdir().unwrap();
        write_file(&temp.path().join("README.md"), "readme");
        write_file(&temp.path().join("config/default.yaml"), "defaults: {}\n");
        write_file(&temp.path().join("src/main.rs"), "fn main() {}\n");

        let entries = build_context_file_list(
            temp.path(),
            Some("config/default.yaml,README.md"),
            Some("README.md,missing.md"),
        );

        assert_eq!(
            entries,
            vec!["README.md", "config/default.yaml", "src/main.rs"]
        );
    }

    #[test]
    fn read_yaml_or_empty_returns_mapping_for_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.yaml");

        let value = read_yaml_or_empty(&path).unwrap();

        match value {
            serde_yaml::Value::Mapping(map) => assert!(map.is_empty()),
            other => panic!("expected mapping, got: {other:?}"),
        }
    }

    #[test]
    fn read_yaml_or_empty_errors_on_invalid_yaml() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("broken.yaml");
        write_file(&path, "defaults: [");

        let err = read_yaml_or_empty(&path).unwrap_err();

        match err {
            CliError::Message(message) => assert!(message.contains("Failed to parse config")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn set_yaml_value_sets_nested_keys_and_overwrites_non_mapping() {
        let mut root = serde_yaml::Value::String("oops".to_string());

        set_yaml_value(&mut root, "alpha.beta", "true");

        let mapping = match root {
            serde_yaml::Value::Mapping(map) => map,
            other => panic!("expected mapping, got: {other:?}"),
        };
        let alpha = mapping
            .get(&serde_yaml::Value::String("alpha".to_string()))
            .unwrap();
        let inner = match alpha {
            serde_yaml::Value::Mapping(map) => map,
            other => panic!("expected mapping, got: {other:?}"),
        };
        let beta_key = serde_yaml::Value::String("beta".to_string());
        assert_eq!(inner.get(&beta_key), Some(&serde_yaml::Value::Bool(true)));
    }

    #[test]
    fn parse_yaml_value_parses_bool_number_and_string() {
        assert_eq!(parse_yaml_value("TRUE"), serde_yaml::Value::Bool(true));
        assert_eq!(parse_yaml_value("false"), serde_yaml::Value::Bool(false));
        match parse_yaml_value("42") {
            serde_yaml::Value::Number(value) => assert_eq!(value.as_i64(), Some(42)),
            other => panic!("expected number, got: {other:?}"),
        }
        match parse_yaml_value("-7") {
            serde_yaml::Value::Number(value) => assert_eq!(value.as_i64(), Some(-7)),
            other => panic!("expected number, got: {other:?}"),
        }
        assert_eq!(
            parse_yaml_value("1.5"),
            serde_yaml::Value::String("1.5".to_string())
        );
        assert_eq!(
            parse_yaml_value("maybe"),
            serde_yaml::Value::String("maybe".to_string())
        );
    }

    #[test]
    fn ensure_mapping_replaces_non_mapping_value() {
        let mut value = serde_yaml::Value::String("oops".to_string());

        let mapping = ensure_mapping(&mut value);

        assert!(mapping.is_empty());
        assert!(matches!(value, serde_yaml::Value::Mapping(_)));
    }

    #[test]
    fn is_markdown_path_detects_extensions() {
        assert!(is_markdown_path(Path::new("README.md")));
        assert!(is_markdown_path(Path::new("notes.markdown")));
        assert!(!is_markdown_path(Path::new("README.MD")));
        assert!(!is_markdown_path(Path::new("README")));
    }

    #[test]
    fn format_display_path_returns_relative_when_possible() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        let nested = base.join("docs/README.md");
        let expected = nested
            .strip_prefix(base)
            .unwrap()
            .to_string_lossy()
            .to_string();

        let display = format_display_path(&nested, base);

        assert_eq!(display, expected);
    }

    #[test]
    fn format_display_path_returns_full_when_outside_base() {
        let temp = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let base = temp.path();
        let path = other.path().join("README.md");

        let display = format_display_path(&path, base);

        assert_eq!(display, path.to_string_lossy().to_string());
    }

    #[test]
    fn resolve_log_file_prefers_session_entry_or_dir_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("custom.log");
        let session = json!({
            "log_file": log_path.to_string_lossy().to_string(),
            "dir": temp.path().to_string_lossy().to_string(),
        });

        let resolved = loop_session::resolve_log_file("demo", &session).unwrap();
        assert_eq!(resolved, log_path);

        let session = json!({
            "log_file": "",
            "dir": temp.path().to_string_lossy().to_string(),
        });

        let resolved = loop_session::resolve_log_file("demo", &session).unwrap();
        assert_eq!(resolved, temp.path().join(".gralph").join("demo.log"));
    }

    #[test]
    fn resolve_log_file_falls_back_when_missing_log_file() {
        let temp = tempfile::tempdir().unwrap();
        let session = json!({
            "dir": temp.path().to_string_lossy().to_string(),
        });

        let resolved = loop_session::resolve_log_file("demo", &session).unwrap();
        assert_eq!(resolved, temp.path().join(".gralph").join("demo.log"));
    }

    #[test]
    fn resolve_log_file_falls_back_for_whitespace_log_file() {
        let temp = tempfile::tempdir().unwrap();
        let session = json!({
            "log_file": "   ",
            "dir": temp.path().to_string_lossy().to_string(),
        });

        let resolved = loop_session::resolve_log_file("demo", &session).unwrap();
        assert_eq!(resolved, temp.path().join(".gralph").join("demo.log"));
    }

    #[test]
    fn resolve_log_file_errors_when_missing_dir() {
        let session = json!({
            "log_file": "",
        });

        let err = loop_session::resolve_log_file("demo", &session).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Missing dir for session demo"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_logs_uses_session_log_file() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = StateStore::new_from_env();
        store.init_state().unwrap();
        let log_path = temp.path().join("custom.log");
        write_file(&log_path, "line one\nline two\n");
        let log_path_string = log_path.to_string_lossy().to_string();
        let dir_string = temp.path().to_string_lossy().to_string();
        store
            .set_session(
                "demo",
                &[("dir", &dir_string), ("log_file", &log_path_string)],
            )
            .unwrap();

        let args = cli::LogsArgs {
            name: "demo".to_string(),
            follow: false,
        };
        loop_session::cmd_logs(args, &Deps::real()).unwrap();
        clear_env_overrides();
    }

    #[test]
    fn cmd_logs_falls_back_to_session_dir_log() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = StateStore::new_from_env();
        store.init_state().unwrap();
        let project_dir = temp.path().join("project");
        let log_path = project_dir.join(".gralph").join("demo.log");
        write_file(&log_path, "line one\n");
        let dir_string = project_dir.to_string_lossy().to_string();
        store.set_session("demo", &[("dir", &dir_string)]).unwrap();

        let args = cli::LogsArgs {
            name: "demo".to_string(),
            follow: false,
        };
        loop_session::cmd_logs(args, &Deps::real()).unwrap();
        clear_env_overrides();
    }

    #[test]
    fn auto_worktree_branch_name_uses_session_and_timestamp() {
        let name = auto_worktree_branch_name("demo-app", "20260126-120000");
        assert_eq!(name, "prd-demo-app-20260126-120000");

        let empty = auto_worktree_branch_name("", "20260126-120000");
        assert_eq!(empty, "prd-20260126-120000");
    }

    #[test]
    fn auto_worktree_branch_name_sanitizes_session_name() {
        let name = auto_worktree_branch_name("My Session@2026!", "20260126-120000");
        assert_eq!(name, "prd-My-Session-2026--20260126-120000");
    }

    #[test]
    fn auto_worktree_branch_name_differs_by_timestamp() {
        let first = auto_worktree_branch_name("demo-app", "20260126-120000");
        let second = auto_worktree_branch_name("demo-app", "20260126-120001");
        assert_ne!(first, second);
    }

    #[test]
    fn worktree_timestamp_slug_format_is_stable() {
        let slug = worktree_timestamp_slug();

        assert_eq!(slug.len(), 15);
        assert_eq!(slug.chars().nth(8), Some('-'));
        for (index, ch) in slug.chars().enumerate() {
            if index == 8 {
                continue;
            }
            assert!(ch.is_ascii_digit(), "expected digit at {index}, got {ch}");
        }
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
    #[cfg(unix)]
    fn session_name_falls_back_for_non_utf8_dir_name() {
        use std::os::unix::ffi::OsStringExt;

        let raw = std::ffi::OsString::from_vec(vec![0xff, 0xfe]);
        let dir = PathBuf::from(raw);
        let resolved = session_name(&None, &dir).unwrap();
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
    fn resolve_auto_worktree_disables_when_cli_override_set() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join(".gralph.yaml"),
            "defaults:\n  auto_worktree: true\n",
        );
        let config = Config::load(Some(temp.path())).unwrap();

        assert!(!resolve_auto_worktree(&config, true));
    }

    #[test]
    fn resolve_auto_worktree_defaults_true_on_invalid_config_value() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join(".gralph.yaml"),
            "defaults:\n  auto_worktree: maybe\n",
        );
        let config = Config::load(Some(temp.path())).unwrap();

        assert!(resolve_auto_worktree(&config, false));
    }

    #[test]
    fn resolve_auto_worktree_defaults_true_on_empty_config_value() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join(".gralph.yaml"),
            "defaults:\n  auto_worktree: \"\"\n",
        );
        let config = Config::load(Some(temp.path())).unwrap();

        assert!(resolve_auto_worktree(&config, false));
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

    #[test]
    fn ensure_unique_worktree_branch_handles_branch_only_collision() {
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();
        git_status_ok(temp.path(), &["branch", "prd-collision"]);

        let branch = ensure_unique_worktree_branch(
            temp.path().to_str().unwrap(),
            &worktrees_dir,
            "prd-collision",
        );

        assert_eq!(branch, "prd-collision-2");
    }

    #[test]
    fn ensure_unique_worktree_branch_handles_path_only_collision() {
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();
        fs::create_dir_all(worktrees_dir.join("prd-collision")).unwrap();

        let branch = ensure_unique_worktree_branch(
            temp.path().to_str().unwrap(),
            &worktrees_dir,
            "prd-collision",
        );

        assert_eq!(branch, "prd-collision-2");
    }

    #[test]
    fn ensure_unique_worktree_branch_returns_base_when_available() {
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let worktrees_dir = temp.path().join(".worktrees");
        fs::create_dir_all(&worktrees_dir).unwrap();

        let branch = ensure_unique_worktree_branch(
            temp.path().to_str().unwrap(),
            &worktrees_dir,
            "prd-free",
        );

        assert_eq!(branch, "prd-free");
    }

    #[test]
    fn create_worktree_at_rejects_existing_branch() {
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let branch = "task-C-1";
        git_status_ok(temp.path(), &["branch", branch]);
        let worktree_path = temp.path().join(".worktrees").join(branch);

        let err =
            create_worktree_at(temp.path().to_str().unwrap(), branch, &worktree_path).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Branch already exists"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn create_worktree_at_rejects_existing_path() {
        let temp = tempfile::tempdir().unwrap();
        init_git_repo(temp.path());
        commit_file(temp.path(), "README.md", "initial");
        let branch = "task-C-2";
        let worktree_path = temp.path().join(".worktrees").join(branch);
        fs::create_dir_all(&worktree_path).unwrap();

        let err =
            create_worktree_at(temp.path().to_str().unwrap(), branch, &worktree_path).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Worktree path already exists"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_resume_errors_when_missing_dir() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = StateStore::new_from_env();
        store.init_state().unwrap();
        store.set_session("demo", &[("status", "stopped")]).unwrap();

        let args = cli::ResumeArgs {
            name: Some("demo".to_string()),
        };
        let err = loop_session::cmd_resume(args, &Deps::real()).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Missing dir for session demo"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
        clear_env_overrides();
    }

    #[test]
    fn join_or_none_returns_none_for_empty() {
        let entries: Vec<String> = Vec::new();
        assert_eq!(join_or_none(&entries), "None");
    }

    #[test]
    fn join_or_none_joins_entries() {
        let entries = vec!["one".to_string(), "two".to_string()];
        assert_eq!(join_or_none(&entries), "one, two");
    }

    #[test]
    fn init_template_for_path_selects_known_templates() {
        let architecture = init_template_for_path(Path::new("ARCHITECTURE.md"));
        assert_eq!(architecture, ARCHITECTURE_TEMPLATE);

        let process = init_template_for_path(Path::new("process.md"));
        assert_eq!(process, PROCESS_TEMPLATE);

        let decisions = init_template_for_path(Path::new("DECISIONS.md"));
        assert_eq!(decisions, DECISIONS_TEMPLATE);

        let risk = init_template_for_path(Path::new("risk_register.md"));
        assert_eq!(risk, RISK_REGISTER_TEMPLATE);

        let changelog = init_template_for_path(Path::new("CHANGELOG.md"));
        assert_eq!(changelog, CHANGELOG_TEMPLATE);
    }

    #[test]
    fn generic_markdown_template_uses_stem_title() {
        let template = generic_markdown_template(Path::new("docs/TEAM_NOTES.md"));
        assert_eq!(template, "# TEAM NOTES\n\n## Overview\n\nTBD.\n");
    }

    #[test]
    fn write_atomic_overwrites_target_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("notes.md");
        write_file(&path, "old");

        write_atomic(&path, "new", false).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "new");
    }

    #[test]
    fn add_context_entry_skips_missing_and_dedupes() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path();
        write_file(&target_dir.join("README.md"), "readme");

        let mut entries: Vec<String> = Vec::new();
        let mut seen: BTreeMap<String, bool> = BTreeMap::new();

        add_context_entry(target_dir, "README.md", &mut entries, &mut seen);
        add_context_entry(target_dir, "README.md", &mut entries, &mut seen);
        add_context_entry(target_dir, "missing.md", &mut entries, &mut seen);

        assert_eq!(entries, vec!["README.md".to_string()]);
    }

    #[test]
    fn write_allowed_context_writes_entries_to_temp_file() {
        let entries = vec!["README.md".to_string(), "src/main.rs".to_string()];
        let path = write_allowed_context(&entries).unwrap().unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("README.md"));
        assert!(contents.contains("src/main.rs"));

        let _ = fs::remove_file(&path);
    }
}

const DEFAULT_PRD_TEMPLATE: &str = "## Overview\n\nBriefly describe the project, goals, and intended users.\n\n## Problem Statement\n\n- What problem does this solve?\n- What pain points exist today?\n\n## Solution\n\nHigh-level solution summary.\n\n---\n\n## Functional Requirements\n\n### FR-1: Core Feature\n\nDescribe the primary user-facing behavior.\n\n### FR-2: Secondary Feature\n\nDescribe supporting behavior.\n\n---\n\n## Non-Functional Requirements\n\n### NFR-1: Performance\n\n- Example: Response times under 200ms for key operations.\n\n### NFR-2: Reliability\n\n- Example: Crash recovery or retries where appropriate.\n\n---\n\n## Implementation Tasks\n\nEach task must use a `### Task <ID>` block header and include the required fields.\nEach task block must contain exactly one unchecked task line.\n\n### Task EX-1\n\n- **ID** EX-1\n- **Context Bundle** `path/to/file`, `path/to/other`\n- **DoD** Define the done criteria for this task.\n- **Checklist**\n  * First verification item.\n  * Second verification item.\n- **Dependencies** None\n- [ ] EX-1 Short task summary\n\n---\n\n## Success Criteria\n\n- Define measurable outcomes that indicate completion.\n\n---\n\n## Sources\n\n- List authoritative URLs used as source of truth.\n\n---\n\n## Warnings\n\n- Only include this section if no reliable sources were found.\n- State what is missing and what must be verified.\n";

const ARCHITECTURE_TEMPLATE: &str = "# Architecture\n\n## Overview\n\nDescribe the system at a high level.\n\n## Modules\n\nList key modules and what they own.\n\n## Runtime Flow\n\nDescribe the primary runtime path.\n\n## Storage\n\nRecord where state or data is stored.\n";

const PROCESS_TEMPLATE: &str = "# Process\n\n## Worktree Protocol\n\n1) Read required context files.\n2) Create a task worktree.\n3) Implement the scoped task.\n4) Update shared docs as needed.\n5) Verify changes.\n6) Finish and merge worktree.\n\n## Guardrails\n\n- Keep changes scoped to the assigned task.\n- Update CHANGELOG with the task ID.\n- Record new decisions and risks.\n";

const DECISIONS_TEMPLATE: &str = "# Decisions\n\n## D-001 Decision Title\n- Date: YYYY-MM-DD\n- Status: Proposed\n\n### Context\n\nWhy this decision is needed.\n\n### Decision\n\nWhat was decided.\n\n### Rationale\n\nWhy this choice was made.\n\n### Alternatives\n\nOther options considered.\n";

const RISK_REGISTER_TEMPLATE: &str = "# Risk Register\n\n## R-001 Risk Title\n- Risk: Describe the risk.\n- Impact: Low/Medium/High\n- Mitigation: How to reduce or monitor it.\n";

const CHANGELOG_TEMPLATE: &str = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nThe format is based on Keep a Changelog.\n\n## [Unreleased]\n\n### Added\n\n### Fixed\n";
