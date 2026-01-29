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
        lookup_value(&self.merged, &normalized)
            .and_then(value_to_string)
            .is_some()
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
    use proptest::prelude::*;
    use proptest::string::string_regex;
    use std::fs;

    #[derive(Debug, Clone)]
    enum SeqItem {
        Text(String),
        Bool(bool),
        Number(i64),
        Null,
        Map,
        Sequence(Vec<SeqItem>),
    }

    impl SeqItem {
        fn expected_string(&self) -> String {
            match self {
                SeqItem::Text(text) => text.clone(),
                SeqItem::Bool(flag) => flag.to_string(),
                SeqItem::Number(number) => number.to_string(),
                SeqItem::Null => String::new(),
                SeqItem::Map => String::new(),
                SeqItem::Sequence(items) => items
                    .iter()
                    .map(SeqItem::expected_string)
                    .collect::<Vec<_>>()
                    .join(","),
            }
        }

        fn to_value(&self) -> Value {
            match self {
                SeqItem::Text(text) => Value::String(text.clone()),
                SeqItem::Bool(flag) => Value::Bool(*flag),
                SeqItem::Number(number) => Value::Number(serde_yaml::Number::from(*number)),
                SeqItem::Null => Value::Null,
                SeqItem::Map => {
                    let mut map = Mapping::new();
                    map.insert(
                        Value::String("key".to_string()),
                        Value::String("value".to_string()),
                    );
                    Value::Mapping(map)
                }
                SeqItem::Sequence(items) => {
                    Value::Sequence(items.iter().map(SeqItem::to_value).collect())
                }
            }
        }
    }

    fn seq_item_strategy() -> impl Strategy<Value = SeqItem> {
        let text = string_regex("[a-zA-Z0-9_-]{0,8}")
            .unwrap()
            .prop_map(SeqItem::Text);
        let number = (-999i64..=999).prop_map(SeqItem::Number);
        let leaf = prop_oneof![
            text,
            any::<bool>().prop_map(SeqItem::Bool),
            number,
            Just(SeqItem::Null),
            Just(SeqItem::Map),
        ];
        leaf.prop_recursive(2, 16, 4, |inner| {
            prop::collection::vec(inner, 0..4).prop_map(SeqItem::Sequence)
        })
    }

    fn key_segment_strategy() -> impl Strategy<Value = String> {
        let base = string_regex("[A-Za-z0-9_-]{1,8}").unwrap();
        let padding = string_regex("[ ]{0,2}").unwrap();
        (padding.clone(), base, padding)
            .prop_map(|(prefix, segment, suffix)| format!("{prefix}{segment}{suffix}"))
    }
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
            "GRALPH_DEFAULTS_MAX-ITERATIONS",
            "GRALPH_DEFAULTS_TASK_FILE",
            "GRALPH_DEFAULTS_COMPLETION_MARKER",
            "GRALPH_DEFAULTS_BACKEND",
            "GRALPH_DEFAULTS_MODEL",
            "GRALPH_DEFAULTS_AUTO_WORKTREE",
            "GRALPH_DEFAULTS_AUTO-WORKTREE",
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
        assert_eq!(
            normalize_key(" Logging.Log-Level "),
            Some("logging.log_level".to_string())
        );
        assert_eq!(normalize_key("  "), None);
    }

    #[test]
    fn normalize_key_handles_mixed_case_hyphenated_segments() {
        assert_eq!(
            normalize_key(" Defaults.Sub-Section.Log-Level "),
            Some("defaults.sub_section.log_level".to_string())
        );
    }

    #[test]
    fn normalize_key_preserves_empty_segments() {
        assert_eq!(
            normalize_key("defaults..backend"),
            Some("defaults..backend".to_string())
        );
        assert_eq!(
            normalize_key("defaults. .backend"),
            Some("defaults..backend".to_string())
        );
    }

    #[test]
    fn normalize_key_preserves_underscores_and_hyphen_mix() {
        assert_eq!(
            normalize_key(" Defaults.Auto_Worktree-Mode "),
            Some("defaults.auto_worktree_mode".to_string())
        );
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
    fn lookup_value_resolves_nested_mixed_case_and_hyphenated_keys() {
        let value: Value = serde_yaml::from_str(
            "Defaults:\n  Task-File: PRD.md\n  Sub-Section:\n    Mixed-Key: 12\n",
        )
        .unwrap();

        let task_file = lookup_value(&value, "defaults.task-file").and_then(Value::as_str);
        assert_eq!(task_file, Some("PRD.md"));

        let nested_value =
            lookup_value(&value, "DeFaUlts.Sub-Section.Mixed-Key").and_then(Value::as_i64);
        assert_eq!(nested_value, Some(12));
    }

    #[test]
    fn lookup_value_resolves_mixed_case_hyphenated_segments() {
        let value: Value =
            serde_yaml::from_str("Defaults:\n  Sub-Section:\n    Log-Level: warn\n").unwrap();

        let level = lookup_value(&value, "defaults.sub-section.log-level").and_then(Value::as_str);
        assert_eq!(level, Some("warn"));
    }

    #[test]
    fn lookup_value_resolves_mixed_case_hyphenated_key() {
        let value: Value = serde_yaml::from_str("Logging:\n  Log-Level: debug\n").unwrap();

        let level = lookup_value(&value, "logging.log-level").and_then(Value::as_str);
        assert_eq!(level, Some("debug"));
    }

    #[test]
    fn lookup_value_returns_none_for_empty_segments() {
        let value: Value = serde_yaml::from_str("defaults:\n  backend: claude\n").unwrap();

        assert!(lookup_value(&value, "defaults..backend").is_none());
        assert!(lookup_value(&value, "defaults. .backend").is_none());
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
        assert_eq!(
            config.get("defaults.auto-worktree").as_deref(),
            Some("true")
        );

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
    fn value_to_string_handles_tagged_mixed_sequences() {
        let sequence: Value = serde_yaml::from_str("[!tagged 2, null, ok]").unwrap();
        assert_eq!(value_to_string(&sequence).as_deref(), Some("2,,ok"));
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
    fn value_to_string_renders_sequence_with_mapping_entries() {
        let sequence: Value = serde_yaml::from_str("[one, {key: value}, null]").unwrap();
        assert_eq!(value_to_string(&sequence).as_deref(), Some("one,,"));
    }

    #[test]
    fn value_to_string_returns_none_for_map_values() {
        let mapping: Value = serde_yaml::from_str("key: value").unwrap();
        assert!(matches!(mapping, Value::Mapping(_)));
        assert!(value_to_string(&mapping).is_none());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]
        #[test]
        fn normalize_key_stability_matches_lookup(segments in prop::collection::vec(key_segment_strategy(), 1..4)) {
            let raw_key = segments.join(".");
            let normalized = normalize_key(&raw_key).expect("normalized key");
            let mut current = Value::String("value".to_string());
            for segment in normalized.split('.').rev() {
                let mut map = Mapping::new();
                map.insert(Value::String(segment.to_string()), current);
                current = Value::Mapping(map);
            }

            let raw_lookup = lookup_value(&current, &raw_key).and_then(Value::as_str);
            let normalized_lookup = lookup_value(&current, &normalized).and_then(Value::as_str);

            prop_assert_eq!(raw_lookup, Some("value"));
            prop_assert_eq!(normalized_lookup, Some("value"));
        }

        #[test]
        fn value_to_string_mixed_sequences_render_csv(items in prop::collection::vec(seq_item_strategy(), 0..6)) {
            let sequence = Value::Sequence(items.iter().map(SeqItem::to_value).collect());
            let expected = items
                .iter()
                .map(SeqItem::expected_string)
                .collect::<Vec<_>>()
                .join(",");
            let actual = value_to_string(&sequence);
            prop_assert_eq!(actual.as_deref(), Some(expected.as_str()));
        }
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
    fn merge_values_overrides_nested_mappings() {
        let base: Value = serde_yaml::from_str(
            "defaults:\n  backend: claude\n  nested:\n    level: info\n    enabled: true\n",
        )
        .unwrap();
        let overlay: Value =
            serde_yaml::from_str("defaults:\n  backend: gemini\n  nested:\n    enabled: false\n")
                .unwrap();

        let merged = merge_values(base, overlay);

        assert_eq!(
            lookup_value(&merged, "defaults.backend").and_then(Value::as_str),
            Some("gemini")
        );
        assert_eq!(
            lookup_value(&merged, "defaults.nested.level").and_then(Value::as_str),
            Some("info")
        );
        assert_eq!(
            lookup_value(&merged, "defaults.nested.enabled").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn env_override_precedes_project_config() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let default_path = root.join("default.yaml");
        let global_path = root.join("global.yaml");
        let project_dir = root.join("project");
        let project_path = project_dir.join(".gralph.yaml");

        write_file(&default_path, "defaults:\n  backend: claude\n");
        write_file(&global_path, "defaults:\n  backend: gemini\n");
        write_file(&project_path, "defaults:\n  backend: opencode\n");

        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);
        set_env("GRALPH_PROJECT_CONFIG_NAME", ".gralph.yaml");
        set_env("GRALPH_DEFAULTS_BACKEND", "codex");

        let config = Config::load(Some(&project_dir)).unwrap();
        assert_eq!(config.get("defaults.backend").as_deref(), Some("codex"));

        remove_env("GRALPH_DEFAULTS_BACKEND");
        remove_env("GRALPH_PROJECT_CONFIG_NAME");
        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
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
    fn legacy_env_override_precedes_compat_when_both_set() {
        let _guard = env_guard();
        set_env("GRALPH_MAX_ITERATIONS", "legacy");
        set_env("GRALPH_DEFAULTS_MAX-ITERATIONS", "compat");

        let value = resolve_env_override("defaults.max-iterations", "defaults.max_iterations");
        assert_eq!(value.as_deref(), Some("legacy"));

        remove_env("GRALPH_DEFAULTS_MAX-ITERATIONS");
        remove_env("GRALPH_MAX_ITERATIONS");
    }

    #[test]
    fn resolve_env_override_keeps_empty_values() {
        let _guard = env_guard();
        set_env("GRALPH_DEFAULTS_TASK_FILE", "");

        let value = resolve_env_override("defaults.task_file", "defaults.task_file");
        assert_eq!(value.as_deref(), Some(""));

        remove_env("GRALPH_DEFAULTS_TASK_FILE");
    }

    #[test]
    fn legacy_env_override_empty_value_precedes_normalized() {
        let _guard = env_guard();
        set_env("GRALPH_MAX_ITERATIONS", "");
        set_env("GRALPH_DEFAULTS_MAX_ITERATIONS", "55");

        let value = resolve_env_override("defaults.max_iterations", "defaults.max_iterations");
        assert_eq!(value.as_deref(), Some(""));

        remove_env("GRALPH_DEFAULTS_MAX_ITERATIONS");
        remove_env("GRALPH_MAX_ITERATIONS");
    }

    #[test]
    fn normalized_env_override_precedes_compat_without_legacy_alias() {
        let _guard = env_guard();
        set_env("GRALPH_DEFAULTS_AUTO_WORKTREE", "normalized");
        set_env("GRALPH_DEFAULTS_AUTO-WORKTREE", "compat");

        let value = resolve_env_override("defaults.auto-worktree", "defaults.auto_worktree");
        assert_eq!(value.as_deref(), Some("normalized"));

        remove_env("GRALPH_DEFAULTS_AUTO-WORKTREE");
        remove_env("GRALPH_DEFAULTS_AUTO_WORKTREE");
    }

    #[test]
    fn normalized_empty_override_precedes_compat_for_mixed_key() {
        let _guard = env_guard();
        set_env("GRALPH_DEFAULTS_AUTO_WORKTREE_MODE", "");
        set_env("GRALPH_DEFAULTS_AUTO-WORKTREE_MODE", "true");

        let value =
            resolve_env_override("defaults.auto-worktree_mode", "defaults.auto_worktree_mode");
        assert_eq!(value.as_deref(), Some(""));

        remove_env("GRALPH_DEFAULTS_AUTO-WORKTREE_MODE");
        remove_env("GRALPH_DEFAULTS_AUTO_WORKTREE_MODE");
    }

    #[test]
    fn normalized_empty_override_precedes_compat_for_hyphenated_key() {
        let _guard = env_guard();
        set_env("GRALPH_DEFAULTS_AUTO_WORKTREE", "");
        set_env("GRALPH_DEFAULTS_AUTO-WORKTREE", "true");

        let value = resolve_env_override("defaults.auto-worktree", "defaults.auto_worktree");
        assert_eq!(value.as_deref(), Some(""));

        remove_env("GRALPH_DEFAULTS_AUTO-WORKTREE");
        remove_env("GRALPH_DEFAULTS_AUTO_WORKTREE");
    }

    #[test]
    fn legacy_compat_env_override_keeps_empty_value() {
        let _guard = env_guard();
        set_env("GRALPH_DEFAULTS_AUTO-WORKTREE", "");

        let value = resolve_env_override("defaults.auto-worktree", "defaults.auto_worktree");
        assert_eq!(value.as_deref(), Some(""));

        remove_env("GRALPH_DEFAULTS_AUTO-WORKTREE");
    }

    #[test]
    fn env_override_precedence_handles_mixed_case_hyphenated_keys() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(&default_path, "defaults:\n  auto_worktree: false\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_DEFAULTS_AUTO_WORKTREE", "true");
        set_env("GRALPH_DEFAULTS_AUTO-WORKTREE", "false");

        let config = Config::load(None).unwrap();
        assert_eq!(
            config.get(" Defaults.Auto-Worktree ").as_deref(),
            Some("true")
        );

        remove_env("GRALPH_DEFAULTS_AUTO-WORKTREE");
        remove_env("GRALPH_DEFAULTS_AUTO_WORKTREE");
        remove_env("GRALPH_DEFAULT_CONFIG");
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
    fn get_normalizes_case_and_hyphenated_segments() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("missing-global.yaml");

        write_file(&default_path, "Logging:\n  Log-Level: INFO\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("logging.log-level").as_deref(), Some("INFO"));
        assert_eq!(config.get(" Logging.Log-Level ").as_deref(), Some("INFO"));

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
    fn config_paths_skips_project_when_project_dir_is_file() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("global.yaml");
        let project_file = temp.path().join("project-file");

        write_file(&default_path, "defaults:\n  max_iterations: 1\n");
        write_file(&global_path, "defaults:\n  backend: claude\n");
        write_file(&project_file, "not-a-dir");

        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let paths = config_paths(Some(&project_file));
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
        assert_eq!(
            paths,
            vec![
                default_path.clone(),
                global_path.clone(),
                project_path.clone()
            ]
        );

        remove_env("GRALPH_PROJECT_CONFIG_NAME");
        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn config_paths_order_default_global_project() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let default_path = root.join("default.yaml");
        let global_path = root.join("global.yaml");
        let project_dir = root.join("project");
        let project_path = project_dir.join(".gralph.yaml");

        write_file(&default_path, "defaults:\n  max_iterations: 1\n");
        write_file(&global_path, "defaults:\n  backend: claude\n");
        write_file(&project_path, "defaults:\n  backend: gemini\n");

        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let paths = config_paths(Some(&project_dir));
        assert_eq!(
            paths,
            vec![
                default_path.clone(),
                global_path.clone(),
                project_path.clone()
            ]
        );

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn config_paths_skip_missing_custom_project_config() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("global.yaml");
        let project_dir = temp.path().join("project");

        write_file(&default_path, "defaults:\n  max_iterations: 1\n");
        write_file(&global_path, "defaults:\n  backend: claude\n");
        fs::create_dir_all(&project_dir).unwrap();

        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);
        set_env("GRALPH_PROJECT_CONFIG_NAME", "custom.yaml");

        let paths = config_paths(Some(&project_dir));
        assert_eq!(paths, vec![default_path.clone(), global_path.clone()]);

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
        assert!(list
            .iter()
            .any(|(key, value)| key == "defaults.max_iterations" && value == "5"));
        assert!(list
            .iter()
            .any(|(key, value)| key == "logging.level" && value == "info"));

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn list_renders_sequences_and_null_values() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("missing-global.yaml");

        write_file(
            &default_path,
            "defaults:\n  flags:\n    - one\n    - 2\n  empty: null\nlogging:\n  level: info\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        let list = config.list();
        assert_eq!(
            list,
            vec![
                ("defaults.empty".to_string(), "".to_string()),
                ("defaults.flags".to_string(), "one,2".to_string()),
                ("logging.level".to_string(), "info".to_string()),
            ]
        );

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn list_renders_tagged_and_null_values() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("missing-global.yaml");

        write_file(
            &default_path,
            "defaults:\n  tagged: !tagged hello\n  flags:\n    - !tagged one\n    - null\n    - !tagged 2\nlogging:\n  level: info\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        let list = config.list();
        assert_eq!(
            list,
            vec![
                ("defaults.flags".to_string(), "one,,2".to_string()),
                ("defaults.tagged".to_string(), "hello".to_string()),
                ("logging.level".to_string(), "info".to_string()),
            ]
        );

        remove_env("GRALPH_GLOBAL_CONFIG");
        remove_env("GRALPH_DEFAULT_CONFIG");
    }

    #[test]
    fn list_renders_sequences_with_null_entries() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        let global_path = temp.path().join("missing-global.yaml");

        write_file(
            &default_path,
            "defaults:\n  flags:\n    - one\n    - null\n    - two\n  empty: null\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", &global_path);

        let config = Config::load(None).unwrap();
        let list = config.list();
        assert_eq!(
            list,
            vec![
                ("defaults.empty".to_string(), "".to_string()),
                ("defaults.flags".to_string(), "one,,two".to_string()),
            ]
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
    fn get_renders_sequences_with_null_entries() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(
            &default_path,
            "defaults:\n  flags:\n    - one\n    - null\n    - two\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);

        let config = Config::load(None).unwrap();
        assert_eq!(config.get("defaults.flags").as_deref(), Some("one,,two"));

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

    #[test]
    fn exists_returns_false_for_invalid_or_mapping_keys() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");

        write_file(
            &default_path,
            "defaults:\n  max_iterations: 5\nlogging:\n  level: info\n",
        );
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);

        let config = Config::load(None).unwrap();
        assert!(!config.exists(" "));
        assert!(!config.exists("defaults."));
        assert!(!config.exists("defaults"));

        remove_env("GRALPH_DEFAULT_CONFIG");
    }
}
