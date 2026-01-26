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
        if content.trim().is_empty() {
            return Err(StateError::InvalidState(
                "refusing to write empty state content".to_string(),
            ));
        }
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

fn default_state_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("gralph")
}

fn acquire_lock(file: &File, timeout: Duration) -> Result<(), StateError> {
    let start = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() >= timeout {
                    return Err(StateError::LockTimeout { timeout });
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                return Err(StateError::Io {
                    path: PathBuf::from("state.lock"),
                    source: err,
                })
            }
        }
    }
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
    use std::sync::Arc;

    fn store_for_test(dir: &Path, timeout: Duration) -> StateStore {
        let state_dir = dir.join("state");
        let state_file = state_dir.join("state.json");
        let lock_file = state_dir.join("state.lock");
        StateStore::with_paths(state_dir, state_file, lock_file, timeout)
    }

    #[test]
    fn lock_times_out_when_held() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(store_for_test(temp.path(), Duration::from_millis(200)));
        store.init_state().unwrap();

        let blocker = Arc::clone(&store);
        let handle = thread::spawn(move || {
            blocker
                .with_lock(|| {
                    thread::sleep(Duration::from_millis(400));
                    Ok(())
                })
                .unwrap();
        });

        thread::sleep(Duration::from_millis(50));
        let result = store.with_lock(|| Ok(()));
        assert!(matches!(result, Err(StateError::LockTimeout { .. })));
        handle.join().unwrap();
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
}
