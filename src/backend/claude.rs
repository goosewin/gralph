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
}
