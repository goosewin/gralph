use fs2::FileExt;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum StateError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    LockTimeout {
        timeout: Duration,
    },
    InvalidSessionName,
    InvalidState(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::Io { path, source } => {
                write!(f, "state io error at {}: {}", path.display(), source)
            }
            StateError::Json { path, source } => {
                write!(f, "state json error at {}: {}", path.display(), source)
            }
            StateError::LockTimeout { timeout } => {
                write!(f, "failed to acquire state lock within {:?}", timeout)
            }
            StateError::InvalidSessionName => write!(f, "session name is required"),
            StateError::InvalidState(message) => write!(f, "invalid state: {}", message),
        }
    }
}

impl Error for StateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StateError::Io { source, .. } => Some(source),
            StateError::Json { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    Mark,
    Remove,
}

#[derive(Debug, Clone)]
pub struct StateStore {
    state_dir: PathBuf,
    state_file: PathBuf,
    lock_file: PathBuf,
    lock_timeout: Duration,
}

impl StateStore {
    pub fn new_from_env() -> Self {
        let state_dir = env::var("GRALPH_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_state_dir());
        let state_file = env::var("GRALPH_STATE_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| state_dir.join("state.json"));
        let lock_file = env::var("GRALPH_LOCK_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| state_dir.join("state.lock"));
        let lock_timeout = env::var("GRALPH_LOCK_TIMEOUT")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(10));

        Self::with_paths(state_dir, state_file, lock_file, lock_timeout)
    }

    pub fn with_paths(
        state_dir: PathBuf,
        state_file: PathBuf,
        lock_file: PathBuf,
        lock_timeout: Duration,
    ) -> Self {
        Self {
            state_dir,
            state_file,
            lock_file,
            lock_timeout,
        }
    }

    pub fn init_state(&self) -> Result<(), StateError> {
        if !self.state_dir.exists() {
            fs::create_dir_all(&self.state_dir).map_err(|source| StateError::Io {
                path: self.state_dir.clone(),
                source,
            })?;
        }

        if !self.state_file.exists() {
            let empty = empty_state();
            self.write_state(&empty)?;
        }

        match self.read_state() {
            Ok(_) => Ok(()),
            Err(StateError::Json { .. }) => {
                let empty = empty_state();
                self.write_state(&empty)
            }
            Err(error) => Err(error),
        }
    }

    pub fn get_session(&self, name: &str) -> Result<Option<Value>, StateError> {
        if name.trim().is_empty() {
            return Err(StateError::InvalidSessionName);
        }

        self.with_lock(|| {
            self.init_state()?;
            let state = self.read_state()?;
            Ok(state.sessions.get(name).cloned())
        })
    }

    pub fn set_session(&self, name: &str, fields: &[(&str, &str)]) -> Result<(), StateError> {
        if name.trim().is_empty() {
            return Err(StateError::InvalidSessionName);
        }

        self.with_lock(|| {
            self.init_state()?;
            let mut state = self.read_state()?;
            let mut session = state
                .sessions
                .remove(name)
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_else(Map::new);
            session.insert("name".to_string(), Value::String(name.to_string()));
            for (key, raw) in fields {
                if key.trim().is_empty() {
                    continue;
                }
                let value = parse_value(raw);
                session.insert((*key).to_string(), value);
            }
            state
                .sessions
                .insert(name.to_string(), Value::Object(session));
            self.write_state(&state)
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<Value>, StateError> {
        self.with_lock(|| {
            self.init_state()?;
            let state = self.read_state()?;
            let mut sessions = Vec::new();
            for (name, value) in state.sessions {
                let session = match value {
                    Value::Object(mut map) => {
                        map.insert("name".to_string(), Value::String(name));
                        Value::Object(map)
                    }
                    _ => {
                        let mut map = Map::new();
                        map.insert("name".to_string(), Value::String(name));
                        Value::Object(map)
                    }
                };
                sessions.push(session);
            }
            Ok(sessions)
        })
    }

    pub fn delete_session(&self, name: &str) -> Result<(), StateError> {
        if name.trim().is_empty() {
            return Err(StateError::InvalidSessionName);
        }

        self.with_lock(|| {
            self.init_state()?;
            let mut state = self.read_state()?;
            if state.sessions.remove(name).is_none() {
                return Err(StateError::InvalidState(format!(
                    "session '{}' not found",
                    name
                )));
            }
            self.write_state(&state)
        })
    }

    pub fn cleanup_stale(&self, mode: CleanupMode) -> Result<Vec<String>, StateError> {
        self.with_lock(|| {
            self.init_state()?;
            let mut state = self.read_state()?;
            let mut cleaned = Vec::new();
            let mut updates: BTreeMap<String, Value> = BTreeMap::new();

            for (name, value) in &state.sessions {
                let Some(map) = value.as_object() else {
                    continue;
                };
                let status = map.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status != "running" {
                    continue;
                }
                let pid = map.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
                if pid <= 0 {
                    continue;
                }
                if is_process_alive(pid) {
                    continue;
                }

                cleaned.push(name.clone());
                match mode {
                    CleanupMode::Remove => {
                        updates.insert(name.clone(), Value::Null);
                    }
                    CleanupMode::Mark => {
                        let mut session = map.clone();
                        session.insert("status".to_string(), Value::String("stale".to_string()));
                        updates.insert(name.clone(), Value::Object(session));
                    }
                }
            }

            for (name, value) in updates {
                if value.is_null() {
                    state.sessions.remove(&name);
                } else {
                    state.sessions.insert(name, value);
                }
            }

            if !cleaned.is_empty() {
                self.write_state(&state)?;
            }

            Ok(cleaned)
        })
    }

    fn with_lock<T>(&self, op: impl FnOnce() -> Result<T, StateError>) -> Result<T, StateError> {
        if !self.state_dir.exists() {
            fs::create_dir_all(&self.state_dir).map_err(|source| StateError::Io {
                path: self.state_dir.clone(),
                source,
            })?;
        }
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&self.lock_file)
            .map_err(|source| StateError::Io {
                path: self.lock_file.clone(),
                source,
            })?;
        acquire_lock(&lock_file, self.lock_timeout)?;
        let result = op();
        drop(lock_file);
        result
    }

    fn read_state(&self) -> Result<StateData, StateError> {
        let contents = fs::read_to_string(&self.state_file).map_err(|source| StateError::Io {
            path: self.state_file.clone(),
            source,
        })?;
        serde_json::from_str(&contents).map_err(|source| StateError::Json {
            path: self.state_file.clone(),
            source,
        })
    }

    fn write_state(&self, state: &StateData) -> Result<(), StateError> {
        let content = serde_json::to_string(state).map_err(|source| StateError::Json {
            path: self.state_file.clone(),
            source,
        })?;
        validate_state_content(&content)?;
        let tmp_file = self
            .state_file
            .with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&tmp_file, content).map_err(|source| StateError::Io {
            path: tmp_file.clone(),
            source,
        })?;
        fs::rename(&tmp_file, &self.state_file).map_err(|source| StateError::Io {
            path: self.state_file.clone(),
            source,
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StateData {
    sessions: BTreeMap<String, Value>,
}

fn empty_state() -> StateData {
    StateData {
        sessions: BTreeMap::new(),
    }
}

fn validate_state_content(content: &str) -> Result<(), StateError> {
    if content.trim().is_empty() {
        return Err(StateError::InvalidState(
            "refusing to write empty state content".to_string(),
        ));
    }
    Ok(())
}

fn default_state_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("gralph")
}

fn acquire_lock(file: &File, timeout: Duration) -> Result<(), StateError> {
    let start = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(err) if is_lock_contention(&err) => {
                if start.elapsed() >= timeout {
                    return Err(StateError::LockTimeout { timeout });
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                return Err(StateError::Io {
                    path: PathBuf::from("state.lock"),
                    source: err,
                });
            }
        }
    }
}

/// Check if an error indicates lock contention (file is locked by another process)
fn is_lock_contention(err: &std::io::Error) -> bool {
    // Unix returns WouldBlock
    if err.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    // Windows returns raw OS error 33 (ERROR_LOCK_VIOLATION)
    #[cfg(windows)]
    if err.raw_os_error() == Some(33) {
        return true;
    }
    false
}

fn parse_value(raw: &str) -> Value {
    if raw.is_empty() {
        return Value::String(String::new());
    }
    if raw == "true" {
        return Value::Bool(true);
    }
    if raw == "false" {
        return Value::Bool(false);
    }
    if raw.chars().all(|ch| ch.is_ascii_digit()) {
        if let Ok(number) = raw.parse::<i64>() {
            return Value::Number(number.into());
        }
    }
    Value::String(raw.to_string())
}

fn is_process_alive(pid: i64) -> bool {
    if pid <= 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, 0) };
        if result == 0 {
            return true;
        }
        let err = std::io::Error::last_os_error();
        return err.kind() == std::io::ErrorKind::PermissionDenied;
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    #[cfg(unix)]
    use std::os::unix::io::FromRawFd;
    use std::path::Path;
    use std::process::Command;
    use std::sync::{Arc, Mutex};

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

    fn store_for_test(dir: &Path, timeout: Duration) -> StateStore {
        let state_dir = dir.join("state");
        let state_file = state_dir.join("state.json");
        let lock_file = state_dir.join("state.lock");
        StateStore::with_paths(state_dir, state_file, lock_file, timeout)
    }

    #[test]
    fn lock_times_out_when_held() {
        let temp = tempfile::tempdir().unwrap();
        // Short timeout (100ms) so test completes quickly
        let store = Arc::new(store_for_test(temp.path(), Duration::from_millis(100)));
        store.init_state().unwrap();

        let blocker = Arc::clone(&store);
        let handle = thread::spawn(move || {
            blocker
                .with_lock(|| {
                    // Hold lock much longer than timeout (2s vs 100ms)
                    thread::sleep(Duration::from_secs(2));
                    Ok(())
                })
                .unwrap();
        });

        // Wait for blocker thread to acquire lock
        thread::sleep(Duration::from_millis(100));
        let result = store.with_lock(|| Ok(()));
        assert!(matches!(result, Err(StateError::LockTimeout { .. })));
        handle.join().unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn acquire_lock_maps_non_contention_errors() {
        let mut fds = [0; 2];
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(result, 0);
        let read_fd = fds[0];
        let write_fd = fds[1];
        unsafe {
            libc::close(write_fd);
        }

        let file = unsafe { File::from_raw_fd(read_fd) };
        let err = acquire_lock(&file, Duration::from_millis(10)).unwrap_err();
        match err {
            StateError::Io { path, .. } => {
                assert_eq!(path, PathBuf::from("state.lock"));
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn atomic_write_persists_state() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store
            .set_session("alpha", &[("status", "running"), ("pid", "123")])
            .unwrap();

        let contents = fs::read_to_string(store.state_file).unwrap();
        assert!(!contents.trim().is_empty());
        let parsed: StateData = serde_json::from_str(&contents).unwrap();
        let session = parsed.sessions.get("alpha").unwrap();
        assert_eq!(
            session.get("status").and_then(|v| v.as_str()),
            Some("running")
        );
        assert_eq!(session.get("pid").and_then(|v| v.as_i64()), Some(123));
    }

    #[test]
    fn read_state_propagates_io_error() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        fs::create_dir_all(&store.state_dir).unwrap();
        fs::create_dir_all(&store.state_file).unwrap();

        let err = store.read_state().unwrap_err();
        match err {
            StateError::Io { path, .. } => {
                assert_eq!(path, store.state_file);
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn read_state_propagates_json_error() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        fs::create_dir_all(&store.state_dir).unwrap();
        fs::write(&store.state_file, "{not json").unwrap();

        let err = store.read_state().unwrap_err();
        match err {
            StateError::Json { path, .. } => {
                assert_eq!(path, store.state_file);
            }
            other => panic!("expected Json, got {other:?}"),
        }
    }

    #[test]
    fn write_state_propagates_rename_error() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        fs::create_dir_all(&store.state_dir).unwrap();
        fs::create_dir_all(&store.state_file).unwrap();

        let err = store.write_state(&empty_state()).unwrap_err();
        match err {
            StateError::Io { path, .. } => {
                assert_eq!(path, store.state_file);
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn set_get_list_and_delete_session_flow() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        store
            .set_session("alpha", &[("status", "running"), ("pid", "123")])
            .unwrap();
        let session = store.get_session("alpha").unwrap().unwrap();
        assert_eq!(
            session.get("status").and_then(|v| v.as_str()),
            Some("running")
        );
        assert_eq!(session.get("pid").and_then(|v| v.as_i64()), Some(123));

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        store.delete_session("alpha").unwrap();
        assert!(store.get_session("alpha").unwrap().is_none());
    }

    #[test]
    fn set_session_skips_empty_field_keys() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));

        store
            .set_session(
                "alpha",
                &[("", "ignore"), (" ", "skip"), ("status", "running")],
            )
            .unwrap();

        let session = store.get_session("alpha").unwrap().unwrap();
        let map = session.as_object().unwrap();
        assert_eq!(map.get("status").and_then(|v| v.as_str()), Some("running"));
        assert!(!map.contains_key(""));
        assert!(!map.contains_key(" "));
    }

    #[test]
    fn list_sessions_handles_non_object_values() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        let mut sessions = BTreeMap::new();
        sessions.insert("alpha".to_string(), Value::String("oops".to_string()));
        sessions.insert("beta".to_string(), Value::Number(5.into()));
        sessions.insert(
            "gamma".to_string(),
            Value::Object(Map::from_iter([(
                "status".to_string(),
                Value::String("running".to_string()),
            )])),
        );
        let state = StateData { sessions };
        store.write_state(&state).unwrap();

        let listed = store.list_sessions().unwrap();
        assert_eq!(listed.len(), 3);

        let mut alpha_seen = false;
        let mut beta_seen = false;
        let mut gamma_seen = false;

        for session in listed {
            let map = session.as_object().unwrap();
            let name = map.get("name").and_then(|v| v.as_str()).unwrap();
            match name {
                "alpha" => {
                    alpha_seen = true;
                    assert_eq!(map.len(), 1);
                }
                "beta" => {
                    beta_seen = true;
                    assert_eq!(map.len(), 1);
                }
                "gamma" => {
                    gamma_seen = true;
                    assert_eq!(map.get("status").and_then(|v| v.as_str()), Some("running"));
                }
                other => panic!("unexpected session {other}"),
            }
        }

        assert!(alpha_seen && beta_seen && gamma_seen);
    }

    #[test]
    fn cleanup_stale_marks_dead_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        store
            .set_session("stale-session", &[("status", "running"), ("pid", "999999")])
            .unwrap();

        let cleaned = store.cleanup_stale(CleanupMode::Mark).unwrap();
        assert_eq!(cleaned, vec!["stale-session".to_string()]);

        let session = store.get_session("stale-session").unwrap().unwrap();
        assert_eq!(
            session.get("status").and_then(|v| v.as_str()),
            Some("stale")
        );
    }

    #[test]
    #[cfg(unix)]
    fn cleanup_stale_skips_live_pid() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        let mut child = Command::new("sleep").arg("2").spawn().unwrap();
        let pid = child.id() as i64;
        let pid_value = pid.to_string();
        store
            .set_session(
                "alive-session",
                &[("status", "running"), ("pid", &pid_value)],
            )
            .unwrap();

        assert!(is_process_alive(pid));
        let cleaned = store.cleanup_stale(CleanupMode::Mark).unwrap();
        assert!(cleaned.is_empty());

        let session = store.get_session("alive-session").unwrap().unwrap();
        assert_eq!(
            session.get("status").and_then(|v| v.as_str()),
            Some("running")
        );

        let _ = child.wait();
    }

    #[test]
    fn cleanup_stale_removes_dead_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        store
            .set_session("stale-remove", &[("status", "running"), ("pid", "999999")])
            .unwrap();

        let cleaned = store.cleanup_stale(CleanupMode::Remove).unwrap();
        assert_eq!(cleaned, vec!["stale-remove".to_string()]);
        assert!(store.get_session("stale-remove").unwrap().is_none());
    }

    #[test]
    fn cleanup_stale_skips_non_running_or_invalid_pid() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        store
            .set_session("idle", &[("status", "complete"), ("pid", "999999")])
            .unwrap();
        store
            .set_session("missing-pid", &[("status", "running"), ("pid", "0")])
            .unwrap();

        let cleaned = store.cleanup_stale(CleanupMode::Remove).unwrap();
        assert!(cleaned.is_empty());

        let idle = store.get_session("idle").unwrap().unwrap();
        assert_eq!(idle.get("status").and_then(|v| v.as_str()), Some("complete"));
        assert_eq!(idle.get("pid").and_then(|v| v.as_i64()), Some(999999));

        let missing_pid = store.get_session("missing-pid").unwrap().unwrap();
        assert_eq!(missing_pid.get("status").and_then(|v| v.as_str()), Some("running"));
        assert_eq!(missing_pid.get("pid").and_then(|v| v.as_i64()), Some(0));
    }

