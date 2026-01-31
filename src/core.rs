use crate::backend::{Backend, BackendError};
use crate::config::Config;
use crate::task::{is_task_header, is_unchecked_line, task_blocks_from_contents};
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_PROMPT_TEMPLATE: &str = "Read {task_file} carefully. Find any task marked '- [ ]' (unchecked).\n\nIf unchecked tasks exist:\n- Complete ONE task fully\n- Mark it '- [x]' in {task_file}\n- Commit changes with a concise, lower-case conventional commit message (e.g. 'feat: add worktree collision checks')\n- Exit normally (do NOT output completion promise)\n\nIf ZERO '- [ ]' remain (all complete):\n- Verify by searching the file\n- Output ONLY: <promise>{completion_marker}</promise>\n\nCRITICAL: Never mention the promise unless outputting it as the completion signal.\n\n{context_files_section}Task Block:\n{task_block}\n\nIteration: {iteration}/{max_iterations}";

pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
    fn sleep(&self, duration: Duration);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

#[derive(Debug)]
pub enum CoreError {
    Io { path: PathBuf, source: io::Error },
    Backend(BackendError),
    InvalidInput(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io { path, source } => {
                write!(f, "core io error at {}: {}", path.display(), source)
            }
            CoreError::Backend(error) => write!(f, "backend error: {}", error),
            CoreError::InvalidInput(message) => write!(f, "invalid input: {}", message),
        }
    }
}

impl Error for CoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CoreError::Io { source, .. } => Some(source),
            CoreError::Backend(error) => Some(error),
            CoreError::InvalidInput(_) => None,
        }
    }
}

