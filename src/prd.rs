use crate::task::{
    is_task_block_end, is_task_header, is_unchecked_line, task_blocks_from_contents,
};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StackDetection {
    pub root: Option<PathBuf>,
    pub ids: Vec<String>,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub tools: Vec<String>,
    pub runtimes: Vec<String>,
    pub package_managers: Vec<String>,
    pub evidence: Vec<String>,
    pub selected_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PrdValidationError {
    pub messages: Vec<String>,
}

impl fmt::Display for PrdValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.messages.join("\n"))
    }
}

impl std::error::Error for PrdValidationError {}

#[derive(Debug)]
pub enum PrdError {
    Io { path: PathBuf, source: io::Error },
    Validation(PrdValidationError),
}

impl fmt::Display for PrdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrdError::Io { path, source } => {
                write!(f, "prd io error at {}: {}", path.display(), source)
            }
            PrdError::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for PrdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PrdError::Io { source, .. } => Some(source),
            PrdError::Validation(err) => Some(err),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct AllowedContext {
    ordered: Vec<String>,
    lookup: HashSet<String>,
}

impl AllowedContext {
    fn insert(&mut self, value: &str) {
        if value.is_empty() {
            return;
        }
        let entry = value.to_string();
        if self.lookup.insert(entry.clone()) {
            self.ordered.push(entry);
        }
    }

    fn is_empty(&self) -> bool {
        self.lookup.is_empty()
    }

    fn contains(&self, value: &str) -> bool {
        self.lookup.contains(value)
    }
}

pub fn prd_validate_file(
    task_file: &Path,
    allow_missing_context: bool,
    base_dir_override: Option<&Path>,
) -> Result<(), PrdValidationError> {
    let mut errors = Vec::new();

    if task_file.as_os_str().is_empty() {
        errors.push("Error: task_file is required".to_string());
        return Err(PrdValidationError { messages: errors });
    }
    if !task_file.is_file() {
        errors.push(format!(
            "Error: Task file does not exist: {}",
            task_file.display()
        ));
        return Err(PrdValidationError { messages: errors });
    }

    let base_dir = resolve_base_dir(task_file, base_dir_override);
    let contents = match fs::read_to_string(task_file) {
        Ok(contents) => contents,
        Err(err) => {
            errors.push(format!(
                "Error: Task file could not be read: {}: {}",
                task_file.display(),
                err
            ));
            return Err(PrdValidationError { messages: errors });
        }
    };

    prd_validate_contents(
        &contents,
        task_file,
        allow_missing_context,
        base_dir.as_deref(),
    )
}

pub fn prd_validate_contents(
    contents: &str,
    task_file: &Path,
    allow_missing_context: bool,
    base_dir: Option<&Path>,
) -> Result<(), PrdValidationError> {
    let mut errors = Vec::new();

    if contents.trim().is_empty() {
        errors.push(format!(
            "PRD validation error: {}: Task file is empty",
            task_file.display()
        ));
        return Err(PrdValidationError { messages: errors });
    }

    if has_open_questions_section(contents) {
        errors.push(format!(
            "PRD validation error: {}: Open Questions section is not allowed",
            task_file.display()
        ));
    }

    if let Some(stray_message) = validate_stray_unchecked(contents, task_file) {
        errors.extend(stray_message);
    }

    for block in task_blocks_from_contents(contents) {
        errors.extend(validate_task_block(
            &block,
            task_file,
            allow_missing_context,
            base_dir,
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(PrdValidationError { messages: errors })
    }
}

pub fn prd_sanitize_generated_file(
    task_file: &Path,
    base_dir: Option<&Path>,
    allowed_context_file: Option<&Path>,
) -> Result<(), PrdError> {
    if task_file.as_os_str().is_empty() || !task_file.is_file() {
        return Ok(());
    }

    let base_dir = base_dir
        .map(|path| path.to_path_buf())
        .or_else(|| task_file.parent().map(|path| path.to_path_buf()));

    let contents = fs::read_to_string(task_file).map_err(|source| PrdError::Io {
        path: task_file.to_path_buf(),
        source,
    })?;

    let allowed_context = load_allowed_context(allowed_context_file);
    let output = prd_sanitize_contents(&contents, base_dir.as_deref(), &allowed_context);

    fs::write(task_file, output).map_err(|source| PrdError::Io {
        path: task_file.to_path_buf(),
        source,
    })?;

    Ok(())
}

fn prd_sanitize_contents(
    contents: &str,
    base_dir: Option<&Path>,
    allowed_context: &AllowedContext,
) -> String {
    let mut output = String::new();
    let mut block = String::new();
    let mut in_block = false;
    let mut in_open_questions = false;
    let mut started = false;

    for line in contents.lines() {
        let lower = line.to_lowercase();

        if is_open_questions_heading(&lower) {
            in_open_questions = true;
            continue;
        }

        if in_open_questions {
            if is_heading(line) {
                in_open_questions = false;
            } else {
                continue;
            }
        }

        if !started {
            if line.trim_start().starts_with('#') {
                started = true;
            } else {
                continue;
            }
        }

        if is_task_header(line) {
            if in_block {
                output.push_str(&sanitize_task_block_with_allowed(
                    &block,
                    base_dir,
                    allowed_context,
                ));
            }
            in_block = true;
            block.clear();
            block.push_str(line);
            continue;
        }

        if in_block && is_task_block_end(line) {
            output.push_str(&sanitize_task_block_with_allowed(
                &block,
                base_dir,
                allowed_context,
            ));
            in_block = false;
            block.clear();
        }

        if in_block {
            block.push('\n');
            block.push_str(line);
        } else {
            let sanitized = remove_unchecked_checkbox(line);
            output.push_str(&sanitized);
            output.push('\n');
        }
    }

    if in_block {
        output.push_str(&sanitize_task_block_with_allowed(
            &block,
            base_dir,
            allowed_context,
        ));
    }

    output
}

pub fn prd_detect_stack(target_dir: &Path) -> StackDetection {
    let mut detection = StackDetection::default();
    if target_dir.as_os_str().is_empty() || !target_dir.is_dir() {
        return detection;
    }

    let root = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.to_path_buf());
    detection.root = Some(root.clone());

    let package_json = root.join("package.json");
    if package_json.is_file() {
        add_unique(&mut detection.ids, "Node.js");
        add_unique(&mut detection.runtimes, "Node.js");
        add_unique(&mut detection.languages, "JavaScript");
        record_stack_file(&mut detection, &package_json);

        let tsconfig = root.join("tsconfig.json");
        if tsconfig.is_file() {
            add_unique(&mut detection.languages, "TypeScript");
            record_stack_file(&mut detection, &tsconfig);
        }

        let pnpm_lock = root.join("pnpm-lock.yaml");
        if pnpm_lock.is_file() {
            add_unique(&mut detection.package_managers, "pnpm");
            record_stack_file(&mut detection, &pnpm_lock);
        }

        let yarn_lock = root.join("yarn.lock");
        if yarn_lock.is_file() {
            add_unique(&mut detection.package_managers, "yarn");
            record_stack_file(&mut detection, &yarn_lock);
        }

        let npm_lock = root.join("package-lock.json");
        if npm_lock.is_file() {
            add_unique(&mut detection.package_managers, "npm");
            record_stack_file(&mut detection, &npm_lock);
        }

        let bun_lock = root.join("bun.lockb");
        if bun_lock.is_file() {
            add_unique(&mut detection.runtimes, "Bun");
            add_unique(&mut detection.package_managers, "bun");
            record_stack_file(&mut detection, &bun_lock);
        }

        let bunfig = root.join("bunfig.toml");
        if bunfig.is_file() {
            add_unique(&mut detection.runtimes, "Bun");
            add_unique(&mut detection.package_managers, "bun");
            record_stack_file(&mut detection, &bunfig);
        }

        add_framework_if_file_exists(&mut detection, &root, "next.config.js", "Next.js");
        add_framework_if_file_exists(&mut detection, &root, "next.config.mjs", "Next.js");
        add_framework_if_file_exists(&mut detection, &root, "next.config.cjs", "Next.js");
        add_framework_if_file_exists(&mut detection, &root, "nuxt.config.js", "Nuxt");
        add_framework_if_file_exists(&mut detection, &root, "nuxt.config.ts", "Nuxt");
        add_framework_if_file_exists(&mut detection, &root, "svelte.config.js", "Svelte");
        add_framework_if_file_exists(&mut detection, &root, "svelte.config.ts", "Svelte");

        add_tool_if_file_exists(&mut detection, &root, "vite.config.js", "Vite");
        add_tool_if_file_exists(&mut detection, &root, "vite.config.ts", "Vite");
        add_tool_if_file_exists(&mut detection, &root, "vite.config.mjs", "Vite");

        add_framework_if_file_exists(&mut detection, &root, "angular.json", "Angular");
        add_framework_if_file_exists(&mut detection, &root, "vue.config.js", "Vue");

        if json_has_dependency(&package_json, "react") {
            add_unique(&mut detection.frameworks, "React");
        }
        if json_has_dependency(&package_json, "next") {
            add_unique(&mut detection.frameworks, "Next.js");
        }
        if json_has_dependency(&package_json, "vue") {
            add_unique(&mut detection.frameworks, "Vue");
        }
        if json_has_dependency(&package_json, "@angular/core") {
            add_unique(&mut detection.frameworks, "Angular");
        }
        if json_has_dependency(&package_json, "svelte") {
            add_unique(&mut detection.frameworks, "Svelte");
        }
        if json_has_dependency(&package_json, "nuxt") {
            add_unique(&mut detection.frameworks, "Nuxt");
        }
        if json_has_dependency(&package_json, "express") {
            add_unique(&mut detection.frameworks, "Express");
        }
        if json_has_dependency(&package_json, "fastify") {
            add_unique(&mut detection.frameworks, "Fastify");
        }
        if json_has_dependency(&package_json, "@nestjs/core") {
            add_unique(&mut detection.frameworks, "NestJS");
        }
    }

    let go_mod = root.join("go.mod");
    if go_mod.is_file() {
        add_unique(&mut detection.ids, "Go");
        add_unique(&mut detection.languages, "Go");
        add_unique(&mut detection.tools, "Go modules");
        record_stack_file(&mut detection, &go_mod);
    }

    let cargo = root.join("Cargo.toml");
    if cargo.is_file() {
        add_unique(&mut detection.ids, "Rust");
        add_unique(&mut detection.languages, "Rust");
        add_unique(&mut detection.tools, "Cargo");
        record_stack_file(&mut detection, &cargo);
    }

    let pyproject = root.join("pyproject.toml");
    let requirements = root.join("requirements.txt");
    let poetry_lock = root.join("poetry.lock");
    let pipfile = root.join("Pipfile");
    let pipfile_lock = root.join("Pipfile.lock");
    if pyproject.is_file()
        || requirements.is_file()
        || poetry_lock.is_file()
        || pipfile.is_file()
        || pipfile_lock.is_file()
    {
        add_unique(&mut detection.ids, "Python");
        add_unique(&mut detection.languages, "Python");
        if pyproject.is_file() {
            record_stack_file(&mut detection, &pyproject);
            if contains_case_insensitive(&pyproject, "[tool.poetry]") {
                add_unique(&mut detection.tools, "Poetry");
            }
        }
        if requirements.is_file() {
            record_stack_file(&mut detection, &requirements);
            if requirements_contains(&requirements, "django") {
                add_unique(&mut detection.frameworks, "Django");
            }
            if requirements_contains(&requirements, "flask") {
                add_unique(&mut detection.frameworks, "Flask");
            }
            if requirements_contains(&requirements, "fastapi") {
                add_unique(&mut detection.frameworks, "FastAPI");
            }
        }
        if poetry_lock.is_file() {
            record_stack_file(&mut detection, &poetry_lock);
        }
        if pipfile.is_file() {
            record_stack_file(&mut detection, &pipfile);
        }
        if pipfile_lock.is_file() {
            record_stack_file(&mut detection, &pipfile_lock);
        }

        if pyproject.is_file()
            && (contains_case_insensitive(&pyproject, "django")
                || contains_case_insensitive(&pyproject, "flask")
                || contains_case_insensitive(&pyproject, "fastapi"))
        {
            if contains_case_insensitive(&pyproject, "django") {
                add_unique(&mut detection.frameworks, "Django");
            }
            if contains_case_insensitive(&pyproject, "flask") {
                add_unique(&mut detection.frameworks, "Flask");
            }
            if contains_case_insensitive(&pyproject, "fastapi") {
                add_unique(&mut detection.frameworks, "FastAPI");
            }
        }
    }

    let gemfile = root.join("Gemfile");
    if gemfile.is_file() {
        add_unique(&mut detection.ids, "Ruby");
        add_unique(&mut detection.languages, "Ruby");
        record_stack_file(&mut detection, &gemfile);
        if contains_case_insensitive(&gemfile, "rails") {
            add_unique(&mut detection.frameworks, "Rails");
        }
        if contains_case_insensitive(&gemfile, "sinatra") {
            add_unique(&mut detection.frameworks, "Sinatra");
        }
    }

    let mix = root.join("mix.exs");
    if mix.is_file() {
        add_unique(&mut detection.ids, "Elixir");
        add_unique(&mut detection.languages, "Elixir");
        record_stack_file(&mut detection, &mix);
        if contains_case_insensitive(&mix, "phoenix") {
            add_unique(&mut detection.frameworks, "Phoenix");
        }
    }

    let composer = root.join("composer.json");
    if composer.is_file() {
        add_unique(&mut detection.ids, "PHP");
        add_unique(&mut detection.languages, "PHP");
        record_stack_file(&mut detection, &composer);
        if contains_case_insensitive(&composer, "laravel") {
            add_unique(&mut detection.frameworks, "Laravel");
        }
    }

    let pom = root.join("pom.xml");
    if pom.is_file() {
        add_unique(&mut detection.ids, "Java");
        add_unique(&mut detection.languages, "Java");
        add_unique(&mut detection.tools, "Maven");
        record_stack_file(&mut detection, &pom);
        if contains_case_insensitive(&pom, "spring-boot") {
            add_unique(&mut detection.frameworks, "Spring Boot");
        }
    }

    let gradle = root.join("build.gradle");
    if gradle.is_file() {
        add_unique(&mut detection.ids, "Java");
        add_unique(&mut detection.languages, "Java");
        add_unique(&mut detection.tools, "Gradle");
        record_stack_file(&mut detection, &gradle);
        if contains_case_insensitive(&gradle, "spring-boot") {
            add_unique(&mut detection.frameworks, "Spring Boot");
        }
    }
    let gradle_kts = root.join("build.gradle.kts");
    if gradle_kts.is_file() {
        add_unique(&mut detection.ids, "Java");
        add_unique(&mut detection.languages, "Java");
        add_unique(&mut detection.tools, "Gradle");
        record_stack_file(&mut detection, &gradle_kts);
        if contains_case_insensitive(&gradle_kts, "spring-boot") {
            add_unique(&mut detection.frameworks, "Spring Boot");
        }
    }

    let mut has_dotnet = false;
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if ext.eq_ignore_ascii_case("csproj") || ext.eq_ignore_ascii_case("sln") {
                    record_stack_file(&mut detection, &path);
                    has_dotnet = true;
                }
            }
        }
    }
    if has_dotnet {
        add_unique(&mut detection.ids, ".NET");
        add_unique(&mut detection.languages, "C#");
    }

    let dockerfile = root.join("Dockerfile");
    if dockerfile.is_file() {
        add_unique(&mut detection.tools, "Docker");
        record_stack_file(&mut detection, &dockerfile);
    }
    let compose_yml = root.join("docker-compose.yml");
    if compose_yml.is_file() {
        add_unique(&mut detection.tools, "Docker Compose");
        record_stack_file(&mut detection, &compose_yml);
    }
    let compose_yaml = root.join("docker-compose.yaml");
    if compose_yaml.is_file() {
        add_unique(&mut detection.tools, "Docker Compose");
        record_stack_file(&mut detection, &compose_yaml);
    }
    let makefile = root.join("Makefile");
    if makefile.is_file() {
        add_unique(&mut detection.tools, "Make");
        record_stack_file(&mut detection, &makefile);
    }

    let mut has_terraform = false;
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("tf"))
                .unwrap_or(false)
            {
                record_stack_file(&mut detection, &path);
                has_terraform = true;
            }
        }
    }
    if has_terraform {
        add_unique(&mut detection.tools, "Terraform");
    }

    detection.selected_ids = detection.ids.clone();
    detection
}

