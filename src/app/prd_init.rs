use super::{join_or_none, normalize_csv, CliError};
use crate::backend::backend_from_name;
use crate::cli::{InitArgs, PrdArgs, PrdCheckArgs, PrdCommand, PrdCreateArgs};
use crate::config::Config;
use crate::prd;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(super) fn cmd_prd(args: PrdArgs) -> Result<(), CliError> {
    match args.command {
        PrdCommand::Check(args) => cmd_prd_check(args),
        PrdCommand::Create(args) => cmd_prd_create(args),
    }
}

pub(super) fn cmd_init(args: InitArgs) -> Result<(), CliError> {
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

pub(super) fn resolve_prd_output(
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

pub(super) fn invalid_prd_path(output: &Path, force: bool) -> PathBuf {
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

pub(super) fn read_prd_template_with_manifest(
    dir: &Path,
    manifest_dir: &Path,
) -> Result<String, CliError> {
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

pub(super) fn resolve_init_context_files(
    target_dir: &Path,
    config_list: Option<&str>,
) -> Vec<String> {
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

pub(super) fn read_readme_context_files(target_dir: &Path) -> Vec<String> {
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

pub(super) fn default_context_files() -> [&'static str; 5] {
    [
        "ARCHITECTURE.md",
        "PROCESS.md",
        "DECISIONS.md",
        "RISK_REGISTER.md",
        "CHANGELOG.md",
    ]
}

pub(super) fn is_markdown_path(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md") | Some("markdown") => true,
        _ => false,
    }
}

pub(super) fn format_display_path(path: &Path, base: &Path) -> String {
    if path.starts_with(base) {
        path.strip_prefix(base)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    } else {
        path.to_string_lossy().to_string()
    }
}

pub(super) fn init_template_for_path(path: &Path) -> String {
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

pub(super) fn generic_markdown_template(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Context");
    let title = stem.replace('_', " ");
    format!("# {}\n\n## Overview\n\nTBD.\n", title)
}

pub(super) fn write_atomic(path: &Path, contents: &str, _force: bool) -> Result<(), io::Error> {
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

pub(super) fn build_context_file_list(
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

pub(super) fn add_context_entry(
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

pub(super) fn write_allowed_context(entries: &[String]) -> Result<Option<PathBuf>, CliError> {
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

pub(super) const DEFAULT_PRD_TEMPLATE: &str = "## Overview\n\nBriefly describe the project, goals, and intended users.\n\n## Problem Statement\n\n- What problem does this solve?\n- What pain points exist today?\n\n## Solution\n\nHigh-level solution summary.\n\n---\n\n## Functional Requirements\n\n### FR-1: Core Feature\n\nDescribe the primary user-facing behavior.\n\n### FR-2: Secondary Feature\n\nDescribe supporting behavior.\n\n---\n\n## Non-Functional Requirements\n\n### NFR-1: Performance\n\n- Example: Response times under 200ms for key operations.\n\n### NFR-2: Reliability\n\n- Example: Crash recovery or retries where appropriate.\n\n---\n\n## Implementation Tasks\n\nEach task must use a `### Task <ID>` block header and include the required fields.\nEach task block must contain exactly one unchecked task line.\n\n### Task EX-1\n\n- **ID** EX-1\n- **Context Bundle** `path/to/file`, `path/to/other`\n- **DoD** Define the done criteria for this task.\n- **Checklist**\n  * First verification item.\n  * Second verification item.\n- **Dependencies** None\n- [ ] EX-1 Short task summary\n\n---\n\n## Success Criteria\n\n- Define measurable outcomes that indicate completion.\n\n---\n\n## Sources\n\n- List authoritative URLs used as source of truth.\n\n---\n\n## Warnings\n\n- Only include this section if no reliable sources were found.\n- State what is missing and what must be verified.\n";

pub(super) const ARCHITECTURE_TEMPLATE: &str = "# Architecture\n\n## Overview\n\nDescribe the system at a high level.\n\n## Modules\n\nList key modules and what they own.\n\n## Runtime Flow\n\nDescribe the primary runtime path.\n\n## Storage\n\nRecord where state or data is stored.\n";

pub(super) const PROCESS_TEMPLATE: &str = "# Process\n\n## Worktree Protocol\n\n1) Read required context files.\n2) Create a task worktree.\n3) Implement the scoped task.\n4) Update shared docs as needed.\n5) Verify changes.\n6) Finish and merge worktree.\n\n## Guardrails\n\n- Keep changes scoped to the assigned task.\n- Update CHANGELOG with the task ID.\n- Record new decisions and risks.\n";

pub(super) const DECISIONS_TEMPLATE: &str = "# Decisions\n\n## D-001 Decision Title\n- Date: YYYY-MM-DD\n- Status: Proposed\n\n### Context\n\nWhy this decision is needed.\n\n### Decision\n\nWhat was decided.\n\n### Rationale\n\nWhy this choice was made.\n\n### Alternatives\n\nOther options considered.\n";

pub(super) const RISK_REGISTER_TEMPLATE: &str = "# Risk Register\n\n## R-001 Risk Title\n- Risk: Describe the risk.\n- Impact: Low/Medium/High\n- Mitigation: How to reduce or monitor it.\n";

pub(super) const CHANGELOG_TEMPLATE: &str = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nThe format is based on Keep a Changelog.\n\n## [Unreleased]\n\n### Added\n\n### Fixed\n";
