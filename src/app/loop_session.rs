use super::{CliError, Deps, FileSystem, ProcessRunner};
use crate::backend::backend_from_name;
use crate::cli::{LogsArgs, ResumeArgs, RunLoopArgs, StartArgs, StopArgs};
use crate::config::Config;
use crate::core::{self, LoopStatus};
use crate::notify;
use crate::prd;
use crate::state::{CleanupMode, StateStore};
use crate::update;
use crate::verifier;
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
    let no_tmux = args.no_tmux;
    let session_name = super::session_name(&args.name, &args.dir)?;
    let config = Config::load(Some(&args.dir)).map_err(|err| CliError::Message(err.to_string()))?;
    let mut run_args = run_loop_args_from_start(args, session_name)?;
    deps.worktree()
        .maybe_create_auto_worktree(&mut run_args, &config)?;
    if no_tmux {
        return run_loop_with_state(run_args, deps);
    }

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

pub(super) fn cmd_status(deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
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

pub(super) fn cmd_logs(args: LogsArgs, deps: &Deps) -> Result<(), CliError> {
    let store = deps.state_store();
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
        follow_log(&log_file, deps.fs(), deps.clock())?;
    } else {
        print_tail(&log_file, 200, deps.fs())?;
    }
    Ok(())
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
    maybe_check_for_update();
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
    let now = format_rfc3339(deps.clock());
    let remaining = core::count_remaining_tasks(&args.dir.join(&task_file));
    let log_file = args.dir.join(".gralph").join(format!("{}.log", args.name));

    store
        .set_session(
            &args.name,
            &[
                ("dir", &args.dir.to_string_lossy()),
                ("task_file", &task_file),
                ("pid", &deps.process().pid().to_string()),
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

    let auto_run_verifier = verifier::resolve_verifier_auto_run(&config);
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

fn spawn_run_loop(
    args: &RunLoopArgs,
    process: &dyn ProcessRunner,
) -> Result<std::process::Child, CliError> {
    let exe = process.current_exe().map_err(CliError::Io)?;
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
    use std::path::{Path, PathBuf};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = crate::test_support::env_lock();
        clear_env_overrides();
        guard
    }

    fn clear_env_overrides() {
        for key in [
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
        ] {
            remove_env(key);
        }
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

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
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
}
