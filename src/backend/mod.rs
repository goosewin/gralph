use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_selection_returns_expected_type() {
        let cases = ["claude", "opencode", "gemini", "codex"];

        for name in cases {
            assert!(backend_from_name(name).is_ok(), "{} should resolve", name);
        }
        assert!(backend_from_name("unknown").is_err());
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
}
