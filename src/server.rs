use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Map, Value, json};
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::core::count_remaining_tasks;
use crate::state::{StateError, StateStore};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
    pub open: bool,
    pub max_body_bytes: usize,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let host = env::var("GRALPH_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("GRALPH_SERVER_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(8080);
        let token = env::var("GRALPH_SERVER_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());
        let open = env::var("GRALPH_SERVER_OPEN")
            .ok()
            .map(|value| value == "true")
            .unwrap_or(false);
        let max_body_bytes = env::var("GRALPH_SERVER_MAX_BODY_BYTES")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(4096);

        Self {
            host,
            port,
            token,
            open,
            max_body_bytes,
        }
    }

    pub fn validate(&self) -> Result<(), ServerError> {
        if self.port == 0 {
            return Err(ServerError::InvalidConfig(
                "port must be between 1 and 65535".to_string(),
            ));
        }
        if !is_localhost(&self.host) && self.token.is_none() && !self.open {
            return Err(ServerError::InvalidConfig(format!(
                "token required when binding to non-localhost address ({})",
                self.host
            )));
        }
        Ok(())
    }

    pub fn addr(&self) -> Result<SocketAddr, ServerError> {
        let addr = format!("{}:{}", self.host, self.port);
        addr.parse::<SocketAddr>()
            .map_err(|err| ServerError::InvalidConfig(format!("invalid server address: {}", err)))
    }
}

#[derive(Debug)]
pub enum ServerError {
    InvalidConfig(String),
    Io(std::io::Error),
    State(StateError),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::InvalidConfig(message) => {
                write!(f, "invalid server configuration: {}", message)
            }
            ServerError::Io(error) => write!(f, "server io error: {}", error),
            ServerError::State(error) => write!(f, "server state error: {}", error),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        ServerError::Io(value)
    }
}

impl From<StateError> for ServerError {
    fn from(value: StateError) -> Self {
        ServerError::State(value)
    }
}

#[derive(Clone)]
struct AppState {
    config: ServerConfig,
    store: StateStore,
}

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    config.validate()?;
    let store = StateStore::new_from_env();
    store.init_state()?;
    let app_state = Arc::new(AppState { config, store });
    let app = build_router(app_state.clone());
    let listener = TcpListener::bind(app_state.config.addr()?).await?;
    axum::serve(listener, app).await.map_err(ServerError::Io)
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root_handler).options(options_handler))
        .route("/status", get(status_handler).options(options_handler))
        .route(
            "/status/:name",
            get(status_name_handler).options(options_handler),
        )
        .route("/stop/:name", post(stop_handler).options(options_handler))
        .fallback(fallback_handler)
        .with_state(state)
}

async fn options_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cors_origin = resolve_cors_origin(&headers, &state.config);
    let mut response = StatusCode::NO_CONTENT.into_response();
    apply_cors(&mut response, cors_origin);
    response
}

async fn root_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cors_origin = resolve_cors_origin(&headers, &state.config);
    if let Some(response) = check_auth(&headers, &state, cors_origin.as_deref()) {
        return response;
    }
    json_response(
        StatusCode::OK,
        json!({"status": "ok", "service": "gralph-server"}),
        cors_origin,
    )
}

async fn status_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cors_origin = resolve_cors_origin(&headers, &state.config);
    if let Some(response) = check_auth(&headers, &state, cors_origin.as_deref()) {
        return response;
    }
    let sessions = match state.store.list_sessions() {
        Ok(list) => list,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}", error),
                cors_origin,
            );
        }
    };
    let enriched: Vec<Value> = sessions.into_iter().map(enrich_session).collect();
    json_response(StatusCode::OK, json!({"sessions": enriched}), cors_origin)
}

async fn status_name_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let cors_origin = resolve_cors_origin(&headers, &state.config);
    if let Some(response) = check_auth(&headers, &state, cors_origin.as_deref()) {
        return response;
    }
    match state.store.get_session(&name) {
        Ok(Some(session)) => json_response(StatusCode::OK, enrich_session(session), cors_origin),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            format!("Session not found: {}", name),
            cors_origin,
        ),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{}", error),
            cors_origin,
        ),
    }
}

