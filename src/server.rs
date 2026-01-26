use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, options, post};
use axum::{Json, Router};
use serde_json::{json, Map, Value};
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
        let token = env::var("GRALPH_SERVER_TOKEN").ok().filter(|value| !value.is_empty());
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
            return Err(ServerError::InvalidConfig("port must be between 1 and 65535".to_string()));
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
            ServerError::InvalidConfig(message) => write!(f, "invalid server configuration: {}", message),
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
        .route("/", get(root_handler))
        .route("/status", get(status_handler))
        .route("/status/:name", get(status_name_handler))
        .route("/stop/:name", post(stop_handler))
        .route("/*path", options(options_handler))
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
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": format!("{}", error)}),
                cors_origin,
            );
        }
    };
    let enriched: Vec<Value> = sessions
        .into_iter()
        .map(enrich_session)
        .collect();
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
        Ok(None) => json_response(
            StatusCode::NOT_FOUND,
            json!({"error": format!("Session not found: {}", name)}),
            cors_origin,
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": format!("{}", error)}),
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
            return json_response(
                StatusCode::NOT_FOUND,
                json!({"error": format!("Session not found: {}", name)}),
                cors_origin,
            );
        }
        Err(error) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": format!("{}", error)}),
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
    if let Some(response) = check_auth(&headers, &state, cors_origin.as_deref()) {
        return response;
    }
    json_response(
        StatusCode::NOT_FOUND,
        json!({"error": format!("Unknown endpoint: {} {}", method, uri.path())}),
        cors_origin,
    )
}

fn check_auth(headers: &HeaderMap, state: &AppState, cors_origin: Option<&str>) -> Option<Response> {
    let Some(expected) = state.config.token.as_deref() else {
        return None;
    };
    let header = match headers.get(axum::http::header::AUTHORIZATION) {
        Some(value) => value,
        None => {
            return Some(json_response(
                StatusCode::UNAUTHORIZED,
                json!({"error": "Invalid or missing Bearer token"}),
                cors_origin.map(|value| value.to_string()),
            ))
        }
    };
    let header = match header.to_str() {
        Ok(value) => value,
        Err(_) => {
            return Some(json_response(
                StatusCode::UNAUTHORIZED,
                json!({"error": "Invalid or missing Bearer token"}),
                cors_origin.map(|value| value.to_string()),
            ))
        }
    };
    let Some(token) = header.strip_prefix("Bearer ") else {
        return Some(json_response(
            StatusCode::UNAUTHORIZED,
            json!({"error": "Invalid or missing Bearer token"}),
            cors_origin.map(|value| value.to_string()),
        ));
    };
    if token == expected {
        None
    } else {
        Some(json_response(
            StatusCode::UNAUTHORIZED,
            json!({"error": "Invalid or missing Bearer token"}),
            cors_origin.map(|value| value.to_string()),
        ))
    }
}

fn enrich_session(session: Value) -> Value {
    let mut map = match session.as_object() {
        Some(map) => map.clone(),
        None => Map::new(),
    };
    let dir = map.get("dir").and_then(|value| value.as_str()).unwrap_or("");
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
        let _ = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    }
}

fn json_response(status: StatusCode, body: Value, cors_origin: Option<String>) -> Response {
    let mut response = Json(body).into_response();
    *response.status_mut() = status;
    apply_cors(&mut response, cors_origin);
    response
}

fn resolve_cors_origin(headers: &HeaderMap, config: &ServerConfig) -> Option<String> {
    let origin = headers.get(axum::http::header::ORIGIN)?;
    let origin = origin.to_str().ok()?;

    if config.open {
        return Some("*".to_string());
    }

    match origin {
        "http://localhost" | "http://127.0.0.1" | "http://[::1]" => return Some(origin.to_string()),
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
        headers.insert(
            axum::http::header::VARY,
            HeaderValue::from_static("Origin"),
        );
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
    use reqwest::Client;
    use tokio::sync::oneshot;

    fn store_for_test(dir: &std::path::Path) -> StateStore {
        let state_dir = dir.join("state");
        let state_file = state_dir.join("state.json");
        let lock_file = state_dir.join("state.lock");
        StateStore::with_paths(state_dir, state_file, lock_file, std::time::Duration::from_secs(1))
    }

    async fn spawn_app(state: Arc<AppState>) -> (String, oneshot::Sender<()>) {
        let app = build_router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });
        (format!("http://{}", addr), shutdown_tx)
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
        let (base, shutdown) = spawn_app(state).await;
        let client = Client::new();

        let resp = client.get(format!("{}/status", base)).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body_text = resp.text().await.unwrap();
        let body: Value = serde_json::from_str(&body_text).unwrap();
        assert_eq!(body["error"], "Invalid or missing Bearer token");

        let resp = client
            .get(format!("{}/status", base))
            .bearer_auth("secret")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = shutdown.send(());
    }
}
