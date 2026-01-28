use super::{Backend, BackendError, command_in_path, stream_command_output};
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct OpenCodeBackend {
    command: String,
}

impl OpenCodeBackend {
    pub fn new() -> Self {
        Self {
            command: "opencode".to_string(),
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

const OPENCODE_LSP_ENV: &str = "OPENCODE_EXPERIMENTAL_LSP_TOOL";

impl Default for OpenCodeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for OpenCodeBackend {
    fn check_installed(&self) -> bool {
        command_in_path(&self.command)
    }

    fn run_iteration(
        &self,
        prompt: &str,
        model: Option<&str>,
        variant: Option<&str>,
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
        cmd.env(OPENCODE_LSP_ENV, "true");
        cmd.arg("run");
        if let Some(model) = model {
            if !model.trim().is_empty() {
                cmd.arg("--model").arg(model);
            }
        }
        if let Some(variant) = variant {
            if !variant.trim().is_empty() {
                cmd.arg("--variant").arg(variant);
            }
        }
        cmd.arg(prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|err| BackendError::Command(format!("failed to spawn opencode: {}", err)))?;

        let stdout_stream = io::stdout();
        let mut stdout_lock = stdout_stream.lock();

        stream_command_output(child, "opencode", |line| {
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
        vec![
            "opencode/example-code-model".to_string(),
            "anthropic/claude-opus-4-5".to_string(),
            "google/gemini-1.5-pro".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::fs;

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
    fn parse_text_returns_raw_contents() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("opencode.txt");
        fs::write(&path, "hello world\n").unwrap();

        let backend = OpenCodeBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, "hello world\n");
    }

    #[test]
    fn check_installed_reflects_path_entries() {
        let temp = tempfile::tempdir().unwrap();
        let command_name = "opencode-stub";

        {
            let _guard = PathGuard::set(None);
            let backend = OpenCodeBackend::with_command(command_name.to_string());
            assert!(!backend.check_installed());
        }

        let _guard = PathGuard::set(Some(temp.path().as_os_str()));
        let backend = OpenCodeBackend::with_command(command_name.to_string());
        assert!(!backend.check_installed());

        fs::write(temp.path().join(command_name), "stub").unwrap();
        assert!(backend.check_installed());
    }

    #[test]
    fn run_iteration_rejects_empty_prompt() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("output.txt");
        let backend = OpenCodeBackend::with_command("opencode".to_string());

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
        let missing_command = temp.path().join("missing-opencode");
        let backend = OpenCodeBackend::with_command(missing_command.to_string_lossy().to_string());

        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message.contains("failed to spawn opencode")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_sets_env_and_passes_model_variant() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-mock");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'env:%s\\n' \"$OPENCODE_EXPERIMENTAL_LSP_TOOL\"\nprintf 'args:'\nfor arg in \"$@\"; do\n  printf '%s|' \"$arg\"\ndone\nprintf '\\n'\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration(
                "prompt",
                Some("model-x"),
                Some("variant-y"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|--model|model-x|--variant|variant-y|prompt|"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_skips_empty_model_variant() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-mock");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'env:%s\\n' \"$OPENCODE_EXPERIMENTAL_LSP_TOOL\"\nprintf 'args:'\nfor arg in \"$@\"; do\n  printf '%s|' \"$arg\"\ndone\nprintf '\\n'\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration(
                "prompt",
                Some("  "),
                Some("\t"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|prompt|"));
        assert!(!output.contains("--model"));
        assert!(!output.contains("--variant"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_reports_non_zero_exit() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-fail");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'boom\\n'\nexit 3\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message.contains("opencode exited with")
        ));
    }
}