pub fn prd_format_stack_summary(detection: &StackDetection, heading_level: u8) -> String {
    let header_prefix = if heading_level == 1 { "#" } else { "##" };
    let stacks_line = join_or_default(&detection.ids, "Unknown");
    let languages_line = join_or_default(&detection.languages, "Unknown");
    let frameworks_line = join_or_default(&detection.frameworks, "None detected");
    let tools_line = join_or_default(&detection.tools, "None detected");
    let runtimes_line = join_or_default(&detection.runtimes, "Unknown");
    let package_line = join_or_default(&detection.package_managers, "None detected");

    let mut output = String::new();
    output.push_str(header_prefix);
    output.push_str(" Stack Summary\n\n");
    output.push_str(&format!("- Stacks: {}\n", stacks_line));
    output.push_str(&format!("- Languages: {}\n", languages_line));
    output.push_str(&format!("- Runtimes: {}\n", runtimes_line));
    output.push_str(&format!("- Frameworks: {}\n", frameworks_line));
    output.push_str(&format!("- Tools: {}\n", tools_line));
    output.push_str(&format!("- Package managers: {}\n", package_line));

    if !detection.selected_ids.is_empty() && detection.selected_ids.len() < detection.ids.len() {
        let selected_line = join_or_default(&detection.selected_ids, "");
        output.push_str(&format!("- Stack focus: {}\n", selected_line));
    }

    output.push_str("\nEvidence:\n");
    if detection.evidence.is_empty() {
        output.push_str("- None found\n");
    } else {
        for item in &detection.evidence {
            output.push_str(&format!("- {}\n", item));
        }
    }

    output
}

fn join_or_default(values: &[String], default_value: &str) -> String {
    if values.is_empty() {
        default_value.to_string()
    } else {
        values.join(", ")
    }
}

fn add_unique(values: &mut Vec<String>, value: &str) {
    if value.is_empty() || values.iter().any(|item| item == value) {
        return;
    }
    values.push(value.to_string());
}

fn record_stack_file(detection: &mut StackDetection, path: &Path) {
    let mut display = path.to_path_buf();
    if let Some(root) = detection.root.as_ref() {
        if path.starts_with(root) {
            if let Ok(rel) = path.strip_prefix(root) {
                display = rel.to_path_buf();
            }
        }
    }
    let display = display.to_string_lossy().to_string();
    add_unique(&mut detection.evidence, &display);
}

fn add_framework_if_file_exists(
    detection: &mut StackDetection,
    root: &Path,
    filename: &str,
    framework: &str,
) {
    let path = root.join(filename);
    if path.is_file() {
        add_unique(&mut detection.frameworks, framework);
        record_stack_file(detection, &path);
    }
}

fn add_tool_if_file_exists(
    detection: &mut StackDetection,
    root: &Path,
    filename: &str,
    tool: &str,
) {
    let path = root.join(filename);
    if path.is_file() {
        add_unique(&mut detection.tools, tool);
        record_stack_file(detection, &path);
    }
}

fn json_has_dependency(package_json: &Path, dep: &str) -> bool {
    if dep.is_empty() || !package_json.is_file() {
        return false;
    }
    let contents = match fs::read_to_string(package_json) {
        Ok(contents) => contents,
        Err(_) => return false,
    };

    if let Ok(value) = serde_json::from_str::<Value>(&contents) {
        if json_dep_exists(&value, "dependencies", dep)
            || json_dep_exists(&value, "devDependencies", dep)
            || json_dep_exists(&value, "peerDependencies", dep)
        {
            return true;
        }
    }

    contents.contains(&format!("\"{}\"", dep))
}

fn json_dep_exists(value: &Value, key: &str, dep: &str) -> bool {
    value
        .get(key)
        .and_then(|deps| deps.as_object())
        .map(|deps| deps.contains_key(dep))
        .unwrap_or(false)
}

fn contains_case_insensitive(path: &Path, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return false,
    };
    contents.to_lowercase().contains(&needle.to_lowercase())
}

fn requirements_contains(path: &Path, name: &str) -> bool {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return false,
    };
    let name = name.to_lowercase();
    for line in contents.lines() {
        let lower = line.to_lowercase();
        if lower.trim().is_empty() {
            continue;
        }
        if let Some(index) = lower.find(&name) {
            let before = lower[..index].chars().last();
            let after_index = index + name.len();
            let after = lower[after_index..].chars().next();
            let before_ok = before.map(|c| c.is_whitespace()).unwrap_or(true);
            let after_ok = after
                .map(|c| c.is_whitespace() || c == '<' || c == '>' || c == '=')
                .unwrap_or(true);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

fn resolve_base_dir(task_file: &Path, base_dir_override: Option<&Path>) -> Option<PathBuf> {
    if let Some(override_path) = base_dir_override {
        if let Ok(path) = override_path.canonicalize() {
            return Some(path);
        }
        return Some(override_path.to_path_buf());
    }
    task_file.parent().and_then(|parent| {
        parent
            .canonicalize()
            .ok()
            .or_else(|| Some(parent.to_path_buf()))
    })
}

fn canonicalize_for_compare(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if let Some(parent) = path.parent() {
        if let Ok(canonical_parent) = parent.canonicalize() {
            if let Some(file_name) = path.file_name() {
                return canonical_parent.join(file_name);
            }
            return canonical_parent;
        }
    }
    path.to_path_buf()
}

fn has_open_questions_section(contents: &str) -> bool {
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            continue;
        }
        let without_hashes = trimmed.trim_start_matches('#');
        if !without_hashes.starts_with(' ') {
            continue;
        }
        let rest = without_hashes.trim_start();
        if rest.starts_with("Open Questions") {
            return true;
        }
    }
    false
}

