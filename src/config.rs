use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_yaml::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io { path, source } => {
                write!(f, "failed to read config at {}: {}", path.display(), source)
            }
            ConfigError::Parse { path, source } => {
                write!(
                    f,
                    "failed to parse config at {}: {}",
                    path.display(),
                    source
                )
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Io { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    merged: Value,
}

impl Config {
    pub fn load(project_dir: Option<&Path>) -> Result<Self, ConfigError> {
        let mut merged = Value::Mapping(Mapping::new());
        // Merge precedence: default < global < project (later overrides earlier).
        for path in config_paths(project_dir) {
            let value = read_yaml(&path)?;
            merged = merge_values(merged, value);
        }
        Ok(Self { merged })
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let normalized = normalize_key(key)?;
        if let Some(value) = resolve_env_override(key, &normalized) {
            return Some(value);
        }
        lookup_value(&self.merged, &normalized).and_then(value_to_string)
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }

    pub fn exists(&self, key: &str) -> bool {
        let Some(normalized) = normalize_key(key) else {
            return false;
        };
        if resolve_env_override(key, &normalized).is_some() {
            return true;
        }
        lookup_value(&self.merged, &normalized).is_some()
    }

    pub fn list(&self) -> Vec<(String, String)> {
        let mut entries: BTreeMap<String, String> = BTreeMap::new();
        flatten_value("", &self.merged, &mut entries);
        entries.into_iter().collect()
    }
}

fn config_paths(project_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let default_path = default_config_path();
    if default_path.exists() {
        paths.push(default_path);
    }

    let global_path = global_config_path();
    if global_path.exists() {
        paths.push(global_path);
    }

    if let Some(project_dir) = project_dir {
        if project_dir.is_dir() {
            let project_name = env::var("GRALPH_PROJECT_CONFIG_NAME")
                .unwrap_or_else(|_| ".gralph.yaml".to_string());
            let project_path = project_dir.join(project_name);
            if project_path.exists() {
                paths.push(project_path);
            }
        }
    }

    paths
}

fn default_config_path() -> PathBuf {
    if let Ok(path) = env::var("GRALPH_DEFAULT_CONFIG") {
        return PathBuf::from(path);
    }

    let config_dir = config_dir();
    let installed_default = config_dir.join("config").join("default.yaml");
    if installed_default.exists() {
        return installed_default;
    }

    let manifest_default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("default.yaml");
    if manifest_default.exists() {
        return manifest_default;
    }

    PathBuf::from("config/default.yaml")
}

fn global_config_path() -> PathBuf {
    if let Ok(path) = env::var("GRALPH_GLOBAL_CONFIG") {
        return PathBuf::from(path);
    }
    config_dir().join("config.yaml")
}

fn config_dir() -> PathBuf {
    if let Ok(path) = env::var("GRALPH_CONFIG_DIR") {
        return PathBuf::from(path);
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("gralph")
}

fn read_yaml(path: &Path) -> Result<Value, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_yaml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn merge_values(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Mapping(mut base_map), Value::Mapping(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                let merged = match base_map.remove(&key) {
                    Some(base_value) => merge_values(base_value, overlay_value),
                    None => overlay_value,
                };
                base_map.insert(key, merged);
            }
            Value::Mapping(base_map)
        }
        (_, overlay_value) => overlay_value,
    }
}

