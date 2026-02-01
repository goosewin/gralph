use super::{CliError, Deps, FileSystem, ProcessRunner};
use crate::backend::backend_from_name;
use crate::cli::{
    AttachArgs, CleanupArgs, LogsArgs, ResumeArgs, RunLoopArgs, StartArgs, StatusArgs, StepArgs,
    StopArgs,
};
use crate::config::Config;
use crate::core::{self, LoopStatus};
use crate::notify;
use crate::prd;
use crate::state::{CleanupMode, StateStore};
use crate::update;
use crate::verifier;
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcCommand, Stdio};
use std::time::Duration;

pub(super) fn cmd_start(args: StartArgs, deps: &Deps) -> Result<(), CliError> {
    if !args.dir.is_dir() {
        return Err(CliError::Message(format!(
            "Directory does not exist: {}",
            args.dir.display()
        )));
    }
    if args.dry_run {
        return cmd_start_dry_run(args, deps);
    }
    let session_name = super::session_name(&args.name, &args.dir)?;
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let mut run_args = run_loop_args_from_start(args, session_name)?;
    ensure_tmux_available()?;
    let tmux_session = unique_tmux_session_name(&run_args.name)?;
    run_args.tmux_session = Some(tmux_session.clone());
    deps.worktree()
        .maybe_create_auto_worktree(&mut run_args, &config)?;
    let child = spawn_run_loop(&run_args, deps.process())?;

    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let now = format_rfc3339(deps.clock());
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
    let raw_log_file = core::raw_log_path(&log_file);

    store
        .set_session(
            &run_args.name,
            &[
                ("dir", &run_args.dir.to_string_lossy()),
                ("task_file", &task_file),
                ("pid", &child.id().to_string()),
                ("tmux_session", &tmux_session),
                ("started_at", &now),
                ("iteration", "1"),
                ("max_iterations", &max_iterations.to_string()),
                ("status", "running"),
                ("last_task_count", &remaining.to_string()),
                ("completion_marker", &completion_marker),
                ("log_file", &log_file.to_string_lossy()),
                ("raw_log_file", &raw_log_file.to_string_lossy()),
                ("backend", run_args.backend.as_deref().unwrap_or("claude")),
                ("model", run_args.model.as_deref().unwrap_or("")),
                ("variant", run_args.variant.as_deref().unwrap_or("")),
                ("webhook", run_args.webhook.as_deref().unwrap_or("")),
            ],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;

    println!("Gralph loop started in background (PID: {}).", child.id());
    println!("Tmux session: {}", tmux_session);
    println!("Logs: {}", log_file.display());
    println!(
        "Tail logs: gralph logs {} --follow (or tail -f {}).",
        run_args.name,
        log_file.display()
    );
    Ok(())
}

fn cmd_start_dry_run(args: StartArgs, deps: &Deps) -> Result<(), CliError> {
    let session_name = super::session_name(&args.name, &args.dir)?;
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let run_args = run_loop_args_from_start(args, session_name)?;
    let task_file = resolve_task_file(&run_args, &config);
    let max_iterations = resolve_max_iterations(&run_args, &config);
    let completion_marker = resolve_completion_marker(&run_args, &config);

    if should_validate_prd(run_args.strict_prd) {
        prd::prd_validate_file(&run_args.dir.join(&task_file), false, Some(&run_args.dir))
            .map_err(|err| CliError::Message(err.to_string()))?;
    }

    let prompt_template = match &run_args.prompt_template {
        Some(path) => Some(deps.fs().read_to_string(path).map_err(CliError::Io)?),
        None => None,
    };

    let rendered = core::render_iteration_prompt(
        &run_args.dir,
        &task_file,
        1,
        max_iterations,
        &completion_marker,
        prompt_template.as_deref(),
        Some(&config),
    )
    .map_err(|err| CliError::Message(err.to_string()))?;

    println!("Next task block:");
    if let Some(block) = rendered.task_block {
        println!("{}", block);
    } else {
        println!("(none)");
    }
    println!();
    println!("Resolved prompt:");
    println!("{}", rendered.prompt);
    Ok(())
}

pub(super) fn cmd_step(args: StepArgs, deps: &Deps) -> Result<(), CliError> {
    if !args.dir.is_dir() {
        return Err(CliError::Message(format!(
            "Directory does not exist: {}",
            args.dir.display()
        )));
    }
    let session_name = super::session_name(&args.name, &args.dir)?;
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let mut run_args = run_loop_args_from_step(args, session_name)?;
    deps.worktree()
        .maybe_create_auto_worktree(&mut run_args, &config)?;
    run_single_iteration(run_args, &config, deps)
}

pub(super) fn cmd_run_loop(mut args: RunLoopArgs, deps: &Deps) -> Result<(), CliError> {
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    deps.worktree()
        .maybe_create_auto_worktree(&mut args, &config)?;
    run_loop_with_state(args, deps)
}

pub(super) fn cmd_stop(args: StopArgs, deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;

    if args.all {
        let sessions = store
            .list_sessions()
            .map_err(|err| CliError::Message(err.to_string()))?;
        for session in sessions {
            if let Some(name) = session.get("name").and_then(|v| v.as_str()) {
                stop_session(&store, name, &session, deps.process())?;
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

    stop_session(&store, &name, &session, deps.process())?;
    println!("Stopped session: {}", name);
    Ok(())
}

pub(super) fn cmd_status(args: StatusArgs, deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let _ = store.cleanup_stale(CleanupMode::Mark);

    let sessions = store
        .list_sessions()
        .map_err(|err| CliError::Message(err.to_string()))?;
    if sessions.is_empty() {
        if args.json {
            let output = serde_json::json!({"sessions": []});
            let rendered =
                serde_json::to_string(&output).map_err(|err| CliError::Message(err.to_string()))?;
            println!("{}", rendered);
        } else {
            println!("No sessions found.");
        }
        return Ok(());
    }

    let enriched = sessions
        .into_iter()
        .map(|session| enrich_status_session(session, deps.process()))
        .collect::<Vec<_>>();

    if args.json {
        let output = serde_json::json!({"sessions": enriched});
        let rendered =
            serde_json::to_string(&output).map_err(|err| CliError::Message(err.to_string()))?;
        println!("{}", rendered);
        return Ok(());
    }

    let mut rows = Vec::new();
    for session in &enriched {
        let name = session
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let dir = session.get("dir").and_then(|v| v.as_str()).unwrap_or("");
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
        let remaining = session
            .get("current_remaining")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        rows.push(vec![
            name.to_string(),
            dir.to_string(),
            format!("{}/{}", iteration, max_iterations),
            status.to_string(),
            format!("{}", remaining),
        ]);
    }

    print_table(&["NAME", "DIR", "ITERATION", "STATUS", "REMAINING"], &rows);
    if args.verbose {
        print_status_verbose(&enriched);
    }
    Ok(())
}

fn enrich_status_session(session: Value, process: &dyn ProcessRunner) -> Value {
    let mut map = match session.as_object() {
        Some(map) => map.clone(),
        None => Map::new(),
    };
    let name_raw = map.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let dir = map.get("dir").and_then(|v| v.as_str()).unwrap_or("");
    let task_file = map
        .get("task_file")
        .and_then(|v| v.as_str())
        .unwrap_or("PRD.md");
    let mut status = map
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let pid = map.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
    let mut is_alive = false;
    if status == "running" && pid > 0 {
        if process.is_alive(pid) {
            is_alive = true;
        } else {
            status = "stale".to_string();
        }
    }

    let remaining = if dir.is_empty() {
        map.get("last_task_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i64
    } else {
        core::count_remaining_tasks(&PathBuf::from(dir).join(task_file)) as i64
    };

    let log_file = resolve_status_log_file(&map, name_raw, dir);
    let raw_log_file = resolve_status_raw_log_file(&map, log_file.as_ref());
    let last_task_id = if dir.is_empty() {
        None
    } else {
        prd::prd_next_task_id(&PathBuf::from(dir).join(task_file))
    };
    let last_log = log_file
        .as_ref()
        .and_then(|path| core::last_log_line(path.as_path()));
    let last_error = log_file
        .as_ref()
        .and_then(|path| core::last_error_line(path.as_path()));

    map.insert(
        "current_remaining".to_string(),
        Value::Number(remaining.into()),
    );
    map.insert("is_alive".to_string(), Value::Bool(is_alive));
    map.insert("status".to_string(), Value::String(status));
    map.insert(
        "log_file".to_string(),
        log_file
            .as_ref()
            .map(|path| Value::String(path.to_string_lossy().to_string()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "raw_log_file".to_string(),
        raw_log_file
            .as_ref()
            .map(|path| Value::String(path.to_string_lossy().to_string()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "last_task_id".to_string(),
        last_task_id.map(Value::String).unwrap_or(Value::Null),
    );
    map.insert(
        "last_log_line".to_string(),
        last_log.map(Value::String).unwrap_or(Value::Null),
    );
    map.insert(
        "last_error".to_string(),
        last_error.map(Value::String).unwrap_or(Value::Null),
    );
    Value::Object(map)
}

fn resolve_status_log_file(map: &Map<String, Value>, name: &str, dir: &str) -> Option<PathBuf> {
    if let Some(path) = map.get("log_file").and_then(|value| value.as_str()) {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    if dir.is_empty() || name.is_empty() {
        return None;
    }
    Some(
        PathBuf::from(dir)
            .join(".gralph")
            .join(format!("{}.log", name)),
    )
}

fn resolve_status_raw_log_file(
    map: &Map<String, Value>,
    log_file: Option<&PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = map.get("raw_log_file").and_then(|value| value.as_str()) {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    log_file.map(|path| core::raw_log_path(path.as_path()))
}

fn print_status_verbose(sessions: &[Value]) {
    for session in sessions {
        let name = session
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let log_file = session
            .get("log_file")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let raw_log_file = session
            .get("raw_log_file")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let last_error = session
            .get("last_error")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        println!();
        println!("{}:", name);
        println!(
            "  log_file: {}",
            if log_file.is_empty() {
                "none"
            } else {
                log_file
            }
        );
        println!(
            "  raw_log_file: {}",
            if raw_log_file.is_empty() {
                "none"
            } else {
                raw_log_file
            }
        );
        println!(
            "  last_error: {}",
            if last_error.is_empty() {
                "none"
            } else {
                last_error
            }
        );
    }
}

pub(super) fn cmd_cleanup(args: CleanupArgs, deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;

    if args.purge {
        let purged = store
            .purge_all()
            .map_err(|err| CliError::Message(err.to_string()))?;
        print_cleanup_result("Purged", "No sessions found.", &purged);
        return Ok(());
    }

    let mode = if args.remove {
        CleanupMode::Remove
    } else {
        CleanupMode::Mark
    };
    let cleaned = store
        .cleanup_stale(mode)
        .map_err(|err| CliError::Message(err.to_string()))?;
    let action = match mode {
        CleanupMode::Mark => "Marked",
        CleanupMode::Remove => "Removed",
    };
    print_cleanup_result(action, "No stale sessions found.", &cleaned);
    Ok(())
}

pub(super) fn cmd_logs(args: LogsArgs, deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let session = store
        .get_session(&args.name)
        .map_err(|err| CliError::Message(err.to_string()))?
        .ok_or_else(|| CliError::Message(format!("Session not found: {}", args.name)))?;
    let log_file = if args.raw {
        resolve_raw_log_file(&args.name, &session)?
    } else {
        resolve_log_file(&args.name, &session)?
    };
    if !log_file.is_file() {
        return Err(CliError::Message(format!(
            "{} does not exist: {}",
            if args.raw { "Raw log file" } else { "Log file" },
            log_file.display()
        )));
    }

    if args.follow {
        follow_log(&log_file, deps.fs(), deps.clock())?;
    } else {
        print_tail(&log_file, 200, deps.fs())?;
    }
    Ok(())
}

pub(super) fn cmd_attach(args: AttachArgs, deps: &Deps) -> Result<(), CliError> {
    ensure_tmux_available()?;
    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let session = store
        .get_session(&args.name)
        .map_err(|err| CliError::Message(err.to_string()))?
        .ok_or_else(|| CliError::Message(format!("Session not found: {}", args.name)))?;
    let tmux_session = session
        .get("tmux_session")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    if tmux_session.is_empty() {
        return Err(CliError::Message(format!(
            "No tmux session recorded for {}",
            args.name
        )));
    }

    let status = ProcCommand::new("tmux")
        .arg("attach-session")
        .arg("-t")
        .arg(tmux_session)
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(CliError::Message(format!(
            "Failed to attach to tmux session {} (exit code {}).",
            tmux_session,
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ))),
        Err(err) => Err(CliError::Message(format!(
            "Failed to attach to tmux session {}: {}",
            tmux_session, err
        ))),
    }
}

pub(super) fn cmd_resume(args: ResumeArgs, deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
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
        let pid_alive = status == "running" && pid > 0 && deps.process().is_alive(pid);
        let should_resume = should_resume_session(status, pid, pid_alive);
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
            tmux_session: None,
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
        let child = spawn_run_loop(&run_args, deps.process())?;
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

fn maybe_check_for_update() {
    let current_version = crate::version::VERSION;
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

fn should_check_for_update(config: &Config) -> bool {
    if let Ok(value) = env::var("GRALPH_NO_UPDATE_CHECK") {
        if value.trim().is_empty() {
            return false;
        }
        if let Some(parsed) = super::parse_bool_value(&value) {
            return !parsed;
        }
        return false;
    }
    config
        .get("defaults.check_updates")
        .as_deref()
        .and_then(super::parse_bool_value)
        .unwrap_or(true)
}

fn format_rfc3339(clock: &dyn core::Clock) -> String {
    let datetime: chrono::DateTime<chrono::Local> = clock.now().into();
    datetime.to_rfc3339()
}

fn resolve_task_file(args: &RunLoopArgs, config: &Config) -> String {
    args.task_file
        .clone()
        .or_else(|| config.get("defaults.task_file"))
        .unwrap_or_else(|| "PRD.md".to_string())
}

fn resolve_max_iterations(args: &RunLoopArgs, config: &Config) -> u32 {
    args.max_iterations
        .or_else(|| {
            config
                .get("defaults.max_iterations")
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(30)
}

fn resolve_completion_marker(args: &RunLoopArgs, config: &Config) -> String {
    args.completion_marker
        .clone()
        .or_else(|| config.get("defaults.completion_marker"))
        .unwrap_or_else(|| "COMPLETE".to_string())
}

fn resolve_backend_name(args: &RunLoopArgs, config: &Config) -> String {
    args.backend
        .clone()
        .or_else(|| config.get("defaults.backend"))
        .unwrap_or_else(|| "claude".to_string())
}

fn resolve_model(args: &RunLoopArgs, config: &Config, backend_name: &str) -> Option<String> {
    let mut model = args.model.clone().or_else(|| config.get("defaults.model"));
    if model.as_deref().unwrap_or("").is_empty() && backend_name == "opencode" {
        model = config.get("opencode.default_model");
    }
    model
}

fn should_validate_prd(strict_prd: bool) -> bool {
    strict_prd
}

fn should_resume_session(status: &str, pid: i64, pid_alive: bool) -> bool {
    if matches!(status, "stale" | "stopped" | "failed") {
        return true;
    }
    if status == "running" {
        return pid <= 0 || !pid_alive;
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutcomeStatusPlan {
    Final {
        status: &'static str,
    },
    Verify {
        initial_status: &'static str,
        verifying_status: &'static str,
        verified_status: &'static str,
        verify_failed_status: &'static str,
    },
}

impl OutcomeStatusPlan {
    fn initial_status(&self) -> &'static str {
        match self {
            OutcomeStatusPlan::Final { status } => status,
            OutcomeStatusPlan::Verify { initial_status, .. } => initial_status,
        }
    }
}

fn outcome_status_plan(status: LoopStatus, auto_run_verifier: bool) -> OutcomeStatusPlan {
    if status == LoopStatus::Complete && auto_run_verifier {
        OutcomeStatusPlan::Verify {
            initial_status: status.as_str(),
            verifying_status: "verifying",
            verified_status: "verified",
            verify_failed_status: "verify-failed",
        }
    } else {
        OutcomeStatusPlan::Final {
            status: status.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotificationDecision {
    Complete,
    Failed { reason: &'static str },
}

fn notification_decision(
    status: LoopStatus,
    notify_on_complete: bool,
) -> Option<NotificationDecision> {
    match status {
        LoopStatus::Complete => notify_on_complete.then_some(NotificationDecision::Complete),
        LoopStatus::Failed => Some(NotificationDecision::Failed { reason: "error" }),
        LoopStatus::MaxIterations => Some(NotificationDecision::Failed {
            reason: "max_iterations",
        }),
        LoopStatus::Running => None,
    }
}

fn run_loop_with_state(args: RunLoopArgs, deps: &Deps) -> Result<(), CliError> {
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    if should_check_for_update(&config) {
        maybe_check_for_update();
    }
    let task_file = resolve_task_file(&args, &config);
    let max_iterations = resolve_max_iterations(&args, &config);
    let completion_marker = resolve_completion_marker(&args, &config);
    let backend_name = resolve_backend_name(&args, &config);
    let model = resolve_model(&args, &config, &backend_name);

    if should_validate_prd(args.strict_prd) {
        prd::prd_validate_file(&args.dir.join(&task_file), false, Some(&args.dir))
            .map_err(|err| CliError::Message(err.to_string()))?;
    }

    let prompt_template = match &args.prompt_template {
        Some(path) => Some(deps.fs().read_to_string(path).map_err(CliError::Io)?),
        None => None,
    };

    let backend = backend_from_name(&backend_name).map_err(CliError::Message)?;
    if !backend.check_installed() {
        return Err(CliError::Message(format!(
            "Backend is not installed: {}",
            backend_name
        )));
    }

    let store = deps.state_store();
    store
        .init_state()
        .map_err(|err| CliError::Message(err.to_string()))?;
    let tmux_session = resolve_tmux_session(&args);
    let now = format_rfc3339(deps.clock());
    let remaining = core::count_remaining_tasks(&args.dir.join(&task_file));
    let log_file = args.dir.join(".gralph").join(format!("{}.log", args.name));
    let raw_log_file = core::raw_log_path(&log_file);

    store
        .set_session(
            &args.name,
            &[
                ("dir", &args.dir.to_string_lossy()),
                ("task_file", &task_file),
                ("pid", &deps.process().pid().to_string()),
                ("tmux_session", &tmux_session),
                ("started_at", &now),
                ("iteration", "1"),
                ("max_iterations", &max_iterations.to_string()),
                ("status", "running"),
                ("last_task_count", &remaining.to_string()),
                ("completion_marker", &completion_marker),
                ("log_file", &log_file.to_string_lossy()),
                ("raw_log_file", &raw_log_file.to_string_lossy()),
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

    let outcome = core::run_loop_with_clock(
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
        deps.clock(),
    )
    .map_err(|err| CliError::Message(err.to_string()))?;

    let auto_run_verifier = verifier::resolve_verifier_auto_run(&config, &args.dir);
    let status_plan = outcome_status_plan(outcome.status, auto_run_verifier);
    store
        .set_session(
            &args.name,
            &[
                ("status", status_plan.initial_status()),
                ("last_task_count", &outcome.remaining_tasks.to_string()),
            ],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;

    if let OutcomeStatusPlan::Verify {
        verifying_status,
        verified_status,
        verify_failed_status,
        ..
    } = status_plan
    {
        store
            .set_session(
                &args.name,
                &[("status", verifying_status), ("last_task_count", "0")],
            )
            .map_err(|err| CliError::Message(err.to_string()))?;
        if let Err(err) = verifier::run_verifier_pipeline(&args.dir, &config, None, None, None) {
            let _ = store.set_session(&args.name, &[("status", verify_failed_status)]);
            return Err(err);
        }
        store
            .set_session(
                &args.name,
                &[("status", verified_status), ("last_task_count", "0")],
            )
            .map_err(|err| CliError::Message(err.to_string()))?;
    }

    notify_if_configured(&config, &args, &outcome, max_iterations, deps.notifier())?;
    Ok(())
}

fn run_single_iteration(args: RunLoopArgs, config: &Config, deps: &Deps) -> Result<(), CliError> {
    let task_file = resolve_task_file(&args, config);
    let max_iterations = resolve_max_iterations(&args, config);
    let completion_marker = resolve_completion_marker(&args, config);
    let backend_name = resolve_backend_name(&args, config);
    let model = resolve_model(&args, config, &backend_name);

    if should_validate_prd(args.strict_prd) {
        prd::prd_validate_file(&args.dir.join(&task_file), false, Some(&args.dir))
            .map_err(|err| CliError::Message(err.to_string()))?;
    }

    let prompt_template = match &args.prompt_template {
        Some(path) => Some(deps.fs().read_to_string(path).map_err(CliError::Io)?),
        None => None,
    };

    let backend = backend_from_name(&backend_name).map_err(CliError::Message)?;
    if !backend.check_installed() {
        return Err(CliError::Message(format!(
            "Backend is not installed: {}",
            backend_name
        )));
    }

    let gralph_dir = args.dir.join(".gralph");
    fs::create_dir_all(&gralph_dir).map_err(CliError::Io)?;
    let log_file = gralph_dir.join(format!("{}.log", args.name));

    let iteration_result = core::run_iteration(
        &*backend,
        &args.dir,
        &task_file,
        1,
        max_iterations,
        &completion_marker,
        model.as_deref(),
        args.variant.as_deref(),
        Some(&log_file),
        prompt_template.as_deref(),
        Some(config),
    )
    .map_err(|err| CliError::Message(err.to_string()))?;

    let remaining = core::count_remaining_tasks(&args.dir.join(&task_file));
    println!("Step completed. Remaining tasks: {}", remaining);

    let complete = core::check_completion(
        &args.dir.join(&task_file),
        &iteration_result.result,
        &completion_marker,
    )
    .map_err(|err| CliError::Message(err.to_string()))?;
    if complete {
        println!("Completion promise detected.");
    }

    Ok(())
}

fn notify_if_configured(
    config: &Config,
    args: &RunLoopArgs,
    outcome: &core::LoopOutcome,
    max_iterations: u32,
    notifier: &dyn notify::Notifier,
) -> Result<(), CliError> {
    let webhook = args
        .webhook
        .clone()
        .or_else(|| config.get("notifications.webhook"));
    let Some(webhook) = webhook else {
        return Ok(());
    };

    let on_complete = config
        .get("notifications.on_complete")
        .map(|v| v == "true")
        .unwrap_or(true);
    match notification_decision(outcome.status, on_complete) {
        Some(NotificationDecision::Complete) => {
            notifier
                .notify_complete(
                    &args.name,
                    &webhook,
                    Some(&args.dir.to_string_lossy()),
                    Some(outcome.iterations),
                    Some(outcome.duration_secs),
                    None,
                )
                .map_err(|err| CliError::Message(err.to_string()))?;
        }
        Some(NotificationDecision::Failed { reason }) => {
            notifier
                .notify_failed(
                    &args.name,
                    &webhook,
                    Some(reason),
                    Some(&args.dir.to_string_lossy()),
                    Some(outcome.iterations),
                    Some(max_iterations),
                    Some(outcome.remaining_tasks as u32),
                    Some(outcome.duration_secs),
                    None,
                )
                .map_err(|err| CliError::Message(err.to_string()))?;
        }
        None => {}
    }

    Ok(())
}

fn resolve_tmux_session(args: &RunLoopArgs) -> String {
    args.tmux_session
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn run_loop_args_from_start(args: StartArgs, name: String) -> Result<RunLoopArgs, CliError> {
    Ok(RunLoopArgs {
        dir: args.dir,
        name,
        tmux_session: None,
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

fn run_loop_args_from_step(args: StepArgs, name: String) -> Result<RunLoopArgs, CliError> {
    Ok(RunLoopArgs {
        dir: args.dir,
        name,
        tmux_session: None,
        max_iterations: args.max_iterations,
        task_file: args.task_file,
        completion_marker: args.completion_marker,
        backend: args.backend,
        model: args.model,
        variant: args.variant,
        prompt_template: args.prompt_template,
        webhook: None,
        no_worktree: args.no_worktree,
        strict_prd: args.strict_prd,
    })
}

fn ensure_tmux_available() -> Result<(), CliError> {
    let status = ProcCommand::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(CliError::Message(
            "tmux is required; install tmux and try again".to_string(),
        )),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(CliError::Message(
            "tmux is required but was not found on PATH".to_string(),
        )),
        Err(err) => Err(CliError::Message(format!(
            "failed to check tmux availability: {}",
            err
        ))),
    }
}

fn unique_tmux_session_name(base: &str) -> Result<String, CliError> {
    let timestamp = super::worktree::worktree_timestamp_slug();
    let mut candidate = if base.trim().is_empty() {
        format!("gralph-{}", timestamp)
    } else {
        format!("{}-{}", base, timestamp)
    };
    let base_candidate = candidate.clone();
    let mut suffix = 2;
    while tmux_session_exists(&candidate)? {
        candidate = format!("{}-{}", base_candidate, suffix);
        suffix += 1;
    }
    Ok(candidate)
}

fn tmux_session_exists(name: &str) -> Result<bool, CliError> {
    let status = ProcCommand::new("tmux")
        .arg("has-session")
        .arg("-t")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(status) => Ok(status.success()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(CliError::Message(
            "tmux is required but was not found on PATH".to_string(),
        )),
        Err(err) => Err(CliError::Message(format!(
            "failed to check tmux session: {}",
            err
        ))),
    }
}

fn append_run_loop_args(cmd: &mut ProcCommand, args: &RunLoopArgs) {
    cmd.arg("run-loop")
        .arg(args.dir.to_string_lossy().as_ref())
        .arg("--name")
        .arg(&args.name);

    if let Some(tmux_session) = args.tmux_session.as_deref() {
        if !tmux_session.trim().is_empty() {
            cmd.arg("--tmux-session").arg(tmux_session);
        }
    }
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
}

fn spawn_run_loop(
    args: &RunLoopArgs,
    process: &dyn ProcessRunner,
) -> Result<std::process::Child, CliError> {
    let exe = process.current_exe().map_err(CliError::Io)?;
    let mut cmd = if let Some(tmux_session) = args.tmux_session.as_deref() {
        if tmux_session.trim().is_empty() {
            let mut cmd = ProcCommand::new(exe);
            append_run_loop_args(&mut cmd, args);
            cmd
        } else {
            let mut cmd = ProcCommand::new("tmux");
            cmd.arg("new-session")
                .arg("-d")
                .arg("-s")
                .arg(tmux_session)
                .arg(exe);
            append_run_loop_args(&mut cmd, args);
            cmd
        }
    } else {
        let mut cmd = ProcCommand::new(exe);
        append_run_loop_args(&mut cmd, args);
        cmd
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    process
        .spawn(&mut cmd)
        .map_err(|err| CliError::Message(format!("Failed to start loop: {}", err)))
}

fn stop_session(
    store: &StateStore,
    name: &str,
    session: &serde_json::Value,
    process: &dyn ProcessRunner,
) -> Result<(), CliError> {
    if let Some(tmux) = session.get("tmux_session").and_then(|v| v.as_str()) {
        if !tmux.trim().is_empty() {
            process.kill_tmux_session(tmux);
        }
    }
    let pid = session.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
    process.kill_pid(pid);
    store
        .set_session(
            name,
            &[("status", "stopped"), ("pid", "0"), ("tmux_session", "")],
        )
        .map_err(|err| CliError::Message(err.to_string()))?;
    Ok(())
}

pub(super) fn resolve_log_file(
    name: &str,
    session: &serde_json::Value,
) -> Result<PathBuf, CliError> {
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

pub(super) fn resolve_raw_log_file(
    name: &str,
    session: &serde_json::Value,
) -> Result<PathBuf, CliError> {
    if let Some(path) = session.get("raw_log_file").and_then(|v| v.as_str()) {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    let log_file = resolve_log_file(name, session)?;
    Ok(core::raw_log_path(&log_file))
}

fn follow_log(path: &Path, fs: &dyn FileSystem, clock: &dyn core::Clock) -> Result<(), CliError> {
    let mut file = fs.open_read(path).map_err(CliError::Io)?;
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
        clock.sleep(Duration::from_millis(500));
    }
}

fn print_tail(path: &Path, lines: usize, fs: &dyn FileSystem) -> Result<(), CliError> {
    let contents = fs.read_to_string(path).map_err(CliError::Io)?;
    let total: Vec<&str> = contents.lines().collect();
    let start = total.len().saturating_sub(lines);
    for line in &total[start..] {
        println!("{}", line);
    }
    Ok(())
}

fn print_cleanup_result(action: &str, empty_message: &str, sessions: &[String]) {
    if sessions.is_empty() {
        println!("{}", empty_message);
        return;
    }
    if sessions.len() <= 10 {
        println!(
            "{} {} session(s): {}",
            action,
            sessions.len(),
            sessions.join(", ")
        );
    } else {
        println!("{} {} sessions.", action, sessions.len());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command};

    struct EnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        original_values: Vec<(String, Option<std::ffi::OsString>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            let lock = crate::test_support::env_lock();
            let keys = env_override_keys();
            let mut original_values = Vec::with_capacity(keys.len());

            for &key in keys {
                original_values.push((key.to_string(), env::var_os(key)));
                remove_env(key);
            }

            Self {
                _lock: lock,
                original_values,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.original_values {
                if let Some(value) = value {
                    unsafe {
                        env::set_var(key, value);
                    }
                } else {
                    unsafe {
                        env::remove_var(key);
                    }
                }
            }
        }
    }

    fn env_guard() -> EnvGuard {
        EnvGuard::new()
    }

    fn env_override_keys() -> &'static [&'static str] {
        &[
            "GRALPH_DEFAULT_CONFIG",
            "GRALPH_GLOBAL_CONFIG",
            "GRALPH_CONFIG_DIR",
            "GRALPH_PROJECT_CONFIG_NAME",
            "GRALPH_DEFAULTS_MAX_ITERATIONS",
            "GRALPH_DEFAULTS_TASK_FILE",
            "GRALPH_DEFAULTS_COMPLETION_MARKER",
            "GRALPH_DEFAULTS_BACKEND",
            "GRALPH_DEFAULTS_MODEL",
            "GRALPH_MAX_ITERATIONS",
            "GRALPH_TASK_FILE",
            "GRALPH_COMPLETION_MARKER",
            "GRALPH_BACKEND",
            "GRALPH_MODEL",
        ]
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

    struct PathGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        previous: Option<std::ffi::OsString>,
    }

    impl PathGuard {
        fn new(path: &Path) -> Self {
            let lock = crate::test_support::env_lock();
            let previous = env::var_os("PATH");
            set_env("PATH", path);
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous {
                set_env("PATH", value);
            } else {
                remove_env("PATH");
            }
        }
    }

    #[cfg(unix)]
    fn write_tmux_stub(dir: &Path, script: &str) -> PathBuf {
        let path = dir.join("tmux");
        fs::write(&path, script).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn set_state_env(root: &Path) -> PathBuf {
        let state_dir = root.join("state");
        set_env("GRALPH_STATE_DIR", &state_dir);
        set_env("GRALPH_STATE_FILE", state_dir.join("state.json"));
        set_env("GRALPH_LOCK_FILE", state_dir.join("state.lock"));
        state_dir
    }

    fn load_config(contents: &str) -> Config {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("default.yaml");
        write_file(&config_path, contents);
        let missing_global = temp.path().join("missing-global.yaml");

        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        set_env("GRALPH_GLOBAL_CONFIG", &missing_global);

        let config = Config::load(None).unwrap();

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");

        config
    }

    fn base_args() -> RunLoopArgs {
        RunLoopArgs {
            dir: PathBuf::from("."),
            name: "session".to_string(),
            tmux_session: None,
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

    struct TestProcessRunner {
        alive: bool,
    }

    impl ProcessRunner for TestProcessRunner {
        fn current_exe(&self) -> io::Result<PathBuf> {
            Ok(PathBuf::from("/bin/true"))
        }

        fn spawn(&self, _cmd: &mut Command) -> io::Result<Child> {
            Err(io::Error::new(io::ErrorKind::Other, "not used"))
        }

        fn kill_tmux_session(&self, _session: &str) {}

        fn kill_pid(&self, _pid: i64) {}

        fn pid(&self) -> u32 {
            0
        }

        fn is_alive(&self, _pid: i64) -> bool {
            self.alive
        }
    }

    #[derive(Debug, Clone)]
    struct CapturedCommand {
        program: String,
        args: Vec<String>,
    }

    struct CaptureProcessRunner {
        exe: PathBuf,
        captured: std::sync::Mutex<Vec<CapturedCommand>>,
    }

    impl CaptureProcessRunner {
        fn new(exe: PathBuf) -> Self {
            Self {
                exe,
                captured: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn last(&self) -> CapturedCommand {
            self.captured
                .lock()
                .unwrap()
                .last()
                .cloned()
                .expect("expected captured command")
        }
    }

    impl ProcessRunner for CaptureProcessRunner {
        fn current_exe(&self) -> io::Result<PathBuf> {
            Ok(self.exe.clone())
        }

        fn spawn(&self, cmd: &mut Command) -> io::Result<Child> {
            let program = cmd.get_program().to_string_lossy().to_string();
            let args = cmd
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect();
            self.captured
                .lock()
                .unwrap()
                .push(CapturedCommand { program, args });
            Err(io::Error::new(io::ErrorKind::Other, "capture only"))
        }

        fn kill_tmux_session(&self, _session: &str) {}

        fn kill_pid(&self, _pid: i64) {}

        fn pid(&self) -> u32 {
            0
        }

        fn is_alive(&self, _pid: i64) -> bool {
            false
        }
    }

    fn capture_run_loop_command(args: RunLoopArgs) -> CapturedCommand {
        let runner = CaptureProcessRunner::new(PathBuf::from("gralph"));
        let _ = spawn_run_loop(&args, &runner);
        runner.last()
    }

    #[test]
    fn resolve_task_file_prefers_cli_config_then_default() {
        let _guard = env_guard();
        let config = load_config("defaults:\n  task_file: Config.md\n");
        let mut args = base_args();

        args.task_file = Some("CLI.md".to_string());
        assert_eq!(resolve_task_file(&args, &config), "CLI.md");

        args.task_file = None;
        assert_eq!(resolve_task_file(&args, &config), "Config.md");

        let config = load_config("defaults:\n  backend: claude\n");
        assert_eq!(resolve_task_file(&args, &config), "PRD.md");
    }

    #[test]
    fn enrich_status_session_includes_log_and_task_fields() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join(".gralph").join("alpha.log");
        write_file(&log_path, "Ok\nIteration failed: bad\n");
        let prd_path = temp.path().join("PRD.md");
        write_file(
            &prd_path,
            "# PRD\n\n### Task UX-4\n- **ID** UX-4\n- [ ] Do\n",
        );

        let session = serde_json::json!({
            "name": "alpha",
            "status": "running",
            "pid": 123,
            "dir": temp.path().to_string_lossy(),
            "task_file": "PRD.md",
            "log_file": log_path.to_string_lossy(),
        });
        let enriched = enrich_status_session(session, &TestProcessRunner { alive: true });
        let expected_raw = core::raw_log_path(&log_path).to_string_lossy().to_string();

        assert_eq!(enriched["last_task_id"], "UX-4");
        assert_eq!(enriched["last_error"], "Iteration failed: bad");
        assert_eq!(enriched["last_log_line"], "Iteration failed: bad");
        assert_eq!(
            enriched["raw_log_file"].as_str(),
            Some(expected_raw.as_str())
        );
    }

    #[test]
    fn print_status_verbose_handles_missing_values() {
        let session = serde_json::json!({
            "name": "alpha",
            "log_file": Value::Null,
            "raw_log_file": Value::Null,
            "last_error": Value::Null,
        });

        print_status_verbose(&[session]);
    }

    #[test]
    fn resolve_max_iterations_prefers_cli_config_then_default() {
        let _guard = env_guard();
        let config = load_config("defaults:\n  max_iterations: 12\n");
        let mut args = base_args();

        args.max_iterations = Some(55);
        assert_eq!(resolve_max_iterations(&args, &config), 55);

        args.max_iterations = None;
        assert_eq!(resolve_max_iterations(&args, &config), 12);

        let config = load_config("defaults:\n  max_iterations: nope\n");
        assert_eq!(resolve_max_iterations(&args, &config), 30);
    }

    #[test]
    fn resolve_completion_marker_prefers_cli_config_then_default() {
        let _guard = env_guard();
        let config = load_config("defaults:\n  completion_marker: DONE\n");
        let mut args = base_args();

        args.completion_marker = Some("FINISH".to_string());
        assert_eq!(resolve_completion_marker(&args, &config), "FINISH");

        args.completion_marker = None;
        assert_eq!(resolve_completion_marker(&args, &config), "DONE");

        let config = load_config("defaults:\n  backend: claude\n");
        assert_eq!(resolve_completion_marker(&args, &config), "COMPLETE");
    }

    #[test]
    fn resolve_backend_prefers_cli_config_then_default() {
        let _guard = env_guard();
        let config = load_config("defaults:\n  backend: gemini\n");
        let mut args = base_args();

        args.backend = Some("codex".to_string());
        assert_eq!(resolve_backend_name(&args, &config), "codex");

        args.backend = None;
        assert_eq!(resolve_backend_name(&args, &config), "gemini");

        let config = load_config("defaults:\n  task_file: PRD.md\n");
        assert_eq!(resolve_backend_name(&args, &config), "claude");
    }

    #[test]
    fn resolve_model_prefers_cli_or_config_and_opencode_default() {
        let _guard = env_guard();
        let config = load_config(
            "defaults:\n  model: config-model\n  backend: opencode\nopencode:\n  default_model: opencode-default\n",
        );
        let mut args = base_args();

        args.model = Some("cli-model".to_string());
        assert_eq!(
            resolve_model(&args, &config, "opencode").as_deref(),
            Some("cli-model")
        );

        args.model = None;
        assert_eq!(
            resolve_model(&args, &config, "claude").as_deref(),
            Some("config-model")
        );

        let config = load_config(
            "defaults:\n  model: \"\"\n  backend: opencode\nopencode:\n  default_model: opencode-default\n",
        );
        assert_eq!(
            resolve_model(&args, &config, "opencode").as_deref(),
            Some("opencode-default")
        );
    }

    #[cfg(unix)]
    #[test]
    fn ensure_tmux_available_succeeds_with_stubbed_tmux() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 0\n");
        let _guard = PathGuard::new(temp.path());

        assert!(ensure_tmux_available().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn ensure_tmux_available_reports_non_zero_exit() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 1\n");
        let _guard = PathGuard::new(temp.path());

        let err = ensure_tmux_available().unwrap_err();
        assert_eq!(
            err.to_string(),
            "tmux is required; install tmux and try again"
        );
    }

    #[cfg(unix)]
    #[test]
    fn ensure_tmux_available_reports_missing_tmux() {
        let temp = tempfile::tempdir().unwrap();
        let _guard = PathGuard::new(temp.path());

        let err = ensure_tmux_available().unwrap_err();
        assert_eq!(
            err.to_string(),
            "tmux is required but was not found on PATH"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unique_tmux_session_name_appends_suffix_on_collision() {
        let temp = tempfile::tempdir().unwrap();
        let counter = temp.path().join("tmux-counter");
        let script = format!(
            "#!/bin/sh\ncounter=\"{}\"\nif [ -f \"$counter\" ]; then\n  exit 1\nfi\n: > \"$counter\"\nexit 0\n",
            counter.display()
        );
        write_tmux_stub(temp.path(), &script);
        let _guard = PathGuard::new(temp.path());

        let name = unique_tmux_session_name("session").unwrap();
        assert!(name.starts_with("session-"));
        assert!(name.ends_with("-2"));
    }

    #[test]
    fn spawn_run_loop_uses_exe_without_tmux_session() {
        let args = base_args();
        let captured = capture_run_loop_command(args);

        assert_eq!(captured.program, "gralph");
        assert_eq!(captured.args, vec!["run-loop", ".", "--name", "session"]);
        assert!(!captured.args.contains(&"--tmux-session".to_string()));
    }

    #[test]
    fn spawn_run_loop_skips_tmux_arg_when_session_blank() {
        let mut args = base_args();
        args.tmux_session = Some("  ".to_string());
        let captured = capture_run_loop_command(args);

        assert_eq!(captured.program, "gralph");
        assert_eq!(captured.args, vec!["run-loop", ".", "--name", "session"]);
        assert!(!captured.args.contains(&"--tmux-session".to_string()));
    }

    #[test]
    fn spawn_run_loop_uses_tmux_and_includes_tmux_session_arg() {
        let mut args = base_args();
        args.tmux_session = Some("tmux-1".to_string());
        let captured = capture_run_loop_command(args);

        assert_eq!(captured.program, "tmux");
        assert_eq!(
            captured.args,
            vec![
                "new-session",
                "-d",
                "-s",
                "tmux-1",
                "gralph",
                "run-loop",
                ".",
                "--name",
                "session",
                "--tmux-session",
                "tmux-1",
            ]
        );
    }

    #[test]
    fn should_validate_prd_matches_flag() {
        assert!(should_validate_prd(true));
        assert!(!should_validate_prd(false));
    }

    #[test]
    fn should_resume_session_handles_status_and_pid() {
        for status in ["stale", "stopped", "failed"] {
            assert!(should_resume_session(status, 123, true));
            assert!(should_resume_session(status, 0, false));
        }

        assert!(should_resume_session("running", 0, false));
        assert!(should_resume_session("running", 123, false));
        assert!(!should_resume_session("running", 123, true));
        assert!(!should_resume_session("complete", 123, false));
        assert!(!should_resume_session("unknown", 0, false));
    }

    #[test]
    fn outcome_status_plan_handles_complete_with_auto_run() {
        let plan = outcome_status_plan(LoopStatus::Complete, true);
        match plan {
            OutcomeStatusPlan::Verify {
                initial_status,
                verifying_status,
                verified_status,
                verify_failed_status,
            } => {
                assert_eq!(initial_status, "complete");
                assert_eq!(verifying_status, "verifying");
                assert_eq!(verified_status, "verified");
                assert_eq!(verify_failed_status, "verify-failed");
            }
            OutcomeStatusPlan::Final { .. } => panic!("expected verify plan"),
        }
    }

    #[test]
    fn outcome_status_plan_handles_failed_and_max_iterations() {
        let plan = outcome_status_plan(LoopStatus::Failed, true);
        assert_eq!(plan, OutcomeStatusPlan::Final { status: "failed" });

        let plan = outcome_status_plan(LoopStatus::MaxIterations, true);
        assert_eq!(
            plan,
            OutcomeStatusPlan::Final {
                status: "max_iterations",
            }
        );

        let plan = outcome_status_plan(LoopStatus::Complete, false);
        assert_eq!(plan, OutcomeStatusPlan::Final { status: "complete" });
    }

    #[test]
    fn notification_decision_maps_statuses() {
        assert_eq!(
            notification_decision(LoopStatus::Complete, true),
            Some(NotificationDecision::Complete)
        );
        assert_eq!(notification_decision(LoopStatus::Complete, false), None);
        assert_eq!(
            notification_decision(LoopStatus::Failed, true),
            Some(NotificationDecision::Failed { reason: "error" })
        );
        assert_eq!(
            notification_decision(LoopStatus::MaxIterations, true),
            Some(NotificationDecision::Failed {
                reason: "max_iterations"
            })
        );
        assert_eq!(notification_decision(LoopStatus::Running, true), None);
    }

    #[test]
    fn cmd_cleanup_marks_stale_sessions() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let state_dir = set_state_env(temp.path());
        let deps = Deps::real();
        let store = deps.state_store();
        store.init_state().unwrap();

        let state = serde_json::json!({
            "sessions": {
                "demo": {
                    "status": "running",
                    "pid": 999999
                }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        cmd_cleanup(
            CleanupArgs {
                remove: false,
                purge: false,
            },
            &deps,
        )
        .unwrap();

        let updated = fs::read_to_string(state_dir.join("state.json")).unwrap();
        let updated: Value = serde_json::from_str(&updated).unwrap();
        assert_eq!(updated["sessions"]["demo"]["status"], "stale");
    }

    #[test]
    fn cmd_cleanup_removes_stale_sessions() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let state_dir = set_state_env(temp.path());
        let deps = Deps::real();
        let store = deps.state_store();
        store.init_state().unwrap();

        let state = serde_json::json!({
            "sessions": {
                "demo": {
                    "status": "running",
                    "pid": 999999
                }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        cmd_cleanup(
            CleanupArgs {
                remove: true,
                purge: false,
            },
            &deps,
        )
        .unwrap();

        let updated = fs::read_to_string(state_dir.join("state.json")).unwrap();
        let updated: Value = serde_json::from_str(&updated).unwrap();
        assert!(updated["sessions"].get("demo").is_none());
    }

    #[test]
    fn cmd_logs_reports_missing_session() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let err = cmd_logs(
            LogsArgs {
                name: "missing".to_string(),
                follow: false,
                raw: false,
            },
            &deps,
        )
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Session not found"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_logs_reports_missing_log_file() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let state_dir = set_state_env(temp.path());
        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let missing_log = temp.path().join("missing.log");
        let state = serde_json::json!({
            "sessions": {
                "demo": {
                    "log_file": missing_log.to_string_lossy(),
                    "dir": temp.path().to_string_lossy()
                }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        let err = cmd_logs(
            LogsArgs {
                name: "demo".to_string(),
                follow: false,
                raw: false,
            },
            &deps,
        )
        .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Log file"));
            }
            other => panic!("unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn cmd_logs_prints_tail_for_existing_log() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let state_dir = set_state_env(temp.path());
        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let log_dir = temp.path().join(".gralph");
        fs::create_dir_all(&log_dir).unwrap();
        let log_file = log_dir.join("demo.log");
        fs::write(&log_file, "line one\nline two\n").unwrap();
        let state = serde_json::json!({
            "sessions": {
                "demo": {
                    "log_file": log_file.to_string_lossy(),
                    "dir": temp.path().to_string_lossy()
                }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        cmd_logs(
            LogsArgs {
                name: "demo".to_string(),
                follow: false,
                raw: false,
            },
            &deps,
        )
        .unwrap();
    }

    #[test]
    fn cmd_start_dry_run_renders_prompt() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(&temp.path().join("README.md"), "context\n");
        write_file(
            &temp.path().join(".gralph.yaml"),
            "defaults:\n  context_files: README.md\n",
        );
        write_file(
            &temp.path().join("PRD.md"),
            "# PRD\n\n## Implementation Tasks\n\n### Task T-1\n\n- **ID** T-1\n- **Context Bundle** `README.md`\n- **DoD** ok\n- **Checklist**\n  * check\n- **Dependencies** None\n- [ ] T-1 test\n",
        );

        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: None,
            max_iterations: None,
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: None,
            webhook: None,
            no_worktree: false,
            strict_prd: true,
            dry_run: true,
        };
        let deps = Deps::real();

        cmd_start_dry_run(args, &deps).unwrap();
    }

    #[test]
    fn cmd_start_rejects_nonexistent_directory() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let args = StartArgs {
            dir: missing.clone(),
            name: None,
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
            dry_run: false,
        };
        let deps = Deps::real();

        let err = cmd_start(args, &deps).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("Directory does not exist"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn run_loop_args_from_start_maps_all_fields() {
        let temp = tempfile::tempdir().unwrap();
        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: Some("custom".to_string()),
            max_iterations: Some(15),
            task_file: Some("TASKS.md".to_string()),
            completion_marker: Some("DONE".to_string()),
            backend: Some("gemini".to_string()),
            model: Some("gemini-pro".to_string()),
            variant: Some("fast".to_string()),
            prompt_template: Some(PathBuf::from("template.txt")),
            webhook: Some("https://hook.example".to_string()),
            no_worktree: true,
            strict_prd: true,
            dry_run: false,
        };

        let run_args = run_loop_args_from_start(args, "session-name".to_string()).unwrap();

        assert_eq!(run_args.dir, temp.path());
        assert_eq!(run_args.name, "session-name");
        assert_eq!(run_args.max_iterations, Some(15));
        assert_eq!(run_args.task_file, Some("TASKS.md".to_string()));
        assert_eq!(run_args.completion_marker, Some("DONE".to_string()));
        assert_eq!(run_args.backend, Some("gemini".to_string()));
        assert_eq!(run_args.model, Some("gemini-pro".to_string()));
        assert_eq!(run_args.variant, Some("fast".to_string()));
        assert_eq!(
            run_args.prompt_template,
            Some(PathBuf::from("template.txt"))
        );
        assert_eq!(run_args.webhook, Some("https://hook.example".to_string()));
        assert!(run_args.no_worktree);
        assert!(run_args.strict_prd);
    }

    #[test]
    fn run_loop_args_from_step_maps_fields_without_webhook() {
        let temp = tempfile::tempdir().unwrap();
        let args = StepArgs {
            dir: temp.path().to_path_buf(),
            name: Some("step-session".to_string()),
            max_iterations: Some(5),
            task_file: Some("STEP.md".to_string()),
            completion_marker: Some("FINISHED".to_string()),
            backend: Some("claude".to_string()),
            model: Some("opus".to_string()),
            variant: Some("slow".to_string()),
            prompt_template: Some(PathBuf::from("step-template.txt")),
            no_worktree: true,
            strict_prd: true,
        };

        let run_args = run_loop_args_from_step(args, "step-name".to_string()).unwrap();

        assert_eq!(run_args.dir, temp.path());
        assert_eq!(run_args.name, "step-name");
        assert_eq!(run_args.max_iterations, Some(5));
        assert_eq!(run_args.task_file, Some("STEP.md".to_string()));
        assert_eq!(run_args.completion_marker, Some("FINISHED".to_string()));
        assert_eq!(run_args.backend, Some("claude".to_string()));
        assert_eq!(run_args.model, Some("opus".to_string()));
        assert_eq!(run_args.variant, Some("slow".to_string()));
        assert_eq!(
            run_args.prompt_template,
            Some(PathBuf::from("step-template.txt"))
        );
        assert!(run_args.webhook.is_none());
        assert!(run_args.no_worktree);
        assert!(run_args.strict_prd);
    }

    #[test]
    fn cmd_step_rejects_nonexistent_directory() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("no-such-dir");
        let args = StepArgs {
            dir: missing.clone(),
            name: None,
            max_iterations: None,
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: None,
            no_worktree: false,
            strict_prd: false,
        };
        let deps = Deps::real();

        let err = cmd_step(args, &deps).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("Directory does not exist"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn enrich_status_session_marks_stale_when_pid_dead() {
        let session = serde_json::json!({
            "name": "test",
            "status": "running",
            "pid": 999999,
            "dir": "",
            "last_task_count": 5,
        });
        let enriched = enrich_status_session(session, &TestProcessRunner { alive: false });

        assert_eq!(enriched["status"], "stale");
        assert_eq!(enriched["is_alive"], false);
    }

    #[test]
    fn enrich_status_session_keeps_running_when_pid_alive() {
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("PRD.md"),
            "# Tasks\n- [ ] One\n- [ ] Two\n",
        );
        let session = serde_json::json!({
            "name": "test",
            "status": "running",
            "pid": 123,
            "dir": temp.path().to_string_lossy(),
            "task_file": "PRD.md",
        });
        let enriched = enrich_status_session(session, &TestProcessRunner { alive: true });

        assert_eq!(enriched["status"], "running");
        assert_eq!(enriched["is_alive"], true);
        assert_eq!(enriched["current_remaining"], 2);
    }

    #[test]
    fn enrich_status_session_uses_last_task_count_for_empty_dir() {
        let session = serde_json::json!({
            "name": "test",
            "status": "complete",
            "pid": 0,
            "dir": "",
            "last_task_count": 7,
        });
        let enriched = enrich_status_session(session, &TestProcessRunner { alive: false });

        assert_eq!(enriched["current_remaining"], 7);
        assert_eq!(enriched["last_task_id"], Value::Null);
    }

    #[test]
    fn resolve_status_log_file_uses_stored_path_when_present() {
        let mut map = Map::new();
        map.insert(
            "log_file".to_string(),
            Value::String("/var/log/test.log".to_string()),
        );
        let result = resolve_status_log_file(&map, "session", "/project");

        assert_eq!(result, Some(PathBuf::from("/var/log/test.log")));
    }

    #[test]
    fn resolve_status_log_file_constructs_path_when_missing() {
        let map = Map::new();
        let result = resolve_status_log_file(&map, "session", "/project");

        assert_eq!(result, Some(PathBuf::from("/project/.gralph/session.log")));
    }

    #[test]
    fn resolve_status_log_file_returns_none_for_empty_inputs() {
        let map = Map::new();

        assert!(resolve_status_log_file(&map, "", "/project").is_none());
        assert!(resolve_status_log_file(&map, "session", "").is_none());
    }

    #[test]
    fn resolve_status_raw_log_file_uses_stored_path_when_present() {
        let mut map = Map::new();
        map.insert(
            "raw_log_file".to_string(),
            Value::String("/var/log/raw.log".to_string()),
        );
        let result = resolve_status_raw_log_file(&map, None);

        assert_eq!(result, Some(PathBuf::from("/var/log/raw.log")));
    }

    #[test]
    fn resolve_status_raw_log_file_derives_from_log_file() {
        let map = Map::new();
        let log_file = PathBuf::from("/project/.gralph/session.log");
        let result = resolve_status_raw_log_file(&map, Some(&log_file));

        assert_eq!(result, Some(core::raw_log_path(&log_file)));
    }

    #[test]
    fn outcome_status_plan_initial_status_returns_correct_value() {
        let final_plan = OutcomeStatusPlan::Final { status: "failed" };
        assert_eq!(final_plan.initial_status(), "failed");

        let verify_plan = OutcomeStatusPlan::Verify {
            initial_status: "complete",
            verifying_status: "verifying",
            verified_status: "verified",
            verify_failed_status: "verify-failed",
        };
        assert_eq!(verify_plan.initial_status(), "complete");
    }

    #[test]
    fn outcome_status_plan_running_status_is_final() {
        let plan = outcome_status_plan(LoopStatus::Running, true);
        assert_eq!(plan, OutcomeStatusPlan::Final { status: "running" });
    }

    struct TestNotifier {
        complete_calls: std::sync::Mutex<Vec<(String, String)>>,
        failed_calls: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
    }

    impl TestNotifier {
        fn new() -> Self {
            Self {
                complete_calls: std::sync::Mutex::new(Vec::new()),
                failed_calls: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl notify::Notifier for TestNotifier {
        fn notify_complete(
            &self,
            session: &str,
            webhook: &str,
            _dir: Option<&str>,
            _iterations: Option<u32>,
            _duration_secs: Option<u64>,
            _timeout_secs: Option<u64>,
        ) -> Result<(), notify::NotifyError> {
            self.complete_calls
                .lock()
                .unwrap()
                .push((session.to_string(), webhook.to_string()));
            Ok(())
        }

        fn notify_failed(
            &self,
            session: &str,
            webhook: &str,
            reason: Option<&str>,
            _dir: Option<&str>,
            _iterations: Option<u32>,
            _max_iterations: Option<u32>,
            _remaining: Option<u32>,
            _duration_secs: Option<u64>,
            _timeout_secs: Option<u64>,
        ) -> Result<(), notify::NotifyError> {
            self.failed_calls.lock().unwrap().push((
                session.to_string(),
                webhook.to_string(),
                reason.map(|s| s.to_string()),
            ));
            Ok(())
        }
    }

    #[test]
    fn notify_if_configured_skips_without_webhook() {
        let _guard = env_guard();
        let config = load_config("defaults:\n  backend: claude\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Complete,
            iterations: 3,
            remaining_tasks: 0,
            duration_secs: 120,
        };
        let notifier = TestNotifier::new();

        let result = notify_if_configured(&config, &args, &outcome, 30, &notifier);

        assert!(result.is_ok());
        assert!(notifier.complete_calls.lock().unwrap().is_empty());
        assert!(notifier.failed_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn notify_if_configured_sends_complete_notification() {
        let _guard = env_guard();
        let config =
            load_config("notifications:\n  webhook: https://hook.test\n  on_complete: true\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Complete,
            iterations: 5,
            remaining_tasks: 0,
            duration_secs: 300,
        };
        let notifier = TestNotifier::new();

        let result = notify_if_configured(&config, &args, &outcome, 30, &notifier);

        assert!(result.is_ok());
        let calls = notifier.complete_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "session");
        assert_eq!(calls[0].1, "https://hook.test");
    }

    #[test]
    fn notify_if_configured_skips_complete_when_disabled() {
        let _guard = env_guard();
        let config =
            load_config("notifications:\n  webhook: https://hook.test\n  on_complete: false\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Complete,
            iterations: 5,
            remaining_tasks: 0,
            duration_secs: 300,
        };
        let notifier = TestNotifier::new();

        let result = notify_if_configured(&config, &args, &outcome, 30, &notifier);

        assert!(result.is_ok());
        assert!(notifier.complete_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn notify_if_configured_sends_failed_notification() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://hook.test\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Failed,
            iterations: 2,
            remaining_tasks: 5,
            duration_secs: 60,
        };
        let notifier = TestNotifier::new();

        let result = notify_if_configured(&config, &args, &outcome, 30, &notifier);

        assert!(result.is_ok());
        let calls = notifier.failed_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "session");
        assert_eq!(calls[0].1, "https://hook.test");
        assert_eq!(calls[0].2, Some("error".to_string()));
    }

    #[test]
    fn notify_if_configured_sends_max_iterations_notification() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://hook.test\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::MaxIterations,
            iterations: 30,
            remaining_tasks: 3,
            duration_secs: 1800,
        };
        let notifier = TestNotifier::new();

        let result = notify_if_configured(&config, &args, &outcome, 30, &notifier);

        assert!(result.is_ok());
        let calls = notifier.failed_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].2, Some("max_iterations".to_string()));
    }

    #[test]
    fn notify_if_configured_uses_args_webhook_over_config() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://config.hook\n");
        let mut args = base_args();
        args.webhook = Some("https://args.hook".to_string());
        let outcome = core::LoopOutcome {
            status: LoopStatus::Complete,
            iterations: 1,
            remaining_tasks: 0,
            duration_secs: 10,
        };
        let notifier = TestNotifier::new();

        let result = notify_if_configured(&config, &args, &outcome, 30, &notifier);

        assert!(result.is_ok());
        let calls = notifier.complete_calls.lock().unwrap();
        assert_eq!(calls[0].1, "https://args.hook");
    }

    #[test]
    fn stop_session_updates_state_and_kills_processes() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let _state_dir = set_state_env(temp.path());
        let store = StateStore::new_from_env();
        store.init_state().unwrap();

        store
            .set_session(
                "demo",
                &[
                    ("status", "running"),
                    ("pid", "12345"),
                    ("tmux_session", "demo-tmux"),
                ],
            )
            .unwrap();

        let session = serde_json::json!({
            "pid": 12345,
            "tmux_session": "demo-tmux",
        });

        let result = stop_session(&store, "demo", &session, &TestProcessRunner { alive: true });
        assert!(result.is_ok());

        let updated = store.get_session("demo").unwrap().unwrap();
        assert_eq!(updated["status"], "stopped");
        assert_eq!(updated["pid"].as_str().unwrap_or("0"), "0");
        assert_eq!(updated["tmux_session"], "");
    }

    #[test]
    fn stop_session_handles_empty_tmux_session() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = StateStore::new_from_env();
        store.init_state().unwrap();

        store
            .set_session("demo", &[("status", "running"), ("pid", "12345")])
            .unwrap();

        let session = serde_json::json!({
            "pid": 12345,
            "tmux_session": "   ",
        });

        let result = stop_session(&store, "demo", &session, &TestProcessRunner { alive: true });
        assert!(result.is_ok());

        let updated = store.get_session("demo").unwrap().unwrap();
        assert_eq!(updated["status"], "stopped");
    }

    #[test]
    fn cmd_stop_handles_missing_session_name() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let args = StopArgs {
            name: None,
            all: false,
        };

        let err = cmd_stop(args, &deps).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("Session name is required"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cmd_stop_handles_nonexistent_session() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let args = StopArgs {
            name: Some("missing".to_string()),
            all: false,
        };

        let err = cmd_stop(args, &deps).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("Session not found"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn resolve_tmux_session_trims_whitespace() {
        let mut args = base_args();
        args.tmux_session = Some("  session-name  ".to_string());
        assert_eq!(resolve_tmux_session(&args), "session-name");

        args.tmux_session = None;
        assert_eq!(resolve_tmux_session(&args), "");

        args.tmux_session = Some("".to_string());
        assert_eq!(resolve_tmux_session(&args), "");
    }

    #[test]
    fn format_rfc3339_produces_valid_timestamp() {
        use std::time::{SystemTime, UNIX_EPOCH};

        struct FixedClock(SystemTime);
        impl core::Clock for FixedClock {
            fn now(&self) -> SystemTime {
                self.0
            }
            fn sleep(&self, _: Duration) {}
        }

        let fixed_time = UNIX_EPOCH + Duration::from_secs(1700000000);
        let clock = FixedClock(fixed_time);
        let result = format_rfc3339(&clock);

        assert!(result.contains("2023"));
        assert!(result.contains("T"));
    }

    #[test]
    fn should_check_for_update_respects_env_var() {
        let _guard = env_guard();
        let config = load_config("defaults:\n  check_updates: true\n");

        set_env("GRALPH_NO_UPDATE_CHECK", "1");
        assert!(!should_check_for_update(&config));

        set_env("GRALPH_NO_UPDATE_CHECK", "true");
        assert!(!should_check_for_update(&config));

        set_env("GRALPH_NO_UPDATE_CHECK", "false");
        assert!(should_check_for_update(&config));

        set_env("GRALPH_NO_UPDATE_CHECK", "");
        assert!(!should_check_for_update(&config));

        remove_env("GRALPH_NO_UPDATE_CHECK");
        assert!(should_check_for_update(&config));
    }

    #[test]
    fn should_check_for_update_respects_config() {
        let _guard = env_guard();

        let config = load_config("defaults:\n  check_updates: false\n");
        assert!(!should_check_for_update(&config));

        let config = load_config("defaults:\n  check_updates: true\n");
        assert!(should_check_for_update(&config));

        let config = load_config("defaults:\n  backend: claude\n");
        assert!(should_check_for_update(&config));
    }

    #[test]
    fn append_run_loop_args_includes_all_optional_flags() {
        let mut args = base_args();
        args.max_iterations = Some(20);
        args.task_file = Some("TASKS.md".to_string());
        args.completion_marker = Some("DONE".to_string());
        args.backend = Some("gemini".to_string());
        args.model = Some("gemini-pro".to_string());
        args.variant = Some("fast".to_string());
        args.prompt_template = Some(PathBuf::from("template.txt"));
        args.webhook = Some("https://hook.test".to_string());
        args.no_worktree = true;
        args.strict_prd = true;

        let mut cmd = ProcCommand::new("gralph");
        append_run_loop_args(&mut cmd, &args);

        let cmd_args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert!(cmd_args.contains(&"--max-iterations".to_string()));
        assert!(cmd_args.contains(&"20".to_string()));
        assert!(cmd_args.contains(&"--task-file".to_string()));
        assert!(cmd_args.contains(&"TASKS.md".to_string()));
        assert!(cmd_args.contains(&"--completion-marker".to_string()));
        assert!(cmd_args.contains(&"DONE".to_string()));
        assert!(cmd_args.contains(&"--backend".to_string()));
        assert!(cmd_args.contains(&"gemini".to_string()));
        assert!(cmd_args.contains(&"--model".to_string()));
        assert!(cmd_args.contains(&"gemini-pro".to_string()));
        assert!(cmd_args.contains(&"--variant".to_string()));
        assert!(cmd_args.contains(&"fast".to_string()));
        assert!(cmd_args.contains(&"--prompt-template".to_string()));
        assert!(cmd_args.contains(&"template.txt".to_string()));
        assert!(cmd_args.contains(&"--webhook".to_string()));
        assert!(cmd_args.contains(&"https://hook.test".to_string()));
        assert!(cmd_args.contains(&"--no-worktree".to_string()));
        assert!(cmd_args.contains(&"--strict-prd".to_string()));
    }

    #[test]
    fn print_cleanup_result_handles_various_counts() {
        print_cleanup_result("Marked", "No stale sessions.", &[]);

        print_cleanup_result(
            "Removed",
            "No stale sessions.",
            &["a".to_string(), "b".to_string()],
        );

        let many: Vec<String> = (0..15).map(|i| format!("session-{}", i)).collect();
        print_cleanup_result("Purged", "No sessions.", &many);
    }

    #[test]
    fn print_table_formats_columns_correctly() {
        let headers = &["NAME", "STATUS"];
        let rows = vec![
            vec!["alpha".to_string(), "running".to_string()],
            vec!["beta-longer-name".to_string(), "ok".to_string()],
        ];

        print_table(headers, &rows);
    }

    #[test]
    fn cmd_cleanup_purges_all_sessions() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let state_dir = set_state_env(temp.path());
        let deps = Deps::real();
        let store = deps.state_store();
        store.init_state().unwrap();

        let state = serde_json::json!({
            "sessions": {
                "a": { "status": "complete" },
                "b": { "status": "stopped" }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        cmd_cleanup(
            CleanupArgs {
                remove: false,
                purge: true,
            },
            &deps,
        )
        .unwrap();

        let updated = fs::read_to_string(state_dir.join("state.json")).unwrap();
        let updated: Value = serde_json::from_str(&updated).unwrap();
        assert!(updated["sessions"].as_object().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unique_tmux_session_name_uses_gralph_prefix_for_empty_base() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 1\n");
        let _guard = PathGuard::new(temp.path());

        let name = unique_tmux_session_name("").unwrap();
        assert!(name.starts_with("gralph-"));

        let name_whitespace = unique_tmux_session_name("   ").unwrap();
        assert!(name_whitespace.starts_with("gralph-"));
    }

    #[cfg(unix)]
    #[test]
    fn unique_tmux_session_name_increments_suffix_on_multiple_collisions() {
        // Use temp directory for counter files to track call count
        let temp = tempfile::tempdir().unwrap();
        let c1 = temp.path().join("c1");
        let c2 = temp.path().join("c2");

        // Script simulates: session exists for first 2 checks, then doesn't exist
        // Uses file existence as counter (no external commands needed)
        // Call sequence:
        //   1st: c1 missing -> create c1 -> exit 0 (exists) -> suffix becomes -2
        //   2nd: c1 exists, c2 missing -> create c2 -> exit 0 (exists) -> suffix becomes -3
        //   3rd: c1 and c2 both exist -> exit 1 (not found) -> return with -3
        let script = format!(
            "#!/bin/sh\n\
             if [ ! -f \"{}\" ]; then : > \"{}\"; exit 0; fi\n\
             if [ ! -f \"{}\" ]; then : > \"{}\"; exit 0; fi\n\
             exit 1\n",
            c1.display(),
            c1.display(),
            c2.display(),
            c2.display()
        );
        write_tmux_stub(temp.path(), &script);
        let _guard = PathGuard::new(temp.path());

        let name = unique_tmux_session_name("test").unwrap();
        assert!(
            name.starts_with("test-"),
            "expected test- prefix, got: {}",
            name
        );
        // After 2 collisions the suffix should be -3
        assert!(name.ends_with("-3"), "expected -3 suffix, got: {}", name);
    }

    #[cfg(unix)]
    #[test]
    fn tmux_session_exists_returns_true_when_session_found() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 0\n");
        let _guard = PathGuard::new(temp.path());

        assert!(tmux_session_exists("any-session").unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn tmux_session_exists_returns_false_when_session_not_found() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 1\n");
        let _guard = PathGuard::new(temp.path());

        assert!(!tmux_session_exists("missing-session").unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn tmux_session_exists_errors_when_tmux_missing() {
        let temp = tempfile::tempdir().unwrap();
        let _guard = PathGuard::new(temp.path());

        let err = tmux_session_exists("session").unwrap_err();
        assert_eq!(
            err.to_string(),
            "tmux is required but was not found on PATH"
        );
    }

    #[cfg(unix)]
    #[test]
    fn cmd_attach_errors_when_session_not_found() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 0\n");
        let _path_guard = PathGuard::new(temp.path());
        let _state_dir = set_state_env(temp.path());

        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let err = cmd_attach(
            AttachArgs {
                name: "nonexistent".to_string(),
            },
            &deps,
        )
        .unwrap_err();
        assert!(err.to_string().contains("Session not found"));
    }

    #[cfg(unix)]
    #[test]
    fn cmd_attach_errors_when_no_tmux_session_recorded() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 0\n");
        let _path_guard = PathGuard::new(temp.path());
        let state_dir = set_state_env(temp.path());

        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let state = serde_json::json!({
            "sessions": {
                "mysession": { "status": "running", "tmux_session": "" }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        let err = cmd_attach(
            AttachArgs {
                name: "mysession".to_string(),
            },
            &deps,
        )
        .unwrap_err();
        assert!(err.to_string().contains("No tmux session recorded"));
    }

    #[cfg(unix)]
    #[test]
    fn cmd_attach_errors_when_tmux_session_whitespace_only() {
        let temp = tempfile::tempdir().unwrap();
        write_tmux_stub(temp.path(), "#!/bin/sh\nexit 0\n");
        let _path_guard = PathGuard::new(temp.path());
        let state_dir = set_state_env(temp.path());

        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let state = serde_json::json!({
            "sessions": {
                "mysession": { "status": "running", "tmux_session": "   " }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        let err = cmd_attach(
            AttachArgs {
                name: "mysession".to_string(),
            },
            &deps,
        )
        .unwrap_err();
        assert!(err.to_string().contains("No tmux session recorded"));
    }

    #[cfg(unix)]
    #[test]
    fn cmd_attach_errors_when_tmux_unavailable() {
        let temp = tempfile::tempdir().unwrap();
        let _path_guard = PathGuard::new(temp.path());
        let _state_dir = set_state_env(temp.path());

        let deps = Deps::real();

        let err = cmd_attach(
            AttachArgs {
                name: "session".to_string(),
            },
            &deps,
        )
        .unwrap_err();
        assert!(err.to_string().contains("tmux is required"));
    }

    #[cfg(unix)]
    #[test]
    fn cmd_attach_reports_failed_attach_with_exit_code() {
        let temp = tempfile::tempdir().unwrap();
        let script = r#"#!/bin/sh
if [ "$1" = "-V" ]; then exit 0; fi
if [ "$1" = "attach-session" ]; then exit 42; fi
exit 0
"#;
        write_tmux_stub(temp.path(), script);
        let _path_guard = PathGuard::new(temp.path());
        let state_dir = set_state_env(temp.path());

        let deps = Deps::real();
        deps.state_store().init_state().unwrap();

        let state = serde_json::json!({
            "sessions": {
                "mysession": { "status": "running", "tmux_session": "tmux-123" }
            }
        });
        fs::write(
            state_dir.join("state.json"),
            serde_json::to_string(&state).unwrap(),
        )
        .unwrap();

        let err = cmd_attach(
            AttachArgs {
                name: "mysession".to_string(),
            },
            &deps,
        )
        .unwrap_err();
        assert!(err.to_string().contains("Failed to attach"));
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn spawn_run_loop_propagates_current_exe_error() {
        struct FailExeRunner;
        impl ProcessRunner for FailExeRunner {
            fn current_exe(&self) -> io::Result<PathBuf> {
                Err(io::Error::new(io::ErrorKind::NotFound, "exe not found"))
            }
            fn spawn(&self, _cmd: &mut Command) -> io::Result<Child> {
                unreachable!()
            }
            fn kill_tmux_session(&self, _session: &str) {}
            fn kill_pid(&self, _pid: i64) {}
            fn pid(&self) -> u32 {
                0
            }
            fn is_alive(&self, _pid: i64) -> bool {
                false
            }
        }

        let args = base_args();
        let err = spawn_run_loop(&args, &FailExeRunner).unwrap_err();
        assert!(matches!(err, CliError::Io(_)));
    }

    // COV80-LS-3: failure path tests for notification flows, max iteration exits, and callbacks

    struct FailingNotifier {
        error_message: String,
    }

    impl FailingNotifier {
        fn new(message: &str) -> Self {
            Self {
                error_message: message.to_string(),
            }
        }
    }

    impl notify::Notifier for FailingNotifier {
        fn notify_complete(
            &self,
            _session: &str,
            _webhook: &str,
            _dir: Option<&str>,
            _iterations: Option<u32>,
            _duration_secs: Option<u64>,
            _timeout_secs: Option<u64>,
        ) -> Result<(), notify::NotifyError> {
            Err(notify::NotifyError::InvalidInput(
                self.error_message.clone(),
            ))
        }

        fn notify_failed(
            &self,
            _session: &str,
            _webhook: &str,
            _reason: Option<&str>,
            _dir: Option<&str>,
            _iterations: Option<u32>,
            _max_iterations: Option<u32>,
            _remaining: Option<u32>,
            _duration_secs: Option<u64>,
            _timeout_secs: Option<u64>,
        ) -> Result<(), notify::NotifyError> {
            Err(notify::NotifyError::InvalidInput(
                self.error_message.clone(),
            ))
        }
    }

    #[test]
    fn notify_if_configured_propagates_complete_notification_error() {
        let _guard = env_guard();
        let config =
            load_config("notifications:\n  webhook: https://hook.test\n  on_complete: true\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Complete,
            iterations: 5,
            remaining_tasks: 0,
            duration_secs: 300,
        };
        let notifier = FailingNotifier::new("complete notification failed");

        let err = notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("complete notification failed"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn notify_if_configured_propagates_failed_notification_error() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://hook.test\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Failed,
            iterations: 2,
            remaining_tasks: 5,
            duration_secs: 60,
        };
        let notifier = FailingNotifier::new("failed notification error");

        let err = notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("failed notification error"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn notify_if_configured_propagates_max_iterations_notification_error() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://hook.test\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::MaxIterations,
            iterations: 30,
            remaining_tasks: 3,
            duration_secs: 1800,
        };
        let notifier = FailingNotifier::new("max iterations notification error");

        let err = notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("max iterations notification error"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn notification_decision_running_returns_none() {
        // Running status should never trigger a notification
        assert!(notification_decision(LoopStatus::Running, true).is_none());
        assert!(notification_decision(LoopStatus::Running, false).is_none());
    }

    #[test]
    fn notification_decision_failed_ignores_on_complete_flag() {
        // Failed notifications should always be sent regardless of on_complete setting
        assert_eq!(
            notification_decision(LoopStatus::Failed, true),
            Some(NotificationDecision::Failed { reason: "error" })
        );
        assert_eq!(
            notification_decision(LoopStatus::Failed, false),
            Some(NotificationDecision::Failed { reason: "error" })
        );
    }

    #[test]
    fn notification_decision_max_iterations_ignores_on_complete_flag() {
        // MaxIterations notifications should always be sent regardless of on_complete setting
        assert_eq!(
            notification_decision(LoopStatus::MaxIterations, true),
            Some(NotificationDecision::Failed {
                reason: "max_iterations"
            })
        );
        assert_eq!(
            notification_decision(LoopStatus::MaxIterations, false),
            Some(NotificationDecision::Failed {
                reason: "max_iterations"
            })
        );
    }

    #[test]
    fn outcome_status_plan_complete_without_auto_run_is_final() {
        let plan = outcome_status_plan(LoopStatus::Complete, false);
        assert_eq!(plan, OutcomeStatusPlan::Final { status: "complete" });
    }

    #[test]
    fn outcome_status_plan_failed_with_auto_run_is_still_final() {
        // Failed status should be final even if auto_run is enabled
        let plan = outcome_status_plan(LoopStatus::Failed, true);
        assert_eq!(plan, OutcomeStatusPlan::Final { status: "failed" });
    }

    #[test]
    fn outcome_status_plan_max_iterations_with_auto_run_is_final() {
        // MaxIterations status should be final even if auto_run is enabled
        let plan = outcome_status_plan(LoopStatus::MaxIterations, true);
        assert_eq!(
            plan,
            OutcomeStatusPlan::Final {
                status: "max_iterations"
            }
        );
    }

    struct CallTrackingNotifier {
        complete_count: std::sync::atomic::AtomicUsize,
        failed_count: std::sync::atomic::AtomicUsize,
        last_reason: std::sync::Mutex<Option<String>>,
    }

    impl CallTrackingNotifier {
        fn new() -> Self {
            Self {
                complete_count: std::sync::atomic::AtomicUsize::new(0),
                failed_count: std::sync::atomic::AtomicUsize::new(0),
                last_reason: std::sync::Mutex::new(None),
            }
        }

        fn complete_count(&self) -> usize {
            self.complete_count
                .load(std::sync::atomic::Ordering::SeqCst)
        }

        fn failed_count(&self) -> usize {
            self.failed_count.load(std::sync::atomic::Ordering::SeqCst)
        }

        fn last_reason(&self) -> Option<String> {
            self.last_reason.lock().unwrap().clone()
        }
    }

    impl notify::Notifier for CallTrackingNotifier {
        fn notify_complete(
            &self,
            _session: &str,
            _webhook: &str,
            _dir: Option<&str>,
            _iterations: Option<u32>,
            _duration_secs: Option<u64>,
            _timeout_secs: Option<u64>,
        ) -> Result<(), notify::NotifyError> {
            self.complete_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn notify_failed(
            &self,
            _session: &str,
            _webhook: &str,
            reason: Option<&str>,
            _dir: Option<&str>,
            _iterations: Option<u32>,
            _max_iterations: Option<u32>,
            _remaining: Option<u32>,
            _duration_secs: Option<u64>,
            _timeout_secs: Option<u64>,
        ) -> Result<(), notify::NotifyError> {
            self.failed_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.last_reason.lock().unwrap() = reason.map(|s| s.to_string());
            Ok(())
        }
    }

    #[test]
    fn notify_if_configured_calls_complete_callback_once() {
        let _guard = env_guard();
        let config =
            load_config("notifications:\n  webhook: https://hook.test\n  on_complete: true\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Complete,
            iterations: 5,
            remaining_tasks: 0,
            duration_secs: 300,
        };
        let notifier = CallTrackingNotifier::new();

        notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap();

        assert_eq!(notifier.complete_count(), 1);
        assert_eq!(notifier.failed_count(), 0);
    }

    #[test]
    fn notify_if_configured_calls_failed_callback_for_error() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://hook.test\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Failed,
            iterations: 2,
            remaining_tasks: 5,
            duration_secs: 60,
        };
        let notifier = CallTrackingNotifier::new();

        notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap();

        assert_eq!(notifier.complete_count(), 0);
        assert_eq!(notifier.failed_count(), 1);
        assert_eq!(notifier.last_reason(), Some("error".to_string()));
    }

    #[test]
    fn notify_if_configured_calls_failed_callback_for_max_iterations() {
        let _guard = env_guard();
        let config = load_config("notifications:\n  webhook: https://hook.test\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::MaxIterations,
            iterations: 30,
            remaining_tasks: 3,
            duration_secs: 1800,
        };
        let notifier = CallTrackingNotifier::new();

        notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap();

        assert_eq!(notifier.complete_count(), 0);
        assert_eq!(notifier.failed_count(), 1);
        assert_eq!(notifier.last_reason(), Some("max_iterations".to_string()));
    }

    #[test]
    fn notify_if_configured_skips_callback_for_running_status() {
        let _guard = env_guard();
        let config =
            load_config("notifications:\n  webhook: https://hook.test\n  on_complete: true\n");
        let args = base_args();
        let outcome = core::LoopOutcome {
            status: LoopStatus::Running,
            iterations: 1,
            remaining_tasks: 10,
            duration_secs: 30,
        };
        let notifier = CallTrackingNotifier::new();

        notify_if_configured(&config, &args, &outcome, 30, &notifier).unwrap();

        assert_eq!(notifier.complete_count(), 0);
        assert_eq!(notifier.failed_count(), 0);
    }

    #[test]
    fn outcome_status_plan_verify_plan_all_fields_accessible() {
        let plan = outcome_status_plan(LoopStatus::Complete, true);
        match plan {
            OutcomeStatusPlan::Verify {
                initial_status,
                verifying_status,
                verified_status,
                verify_failed_status,
            } => {
                // Verify all status values are distinct and meaningful
                assert_eq!(initial_status, "complete");
                assert_eq!(verifying_status, "verifying");
                assert_eq!(verified_status, "verified");
                assert_eq!(verify_failed_status, "verify-failed");
                // Verify initial_status() accessor works
                assert_eq!(plan.initial_status(), "complete");
            }
            OutcomeStatusPlan::Final { .. } => panic!("expected verify plan"),
        }
    }

    #[test]
    fn outcome_status_plan_final_initial_status_matches() {
        for (status, expected) in [
            (LoopStatus::Failed, "failed"),
            (LoopStatus::MaxIterations, "max_iterations"),
            (LoopStatus::Running, "running"),
        ] {
            let plan = outcome_status_plan(status, true);
            match plan {
                OutcomeStatusPlan::Final { status: s } => {
                    assert_eq!(s, expected);
                    assert_eq!(plan.initial_status(), expected);
                }
                OutcomeStatusPlan::Verify { .. } => {
                    panic!("expected final plan for status {:?}", status)
                }
            }
        }
    }

    #[test]
    fn cmd_start_dry_run_prints_none_when_no_task_block() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("PRD.md"),
            "# PRD\n\n## Implementation Tasks\n\n### Task T-1\n\n- **ID** T-1\n- **Context Bundle** `README.md`\n- **DoD** ok\n- **Checklist**\n  * check\n- **Dependencies** None\n- [x] T-1 completed\n",
        );

        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: None,
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
            dry_run: true,
        };
        let deps = Deps::real();

        // Should succeed and print "(none)" for task block
        cmd_start_dry_run(args, &deps).unwrap();
    }

    #[test]
    fn cmd_start_dry_run_uses_custom_prompt_template() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("PRD.md"),
            "# PRD\n\n## Implementation Tasks\n\n### Task T-1\n\n- **ID** T-1\n- **Context Bundle** `README.md`\n- **DoD** ok\n- **Checklist**\n  * check\n- **Dependencies** None\n- [ ] T-1 do\n",
        );
        let template_path = temp.path().join("custom-template.txt");
        write_file(
            &template_path,
            "Custom template: {task_file} iteration {iteration}/{max_iterations}\nTask: {task_block}\n",
        );

        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: None,
            max_iterations: Some(10),
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: Some(template_path),
            webhook: None,
            no_worktree: false,
            strict_prd: false,
            dry_run: true,
        };
        let deps = Deps::real();

        cmd_start_dry_run(args, &deps).unwrap();
    }

    #[test]
    fn cmd_start_dry_run_fails_on_invalid_prd_with_strict() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(
            &temp.path().join("PRD.md"),
            "# PRD\n\n- [ ] Task without proper structure\n",
        );

        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: None,
            max_iterations: None,
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: None,
            webhook: None,
            no_worktree: false,
            strict_prd: true,
            dry_run: true,
        };
        let deps = Deps::real();

        let err = cmd_start_dry_run(args, &deps).unwrap_err();
        match err {
            CliError::Message(msg) => {
                assert!(msg.contains("task") || msg.contains("Task"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cmd_start_dry_run_fails_on_missing_template() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        write_file(&temp.path().join("PRD.md"), "# PRD\n- [ ] Task\n");

        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: None,
            max_iterations: None,
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: Some(temp.path().join("missing-template.txt")),
            webhook: None,
            no_worktree: false,
            strict_prd: false,
            dry_run: true,
        };
        let deps = Deps::real();

        let err = cmd_start_dry_run(args, &deps).unwrap_err();
        match err {
            CliError::Io(_) => {}
            other => panic!("expected IO error, got: {other:?}"),
        }
    }

    #[test]
    fn run_loop_args_from_start_preserves_none_values() {
        let temp = tempfile::tempdir().unwrap();
        let args = StartArgs {
            dir: temp.path().to_path_buf(),
            name: None,
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
            dry_run: false,
        };

        let run_args = run_loop_args_from_start(args, "session".to_string()).unwrap();

        assert!(run_args.max_iterations.is_none());
        assert!(run_args.task_file.is_none());
        assert!(run_args.completion_marker.is_none());
        assert!(run_args.backend.is_none());
        assert!(run_args.model.is_none());
        assert!(run_args.variant.is_none());
        assert!(run_args.prompt_template.is_none());
        assert!(run_args.webhook.is_none());
        assert!(run_args.tmux_session.is_none());
    }

    #[test]
    fn run_loop_args_from_step_preserves_none_values() {
        let temp = tempfile::tempdir().unwrap();
        let args = StepArgs {
            dir: temp.path().to_path_buf(),
            name: None,
            max_iterations: None,
            task_file: None,
            completion_marker: None,
            backend: None,
            model: None,
            variant: None,
            prompt_template: None,
            no_worktree: false,
            strict_prd: false,
        };

        let run_args = run_loop_args_from_step(args, "step".to_string()).unwrap();

        assert!(run_args.max_iterations.is_none());
        assert!(run_args.task_file.is_none());
        assert!(run_args.completion_marker.is_none());
        assert!(run_args.backend.is_none());
        assert!(run_args.model.is_none());
        assert!(run_args.variant.is_none());
        assert!(run_args.prompt_template.is_none());
        assert!(run_args.webhook.is_none());
        assert!(run_args.tmux_session.is_none());
    }
}
