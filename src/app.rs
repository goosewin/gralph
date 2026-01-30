use crate::backend::backend_from_name;
use crate::cli::{
    self, Cli, Command, ConfigArgs, ConfigCommand, RunLoopArgs, ServerArgs, VerifierArgs,
    ASCII_BANNER,
};
use crate::config::Config;
use crate::core;
use crate::notify;
use crate::prd;
use crate::server::{self, ServerConfig};
use crate::state::StateStore;
use crate::update;
use crate::verifier;
use crate::version;
use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod loop_session;
mod prd_init;
pub(crate) mod worktree;

use prd_init::{cmd_init, cmd_prd};

#[cfg(test)]
use prd_init::{
    add_context_entry, build_context_file_list, default_context_files, format_display_path,
    generic_markdown_template, init_template_for_path, invalid_prd_path, is_markdown_path,
    read_prd_template_with_manifest, read_readme_context_files, resolve_init_context_files,
    resolve_prd_output, write_allowed_context, write_atomic, ARCHITECTURE_TEMPLATE,
    CHANGELOG_TEMPLATE, DECISIONS_TEMPLATE, DEFAULT_PRD_TEMPLATE, PROCESS_TEMPLATE,
    RISK_REGISTER_TEMPLATE,
};

pub(crate) trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Seek>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Seek>> {
        let file = fs::File::open(path)?;
        Ok(Box::new(file))
    }
}

pub(crate) trait ProcessRunner: Send + Sync {
    fn current_exe(&self) -> io::Result<PathBuf>;
    fn spawn(&self, cmd: &mut std::process::Command) -> io::Result<std::process::Child>;
    fn kill_tmux_session(&self, session: &str);
    fn kill_pid(&self, pid: i64);
    fn pid(&self) -> u32;
    fn is_alive(&self, pid: i64) -> bool;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RealProcessRunner;

impl ProcessRunner for RealProcessRunner {
    fn current_exe(&self) -> io::Result<PathBuf> {
        env::current_exe()
    }

    fn spawn(&self, cmd: &mut std::process::Command) -> io::Result<std::process::Child> {
        cmd.spawn()
    }

    fn kill_tmux_session(&self, session: &str) {
        let _ = std::process::Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg(session)
            .status();
    }

