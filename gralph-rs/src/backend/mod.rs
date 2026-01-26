use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod claude;

pub trait Backend {
    fn check_installed(&self) -> bool;
    fn run_iteration(
        &self,
        prompt: &str,
        model: Option<&str>,
        output_file: &Path,
    ) -> Result<(), BackendError>;
    fn parse_text(&self, response_file: &Path) -> Result<String, BackendError>;
    fn get_models(&self) -> Vec<String>;
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
