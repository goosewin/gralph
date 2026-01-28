use super::{Backend, BackendError, stream_command_output};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct ClaudeBackend {
    command: String,
}

impl ClaudeBackend {
    pub fn new() -> Self {
        Self {
            command: "claude".to_string(),
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

impl Default for ClaudeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for ClaudeBackend {
    fn check_installed(&self) -> bool {
        Command::new(&self.command)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
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
        cmd.arg("--dangerously-skip-permissions")
            .arg("--verbose")
            .arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("-p")
            .arg(prompt)
            .env("IS_SANDBOX", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(model) = model {
            if !model.trim().is_empty() {
                cmd.arg("--model").arg(model);
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|err| BackendError::Command(format!("failed to spawn claude: {}", err)))?;

        let stdout_stream = io::stdout();
        let mut stdout_lock = stdout_stream.lock();

        stream_command_output(child, "claude", |line| {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            let json_line = trimmed.trim_start();
            if !json_line.starts_with('{') {
                return Ok(());
            }
            writeln!(output, "{}", json_line).map_err(|source| BackendError::Io {
                path: output_file.to_path_buf(),
                source,
            })?;
            if let Ok(value) = serde_json::from_str::<Value>(json_line) {
                for text in extract_assistant_texts(&value) {
                    let mut rendered = text.replace('\n', "\r\n");
                    rendered.push_str("\r\n\n");
                    stdout_lock
                        .write_all(rendered.as_bytes())
                        .map_err(|source| BackendError::Io {
                            path: PathBuf::from("stdout"),
                            source,
                        })?;
                    stdout_lock.flush().map_err(|source| BackendError::Io {
                        path: PathBuf::from("stdout"),
                        source,
                    })?;
                }
            }
            Ok(())
        })
    }

    fn parse_text(&self, response_file: &Path) -> Result<String, BackendError> {
        let contents = fs::read_to_string(response_file).map_err(|source| BackendError::Io {
            path: response_file.to_path_buf(),
            source,
        })?;
        let mut result = None;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            if let Some(text) = extract_result_text(&value) {
                result = Some(text);
            }
        }
        if let Some(text) = result {
            Ok(text)
        } else {
            Ok(contents)
        }
    }

    fn get_models(&self) -> Vec<String> {
        vec!["claude-opus-4-5".to_string()]
    }
}

fn extract_assistant_texts(value: &Value) -> Vec<String> {
    if value.get("type").and_then(|v| v.as_str()) != Some("assistant") {
        return Vec::new();
    }
    let Some(content) = value.pointer("/message/content").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    for item in content {
        if item.get("type").and_then(|v| v.as_str()) != Some("text") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
            texts.push(text.to_string());
        }
    }
    texts
}

fn extract_result_text(value: &Value) -> Option<String> {
    if value.get("type").and_then(|v| v.as_str()) != Some("result") {
        return None;
    }
    value
        .get("result")
        .and_then(|v| v.as_str())
        .map(|text| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn parse_text_returns_result_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stream.json");
        let contents = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}}\n{\"type\":\"result\",\"result\":\"done\"}\n";
        fs::write(&path, contents).unwrap();

        let backend = ClaudeBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, "done");
    }

    #[test]
    fn extract_assistant_texts_filters_by_role_and_content() {
        let assistant = json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "first"},
                    {"type": "image", "source": "ignored"},
                    {"type": "text", "text": "second"}
                ]
            }
        });
        let user = json!({
            "type": "user",
            "message": {"content": [{"type": "text", "text": "nope"}]}
        });

        assert_eq!(
            extract_assistant_texts(&assistant),
            vec!["first".to_string(), "second".to_string()]
        );
        assert!(extract_assistant_texts(&user).is_empty());
    }

    #[test]
    fn extract_assistant_texts_handles_missing_or_non_text_content() {
        let missing_content = json!({
            "type": "assistant",
            "message": {}
        });
        let non_array_content = json!({
            "type": "assistant",
            "message": {"content": "nope"}
        });
        let non_text_only = json!({
            "type": "assistant",
            "message": {"content": [{"type": "image", "source": "ignored"}]}
        });

        assert!(extract_assistant_texts(&missing_content).is_empty());
        assert!(extract_assistant_texts(&non_array_content).is_empty());
        assert!(extract_assistant_texts(&non_text_only).is_empty());
    }

    #[test]
    fn extract_result_text_requires_result_type() {
        let result = json!({"type": "result", "result": "done"});
        let assistant = json!({"type": "assistant", "result": "ignored"});

        assert_eq!(extract_result_text(&result), Some("done".to_string()));
        assert_eq!(extract_result_text(&assistant), None);
    }

    #[test]
    fn parse_text_returns_last_result_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stream.json");
        let contents = "{\"type\":\"result\",\"result\":\"first\"}\n{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}}\n{\"type\":\"result\",\"result\":\"second\"}\n";
        fs::write(&path, contents).unwrap();

        let backend = ClaudeBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, "second");
    }

    #[test]
    fn parse_text_returns_result_when_not_last() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stream.json");
        let contents = "{\"type\":\"result\",\"result\":\"first\"}\n{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}}\ntrailing\n";
        fs::write(&path, contents).unwrap();

        let backend = ClaudeBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, "first");
    }

    #[test]
    fn parse_text_returns_raw_contents_without_result() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stream.json");
        let contents = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}}\nnot-json\n";
        fs::write(&path, contents).unwrap();

        let backend = ClaudeBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, contents);
    }

    #[test]
    fn run_iteration_rejects_empty_prompt() {
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("output.json");
        let backend = ClaudeBackend::with_command("claude".to_string());

        let result = backend.run_iteration("   ", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::InvalidInput(message)) if message == "prompt is required"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_writes_stream_to_output() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("claude-mock");
        let output_path = temp.path().join("output.json");
        let script = "#!/bin/sh\necho '{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Hello\\nworld\"}]}}'\necho '{\"type\":\"result\",\"result\":\"done\"}'\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = ClaudeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", None, None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("\"type\":\"assistant\""));
        assert!(output.contains("\"type\":\"result\""));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_includes_model_flag_when_set() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("claude-mock");
        let output_path = temp.path().join("output.json");
        let script = r#"#!/bin/sh
