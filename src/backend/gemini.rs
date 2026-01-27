use super::{Backend, BackendError};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

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

        let stdout_stream = io::stdout();
        let mut stdout_lock = stdout_stream.lock();

        for line in rx {
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
        }

        let status = child
            .wait()
            .map_err(|err| BackendError::Command(format!("failed to wait for gemini: {}", err)))?;

        let _ = stdout_handle.join();
        let _ = stderr_handle.join();

        if !status.success() {
            return Err(BackendError::Command(format!(
                "gemini exited with {}",
                status
            )));
        }

        Ok(())
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

fn command_in_path(command: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|dir| dir.join(command).is_file())
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