async fn stop_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let cors_origin = resolve_cors_origin(&headers, &state.config);
    if let Some(response) = check_auth(&headers, &state, cors_origin.as_deref()) {
        return response;
    }
    let session = match state.store.get_session(&name) {
        Ok(Some(session)) => session,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                format!("Session not found: {}", name),
                cors_origin,
            );
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}", error),
                cors_origin,
            );
        }
    };

    stop_session(&name, &session);
    let _ = state.store.set_session(&name, &[("status", "stopped")]);
    json_response(
        StatusCode::OK,
        json!({"success": true, "message": "Session stopped"}),
        cors_origin,
    )
}

async fn fallback_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let cors_origin = resolve_cors_origin(&headers, &state.config);
    if method == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        apply_cors(&mut response, cors_origin);
        return response;
    }
    if let Some(response) = check_auth(&headers, &state, cors_origin.as_deref()) {
        return response;
    }
    error_response(
        StatusCode::NOT_FOUND,
        format!("Unknown endpoint: {} {}", method, uri.path()),
        cors_origin,
    )
}

fn check_auth(
    headers: &HeaderMap,
    state: &AppState,
    cors_origin: Option<&str>,
) -> Option<Response> {
    let Some(expected) = state.config.token.as_deref() else {
        return None;
    };
    let header = match headers.get(axum::http::header::AUTHORIZATION) {
        Some(value) => value,
        None => return Some(unauthorized_response(cors_origin)),
    };
    let header = match header.to_str() {
        Ok(value) => value,
        Err(_) => return Some(unauthorized_response(cors_origin)),
    };
    let Some(token) = header.strip_prefix("Bearer ") else {
        return Some(unauthorized_response(cors_origin));
    };
    if token == expected {
        None
    } else {
        Some(unauthorized_response(cors_origin))
    }
}

fn enrich_session(session: Value) -> Value {
    let mut map = match session.as_object() {
        Some(map) => map.clone(),
        None => Map::new(),
    };
    let dir = map
        .get("dir")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let task_file = map
        .get("task_file")
        .and_then(|value| value.as_str())
        .unwrap_or("PRD.md");
    let mut status = map
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let pid = map.get("pid").and_then(|value| value.as_i64()).unwrap_or(0);
    let mut is_alive = false;
    if status == "running" && pid > 0 {
        if is_process_alive(pid) {
            is_alive = true;
        } else {
            status = "stale".to_string();
        }
    }

    let remaining = if dir.is_empty() {
        0
    } else {
        let path = PathBuf::from(dir).join(task_file);
        count_remaining_tasks(&path) as i64
    };

    map.insert(
        "current_remaining".to_string(),
        Value::Number(remaining.into()),
    );
    map.insert("is_alive".to_string(), Value::Bool(is_alive));
    map.insert("status".to_string(), Value::String(status));
    Value::Object(map)
}

fn stop_session(_name: &str, session: &Value) {
    let Some(map) = session.as_object() else {
        return;
    };
    let tmux_session = map
        .get("tmux_session")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();
    let pid = map.get("pid").and_then(|value| value.as_i64()).unwrap_or(0);

    if !tmux_session.is_empty() {
        let _ = std::process::Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg(tmux_session)
            .status();
    } else if pid > 0 {
        #[cfg(unix)]
        {
            let _ = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        }
        #[cfg(windows)]
        {
            // On Windows, use taskkill to terminate the process
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .status();
        }
    }
}

fn json_response(status: StatusCode, body: Value, cors_origin: Option<String>) -> Response {
    let mut response = Json(body).into_response();
    *response.status_mut() = status;
    apply_cors(&mut response, cors_origin);
    response
}

fn error_response(status: StatusCode, message: String, cors_origin: Option<String>) -> Response {
    json_response(status, json!({"error": message}), cors_origin)
}

fn unauthorized_response(cors_origin: Option<&str>) -> Response {
    json_response(
        StatusCode::UNAUTHORIZED,
        json!({"error": "Invalid or missing Bearer token"}),
        cors_origin.map(|value| value.to_string()),
    )
}