fn lookup_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in key.split('.') {
        match current {
            Value::Mapping(map) => {
                current = lookup_mapping_value(map, part)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn lookup_mapping_value<'a>(map: &'a Mapping, part: &str) -> Option<&'a Value> {
    let direct = Value::String(part.to_string());
    if let Some(value) = map.get(&direct) {
        return Some(value);
    }
    let normalized = normalize_segment(part);
    if normalized != part {
        let normalized_key = Value::String(normalized.clone());
        if let Some(value) = map.get(&normalized_key) {
            return Some(value);
        }
    }
    let mut matched = None;
    for (key, value) in map {
        let Some(text) = key.as_str() else {
            continue;
        };
        if normalize_segment(text) == normalized {
            matched = Some(value);
            break;
        }
    }
    matched
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Null => Some(String::new()),
        Value::Sequence(values) => {
            let mut rendered = Vec::new();
            for item in values {
                rendered.push(value_to_string(item).unwrap_or_default());
            }
            Some(rendered.join(","))
        }
        Value::Mapping(_) => None,
        Value::Tagged(tagged) => value_to_string(&tagged.value),
    }
}

fn flatten_value(prefix: &str, value: &Value, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                let key = match key.as_str() {
                    Some(text) => text,
                    None => continue,
                };
                let next_prefix = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_value(&next_prefix, value, out);
            }
        }
        _ => {
            if let Some(rendered) = value_to_string(value) {
                if !prefix.is_empty() {
                    out.insert(prefix.to_string(), rendered);
                }
            }
        }
    }
}

fn resolve_env_override(raw_key: &str, normalized_key: &str) -> Option<String> {
    // Env precedence: legacy aliases -> normalized overrides -> legacy hyphenated overrides.
    if let Some(value) = legacy_env_override(normalized_key) {
        return Some(value);
    }
    if let Some(value) = env_override(normalized_key) {
        return Some(value);
    }
    legacy_env_override_compat(raw_key)
}

fn env_override(key: &str) -> Option<String> {
    let env_key = format!("GRALPH_{}", key_to_env(key));
    env::var(env_key).ok()
}

fn legacy_env_override(key: &str) -> Option<String> {
    let legacy_key = match key {
        "defaults.max_iterations" => "GRALPH_MAX_ITERATIONS",
        "defaults.task_file" => "GRALPH_TASK_FILE",
        "defaults.completion_marker" => "GRALPH_COMPLETION_MARKER",
        "defaults.backend" => "GRALPH_BACKEND",
        "defaults.model" => "GRALPH_MODEL",
        _ => return None,
    };
    env::var(legacy_key).ok()
}

fn legacy_env_override_compat(key: &str) -> Option<String> {
    let env_key = format!("GRALPH_{}", key_to_env_legacy(key));
    let normalized_key = format!("GRALPH_{}", key_to_env(key));
    if env_key == normalized_key {
        return None;
    }
    env::var(env_key).ok()
}

fn key_to_env(key: &str) -> String {
    key.chars()
        .map(|ch| match ch {
            '.' | '-' => '_',
            _ => ch.to_ascii_uppercase(),
        })
        .collect()
}

fn key_to_env_legacy(key: &str) -> String {
    key.chars()
        .map(|ch| match ch {
            '.' => '_',
            _ => ch.to_ascii_uppercase(),
        })
        .collect()
}

fn normalize_key(key: &str) -> Option<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed
        .split('.')
        .map(normalize_segment)
        .collect::<Vec<_>>()
        .join(".");
    Some(normalized)
}

