use crate::{CliError, git_output_in_dir, join_or_none, normalize_csv, parse_bool_value};
use gralph_rs::config::Config;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;
use std::thread;
use std::time::{Duration, Instant};

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

pub(crate) fn run_verifier_pipeline(
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

pub(crate) fn resolve_verifier_auto_run(config: &Config) -> bool {
    config
        .get("verifier.auto_run")
        .as_deref()
        .and_then(parse_bool_value)
        .unwrap_or(DEFAULT_VERIFIER_AUTO_RUN)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn load_project_config(contents: &str) -> Config {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(".gralph.yaml");
        fs::write(&path, contents).unwrap();
        Config::load(Some(temp.path())).unwrap()
    }

    fn base_review_settings() -> ReviewGateSettings {
        ReviewGateSettings {
            enabled: true,
            reviewer: "greptile".to_string(),
            min_rating: 8.0,
            max_issues: 0,
            poll_seconds: 20,
            timeout_seconds: 60,
            require_approval: false,
            require_checks: true,
            merge_method: MergeMethod::Merge,
        }
    }

    fn base_static_settings() -> StaticCheckSettings {
        StaticCheckSettings {
            enabled: true,
            check_todo: true,
            check_comments: true,
            check_duplicates: false,
            allow_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            todo_markers: vec!["TODO".to_string()],
            max_comment_lines: DEFAULT_STATIC_MAX_COMMENT_LINES,
            max_comment_chars: DEFAULT_STATIC_MAX_COMMENT_CHARS,
            duplicate_block_lines: DEFAULT_STATIC_DUPLICATE_BLOCK_LINES,
            duplicate_min_alnum_lines: DEFAULT_STATIC_DUPLICATE_MIN_ALNUM_LINES,
            max_file_bytes: DEFAULT_STATIC_MAX_FILE_BYTES,
        }
    }

    #[test]
    fn resolve_verifier_command_rejects_empty_default() {
        let config = load_project_config("verifier:\n  test_command: \"\"\n");
        let err = resolve_verifier_command(None, &config, "verifier.test_command", " ")
            .unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("empty"));
            }
            other => panic!("expected message error, got {other:?}"),
        }
    }

    #[test]
    fn resolve_verifier_command_prefers_arg_value() {
        let config = Config::load(None).unwrap();
        let command = resolve_verifier_command(
            Some("custom --flag".to_string()),
            &config,
            "verifier.test_command",
            DEFAULT_TEST_COMMAND,
        )
        .unwrap();
        assert_eq!(command, "custom --flag");
    }

    #[test]
    fn resolve_verifier_coverage_min_rejects_out_of_range() {
        let config = Config::load(None).unwrap();
        let err = resolve_verifier_coverage_min(Some(120.0), &config).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("between 0 and 100"));
            }
            other => panic!("expected message error, got {other:?}"),
        }
    }

    #[test]
    fn resolve_verifier_coverage_min_defaults_on_empty_config() {
        let config = load_project_config("verifier:\n  coverage_min: \"\"\n");
        let value = resolve_verifier_coverage_min(None, &config).unwrap();
        assert!((value - DEFAULT_COVERAGE_MIN).abs() < 1e-6);
    }

    #[test]
    fn parse_verifier_command_rejects_empty_input() {
        let err = parse_verifier_command("  ").unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("cannot be empty"));
            }
            other => panic!("expected message error, got {other:?}"),
        }
    }

    #[test]
    fn parse_verifier_command_rejects_invalid_shell_words() {
        let err = parse_verifier_command("cargo test \"unterminated").unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("Failed to parse command"));
            }
            other => panic!("expected message error, got {other:?}"),
        }
    }

    #[test]
    fn parse_percent_from_line_returns_last_percent() {
        let value = parse_percent_from_line("Coverage 55.1% (line 88.2%)").unwrap();
        assert!((value - 88.2).abs() < 1e-6);
    }

    #[test]
    fn parse_percent_from_line_handles_simple_percent() {
        let value = parse_percent_from_line("coverage results: 99% (123/124)").unwrap();
        assert!((value - 99.0).abs() < 1e-6);
    }

    #[test]
    fn extract_coverage_percent_prefers_results_line() {
        let output = "Line Coverage: 70.1%\nCoverage Results: 91.2%\nCoverage: 92.0%";
        assert_eq!(extract_coverage_percent(output), Some(91.2));
    }

    #[test]
    fn extract_coverage_percent_falls_back_to_last_match() {
        let output = "Line Coverage: 70.1%\nTotal coverage: 72.0%";
        assert_eq!(extract_coverage_percent(output), Some(72.0));
    }

    #[test]
    fn extract_coverage_percent_reads_line_coverage() {
        let output = "line coverage: 64.3%";
        assert_eq!(extract_coverage_percent(output), Some(64.3));
    }

    #[test]
    fn extract_coverage_percent_reads_generic_coverage() {
        let output = "coverage: 83.7%";
        assert_eq!(extract_coverage_percent(output), Some(83.7));
    }

    #[test]
    fn parse_review_rating_accepts_fraction_and_percent() {
        let fraction = parse_review_rating("Rating: 8/10").unwrap();
        assert!((fraction - 8.0).abs() < 1e-6);
        let percent = parse_review_rating("Quality score: 92%").unwrap();
        assert!((percent - 9.2).abs() < 1e-6);
    }

    #[test]
    fn parse_review_rating_scales_low_values() {
        let rating = parse_review_rating("Score: 0.8").unwrap();
        assert!((rating - 8.0).abs() < 1e-6);
    }

    #[test]
    fn parse_review_issue_count_handles_zero_and_number() {
        assert_eq!(parse_review_issue_count("No issues found."), Some(0));
        assert_eq!(parse_review_issue_count("Issues: 3 blocking"), Some(3));
    }

    #[test]
    fn parse_review_issue_count_returns_none_without_issue_line() {
        assert_eq!(parse_review_issue_count("Looks good overall."), None);
    }

    #[test]
    fn evaluate_review_gate_waits_for_required_approval() {
        let mut settings = base_review_settings();
        settings.require_approval = true;
        let pr_view = json!({
            "reviews": [
                {
                    "author": { "login": "greptile" },
                    "state": "COMMENTED",
                    "body": "Rating: 9/10",
                    "submittedAt": "2024-01-02T00:00:00Z"
                }
            ]
        });
        let decision = evaluate_review_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Pending(message) if message.contains("approval")));
    }

    #[test]
    fn evaluate_review_gate_waits_for_reviewer() {
        let settings = base_review_settings();
        let pr_view = json!({
            "reviews": []
        });
        let decision = evaluate_review_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Pending(message) if message.contains("waiting")));
    }

    #[test]
    fn evaluate_review_gate_fails_on_changes_requested() {
        let settings = base_review_settings();
        let pr_view = json!({
            "reviews": [
                {
                    "author": { "login": "greptile" },
                    "state": "CHANGES_REQUESTED",
                    "body": "Rating: 9/10",
                    "submittedAt": "2024-01-04T00:00:00Z"
                }
            ]
        });
        let decision = evaluate_review_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Failed(message) if message.contains("requested")));
    }

    #[test]
    fn evaluate_review_gate_fails_on_low_rating() {
        let settings = base_review_settings();
        let pr_view = json!({
            "reviews": [
                {
                    "author": { "login": "greptile" },
                    "state": "APPROVED",
                    "body": "Rating: 6/10",
                    "submittedAt": "2024-01-02T00:00:00Z"
                }
            ]
        });
        let decision = evaluate_review_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Failed(message) if message.contains("rating")));
    }

    #[test]
    fn evaluate_review_gate_passes_with_rating_and_issue_budget() {
        let settings = base_review_settings();
        let pr_view = json!({
            "reviews": [
                {
                    "author": { "login": "greptile" },
                    "state": "APPROVED",
                    "body": "Rating: 9/10\nIssues: 0",
                    "submittedAt": "2024-01-03T00:00:00Z"
                }
            ]
        });
        let decision = evaluate_review_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Passed(_)));
    }

    #[test]
    fn evaluate_review_gate_fails_on_issue_budget() {
        let settings = base_review_settings();
        let pr_view = json!({
            "reviews": [
                {
                    "author": { "login": "greptile" },
                    "state": "APPROVED",
                    "body": "Rating: 9/10\nIssues: 2",
                    "submittedAt": "2024-01-05T00:00:00Z"
                }
            ]
        });
        let decision = evaluate_review_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Failed(message) if message.contains("issue")));
    }

    #[test]
    fn resolve_review_gate_merge_method_rejects_invalid_value() {
        let err = resolve_review_gate_merge_method(Some("fast-forward".to_string())).unwrap_err();
        match err {
            CliError::Message(message) => {
                assert!(message.contains("merge_method"));
            }
            other => panic!("expected message error, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_check_gate_reports_pending_checks() {
        let settings = base_review_settings();
        let pr_view = json!({
            "statusCheckRollup": [
                {
                    "name": "ci",
                    "status": "IN_PROGRESS",
                    "conclusion": ""
                }
            ]
        });
        let decision = evaluate_check_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Pending(message) if message.contains("checks pending")));
    }

    #[test]
    fn evaluate_check_gate_skips_when_disabled() {
        let mut settings = base_review_settings();
        settings.require_checks = false;
        let pr_view = json!({
            "statusCheckRollup": [
                {
                    "name": "ci",
                    "status": "COMPLETED",
                    "conclusion": "FAILURE"
                }
            ]
        });
        let decision = evaluate_check_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Passed(message) if message.contains("skipped")));
    }

    #[test]
    fn evaluate_check_gate_reports_failed_checks() {
        let settings = base_review_settings();
        let pr_view = json!({
            "statusCheckRollup": [
                {
                    "name": "ci",
                    "status": "COMPLETED",
                    "conclusion": "FAILURE"
                }
            ]
        });
        let decision = evaluate_check_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Failed(message) if message.contains("checks failed")));
    }

    #[test]
    fn evaluate_check_gate_passes_successful_checks() {
        let settings = base_review_settings();
        let pr_view = json!({
            "statusCheckRollup": [
                {
                    "name": "ci",
                    "status": "COMPLETED",
                    "conclusion": "SUCCESS"
                }
            ]
        });
        let decision = evaluate_check_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Passed(_)));
    }

    #[test]
    fn evaluate_check_gate_reads_state_and_result_fields() {
        let settings = base_review_settings();
        let pr_view = json!({
            "statusCheckRollup": [
                {
                    "context": "ci",
                    "state": "COMPLETED",
                    "result": "SUCCESS"
                }
            ]
        });
        let decision = evaluate_check_gate(&pr_view, &settings).unwrap();
        assert!(matches!(decision, GateDecision::Passed(_)));
    }

    #[test]
    fn wildcard_match_supports_simple_globs() {
        assert!(wildcard_match("src/*.rs", "src/main.rs"));
        assert!(!wildcard_match("src/*.rs", "src/main.ts"));
        assert!(wildcard_match("docs/*.md", "docs/readme.md"));
    }

    #[test]
    fn path_matches_any_strips_double_star_prefix() {
        let patterns = vec!["**/docs/*.md".to_string(), "README.md".to_string()];
        assert!(path_matches_any("docs/readme.md", &patterns));
        assert!(path_matches_any("root/docs/readme.md", &patterns));
        assert!(path_matches_any("README.md", &patterns));
        assert!(!path_matches_any("docs/readme.txt", &patterns));
    }

    #[test]
    fn line_contains_marker_respects_boundaries() {
        let markers = vec!["TODO".to_string(), "FIXME".to_string()];
        assert_eq!(line_contains_marker("todo: fix", &markers), Some("TODO".to_string()));
        assert_eq!(line_contains_marker("METHODODO", &markers), None);
        assert_eq!(line_contains_marker("TODO_", &markers), None);
        assert_eq!(line_contains_marker("FIXME.", &markers), Some("FIXME".to_string()));
    }

    #[test]
    fn check_todo_markers_reports_violation() {
        let settings = base_static_settings();
        let lines = vec!["clean".to_string(), "// TODO: follow up".to_string()];
        let mut violations = Vec::new();
        check_todo_markers(Path::new("src/main.rs"), &lines, &settings, &mut violations);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 2);
        assert!(violations[0].message.contains("TODO"));
    }

    #[test]
    fn comment_style_for_path_handles_known_extensions() {
        let rust_style = comment_style_for_path(Path::new("lib.rs")).unwrap();
        assert!(rust_style.line_prefixes.contains(&"//"));
        assert_eq!(rust_style.block_start, Some("/*"));
        let sql_style = comment_style_for_path(Path::new("query.sql")).unwrap();
        assert!(sql_style.line_prefixes.contains(&"--"));
        assert_eq!(sql_style.block_end, Some("*/"));
        assert!(comment_style_for_path(Path::new("README.md")).is_none());
    }

    #[test]
    fn comment_text_len_handles_prefixes_and_blocks() {
        let style = comment_style_for_path(Path::new("lib.rs")).unwrap();
        assert_eq!(comment_text_len("// comment", &style), "comment".len());
        assert_eq!(
            comment_text_len("/* comment */", &style),
            "comment */".len()
        );
        assert_eq!(comment_text_len("*/ trailing", &style), "trailing".len());
        assert_eq!(comment_text_len("* continued", &style), "continued".len());
    }

    #[test]
    fn check_verbose_comments_flags_excessive_blocks() {
        let mut settings = base_static_settings();
        settings.max_comment_lines = 1;
        settings.max_comment_chars = 10;
        let lines = vec![
            "// short".to_string(),
            "// this is too long".to_string(),
            "let x = 1;".to_string(),
        ];
        let mut violations = Vec::new();
        check_verbose_comments(Path::new("src/lib.rs"), &lines, &settings, &mut violations);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert!(violations[0].message.contains("Verbose comment block"));
    }

    #[test]
    fn check_verbose_comments_allows_short_blocks() {
        let mut settings = base_static_settings();
        settings.max_comment_lines = 2;
        settings.max_comment_chars = 50;
        let lines = vec!["// ok".to_string(), "let y = 2;".to_string()];
        let mut violations = Vec::new();
        check_verbose_comments(Path::new("src/lib.rs"), &lines, &settings, &mut violations);
        assert!(violations.is_empty());
    }

    #[test]
    fn split_nonempty_blocks_tracks_start_lines() {
        let lines = vec![
            "".to_string(),
            "alpha".to_string(),
            "beta".to_string(),
            "".to_string(),
            " ".to_string(),
            "gamma".to_string(),
        ];
        let blocks = split_nonempty_blocks(&lines);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, 2);
        assert_eq!(blocks[0].1, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(blocks[1].0, 6);
        assert_eq!(blocks[1].1, vec!["gamma".to_string()]);
    }

    #[test]
    fn block_is_substantive_counts_alnum_lines() {
        let non_alnum = vec!["--".to_string(), "   ".to_string(), "##".to_string()];
        assert!(!block_is_substantive(&non_alnum, 1));
        let some_alnum = vec!["--".to_string(), "alpha".to_string(), "123".to_string()];
        assert!(block_is_substantive(&some_alnum, 2));
        assert!(!block_is_substantive(&some_alnum, 3));
    }

    #[test]
    fn find_duplicate_blocks_reports_duplicate_locations() {
        let settings = StaticCheckSettings {
            enabled: true,
            check_todo: false,
            check_comments: false,
            check_duplicates: true,
            allow_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            todo_markers: Vec::new(),
            max_comment_lines: DEFAULT_STATIC_MAX_COMMENT_LINES,
            max_comment_chars: DEFAULT_STATIC_MAX_COMMENT_CHARS,
            duplicate_block_lines: 2,
            duplicate_min_alnum_lines: 1,
            max_file_bytes: DEFAULT_STATIC_MAX_FILE_BYTES,
        };
        let first = FileSnapshot {
            path: PathBuf::from("src/alpha.rs"),
            lines: vec![
                "let a = 1;".to_string(),
                "let b = 2;".to_string(),
                "".to_string(),
                "unique".to_string(),
            ],
        };
        let second = FileSnapshot {
            path: PathBuf::from("src/beta.rs"),
            lines: vec![
                "let   a    =   1;".to_string(),
                "let   b =   2;".to_string(),
                "".to_string(),
                "other".to_string(),
            ],
        };
        let violations = find_duplicate_blocks(&[first, second], &settings);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, PathBuf::from("src/beta.rs"));
        assert_eq!(violations[0].line, 1);
        assert!(violations[0]
            .message
            .contains("Duplicate block matches"));
    }

    #[test]
    fn path_is_allowed_respects_allow_patterns() {
        let allow = vec!["**/*.rs".to_string(), "docs/**".to_string()];
        assert!(path_is_allowed("src/main.rs", &allow));
        assert!(path_is_allowed("docs/guide.md", &allow));
        assert!(!path_is_allowed("README.md", &allow));
    }

    #[test]
    fn path_is_ignored_matches_directory_patterns() {
        let ignore = vec!["**/target/**".to_string()];
        assert!(path_is_ignored("target", true, &ignore));
        assert!(path_is_ignored("target/debug/app", false, &ignore));
    }
}