fn validate_stray_unchecked(contents: &str, task_file: &Path) -> Option<Vec<String>> {
    let mut errors = Vec::new();
    let mut in_block = false;
    for (index, line) in contents.lines().enumerate() {
        if is_task_header(line) {
            in_block = true;
        } else if in_block && is_task_block_end(line) {
            in_block = false;
        }

        if !in_block && is_unchecked_line(line) {
            errors.push(format!(
                "PRD validation error: {}: line {}: Unchecked task line outside task block",
                task_file.display(),
                index + 1
            ));
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

fn validate_task_block(
    block: &str,
    task_file: &Path,
    allow_missing_context: bool,
    base_dir: Option<&Path>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let task_label = task_label(block);
    let fields = ["ID", "Context Bundle", "DoD", "Checklist", "Dependencies"];

    for field in fields {
        if !block_has_field(block, field) {
            errors.push(format!(
                "PRD validation error: {}: {}: Missing required field: {}",
                task_file.display(),
                task_label,
                field
            ));
        }
    }

    let unchecked_count = block.lines().filter(|line| is_unchecked_line(line)).count();
    if unchecked_count == 0 {
        errors.push(format!(
            "PRD validation error: {}: {}: Missing unchecked task line",
            task_file.display(),
            task_label
        ));
    } else if unchecked_count > 1 {
        errors.push(format!(
            "PRD validation error: {}: {}: Multiple unchecked task lines ({})",
            task_file.display(),
            task_label,
            unchecked_count
        ));
    }

    if !allow_missing_context {
        let mut context_entries = Vec::new();
        for entry in extract_context_entries(block) {
            let trimmed = entry.trim();
            if !trimmed.is_empty() {
                context_entries.push(trimmed.to_string());
            }
        }

        if context_entries.is_empty() {
            errors.push(format!(
                "PRD validation error: {}: {}: Context Bundle must include at least one file path",
                task_file.display(),
                task_label
            ));
        } else {
            let base_compare = base_dir.map(canonicalize_for_compare);
            for entry in context_entries {
                let resolved = if Path::new(&entry).is_absolute() {
                    PathBuf::from(&entry)
                } else if let Some(base) = base_dir {
                    base.join(&entry)
                } else {
                    PathBuf::from(&entry)
                };
                let compare_path = if Path::new(&entry).is_absolute() {
                    canonicalize_for_compare(&resolved)
                } else {
                    resolved.clone()
                };

                if Path::new(&entry).is_absolute() {
                    if let Some(base) = base_compare.as_ref() {
                        if !compare_path.starts_with(base) {
                            errors.push(format!(
                                "PRD validation error: {}: {}: Context Bundle path outside repo: {}",
                                task_file.display(),
                                task_label,
                                entry
                            ));
                            continue;
                        }
                    }
                }

                if !resolved.exists() {
                    errors.push(format!(
                        "PRD validation error: {}: {}: Context Bundle path not found: {}",
                        task_file.display(),
                        task_label,
                        entry
                    ));
                }
            }
        }
    }

    errors
}

#[cfg(test)]
fn sanitize_task_block(
    block: &str,
    base_dir: Option<&Path>,
    allowed_context_file: Option<&Path>,
) -> String {
    let allowed_context = load_allowed_context(allowed_context_file);
    sanitize_task_block_with_allowed(block, base_dir, &allowed_context)
}

fn sanitize_task_block_with_allowed(
    block: &str,
    base_dir: Option<&Path>,
    allowed_context: &AllowedContext,
) -> String {
    let mut context_entries = Vec::new();
    for entry in extract_context_entries(block) {
        let trimmed = entry.trim();
        if !trimmed.is_empty() {
            context_entries.push(trimmed.to_string());
        }
    }

    let mut valid_entries = Vec::new();
    for entry in context_entries {
        let display = context_display_path(&entry, base_dir);
        if !context_entry_exists(&display, base_dir) {
            continue;
        }
        if !allowed_context.is_empty() && !allowed_context.contains(&display) {
            continue;
        }
        add_unique(&mut valid_entries, &display);
    }

    if valid_entries.is_empty() {
        if let Some(fallback) = pick_fallback_context(base_dir, allowed_context) {
            valid_entries.push(fallback);
        }
    }

    let context_line = if valid_entries.is_empty() {
        "- **Context Bundle**".to_string()
    } else {
        let formatted = valid_entries
            .iter()
            .map(|entry| format!("`{}`", entry))
            .collect::<Vec<_>>()
            .join(", ");
        format!("- **Context Bundle** {}", formatted)
    };

    let mut output = String::new();
    let mut in_context_block = false;
    let mut unchecked_seen = false;

    for line in block.lines() {
        if let Some(indent) = context_bundle_indent(line) {
            output.push_str(&format!("{}{}\n", indent, context_line));
            in_context_block = true;
            continue;
        }

        if in_context_block {
            if line_has_field(line) {
                in_context_block = false;
            } else {
                continue;
            }
        }

        if let Some((indent, rest)) = unchecked_line_parts(line) {
            if unchecked_seen {
                output.push_str(&format!("{}- {}\n", indent, rest));
            } else {
                unchecked_seen = true;
                output.push_str(line);
                output.push('\n');
            }
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}

fn remove_unchecked_checkbox(line: &str) -> String {
    if let Some((indent, rest)) = unchecked_line_parts(line) {
        format!("{}- {}", indent, rest)
    } else {
        line.to_string()
    }
}

fn unchecked_line_parts(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("- [ ]") {
        return None;
    }
    let indent_len = line.len() - trimmed.len();
    let rest = trimmed.trim_start_matches("- [ ]");
    Some((
        line[..indent_len].to_string(),
        rest.trim_start().to_string(),
    ))
}

fn task_label(block: &str) -> String {
    if let Some(id) = extract_task_id_field(block) {
        return id;
    }
    if let Some(header) = extract_task_header_id(block) {
        return header;
    }
    "unknown".to_string()
}

pub fn prd_task_id_from_block(block: &str) -> Option<String> {
    if let Some(id) = extract_task_id_field(block) {
        return Some(id);
    }
    extract_task_header_id(block)
}

pub fn prd_next_task_id(task_file: &Path) -> Option<String> {
    if task_file.as_os_str().is_empty() || !task_file.is_file() {
        return None;
    }
    let contents = fs::read_to_string(task_file).ok()?;
    for block in task_blocks_from_contents(&contents) {
        if block.lines().any(|line| is_unchecked_line(line)) {
            return prd_task_id_from_block(&block);
        }
    }
    None
}

fn extract_task_header_id(block: &str) -> Option<String> {
    for line in block.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("### Task ") {
            let id = trimmed.trim_start_matches("### Task ").trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn extract_task_id_field(block: &str) -> Option<String> {
    for line in block.lines() {
        if let Some(value) = strip_field_value(line, "ID") {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn block_has_field(block: &str, field: &str) -> bool {
    block.lines().any(|line| line_has_named_field(line, field))
}

fn line_has_field(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return false;
    }
    let after_dash = trimmed[1..].trim_start();
    after_dash.starts_with("**")
}

fn line_has_named_field(line: &str, field: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return false;
    }
    let after_dash = trimmed[1..].trim_start();
    let marker = format!("**{}**", field);
    after_dash.starts_with(&marker)
}

fn strip_field_value(line: &str, field: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return None;
    }
    let after_dash = trimmed[1..].trim_start();
    let marker = format!("**{}**", field);
    if !after_dash.starts_with(&marker) {
        return None;
    }
    let rest = after_dash[marker.len()..].trim_start();
    Some(rest.trim().to_string())
}

fn extract_context_entries(block: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut in_context = false;

    for line in block.lines() {
        if line_has_named_field(line, "Context Bundle") {
            in_context = true;
        } else if in_context && line_has_field(line) {
            break;
        }

        if in_context {
            collect_backtick_entries(line, &mut entries);
        }
    }

    let mut deduped = Vec::new();
    for entry in entries {
        add_unique(&mut deduped, &entry);
    }

    deduped
}

fn collect_backtick_entries(line: &str, entries: &mut Vec<String>) {
    let mut rest = line;
    loop {
        let start = match rest.find('`') {
            Some(index) => index,
            None => break,
        };
        let after_start = &rest[start + 1..];
        let end = match after_start.find('`') {
            Some(index) => index,
            None => break,
        };
        let value = &after_start[..end];
        entries.push(value.to_string());
        rest = &after_start[end + 1..];
    }
}

fn context_entry_exists(entry: &str, base_dir: Option<&Path>) -> bool {
    if entry.is_empty() {
        return false;
    }

    let path = Path::new(entry);
    if path.is_absolute() {
        if let Some(base) = base_dir {
            if !path.starts_with(base) {
                return false;
            }
        }
        return path.exists();
    }

    if let Some(base) = base_dir {
        return base.join(entry).exists();
    }

    false
}

fn context_display_path(entry: &str, base_dir: Option<&Path>) -> String {
    let path = Path::new(entry);
    if path.is_absolute() {
        if let Some(base) = base_dir {
            if path.starts_with(base) {
                if let Ok(rel) = path.strip_prefix(base) {
                    return rel.to_string_lossy().to_string();
                }
            }
        }
    }
    entry.to_string()
}

fn load_allowed_context(path: Option<&Path>) -> AllowedContext {
    let mut allowed = AllowedContext::default();
    let Some(path) = path else {
        return allowed;
    };
    if !path.is_file() {
        return allowed;
    }
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let trimmed = line.trim();
            allowed.insert(trimmed);
        }
    }
    allowed
}

fn pick_fallback_context(
    base_dir: Option<&Path>,
    allowed_context: &AllowedContext,
) -> Option<String> {
    let base_dir = base_dir?;
    for entry in &allowed_context.ordered {
        if base_dir.join(entry).exists() {
            return Some(entry.clone());
        }
    }

    let readme = base_dir.join("README.md");
    if readme.is_file() {
        return Some("README.md".to_string());
    }

    None
}

fn context_bundle_indent(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return None;
    }
    let after_dash = trimmed[1..].trim_start();
    if after_dash.starts_with("**Context Bundle**") {
        let indent_len = line.len() - trimmed.len();
        return Some(line[..indent_len].to_string());
    }
    None
}

fn is_open_questions_heading(lower: &str) -> bool {
    let trimmed = lower.trim_start();
    if !trimmed.starts_with("## ") {
        return false;
    }
    let rest = trimmed[3..].trim_start();
    rest.starts_with("open questions")
}

fn is_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return false;
    }
    let rest = trimmed.trim_start_matches('#');
    rest.starts_with(' ')
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::string::string_regex;
    use tempfile::tempdir;

    fn context_entry_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9_./-]{1,16}").unwrap()
    }

    fn whitespace_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[ \t]{0,4}").unwrap()
    }

    fn newline_strategy() -> impl Strategy<Value = String> {
        prop_oneof![Just("\n".to_string()), Just("\r\n".to_string())]
    }

    fn task_id_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Z0-9-]{1,8}").unwrap()
    }

    fn safe_line_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9][A-Za-z0-9 .,]{0,20}").unwrap()
    }

    fn noise_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9 ./,-]{0,6}").unwrap()
    }

    fn heading_title_strategy() -> impl Strategy<Value = String> {
        string_regex(r"[A-Za-z0-9][A-Za-z0-9 ]{0,12}").unwrap()
    }

    fn relative_path_strategy() -> impl Strategy<Value = String> {
        let segment = string_regex(r"[A-Za-z0-9_-]{1,8}").unwrap();
        prop::collection::vec(segment, 1..=3).prop_map(|segments| {
            let mut path = segments.join("/");
            path.push_str(".md");
            path
        })
    }

    fn open_questions_heading_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Open Questions".to_string()),
            Just("OPEN QUESTIONS".to_string()),
            Just("Open questions".to_string()),
            Just("open questions".to_string()),
        ]
    }

    fn dedupe_entries(entries: Vec<String>) -> Vec<String> {
        let mut deduped = Vec::new();
        for entry in entries {
            add_unique(&mut deduped, &entry);
        }
        deduped
    }

    fn allowed_context_from(entries: &[&str]) -> AllowedContext {
        let mut allowed = AllowedContext::default();
        for entry in entries {
            allowed.insert(entry);
        }
        allowed
    }

    #[test]
    fn prd_task_id_from_block_prefers_id_field() {
        let block = "### Task UX-4\n- **ID** UX-4\n- [ ] UX-4 Task\n";
        assert_eq!(prd_task_id_from_block(block).as_deref(), Some("UX-4"));
    }

    #[test]
    fn prd_next_task_id_returns_first_unchecked_block_id() {
        let temp = tempdir().unwrap();
        let prd_path = temp.path().join("PRD.md");
        fs::write(
            &prd_path,
            "# PRD\n\n### Task A-1\n- **ID** A-1\n- [x] Done\n---\n### Task B-2\n- **ID** B-2\n- [ ] Pending\n",
        )
        .unwrap();

        let next_id = prd_next_task_id(&prd_path);
        assert_eq!(next_id.as_deref(), Some("B-2"));
    }

    #[derive(Clone, Debug)]
    enum MissingField {
        Id,
        ContextBundle,
        DoD,
        Checklist,
        Dependencies,
    }

    fn missing_field_strategy() -> impl Strategy<Value = MissingField> {
        prop_oneof![
            Just(MissingField::Id),
            Just(MissingField::ContextBundle),
            Just(MissingField::DoD),
            Just(MissingField::Checklist),
            Just(MissingField::Dependencies),
        ]
    }

    #[test]
    fn prd_validate_file_accepts_valid() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let lib_dir = base.join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("context.txt"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-1\n- **ID** D-1\n- **Context Bundle** `lib/context.txt`\n- **DoD** Implement the feature.\n- **Checklist**\n  * Task implemented.\n- **Dependencies** None\n- [ ] D-1 Implement PRD validation\n",
        )
        .unwrap();

        assert!(prd_validate_file(&prd, false, None).is_ok());
    }

    #[test]
    fn prd_validate_file_rejects_empty_task_file() {
        let temp = tempdir().unwrap();
        let base = temp.path();

        let prd = base.join("prd.md");
        fs::write(&prd, "").unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Task file is empty")));
    }

    #[test]
    fn prd_validate_file_reports_missing_field() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let lib_dir = base.join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("context.txt"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-2\n- **ID** D-2\n- **Context Bundle** `lib/context.txt`\n- **Checklist**\n  * Missing DoD field.\n- **Dependencies** D-1\n- [ ] D-2 Missing DoD\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Missing required field: DoD")));
    }

    #[test]
    fn validate_task_block_reports_missing_required_field() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let block = "### Task V-1\n- **ID** V-1\n- **Context Bundle** `docs/context.md`\n- **Checklist**\n  * Missing DoD.\n- **Dependencies** None\n- [ ] V-1 Task\n";

        let errors = validate_task_block(block, Path::new("prd.md"), false, Some(base));

        assert!(errors
            .iter()
            .any(|line| line.contains("Missing required field: DoD")));
    }

    #[test]
    fn prd_validate_file_reports_missing_context_bundle_field() {
        let temp = tempdir().unwrap();
        let base = temp.path();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-2A\n- **ID** D-2A\n- **DoD** Provide details.\n- **Checklist**\n  * Missing context bundle.\n- **Dependencies** None\n- [ ] D-2A Missing context\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| { line.contains("Missing required field: Context Bundle") }));
    }

    #[test]
    fn prd_validate_file_rejects_multiple_unchecked() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let lib_dir = base.join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("context.txt"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-3\n- **ID** D-3\n- **Context Bundle** `lib/context.txt`\n- **DoD** Add strict PRD validation.\n- **Checklist**\n  * Validation added.\n- **Dependencies** D-2\n- [ ] D-3 Add strict PRD validation\n- [ ] D-3 Update error handling\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Multiple unchecked task lines")));
    }

    #[test]
    fn validate_task_block_reports_multiple_unchecked_lines() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let block = "### Task V-2\n- **ID** V-2\n- **Context Bundle** `docs/context.md`\n- **DoD** Validate output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] V-2 Task\n- [ ] V-2 Extra\n";

        let errors = validate_task_block(block, Path::new("prd.md"), false, Some(base));

        assert!(errors
            .iter()
            .any(|line| line.contains("Multiple unchecked task lines")));
    }

    #[test]
    fn validate_task_block_rejects_absolute_context_outside_repo_root() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.md");
        fs::write(&outside_file, "ok").unwrap();

        let block = format!(
            "### Task D-OUT\n- **ID** D-OUT\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Validate paths.\n- **Dependencies** None\n- [ ] D-OUT Task\n",
            outside_file.display()
        );

        let errors = validate_task_block(&block, Path::new("prd.md"), false, Some(base));

        assert!(errors
            .iter()
            .any(|line| line.contains("Context Bundle path outside repo")));
        assert!(!errors.iter().any(|line| line.contains("path not found")));
    }

    #[test]
    fn validate_task_block_accepts_absolute_context_inside_repo_root() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let inside = docs.join("inside.md");
        fs::write(&inside, "ok").unwrap();

        let block = format!(
            "### Task D-IN\n- **ID** D-IN\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Validate paths.\n- **Dependencies** None\n- [ ] D-IN Task\n",
            inside.display()
        );

        let errors = validate_task_block(&block, Path::new("prd.md"), false, Some(base));

        assert!(errors.is_empty());
    }

    #[test]
    fn validate_task_block_reports_missing_absolute_context_inside_repo_root() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let missing = docs.join("missing.md");

        let block = format!(
            "### Task D-MISS\n- **ID** D-MISS\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Validate paths.\n- **Dependencies** None\n- [ ] D-MISS Task\n",
            missing.display()
        );

        let errors = validate_task_block(&block, Path::new("prd.md"), false, Some(base));

        assert!(errors
            .iter()
            .any(|line| line.contains("Context Bundle path not found")));
        assert!(!errors
            .iter()
            .any(|line| line.contains("Context Bundle path outside repo")));
    }

    #[test]
    fn prd_validate_file_rejects_stray_checkbox() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let context_dir = base.join("context");
        fs::create_dir_all(&context_dir).unwrap();
        fs::write(context_dir.join("valid.txt"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n- [ ] Stray unchecked outside task block\n\n### Task D-4\n- **ID** D-4\n- **Context Bundle** `context/valid.txt`\n- **DoD** Fix validation.\n- **Checklist**\n  * Add guard.\n- **Dependencies** None\n- [ ] D-4 Add guard\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Unchecked task line outside task block")));
    }

    #[test]
    fn prd_validate_file_rejects_missing_context() {
        let temp = tempdir().unwrap();
        let base = temp.path();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-5\n- **ID** D-5\n- **Context Bundle** `missing/file.txt`\n- **DoD** Ensure context exists.\n- **Checklist**\n  * Validation fails.\n- **Dependencies** None\n- [ ] D-5 Missing context\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Context Bundle path not found")));
    }

    #[test]
    fn prd_validate_file_rejects_open_questions_section() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n## Open Questions\n- Should be removed\n\n### Task D-5A\n- **ID** D-5A\n- **Context Bundle** `docs/context.md`\n- **DoD** Reject open questions.\n- **Checklist**\n  * Validation fails.\n- **Dependencies** None\n- [ ] D-5A Reject open questions\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Open Questions section is not allowed")));
    }

    #[test]
    fn prd_validate_file_allows_missing_context_when_flagged() {
        let temp = tempdir().unwrap();
        let base = temp.path();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-6\n- **ID** D-6\n- **Context Bundle** `missing/ok.txt`\n- **DoD** Skip context validation.\n- **Checklist**\n  * Validation passes.\n- **Dependencies** None\n- [ ] D-6 Allow missing context\n",
        )
        .unwrap();

        assert!(prd_validate_file(&prd, true, None).is_ok());
    }

    #[test]
    fn prd_detect_stack_identifies_multiple_ids() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(
            base.join("package.json"),
            "{\n  \"name\": \"stack-detect\",\n  \"version\": \"1.0.0\"\n}\n",
        )
        .unwrap();
        fs::write(
            base.join("go.mod"),
            "module example.com/stack\n\n go 1.21\n",
        )
        .unwrap();

        let detection = prd_detect_stack(base);
        assert!(detection.ids.contains(&"Node.js".to_string()));
        assert!(detection.ids.contains(&"Go".to_string()));
    }

    #[test]
    fn prd_detect_stack_records_cargo_evidence() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(base.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        let detection = prd_detect_stack(base);

        assert!(detection.ids.contains(&"Rust".to_string()));
        assert!(detection.languages.contains(&"Rust".to_string()));
        assert!(detection.tools.contains(&"Cargo".to_string()));
        assert!(detection.evidence.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn prd_sanitize_generated_file_filters_open_questions_and_context() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(base.join("README.md"), "readme").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "Noise before heading\n# PRD\n\n## Open Questions\n- Should be removed\n\n### Task D-7\n- **ID** D-7\n- **Context Bundle** `missing.txt`, `docs/allowed.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-7 Keep first\n- [ ] D-7 Drop checkbox\n\n- [ ] Outside checkbox\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), Some(&allowed)).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(!sanitized.contains("Open Questions"));
        assert!(!sanitized.contains("Noise before heading"));
        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(sanitized.contains("- [ ] D-7 Keep first"));
        assert!(sanitized.contains("- D-7 Drop checkbox"));
        assert!(sanitized.contains("- Outside checkbox"));
    }

    #[test]
    fn prd_sanitize_generated_file_dedupes_context_and_strips_stray_unchecked() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("keep.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/keep.md\n").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n- [ ] Stray one\n- [ ] Stray two\n\n### Task D-7D\n- **ID** D-7D\n- **Context Bundle** `docs/keep.md`, `docs/keep.md`, `missing.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-7D Task\n\n- [ ] Another stray\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), Some(&allowed)).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(sanitized.contains("- **Context Bundle** `docs/keep.md`"));
        assert!(!sanitized.contains("missing.md"));
        assert_eq!(sanitized.matches("`docs/keep.md`").count(), 1);
        assert!(!sanitized.contains("- [ ] Stray one"));
        assert!(sanitized.contains("- Stray one"));
        assert!(sanitized.contains("- Stray two"));
        assert!(sanitized.contains("- Another stray"));
    }

    #[test]
    fn prd_sanitize_contents_removes_open_questions_section() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let contents = "# PRD\n\n## Open Questions\n- remove\n\n### Task D-14\n- **ID** D-14\n- **Context Bundle** `docs/context.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Done.\n- **Dependencies** None\n- [ ] D-14 Task\n";
        let allowed = allowed_context_from(&[]);
        let sanitized = prd_sanitize_contents(contents, Some(base), &allowed);

        assert!(!sanitized.contains("Open Questions"));
        assert!(!sanitized.contains("remove"));
        assert!(sanitized.contains("- [ ] D-14 Task"));
    }

    #[test]
    fn prd_sanitize_contents_cleans_stray_unchecked_outside_blocks() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let contents = "# PRD\n\n- [ ] Stray one\n\n### Task D-15\n- **ID** D-15\n- **Context Bundle** `docs/context.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Done.\n- **Dependencies** None\n- [ ] D-15 Task\n\n- [ ] Stray two\n";
        let sanitized = prd_sanitize_contents(contents, Some(base), &AllowedContext::default());

        assert!(!sanitized.contains("- [ ] Stray one"));
        assert!(sanitized.contains("- Stray one"));
        assert!(!sanitized.contains("- [ ] Stray two"));
        assert!(sanitized.contains("- Stray two"));
        assert!(sanitized.contains("- [ ] D-15 Task"));
    }

    #[test]
    fn prd_sanitize_contents_normalizes_absolute_context_paths() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let context = docs.join("alpha.md");
        fs::write(&context, "ok").unwrap();

        let contents = format!(
            "# PRD\n\n### Task D-16\n- **ID** D-16\n- **Context Bundle** `{}`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Done.\n- **Dependencies** None\n- [ ] D-16 Task\n",
            context.display()
        );
        let sanitized = prd_sanitize_contents(&contents, Some(base), &AllowedContext::default());

        assert!(sanitized.contains("- **Context Bundle** `docs/alpha.md`"));
        assert!(!sanitized.contains(context.to_string_lossy().as_ref()));
    }

    #[test]
    fn prd_sanitize_contents_normalizes_crlf() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let contents = "# PRD\r\n\r\n### Task D-17\r\n- **ID** D-17\r\n- **Context Bundle** `docs/context.md`\r\n- **DoD** Sanitize output.\r\n- **Checklist**\r\n  * Done.\r\n- **Dependencies** None\r\n- [ ] D-17 Task\r\n";
        let sanitized = prd_sanitize_contents(contents, Some(base), &AllowedContext::default());

        assert!(!sanitized.contains('\r'));
        assert!(sanitized.contains("- [ ] D-17 Task"));
    }

    #[test]
    fn prd_sanitize_generated_file_removes_open_questions_case_insensitive() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n## OPEN QUESTIONS\n- Should be removed\n\n### Task D-7X\n- **ID** D-7X\n- **Context Bundle** `docs/context.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-7X Task\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), None).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(!sanitized.contains("OPEN QUESTIONS"));
        assert!(!sanitized.contains("Should be removed"));
    }

    #[test]
    fn prd_sanitize_generated_file_removes_open_questions_until_end() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-13\n- **ID** D-13\n- **Context Bundle** `docs/context.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-13 Task\n\n## Open Questions\n- Should be removed\nTrailing text\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), None).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(sanitized.contains("- [ ] D-13 Task"));
        assert!(!sanitized.contains("Open Questions"));
        assert!(!sanitized.contains("Trailing text"));
    }

    #[test]
    fn prd_sanitize_generated_file_filters_context_by_allowed_list_and_relativizes() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(docs.join("blocked.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let allowed_abs = docs.join("allowed.md");
        let prd = base.join("prd.md");
        let contents = format!(
            "# PRD\n\n### Task D-7A\n- **ID** D-7A\n- **Context Bundle** `{}`, `docs/blocked.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-7A Task\n",
            allowed_abs.display()
        );
        fs::write(&prd, contents).unwrap();

        prd_sanitize_generated_file(&prd, Some(base), Some(&allowed)).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(!sanitized.contains("docs/blocked.md"));
        assert!(!sanitized.contains(base.to_string_lossy().as_ref()));
    }

    #[test]
    fn prd_sanitize_generated_file_falls_back_to_readme_without_allowed_context_file() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(base.join("README.md"), "readme").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-7B\n- **ID** D-7B\n- **Context Bundle** `missing.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-7B Task\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), None).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(sanitized.contains("- **Context Bundle** `README.md`"));
    }

    #[test]
    fn sanitize_task_block_keeps_relative_context_with_base_dir() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("keep.md"), "ok").unwrap();

        let block = "### Task X-2A\n- **ID** X-2A\n- **Context Bundle** `docs/keep.md`\n- **DoD** Confirm sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-2A Task\n";

        let sanitized = sanitize_task_block(block, Some(base), None);

        assert!(sanitized.contains("- **Context Bundle** `docs/keep.md`"));
        assert!(sanitized.contains("- [ ] X-2A Task"));
    }

    #[test]
    fn has_open_questions_section_detects_heading() {
        let contents = "# PRD\n\n## Open Questions\n- Remove these\n";
        assert!(has_open_questions_section(contents));
    }

    #[test]
    fn has_open_questions_section_ignores_non_matching_heading() {
        let contents = "# PRD\n\n## Open questions\n- Lowercase\n";
        assert!(!has_open_questions_section(contents));
    }

    #[test]
    fn validate_stray_unchecked_reports_line_number() {
        let contents = "# PRD\n\n### Task D-8\n- **ID** D-8\n- **Context Bundle** `README.md`\n- **DoD** Confirm stray validation.\n- **Checklist**\n  * Done.\n- **Dependencies** None\n- [ ] D-8 Task\n## Notes\n- [ ] Stray unchecked\n";
        let task_file = Path::new("prd.md");

        let errors = validate_stray_unchecked(contents, task_file).unwrap();
        assert!(errors.iter().any(
            |line| line.contains("Unchecked task line outside task block")
                && line.contains("line 12")
        ));
    }

    #[test]
    fn prd_validate_file_rejects_absolute_context_outside_repo() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.md");
        fs::write(&outside_file, "ok").unwrap();

        let prd = base.join("prd.md");
        let contents = format!(
            "# PRD\n\n### Task D-9\n- **ID** D-9\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Check absolute paths.\n- **Dependencies** None\n- [ ] D-9 Guard\n",
            outside_file.display()
        );
        fs::write(&prd, contents).unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Context Bundle path outside repo")));
    }

    #[test]
    fn prd_validate_file_rejects_absolute_context_missing_inside_repo() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let missing = base.join("missing.md");

        let prd = base.join("prd.md");
        let contents = format!(
            "# PRD\n\n### Task D-10\n- **ID** D-10\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Check absolute paths.\n- **Dependencies** None\n- [ ] D-10 Guard\n",
            missing.display()
        );
        fs::write(&prd, contents).unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Context Bundle path not found")));
        assert!(!err
            .messages
            .iter()
            .any(|line| line.contains("Context Bundle path outside repo")));
    }

    #[test]
    fn prd_validate_file_uses_base_dir_override_for_relative_context_entries() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let repo_root = base.join("repo");
        let docs = repo_root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let tasks_dir = repo_root.join("tasks");
        fs::create_dir_all(&tasks_dir).unwrap();
        let prd = tasks_dir.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-9A\n- **ID** D-9A\n- **Context Bundle** `docs/context.md`\n- **DoD** Guard context.\n- **Checklist**\n  * Check base override.\n- **Dependencies** None\n- [ ] D-9A Guard\n",
        )
        .unwrap();

        let err = prd_validate_file(&prd, false, None).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Context Bundle path not found")));
        assert!(prd_validate_file(&prd, false, Some(&repo_root)).is_ok());
    }

    #[test]
    fn prd_validate_file_accepts_absolute_context_with_base_dir_override() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let repo_root = base.join("repo");
        let docs = repo_root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let context = docs.join("context.md");
        fs::write(&context, "ok").unwrap();

        let prd_dir = base.join("tasks");
        fs::create_dir_all(&prd_dir).unwrap();
        let prd = prd_dir.join("prd.md");
        let contents = format!(
            "# PRD\n\n### Task D-10A\n- **ID** D-10A\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Check base override.\n- **Dependencies** None\n- [ ] D-10A Guard\n",
            context.display()
        );
        fs::write(&prd, contents).unwrap();

        assert!(prd_validate_file(&prd, false, Some(&repo_root)).is_ok());
    }

    #[test]
    fn prd_validate_file_rejects_absolute_context_outside_base_dir_override() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let repo_root = base.join("repo");
        fs::create_dir_all(&repo_root).unwrap();
        let outside = base.join("outside.md");
        fs::write(&outside, "ok").unwrap();

        let prd = base.join("prd.md");
        let contents = format!(
            "# PRD\n\n### Task D-10B\n- **ID** D-10B\n- **Context Bundle** `{}`\n- **DoD** Guard context.\n- **Checklist**\n  * Check base override.\n- **Dependencies** None\n- [ ] D-10B Guard\n",
            outside.display()
        );
        fs::write(&prd, contents).unwrap();

        let err = prd_validate_file(&prd, false, Some(&repo_root)).unwrap_err();
        assert!(err
            .messages
            .iter()
            .any(|line| line.contains("Context Bundle path outside repo")));
    }

    #[test]
    fn prd_sanitize_generated_file_falls_back_to_readme_when_allowed_context_empty() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(base.join("README.md"), "readme").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "\n").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-11\n- **ID** D-11\n- **Context Bundle** `missing.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-11 Task\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), Some(&allowed)).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(sanitized.contains("- **Context Bundle** `README.md`"));
    }

    #[test]
    fn prd_sanitize_generated_file_uses_allowed_fallback_when_context_filtered() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(docs.join("blocked.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let prd = base.join("prd.md");
        fs::write(
            &prd,
            "# PRD\n\n### Task D-12\n- **ID** D-12\n- **Context Bundle** `docs/blocked.md`\n- **DoD** Sanitize output.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] D-12 Task\n",
        )
        .unwrap();

        prd_sanitize_generated_file(&prd, Some(base), Some(&allowed)).unwrap();
        let sanitized = fs::read_to_string(&prd).unwrap();

        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(!sanitized.contains("docs/blocked.md"));
    }

    #[test]
    fn extract_context_entries_handles_multiline_context_bundle() {
        let block = "### Task X-1\n- **ID** X-1\n- **Context Bundle** `README.md`,\n  `docs/alpha.md`, `docs/beta.md`\n- **DoD** Confirm parsing.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-1 Task\n";

        let entries = extract_context_entries(block);

        assert_eq!(
            entries,
            vec![
                "README.md".to_string(),
                "docs/alpha.md".to_string(),
                "docs/beta.md".to_string()
            ]
        );
    }

    #[test]
    fn extract_context_entries_handles_mixed_backtick_entries() {
        let block = "### Task X-1B\n- **ID** X-1B\n- **Context Bundle** `README.md`, notes `docs/alpha.md`\n  and `docs/beta.md`, plus `docs/gamma.md`\n- **DoD** Confirm parsing.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-1B Task\n";

        let entries = extract_context_entries(block);

        assert_eq!(
            entries,
            vec![
                "README.md".to_string(),
                "docs/alpha.md".to_string(),
                "docs/beta.md".to_string(),
                "docs/gamma.md".to_string()
            ]
        );
    }

    #[test]
    fn extract_context_entries_stops_at_next_field() {
        let block = "### Task X-1A\n- **ID** X-1A\n- **Context Bundle** `README.md`,\n  `docs/alpha.md`\n  `docs/beta.md`\n- **DoD** Reference `docs/ignored.md`\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-1A Task\n";

        let entries = extract_context_entries(block);

        assert_eq!(
            entries,
            vec![
                "README.md".to_string(),
                "docs/alpha.md".to_string(),
                "docs/beta.md".to_string()
            ]
        );
        assert!(!entries.iter().any(|entry| entry == "docs/ignored.md"));
    }

    proptest! {
        #[test]
        fn prop_task_block_parsing_stable_across_whitespace_and_separators(
            newline in newline_strategy(),
            header_leading in whitespace_strategy(),
            separator_leading in whitespace_strategy(),
            separator_trailing in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..4),
            heading in heading_title_strategy(),
            use_heading in any::<bool>()
        ) {
            let header = format!("{}### Task {}", header_leading, id);
            let mut lines = vec!["# PRD".to_string(), header.clone()];

            let mut expected = header;
            for line in &body {
                lines.push(line.clone());
                expected.push('\n');
                expected.push_str(line);
            }

            let separator = if use_heading {
                format!("{}## {}", separator_leading, heading)
            } else {
                format!("{}---{}", separator_leading, separator_trailing)
            };
            lines.push(separator);

            let contents = lines.join(&newline);
            let blocks_out = task_blocks_from_contents(&contents);

            prop_assert_eq!(blocks_out.len(), 1);
            let block = &blocks_out[0];
            prop_assert_eq!(block, &expected);
            prop_assert!(!block.contains('\r'));
        }

        #[test]
        fn prop_validate_stray_unchecked_only_outside_task_blocks(
            newline in newline_strategy(),
            header_leading in whitespace_strategy(),
            separator_leading in whitespace_strategy(),
            separator_trailing in whitespace_strategy(),
            prefix_leading in whitespace_strategy(),
            suffix_leading in whitespace_strategy(),
            id in task_id_strategy(),
            body in prop::collection::vec(safe_line_strategy(), 0..3),
            heading in heading_title_strategy(),
            use_heading in any::<bool>(),
            prefix_unchecked in any::<bool>(),
            suffix_unchecked in any::<bool>()
        ) {
            let mut lines = vec!["# PRD".to_string()];
            if prefix_unchecked {
                lines.push(format!("{}- [ ] Outside before", prefix_leading));
            } else {
                lines.push(format!("{}Notes", prefix_leading));
            }

            lines.push(format!("{}### Task {}", header_leading, id));
            lines.push("- [ ] Inside".to_string());
            lines.extend(body.iter().cloned());

            let terminator = if use_heading {
                format!("{}## {}", separator_leading, heading)
            } else {
                format!("{}---{}", separator_leading, separator_trailing)
            };
            lines.push(terminator);

            if suffix_unchecked {
                lines.push(format!("{}- [ ] Outside after", suffix_leading));
            } else {
                lines.push(format!("{}Trailing", suffix_leading));
            }

            let contents = lines.join(&newline);
            let errors = validate_stray_unchecked(&contents, Path::new("prd.md"));
            let expected = (prefix_unchecked as usize) + (suffix_unchecked as usize);

            match errors {
                None => prop_assert_eq!(expected, 0),
                Some(messages) => {
                    prop_assert_eq!(messages.len(), expected);
                    prop_assert!(messages
                        .iter()
                        .all(|line| line.contains("Unchecked task line outside task block")));
                }
            }
        }

        #[test]
        fn prop_extract_context_entries_round_trip(
            entries in prop::collection::vec(context_entry_strategy(), 0..6)
        ) {
            let mut block = String::from("### Task P-1\n- **ID** P-1\n");
            if entries.is_empty() {
                block.push_str("- **Context Bundle**\n");
            } else {
                block.push_str("- **Context Bundle** ");
                for (index, entry) in entries.iter().enumerate() {
                    if index > 0 {
                        if index % 2 == 0 {
                            block.push('\n');
                            block.push_str("  ");
                        } else {
                            block.push_str(", ");
                        }
                    }
                    block.push('`');
                    block.push_str(entry);
                    block.push('`');
                }
                block.push('\n');
            }
            block.push_str("- **DoD** Example\n- **Checklist**\n  * Work\n- **Dependencies** None\n- [ ] P-1 Task\n");

            let extracted = extract_context_entries(&block);
            let expected = dedupe_entries(entries);

            prop_assert_eq!(extracted, expected);
        }

        #[test]
        fn prop_extract_context_entries_ignores_other_fields(
            entries in prop::collection::vec(context_entry_strategy(), 0..4),
            noise in prop::collection::vec(context_entry_strategy(), 0..4)
        ) {
            let mut block = String::from("### Task P-CTX\n- **ID** P-CTX\n");
            if entries.is_empty() {
                block.push_str("- **Context Bundle**\n");
            } else {
                block.push_str("- **Context Bundle** ");
                for (index, entry) in entries.iter().enumerate() {
                    if index > 0 {
                        block.push_str(", ");
                    }
                    block.push('`');
                    block.push_str(entry);
                    block.push('`');
                }
                block.push('\n');
            }
            if noise.is_empty() {
                block.push_str("- **DoD** Example\n");
            } else {
                block.push_str("- **DoD** Example ");
                for entry in &noise {
                    block.push('`');
                    block.push_str(entry);
                    block.push('`');
                    block.push(' ');
                }
                block.push('\n');
            }
            block.push_str("- **Checklist**\n  * Work `ignored.md`\n- **Dependencies** None\n- [ ] P-CTX Task\n");

            let extracted = extract_context_entries(&block);
            let expected = dedupe_entries(entries);

            prop_assert_eq!(extracted, expected);
        }

        #[test]
        fn prop_extract_context_entries_empty_without_context_bundle(
            noise in prop::collection::vec(context_entry_strategy(), 0..4)
        ) {
            let mut block = String::from("### Task P-NOCTX\n- **ID** P-NOCTX\n");
            if noise.is_empty() {
                block.push_str("- **DoD** Example\n");
            } else {
                block.push_str("- **DoD** Example ");
                for entry in &noise {
                    block.push('`');
                    block.push_str(entry);
                    block.push('`');
                    block.push(' ');
                }
                block.push('\n');
            }
            block.push_str("- **Checklist**\n  * Work `ignored.md`\n- **Dependencies** None\n- [ ] P-NOCTX Task\n");

            let extracted = extract_context_entries(&block);

            prop_assert!(extracted.is_empty());
        }

        #[test]
        fn prop_extract_context_entries_handles_noise_and_breaks(
            entries in prop::collection::vec(context_entry_strategy(), 1..6),
            prefixes in prop::collection::vec(noise_strategy(), 1..6),
            suffixes in prop::collection::vec(noise_strategy(), 1..6),
            breaks in prop::collection::vec(any::<bool>(), 1..6)
        ) {
            let mut block = String::from("### Task P-CTX-N\n- **ID** P-CTX-N\n- **Context Bundle** ");
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    if breaks[index % breaks.len()] {
                        block.push('\n');
                        block.push_str("  ");
                    } else {
                        block.push_str(", ");
                    }
                }
                let prefix = &prefixes[index % prefixes.len()];
                let suffix = &suffixes[index % suffixes.len()];
                block.push_str(prefix);
                block.push('`');
                block.push_str(entry);
                block.push('`');
                block.push_str(suffix);
            }
            block.push('\n');
            block.push_str("- **DoD** Example\n- **Checklist**\n  * Work\n- **Dependencies** None\n- [ ] P-CTX-N Task\n");

            let extracted = extract_context_entries(&block);
            let expected = dedupe_entries(entries);

            prop_assert_eq!(extracted, expected);
        }

        #[test]
        fn prop_extract_context_entries_collects_backticked_only_and_dedupes(
            entries in prop::collection::vec(context_entry_strategy(), 1..6),
            backticked in prop::collection::vec(any::<bool>(), 1..6),
            repeats in prop::collection::vec(any::<bool>(), 1..6),
            breaks in prop::collection::vec(any::<bool>(), 1..6)
        ) {
            let mut block = String::from("### Task P-MIX\n- **ID** P-MIX\n- **Context Bundle** ");
            let mut expected_raw = Vec::new();
            let mut first = true;

            let mut push_token = |token: &str, break_line: bool| {
                if !first {
                    if break_line {
                        block.push('\n');
                        block.push_str("  ");
                    } else {
                        block.push_str(", ");
                    }
                }
                block.push_str(token);
                first = false;
            };

            for (index, entry) in entries.iter().enumerate() {
                let is_backticked = backticked[index % backticked.len()];
                let repeat = repeats[index % repeats.len()];
                let break_line = breaks[index % breaks.len()];

                if is_backticked {
                    let token = format!("`{}`", entry);
                    push_token(token.as_str(), break_line);
                    expected_raw.push(entry.clone());

                    if repeat {
                        let repeat_break = breaks[(index + 1) % breaks.len()];
                        push_token(token.as_str(), repeat_break);
                        expected_raw.push(entry.clone());
                    }
                } else {
                    push_token(entry, break_line);
                    if repeat {
                        let repeat_break = breaks[(index + 1) % breaks.len()];
                        push_token(entry, repeat_break);
                    }
                }
            }
            block.push('\n');
            block.push_str("- **DoD** Example\n- **Checklist**\n  * Work\n- **Dependencies** None\n- [ ] P-MIX Task\n");

            let extracted = extract_context_entries(&block);
            let expected = dedupe_entries(expected_raw);

            prop_assert_eq!(extracted, expected);
        }

        #[test]
        fn prop_validate_task_block_unchecked_invariants(
            unchecked_count in 0usize..4
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            fs::write(base.join("context.md"), "ok").unwrap();

            let mut block = String::from(
                "### Task P-2\n- **ID** P-2\n- **Context Bundle** `context.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n"
            );
            for index in 0..unchecked_count {
                block.push_str(&format!("- [ ] P-2 Task {}\n", index));
            }

            let errors = validate_task_block(&block, Path::new("prd.md"), false, Some(base));
            let missing_unchecked = errors
                .iter()
                .any(|line| line.contains("Missing unchecked task line"));
            let multiple_unchecked = errors
                .iter()
                .any(|line| line.contains("Multiple unchecked task lines"));

            match unchecked_count {
                0 => {
                    prop_assert!(missing_unchecked);
                    prop_assert!(!multiple_unchecked);
                }
                1 => {
                    prop_assert!(!missing_unchecked);
                    prop_assert!(!multiple_unchecked);
                }
                _ => {
                    prop_assert!(!missing_unchecked);
                    prop_assert!(multiple_unchecked);
                }
            }
        }

        #[test]
        fn prop_validate_task_block_reports_missing_context_entries(
            entries in prop::collection::hash_set(relative_path_strategy(), 1..5),
            exists in prop::collection::vec(any::<bool>(), 1..5)
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let mut entries: Vec<String> = entries.into_iter().collect();
            entries.sort();

            for (index, entry) in entries.iter().enumerate() {
                if exists[index % exists.len()] {
                    let path = base.join(entry);
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    fs::write(&path, "ok").unwrap();
                }
            }

            let mut block = String::from(
                "### Task P-CHECK\n- **ID** P-CHECK\n- **Context Bundle** ",
            );
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    block.push_str(", ");
                }
                block.push('`');
                block.push_str(entry);
                block.push('`');
            }
            block.push_str(
                "\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] P-CHECK Task\n",
            );

            let errors = validate_task_block(&block, Path::new("prd.md"), false, Some(base));
            let missing_count = entries
                .iter()
                .enumerate()
                .filter(|(index, _)| !exists[index % exists.len()])
                .count();
            let missing_errors = errors
                .iter()
                .filter(|line| line.contains("Context Bundle path not found"))
                .count();

            prop_assert_eq!(missing_errors, missing_count);
        }

        #[test]
        fn prop_sanitize_task_block_single_unchecked_and_fallback_context(
            unchecked_count in 1usize..5,
            invalid_entries in prop::collection::vec(context_entry_strategy(), 0..4)
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("allowed.md"), "ok").unwrap();

            let allowed_entry = "docs/allowed.md";
            let allowed = base.join("allowed.txt");
            fs::write(&allowed, format!("{}\n", allowed_entry)).unwrap();

            prop_assume!(!invalid_entries.iter().any(|entry| entry == allowed_entry));

            let mut block = String::from("### Task P-3\n- **ID** P-3\n- **Context Bundle** ");
            if invalid_entries.is_empty() {
                block.push_str("`missing.md`\n");
            } else {
                for (index, entry) in invalid_entries.iter().enumerate() {
                    if index > 0 {
                        block.push_str(", ");
                    }
                    block.push('`');
                    block.push_str(entry);
                    block.push('`');
                }
                block.push('\n');
            }
            block.push_str("- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n");
            for index in 0..unchecked_count {
                block.push_str(&format!("- [ ] P-3 Task {}\n", index));
            }

            let sanitized = sanitize_task_block(&block, Some(base), Some(&allowed));
            let unchecked_lines = sanitized
                .lines()
                .filter(|line| is_unchecked_line(line))
                .count();

            prop_assert_eq!(unchecked_lines, 1);
            prop_assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        }

        #[test]
        fn prop_sanitize_task_block_preserves_first_unchecked_and_strips_rest(
            indents in prop::collection::vec(whitespace_strategy(), 1..5)
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("allowed.md"), "ok").unwrap();

            let mut block = String::from("### Task P-U\n- **ID** P-U\n- **Context Bundle** `docs/allowed.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n");
            for (index, indent) in indents.iter().enumerate() {
                block.push_str(&format!("{indent}- [ ] P-U Item-{index}\n"));
            }

            let sanitized = sanitize_task_block(&block, Some(base), None);
            let unchecked_lines = sanitized
                .lines()
                .filter(|line| is_unchecked_line(line))
                .count();

            prop_assert_eq!(unchecked_lines, 1);

            for (index, indent) in indents.iter().enumerate() {
                let token = format!("Item-{index}");
                let line = sanitized
                    .lines()
                    .find(|line| line.contains(&token))
                    .unwrap_or("");
                prop_assert!(!line.is_empty());
                if index == 0 {
                    prop_assert!(line.trim_start().starts_with("- [ ]"));
                } else {
                    prop_assert!(!line.trim_start().starts_with("- [ ]"));
                    prop_assert!(line.trim_start().starts_with("- "));
                }
                if !indent.is_empty() {
                    prop_assert!(line.starts_with(indent));
                }
            }
        }

        #[test]
        fn prop_sanitize_task_block_keeps_single_unchecked_line(
            unchecked_count in 1usize..6,
            indent in whitespace_strategy()
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("context.md"), "ok").unwrap();

            let mut block = String::from(
                "### Task P-ONE\n- **ID** P-ONE\n- **Context Bundle** `docs/context.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n",
            );
            for index in 0..unchecked_count {
                block.push_str(&format!("{indent}- [ ] P-ONE Task {index}\n"));
            }

            let sanitized = sanitize_task_block(&block, Some(base), None);
            let unchecked_lines = sanitized
                .lines()
                .filter(|line| is_unchecked_line(line))
                .count();

            prop_assert_eq!(unchecked_lines, 1);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn prop_prd_validate_file_reports_stray_unchecked_line(
            prefix in prop::collection::vec(safe_line_strategy(), 0..3)
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("context.md"), "ok").unwrap();

            let mut contents = String::from("# PRD\n\n");
            for line in prefix {
                contents.push_str(&line);
                contents.push('\n');
            }
            contents.push_str("- [ ] Stray unchecked\n\n");
            contents.push_str("### Task P-STRAY\n- **ID** P-STRAY\n- **Context Bundle** `docs/context.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] P-STRAY Task\n");

            let prd = base.join("prd.md");
            fs::write(&prd, contents).unwrap();

            let err = prd_validate_file(&prd, false, None).unwrap_err();
            prop_assert!(err
                .messages
                .iter()
                .any(|line| line.contains("Unchecked task line outside task block")));
        }

        #[test]
        fn prop_validate_task_block_reports_missing_required_fields(
            missing in missing_field_strategy()
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("context.md"), "ok").unwrap();

            let mut block = String::from("### Task P-MISS\n");
            if !matches!(missing, MissingField::Id) {
                block.push_str("- **ID** P-MISS\n");
            }
            if !matches!(missing, MissingField::ContextBundle) {
                block.push_str("- **Context Bundle** `docs/context.md`\n");
            }
            if !matches!(missing, MissingField::DoD) {
                block.push_str("- **DoD** Example.\n");
            }
            if !matches!(missing, MissingField::Checklist) {
                block.push_str("- **Checklist**\n  * Work.\n");
            }
            if !matches!(missing, MissingField::Dependencies) {
                block.push_str("- **Dependencies** None\n");
            }
            block.push_str("- [ ] P-MISS Task\n");

            let errors = validate_task_block(&block, Path::new("prd.md"), false, Some(base));
            let missing_label = match missing {
                MissingField::Id => "ID",
                MissingField::ContextBundle => "Context Bundle",
                MissingField::DoD => "DoD",
                MissingField::Checklist => "Checklist",
                MissingField::Dependencies => "Dependencies",
            };
            let expected = format!("Missing required field: {}", missing_label);

            prop_assert!(errors
                .iter()
                .any(|line| line.contains(&expected)));
        }

        #[test]
        fn prop_prd_sanitize_generated_file_removes_open_questions_section(
            heading in open_questions_heading_strategy(),
            questions in prop::collection::vec(safe_line_strategy(), 1..4)
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("context.md"), "ok").unwrap();

            let mut contents = String::from("# PRD\n\n");
            contents.push_str(&format!("## {}\n", heading));
            for line in &questions {
                contents.push_str(&format!("- {}\n", line));
            }
            contents.push_str("\n### Task P-OPEN\n- **ID** P-OPEN\n- **Context Bundle** `docs/context.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] P-OPEN Task\n");

            let prd = base.join("prd.md");
            fs::write(&prd, contents).unwrap();

            prd_sanitize_generated_file(&prd, Some(base), None).unwrap();
            let sanitized = fs::read_to_string(&prd).unwrap();

            prop_assert!(!sanitized.to_lowercase().contains("open questions"));
            for line in &questions {
                let question_line = format!("- {}", line);
                prop_assert!(!sanitized
                    .lines()
                    .any(|san_line| san_line.trim_end() == question_line));
            }
            prop_assert!(sanitized.contains("- [ ] P-OPEN Task"));
        }

        #[test]
        fn prop_prd_sanitize_generated_file_strips_stray_unchecked_lines(
            prefix in prop::collection::vec(
                (any::<bool>(), safe_line_strategy(), whitespace_strategy()),
                0..4
            ),
            suffix in prop::collection::vec(
                (any::<bool>(), safe_line_strategy(), whitespace_strategy()),
                0..4
            )
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let docs = base.join("docs");
            fs::create_dir_all(&docs).unwrap();
            fs::write(docs.join("context.md"), "ok").unwrap();

            let mut contents = String::from("# PRD\n\n");
            for (unchecked, line, indent) in &prefix {
                if *unchecked {
                    contents.push_str(&format!("{indent}- [ ] {line}\n"));
                } else {
                    contents.push_str(&format!("{indent}{line}\n"));
                }
            }
            contents.push('\n');
            contents.push_str("### Task P-STRAY-OUT\n- **ID** P-STRAY-OUT\n- **Context Bundle** `docs/context.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] P-STRAY-OUT Task\n---\n");
            for (unchecked, line, indent) in &suffix {
                if *unchecked {
                    contents.push_str(&format!("{indent}- [ ] {line}\n"));
                } else {
                    contents.push_str(&format!("{indent}{line}\n"));
                }
            }

            let prd = base.join("prd.md");
            fs::write(&prd, contents).unwrap();

            prd_sanitize_generated_file(&prd, Some(base), None).unwrap();
            let sanitized = fs::read_to_string(&prd).unwrap();

            let unchecked_lines = sanitized
                .lines()
                .filter(|line| is_unchecked_line(line))
                .count();
            prop_assert_eq!(unchecked_lines, 1);
        }

        #[test]
        fn prop_prd_sanitize_generated_file_filters_context_allowed_list_and_base_dir(
            entries in prop::collection::hash_set(relative_path_strategy(), 1..5),
            allowed in prop::collection::vec(any::<bool>(), 1..5),
            use_absolute in prop::collection::vec(any::<bool>(), 1..5),
            unchecked_count in 1usize..5
        ) {
            let temp = tempdir().unwrap();
            let base = temp.path();
            let mut entries: Vec<String> = entries.into_iter().collect();
            entries.sort();

            let mut allowed = allowed;
            if !entries
                .iter()
                .enumerate()
                .any(|(index, _)| allowed[index % allowed.len()])
            {
                allowed[0] = true;
            }
            let mut use_absolute = use_absolute;

            let mut first_allowed = None;
            for index in 0..entries.len() {
                if allowed[index % allowed.len()] {
                    first_allowed = Some(index);
                    break;
                }
            }
            if let Some(index) = first_allowed {
                let idx = index % use_absolute.len();
                if !use_absolute[idx] {
                    use_absolute[idx] = true;
                }
            }

            for entry in &entries {
                let path = base.join(entry);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&path, "ok").unwrap();
            }

            let allowed_path = base.join("allowed.txt");
            let mut allowed_lines = String::new();
            for (index, entry) in entries.iter().enumerate() {
                if allowed[index % allowed.len()] {
                    allowed_lines.push_str(entry);
                    allowed_lines.push('\n');
                }
            }
            fs::write(&allowed_path, allowed_lines).unwrap();

            let mut context_line = String::from("- **Context Bundle** ");
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    context_line.push_str(", ");
                }
                let use_abs = use_absolute[index % use_absolute.len()];
                if use_abs {
                    let abs = base.join(entry);
                    context_line.push_str(&format!("`{}`", abs.display()));
                } else {
                    context_line.push_str(&format!("`{}`", entry));
                }
            }

            let mut contents = String::from("# PRD\n\n### Task P-CONTEXT\n- **ID** P-CONTEXT\n");
            contents.push_str(&format!("{}\n", context_line));
            contents.push_str("- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n");
            for index in 0..unchecked_count {
                contents.push_str(&format!("- [ ] P-CONTEXT Task {}\n", index));
            }

            let prd_dir = base.join("tasks");
            fs::create_dir_all(&prd_dir).unwrap();
            let prd = prd_dir.join("prd.md");
            fs::write(&prd, contents).unwrap();

            prd_sanitize_generated_file(&prd, Some(base), Some(&allowed_path)).unwrap();
            let sanitized = fs::read_to_string(&prd).unwrap();

            prop_assert!(sanitized.contains("- **Context Bundle**"));
            prop_assert!(!sanitized.contains(base.to_string_lossy().as_ref()));

            for (index, entry) in entries.iter().enumerate() {
                let formatted = format!("`{}`", entry);
                if allowed[index % allowed.len()] {
                    prop_assert!(sanitized.contains(&formatted));
                } else {
                    prop_assert!(!sanitized.contains(&formatted));
                }
            }

            let unchecked_lines = sanitized
                .lines()
                .filter(|line| is_unchecked_line(line))
                .count();
            prop_assert_eq!(unchecked_lines, 1);
        }
    }

    #[test]
    fn context_bundle_indent_detects_indentation() {
        let indent = context_bundle_indent("  - **Context Bundle** `README.md`").unwrap();
        assert_eq!(indent, "  ");
        assert!(context_bundle_indent("- **DoD** Sample").is_none());
    }

    #[test]
    fn context_paths_resolve_inside_and_outside_base_dir() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let inside_rel = "docs/inside.md";
        let inside_abs = docs.join("inside.md");
        fs::write(&inside_abs, "ok").unwrap();

        let parent = base.parent().unwrap();
        let outside_abs = parent.join("outside.md");
        fs::write(&outside_abs, "ok").unwrap();
        let outside_rel = "../outside.md";

        assert!(context_entry_exists(inside_rel, Some(base)));
        assert!(context_entry_exists(
            inside_abs.to_string_lossy().as_ref(),
            Some(base)
        ));
        assert!(!context_entry_exists(
            outside_abs.to_string_lossy().as_ref(),
            Some(base)
        ));
        assert!(context_entry_exists(outside_rel, Some(base)));

        assert_eq!(context_display_path(inside_rel, Some(base)), inside_rel);
        assert_eq!(
            context_display_path(inside_abs.to_string_lossy().as_ref(), Some(base)),
            inside_rel
        );
        assert_eq!(
            context_display_path(outside_abs.to_string_lossy().as_ref(), Some(base)),
            outside_abs.to_string_lossy()
        );
        assert_eq!(context_display_path(outside_rel, Some(base)), outside_rel);
    }

    #[test]
    fn context_display_path_keeps_absolute_without_base_dir() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let abs = docs.join("abs.md");
        fs::write(&abs, "ok").unwrap();

        let display = context_display_path(abs.to_string_lossy().as_ref(), None);

        assert_eq!(display, abs.to_string_lossy());
    }

    #[test]
    fn context_entry_exists_requires_base_dir_for_relative_paths() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let rel = "docs/context.md";
        let abs = docs.join("context.md");
        fs::write(&abs, "ok").unwrap();

        assert!(context_entry_exists(rel, Some(base)));
        assert!(!context_entry_exists(rel, None));
        assert!(context_entry_exists(
            abs.to_string_lossy().as_ref(),
            Some(base)
        ));
    }

    #[test]
    fn canonicalize_for_compare_uses_canonical_parent_when_missing_file() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let missing = base.join("missing.md");

        let expected_parent = base.canonicalize().unwrap();
        let expected = expected_parent.join("missing.md");

        let compare = canonicalize_for_compare(&missing);

        assert_eq!(compare, expected);
    }

    #[test]
    fn canonicalize_for_compare_returns_original_when_parent_missing() {
        let temp = tempdir().unwrap();
        let missing = temp.path().join("missing").join("file.md");

        let compare = canonicalize_for_compare(&missing);

        assert_eq!(compare, missing);
    }

    #[test]
    fn resolve_base_dir_falls_back_to_override_when_canonicalize_fails() {
        let temp = tempdir().unwrap();
        let override_path = temp.path().join("missing-root");

        let resolved = resolve_base_dir(Path::new("prd.md"), Some(&override_path));

        assert_eq!(resolved, Some(override_path));
    }

    #[test]
    fn sanitize_task_block_rebuilds_context_and_dedupes_unchecked_lines() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(docs.join("blocked.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let block = "### Task X-2\n- **ID** X-2\n- **Context Bundle** `missing.md`,\n  `docs/blocked.md`\n- **DoD** Confirm sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-2 Keep\n- [ ] X-2 Drop\n";

        let sanitized = sanitize_task_block(block, Some(base), Some(&allowed));

        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(sanitized.contains("- [ ] X-2 Keep"));
        assert!(sanitized.contains("- X-2 Drop"));
        assert!(!sanitized.contains("- [ ] X-2 Drop"));
        assert!(!sanitized.contains("missing.md"));
        assert!(!sanitized.contains("docs/blocked.md"));
    }

    #[test]
    fn sanitize_task_block_filters_absolute_context_and_collapses_unchecked() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(docs.join("blocked.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let allowed_abs = docs.join("allowed.md");
        let block = format!(
            "### Task X-3\n- **ID** X-3\n- **Context Bundle** `{}`, `docs/blocked.md`\n- **DoD** Confirm sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-3 Keep\n- [ ] X-3 Drop\n",
            allowed_abs.display()
        );

        let sanitized = sanitize_task_block(&block, Some(base), Some(&allowed));

        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(!sanitized.contains("docs/blocked.md"));
        assert!(sanitized.contains("- [ ] X-3 Keep"));
        assert!(sanitized.contains("- X-3 Drop"));
        assert!(!sanitized.contains("- [ ] X-3 Drop"));
    }

    #[test]
    fn sanitize_task_block_keeps_absolute_context_inside_base_dir_without_allowed_list() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let inside = docs.join("keep.md");
        fs::write(&inside, "ok").unwrap();

        let block = format!(
            "### Task X-3A\n- **ID** X-3A\n- **Context Bundle** `{}`\n- **DoD** Confirm sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-3A Keep\n",
            inside.display()
        );

        let sanitized = sanitize_task_block(&block, Some(base), None);

        assert!(sanitized.contains("- **Context Bundle** `docs/keep.md`"));
        assert!(!sanitized.contains(base.to_string_lossy().as_ref()));
    }

    #[test]
    fn sanitize_task_block_filters_invalid_context_entries_without_allowed_list() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("keep.md"), "ok").unwrap();

        let outside = tempdir().unwrap();
        let outside_path = outside.path().join("outside.md");
        fs::write(&outside_path, "ok").unwrap();

        let block = format!(
            "### Task X-4\n- **ID** X-4\n- **Context Bundle** `docs/keep.md`, `missing.md`, `{}`\n- **DoD** Confirm sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-4 Task\n",
            outside_path.display()
        );

        let sanitized = sanitize_task_block(&block, Some(base), None);

        assert!(sanitized.contains("- **Context Bundle** `docs/keep.md`"));
        assert!(!sanitized.contains("missing.md"));
        assert!(!sanitized.contains(outside_path.to_string_lossy().as_ref()));
    }

    #[test]
    fn sanitize_task_block_falls_back_to_readme_when_context_invalid_without_allowed_list() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(base.join("README.md"), "readme").unwrap();

        let block = "### Task X-5\n- **ID** X-5\n- **Context Bundle** `missing.md`\n- **DoD** Confirm sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] X-5 Task\n";

        let sanitized = sanitize_task_block(block, Some(base), None);

        assert!(sanitized.contains("- **Context Bundle** `README.md`"));
        assert!(!sanitized.contains("missing.md"));
    }

    #[test]
    fn sanitize_task_block_rejects_absolute_context_not_in_allowed_list() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(docs.join("blocked.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let blocked_abs = docs.join("blocked.md");
        let block = format!(
            "### Task V-3A\n- **ID** V-3A\n- **Context Bundle** `{}`\n- **DoD** Validate sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] V-3A Task\n",
            blocked_abs.display()
        );

        let sanitized = sanitize_task_block(&block, Some(base), Some(&allowed));

        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(!sanitized.contains("blocked.md"));
        assert!(!sanitized.contains(base.to_string_lossy().as_ref()));
    }

    #[test]
    fn sanitize_task_block_removes_context_not_in_allowed_list_without_fallback() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("context.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/other.md\n").unwrap();

        let block = "### Task V-3\n- **ID** V-3\n- **Context Bundle** `docs/context.md`\n- **DoD** Validate sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] V-3 Task\n";

        let sanitized = sanitize_task_block(block, Some(base), Some(&allowed));

        assert!(sanitized.contains("- **Context Bundle**"));
        assert!(!sanitized.contains("docs/context.md"));
    }

    #[test]
    fn sanitize_task_block_falls_back_to_allowed_context_when_entries_invalid() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("allowed.md"), "ok").unwrap();
        fs::write(docs.join("blocked.md"), "ok").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/allowed.md\n").unwrap();

        let block = "### Task V-4\n- **ID** V-4\n- **Context Bundle** `docs/blocked.md`, `missing.md`\n- **DoD** Validate sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] V-4 Task\n";

        let sanitized = sanitize_task_block(block, Some(base), Some(&allowed));

        assert!(sanitized.contains("- **Context Bundle** `docs/allowed.md`"));
        assert!(!sanitized.contains("docs/blocked.md"));
        assert!(!sanitized.contains("missing.md"));
    }

    #[test]
    fn pick_fallback_context_skips_invalid_allowed_entries() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        let docs = base.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("keep.md"), "ok").unwrap();

        let allowed_path = base.join("allowed.txt");
        fs::write(&allowed_path, "docs/missing.md\n\ndocs/keep.md\n").unwrap();

        let allowed = load_allowed_context(Some(&allowed_path));
        let fallback = pick_fallback_context(Some(base), &allowed);

        assert_eq!(fallback, Some("docs/keep.md".to_string()));
    }

    #[test]
    fn sanitize_task_block_falls_back_to_readme_when_allowed_context_missing() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(base.join("README.md"), "readme").unwrap();

        let allowed = base.join("allowed.txt");
        fs::write(&allowed, "docs/missing.md\n").unwrap();

        let block = "### Task V-5\n- **ID** V-5\n- **Context Bundle** `docs/unknown.md`\n- **DoD** Validate sanitize.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ] V-5 Task\n";

        let sanitized = sanitize_task_block(block, Some(base), Some(&allowed));

        assert!(sanitized.contains("- **Context Bundle** `README.md`"));
        assert!(!sanitized.contains("docs/unknown.md"));
        assert!(!sanitized.contains("docs/missing.md"));
    }

    #[test]
    fn sanitize_task_block_normalizes_unchecked_line_spacing() {
        let temp = tempdir().unwrap();
        let base = temp.path();
        fs::write(base.join("README.md"), "readme").unwrap();

        let block = "### Task U-1\n- **ID** U-1\n- **Context Bundle** `README.md`\n- **DoD** Example.\n- **Checklist**\n  * Work.\n- **Dependencies** None\n- [ ]   U-1 First\n\t- [ ]\tU-1 Second\n";

        let sanitized = sanitize_task_block(block, Some(base), None);
        let unchecked_lines = sanitized
            .lines()
            .filter(|line| is_unchecked_line(line))
            .count();

        assert_eq!(unchecked_lines, 1);
        assert!(sanitized.contains("- [ ]   U-1 First"));
        let second_line = sanitized
            .lines()
            .find(|line| line.contains("U-1 Second"))
            .unwrap_or("");
        assert!(second_line.starts_with("\t- "));
        assert!(!second_line.contains("[ ]"));
    }

    #[test]
    fn remove_unchecked_checkbox_strips_marker_and_preserves_checked() {
        assert_eq!(
            remove_unchecked_checkbox("  - [ ] Do the thing"),
            "  - Do the thing"
        );
        assert_eq!(remove_unchecked_checkbox("- [x] Done"), "- [x] Done");
        assert_eq!(remove_unchecked_checkbox("- Task"), "- Task");
    }

    #[test]
    fn prd_format_stack_summary_includes_stack_focus_line() {
        let detection = StackDetection {
            ids: vec!["Rust".to_string(), "Node.js".to_string()],
            selected_ids: vec!["Rust".to_string()],
            ..StackDetection::default()
        };

        let summary = prd_format_stack_summary(&detection, 2);

        assert!(summary.contains("- Stack focus: Rust"));
    }
}