impl From<BackendError> for CoreError {
    fn from(error: BackendError) -> Self {
        CoreError::Backend(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopStatus {
    Running,
    Failed,
    Complete,
    MaxIterations,
}

impl LoopStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoopStatus::Running => "running",
            LoopStatus::Failed => "failed",
            LoopStatus::Complete => "complete",
            LoopStatus::MaxIterations => "max_iterations",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IterationResult {
    pub result: String,
    pub raw_output_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LoopOutcome {
    pub status: LoopStatus,
    pub iterations: u32,
    pub remaining_tasks: usize,
    pub duration_secs: u64,
}

#[derive(Debug, Clone)]
pub struct PromptRender {
    pub prompt: String,
    pub task_block: Option<String>,
}

pub fn render_iteration_prompt(
    project_dir: &Path,
    task_file: &str,
    iteration: u32,
    max_iterations: u32,
    completion_marker: &str,
    prompt_template: Option<&str>,
    config: Option<&Config>,
) -> Result<PromptRender, CoreError> {
    if project_dir.as_os_str().is_empty() {
        return Err(CoreError::InvalidInput(
            "project_dir is required".to_string(),
        ));
    }
    if iteration == 0 {
        return Err(CoreError::InvalidInput(
            "iteration number is required".to_string(),
        ));
    }
    if max_iterations == 0 {
        return Err(CoreError::InvalidInput(
            "max_iterations is required".to_string(),
        ));
    }

    if !project_dir.is_dir() {
        return Err(CoreError::InvalidInput(format!(
            "project directory does not exist: {}",
            project_dir.display()
        )));
    }

    let full_task_path = project_dir.join(task_file);
    if !full_task_path.is_file() {
        return Err(CoreError::InvalidInput(format!(
            "task file does not exist: {}",
            full_task_path.display()
        )));
    }

    let resolved_template = resolve_prompt_template(project_dir, prompt_template)?;
    let mut task_block = get_next_unchecked_task_block(&full_task_path)?;
    if task_block.is_none() {
        let remaining = count_remaining_tasks(&full_task_path);
        if remaining > 0 {
            task_block = first_unchecked_line(&full_task_path)?;
        }
    }

    let context_files = config
        .and_then(|cfg| cfg.get("defaults.context_files"))
        .unwrap_or_default();
    let normalized_context_files = normalize_context_files(&context_files);

    let prompt = render_prompt_template(
        &resolved_template,
        task_file,
        completion_marker,
        iteration,
        max_iterations,
        task_block.as_deref(),
        if normalized_context_files.is_empty() {
            None
        } else {
            Some(normalized_context_files.as_str())
        },
    );

    Ok(PromptRender { prompt, task_block })
}

pub fn run_iteration<B: Backend + ?Sized>(
    backend: &B,
    project_dir: &Path,
    task_file: &str,
    iteration: u32,
    max_iterations: u32,
    completion_marker: &str,
    model: Option<&str>,
    variant: Option<&str>,
    log_file: Option<&Path>,
    prompt_template: Option<&str>,
    config: Option<&Config>,
) -> Result<IterationResult, CoreError> {
    run_iteration_with_clock(
        backend,
        project_dir,
        task_file,
        iteration,
        max_iterations,
        completion_marker,
        model,
        variant,
        log_file,
        prompt_template,
        config,
        &SystemClock,
    )
}

pub fn run_iteration_with_clock<B: Backend + ?Sized>(
    backend: &B,
    project_dir: &Path,
    task_file: &str,
    iteration: u32,
    max_iterations: u32,
    completion_marker: &str,
    model: Option<&str>,
    variant: Option<&str>,
    log_file: Option<&Path>,
    prompt_template: Option<&str>,
    config: Option<&Config>,
    clock: &dyn Clock,
) -> Result<IterationResult, CoreError> {
    if project_dir.as_os_str().is_empty() {
        return Err(CoreError::InvalidInput(
            "project_dir is required".to_string(),
        ));
    }
    if iteration == 0 {
        return Err(CoreError::InvalidInput(
            "iteration number is required".to_string(),
        ));
    }
    if max_iterations == 0 {
        return Err(CoreError::InvalidInput(
            "max_iterations is required".to_string(),
        ));
    }

    if !project_dir.is_dir() {
        return Err(CoreError::InvalidInput(format!(
            "project directory does not exist: {}",
            project_dir.display()
        )));
    }

    let full_task_path = project_dir.join(task_file);
    if !full_task_path.is_file() {
        return Err(CoreError::InvalidInput(format!(
            "task file does not exist: {}",
            full_task_path.display()
        )));
    }

    if !backend.check_installed() {
        return Err(CoreError::InvalidInput(
            "backend is not installed".to_string(),
        ));
    }

    let tmpfile = create_temp_file_with_clock("gralph-iteration", clock)?;

    let raw_output_file = log_file.map(|path| raw_log_path(path));

    let prompt = render_iteration_prompt(
        project_dir,
        task_file,
        iteration,
        max_iterations,
        completion_marker,
        prompt_template,
        config,
    )?
    .prompt;

    let backend_result = backend.run_iteration(&prompt, model, variant, &tmpfile, project_dir);

    if let Some(raw_path) = raw_output_file.as_ref() {
        if let Err(err) = copy_if_exists(&tmpfile, raw_path) {
            log_message(
                log_file,
                &format!("Warning: failed to copy raw output: {}", err),
            )?;
        }
    }

    if backend_result.is_err() {
        if fs::metadata(&tmpfile).map(|meta| meta.len()).unwrap_or(0) == 0 {
            if let Some(raw_path) = raw_output_file.as_ref() {
                log_message(
                    log_file,
                    &format!("Raw output saved to: {}", raw_path.display()),
                )?;
            }
        }
        return Err(backend_result.err().unwrap().into());
    }

    if fs::metadata(&tmpfile).map(|meta| meta.len()).unwrap_or(0) == 0 {
        log_message(log_file, "Error: backend produced no JSON output.")?;
        if let Some(raw_path) = raw_output_file.as_ref() {
            log_message(
                log_file,
                &format!("Raw output saved to: {}", raw_path.display()),
            )?;
        }
        return Err(CoreError::InvalidInput(
            "backend produced no output".to_string(),
        ));
    }

    let result = backend.parse_text(&tmpfile)?;
    if result.trim().is_empty() {
        log_message(log_file, "Error: backend returned no parsed result.")?;
        if let Some(raw_path) = raw_output_file.as_ref() {
            log_message(
                log_file,
                &format!("Raw output saved to: {}", raw_path.display()),
            )?;
        }
        return Err(CoreError::InvalidInput(
            "backend returned no parsed result".to_string(),
        ));
    }

    Ok(IterationResult {
        result,
        raw_output_file,
    })
}

pub fn count_remaining_tasks(task_file: &Path) -> usize {
    if task_file.as_os_str().is_empty() || !task_file.is_file() {
        return 0;
    }

    let contents = match fs::read_to_string(task_file) {
        Ok(contents) => contents,
        Err(_) => return 0,
    };

    if contents.lines().any(is_task_header) {
        let mut count = 0;
        for block in task_blocks_from_contents(&contents) {
            count += block.lines().filter(|line| is_unchecked_line(line)).count();
        }
        count
    } else {
        contents
            .lines()
            .filter(|line| is_unchecked_line(line))
            .count()
    }
}

pub fn check_completion(
    task_file: &Path,
    result: &str,
    completion_marker: &str,
) -> Result<bool, CoreError> {
    if task_file.as_os_str().is_empty() {
        return Err(CoreError::InvalidInput("task_file is required".to_string()));
    }
    if result.trim().is_empty() {
        return Ok(false);
    }
    if !task_file.is_file() {
        return Err(CoreError::InvalidInput(format!(
            "task file does not exist: {}",
            task_file.display()
        )));
    }

    let remaining = count_remaining_tasks(task_file);
    if remaining > 0 {
        return Ok(false);
    }

    let promise_line = last_non_empty_line(result).unwrap_or_default();
    if is_negated_promise(&promise_line) {
        return Ok(false);
    }

    let expected = format!("<promise>{}</promise>", completion_marker);
    if promise_line.trim() != expected {
        return Ok(false);
    }

    Ok(true)
}

pub fn run_loop<B: Backend + ?Sized>(
    backend: &B,
    project_dir: &Path,
    task_file: Option<&str>,
    max_iterations: Option<u32>,
    completion_marker: Option<&str>,
    model: Option<&str>,
    variant: Option<&str>,
    session_name: Option<&str>,
    prompt_template: Option<&str>,
    config: Option<&Config>,
    state_callback: Option<&mut dyn FnMut(Option<&str>, u32, LoopStatus, usize)>,
) -> Result<LoopOutcome, CoreError> {
    run_loop_with_clock(
        backend,
        project_dir,
        task_file,
        max_iterations,
        completion_marker,
        model,
        variant,
        session_name,
        prompt_template,
        config,
        state_callback,
        &SystemClock,
    )
}

pub fn run_loop_with_clock<B: Backend + ?Sized>(
    backend: &B,
    project_dir: &Path,
    task_file: Option<&str>,
    max_iterations: Option<u32>,
    completion_marker: Option<&str>,
    model: Option<&str>,
    variant: Option<&str>,
    session_name: Option<&str>,
    prompt_template: Option<&str>,
    config: Option<&Config>,
    mut state_callback: Option<&mut dyn FnMut(Option<&str>, u32, LoopStatus, usize)>,
    clock: &dyn Clock,
) -> Result<LoopOutcome, CoreError> {
    if project_dir.as_os_str().is_empty() {
        return Err(CoreError::InvalidInput(
            "project_dir is required".to_string(),
        ));
    }

    let max_iterations = max_iterations.unwrap_or(30);
    if max_iterations == 0 {
        return Err(CoreError::InvalidInput(
            "max_iterations must be a positive integer".to_string(),
        ));
    }

    let project_dir = project_dir.canonicalize().map_err(|source| CoreError::Io {
        path: project_dir.to_path_buf(),
        source,
    })?;

    if !project_dir.is_dir() {
        return Err(CoreError::InvalidInput(format!(
            "project directory does not exist: {}",
            project_dir.display()
        )));
    }

    let task_file = task_file.unwrap_or("PRD.md");
    let completion_marker = completion_marker.unwrap_or("COMPLETE");
    let full_task_path = project_dir.join(task_file);
    if !full_task_path.is_file() {
        return Err(CoreError::InvalidInput(format!(
            "task file does not exist: {}",
            full_task_path.display()
        )));
    }

    let gralph_dir = project_dir.join(".gralph");
    fs::create_dir_all(&gralph_dir).map_err(|source| CoreError::Io {
        path: gralph_dir.clone(),
        source,
    })?;

    cleanup_old_logs_with_clock(&gralph_dir, config, clock)?;

    let log_name = session_name.unwrap_or("gralph");
    let log_file = gralph_dir.join(format!("{}.log", log_name));

    let loop_start = clock.now();
    let mut iteration = 1;

    log_message(
        Some(&log_file),
        &format!("Starting gralph loop in {}", project_dir.display()),
    )?;
    log_message(Some(&log_file), &format!("Task file: {}", task_file))?;
    log_message(
        Some(&log_file),
        &format!("Max iterations: {}", max_iterations),
    )?;
    log_message(
        Some(&log_file),
        &format!("Completion marker: {}", completion_marker),
    )?;
    if let Some(model) = model {
        log_message(Some(&log_file), &format!("Model: {}", model))?;
    }
    if let Some(variant) = variant {
        log_message(Some(&log_file), &format!("Variant: {}", variant))?;
    }
    log_message(
        Some(&log_file),
        &format!("Started at: {}", format_timestamp(loop_start)),
    )?;

    let initial_remaining = count_remaining_tasks(&full_task_path);
    log_message(
        Some(&log_file),
        &format!("Initial remaining tasks: {}", initial_remaining),
    )?;

    while iteration <= max_iterations {
        let remaining_before = count_remaining_tasks(&full_task_path);

        log_message(Some(&log_file), "")?;
        log_message(
            Some(&log_file),
            &format!(
                "=== Iteration {}/{} (Remaining: {}) ===",
                iteration, max_iterations, remaining_before
            ),
        )?;

        if let Some(callback) = state_callback.as_deref_mut() {
            callback(
                session_name,
                iteration,
                LoopStatus::Running,
                remaining_before,
            );
        }

        if remaining_before == 0 {
            log_message(
                Some(&log_file),
                "Zero tasks remaining before iteration, verifying completion...",
            )?;
        }

        let iteration_result = run_iteration(
            backend,
            &project_dir,
            task_file,
            iteration,
            max_iterations,
            completion_marker,
            model,
            variant,
            Some(&log_file),
            prompt_template,
            config,
        );

        if let Err(error) = iteration_result {
            if let Some(callback) = state_callback.as_deref_mut() {
                callback(
                    session_name,
                    iteration,
                    LoopStatus::Failed,
                    remaining_before,
                );
            }
            log_message(Some(&log_file), &format!("Iteration failed: {}", error))?;
            return Err(error);
        }

        let iteration_result = iteration_result.unwrap();

        if check_completion(&full_task_path, &iteration_result.result, completion_marker)? {
            let duration_secs = clock
                .now()
                .duration_since(loop_start)
                .unwrap_or_default()
                .as_secs();

            log_message(Some(&log_file), "")?;
            log_message(
                Some(&log_file),
                &format!("Gralph complete after {} iterations.", iteration),
            )?;
            log_message(
                Some(&log_file),
                &format!("Duration: {}", format_duration(duration_secs)),
            )?;
            log_message(
                Some(&log_file),
                &format!("FINISHED: {}", format_timestamp(clock.now())),
            )?;

            if let Some(callback) = state_callback.as_deref_mut() {
                callback(session_name, iteration, LoopStatus::Complete, 0);
            }

            return Ok(LoopOutcome {
                status: LoopStatus::Complete,
                iterations: iteration,
                remaining_tasks: 0,
                duration_secs,
            });
        }

        let remaining_after = count_remaining_tasks(&full_task_path);
        log_message(
            Some(&log_file),
            &format!("Tasks remaining after iteration: {}", remaining_after),
        )?;

        if let Some(callback) = state_callback.as_deref_mut() {
            callback(
                session_name,
                iteration,
                LoopStatus::Running,
                remaining_after,
            );
        }

        iteration += 1;
        if iteration <= max_iterations {
            clock.sleep(Duration::from_secs(2));
        }
    }

    let final_remaining = count_remaining_tasks(&full_task_path);
    let duration_secs = clock
        .now()
        .duration_since(loop_start)
        .unwrap_or_default()
        .as_secs();

    log_message(Some(&log_file), "")?;
    log_message(
        Some(&log_file),
        &format!("Hit max iterations ({})", max_iterations),
    )?;
    log_message(
        Some(&log_file),
        &format!("Remaining tasks: {}", final_remaining),
    )?;
    log_message(
        Some(&log_file),
        &format!("Duration: {}", format_duration(duration_secs)),
    )?;
    log_message(
        Some(&log_file),
        &format!("FINISHED: {}", format_timestamp(clock.now())),
    )?;

    if let Some(callback) = state_callback.as_deref_mut() {
        callback(
            session_name,
            max_iterations,
            LoopStatus::MaxIterations,
            final_remaining,
        );
    }

    Ok(LoopOutcome {
        status: LoopStatus::MaxIterations,
        iterations: max_iterations,
        remaining_tasks: final_remaining,
        duration_secs,
    })
}

pub fn get_next_unchecked_task_block(task_file: &Path) -> Result<Option<String>, CoreError> {
    if task_file.as_os_str().is_empty() || !task_file.is_file() {
        return Ok(None);
    }
    let contents = fs::read_to_string(task_file).map_err(|source| CoreError::Io {
        path: task_file.to_path_buf(),
        source,
    })?;
    let blocks = task_blocks_from_contents(&contents);
    for block in blocks {
        if block.lines().any(|line| is_unchecked_line(line)) {
            return Ok(Some(block));
        }
    }
    Ok(None)
}

pub fn get_task_blocks(task_file: &Path) -> Result<Vec<String>, CoreError> {
    if task_file.as_os_str().is_empty() || !task_file.is_file() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(task_file).map_err(|source| CoreError::Io {
        path: task_file.to_path_buf(),
        source,
    })?;
    Ok(task_blocks_from_contents(&contents))
}

pub fn normalize_context_files(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    let mut normalized = Vec::new();
    for entry in raw.split(',') {
        let trimmed = entry.trim();
        if !trimmed.is_empty() {
            normalized.push(trimmed.to_string());
        }
    }
    normalized.join("\n")
}

pub fn render_prompt_template(
    template: &str,
    task_file: &str,
    completion_marker: &str,
    iteration: u32,
    max_iterations: u32,
    task_block: Option<&str>,
    context_files: Option<&str>,
) -> String {
    let task_block = task_block.unwrap_or("No task block available.");
    let context_files_section = if let Some(context_files) = context_files {
        if context_files.trim().is_empty() {
            String::new()
        } else {
            format!("Context Files (read these first):\n{}\n", context_files)
        }
    } else {
        String::new()
    };

    template
        .replace("{task_file}", task_file)
        .replace("{completion_marker}", completion_marker)
        .replace("{iteration}", &iteration.to_string())
        .replace("{max_iterations}", &max_iterations.to_string())
        .replace("{task_block}", task_block)
        .replace("{context_files}", context_files.unwrap_or(""))
        .replace("{context_files_section}", &context_files_section)
}

fn first_unchecked_line(task_file: &Path) -> Result<Option<String>, CoreError> {
    let contents = fs::read_to_string(task_file).map_err(|source| CoreError::Io {
        path: task_file.to_path_buf(),
        source,
    })?;
    for line in contents.lines() {
        if is_unchecked_line(line) {
            return Ok(Some(line.to_string()));
        }
    }
    Ok(None)
}

fn last_non_empty_line(text: &str) -> Option<String> {
    let mut last = None;
    for line in text.lines() {
        if !line.trim().is_empty() {
            last = Some(line.to_string());
        }
    }
    last
}

fn is_negated_promise(line: &str) -> bool {
    let lower = line.to_lowercase();
    let Some(promise_index) = lower.find("<promise>") else {
        return false;
    };
    let prefix = &lower[..promise_index];
    let phrases = [
        "cannot",
        "can't",
        "won't",
        "will not",
        "do not",
        "don't",
        "should not",
        "shouldn't",
        "must not",
        "mustn't",
    ];
    phrases.iter().any(|phrase| prefix.contains(phrase))
}

fn resolve_prompt_template(
    project_dir: &Path,
    prompt_template: Option<&str>,
) -> Result<String, CoreError> {
    if let Some(template) = prompt_template {
        if !template.trim().is_empty() {
            return Ok(template.to_string());
        }
    }

    if let Ok(path) = std::env::var("GRALPH_PROMPT_TEMPLATE_FILE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return fs::read_to_string(&path).map_err(|source| CoreError::Io { path, source });
        }
    }

    let template_path = project_dir.join(".gralph").join("prompt-template.txt");
    if template_path.is_file() {
        return fs::read_to_string(&template_path).map_err(|source| CoreError::Io {
            path: template_path,
            source,
        });
    }

    Ok(DEFAULT_PROMPT_TEMPLATE.to_string())
}

fn log_message(log_file: Option<&Path>, message: &str) -> Result<(), CoreError> {
    println!("{}", message);
    if let Some(path) = log_file {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| CoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        writeln!(file, "{}", message).map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

pub(crate) fn raw_log_path(log_file: &Path) -> PathBuf {
    let log_str = log_file.to_string_lossy();
    if log_str.ends_with(".log") {
        PathBuf::from(log_str.trim_end_matches(".log")).with_extension("raw.log")
    } else {
        PathBuf::from(format!("{}.raw.log", log_str))
    }
}

pub fn last_log_line(log_file: &Path) -> Option<String> {
    if log_file.as_os_str().is_empty() || !log_file.is_file() {
        return None;
    }
    let contents = fs::read_to_string(log_file).ok()?;
    let mut last = None;
    for line in contents.lines() {
        if !line.trim().is_empty() {
            last = Some(line.to_string());
        }
    }
    last
}

pub fn last_error_line(log_file: &Path) -> Option<String> {
    if log_file.as_os_str().is_empty() || !log_file.is_file() {
        return None;
    }
    let contents = fs::read_to_string(log_file).ok()?;
    let mut last = None;
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("Error:") || trimmed.starts_with("Iteration failed:") {
            last = Some(line.to_string());
        }
    }
    last
}

fn copy_if_exists(from: &Path, to: &Path) -> Result<(), CoreError> {
    if !from.is_file() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|source| CoreError::Io {
        path: to.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn format_timestamp(timestamp: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = timestamp.into();
    datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string()
}

fn format_duration(duration_secs: u64) -> String {
    let hours = duration_secs / 3600;
    let minutes = (duration_secs % 3600) / 60;
    let seconds = duration_secs % 60;
    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 || hours > 0 {
        parts.push(format!("{}m", minutes));
    }
    parts.push(format!("{}s", seconds));
    format!("{} ({}s)", parts.join(" "), duration_secs)
}

#[cfg(test)]
fn cleanup_old_logs(log_dir: &Path, config: Option<&Config>) -> Result<(), CoreError> {
    cleanup_old_logs_with_clock(log_dir, config, &SystemClock)
}

fn cleanup_old_logs_with_clock(
    log_dir: &Path,
    config: Option<&Config>,
    clock: &dyn Clock,
) -> Result<(), CoreError> {
    if !log_dir.is_dir() {
        return Ok(());
    }
    let retain_days = config
        .and_then(|cfg| cfg.get("logging.retain_days"))
        .and_then(|value| value.parse::<u64>().ok());
    if retain_days == Some(0) {
        return Ok(());
    }
    let retain_days = retain_days.unwrap_or(7);

    let cutoff = clock
        .now()
        .checked_sub(Duration::from_secs(retain_days.saturating_mul(86400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for entry in fs::read_dir(log_dir).map_err(|source| CoreError::Io {
        path: log_dir.to_path_buf(),
        source,
    })? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("log") {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let modified = match metadata.modified() {
            Ok(time) => time,
            Err(_) => continue,
        };
        if modified < cutoff {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

fn create_temp_file_with_clock(prefix: &str, clock: &dyn Clock) -> Result<PathBuf, CoreError> {
    let base_dir = std::env::temp_dir();
    for attempt in 0..100u32 {
        let now = clock
            .now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("{}-{}-{}-{}.tmp", prefix, std::process::id(), now, attempt);
        let path = base_dir.join(filename);
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(_) => return Ok(path),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(CoreError::Io { path, source: err }),
        }
    }
    Err(CoreError::InvalidInput(
        "failed to create temp file".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::string::string_regex;
    use std::cell::RefCell;
    use std::env;
    use std::fs;
    use std::fs::OpenOptions;
    use std::sync::Mutex;
    use std::time::{Duration, SystemTime};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
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

    fn set_modified(path: &Path, time: SystemTime) {
        let file = OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(time).unwrap();
    }

    fn task_id_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Z0-9]{1,8}").unwrap()
    }

    fn safe_line_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9][A-Za-z0-9 .,]{0,12}").unwrap()
    }

    fn context_entry_strategy() -> impl Strategy<Value = String> {
        let leading_ws = string_regex(r"[ \t]{0,3}").unwrap();
        let trailing_ws = string_regex(r"[ \t]{0,3}").unwrap();
        let value = prop_oneof![
            Just(String::new()),
            string_regex(r"[A-Za-z0-9._/-]{1,12}").unwrap(),
        ];
        (leading_ws, value, trailing_ws)
            .prop_map(|(prefix, entry, suffix)| format!("{}{}{}", prefix, entry, suffix))
    }

    fn block_lines_strategy(unchecked: bool) -> impl Strategy<Value = Vec<String>> {
        (
            task_id_strategy(),
            prop::collection::vec(safe_line_strategy(), 0..3),
        )
            .prop_map(move |(id, body)| {
                let mut lines = Vec::new();
                lines.push(format!("### Task {}", id));
                lines.push(if unchecked {
                    "- [ ] Task".to_string()
                } else {
                    "- [x] Task".to_string()
                });
                lines.extend(body);
                lines
            })
    }

    fn completion_marker_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9_-]{1,12}").unwrap()
    }

    fn negation_phrase_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("cannot"),
            Just("can't"),
            Just("won't"),
            Just("will not"),
            Just("do not"),
            Just("don't"),
            Just("should not"),
            Just("shouldn't"),
            Just("must not"),
            Just("mustn't"),
        ]
    }

    struct TestBackend {
        prompt: RefCell<Option<String>>,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                prompt: RefCell::new(None),
            }
        }
    }

    impl Backend for TestBackend {
        fn check_installed(&self) -> bool {
            true
        }

        fn run_iteration(
            &self,
            prompt: &str,
            _model: Option<&str>,
            _variant: Option<&str>,
            output_file: &Path,
            _working_dir: &Path,
        ) -> Result<(), BackendError> {
            *self.prompt.borrow_mut() = Some(prompt.to_string());
            fs::write(output_file, "ok").map_err(|source| BackendError::Io {
                path: output_file.to_path_buf(),
                source,
            })
        }

        fn parse_text(&self, response_file: &Path) -> Result<String, BackendError> {
            fs::read_to_string(response_file).map_err(|source| BackendError::Io {
                path: response_file.to_path_buf(),
                source,
            })
        }

        fn get_models(&self) -> Vec<String> {
            Vec::new()
        }
    }

    struct LoopBackend {
        response: String,
        fail_run: bool,
    }

    impl LoopBackend {
        fn success(response: &str) -> Self {
            Self {
                response: response.to_string(),
                fail_run: false,
            }
        }

        fn fail() -> Self {
            Self {
                response: String::new(),
                fail_run: true,
            }
        }
    }

    impl Backend for LoopBackend {
        fn check_installed(&self) -> bool {
            true
        }

        fn run_iteration(
            &self,
            _prompt: &str,
            _model: Option<&str>,
            _variant: Option<&str>,
            output_file: &Path,
            _working_dir: &Path,
        ) -> Result<(), BackendError> {
            if self.fail_run {
                return Err(BackendError::Command("backend error".to_string()));
            }
            fs::write(output_file, &self.response).map_err(|source| BackendError::Io {
                path: output_file.to_path_buf(),
                source,
            })
        }

        fn parse_text(&self, response_file: &Path) -> Result<String, BackendError> {
            fs::read_to_string(response_file).map_err(|source| BackendError::Io {
                path: response_file.to_path_buf(),
                source,
            })
        }

        fn get_models(&self) -> Vec<String> {
            Vec::new()
        }
    }

    struct StubBackend {
        output: String,
        parsed: Option<String>,
    }

    impl StubBackend {
        fn new(output: &str, parsed: Option<&str>) -> Self {
            Self {
                output: output.to_string(),
                parsed: parsed.map(ToString::to_string),
            }
        }
    }

    impl Backend for StubBackend {
        fn check_installed(&self) -> bool {
            true
        }

        fn run_iteration(
            &self,
            _prompt: &str,
            _model: Option<&str>,
            _variant: Option<&str>,
            output_file: &Path,
            _working_dir: &Path,
        ) -> Result<(), BackendError> {
            fs::write(output_file, &self.output).map_err(|source| BackendError::Io {
                path: output_file.to_path_buf(),
                source,
            })
        }

        fn parse_text(&self, response_file: &Path) -> Result<String, BackendError> {
            if let Some(parsed) = self.parsed.clone() {
                return Ok(parsed);
            }
            fs::read_to_string(response_file).map_err(|source| BackendError::Io {
                path: response_file.to_path_buf(),
                source,
            })
        }

        fn get_models(&self) -> Vec<String> {
            Vec::new()
        }
    }

    struct ParseFailBackend {
        output: String,
        error_message: String,
    }

    impl ParseFailBackend {
        fn new(output: &str, error_message: &str) -> Self {
            Self {
                output: output.to_string(),
                error_message: error_message.to_string(),
            }
        }
    }

    impl Backend for ParseFailBackend {
        fn check_installed(&self) -> bool {
            true
        }

        fn run_iteration(
            &self,
            _prompt: &str,
            _model: Option<&str>,
            _variant: Option<&str>,
            output_file: &Path,
            _working_dir: &Path,
        ) -> Result<(), BackendError> {
            fs::write(output_file, &self.output).map_err(|source| BackendError::Io {
                path: output_file.to_path_buf(),
                source,
            })
        }

        fn parse_text(&self, _response_file: &Path) -> Result<String, BackendError> {
            Err(BackendError::Command(self.error_message.clone()))
        }

        fn get_models(&self) -> Vec<String> {
            Vec::new()
        }
    }

    struct UninstalledBackend;

    impl Backend for UninstalledBackend {
        fn check_installed(&self) -> bool {
            false
        }

        fn run_iteration(
            &self,
            _prompt: &str,
            _model: Option<&str>,
            _variant: Option<&str>,
            _output_file: &Path,
            _working_dir: &Path,
        ) -> Result<(), BackendError> {
            panic!("backend should not run when uninstalled");
        }

        fn parse_text(&self, _response_file: &Path) -> Result<String, BackendError> {
            panic!("backend should not parse when uninstalled");
        }

        fn get_models(&self) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn count_remaining_tasks_ignores_outside_blocks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        let contents = "### Task RS-1\n- [ ] First\n---\n- [ ] Outside\n";
        fs::write(&path, contents).unwrap();

        let count = count_remaining_tasks(&path);
        assert_eq!(count, 1);
    }

    #[test]
    fn check_completion_requires_promise_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "All done\n<promise>COMPLETE</promise>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(complete);

        let not_complete = check_completion(&path, "All done", "COMPLETE").unwrap();
        assert!(!not_complete);
    }

    #[test]
    fn check_completion_accepts_trailing_whitespace_with_zero_tasks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "").unwrap();

        let result = "<promise>COMPLETE</promise>   \n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(complete);
    }