    #[test]
    fn cleanup_stale_skips_non_object_and_missing_fields() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        let mut sessions = BTreeMap::new();
        sessions.insert("stringy".to_string(), Value::String("oops".to_string()));
        sessions.insert("numbery".to_string(), Value::Number(5.into()));
        sessions.insert("missing-fields".to_string(), Value::Object(Map::new()));
        sessions.insert(
            "missing-pid".to_string(),
            Value::Object(Map::from_iter([(
                "status".to_string(),
                Value::String("running".to_string()),
            )])),
        );
        sessions.insert(
            "missing-status".to_string(),
            Value::Object(Map::from_iter([("pid".to_string(), Value::Number(12.into()))])),
        );
        let state = StateData { sessions };
        store.write_state(&state).unwrap();

        let cleaned = store.cleanup_stale(CleanupMode::Mark).unwrap();
        assert!(cleaned.is_empty());

        let reloaded = store.read_state().unwrap();
        assert!(matches!(
            reloaded.sessions.get("stringy"),
            Some(Value::String(_))
        ));
        assert!(matches!(
            reloaded.sessions.get("numbery"),
            Some(Value::Number(_))
        ));
        let missing_fields = reloaded.sessions.get("missing-fields").unwrap();
        assert!(missing_fields.as_object().unwrap().is_empty());
        let missing_pid = reloaded.sessions.get("missing-pid").unwrap();
        assert_eq!(
            missing_pid
                .get("status")
                .and_then(|value| value.as_str()),
            Some("running")
        );
        let missing_status = reloaded.sessions.get("missing-status").unwrap();
        assert_eq!(missing_status.get("pid").and_then(|value| value.as_i64()), Some(12));
    }

    #[test]
    fn invalid_session_names_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));

        assert!(matches!(
            store.get_session(" "),
            Err(StateError::InvalidSessionName)
        ));
        assert!(matches!(
            store.set_session(" ", &[("status", "running")]),
            Err(StateError::InvalidSessionName)
        ));
        assert!(matches!(
            store.delete_session("\t"),
            Err(StateError::InvalidSessionName)
        ));
    }

    #[test]
    fn delete_missing_session_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));
        store.init_state().unwrap();

        let err = store.delete_session("missing").unwrap_err();
        match err {
            StateError::InvalidState(message) => {
                assert!(message.contains("missing"));
            }
            other => panic!("expected InvalidState, got {other:?}"),
        }
    }

    #[test]
    fn parse_value_handles_bool_and_numeric() {
        assert_eq!(parse_value(""), Value::String(String::new()));
        assert_eq!(parse_value("true"), Value::Bool(true));
        assert_eq!(parse_value("false"), Value::Bool(false));
        assert_eq!(parse_value("42").as_i64(), Some(42));
        assert_eq!(parse_value("007").as_i64(), Some(7));
        assert_eq!(parse_value("12ab"), Value::String("12ab".to_string()));
    }

    #[test]
    fn parse_value_handles_leading_zeros_and_mixed_input() {
        assert_eq!(parse_value("00012").as_i64(), Some(12));
        assert_eq!(parse_value("007bond"), Value::String("007bond".to_string()));
    }

    #[test]
    fn parse_value_handles_negative_and_mixed_strings() {
        assert_eq!(parse_value("-5"), Value::String("-5".to_string()));
        assert_eq!(parse_value("12-3"), Value::String("12-3".to_string()));
    }

    #[test]
    fn parse_value_handles_negative_numbers_and_alphanumeric() {
        assert_eq!(parse_value("-42"), Value::String("-42".to_string()));
        assert_eq!(parse_value("42x"), Value::String("42x".to_string()));
        assert_eq!(parse_value("x42"), Value::String("x42".to_string()));
    }

    #[test]
    fn default_state_dir_uses_home_env() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let original = env::var_os("HOME");
        set_env("HOME", temp.path());

        let resolved = default_state_dir();
        assert_eq!(resolved, temp.path().join(".config").join("gralph"));

        match original {
            Some(value) => set_env("HOME", value),
            None => remove_env("HOME"),
        }
    }

    #[test]
    fn default_state_dir_falls_back_when_home_missing() {
        let _guard = env_guard();
        let original = env::var_os("HOME");
        remove_env("HOME");

        let resolved = default_state_dir();
        assert_eq!(resolved, PathBuf::from(".").join(".config").join("gralph"));

        match original {
            Some(value) => set_env("HOME", value),
            None => remove_env("HOME"),
        }
    }

    #[test]
    fn init_state_recovers_from_corrupted_json() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));

        fs::create_dir_all(&store.state_dir).unwrap();
        fs::write(&store.state_file, "{not valid json").unwrap();

        store.init_state().unwrap();
        let state = store.read_state().unwrap();
        assert!(state.sessions.is_empty());
    }

    #[test]
    fn init_state_creates_missing_state_file() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_secs(1));

        store.init_state().unwrap();
        assert!(store.state_file.exists());
        let state = store.read_state().unwrap();
        assert!(state.sessions.is_empty());
    }

    #[test]
    fn lock_path_directory_returns_io_error() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path(), Duration::from_millis(100));
        fs::create_dir_all(&store.state_dir).unwrap();
        fs::create_dir_all(&store.lock_file).unwrap();

        let err = store.get_session("alpha").unwrap_err();
        match err {
            StateError::Io { path, .. } => {
                assert_eq!(path, store.lock_file);
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn lock_path_missing_parent_returns_io_error() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().join("state");
        let state_file = state_dir.join("state.json");
        let lock_file = temp.path().join("missing").join("state.lock");
        let store = StateStore::with_paths(
            state_dir,
            state_file,
            lock_file.clone(),
            Duration::from_millis(100),
        );

        let err = store.get_session("alpha").unwrap_err();
        match err {
            StateError::Io { path, .. } => {
                assert_eq!(path, lock_file);
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn validate_state_content_rejects_empty_payloads() {
        let err = validate_state_content("").unwrap_err();
        match err {
            StateError::InvalidState(message) => {
                assert!(message.contains("empty state"));
            }
            other => panic!("expected InvalidState, got {other:?}"),
        }
        let err = validate_state_content("   ").unwrap_err();
        match err {
            StateError::InvalidState(message) => {
                assert!(message.contains("empty state"));
            }
            other => panic!("expected InvalidState, got {other:?}"),
        }
        assert!(validate_state_content("{\"sessions\":{}}").is_ok());
    }
}
