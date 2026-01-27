use reqwest::blocking::Client;
use serde_json::Value;
use std::cmp::Ordering;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RELEASE_URL: &str = "https://api.github.com/repos/goosewin/gralph/releases/latest";
const RELEASE_DOWNLOAD_URL: &str = "https://github.com/goosewin/gralph/releases/download";
const USER_AGENT: &str = "gralph-cli";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
}

#[derive(Debug)]
pub enum UpdateError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Io(io::Error),
    MissingTag,
    MissingBinary(String),
    InvalidVersion(String),
    UnsupportedPlatform(String),
    PermissionDenied(String),
    CommandFailed(String),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::Http(err) => write!(f, "{}", err),
            UpdateError::Json(err) => write!(f, "{}", err),
            UpdateError::Io(err) => write!(f, "{}", err),
            UpdateError::MissingTag => write!(f, "Latest release tag missing."),
            UpdateError::MissingBinary(path) => {
                write!(f, "Downloaded archive missing binary at {}", path)
            }
            UpdateError::InvalidVersion(value) => {
                write!(f, "Invalid version string: {}", value)
            }
            UpdateError::UnsupportedPlatform(value) => {
                write!(f, "Unsupported platform: {}", value)
            }
            UpdateError::PermissionDenied(path) => write!(
                f,
                "Permission denied writing to {}. Set GRALPH_INSTALL_DIR or run with elevated privileges.",
                path
            ),
            UpdateError::CommandFailed(value) => write!(f, "{}", value),
        }
    }
}

impl From<reqwest::Error> for UpdateError {
    fn from(value: reqwest::Error) -> Self {
        UpdateError::Http(value)
    }
}

impl From<serde_json::Error> for UpdateError {
    fn from(value: serde_json::Error) -> Self {
        UpdateError::Json(value)
    }
}

impl From<io::Error> for UpdateError {
    fn from(value: io::Error) -> Self {
        UpdateError::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    pub version: String,
    pub install_dir: PathBuf,
    pub install_path: PathBuf,
    pub resolved_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    fn parse(raw: &str) -> Result<Self, UpdateError> {
        let trimmed = raw.trim().trim_start_matches('v');
        if trimmed.is_empty() {
            return Err(UpdateError::InvalidVersion(raw.to_string()));
        }
        if trimmed.contains('-') || trimmed.contains('+') {
            return Err(UpdateError::InvalidVersion(raw.to_string()));
        }
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() != 3 {
            return Err(UpdateError::InvalidVersion(raw.to_string()));
        }
        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| UpdateError::InvalidVersion(raw.to_string()))?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| UpdateError::InvalidVersion(raw.to_string()))?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| UpdateError::InvalidVersion(raw.to_string()))?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn check_for_update(current_version: &str) -> Result<Option<UpdateInfo>, UpdateError> {
    let latest_tag = fetch_latest_release_tag()?;
    let latest_version = Version::parse(&latest_tag)?;
    let current_version_parsed = Version::parse(current_version)?;
    if latest_version > current_version_parsed {
        Ok(Some(UpdateInfo {
            current: current_version_parsed.to_string(),
            latest: latest_version.to_string(),
        }))
    } else {
        Ok(None)
    }
}

pub fn install_release() -> Result<InstallOutcome, UpdateError> {
    let install_dir =
        env::var("GRALPH_INSTALL_DIR").unwrap_or_else(|_| "/usr/local/bin".to_string());
    let requested_version = env::var("GRALPH_VERSION").unwrap_or_else(|_| "latest".to_string());
    let version = resolve_install_version(&requested_version)?;
    let platform = detect_platform()?;

    let url = format!(
        "{}/v{}/gralph-{}-{}.tar.gz",
        RELEASE_DOWNLOAD_URL, version, version, platform
    );
    let temp_dir = TempDir::new("gralph-update")?;
    let archive_path = temp_dir.path.join("gralph.tar.gz");
    download_release(&url, &archive_path)?;
    extract_archive(&archive_path, &temp_dir.path)?;

    let binary_path = temp_dir
        .path
        .join(format!("gralph-{}", version))
        .join("gralph");
    if !binary_path.is_file() {
        return Err(UpdateError::MissingBinary(
            binary_path.display().to_string(),
        ));
    }

    let install_dir = PathBuf::from(install_dir);
    install_binary(&binary_path, &install_dir)?;
    let install_path = install_dir.join("gralph");
    let resolved_path = resolve_in_path("gralph");

    Ok(InstallOutcome {
        version,
        install_dir,
        install_path,
        resolved_path,
    })
}

fn fetch_latest_release_tag() -> Result<String, UpdateError> {
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let response = client
        .get(RELEASE_URL)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()?
        .error_for_status()?;
    let body = response.text()?;
    parse_release_tag(&body)
}

fn parse_release_tag(body: &str) -> Result<String, UpdateError> {
    let json: Value = serde_json::from_str(body)?;
    let tag = json
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(UpdateError::MissingTag)?;
    Ok(tag)
}

