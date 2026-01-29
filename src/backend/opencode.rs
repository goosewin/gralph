use super::{command_in_path, stream_command_output, Backend, BackendError};
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
    fn command_accessor_returns_configured_value() {
        let backend = OpenCodeBackend::with_command("custom-opencode");

        assert_eq!(backend.command(), "custom-opencode");
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
    fn parse_text_returns_empty_string_for_empty_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let backend = OpenCodeBackend::new();
        let result = backend.parse_text(&path).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn parse_text_preserves_trailing_whitespace() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("trailing.txt");
        let contents = "line one  \nline two\n\n";
        fs::write(&path, contents).unwrap();

        let backend = OpenCodeBackend::new();
        let result = backend.parse_text(&path).unwrap();

        assert_eq!(result, contents);
    }

    #[test]
    fn parse_text_returns_io_error_for_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.txt");

        let backend = OpenCodeBackend::new();
        let result = backend.parse_text(&path);

        assert!(matches!(
            result,
            Err(BackendError::Io { path: error_path, .. }) if error_path == path
        ));
    }

    #[test]
    fn parse_text_returns_io_error_for_directory() {
        let temp = tempfile::tempdir().unwrap();
        let dir_path = temp.path().join("opencode-dir");
        fs::create_dir(&dir_path).unwrap();

        let backend = OpenCodeBackend::new();
        let result = backend.parse_text(&dir_path);

        assert!(matches!(
            result,
            Err(BackendError::Io { path, .. }) if path == dir_path
        ));
    }

    #[test]
    fn check_installed_reflects_path_entries() {
        let _lock = crate::test_support::env_lock();
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
    fn check_installed_uses_path_override() {
        let _lock = crate::test_support::env_lock();
        let temp = tempfile::tempdir().unwrap();
        let command_name = "opencode-custom";
        let backend = OpenCodeBackend::with_command(command_name.to_string());

        {
            let _guard = PathGuard::set(None);
            assert!(!backend.check_installed());
        }

        fs::write(temp.path().join(command_name), "stub").unwrap();
        let combined =
            env::join_paths([temp.path(), temp.path().join("missing").as_path()]).unwrap();
        let _guard = PathGuard::set(Some(combined.as_os_str()));

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

    #[test]
    fn run_iteration_returns_io_when_output_path_is_directory() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("output-dir");
        fs::create_dir(&output_path).unwrap();
        let backend = OpenCodeBackend::with_command("opencode".to_string());

        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Io { path, .. }) if path == output_path
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
        let backend = OpenCodeBackend::with_command("opencode".to_string());
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
    fn run_iteration_sets_lsp_env_and_orders_model_variant_prompt() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-order");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'env:%s\\n' \"$OPENCODE_EXPERIMENTAL_LSP_TOOL\"\nfor arg in \"$@\"; do\n  printf '%s\\n' \"$arg\"\ndone\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration(
                "final-prompt",
                Some("model-x"),
                Some("variant-y"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        let mut lines = output.lines();
        assert_eq!(lines.next(), Some("env:true"));

        let args: Vec<&str> = lines.collect();
        assert_eq!(
            args,
            vec![
                "run",
                "--model",
                "model-x",
                "--variant",
                "variant-y",
                "final-prompt",
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_sets_lsp_env_without_model_variant() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-no-flags");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'env:%s\\n' \"$OPENCODE_EXPERIMENTAL_LSP_TOOL\"\nfor arg in \"$@\"; do\n  printf '%s\\n' \"$arg\"\ndone\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt-only", None, None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        let mut lines = output.lines();
        assert_eq!(lines.next(), Some("env:true"));
        assert_eq!(lines.next(), Some("run"));
        assert_eq!(lines.next(), Some("prompt-only"));
        assert_eq!(lines.next(), None);
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
            .run_iteration("prompt", Some("  "), Some("\t"), &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|prompt|"));
        assert!(!output.contains("--model"));
        assert!(!output.contains("--variant"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_skips_empty_model_but_keeps_variant() {
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
                Some("   "),
                Some("variant-z"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|--variant|variant-z|prompt|"));
        assert!(!output.contains("--model"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_skips_empty_variant_but_keeps_model() {
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
                Some("\t"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|--model|model-x|prompt|"));
        assert!(!output.contains("--variant"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_orders_args_with_model_only() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-model-only");
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
                Some("model-only"),
                None,
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|--model|model-only|prompt|"));
        assert!(!output.contains("--variant"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_orders_args_with_variant_only() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-variant-only");
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
                None,
                Some("variant-only"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("args:run|--variant|variant-only|prompt|"));
        assert!(!output.contains("--model"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_writes_stdout_and_keeps_prompt_last() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-args");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'env:%s\\n' \"$OPENCODE_EXPERIMENTAL_LSP_TOOL\"\nprintf 'stdout-line\\n'\nprintf 'args:'\nfor arg in \"$@\"; do\n  printf '%s|' \"$arg\"\ndone\nprintf '\\n'\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration(
                "final-prompt",
                Some("model-a"),
                Some("variant-b"),
                &output_path,
                temp.path(),
            )
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("env:true"));
        assert!(output.contains("stdout-line"));

        let args_line = output
            .lines()
            .find(|line| line.starts_with("args:"))
            .expect("args line should be present");
        let args: Vec<&str> = args_line
            .trim_start_matches("args:")
            .split('|')
            .filter(|value| !value.is_empty())
            .collect();
        assert_eq!(args.last().copied(), Some("final-prompt"));
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

    #[cfg(unix)]
    #[test]
    fn run_iteration_captures_stderr_only_output() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-stderr");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'stderr-one\\n' 1>&2\nprintf 'stderr-two\\n' 1>&2\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", None, None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert_eq!(output, "stderr-one\nstderr-two\n");
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_captures_stdout_and_stderr() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("opencode-mixed");
        let output_path = temp.path().join("output.txt");
        let script = "#!/bin/sh\nprintf 'stdout-one\\n'\nprintf 'stderr-one\\n' 1>&2\nprintf 'stdout-two\\n'\nprintf 'stderr-two\\n' 1>&2\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = OpenCodeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", None, None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("stdout-one\n"));
        assert!(output.contains("stderr-one\n"));
        assert!(output.contains("stdout-two\n"));
        assert!(output.contains("stderr-two\n"));
    }
}