printf '{"type":"result","result":"'
for arg in "$@"; do
  printf '%s|' "$arg"
done
printf '"}\n'
"#;
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = ClaudeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", Some("model-x"), None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        let value: Value = serde_json::from_str(output.trim()).unwrap();
        let result = value.get("result").and_then(|value| value.as_str()).unwrap();
        assert!(result.contains("--model|model-x|"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_skips_empty_model_flag() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("claude-mock");
        let output_path = temp.path().join("output.json");
        let script = r#"#!/bin/sh
printf '{"type":"result","result":"'
for arg in "$@"; do
  printf '%s|' "$arg"
done
printf '"}\n'
"#;
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = ClaudeBackend::with_command(script_path.to_string_lossy().to_string());
        backend
            .run_iteration("prompt", Some("  "), None, &output_path, temp.path())
            .expect("run_iteration should succeed");

        let output = fs::read_to_string(&output_path).unwrap();
        let value: Value = serde_json::from_str(output.trim()).unwrap();
        let result = value.get("result").and_then(|value| value.as_str()).unwrap();
        assert!(!result.contains("--model"));
    }

    #[cfg(unix)]
    #[test]
    fn run_iteration_propagates_non_zero_exit() {
        let temp = tempfile::tempdir().unwrap();
        let script_path = temp.path().join("claude-mock");
        let output_path = temp.path().join("output.json");
        let script = "#!/bin/sh\nexit 2\n";
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let backend = ClaudeBackend::with_command(script_path.to_string_lossy().to_string());
        let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

        assert!(matches!(
            result,
            Err(BackendError::Command(message)) if message.contains("claude exited with")
        ));
    }
}