    fn kill_pid(&self, pid: i64) {
        if pid <= 0 {
            return;
        }
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

    fn pid(&self) -> u32 {
        std::process::id()
    }

    fn is_alive(&self, pid: i64) -> bool {
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
}

pub struct Deps {
    worktree: worktree::Worktree,
    fs: Box<dyn FileSystem>,
    process: Box<dyn ProcessRunner>,
    clock: Box<dyn core::Clock>,
    notifier: Box<dyn notify::Notifier>,
}

impl Default for Deps {
    fn default() -> Self {
        Self::real()
    }
}

impl Deps {
    pub fn real() -> Self {
        Self {
            worktree: worktree::Worktree::default(),
            fs: Box::new(RealFileSystem),
            process: Box::new(RealProcessRunner),
            clock: Box::new(core::SystemClock),
            notifier: Box::new(notify::RealNotifier),
        }
    }

    pub(crate) fn with_parts(
        worktree: worktree::Worktree,
        fs: Box<dyn FileSystem>,
        process: Box<dyn ProcessRunner>,
        clock: Box<dyn core::Clock>,
        notifier: Box<dyn notify::Notifier>,
    ) -> Self {
        Self {
            worktree,
            fs,
            process,
            clock,
            notifier,
        }
    }

    pub fn worktree(&self) -> &worktree::Worktree {
        &self.worktree
    }

    pub(crate) fn fs(&self) -> &dyn FileSystem {
        self.fs.as_ref()
    }

    pub(crate) fn process(&self) -> &dyn ProcessRunner {
        self.process.as_ref()
    }

    pub(crate) fn clock(&self) -> &dyn core::Clock {
        self.clock.as_ref()
    }

    pub(crate) fn notifier(&self) -> &dyn notify::Notifier {
        self.notifier.as_ref()
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
        Command::Worktree(args) => deps.worktree().cmd_worktree(args),
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

pub(crate) fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" | "on" => Some(true),
        "false" | "0" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
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

pub(crate) fn normalize_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect()
}

pub(crate) fn join_or_none(entries: &[String]) -> String {
    if entries.is_empty() {
        "None".to_string()
    } else {
        entries.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::worktree;
    use super::*;
    use crate::cli::InitArgs;
    use clap::Parser;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;
    use std::process::Command as ProcCommand;
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
            assert!(
                worktree::validate_task_id(value).is_ok(),
                "expected valid: {value}"
            );
        }
    }

    #[test]
    fn validate_task_id_rejects_invalid_formats() {
        for value in [
            "", "A", "A-", "-1", "1-2", "1A-2", "A--1", "A-1b", "A-1-2", "A_1",
        ] {
            assert!(
                worktree::validate_task_id(value).is_err(),
                "expected invalid: {value}"
            );
        }
    }

    #[test]
    fn validate_task_id_reports_expected_error() {
        let err = worktree::validate_task_id("A-1b").unwrap_err();
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
        let name = worktree::auto_worktree_branch_name("demo-app", "20260126-120000");
        assert_eq!(name, "prd-demo-app-20260126-120000");

        let empty = worktree::auto_worktree_branch_name("", "20260126-120000");
        assert_eq!(empty, "prd-20260126-120000");
    }

    #[test]
    fn auto_worktree_branch_name_sanitizes_session_name() {
        let name = worktree::auto_worktree_branch_name("My Session@2026!", "20260126-120000");
        assert_eq!(name, "prd-My-Session-2026--20260126-120000");
    }

    #[test]
    fn auto_worktree_branch_name_differs_by_timestamp() {
        let first = worktree::auto_worktree_branch_name("demo-app", "20260126-120000");
        let second = worktree::auto_worktree_branch_name("demo-app", "20260126-120001");
        assert_ne!(first, second);
    }

    #[test]
    fn worktree_timestamp_slug_format_is_stable() {
        let slug = worktree::worktree_timestamp_slug();

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

        assert!(worktree::resolve_auto_worktree(&config, false));
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

        assert!(!worktree::resolve_auto_worktree(&config, false));
        assert!(!worktree::resolve_auto_worktree(&config, true));
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

        assert!(!worktree::resolve_auto_worktree(&config, true));
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

        assert!(worktree::resolve_auto_worktree(&config, false));
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

        assert!(worktree::resolve_auto_worktree(&config, false));
    }

    #[test]
    fn auto_worktree_skips_non_git_directory() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config = Config::load(Some(temp.path())).unwrap();
        let mut args = run_loop_args(temp.path().to_path_buf());
        let original = args.dir.clone();

        worktree::maybe_create_auto_worktree(&mut args, &config).unwrap();

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

        worktree::maybe_create_auto_worktree(&mut args, &config).unwrap();

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

        worktree::maybe_create_auto_worktree(&mut args, &config).unwrap();

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

        worktree::maybe_create_auto_worktree(&mut args, &config).unwrap();

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

        worktree::maybe_create_auto_worktree(&mut args, &config).unwrap();

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
        let base_branch = worktree::auto_worktree_branch_name(&args.name, timestamp);
        git_status_ok(temp.path(), &["branch", base_branch.as_str()]);
        fs::create_dir_all(worktrees_dir.join(&base_branch)).unwrap();
        fs::create_dir_all(worktrees_dir.join(format!("{}-2", base_branch))).unwrap();

        worktree::maybe_create_auto_worktree_with_timestamp(&mut args, &config, timestamp).unwrap();

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

        let branch = worktree::ensure_unique_worktree_branch(
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

        let branch = worktree::ensure_unique_worktree_branch(
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

        let branch = worktree::ensure_unique_worktree_branch(
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

        let branch = worktree::ensure_unique_worktree_branch(
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
            worktree::create_worktree_at(temp.path().to_str().unwrap(), branch, &worktree_path)
                .unwrap_err();
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
            worktree::create_worktree_at(temp.path().to_str().unwrap(), branch, &worktree_path)
                .unwrap_err();
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
