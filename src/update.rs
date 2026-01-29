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

fn release_url() -> String {
    #[cfg(test)]
    {
        if let Ok(value) = env::var("GRALPH_TEST_RELEASE_URL") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    RELEASE_URL.to_string()
}

fn release_download_url() -> String {
    #[cfg(test)]
    {
        if let Ok(value) = env::var("GRALPH_TEST_RELEASE_DOWNLOAD_URL") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    RELEASE_DOWNLOAD_URL.to_string()
}

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
    // Default to ~/.local/bin for user installs (no sudo needed).
    // Falls back to /usr/local/bin if HOME is not set (rare edge case).
    let install_dir = env::var("GRALPH_INSTALL_DIR").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".local/bin").to_string_lossy().to_string())
            .unwrap_or_else(|| "/usr/local/bin".to_string())
    });
    let requested_version = env::var("GRALPH_VERSION").unwrap_or_else(|_| "latest".to_string());
    let version = resolve_install_version(&requested_version)?;
    let platform = detect_platform()?;

    let url = format!(
        "{}/v{}/gralph-{}-{}.tar.gz",
        release_download_url(),
        version,
        version,
        platform
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
    if let Some(tag) = latest_release_override() {
        return Ok(tag);
    }
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let response = client
        .get(release_url())
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
        if let Some(tag) = latest_release_override() {
            return normalize_version(&tag);
        }
        let tag = fetch_latest_release_tag()?;
        return normalize_version(&tag);
    }
    normalize_version(raw)
}

fn latest_release_override() -> Option<String> {
    #[cfg(test)]
    {
        env::var("GRALPH_TEST_LATEST_TAG")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }
    #[cfg(not(test))]
    {
        None
    }
}

fn normalize_version(raw: &str) -> Result<String, UpdateError> {
    let parsed = Version::parse(raw)?;
    Ok(parsed.to_string())
}

fn detect_platform() -> Result<String, UpdateError> {
    detect_platform_for(env::consts::OS, env::consts::ARCH)
}

