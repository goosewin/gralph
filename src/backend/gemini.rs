use super::{command_in_path, stream_command_output, Backend, BackendError};
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct GeminiBackend {
    command: String,
}

impl GeminiBackend {
    pub fn new() -> Self {
        Self {
            command: "gemini".to_string(),
        }
    }

    pub fn with_command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    pub fn command(&self) -> &str {
        &self.command
    }
}

impl Default for GeminiBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for GeminiBackend {
    fn check_installed(&self) -> bool {
        command_in_path(&self.command)
    }

    fn run_iteration(
        &self,
        prompt: &str,
        model: Option<&str>,
        _variant: Option<&str>,
        output_file: &Path,
        working_dir: &Path,
    ) -> Result<(), BackendError> {
        if prompt.trim().is_empty() {
            return Err(BackendError::InvalidInput("prompt is required".to_string()));
        }

        let file = File::create(output_file).map_err(|source| BackendError::Io {
            path: output_file.to_path_buf(),
            source,
        })?;
        let mut output = BufWriter::new(file);

        let mut cmd = Command::new(&self.command);
        cmd.current_dir(working_dir);
        cmd.arg("--headless");
        if let Some(model) = model {
            if !model.trim().is_empty() {
                cmd.arg("--model").arg(model);
            }
        }
        cmd.arg(prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|err| BackendError::Command(format!("failed to spawn gemini: {}", err)))?;

        let stdout_stream = io::stdout();
        let mut stdout_lock = stdout_stream.lock();

        stream_command_output(child, "gemini", |line| {
            output
                .write_all(line.as_bytes())
                .map_err(|source| BackendError::Io {
                    path: output_file.to_path_buf(),
                    source,
                })?;
            stdout_lock
                .write_all(line.as_bytes())
                .map_err(|source| BackendError::Io {
                    path: PathBuf::from("stdout"),
                    source,
                })?;
            stdout_lock.flush().map_err(|source| BackendError::Io {
                path: PathBuf::from("stdout"),
                source,
            })?;
            Ok(())
        })
    }

    fn parse_text(&self, response_file: &Path) -> Result<String, BackendError> {
        fs::read_to_string(response_file).map_err(|source| BackendError::Io {
            path: response_file.to_path_buf(),
            source,
        })
    }

    fn get_models(&self) -> Vec<String> {
        vec!["gemini-1.5-pro".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::io;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

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
    fn command_accessor_returns_configured_command() {
        let backend = GeminiBackend::with_command("gemini-custom".to_string());

        assert_eq!(backend.command(), "gemini-custom");
    }

    #[test]
    fn parse_text_returns_raw_contents() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("gemini.txt");
        fs::write(&path, "hello gemini\n").unwrap();

        let backend = GeminiBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, "hello gemini\n");
    }

    #[test]
    fn parse_text_allows_empty_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let backend = GeminiBackend::new();
        let result = backend.parse_text(&path).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn parse_text_returns_io_error_for_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.txt");

        let backend = GeminiBackend::new();
        let result = backend.parse_text(&path);