fn resolve_install_version(raw: &str) -> Result<String, UpdateError> {
    if raw.trim().eq_ignore_ascii_case("latest") {
        let tag = fetch_latest_release_tag()?;
        return normalize_version(&tag);
    }
    normalize_version(raw)
}

fn normalize_version(raw: &str) -> Result<String, UpdateError> {
    let parsed = Version::parse(raw)?;
    Ok(parsed.to_string())
}

fn detect_platform() -> Result<String, UpdateError> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let os = match os {
        "linux" => "linux",
        "macos" => "macos",
        other => return Err(UpdateError::UnsupportedPlatform(other.to_string())),
    };
    let arch = match (os, arch) {
        ("linux", "x86_64") => "x86_64",
        ("linux", "aarch64") | ("linux", "arm64") => "aarch64",
        ("macos", "x86_64") => "x86_64",
        ("macos", "aarch64") | ("macos", "arm64") => "arm64",
        (_, other) => {
            return Err(UpdateError::UnsupportedPlatform(format!(
                "{}-{}",
                os, other
            )))
        }
    };
    Ok(format!("{}-{}", os, arch))
}

fn download_release(url: &str, dest: &Path) -> Result<(), UpdateError> {
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()?
        .error_for_status()?;
    let body = response.bytes()?;
    fs::write(dest, body)?;
    Ok(())
}

fn extract_archive(archive_path: &Path, target_dir: &Path) -> Result<(), UpdateError> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(target_dir)
        .output()
        .map_err(UpdateError::Io)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(UpdateError::CommandFailed(format!(
            "Failed to extract archive: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

fn install_binary(binary_path: &Path, install_dir: &Path) -> Result<(), UpdateError> {
    if !install_dir.exists() {
        fs::create_dir_all(install_dir)?;
    }
    let target = install_dir.join("gralph");
    match fs::copy(binary_path, &target) {
        Ok(_) => {
            make_executable(&target)?;
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => Err(
            UpdateError::PermissionDenied(install_dir.display().to_string()),
        ),
        Err(err) => Err(UpdateError::Io(err)),
    }
}

fn make_executable(path: &Path) -> Result<(), UpdateError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

fn resolve_in_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for path in env::split_paths(&paths) {
        let candidate = path.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Result<Self, UpdateError> {
        let mut path = env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_millis();
        path.push(format!("{}-{}-{}", prefix, std::process::id(), stamp));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_version_accepts_v_prefix() {
        let version = Version::parse("v0.2.1").expect("version parsed");
        assert_eq!(
            version,
            Version {
                major: 0,
                minor: 2,
                patch: 1
            }
        );
    }

    #[test]
    fn parse_version_rejects_missing_patch() {
        let result = Version::parse("0.2");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn parse_release_tag_requires_tag_name() {
        let result = parse_release_tag("{}");
        assert!(matches!(result, Err(UpdateError::MissingTag)));
    }

    #[test]
    fn parse_release_tag_accepts_valid_tag() {
        let body = r#"{ "tag_name": "v0.2.1" }"#;
        let tag = parse_release_tag(body).expect("tag parsed");
        assert_eq!(tag, "v0.2.1");
    }

    #[test]
    fn detect_newer_version() {
        let latest = Version::parse("0.2.1").expect("latest parsed");
        let current = Version::parse("0.2.0").expect("current parsed");
        assert!(latest > current);
    }

    #[test]
    fn reject_invalid_version_in_check() {
        let result = Version::parse("0.2.beta");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn normalize_version_accepts_plain_input() {
        let version = normalize_version("0.2.1").expect("normalized");
        assert_eq!(version, "0.2.1");
    }

    #[test]
    fn detect_platform_reports_current_target() {
        if env::consts::OS == "windows" {
            let result = detect_platform();
            assert!(matches!(result, Err(UpdateError::UnsupportedPlatform(_))));
            return;
        }
        let os = match env::consts::OS {
            "linux" => "linux",
            "macos" => "macos",
            other => other,
        };
        let arch = match (os, env::consts::ARCH) {
            ("linux", "x86_64") => "x86_64",
            ("linux", "aarch64") | ("linux", "arm64") => "aarch64",
            ("macos", "x86_64") => "x86_64",
            ("macos", "aarch64") | ("macos", "arm64") => "arm64",
            (_, other) => other,
        };
        let expected = format!("{}-{}", os, arch);
        let resolved = detect_platform().expect("platform");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_in_path_finds_binary() {
        let temp = tempdir().expect("tempdir");
        let bin_path = temp.path().join("gralph");
        fs::write(&bin_path, "binary").expect("write");
        let original_path = env::var_os("PATH");
        unsafe {
            env::set_var("PATH", temp.path());
        }
        let resolved = resolve_in_path("gralph");
        if let Some(value) = original_path {
            unsafe {
                env::set_var("PATH", value);
            }
        }
        assert_eq!(resolved.as_deref(), Some(bin_path.as_path()));
    }

    #[test]
    fn install_binary_copies_to_install_dir() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("gralph");
        fs::write(&source, "binary").expect("write");
        let install_dir = temp.path().join("install");
        install_binary(&source, &install_dir).expect("install");
        let target = install_dir.join("gralph");
        assert!(target.is_file());
    }
}
