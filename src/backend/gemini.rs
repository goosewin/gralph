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
    use std::fs;

    #[test]
    fn parse_text_returns_raw_contents() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("gemini.txt");
        fs::write(&path, "hello gemini\n").unwrap();

        let backend = GeminiBackend::new();
        let result = backend.parse_text(&path).unwrap();
        assert_eq!(result, "hello gemini\n");
    }
}