        assert!(matches!(
            result,
            Err(BackendError::Io { path: error_path, .. }) if error_path == path
        ));
    }

    #[test]
    fn parse_text_returns_io_error_for_directory() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("response-dir");
        fs::create_dir(&path).unwrap();

        let backend = GeminiBackend::new();
        let result = backend.parse_text(&path);

        assert!(matches!(
            result,
            Err(BackendError::Io { path: error_path, .. }) if error_path == path
        ));
    }

    #[test]
    fn parse_text_returns_io_error_for_invalid_utf8() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("invalid.txt");
        fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();

        let backend = GeminiBackend::new();
        let result = backend.parse_text(&path);

        match result {
            Err(BackendError::Io {
                path: error_path,
                source,
            }) => {
                assert_eq!(error_path, path);
                assert_eq!(source.kind(), io::ErrorKind::InvalidData);
            }
            _ => panic!("expected invalid utf-8 io error"),
        }
    }

    #[test]
    fn check_installed_respects_path_override() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        let command_name = "gemini-path";
        fs::write(temp.path().join(command_name), "stub").unwrap();

        let _guard = PathGuard::set(Some(temp.path().as_os_str()));
        let backend = GeminiBackend::with_command(command_name.to_string());

        assert!(backend.check_installed());
    }

    #[test]
    fn check_installed_returns_false_when_path_unset() {
        let _lock = crate::test_support::env_lock();
        let _guard = PathGuard::set(None);
        let backend = GeminiBackend::with_command("gemini-unset".to_string());

        assert!(!backend.check_installed());
    }

    #[test]
    fn check_installed_reflects_path_entries() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        let command_name = "gemini-stub";

        {
            let _guard = PathGuard::set(None);
            let backend = GeminiBackend::with_command(command_name.to_string());
            assert!(!backend.check_installed());
        }

        let _guard = PathGuard::set(Some(temp.path().as_os_str()));
        let backend = GeminiBackend::with_command(command_name.to_string());
        assert!(!backend.check_installed());

        fs::write(temp.path().join(command_name), "stub").unwrap();
        assert!(backend.check_installed());
    }

    #[test]
    fn check_installed_ignores_non_directory_path_entries() {
        let _lock = crate::test_support::env_lock();
        let command_name = "gemini-nondir";
        let command_dir = tempfile::tempdir().unwrap();
        let file_entry_dir = tempfile::tempdir().unwrap();
        let file_entry = file_entry_dir.path().join("not-a-dir");
        fs::write(&file_entry, "stub").unwrap();
        fs::write(command_dir.path().join(command_name), "stub").unwrap();

        let _guard = PathGuard::set(None);
        unsafe {
            env::set_var("PATH", &file_entry);
        }
        let backend = GeminiBackend::with_command(command_name.to_string());
        assert!(!backend.check_installed());

        let combined = env::join_paths([file_entry.as_path(), command_dir.path()]).unwrap();
        unsafe {
            env::set_var("PATH", &combined);
        }
        assert!(backend.check_installed());
    }

    #[test]
    fn run_iteration_rejects_empty_prompt() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("output.txt");
        let backend = GeminiBackend::with_command("gemini".to_string());

        let result = backend.run_iteration("   ", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::InvalidInput(message)) if message == "prompt is required"
        ));
    }

    #[test]
    fn run_iteration_reports_spawn_failure() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("output.txt");
        let missing_command = temp.path().join("missing-gemini");
        let backend = GeminiBackend::with_command(missing_command.to_string_lossy().to_string());

        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message.contains("failed to spawn gemini")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_returns_io_when_output_dir_is_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let output_dir = temp.path().join("readonly");
        fs::create_dir(&output_dir).unwrap();
        let mut permissions = fs::metadata(&output_dir).unwrap().permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&output_dir, permissions).unwrap();

        let output_path = output_dir.join("output.txt");
        let backend = GeminiBackend::with_command("gemini".to_string());
        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Io { path, .. }) if path == output_path
        ));

        let mut permissions = fs::metadata(&output_dir).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&output_dir, permissions).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_includes_headless_and_model() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("gemini-mock");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'args:'\nfor arg in \"$@\"; do\n  printf '%s|' \"$arg\"\ndone\nprintf '\\n'\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = GeminiBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", Some("model-x"), None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("args:--headless|--model|model-x|prompt|"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_keeps_prompt_last_and_headless_first() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("gemini-mock");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf '%s\\n' \"$@\"\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = GeminiBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration(
                "final-prompt",
                Some("model-y"),
                None,
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        let args: Vec<&str> = output.lines().collect();
        assert_eq!(
            args,
            vec!["--headless", "--model", "model-y", "final-prompt"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_omits_model_flag_when_none() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("gemini-mock");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf '%s\\n' \"$@\"\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = GeminiBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("final-prompt", None, None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        let args: Vec<&str> = output.lines().collect();
        assert_eq!(args, vec!["--headless", "final-prompt"]);
        assert!(!output.contains("--model"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_skips_empty_model() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("gemini-mock");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'args:'\nfor arg in \"$@\"; do\n  printf '%s|' \"$arg\"\ndone\nprintf '\\n'\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = GeminiBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", Some("  "), None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert_eq!(output, "args:--headless|prompt|\n");
        assert!(!output.contains("--model"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_reports_non_zero_exit() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("gemini-fail");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'boom\\n'\nexit 3\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = GeminiBackend::with_command(script_path.to_string_lossy().to_string());
        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message.contains("gemini exited with")
        ));
    }
}