fn resolve_cors_origin(headers: &HeaderMap, config: &ServerConfig) -> Option<String> {
    let origin = headers.get(axum::http::header::ORIGIN)?;
    let origin = origin.to_str().ok()?;

    if config.open {
        return Some("*".to_string());
    }

    match origin {
        "http://localhost" | "http://127.0.0.1" | "http://[::1]" => {
            return Some(origin.to_string());
        }
        _ => {}
    }

    if !config.host.is_empty() && config.host != "0.0.0.0" && config.host != "::" {
        let expected = format!("http://{}", config.host);
        if origin == expected {
            return Some(origin.to_string());
        }
    }

    None
}

fn apply_cors(response: &mut Response, cors_origin: Option<String>) {
    let Some(origin) = cors_origin else {
        return;
    };
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&origin) {
        headers.insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    if origin != "*" {
        headers.insert(axum::http::header::VARY, HeaderValue::from_static("Origin"));
    }
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Authorization, Content-Type"),
    );
    headers.insert(
        axum::http::header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static("Content-Length, Content-Type"),
    );
    headers.insert(
        axum::http::header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
}

fn is_localhost(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn is_process_alive(pid: i64) -> bool {
    if pid <= 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, 0) };
        if result == 0 {
            return true;
        }
        let err = std::io::Error::last_os_error();
        return err.kind() == std::io::ErrorKind::PermissionDenied;
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use std::env;
    use std::fs;
    use std::sync::Mutex;
    use tower::util::ServiceExt;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    struct EnvSnapshot {
        keys: Vec<&'static str>,
        values: Vec<Option<std::ffi::OsString>>,
    }

    impl EnvSnapshot {
        fn new(keys: &[&'static str]) -> Self {
            let values = keys.iter().map(|key| env::var_os(key)).collect();
            Self {
                keys: keys.to_vec(),
                values,
            }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            for (key, original) in self.keys.iter().zip(self.values.iter()) {
                match original {
                    Some(value) => set_env(key, value),
                    None => remove_env(key),
                }
            }
        }
    }

    fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env(key: &str) {
        unsafe {
            env::remove_var(key);
        }
    }

    fn store_for_test(dir: &std::path::Path) -> StateStore {
        let state_dir = dir.join("state");
        let state_file = state_dir.join("state.json");
        let lock_file = state_dir.join("state.lock");
        StateStore::with_paths(
            state_dir,
            state_file,
            lock_file,
            std::time::Duration::from_secs(1),
        )
    }

    fn assert_cors_headers(headers: &HeaderMap, origin: &str) {
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some(origin)
        );
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_METHODS)
                .and_then(|value| value.to_str().ok()),
            Some("GET, POST, OPTIONS")
        );
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS)
                .and_then(|value| value.to_str().ok()),
            Some("Authorization, Content-Type")
        );
    }

    #[test]
    fn server_config_from_env_defaults_when_port_invalid() {
        let _guard = env_guard();
        let _snapshot = EnvSnapshot::new(&[
            "GRALPH_SERVER_HOST",
            "GRALPH_SERVER_PORT",
            "GRALPH_SERVER_TOKEN",
            "GRALPH_SERVER_OPEN",
            "GRALPH_SERVER_MAX_BODY_BYTES",
        ]);

        set_env("GRALPH_SERVER_HOST", "127.0.0.1");
        set_env("GRALPH_SERVER_PORT", "not-a-number");
        set_env("GRALPH_SERVER_TOKEN", "");
        set_env("GRALPH_SERVER_OPEN", "true");
        set_env("GRALPH_SERVER_MAX_BODY_BYTES", "9000");

        let config = ServerConfig::from_env();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.token, None);
        assert!(config.open);
        assert_eq!(config.max_body_bytes, 9000);
    }

    #[test]
    fn server_config_addr_rejects_invalid_host() {
        let config = ServerConfig {
            host: "bad host".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };

        let err = config.addr().unwrap_err();
        match err {
            ServerError::InvalidConfig(message) => {
                assert!(message.contains("invalid server address"));
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn server_config_validate_rejects_port_zero() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("token".to_string()),
            open: false,
            max_body_bytes: 4096,
        };

        let err = config.validate().unwrap_err();
        match err {
            ServerError::InvalidConfig(message) => {
                assert!(message.contains("port must be between 1 and 65535"));
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn server_config_validate_requires_token_for_non_localhost() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };

        let err = config.validate().unwrap_err();
        match err {
            ServerError::InvalidConfig(message) => {
                assert!(message.contains("token required"));
                assert!(message.contains("0.0.0.0"));
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn server_config_validate_allows_open_mode_without_token() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: None,
            open: true,
            max_body_bytes: 4096,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn server_config_validate_allows_localhost_without_token() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn server_config_validate_allows_non_localhost_with_token() {
        let config = ServerConfig {
            host: "example.com".to_string(),
            port: 8080,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn resolve_cors_origin_allows_open_mode() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: None,
            open: true,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "https://example.com".parse().unwrap(),
        );

        assert_eq!(
            resolve_cors_origin(&headers, &config),
            Some("*".to_string())
        );
    }

    #[test]
    fn resolve_cors_origin_allows_localhost_origins() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://localhost".parse().unwrap(),
        );

        assert_eq!(
            resolve_cors_origin(&headers, &config),
            Some("http://localhost".to_string())
        );
    }

    #[test]
    fn resolve_cors_origin_allows_host_match() {
        let config = ServerConfig {
            host: "example.com".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://example.com".parse().unwrap(),
        );

        assert_eq!(
            resolve_cors_origin(&headers, &config),
            Some("http://example.com".to_string())
        );
    }

    #[test]
    fn resolve_cors_origin_rejects_untrusted_origin_when_closed() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "https://example.com".parse().unwrap(),
        );

        assert_eq!(resolve_cors_origin(&headers, &config), None);
    }

    #[test]
    fn resolve_cors_origin_returns_none_without_origin_header() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let headers = HeaderMap::new();

        assert_eq!(resolve_cors_origin(&headers, &config), None);
    }

    #[test]
    fn resolve_cors_origin_returns_none_for_invalid_origin_header() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        let value = HeaderValue::from_bytes(b"http://example.com/\xFF").unwrap();
        headers.insert(axum::http::header::ORIGIN, value);

        assert_eq!(resolve_cors_origin(&headers, &config), None);
    }

    #[test]
    fn resolve_cors_origin_rejects_wildcard_host_origin() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://0.0.0.0".parse().unwrap(),
        );

        assert_eq!(resolve_cors_origin(&headers, &config), None);
    }

    #[test]
    fn resolve_cors_origin_rejects_origin_when_host_is_wildcard() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://example.com".parse().unwrap(),
        );

        assert_eq!(resolve_cors_origin(&headers, &config), None);
    }

    #[test]
    fn resolve_cors_origin_allows_explicit_ip_host_match() {
        let config = ServerConfig {
            host: "192.168.1.10".to_string(),
            port: 8080,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://192.168.1.10".parse().unwrap(),
        );

        assert_eq!(
            resolve_cors_origin(&headers, &config),
            Some("http://192.168.1.10".to_string())
        );
    }

    #[tokio::test]
    async fn check_auth_rejects_missing_header() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let headers = HeaderMap::new();

        let response = check_auth(&headers, &state, None).expect("missing header unauthorized");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn check_auth_rejects_invalid_scheme() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Basic secret".parse().unwrap(),
        );

        let response = check_auth(&headers, &state, None).expect("invalid scheme unauthorized");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn check_auth_rejects_bearer_without_token() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer"),
        );

        let response = check_auth(&headers, &state, None).expect("missing token unauthorized");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn check_auth_rejects_bearer_with_empty_token() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer "),
        );

        let response = check_auth(&headers, &state, None).expect("empty bearer token unauthorized");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn check_auth_rejects_invalid_header_encoding() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_bytes(b"Bearer \xFF").unwrap(),
        );

        let response =
            check_auth(&headers, &state, None).expect("invalid header encoding unauthorized");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn check_auth_rejects_wrong_token() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer wrong".parse().unwrap(),
        );

        let response = check_auth(&headers, &state, None).expect("wrong token unauthorized");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[test]
    fn check_auth_allows_valid_bearer_token() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let state = AppState {
            config: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                token: Some("secret".to_string()),
                open: false,
                max_body_bytes: 4096,
            },
            store,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer secret".parse().unwrap(),
        );

        let response = check_auth(&headers, &state, None);
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn status_endpoint_allows_requests_when_token_disabled() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert!(
            body.get("sessions")
                .and_then(|value| value.as_array())
                .is_some()
        );
    }

    #[tokio::test]
    async fn auth_required_for_status_endpoint() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn status_endpoint_rejects_malformed_authorization_header() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer\tsecret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");
    }

    #[tokio::test]
    async fn status_endpoint_returns_sessions_with_valid_token() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();
        store
            .set_session("alpha", &[("status", "running"), ("pid", "0")])
            .unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        let sessions = body
            .get("sessions")
            .and_then(|value| value.as_array())
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].get("name").and_then(|value| value.as_str()),
            Some("alpha")
        );
    }

    #[tokio::test]
    async fn status_endpoint_handles_incomplete_session_data() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();
        store.set_session("alpha", &[]).unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        let sessions = body
            .get("sessions")
            .and_then(|value| value.as_array())
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["name"], "alpha");
        assert_eq!(sessions[0]["status"], "unknown");
        assert_eq!(sessions[0]["current_remaining"], 0);
        assert_eq!(sessions[0]["is_alive"], false);
    }

    #[tokio::test]
    async fn status_endpoint_includes_cors_headers() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .header(axum::http::header::ORIGIN, "http://localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some("http://localhost")
        );
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_METHODS)
                .and_then(|value| value.to_str().ok()),
            Some("GET, POST, OPTIONS")
        );
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS)
                .and_then(|value| value.to_str().ok()),
            Some("Authorization, Content-Type")
        );
    }

    #[tokio::test]
    async fn options_handler_includes_cors_headers() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("OPTIONS")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .header(axum::http::header::ORIGIN, "http://localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let headers = response.headers();
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some("http://localhost")
        );
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_METHODS)
                .and_then(|value| value.to_str().ok()),
            Some("GET, POST, OPTIONS")
        );
    }

    #[tokio::test]
    async fn options_handler_allows_wildcard_origin_in_open_mode() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 0,
            token: None,
            open: true,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("OPTIONS")
                    .header(axum::http::header::ORIGIN, "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let headers = response.headers();
        assert_cors_headers(headers, "*");
        assert!(headers.get(axum::http::header::VARY).is_none());
    }

    #[tokio::test]
    async fn root_handler_returns_ok_with_cors_headers() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .header(axum::http::header::ORIGIN, "http://localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(
            headers
                .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|value| value.to_str().ok()),
            Some("http://localhost")
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["status"], "ok");
    }

    #[test]
    fn apply_cors_allows_open_origin_without_vary() {
        let mut response = StatusCode::NO_CONTENT.into_response();
        apply_cors(&mut response, Some("*".to_string()));

        let headers = response.headers();
        assert_cors_headers(headers, "*");
        assert!(headers.get(axum::http::header::VARY).is_none());
    }

    #[test]
    fn apply_cors_sets_vary_for_specific_origin() {
        let mut response = StatusCode::NO_CONTENT.into_response();
        apply_cors(&mut response, Some("http://example.com".to_string()));

        let headers = response.headers();
        assert_cors_headers(headers, "http://example.com");
        assert_eq!(
            headers
                .get(axum::http::header::VARY)
                .and_then(|value| value.to_str().ok()),
            Some("Origin")
        );
    }

    fn dead_pid() -> i64 {
        #[cfg(unix)]
        {
            let mut child = std::process::Command::new("true").spawn().unwrap();
            let pid = child.id() as i64;
            let _ = child.wait();
            pid
        }
        #[cfg(not(unix))]
        {
            1
        }
    }

    #[tokio::test]
    async fn fallback_handler_returns_not_found_for_unknown_path() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/missing")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Unknown endpoint: GET /missing");
    }

    #[tokio::test]
    async fn status_name_unknown_returns_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status/missing")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Session not found: missing");
    }

    #[tokio::test]
    async fn status_name_error_includes_cors_headers() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status/missing")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .header(axum::http::header::ORIGIN, "http://localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_cors_headers(response.headers(), "http://localhost");
    }

    #[tokio::test]
    async fn status_name_handler_returns_error_when_state_unreadable() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().join("state");
        let state_file = state_dir.join("state.json");
        fs::create_dir_all(&state_dir).unwrap();
        fs::create_dir_all(&state_file).unwrap();
        let store = store_for_test(temp.path());

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status/alpha")
                    .method("GET")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert!(body["error"].as_str().unwrap().contains("state io error"));
    }

    #[tokio::test]
    async fn stop_endpoint_marks_session_stopped() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();
        store
            .set_session("alpha", &[("status", "running"), ("pid", "0")])
            .unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/alpha")
                    .method("POST")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Session stopped");

        let session = state.store.get_session("alpha").unwrap().unwrap();
        assert_eq!(
            session.get("status").and_then(|v| v.as_str()),
            Some("stopped")
        );
    }

    #[tokio::test]
    async fn stop_endpoint_marks_tmux_session_stopped() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();
        store
            .set_session(
                "alpha",
                &[
                    ("status", "running"),
                    ("pid", "123"),
                    ("tmux_session", "abc"),
                ],
            )
            .unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/alpha")
                    .method("POST")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let session = state.store.get_session("alpha").unwrap().unwrap();
        assert_eq!(
            session.get("status").and_then(|v| v.as_str()),
            Some("stopped")
        );
    }

    #[tokio::test]
    async fn stop_endpoint_unknown_session_returns_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/missing")
                    .method("POST")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Session not found: missing");
    }

    #[tokio::test]
    async fn stop_endpoint_unknown_session_allows_open_access() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: None,
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/missing")
                    .method("POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], "Session not found: missing");
    }

    #[tokio::test]
    async fn stop_endpoint_missing_session_sets_open_cors_headers() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 0,
            token: None,
            open: true,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/missing")
                    .method("POST")
                    .header(axum::http::header::ORIGIN, "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let headers = response.headers();
        assert_cors_headers(headers, "*");
        assert!(headers.get(axum::http::header::VARY).is_none());
    }

    #[tokio::test]
    async fn stop_endpoint_error_includes_cors_headers() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/missing")
                    .method("POST")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .header(axum::http::header::ORIGIN, "http://localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_cors_headers(response.headers(), "http://localhost");
    }

    #[tokio::test]
    async fn stop_handler_returns_error_when_state_unreadable() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().join("state");
        let state_file = state_dir.join("state.json");
        fs::create_dir_all(&state_dir).unwrap();
        fs::create_dir_all(&state_file).unwrap();
        let store = store_for_test(temp.path());

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/alpha")
                    .method("POST")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert!(body["error"].as_str().unwrap().contains("state io error"));
    }

    #[tokio::test]
    async fn stop_handler_returns_error_when_lock_file_is_directory() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().join("state");
        let lock_file = state_dir.join("state.lock");
        fs::create_dir_all(&state_dir).unwrap();
        fs::create_dir_all(&lock_file).unwrap();
        let store = store_for_test(temp.path());

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stop/alpha")
                    .method("POST")
                    .header(axum::http::header::AUTHORIZATION, "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert!(body["error"].as_str().unwrap().contains("state io error"));
    }

    #[tokio::test]
    async fn error_response_has_expected_schema() {
        let temp = tempfile::tempdir().unwrap();
        let store = store_for_test(temp.path());
        store.init_state().unwrap();

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            token: Some("secret".to_string()),
            open: false,
            max_body_bytes: 4096,
        };
        let state = Arc::new(AppState { config, store });
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        let object = body.as_object().expect("error response object");
        assert_eq!(object.len(), 1);
        assert!(
            object
                .get("error")
                .and_then(|value| value.as_str())
                .is_some()
        );
    }

    #[test]
    fn enrich_session_marks_stale_when_pid_is_dead() {
        let pid = dead_pid();

        let session = json!({
            "name": "alpha",
            "status": "running",
            "pid": pid,
            "dir": "",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["status"], "stale");
        assert_eq!(enriched["is_alive"], false);
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[cfg(unix)]
    #[test]
    fn enrich_session_keeps_running_when_pid_is_alive() {
        let mut child = std::process::Command::new("sleep").arg("2").spawn().unwrap();
        let pid = child.id() as i64;

        let session = json!({
            "name": "alpha",
            "status": "running",
            "pid": pid,
            "dir": "",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["status"], "running");
        assert_eq!(enriched["is_alive"], true);
        assert_eq!(enriched["current_remaining"], 0);

        let _ = child.wait();
    }

    #[test]
    fn enrich_session_marks_stale_when_dir_missing() {
        let session = json!({
            "name": "alpha",
            "status": "running",
            "pid": dead_pid(),
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["status"], "stale");
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[test]
    fn enrich_session_defaults_task_file_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let task_path = temp.path().join("PRD.md");
        fs::write(&task_path, "- [ ] First\n- [x] Done\n").unwrap();

        let session = json!({
            "name": "alpha",
            "status": "idle",
            "pid": 0,
            "dir": temp.path().to_string_lossy(),
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 1);
    }

    #[test]
    fn enrich_session_returns_zero_when_task_file_missing() {
        let temp = tempfile::tempdir().unwrap();

        let session = json!({
            "name": "alpha",
            "status": "idle",
            "pid": 0,
            "dir": temp.path().to_string_lossy(),
            "task_file": "missing.md",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[test]
    fn enrich_session_defaults_to_zero_when_default_task_file_missing() {
        let temp = tempfile::tempdir().unwrap();

        let session = json!({
            "name": "alpha",
            "status": "idle",
            "pid": 0,
            "dir": temp.path().to_string_lossy(),
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[test]
    fn enrich_session_returns_zero_when_dir_missing_on_disk() {
        let temp = tempfile::tempdir().unwrap();
        let missing_dir = temp.path().join("missing-dir");

        let session = json!({
            "name": "alpha",
            "status": "idle",
            "pid": 0,
            "dir": missing_dir.to_string_lossy(),
            "task_file": "tasks.md",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[test]
    fn enrich_session_marks_stale_when_pid_dead_and_task_file_missing() {
        let temp = tempfile::tempdir().unwrap();

        let session = json!({
            "name": "alpha",
            "status": "running",
            "pid": dead_pid(),
            "dir": temp.path().to_string_lossy(),
            "task_file": "missing.md",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["status"], "stale");
        assert_eq!(enriched["is_alive"], false);
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[cfg(unix)]
    #[test]
    fn enrich_session_returns_zero_when_task_file_unreadable() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let task_path = temp.path().join("tasks.md");
        fs::write(&task_path, "- [ ] First\n- [x] Done\n").unwrap();
        let mut permissions = fs::metadata(&task_path).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&task_path, permissions).unwrap();

        let session = json!({
            "name": "alpha",
            "status": "idle",
            "pid": 0,
            "dir": temp.path().to_string_lossy(),
            "task_file": "tasks.md",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 0);
    }

    #[test]
    fn enrich_session_uses_task_file_for_remaining() {
        let temp = tempfile::tempdir().unwrap();
        let task_path = temp.path().join("tasks.md");
        fs::write(&task_path, "- [ ] First\n- [x] Done\n").unwrap();

        let session = json!({
            "name": "alpha",
            "status": "running",
            "pid": 0,
            "dir": temp.path().to_string_lossy(),
            "task_file": "tasks.md",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 1);
    }

    #[test]
    fn enrich_session_counts_multiple_remaining_tasks() {
        let temp = tempfile::tempdir().unwrap();
        let task_path = temp.path().join("tasks.md");
        fs::write(&task_path, "- [ ] One\n- [ ] Two\n- [x] Done\n").unwrap();

        let session = json!({
            "name": "alpha",
            "status": "idle",
            "pid": 0,
            "dir": temp.path().to_string_lossy(),
            "task_file": "tasks.md",
        });
        let enriched = enrich_session(session);
        assert_eq!(enriched["current_remaining"], 2);
    }

    #[test]
    fn enrich_session_handles_non_object_input() {
        let enriched = enrich_session(json!("not-a-map"));
        assert_eq!(
            enriched.get("current_remaining").and_then(|v| v.as_i64()),
            Some(0)
        );
        assert_eq!(
            enriched.get("is_alive").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            enriched.get("status").and_then(|v| v.as_str()),
            Some("unknown")
        );
    }
}
