use crate::backend::{Backend, BackendError};
use crate::config::Config;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_PROMPT_TEMPLATE: &str = "Read {task_file} carefully. Find any task marked '- [ ]' (unchecked).\n\nIf unchecked tasks exist:\n- Complete ONE task fully\n- Mark it '- [x]' in {task_file}\n- Commit changes\n- Exit normally (do NOT output completion promise)\n\nIf ZERO '- [ ]' remain (all complete):\n- Verify by searching the file\n- Output ONLY: <promise>{completion_marker}</promise>\n\nCRITICAL: Never mention the promise unless outputting it as the completion signal.\n\n{context_files_section}Task Block:\n{task_block}\n\nIteration: {iteration}/{max_iterations}";

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

pub fn run_iteration<B: Backend>(
    backend: &B,
    project_dir: &Path,
    task_file: &str,
    iteration: u32,
    max_iterations: u32,
    completion_marker: &str,
    model: Option<&str>,
    log_file: Option<&Path>,
    prompt_template: Option<&str>,
    config: Option<&Config>,
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

    let tmpfile = create_temp_file("gralph-iteration")?;

    let raw_output_file = log_file.map(|path| raw_log_path(path));

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

    let previous_dir = std::env::current_dir().map_err(|source| CoreError::Io {
        path: PathBuf::from("."),
        source,
    })?;
    std::env::set_current_dir(project_dir).map_err(|source| CoreError::Io {
        path: project_dir.to_path_buf(),
        source,
    })?;

    let backend_result = backend.run_iteration(&prompt, model, &tmpfile);

    std::env::set_current_dir(&previous_dir).map_err(|source| CoreError::Io {
        path: previous_dir.clone(),
        source,
    })?;

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
        if let Ok(blocks) = get_task_blocks_from_contents(&contents) {
            for block in blocks {
                count += block.lines().filter(|line| is_unchecked_line(line)).count();
            }
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
    let expected = format!("<promise>{}</promise>", completion_marker);
    if promise_line.trim() != expected {
        return Ok(false);
    }

    if is_negated_promise(&promise_line) {
        return Ok(false);
    }

    Ok(true)
}

pub fn run_loop<B: Backend>(
    backend: &B,
    project_dir: &Path,
    task_file: Option<&str>,
    max_iterations: Option<u32>,
    completion_marker: Option<&str>,
    model: Option<&str>,
    session_name: Option<&str>,
    prompt_template: Option<&str>,
    config: Option<&Config>,
    mut state_callback: Option<&mut dyn FnMut(Option<&str>, u32, LoopStatus, usize)>,
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

    cleanup_old_logs(&gralph_dir, config)?;

    let log_name = session_name.unwrap_or("gralph");
    let log_file = gralph_dir.join(format!("{}.log", log_name));

    let loop_start = SystemTime::now();
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
    log_message(
        Some(&log_file),
        &format!("Started at: {}", timestamp_seconds()),
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
            let duration_secs = loop_start
                .elapsed()
                .unwrap_or_else(|_| Duration::from_secs(0))
                .as_secs();

            log_message(Some(&log_file), "")?;
            log_message(
                Some(&log_file),
                &format!("Gralph complete after {} iterations.", iteration),
            )?;
            log_message(Some(&log_file), &format!("Duration: {}s", duration_secs))?;
            log_message(
                Some(&log_file),
                &format!("FINISHED: {}", timestamp_seconds()),
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
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    let final_remaining = count_remaining_tasks(&full_task_path);
    let duration_secs = loop_start
        .elapsed()
        .unwrap_or_else(|_| Duration::from_secs(0))
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
    log_message(Some(&log_file), &format!("Duration: {}s", duration_secs))?;
    log_message(
        Some(&log_file),
        &format!("FINISHED: {}", timestamp_seconds()),
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
    let blocks = get_task_blocks_from_contents(&contents)?;
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
    get_task_blocks_from_contents(&contents)
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

fn get_task_blocks_from_contents(contents: &str) -> Result<Vec<String>, CoreError> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut block = String::new();

    for line in contents.lines() {
        if is_task_header(line) {
            if in_block {
                blocks.push(block.clone());
                block.clear();
            }
            in_block = true;
            block.push_str(line);
            continue;
        }

        if in_block && is_task_block_end(line) {
            blocks.push(block.clone());
            block.clear();
            in_block = false;
            continue;
        }

        if in_block {
            block.push('\n');
            block.push_str(line);
        }
    }

    if in_block && !block.is_empty() {
        blocks.push(block);
    }

    Ok(blocks)
}

fn is_task_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("### Task ")
}

fn is_task_block_end(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed == "---" {
        return true;
    }
    let trimmed_start = line.trim_start();
    trimmed_start.starts_with("## ")
}

fn is_unchecked_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- [ ]")
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

fn raw_log_path(log_file: &Path) -> PathBuf {
    let log_str = log_file.to_string_lossy();
    if log_str.ends_with(".log") {
        PathBuf::from(log_str.trim_end_matches(".log")).with_extension("raw.log")
    } else {
        PathBuf::from(format!("{}.raw.log", log_str))
    }
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

fn timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn cleanup_old_logs(log_dir: &Path, config: Option<&Config>) -> Result<(), CoreError> {
    if !log_dir.is_dir() {
        return Ok(());
    }
    let retain_days = config
        .and_then(|cfg| cfg.get("logging.retain_days"))
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(7);

    let cutoff = SystemTime::now()
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

fn create_temp_file(prefix: &str) -> Result<PathBuf, CoreError> {
    let base_dir = std::env::temp_dir();
    for attempt in 0..10u32 {
        let now = timestamp_seconds();
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
    use std::fs;

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
}
