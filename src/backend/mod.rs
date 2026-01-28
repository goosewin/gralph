use std::env;
use std::error::Error;
use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc;
use std::thread;

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod opencode;

use self::claude::ClaudeBackend;
use self::codex::CodexBackend;
use self::gemini::GeminiBackend;
use self::opencode::OpenCodeBackend;

pub trait Backend {
    fn check_installed(&self) -> bool;
    fn run_iteration(
        &self,
        prompt: &str,
        model: Option<&str>,
        variant: Option<&str>,
        output_file: &Path,
        working_dir: &Path,
    ) -> Result<(), BackendError>;
    fn parse_text(&self, response_file: &Path) -> Result<String, BackendError>;
    fn get_models(&self) -> Vec<String>;
}

pub fn backend_from_name(name: &str) -> Result<Box<dyn Backend>, String> {
    match name {
        "claude" => Ok(Box::new(ClaudeBackend::new())),
        "opencode" => Ok(Box::new(OpenCodeBackend::new())),
        "gemini" => Ok(Box::new(GeminiBackend::new())),
        "codex" => Ok(Box::new(CodexBackend::new())),
        other => Err(format!("Unknown backend: {}", other)),
    }
}

#[derive(Debug)]
pub enum BackendError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        source: serde_json::Error,
    },
    Command(String),
    InvalidInput(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::Io { path, source } => {
                write!(f, "backend io error at {}: {}", path.display(), source)
            }
            BackendError::Json { source } => write!(f, "backend json error: {}", source),
            BackendError::Command(message) => write!(f, "backend command error: {}", message),
            BackendError::InvalidInput(message) => {
                write!(f, "backend input error: {}", message)
            }
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BackendError::Io { source, .. } => Some(source),
            BackendError::Json { source } => Some(source),
            _ => None,
        }
    }
}

pub(crate) fn command_in_path(command: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|dir| dir.join(command).is_file())
}

pub(crate) fn stream_command_output<F>(
    mut child: Child,
    backend_label: &str,
    mut on_line: F,
) -> Result<(), BackendError>
where
    F: FnMut(String) -> Result<(), BackendError>,
{
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| BackendError::Command("failed to capture stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| BackendError::Command("failed to capture stderr".to_string()))?;

    let (tx, rx) = mpsc::channel();
    let stdout_handle = spawn_reader(stdout, tx.clone());
    let stderr_handle = spawn_reader(stderr, tx);

    for line in rx {
        on_line(line)?;
    }

    let status = child.wait().map_err(|err| {
        BackendError::Command(format!("failed to wait for {}: {}", backend_label, err))
    })?;

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    if !status.success() {
        return Err(BackendError::Command(format!(
            "{} exited with {}",
            backend_label, status
        )));
    }

    Ok(())
}

fn spawn_reader<R: Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buffer).to_string();
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{OsStr, OsString};
    use std::fs;

    #[cfg(unix)]
    use std::process::{Command, Stdio};

    struct PathGuard {
        original: Option<OsString>,
    }

    impl PathGuard {
        fn set(value: Option<&OsStr>) -> Self {
            let original = env::var_os("PATH");
            match value {
                Some(value) => unsafe {
                    env::set_var("PATH", value);
                },
                None => unsafe {
                    env::remove_var("PATH");
                },
            }
            Self { original }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            match self.original.as_ref() {
                Some(value) => unsafe {
                    env::set_var("PATH", value);
                },
                None => unsafe {
                    env::remove_var("PATH");
                },
            }
        }
    }

    #[test]
    fn backend_selection_returns_expected_type() {
        let cases = ["claude", "opencode", "gemini", "codex"];

        for name in cases {
            assert!(backend_from_name(name).is_ok(), "{} should resolve", name);
        }
        let err = backend_from_name("unknown").unwrap_err();
        assert_eq!(err, "Unknown backend: unknown");
    }

    #[test]
    fn backend_models_are_non_empty_and_stable() {
        let cases = [
            ("claude", vec!["claude-opus-4-5".to_string()]),
            (
                "opencode",
                vec![
                    "opencode/example-code-model".to_string(),
                    "anthropic/claude-opus-4-5".to_string(),
                    "google/gemini-1.5-pro".to_string(),
                ],
            ),
            ("gemini", vec!["gemini-1.5-pro".to_string()]),
            ("codex", vec!["example-codex-model".to_string()]),
        ];

        for (name, expected_models) in cases {
            let backend = backend_from_name(name).expect("backend should be available");
            let models = backend.get_models();
            assert!(!models.is_empty(), "{} models should be non-empty", name);
            assert_eq!(models, expected_models, "{} models should be stable", name);
        }
    }

    #[test]
    fn command_in_path_handles_missing_and_empty_path() {
        let dir_temp = tempfile::tempdir().unwrap();
        let file_temp = tempfile::tempdir().unwrap();
        let command_name = "gralph-test-command";
        fs::create_dir(dir_temp.path().join(command_name)).unwrap();
        fs::write(file_temp.path().join(command_name), "stub").unwrap();

        let _guard = PathGuard::set(None);
        assert!(!command_in_path(command_name));

        unsafe {
            env::set_var("PATH", "");
        }
        assert!(!command_in_path(command_name));

        unsafe {
            env::set_var("PATH", dir_temp.path());
        }
        assert!(!command_in_path(command_name));

        unsafe {
            env::set_var("PATH", file_temp.path());
        }
        assert!(command_in_path(command_name));
    }

    #[cfg(unix)]
    #[test]
    fn stream_command_output_returns_ok_on_success() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("printf 'stdout-line\\n'; printf 'stderr-line\\n' 1>&2")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut lines = Vec::new();
        let result = stream_command_output(child, "stub", |line| {
            lines.push(line);
            Ok(())
        });

        assert!(result.is_ok());
        assert!(lines.iter().any(|line| line.contains("stdout-line")));
        assert!(lines.iter().any(|line| line.contains("stderr-line")));
    }

    #[cfg(unix)]
    #[test]
    fn stream_command_output_propagates_on_line_error() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("printf 'stdout-line\\n'")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let result = stream_command_output(child, "stub", |_| {
            Err(BackendError::Command("line failed".to_string()))
        });

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message == "line failed"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn stream_command_output_reports_non_zero_exit() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("printf 'stderr-line\\n' 1>&2; exit 2")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let result = stream_command_output(child, "stub", |_| Ok(()));

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message.contains("stub exited with")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn stream_command_output_errors_when_stdout_missing() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let result = stream_command_output(child, "stub", |_| Ok(()));

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message == "failed to capture stdout"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn stream_command_output_errors_when_stderr_missing() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let result = stream_command_output(child, "stub", |_| Ok(()));

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message == "failed to capture stderr"
        ));
    }
}