    #[test]
    fn check_completion_rejects_zero_tasks_without_promise_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "").unwrap();

        let result = "All done\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_uses_last_non_empty_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "<promise>COMPLETE</promise>\nStill working\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_rejects_negated_promise_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "Cannot <promise>COMPLETE</promise>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_rejects_negated_promise_phrase() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "We will not <promise>COMPLETE</promise>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_rejects_multiple_negated_phrases() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let phrases = ["can't", "won't", "do not", "don't", "shouldn't", "must not"];
        for phrase in phrases {
            let result = format!("We {} <promise>COMPLETE</promise>\n", phrase);
            let complete = check_completion(&path, &result, "COMPLETE").unwrap();
            assert!(!complete, "phrase should be rejected: {}", phrase);
        }
    }

    #[test]
    fn check_completion_matches_last_non_empty_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "Cannot <promise>COMPLETE</promise>\n\n<promise>COMPLETE</promise>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(complete);
    }

    #[test]
    fn check_completion_rejects_mismatched_marker() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "<promise>DONE</promise>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_rejects_malformed_promise_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let result = "<promise>COMPLETE</promis>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    proptest! {
        #[test]
        fn completion_marker_requires_exact_match(
            marker in completion_marker_strategy(),
            other in completion_marker_strategy(),
        ) {
            prop_assume!(marker != other);
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("PRD.md");
            fs::write(&path, "- [x] Done\n").unwrap();

            let exact = format!("<promise>{}</promise>", marker);
            prop_assert!(check_completion(&path, &exact, &marker).unwrap());

            let mismatch = format!("<promise>{}</promise>", other);
            prop_assert!(!check_completion(&path, &mismatch, &marker).unwrap());
        }
    }

    proptest! {
        #[test]
        fn negated_promise_prefix_prevents_completion(
            phrase in negation_phrase_strategy(),
            uppercase in any::<bool>(),
        ) {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("PRD.md");
            fs::write(&path, "- [x] Done\n").unwrap();

            let phrase = if uppercase {
                phrase.to_uppercase()
            } else {
                phrase.to_string()
            };
            let line = format!("{} <promise>COMPLETE</promise>", phrase);
            prop_assert!(!check_completion(&path, &line, "COMPLETE").unwrap());
        }
    }

    #[test]
    fn check_completion_rejects_remaining_tasks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let result = "<promise>COMPLETE</promise>\n";
        let complete = check_completion(&path, result, "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_returns_false_on_empty_result() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let complete = check_completion(&path, "  \n", "COMPLETE").unwrap();
        assert!(!complete);
    }

    #[test]
    fn check_completion_errors_when_task_file_missing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.md");

        let result = check_completion(&path, "<promise>COMPLETE</promise>", "COMPLETE");
        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("task file does not exist")
        ));
    }

    #[test]
    fn get_task_blocks_extracts_blocks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        let contents = "### Task RS-1\n- [ ] First\n---\n### Task RS-2\n- [ ] Second\n";
        fs::write(&path, contents).unwrap();

        let blocks = get_task_blocks(&path).unwrap();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("RS-1"));
        assert!(blocks[1].contains("RS-2"));
    }

    #[test]
    fn normalize_context_files_trims_and_splits() {
        let raw = "README.md,  ARCHITECTURE.md ,";
        let normalized = normalize_context_files(raw);
        assert_eq!(normalized, "README.md\nARCHITECTURE.md");
    }

    #[test]
    fn render_prompt_template_includes_context_files_section() {
        let template = "Header\n{context_files_section}Footer";
        let rendered = render_prompt_template(
            template,
            "PRD.md",
            "COMPLETE",
            1,
            3,
            Some("Block"),
            Some("ARCHITECTURE.md\nPROCESS.md"),
        );

        assert!(
            rendered.contains("Context Files (read these first):\nARCHITECTURE.md\nPROCESS.md\n")
        );
        assert!(rendered.contains("Header"));
        assert!(rendered.contains("Footer"));
    }

    #[test]
    fn resolve_prompt_template_prefers_explicit_template() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();
        let gralph_dir = project_dir.join(".gralph");
        fs::create_dir_all(&gralph_dir).unwrap();
        let project_path = gralph_dir.join("prompt-template.txt");
        fs::write(&project_path, "project").unwrap();

        let env_path = project_dir.join("env-template.txt");
        fs::write(&env_path, "env").unwrap();
        set_env("GRALPH_PROMPT_TEMPLATE_FILE", &env_path);

        let resolved = resolve_prompt_template(project_dir, Some("explicit template")).unwrap();
        assert_eq!(resolved, "explicit template");

        remove_env("GRALPH_PROMPT_TEMPLATE_FILE");
    }

    #[test]
    fn resolve_prompt_template_ignores_empty_explicit_template() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();
        let gralph_dir = project_dir.join(".gralph");
        fs::create_dir_all(&gralph_dir).unwrap();
        let project_path = gralph_dir.join("prompt-template.txt");
        fs::write(&project_path, "project").unwrap();

        let env_path = project_dir.join("env-template.txt");
        fs::write(&env_path, "env").unwrap();
        set_env("GRALPH_PROMPT_TEMPLATE_FILE", &env_path);

        let resolved = resolve_prompt_template(project_dir, Some("  ")).unwrap();
        assert_eq!(resolved, "env");

        remove_env("GRALPH_PROMPT_TEMPLATE_FILE");
    }

    #[test]
    fn resolve_prompt_template_respects_env_then_project_then_default() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();
        let gralph_dir = project_dir.join(".gralph");
        fs::create_dir_all(&gralph_dir).unwrap();
        let project_path = gralph_dir.join("prompt-template.txt");
        fs::write(&project_path, "project").unwrap();

        let env_path = project_dir.join("env-template.txt");
        fs::write(&env_path, "env").unwrap();
        set_env("GRALPH_PROMPT_TEMPLATE_FILE", &env_path);

        let resolved = resolve_prompt_template(project_dir, None).unwrap();
        assert_eq!(resolved, "env");

        remove_env("GRALPH_PROMPT_TEMPLATE_FILE");
        let resolved = resolve_prompt_template(project_dir, None).unwrap();
        assert_eq!(resolved, "project");

        fs::remove_file(&project_path).unwrap();
        let resolved = resolve_prompt_template(project_dir, None).unwrap();
        assert_eq!(resolved, DEFAULT_PROMPT_TEMPLATE);
    }

    #[test]
    fn resolve_prompt_template_falls_back_when_env_and_project_missing() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();
        let env_path = project_dir.join("missing-template.txt");
        set_env("GRALPH_PROMPT_TEMPLATE_FILE", &env_path);

        let resolved = resolve_prompt_template(project_dir, None).unwrap();
        assert_eq!(resolved, DEFAULT_PROMPT_TEMPLATE);

        remove_env("GRALPH_PROMPT_TEMPLATE_FILE");
    }

    #[test]
    fn resolve_prompt_template_ignores_env_path_when_not_file() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();
        let gralph_dir = project_dir.join(".gralph");
        fs::create_dir_all(&gralph_dir).unwrap();
        let project_path = gralph_dir.join("prompt-template.txt");
        fs::write(&project_path, "project").unwrap();

        set_env("GRALPH_PROMPT_TEMPLATE_FILE", project_dir);

        let resolved = resolve_prompt_template(project_dir, None).unwrap();
        assert_eq!(resolved, "project");

        remove_env("GRALPH_PROMPT_TEMPLATE_FILE");
    }

    #[test]
    fn task_blocks_end_on_separator_and_section_heading() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        let contents = "### Task RS-1\n- [ ] First\n---\n- [ ] Outside\n## Success Criteria\n- Pass\n### Task RS-2\n- [ ] Second\n";
        fs::write(&path, contents).unwrap();

        let blocks = get_task_blocks(&path).unwrap();
        assert_eq!(blocks.len(), 2);
        assert!(!blocks[0].contains("Outside"));
        assert!(!blocks[0].contains("Success Criteria"));
        assert!(blocks[1].contains("Second"));
    }

    #[test]
    fn get_next_unchecked_task_block_ignores_stray_outside_blocks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        let contents = "### Task RS-1\n- [x] Done\n---\n- [ ] Outside\n";
        fs::write(&path, contents).unwrap();

        let block = get_next_unchecked_task_block(&path).unwrap();
        assert!(block.is_none());
    }

    #[test]
    fn log_message_creates_parent_and_appends() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("logs").join("loop.log");

        log_message(Some(&log_path), "first").unwrap();
        log_message(Some(&log_path), "second").unwrap();

        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("first"));
        assert!(contents.contains("second"));
    }

    #[test]
    fn log_message_errors_when_path_is_directory() {
        let temp = tempfile::tempdir().unwrap();
        let dir_path = temp.path().join("logs");
        fs::create_dir_all(&dir_path).unwrap();

        let result = log_message(Some(&dir_path), "message");
        assert!(matches!(result, Err(CoreError::Io { .. })));
    }

    #[test]
    fn last_log_line_returns_last_non_empty_line() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("session.log");
        fs::write(&log_path, "first\n\nsecond\n").unwrap();

        let last = last_log_line(&log_path);
        assert_eq!(last.as_deref(), Some("second"));
    }

    #[test]
    fn last_error_line_returns_last_matching_error() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("session.log");
        fs::write(
            &log_path,
            "Starting\nError: backend produced no output.\nIteration failed: bad output\n",
        )
        .unwrap();

        let last = last_error_line(&log_path);
        assert_eq!(last.as_deref(), Some("Iteration failed: bad output"));
    }

    #[test]
    fn raw_log_path_rewrites_log_extension() {
        let path = Path::new("/tmp/session.log");
        let raw = raw_log_path(path);
        assert!(raw.to_string_lossy().ends_with("session.raw.log"));
    }

    #[test]
    fn raw_log_path_appends_extension_when_missing() {
        let path = Path::new("/tmp/session");
        let raw = raw_log_path(path);
        assert!(raw.to_string_lossy().ends_with("session.raw.log"));
    }

    #[test]
    fn copy_if_exists_skips_missing_source() {
        let temp = tempfile::tempdir().unwrap();
        let from = temp.path().join("missing.txt");
        let to = temp.path().join("target.txt");

        copy_if_exists(&from, &to).unwrap();
        assert!(!to.exists());
    }

    #[test]
    fn copy_if_exists_copies_source_file() {
        let temp = tempfile::tempdir().unwrap();
        let from = temp.path().join("source.txt");
        let to = temp.path().join("target.txt");
        fs::write(&from, "data").unwrap();

        copy_if_exists(&from, &to).unwrap();

        let contents = fs::read_to_string(&to).unwrap();
        assert_eq!(contents, "data");
    }

    #[test]
    fn cleanup_old_logs_removes_only_old_log_files() {
        let temp = tempfile::tempdir().unwrap();
        let log_dir = temp.path().join(".gralph");
        fs::create_dir_all(&log_dir).unwrap();

        let old_log = log_dir.join("old.log");
        let recent_log = log_dir.join("recent.log");
        let keep_txt = log_dir.join("keep.txt");

        fs::write(&old_log, "old").unwrap();
        fs::write(&recent_log, "recent").unwrap();
        fs::write(&keep_txt, "keep").unwrap();

        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(9 * 86400))
            .unwrap();
        set_modified(&old_log, old_time);
        set_modified(&keep_txt, old_time);

        cleanup_old_logs(&log_dir, None).unwrap();

        assert!(!old_log.exists());
        assert!(recent_log.exists());
        assert!(keep_txt.exists());
    }

    #[test]
    fn cleanup_old_logs_respects_retention_days_from_config() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let log_dir = temp.path().join(".gralph");
        fs::create_dir_all(&log_dir).unwrap();

        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, "logging:\n  retain_days: 1\n").unwrap();
        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        let global_path = temp.path().join("missing-global.yaml");
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();

        let old_log = log_dir.join("old.log");
        let recent_log = log_dir.join("recent.log");
        fs::write(&old_log, "old").unwrap();
        fs::write(&recent_log, "recent").unwrap();

        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(2 * 86400))
            .unwrap();
        let recent_time = SystemTime::now()
            .checked_sub(Duration::from_secs(12 * 3600))
            .unwrap();
        set_modified(&old_log, old_time);
        set_modified(&recent_log, recent_time);

        cleanup_old_logs(&log_dir, Some(&config)).unwrap();

        assert!(!old_log.exists());
        assert!(recent_log.exists());

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn cleanup_old_logs_respects_retention_boundary() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let log_dir = temp.path().join(".gralph");
        fs::create_dir_all(&log_dir).unwrap();

        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, "logging:\n  retain_days: 1\n").unwrap();
        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        let global_path = temp.path().join("missing-global.yaml");
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();

        let old_log = log_dir.join("old.log");
        let edge_log = log_dir.join("edge.log");
        fs::write(&old_log, "old").unwrap();
        fs::write(&edge_log, "edge").unwrap();

        let base_time = SystemTime::now();
        let old_time = base_time
            .checked_sub(Duration::from_secs(86400 + 10))
            .unwrap();
        let edge_time = base_time
            .checked_sub(Duration::from_secs(86400 - 10))
            .unwrap();
        set_modified(&old_log, old_time);
        set_modified(&edge_log, edge_time);

        cleanup_old_logs(&log_dir, Some(&config)).unwrap();

        assert!(!old_log.exists());
        assert!(edge_log.exists());

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn cleanup_old_logs_skips_missing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let log_dir = temp.path().join("missing");

        cleanup_old_logs(&log_dir, None).unwrap();
        assert!(!log_dir.exists());
    }

    #[test]
    fn cleanup_old_logs_skips_when_retention_disabled() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let log_dir = temp.path().join(".gralph");
        fs::create_dir_all(&log_dir).unwrap();

        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, "logging:\n  retain_days: 0\n").unwrap();
        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        let global_path = temp.path().join("missing-global.yaml");
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();

        let old_log = log_dir.join("old.log");
        fs::write(&old_log, "old").unwrap();
        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(30 * 86400))
            .unwrap();
        set_modified(&old_log, old_time);

        cleanup_old_logs(&log_dir, Some(&config)).unwrap();

        assert!(old_log.exists());

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn run_iteration_falls_back_to_first_unchecked_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Solo task\n").unwrap();

        let backend = TestBackend::new();
        let _ = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            2,
            "COMPLETE",
            None,
            None,
            None,
            Some("Block:\n{task_block}\n"),
            None,
        )
        .unwrap();

        let prompt = backend.prompt.borrow().clone().unwrap();
        assert!(prompt.contains("- [ ] Solo task"));
        assert!(!prompt.contains("No task block available."));
    }

    #[test]
    fn run_iteration_includes_context_files_from_config() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "### Task COV-CORE\n- [ ] Task\n").unwrap();

        let config_path = temp.path().join("config.yaml");
        fs::write(
            &config_path,
            "defaults:\n  context_files:\n    - ARCHITECTURE.md\n    - PROCESS.md\n",
        )
        .unwrap();
        set_env("GRALPH_DEFAULT_CONFIG", &config_path);
        let global_path = temp.path().join("missing-global.yaml");
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();

        let backend = TestBackend::new();
        let _ = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            2,
            "COMPLETE",
            None,
            None,
            None,
            Some("{context_files_section}Task:\n{task_block}\n"),
            Some(&config),
        )
        .unwrap();

        let prompt = backend.prompt.borrow().clone().unwrap();
        assert!(prompt.contains("Context Files (read these first):\nARCHITECTURE.md\nPROCESS.md\n"));

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn run_iteration_rejects_empty_project_dir() {
        let backend = TestBackend::new();
        let result = run_iteration(
            &backend,
            Path::new(""),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("project_dir is required")
        ));
    }

    #[test]
    fn run_iteration_rejects_project_dir_when_not_directory() {
        let temp = tempfile::tempdir().unwrap();
        let project_file = temp.path().join("not-dir");
        fs::write(&project_file, "data").unwrap();

        let backend = TestBackend::new();
        let result = run_iteration(
            &backend,
            &project_file,
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("project directory does not exist")
        ));
    }

    #[test]
    fn run_iteration_rejects_missing_project_dir() {
        let temp = tempfile::tempdir().unwrap();
        let missing_dir = temp.path().join("missing-dir");

        let backend = TestBackend::new();
        let result = run_iteration(
            &backend,
            &missing_dir,
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("project directory does not exist")
        ));
    }

    #[test]
    fn run_iteration_rejects_iteration_zero() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = TestBackend::new();
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            0,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("iteration number is required")
        ));
    }

    #[test]
    fn run_iteration_rejects_max_iterations_zero() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = TestBackend::new();
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            0,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("max_iterations is required")
        ));
    }

    #[test]
    fn run_iteration_rejects_missing_task_file() {
        let temp = tempfile::tempdir().unwrap();
        let backend = TestBackend::new();
        let result = run_iteration(
            &backend,
            temp.path(),
            "missing.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("task file does not exist")
        ));
    }

    #[test]
    fn run_iteration_rejects_uninstalled_backend() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = UninstalledBackend;
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message)) if message.contains("backend is not installed")
        ));
    }

    #[test]
    fn run_iteration_rejects_empty_output_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = StubBackend::new("", None);
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message)) if message.contains("backend produced no output")
        ));
    }

    #[test]
    fn run_iteration_logs_raw_output_when_backend_output_empty() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let log_path = temp.path().join("loop.log");
        let raw_path = raw_log_path(&log_path);

        let backend = StubBackend::new("", None);
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            Some(&log_path),
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message)) if message.contains("backend produced no output")
        ));

        let log_contents = fs::read_to_string(&log_path).unwrap();
        assert!(log_contents.contains("Error: backend produced no JSON output."));
        assert!(log_contents.contains(&format!("Raw output saved to: {}", raw_path.display())));
        assert!(raw_path.exists());
    }

    #[test]
    fn run_iteration_logs_raw_output_when_backend_fails_with_empty_output() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let log_path = temp.path().join("loop.log");
        let raw_path = raw_log_path(&log_path);

        let backend = LoopBackend::fail();
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            Some(&log_path),
            None,
            None,
        );

        assert!(matches!(result, Err(CoreError::Backend(_))));

        let log_contents = fs::read_to_string(&log_path).unwrap();
        assert!(log_contents.contains(&format!("Raw output saved to: {}", raw_path.display())));
        assert!(raw_path.exists());
    }

    #[test]
    fn run_iteration_rejects_empty_parsed_result() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = StubBackend::new("{\"ok\":true}", Some("   "));
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("backend returned no parsed result")
        ));
    }

    #[test]
    fn run_iteration_logs_raw_output_when_parsed_result_empty() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let log_path = temp.path().join("loop.log");
        let raw_path = raw_log_path(&log_path);

        let backend = StubBackend::new("raw-output", Some("   "));
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            Some(&log_path),
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("backend returned no parsed result")
        ));

        let log_contents = fs::read_to_string(&log_path).unwrap();
        assert!(log_contents.contains("Error: backend returned no parsed result."));
        assert!(log_contents.contains(&format!("Raw output saved to: {}", raw_path.display())));

        let raw_contents = fs::read_to_string(&raw_path).unwrap();
        assert_eq!(raw_contents, "raw-output");
    }

    #[test]
    fn run_iteration_returns_parse_error_and_copies_raw_output() {
        let _guard = env_guard();
        remove_env("GRALPH_PROMPT_TEMPLATE_FILE");
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let log_path = temp.path().join("loop.log");
        let raw_path = raw_log_path(&log_path);

        let backend = ParseFailBackend::new("raw-output", "parse failed");
        let result = run_iteration(
            &backend,
            temp.path(),
            "PRD.md",
            1,
            1,
            "COMPLETE",
            None,
            None,
            Some(&log_path),
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::Backend(BackendError::Command(message)))
                if message.contains("parse failed")
        ));

        let raw_contents = fs::read_to_string(&raw_path).unwrap();
        assert_eq!(raw_contents, "raw-output");
    }

    #[test]
    fn run_loop_rejects_empty_project_dir() {
        let backend = LoopBackend::success("ok");
        let result = run_loop(
            &backend,
            Path::new(""),
            Some("PRD.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message)) if message.contains("project_dir is required")
        ));
    }

    #[test]
    fn run_loop_rejects_zero_max_iterations() {
        let temp = tempfile::tempdir().unwrap();
        let backend = LoopBackend::success("ok");
        let result = run_loop(
            &backend,
            temp.path(),
            Some("PRD.md"),
            Some(0),
            Some("COMPLETE"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("max_iterations must be a positive integer")
        ));
    }

    #[test]
    fn run_loop_rejects_project_dir_when_not_directory() {
        let temp = tempfile::tempdir().unwrap();
        let project_file = temp.path().join("not-dir");
        fs::write(&project_file, "data").unwrap();

        let backend = LoopBackend::success("ok");
        let result = run_loop(
            &backend,
            &project_file,
            Some("PRD.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("project directory does not exist")
        ));
    }

    #[test]
    fn run_loop_rejects_missing_task_file() {
        let temp = tempfile::tempdir().unwrap();
        let backend = LoopBackend::success("ok");
        let result = run_loop(
            &backend,
            temp.path(),
            Some("missing.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(CoreError::InvalidInput(message))
                if message.contains("task file does not exist")
        ));
    }

    #[test]
    fn loop_completes_with_promise_and_updates_state() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [x] Done\n").unwrap();

        let backend = LoopBackend::success("All done\n<promise>COMPLETE</promise>\n");
        let mut updates: Vec<(u32, LoopStatus, usize)> = Vec::new();
        let mut callback = |_: Option<&str>, iteration, status, remaining| {
            updates.push((iteration, status, remaining));
        };

        let outcome = run_loop(
            &backend,
            temp.path(),
            Some("PRD.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            Some("session"),
            None,
            None,
            Some(&mut callback),
        )
        .unwrap();

        assert_eq!(outcome.status, LoopStatus::Complete);
        assert_eq!(outcome.iterations, 1);
        assert_eq!(outcome.remaining_tasks, 0);
        assert_eq!(
            updates,
            vec![(1, LoopStatus::Running, 0), (1, LoopStatus::Complete, 0)]
        );
    }

    #[test]
    fn loop_reports_backend_error_and_failed_state() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = LoopBackend::fail();
        let mut updates: Vec<(u32, LoopStatus, usize)> = Vec::new();
        let mut callback = |_: Option<&str>, iteration, status, remaining| {
            updates.push((iteration, status, remaining));
        };

        let result = run_loop(
            &backend,
            temp.path(),
            Some("PRD.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            Some("session"),
            None,
            None,
            Some(&mut callback),
        );

        assert!(matches!(result, Err(CoreError::Backend(_))));
        assert_eq!(
            updates,
            vec![(1, LoopStatus::Running, 1), (1, LoopStatus::Failed, 1)]
        );
    }

    #[test]
    fn loop_hits_max_iterations_and_updates_state() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = LoopBackend::success("Still working\n");
        let mut updates: Vec<(u32, LoopStatus, usize)> = Vec::new();
        let mut callback = |_: Option<&str>, iteration, status, remaining| {
            updates.push((iteration, status, remaining));
        };

        let outcome = run_loop(
            &backend,
            temp.path(),
            Some("PRD.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            Some("session"),
            None,
            None,
            Some(&mut callback),
        )
        .unwrap();

        assert_eq!(outcome.status, LoopStatus::MaxIterations);
        assert_eq!(outcome.iterations, 1);
        assert_eq!(outcome.remaining_tasks, 1);
        assert_eq!(
            updates,
            vec![
                (1, LoopStatus::Running, 1),
                (1, LoopStatus::Running, 1),
                (1, LoopStatus::MaxIterations, 1)
            ]
        );
    }

    #[test]
    fn loop_updates_remaining_counts_after_iteration() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("PRD.md");
        fs::write(&path, "- [ ] Task\n").unwrap();

        let backend = LoopBackend::success("Still working\n");
        let mut updates: Vec<(u32, LoopStatus, usize)> = Vec::new();
        let mut flipped = false;
        let task_path = path.clone();
        let mut callback = |_: Option<&str>, iteration, status, remaining| {
            updates.push((iteration, status, remaining));
            if !flipped && status == LoopStatus::Running && remaining == 1 {
                fs::write(&task_path, "- [x] Task\n").unwrap();
                flipped = true;
            }
        };

        let outcome = run_loop(
            &backend,
            temp.path(),
            Some("PRD.md"),
            Some(1),
            Some("COMPLETE"),
            None,
            None,
            Some("session"),
            None,
            None,
            Some(&mut callback),
        )
        .unwrap();

        assert_eq!(outcome.status, LoopStatus::MaxIterations);
        assert_eq!(outcome.remaining_tasks, 0);
        assert_eq!(
            updates,
            vec![
                (1, LoopStatus::Running, 1),
                (1, LoopStatus::Running, 0),
                (1, LoopStatus::MaxIterations, 0)
            ]
        );
    }

    proptest! {
        #[test]
        fn prop_get_next_unchecked_task_block_selects_first_unchecked(
            before_blocks in prop::collection::vec(block_lines_strategy(false), 0..3),
            selected_block in block_lines_strategy(true),
            after_blocks in prop::collection::vec(block_lines_strategy(false), 0..3),
        ) {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("PRD.md");

            let mut lines = Vec::new();
            for block in &before_blocks {
                lines.extend(block.iter().cloned());
            }
            lines.extend(selected_block.iter().cloned());
            for block in &after_blocks {
                lines.extend(block.iter().cloned());
            }

            fs::write(&path, lines.join("\n")).unwrap();

            let expected = selected_block.join("\n");
            let found = get_next_unchecked_task_block(&path).unwrap();
            prop_assert_eq!(found.as_deref(), Some(expected.as_str()));
        }

        #[test]
        fn prop_check_completion_accepts_exact_promise_line(
            prefix in prop::collection::vec(safe_line_strategy(), 0..3),
            marker in string_regex(r"[A-Z]{3,8}").unwrap(),
        ) {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("PRD.md");
            fs::write(&path, "- [x] Done\n").unwrap();

            let mut result = String::new();
            for line in prefix {
                result.push_str(&line);
                result.push('\n');
            }
            result.push_str(&format!("<promise>{}</promise>\n", marker));

            let complete = check_completion(&path, &result, &marker).unwrap();
            prop_assert!(complete);
        }

        #[test]
        fn prop_check_completion_rejects_non_promise_last_line(
            prefix in prop::collection::vec(safe_line_strategy(), 0..3),
            marker in string_regex(r"[A-Z]{3,8}").unwrap(),
            final_line in safe_line_strategy(),
        ) {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("PRD.md");
            fs::write(&path, "- [x] Done\n").unwrap();

            let mut result = String::new();
            for line in prefix {
                result.push_str(&line);
                result.push('\n');
            }
            result.push_str(&final_line);
            result.push('\n');

            let complete = check_completion(&path, &result, &marker).unwrap();
            prop_assert!(!complete);
        }

        #[test]
        fn prop_normalize_context_files_trims_and_drops_empty(
            entries in prop::collection::vec(context_entry_strategy(), 0..6),
        ) {
            let raw = entries.join(",");
            let expected = entries
                .iter()
                .map(|entry| entry.trim())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            let normalized = normalize_context_files(&raw);
            prop_assert_eq!(normalized.as_str(), expected.as_str());
            prop_assert!(!normalized.lines().any(|line| line.trim().is_empty()));
        }

        #[test]
        fn prop_render_prompt_template_handles_context_section_and_task_block(
            context_files in prop::option::of(context_entry_strategy()),
            task_block in prop::option::of(safe_line_strategy()),
        ) {
            let template = "Header\n{context_files_section}Task:\n{task_block}\nFooter";
            let rendered = render_prompt_template(
                template,
                "PRD.md",
                "COMPLETE",
                1,
                2,
                task_block.as_deref(),
                context_files.as_deref(),
            );

            match context_files.as_deref() {
                Some(context) if !context.trim().is_empty() => {
                    prop_assert!(rendered.contains("Context Files (read these first):"));
                    prop_assert!(rendered.contains(context));
                }
                _ => {
                    prop_assert!(!rendered.contains("Context Files (read these first):"));
                }
            }

            match task_block.as_deref() {
                Some(task) => {
                    prop_assert!(rendered.contains(task));
                    prop_assert!(!rendered.contains("No task block available."));
                }
                None => {
                    prop_assert!(rendered.contains("No task block available."));
                }
            }
        }

        #[test]
        fn prop_render_prompt_template_replaces_placeholders(
            task_file in string_regex(r"[A-Za-z0-9._/-]{1,16}").unwrap(),
            completion_marker in string_regex(r"[A-Za-z0-9_-]{1,12}").unwrap(),
            iteration in 1u32..50,
            max_iterations in 1u32..50,
            task_block in safe_line_strategy(),
            context_files in prop::option::of(string_regex(r"[A-Za-z0-9._/-]{1,20}").unwrap()),
        ) {
            prop_assume!(max_iterations >= iteration);
            let template = "File:{task_file}\nMarker:{completion_marker}\nIter:{iteration}/{max_iterations}\nBlock:{task_block}\nCtx:{context_files}\n{context_files_section}End";
            let rendered = render_prompt_template(
                template,
                &task_file,
                &completion_marker,
                iteration,
                max_iterations,
                Some(&task_block),
                context_files.as_deref(),
            );

            let expected_file = format!("File:{}", task_file);
            let expected_marker = format!("Marker:{}", completion_marker);
            let expected_iter = format!("Iter:{}/{}", iteration, max_iterations);
            let expected_block = format!("Block:{}", task_block);

            prop_assert!(rendered.contains(&expected_file));
            prop_assert!(rendered.contains(&expected_marker));
            prop_assert!(rendered.contains(&expected_iter));
            prop_assert!(rendered.contains(&expected_block));

            match context_files.as_deref() {
                Some(context) if !context.is_empty() => {
                    let expected_ctx = format!("Ctx:{}", context);
                    prop_assert!(rendered.contains(&expected_ctx));
                    prop_assert!(rendered.contains("Context Files (read these first):"));
                    prop_assert!(rendered.contains(context));
                }
                _ => {
                    prop_assert!(rendered.contains("Ctx:\n"));
                    prop_assert!(!rendered.contains("Context Files (read these first):"));
                }
            }

            let placeholders = [
                "{task_file}",
                "{completion_marker}",
                "{iteration}",
                "{max_iterations}",
                "{task_block}",
                "{context_files}",
                "{context_files_section}",
            ];

            for placeholder in placeholders {
                prop_assert!(!rendered.contains(placeholder));
            }
        }

    }
}