fn detect_platform_for(os: &str, arch: &str) -> Result<String, UpdateError> {
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
            )));
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
    let metadata = fs::metadata(archive_path).map_err(UpdateError::Io)?;
    if metadata.len() == 0 {
        return Err(UpdateError::CommandFailed(
            "Failed to extract archive: archive is empty".to_string(),
        ));
    }
    let mut cmd = Command::new("tar");
    let path_env = env::var_os("PATH");
    if path_env.as_ref().map_or(true, |value| value.is_empty()) {
        cmd.env("PATH", "/usr/bin:/bin");
    }
    let output = cmd
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
    use crate::version;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    #[cfg(unix)]
    use std::process::Command;
    use std::thread;
    use tempfile::tempdir;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var_os(key);
            unsafe {
                env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.original.as_ref() {
                Some(value) => unsafe {
                    env::set_var(self.key, value);
                },
                None => unsafe {
                    env::remove_var(self.key);
                },
            }
        }
    }

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

    fn start_release_server(response_body: &'static str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind release listener");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let mut buffer = [0u8; 512];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        });
        (format!("http://{}", addr), handle)
    }

    fn start_status_server(
        status: &'static str,
        response_body: &'static str,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind status listener");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let mut buffer = [0u8; 512];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                status,
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        });
        (format!("http://{}", addr), handle)
    }

    fn start_bytes_server(
        status: &'static str,
        response_body: Vec<u8>,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind status listener");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let mut buffer = [0u8; 512];
            let _ = stream.read(&mut buffer);
            let header = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                status,
                response_body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&response_body);
        });
        (format!("http://{}", addr), handle)
    }

    #[cfg(unix)]
    fn build_release_archive(version: &str) -> Vec<u8> {
        let temp = tempdir().expect("tempdir");
        let release_dir = temp.path().join(format!("gralph-{}", version));
        fs::create_dir_all(&release_dir).expect("create release dir");
        let binary_path = release_dir.join("gralph");
        fs::write(&binary_path, "binary").expect("write binary");
        let archive_path = temp.path().join("gralph.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(temp.path())
            .arg(release_dir.file_name().expect("release dir name"))
            .status()
            .expect("run tar");
        assert!(status.success());
        fs::read(&archive_path).expect("read archive")
    }

    #[test]
    fn parse_version_accepts_v_prefix() {
        let version = Version::parse(version::VERSION_TAG).expect("version parsed");
        let expected = Version::parse(version::VERSION).expect("expected parsed");
        assert_eq!(version, expected);
    }

    #[test]
    fn parse_version_rejects_missing_patch() {
        let result = Version::parse("0.2");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn parse_version_rejects_empty_input() {
        let result = Version::parse(" ");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn parse_version_rejects_prerelease_and_build_metadata() {
        let prerelease = Version::parse("1.2.3-beta");
        assert!(matches!(prerelease, Err(UpdateError::InvalidVersion(_))));
        let build = Version::parse("1.2.3+build");
        assert!(matches!(build, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn parse_version_rejects_extra_segments() {
        let result = Version::parse("1.2.3.4");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn parse_release_tag_requires_tag_name() {
        let result = parse_release_tag("{}");
        assert!(matches!(result, Err(UpdateError::MissingTag)));
    }

    #[test]
    fn parse_release_tag_rejects_non_string_tag() {
        let numeric = parse_release_tag(r#"{ "tag_name": 42 }"#);
        assert!(matches!(numeric, Err(UpdateError::MissingTag)));
        let null_value = parse_release_tag(r#"{ "tag_name": null }"#);
        assert!(matches!(null_value, Err(UpdateError::MissingTag)));
    }

    #[test]
    fn parse_release_tag_accepts_valid_tag() {
        let body = format!(r#"{{ "tag_name": "{}" }}"#, version::VERSION_TAG);
        let tag = parse_release_tag(&body).expect("tag parsed");
        assert_eq!(tag, version::VERSION_TAG);
    }

    #[test]
    fn parse_release_tag_trims_whitespace() {
        let tag = parse_release_tag(r#"{ "tag_name": " v1.2.3 " }"#).expect("tag parsed");
        assert_eq!(tag, "v1.2.3");
    }

    #[test]
    fn parse_release_tag_rejects_blank_tag() {
        let result = parse_release_tag(r#"{ "tag_name": "   " }"#);
        assert!(matches!(result, Err(UpdateError::MissingTag)));
    }

    #[test]
    fn parse_release_tag_reports_json_error() {
        let result = parse_release_tag("not-json");
        assert!(matches!(result, Err(UpdateError::Json(_))));
    }

    #[test]
    fn check_for_update_returns_none_when_latest_is_current() {
        let _guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", version::VERSION_TAG);
        let result = check_for_update(version::VERSION).expect("check");
        assert!(result.is_none());
    }

    #[test]
    fn check_for_update_rejects_invalid_current_version() {
        let _lock = crate::test_support::env_lock();
        let _guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "v1.2.3");
        let result = check_for_update("not-a-version");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn check_for_update_reports_http_error() {
        let _lock = crate::test_support::env_lock();
        let (url, handle) = start_status_server("500 Internal Server Error", "boom");
        let _url_guard = EnvGuard::set("GRALPH_TEST_RELEASE_URL", &url);
        let _tag_guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "");
        let result = check_for_update("0.1.0");
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::Http(_))));
    }

    #[test]
    fn release_url_prefers_test_override() {
        let _lock = crate::test_support::env_lock();
        let _guard = EnvGuard::set("GRALPH_TEST_RELEASE_URL", "  http://localhost/test ");
        let resolved = release_url();
        assert_eq!(resolved, "http://localhost/test");
    }

    #[test]
    fn release_download_url_prefers_test_override() {
        let _lock = crate::test_support::env_lock();
        let _guard = EnvGuard::set(
            "GRALPH_TEST_RELEASE_DOWNLOAD_URL",
            "  http://localhost/releases ",
        );
        let resolved = release_download_url();
        assert_eq!(resolved, "http://localhost/releases");
    }

    #[test]
    fn fetch_latest_release_tag_reports_missing_tag_from_local_server() {
        let (url, handle) = start_release_server(r#"{ "name": "release" }"#);
        let _guard = EnvGuard::set("GRALPH_TEST_RELEASE_URL", &url);
        let result = fetch_latest_release_tag();
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::MissingTag)));
    }

    #[test]
    fn resolve_install_version_fetches_latest_from_test_release_url() {
        let _lock = crate::test_support::env_lock();
        let (url, handle) = start_release_server(r#"{ "tag_name": "v9.8.7" }"#);
        let _url_guard = EnvGuard::set("GRALPH_TEST_RELEASE_URL", &url);
        let _tag_guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "");
        let resolved = resolve_install_version("latest").expect("resolved");
        handle.join().expect("server thread");
        assert_eq!(resolved, "9.8.7");
    }

    #[test]
    fn detect_newer_version() {
        let latest = Version::parse(version::VERSION).expect("latest parsed");
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
        let version = normalize_version(version::VERSION).expect("normalized");
        assert_eq!(version, version::VERSION);
    }

    #[test]
    fn normalize_version_accepts_v_prefix() {
        let version = normalize_version("v1.2.3").expect("normalized");
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn normalize_version_trims_whitespace() {
        let version = normalize_version("  v1.2.3  ").expect("normalized");
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn normalize_version_rejects_whitespace_only() {
        let result = normalize_version("   ");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn normalize_version_rejects_invalid_input() {
        let result = normalize_version("1.2");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn resolve_install_version_accepts_concrete_version() {
        let resolved = resolve_install_version("v1.2.3").expect("resolved");
        assert_eq!(resolved, "1.2.3");
    }

    #[test]
    fn resolve_install_version_trims_latest_whitespace() {
        let _lock = crate::test_support::env_lock();
        let _guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "v4.5.6");
        let resolved = resolve_install_version("  latest  ").expect("resolved");
        assert_eq!(resolved, "4.5.6");
    }

    #[test]
    fn resolve_install_version_rejects_whitespace_only_input() {
        let result = resolve_install_version("   ");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn resolve_install_version_uses_gralph_version_env() {
        let _lock = crate::test_support::env_lock();
        let _guard = EnvGuard::set("GRALPH_VERSION", "v3.4.5");
        let raw = env::var("GRALPH_VERSION").expect("env set");
        let resolved = resolve_install_version(&raw).expect("resolved");
        assert_eq!(resolved, "3.4.5");
    }

    #[test]
    fn resolve_install_version_rejects_invalid_version() {
        let result = resolve_install_version("1.2");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn resolve_install_version_rejects_prerelease_and_build_metadata() {
        let prerelease = resolve_install_version("v1.2.3-beta");
        assert!(matches!(prerelease, Err(UpdateError::InvalidVersion(_))));
        let build = resolve_install_version("1.2.3+build");
        assert!(matches!(build, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn resolve_install_version_latest_uses_override() {
        let _guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "v2.0.0");
        let resolved = resolve_install_version("latest").expect("resolved");
        assert_eq!(resolved, "2.0.0");
    }

    #[test]
    fn resolve_install_version_latest_rejects_invalid_override() {
        let _lock = crate::test_support::env_lock();
        let _guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "v1.2");
        let result = resolve_install_version("latest");
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
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
    fn detect_platform_for_rejects_unknown_os() {
        let result = detect_platform_for("solaris", "x86_64");
        assert!(matches!(
            result,
            Err(UpdateError::UnsupportedPlatform(value)) if value == "solaris"
        ));
    }

    #[test]
    fn detect_platform_for_rejects_unknown_arch() {
        let result = detect_platform_for("linux", "mips");
        assert!(matches!(
            result,
            Err(UpdateError::UnsupportedPlatform(value)) if value == "linux-mips"
        ));
    }

    #[test]
    fn resolve_in_path_finds_binary() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let bin_path = temp.path().join("gralph");
        fs::write(&bin_path, "binary").expect("write");
        let _guard = PathGuard::set(Some(temp.path().as_os_str()));
        let resolved = resolve_in_path("gralph");
        assert_eq!(resolved.as_deref(), Some(bin_path.as_path()));
    }

    #[test]
    fn resolve_in_path_prefers_first_match() {
        let _lock = crate::test_support::env_lock();
        let first = tempdir().expect("tempdir");
        let second = tempdir().expect("tempdir");
        let first_path = first.path().join("gralph");
        let second_path = second.path().join("gralph");
        fs::write(&first_path, "first").expect("write");
        fs::write(&second_path, "second").expect("write");
        let joined = env::join_paths([first.path(), second.path()]).expect("join paths");
        let _guard = PathGuard::set(Some(joined.as_os_str()));
        let resolved = resolve_in_path("gralph");
        assert_eq!(resolved.as_deref(), Some(first_path.as_path()));
    }

    #[test]
    fn resolve_in_path_handles_missing_and_empty_path() {
        let _lock = crate::test_support::env_lock();
        {
            let _guard = PathGuard::set(None);
            assert!(resolve_in_path("gralph").is_none());
        }
        {
            let _guard = PathGuard::set(Some(OsStr::new("")));
            assert!(resolve_in_path("gralph").is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_rejects_invalid_or_empty_input() {
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "").expect("write empty");
        let empty_result = extract_archive(&archive_path, temp.path());
        assert!(matches!(empty_result, Err(UpdateError::CommandFailed(_))));

        fs::write(&archive_path, "not a tar").expect("write invalid");
        let invalid_result = extract_archive(&archive_path, temp.path());
        assert!(matches!(invalid_result, Err(UpdateError::CommandFailed(_))));
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_reports_tar_failure_message() {
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "not a tar").expect("write invalid");
        let result = extract_archive(&archive_path, temp.path());
        match result {
            Err(UpdateError::CommandFailed(message)) => {
                assert!(message.starts_with("Failed to extract archive:"));
            }
            other => panic!("expected command failed, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_uses_fallback_path_when_env_empty() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "not a tar").expect("write invalid");
        let _guard = PathGuard::set(Some(OsStr::new("")));
        let result = extract_archive(&archive_path, temp.path());
        assert!(matches!(result, Err(UpdateError::CommandFailed(_))));
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_uses_fallback_path_when_env_missing() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "not a tar").expect("write invalid");
        let _guard = PathGuard::set(None);
        let result = extract_archive(&archive_path, temp.path());
        assert!(matches!(result, Err(UpdateError::CommandFailed(_))));
    }

    #[test]
    fn extract_archive_reports_missing_archive() {
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("missing.tar.gz");
        let result = extract_archive(&archive_path, temp.path());
        assert!(matches!(result, Err(UpdateError::Io(_))));
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_reports_failure_for_missing_target_dir() {
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "not a tar").expect("write invalid");
        let missing_target = temp.path().join("missing");
        let result = extract_archive(&archive_path, &missing_target);
        assert!(matches!(result, Err(UpdateError::CommandFailed(_))));
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_reports_failure_when_target_is_file() {
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "not a tar").expect("write invalid");
        let target_file = temp.path().join("target-file");
        fs::write(&target_file, "not a dir").expect("write target file");
        let result = extract_archive(&archive_path, &target_file);
        assert!(matches!(result, Err(UpdateError::CommandFailed(_))));
    }

    #[cfg(unix)]
    #[test]
    fn extract_archive_reports_missing_tar_binary() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let archive_path = temp.path().join("gralph.tar.gz");
        fs::write(&archive_path, "not a tar").expect("write invalid");
        let empty_path = temp.path().join("bin");
        fs::create_dir_all(&empty_path).expect("create empty path");
        let _guard = PathGuard::set(Some(empty_path.as_os_str()));
        let result = extract_archive(&archive_path, temp.path());
        match result {
            Err(UpdateError::Io(err)) => {
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected missing tar error, got {other:?}"),
        }
    }

    #[test]
    fn download_release_reports_http_failure() {
        let temp = tempdir().expect("tempdir");
        let dest = temp.path().join("gralph.tar.gz");
        let (url, handle) = start_status_server("404 Not Found", "missing");
        let result = download_release(&url, &dest);
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::Http(_))));
    }

    #[test]
    fn download_release_writes_response_body() {
        let temp = tempdir().expect("tempdir");
        let dest = temp.path().join("gralph.tar.gz");
        let (url, handle) = start_status_server("200 OK", "fixture-bytes");
        let result = download_release(&url, &dest);
        handle.join().expect("server thread");
        result.expect("download");
        let contents = fs::read(&dest).expect("read");
        assert_eq!(contents, b"fixture-bytes");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn install_release_reports_missing_archive() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let install_dir = temp.path().join("install");
        let install_dir_value = install_dir.to_string_lossy().to_string();
        let _install_guard = EnvGuard::set("GRALPH_INSTALL_DIR", &install_dir_value);
        let _version_guard = EnvGuard::set("GRALPH_VERSION", "1.2.3");
        let (url, handle) = start_status_server("404 Not Found", "missing");
        let _download_guard = EnvGuard::set("GRALPH_TEST_RELEASE_DOWNLOAD_URL", &url);
        let result = install_release();
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::Http(_))));
    }

    #[cfg(unix)]
    #[test]
    fn install_release_reports_extract_failure() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let install_dir = temp.path().join("install");
        let install_dir_value = install_dir.to_string_lossy().to_string();
        let _install_guard = EnvGuard::set("GRALPH_INSTALL_DIR", &install_dir_value);
        let _version_guard = EnvGuard::set("GRALPH_VERSION", "1.2.3");
        let (url, handle) = start_status_server("200 OK", "not a tar");
        let _download_guard = EnvGuard::set("GRALPH_TEST_RELEASE_DOWNLOAD_URL", &url);
        let result = install_release();
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::CommandFailed(_))));
    }

    #[test]
    fn install_release_rejects_invalid_version_env() {
        let _lock = crate::test_support::env_lock();
        let _version_guard = EnvGuard::set("GRALPH_VERSION", "not-a-version");
        let result = install_release();
        assert!(matches!(result, Err(UpdateError::InvalidVersion(_))));
    }

    #[test]
    fn install_release_reports_missing_release_tag() {
        let _lock = crate::test_support::env_lock();
        let _version_guard = EnvGuard::set("GRALPH_VERSION", "latest");
        let _tag_guard = EnvGuard::set("GRALPH_TEST_LATEST_TAG", "");
        let (url, handle) = start_release_server(r#"{ "name": "release" }"#);
        let _url_guard = EnvGuard::set("GRALPH_TEST_RELEASE_URL", &url);
        let result = install_release();
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::MissingTag)));
    }

    #[test]
    fn install_release_reports_empty_archive() {
        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let install_dir = temp.path().join("install");
        let install_dir_value = install_dir.to_string_lossy().to_string();
        let _install_guard = EnvGuard::set("GRALPH_INSTALL_DIR", &install_dir_value);
        let _version_guard = EnvGuard::set("GRALPH_VERSION", "1.2.3");
        let (url, handle) = start_status_server("200 OK", "");
        let _download_guard = EnvGuard::set("GRALPH_TEST_RELEASE_DOWNLOAD_URL", &url);
        let result = install_release();
        handle.join().expect("server thread");
        assert!(matches!(result, Err(UpdateError::CommandFailed(_))));
    }

    #[cfg(unix)]
    #[test]
    fn install_release_reports_permission_denied_for_override_dir() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = crate::test_support::env_lock();
        let temp = tempdir().expect("tempdir");
        let install_dir = temp.path().join("install");
        fs::create_dir_all(&install_dir).expect("create install dir");
        let mut perms = fs::metadata(&install_dir).expect("metadata").permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&install_dir, perms).expect("set permissions");

        let archive_body = build_release_archive("1.2.3");
        let (url, handle) = start_bytes_server("200 OK", archive_body);
        let _download_guard = EnvGuard::set("GRALPH_TEST_RELEASE_DOWNLOAD_URL", &url);
        let _version_guard = EnvGuard::set("GRALPH_VERSION", "1.2.3");
        let _install_guard =
            EnvGuard::set("GRALPH_INSTALL_DIR", install_dir.to_string_lossy().as_ref());

        let result = install_release();
        handle.join().expect("server thread");

        let mut perms = fs::metadata(&install_dir).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&install_dir, perms).expect("reset permissions");

        assert!(matches!(result, Err(UpdateError::PermissionDenied(_))));
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

    #[test]
    fn install_binary_creates_nested_install_dir() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("gralph");
        fs::write(&source, "binary").expect("write");
        let install_dir = temp.path().join("nested/install/path");
        install_binary(&source, &install_dir).expect("install");
        let target = install_dir.join("gralph");
        assert!(target.is_file());
        assert!(install_dir.is_dir());
    }

    #[test]
    fn install_binary_reports_error_when_install_dir_is_file() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("gralph");
        fs::write(&source, "binary").expect("write");
        let install_dir = temp.path().join("install");
        fs::write(&install_dir, "not a dir").expect("write file");
        let result = install_binary(&source, &install_dir);
        assert!(matches!(result, Err(UpdateError::Io(_))));
    }

    #[test]
    fn permission_denied_message_mentions_install_dir_env() {
        let message = UpdateError::PermissionDenied("/restricted".to_string()).to_string();
        assert!(message.contains("/restricted"));
        assert!(message.contains("GRALPH_INSTALL_DIR"));
    }

    #[cfg(unix)]
    #[test]
    fn install_binary_reports_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("gralph");
        fs::write(&source, "binary").expect("write");
        let install_dir = temp.path().join("install");
        fs::create_dir_all(&install_dir).expect("create install dir");

        let mut perms = fs::metadata(&install_dir).expect("metadata").permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&install_dir, perms).expect("set permissions");

        let result = install_binary(&source, &install_dir);

        let mut perms = fs::metadata(&install_dir).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&install_dir, perms).expect("reset permissions");

        assert!(matches!(result, Err(UpdateError::PermissionDenied(_))));
    }

    #[cfg(unix)]
    #[test]
    fn install_binary_permission_denied_includes_install_dir() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("gralph");
        fs::write(&source, "binary").expect("write");
        let install_dir = temp.path().join("install");
        fs::create_dir_all(&install_dir).expect("create install dir");

        let mut perms = fs::metadata(&install_dir).expect("metadata").permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&install_dir, perms).expect("set permissions");

        let result = install_binary(&source, &install_dir);

        let mut perms = fs::metadata(&install_dir).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&install_dir, perms).expect("reset permissions");

        match result {
            Err(UpdateError::PermissionDenied(path)) => {
                assert_eq!(path, install_dir.display().to_string());
            }
            other => panic!("expected permission denied, got {other:?}"),
        }
    }
}
