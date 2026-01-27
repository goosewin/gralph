use reqwest::blocking::Client;
use serde_json::Value;
use std::cmp::Ordering;
use std::fmt;
use std::time::Duration;

const RELEASE_URL: &str = "https://api.github.com/repos/goosewin/gralph/releases/latest";
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
    MissingTag,
    InvalidVersion(String),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::Http(err) => write!(f, "{}", err),
            UpdateError::Json(err) => write!(f, "{}", err),
            UpdateError::MissingTag => write!(f, "Latest release tag missing."),
            UpdateError::InvalidVersion(value) => {
                write!(f, "Invalid version string: {}", value)
            }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