fn normalize_segment(segment: &str) -> String {
    segment
        .trim()
        .chars()
        .map(|ch| match ch {
            '-' => '_',
            _ => ch.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "GRALPH_DEFAULTS_MAX_ITERATIONS",
            "GRALPH_DEFAULTS_TASK_FILE",
            "GRALPH_DEFAULTS_COMPLETION_MARKER",
            "GRALPH_DEFAULTS_BACKEND",
            "GRALPH_DEFAULTS_MODEL",
            "GRALPH_DEFAULTS_AUTO_WORKTREE",
            "GRALPH_MAX_ITERATIONS",
            "GRALPH_TASK_FILE",
            "GRALPH_COMPLETION_MARKER",
            "GRALPH_BACKEND",
            "GRALPH_MODEL",
            "GRALPH_TEST_FLAGS",
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

    #[test]
    fn load_propagates_parse_error() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: [\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);

        let err = Config::load(None).unwrap_err();
        match err {
            ConfigError::Parse { path, .. } => {
                assert_eq!(path, default_path);
            }
            other => panic!("expected parse error, got {other:?}"),
        }

        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn normalize_key_trims_and_standardizes_segments() {
        assert_eq!(
            normalize_key(" Defaults.Max-Iterations "),
            Some("defaults.max_iterations".to_string())
        );
        assert_eq!(normalize_key("  "), None);
    }

    #[test]
    fn lookup_mapping_value_normalizes_case_and_hyphens() {
        let mut map = Mapping::new();
        map.insert(
            Value::String("max_iterations".to_string()),
            Value::String("10".to_string()),
        );
        map.insert(
            Value::String("Log-Level".to_string()),
            Value::String("info".to_string()),
        );

        let max_value = lookup_mapping_value(&map, "Max-Iterations").and_then(Value::as_str);
        assert_eq!(max_value, Some("10"));

        let log_value = lookup_mapping_value(&map, "log_level").and_then(Value::as_str);
        assert_eq!(log_value, Some("info"));
    }

    #[test]
    fn normalized_env_override_precedes_legacy_hyphenated() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 10\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_DEFAULTS_MAX_ITERATIONS", "42");
        set_env("GRALPH_DEFAULTS_MAX-ITERATIONS", "24");

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.max-iterations").as_deref(), Some("42"));

        remove_env("GRALPH_DEFAULTS_MAX-ITERATIONS");
        remove_env("GRALPH_DEFAULTS_MAX_ITERATIONS");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn default_config_path_prefers_env_override() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let override_path = temp.path().join("override.yaml");
        let config_dir = temp.path().join("config-root");
        let installed_default = config_dir.join("config").join("default.yaml");

        write_file(&override_path, "defaults:\n  backend: gemini\n");
        write_file(&installed_default, "defaults:\n  backend: claude\n");
        set_env("GRALPH_DEFAULT_CONFIG", &override_path);
        set_env("GRALPH_CONFIG_DIR", &config_dir);

        let resolved = default_config_path();
        assert_eq!(resolved, override_path);

        remove_env("GRALPH_CONFIG_DIR");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn default_config_path_prefers_installed_default() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("config-root");
        let installed_default = config_dir.join("config").join("default.yaml");

        write_file(&installed_default, "defaults:\n  backend: claude\n");
        set_env("GRALPH_CONFIG_DIR", &config_dir);

        let resolved = default_config_path();
        assert_eq!(resolved, installed_default);

        remove_env("GRALPH_CONFIG_DIR");
    }

    #[test]
    fn default_config_path_uses_manifest_default_when_installed_missing() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("config-root");
        let manifest_default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join("default.yaml");

        set_env("GRALPH_CONFIG_DIR", &config_dir);

        let resolved = default_config_path();
        assert_eq!(resolved, manifest_default);
        assert!(manifest_default.exists());

        remove_env("GRALPH_CONFIG_DIR");
    }

    #[test]
    fn legacy_hyphenated_env_override_is_resolved() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  auto_worktree: false\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_DEFAULTS_AUTO-WORKTREE", "true");

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.auto-worktree").as_deref(), Some("true"));

        remove_env("GRALPH_DEFAULTS_AUTO-WORKTREE");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn value_to_string_renders_sequences_and_tagged_values() {
        let sequence: Value = serde_yaml::from_str("[one, 2, true]").unwrap();
        assert_eq!(value_to_string(&sequence).as_deref(), Some("one,2,true"));

        let tagged: Value = serde_yaml::from_str("!tagged false").unwrap();
        assert_eq!(value_to_string(&tagged).as_deref(), Some("false"));
    }

    #[test]
    fn value_to_string_handles_null_and_mixed_sequence() {
        assert_eq!(value_to_string(&Value::Null).as_deref(), Some(""));

        let sequence: Value = serde_yaml::from_str("[null, hello, 5, false]").unwrap();
        assert_eq!(
            value_to_string(&sequence).as_deref(),
            Some(",hello,5,false")
        );
    }

    #[test]
    fn value_to_string_returns_none_for_map_values() {
        let mapping: Value = serde_yaml::from_str("key: value").unwrap();
        assert!(matches!(mapping, Value::Mapping(_)));
        assert!(value_to_string(&mapping).is_none());
    }

    #[test]
    fn merge_precedence_default_global_project() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let default_path = root.join("default.yaml");
        let global_path = root.join("global.yaml");
        let project_dir = root.join("project");
        let project_path = project_dir.join(".gralph.yaml");

        write_file(
            &default_path,
            "defaults:\n  max_iterations: 10\n  backend: claude\nlogging:\n  level: info\n",
        );
        write_file(
            &global_path,
            "defaults:\n  max_iterations: 20\nlogging:\n  level: debug\n",
        );
        write_file(&project_path, "defaults:\n  backend: gemini\n");

        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);
        set_env("GRALPH_PROJECT_CONFIG_NAME", ".gralph.yaml");

        let config = Config::load(Some(&project_dir)).unwrap();
        assert_eq!(config.get("defaults.max_iterations").as_deref(), Some("20"));
        assert_eq!(config.get("defaults.backend").as_deref(), Some("gemini"));
        assert_eq!(config.get("logging.level").as_deref(), Some("debug"));

        remove_env("GRALPH_DEFAULT_CONFIG");
        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_PROJECT_CONFIG_NAME");
    }

    #[test]
    fn env_override_wins() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 10\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_DEFAULTS_MAX_ITERATIONS", "42");

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.max_iterations").as_deref(), Some("42"));

        remove_env("GRALPH_DEFAULTS_MAX_ITERATIONS");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn legacy_env_override_wins() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 10\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_MAX_ITERATIONS", "77");

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.max_iterations").as_deref(), Some("77"));

        remove_env("GRALPH_MAX_ITERATIONS");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn legacy_env_override_takes_precedence_over_normalized() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 10\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_DEFAULTS_MAX_ITERATIONS", "42");
        set_env("GRALPH_MAX_ITERATIONS", "77");

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.max_iterations").as_deref(), Some("77"));

        remove_env("GRALPH_MAX_ITERATIONS");
        remove_env("GRALPH_DEFAULTS_MAX_ITERATIONS");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn legacy_env_override_precedes_normalized_and_compat() {
        let _guard = env_guard();
        set_env("GRALPH_MAX_ITERATIONS", "legacy");
        set_env("GRALPH_DEFAULTS_MAX_ITERATIONS", "normalized");
        set_env("GRALPH_DEFAULTS_MAX-ITERATIONS", "compat");

        let value = resolve_env_override("defaults.max-iterations", "defaults.max_iterations");
        assert_eq!(value.as_deref(), Some("legacy"));

        remove_env("GRALPH_DEFAULTS_MAX-ITERATIONS");
        remove_env("GRALPH_DEFAULTS_MAX_ITERATIONS");
        remove_env("GRALPH_MAX_ITERATIONS");
    }

    #[test]
    fn default_config_env_override_used() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("override.yaml");
        let global_path = temp.path().join("missing-global.yaml");

        write_file(&default_path, "defaults:\n  backend: gemini\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.backend").as_deref(), Some("gemini"));

        remove_env("GRALPH_DEFAULT_CONFIG");
        remove_env("GRALPH_GLOBAL_CONFIG");
    }

    #[test]
    fn key_normalization_resolves_hyphenated_keys() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("missing-global.yaml");

        write_file(&default_path, "defaults:\n  max-iterations: 12\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.max_iterations").as_deref(), Some("12"));
        assert!(config.exists("defaults.max-iterations"));

        remove_env("GRALPH_DEFAULT_CONFIG");
        remove_env("GRALPH_GLOBAL_CONFIG");
    }

    #[test]
    fn config_dir_env_sets_global_path() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("config-root");
        let global_path = config_dir.join("config.yaml");

        write_file(&global_path, "defaults:\n  max_iterations: 99\n");
        set_env("GRALPH_CONFIG_DIR", &config_dir);

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.max_iterations").as_deref(), Some("99"));

        remove_env("GRALPH_CONFIG_DIR");
    }

    #[test]
    fn config_paths_skips_missing_project_dir() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("global.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 1\n");
        write_file(&global_path, "defaults:\n  backend: claude\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let missing_project = temp.path().join("missing-project");
        let paths = config_paths(Some(&missing_project));
        assert_eq!(paths, vec![default_path.clone(), global_path.clone()]);

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn key_to_env_normalizes_dots_and_hyphens() {
        assert_eq!(
            key_to_env("defaults.max-Iterations"),
            "DEFAULTS_MAX_ITERATIONS"
        );
    }

    #[test]
    fn key_to_env_legacy_preserves_hyphens() {
        assert_eq!(
            key_to_env_legacy("defaults.max-Iterations"),
            "DEFAULTS_MAX-ITERATIONS"
        );
    }

    #[test]
    fn normalize_segment_trims_case_and_hyphens() {
        assert_eq!(normalize_segment(" Max-Iterations "), "max_iterations");
    }

    #[test]
    fn config_paths_include_project_with_custom_name() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("global.yaml");
        let project_dir = temp.path().join("project");
        let project_path = project_dir.join("custom.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 1\n");
        write_file(&global_path, "defaults:\n  backend: claude\n");
        write_file(&project_path, "defaults:\n  backend: gemini\n");

        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);
        set_env("GRALPH_PROJECT_CONFIG_NAME", "custom.yaml");

        let paths = config_paths(Some(&project_dir));
        assert_eq!(paths, vec![default_path.clone(), global_path.clone(), project_path.clone()]);

        remove_env("GRALPH_PROJECT_CONFIG_NAME");
        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn missing_files_fall_back_to_bundled_default() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("empty-config");

        remove_env("GRALPH_DEFAULT_CONFIG");
        remove_env("GRALPH_GLOBAL_CONFIG");
        set_env("GRALPH_CONFIG_DIR", &config_dir);

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.task_file").as_deref(), Some("PRD.md"));

        remove_env("GRALPH_CONFIG_DIR");
    }

    #[test]
    fn list_includes_nested_entries() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("global.yaml");

        write_file(
            &default_path,
            "defaults:\n  max_iterations: 5\nlogging:\n  level: info\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        let list = config.list();
        assert!(
            list.iter()
                .any(|(key, value)| key == "defaults.max_iterations" && value == "5")
        );
        assert!(
            list.iter()
                .any(|(key, value)| key == "logging.level" && value == "info")
        );

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn flatten_value_ignores_non_string_keys() {
        let mut map = Mapping::new();
        map.insert(
            Value::Number(serde_yaml::Number::from(1)),
            Value::String("one".to_string()),
        );
        map.insert(
            Value::String("valid".to_string()),
            Value::String("yes".to_string()),
        );

        let value = Value::Mapping(map);
        let mut entries = BTreeMap::new();
        flatten_value("", &value, &mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries.get("valid").map(String::as_str), Some("yes"));
    }

    #[test]
    fn arrays_flatten_to_csv() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(
            &default_path,
            "test:\n  flags:\n    - --headless\n    - --verbose\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);

        let config = Config::load(None).unwrap();
        assert_eq!(
            config.get("test.flags").as_deref(),
            Some("--headless,--verbose")
        );

        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn exists_returns_true_for_env_override() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 1\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_TEST_FLAGS", "1");

        let config = Config::load(None).unwrap();
        assert!(config.exists("test.flags"));

        remove_env("GRALPH_TEST_FLAGS");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }
}
