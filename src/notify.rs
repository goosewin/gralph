use reqwest::blocking::Client;
use serde_json::json;
use std::error::Error;
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookType {
    Discord,
    Slack,
    Generic,
}

#[derive(Debug)]
pub enum NotifyError {
    InvalidInput(String),
    Http(reqwest::Error),
    HttpStatus(u16),
    Json(serde_json::Error),
}

pub trait Notifier: Send + Sync {
    fn notify_complete(
        &self,
        session_name: &str,
        webhook_url: &str,
        project_dir: Option<&str>,
        iterations: Option<u32>,
        duration_secs: Option<u64>,
        timeout_secs: Option<u64>,
    ) -> Result<(), NotifyError>;

    fn notify_failed(
        &self,
        session_name: &str,
        webhook_url: &str,
        failure_reason: Option<&str>,
        project_dir: Option<&str>,
        iterations: Option<u32>,
        max_iterations: Option<u32>,
        remaining_tasks: Option<u32>,
        duration_secs: Option<u64>,
        timeout_secs: Option<u64>,
    ) -> Result<(), NotifyError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealNotifier;

impl Notifier for RealNotifier {
    fn notify_complete(
        &self,
        session_name: &str,
        webhook_url: &str,
        project_dir: Option<&str>,
        iterations: Option<u32>,
        duration_secs: Option<u64>,
        timeout_secs: Option<u64>,
    ) -> Result<(), NotifyError> {
        notify_complete(
            session_name,
            webhook_url,
            project_dir,
            iterations,
            duration_secs,
            timeout_secs,
        )
    }

    fn notify_failed(
        &self,
        session_name: &str,
        webhook_url: &str,
        failure_reason: Option<&str>,
        project_dir: Option<&str>,
        iterations: Option<u32>,
        max_iterations: Option<u32>,
        remaining_tasks: Option<u32>,
        duration_secs: Option<u64>,
        timeout_secs: Option<u64>,
    ) -> Result<(), NotifyError> {
        notify_failed(
            session_name,
            webhook_url,
            failure_reason,
            project_dir,
            iterations,
            max_iterations,
            remaining_tasks,
            duration_secs,
            timeout_secs,
        )
    }
}

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotifyError::InvalidInput(message) => write!(f, "invalid input: {}", message),
            NotifyError::Http(err) => write!(f, "http error: {}", err),
            NotifyError::HttpStatus(code) => write!(f, "webhook returned HTTP {}", code),
            NotifyError::Json(err) => write!(f, "json error: {}", err),
        }
    }
}

impl Error for NotifyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotifyError::Http(err) => Some(err),
            NotifyError::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for NotifyError {
    fn from(error: reqwest::Error) -> Self {
        NotifyError::Http(error)
    }
}

impl From<serde_json::Error> for NotifyError {
    fn from(error: serde_json::Error) -> Self {
        NotifyError::Json(error)
    }
}

pub fn detect_webhook_type(url: &str) -> WebhookType {
    let lower = url.to_lowercase();
    if lower.contains("discord.com/api/webhooks") || lower.contains("discordapp.com/api/webhooks") {
        WebhookType::Discord
    } else if lower.contains("hooks.slack.com") {
        WebhookType::Slack
    } else {
        WebhookType::Generic
    }
}

pub fn notify_complete(
    session_name: &str,
    webhook_url: &str,
    project_dir: Option<&str>,
    iterations: Option<u32>,
    duration_secs: Option<u64>,
    timeout_secs: Option<u64>,
) -> Result<(), NotifyError> {
    if session_name.trim().is_empty() {
        return Err(NotifyError::InvalidInput(
            "session name is required".to_string(),
        ));
    }
    if webhook_url.trim().is_empty() {
        return Err(NotifyError::InvalidInput(
            "webhook url is required".to_string(),
        ));
    }

    let project_dir = project_dir.unwrap_or("unknown");
    let iterations = iterations
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let duration_str = format_duration(duration_secs);
    let timestamp = timestamp_iso8601();
    let webhook_type = detect_webhook_type(webhook_url);
    let payload = match webhook_type {
        WebhookType::Discord => format_discord_complete(
            session_name,
            project_dir,
            &iterations,
            &duration_str,
            &timestamp,
        ),
        WebhookType::Slack => format_slack_complete(
            session_name,
            project_dir,
            &iterations,
            &duration_str,
            &timestamp,
        ),
        WebhookType::Generic => format_generic_complete(
            session_name,
            project_dir,
            &iterations,
            &duration_str,
            &timestamp,
        ),
    }?;

    send_webhook(webhook_url, &payload, timeout_secs)
}

pub fn notify_failed(
    session_name: &str,
    webhook_url: &str,
    failure_reason: Option<&str>,
    project_dir: Option<&str>,
    iterations: Option<u32>,
    max_iterations: Option<u32>,
    remaining_tasks: Option<u32>,
    duration_secs: Option<u64>,
    timeout_secs: Option<u64>,
) -> Result<(), NotifyError> {
    if session_name.trim().is_empty() {
        return Err(NotifyError::InvalidInput(
            "session name is required".to_string(),
        ));
    }
    if webhook_url.trim().is_empty() {
        return Err(NotifyError::InvalidInput(
            "webhook url is required".to_string(),
        ));
    }

    let failure_reason = failure_reason.unwrap_or("unknown");
    let project_dir = project_dir.unwrap_or("unknown");
    let iterations = iterations
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let max_iterations = max_iterations
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let remaining_tasks = remaining_tasks
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let duration_str = format_duration(duration_secs);
    let timestamp = timestamp_iso8601();

    let webhook_type = detect_webhook_type(webhook_url);
    let payload = match webhook_type {
        WebhookType::Discord => format_discord_failed(
            session_name,
            project_dir,
            failure_reason,
            &iterations,
            &max_iterations,
            &remaining_tasks,
            &duration_str,
            &timestamp,
        ),
        WebhookType::Slack => format_slack_failed(
            session_name,
            project_dir,
            failure_reason,
            &iterations,
            &max_iterations,
            &remaining_tasks,
            &duration_str,
            &timestamp,
        ),
        WebhookType::Generic => format_generic_failed(
            session_name,
            project_dir,
            failure_reason,
            &iterations,
            &max_iterations,
            &remaining_tasks,
            &duration_str,
            &timestamp,
        ),
    }?;

    send_webhook(webhook_url, &payload, timeout_secs)
}

pub fn send_webhook(
    url: &str,
    payload: &str,
    timeout_secs: Option<u64>,
) -> Result<(), NotifyError> {
    if url.trim().is_empty() {
        return Err(NotifyError::InvalidInput(
            "webhook url is required".to_string(),
        ));
    }
    if payload.trim().is_empty() {
        return Err(NotifyError::InvalidInput("payload is required".to_string()));
    }

    let timeout = timeout_secs.filter(|value| *value > 0).unwrap_or(30);
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(NotifyError::HttpStatus(response.status().as_u16()))
    }
}

const CLI_LABEL: &str = "Gralph CLI";

fn emphasized_session(session_name: &str, marker: &str) -> String {
    format!("{marker}{session_name}{marker}")
}

fn format_complete_description(session_name: &str, marker: &str) -> String {
    format!(
        "Session {} has finished all tasks successfully.",
        emphasized_session(session_name, marker)
    )
}

fn format_failure_description(session_name: &str, failure_reason: &str, marker: &str) -> String {
    let emphasized = emphasized_session(session_name, marker);
    match failure_reason {
        "max_iterations" => format!("Session {} hit maximum iterations limit.", emphasized),
        "error" => format!("Session {} encountered an error.", emphasized),
        "manual_stop" => format!("Session {} was manually stopped.", emphasized),
        _ => format!("Session {} failed: {}", emphasized, failure_reason),
    }
}

fn to_pretty_json(payload: serde_json::Value) -> Result<String, NotifyError> {
    Ok(serde_json::to_string_pretty(&payload)?)
}

fn discord_footer() -> serde_json::Value {
    json!({
        "text": CLI_LABEL
    })
}

fn discord_field(name: &str, value: impl Into<String>, inline: bool) -> serde_json::Value {
    let value = value.into();
    json!({
        "name": name,
        "value": value,
        "inline": inline
    })
}

fn discord_embed(
    title: &str,
    description: String,
    color: u32,
    fields: Vec<serde_json::Value>,
    timestamp: &str,
) -> serde_json::Value {
    json!({
        "title": title,
        "description": description,
        "color": color,
        "fields": fields,
        "footer": discord_footer(),
        "timestamp": timestamp
    })
}

fn slack_header(text: &str) -> serde_json::Value {
    json!({
        "type": "header",
        "text": {
            "type": "plain_text",
            "text": text,
            "emoji": true
        }
    })
}

fn slack_section_text(text: String) -> serde_json::Value {
    json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": text
        }
    })
}

fn slack_fields_block(fields: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "type": "section",
        "fields": fields
    })
}

fn slack_field(label: &str, value: impl AsRef<str>) -> serde_json::Value {
    let value = value.as_ref();
    json!({
        "type": "mrkdwn",
        "text": format!("*{}:*\n{}", label, value)
    })
}

fn slack_project_field(project_dir: &str) -> serde_json::Value {
    slack_field("Project", format!("`{}`", project_dir))
}

fn slack_context(timestamp: &str) -> serde_json::Value {
    json!({
        "type": "context",
        "elements": [{
            "type": "mrkdwn",
            "text": format!("{} • {}", CLI_LABEL, timestamp)
        }]
    })
}

fn slack_attachment(color: &str, blocks: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "color": color,
        "blocks": blocks
    })
}

fn build_generic_payload(
    event: &str,
    status: &str,
    session_name: &str,
    project_dir: &str,
    reason: Option<&str>,
    iterations: &str,
    max_iterations: Option<&str>,
    remaining_tasks: Option<&str>,
    duration_str: &str,
    timestamp: &str,
    message: String,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("event".to_string(), json!(event));
    payload.insert("status".to_string(), json!(status));
    payload.insert("session".to_string(), json!(session_name));
    payload.insert("project".to_string(), json!(project_dir));
    if let Some(reason) = reason {
        payload.insert("reason".to_string(), json!(reason));
    }
    payload.insert("iterations".to_string(), json!(iterations));
    if let Some(max_iterations) = max_iterations {
        payload.insert("max_iterations".to_string(), json!(max_iterations));
    }
    if let Some(remaining_tasks) = remaining_tasks {
        payload.insert("remaining_tasks".to_string(), json!(remaining_tasks));
    }
    payload.insert("duration".to_string(), json!(duration_str));
    payload.insert("timestamp".to_string(), json!(timestamp));
    payload.insert("message".to_string(), json!(message));
    serde_json::Value::Object(payload)
}

fn format_discord_complete(
    session_name: &str,
    project_dir: &str,
    iterations: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let fields = vec![
        discord_field("Project", format!("`{}`", project_dir), false),
        discord_field("Iterations", iterations, true),
        discord_field("Duration", duration_str, true),
    ];
    let embed = discord_embed(
        "✅ Gralph Complete",
        format_complete_description(session_name, "**"),
        5763719,
        fields,
        timestamp,
    );
    let payload = json!({
        "embeds": [embed]
    });
    to_pretty_json(payload)
}

fn format_slack_complete(
    session_name: &str,
    project_dir: &str,
    iterations: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let fields = vec![
        slack_project_field(project_dir),
        slack_field("Iterations", iterations),
        slack_field("Duration", duration_str),
    ];
    let blocks = vec![
        slack_header("✅ Gralph Complete"),
        slack_section_text(format_complete_description(session_name, "*")),
        slack_fields_block(fields),
        slack_context(timestamp),
    ];
    let payload = json!({
        "attachments": [slack_attachment("#57F287", blocks)]
    });
    to_pretty_json(payload)
}

fn format_slack_failed(
    session_name: &str,
    project_dir: &str,
    failure_reason: &str,
    iterations: &str,
    max_iterations: &str,
    remaining_tasks: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let description = format_failure_description(session_name, failure_reason, "*");
    let fields = vec![
        slack_project_field(project_dir),
        slack_field("Reason", failure_reason),
        slack_field("Iterations", format!("{}/{}", iterations, max_iterations)),
        slack_field("Remaining Tasks", remaining_tasks),
        slack_field("Duration", duration_str),
    ];
    let blocks = vec![
        slack_header("❌ Gralph Failed"),
        slack_section_text(description),
        slack_fields_block(fields),
        slack_context(timestamp),
    ];
    let payload = json!({
        "attachments": [slack_attachment("#ED4245", blocks)]
    });
    to_pretty_json(payload)
}

fn format_discord_failed(
    session_name: &str,
    project_dir: &str,
    failure_reason: &str,
    iterations: &str,
    max_iterations: &str,
    remaining_tasks: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let description = format_failure_description(session_name, failure_reason, "**");
    let fields = vec![
        discord_field("Project", format!("`{}`", project_dir), false),
        discord_field("Reason", failure_reason, true),
        discord_field(
            "Iterations",
            format!("{}/{}", iterations, max_iterations),
            true,
        ),
        discord_field("Remaining Tasks", remaining_tasks, true),
        discord_field("Duration", duration_str, true),
    ];
    let embed = discord_embed("❌ Gralph Failed", description, 15548997, fields, timestamp);
    let payload = json!({
        "embeds": [embed]
    });
    to_pretty_json(payload)
}

fn format_generic_complete(
    session_name: &str,
    project_dir: &str,
    iterations: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let message = format!(
        "Gralph loop '{}' completed successfully after {} iterations ({})",
        session_name, iterations, duration_str
    );
    let payload = build_generic_payload(
        "complete",
        "success",
        session_name,
        project_dir,
        None,
        iterations,
        None,
        None,
        duration_str,
        timestamp,
        message,
    );
    to_pretty_json(payload)
}

fn format_generic_failed(
    session_name: &str,
    project_dir: &str,
    failure_reason: &str,
    iterations: &str,
    max_iterations: &str,
    remaining_tasks: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let message = match failure_reason {
        "max_iterations" => format!(
            "Gralph loop '{}' failed: hit max iterations ({}/{}) with {} tasks remaining",
            session_name, iterations, max_iterations, remaining_tasks
        ),
        "error" => format!(
            "Gralph loop '{}' failed due to an error after {} iterations",
            session_name, iterations
        ),
        "manual_stop" => format!(
            "Gralph loop '{}' was manually stopped after {} iterations with {} tasks remaining",
            session_name, iterations, remaining_tasks
        ),
        _ => format!(
            "Gralph loop '{}' failed: {} after {} iterations",
            session_name, failure_reason, iterations
        ),
    };
    let payload = build_generic_payload(
        "failed",
        "failure",
        session_name,
        project_dir,
        Some(failure_reason),
        iterations,
        Some(max_iterations),
        Some(remaining_tasks),
        duration_str,
        timestamp,
        message,
    );
    to_pretty_json(payload)
}

fn format_duration(duration_secs: Option<u64>) -> String {
    let Some(total) = duration_secs else {
        return "unknown".to_string();
    };
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

fn timestamp_iso8601() -> String {
    chrono::Local::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    struct CapturedRequest {
        method: String,
        path: String,
        headers: HashMap<String, String>,
        body: String,
    }

    fn read_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut buffer = Vec::new();
        let mut temp = [0u8; 1024];
        let header_end = loop {
            let read = stream.read(&mut temp).unwrap_or(0);
            if read == 0 {
                break None;
            }
            buffer.extend_from_slice(&temp[..read]);
            if let Some(pos) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                break Some(pos + 4);
            }
        };

        let header_end = header_end.unwrap_or(buffer.len());
        let (header_bytes, body_bytes) = buffer.split_at(header_end);
        let header_text = String::from_utf8_lossy(header_bytes);
        let mut lines = header_text.lines();
        let request_line = lines.next().unwrap_or_default();
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().unwrap_or_default().to_string();
        let path = request_parts.next().unwrap_or_default().to_string();

        let mut headers = HashMap::new();
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_lowercase(), value.trim().to_string());
            }
        }

        let content_length = headers
            .get("content-length")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);

        let mut full_body = body_bytes.to_vec();
        if full_body.len() < content_length {
            let mut remaining = vec![0u8; content_length - full_body.len()];
            stream.read_exact(&mut remaining).unwrap();
            full_body.extend_from_slice(&remaining);
        }

        let body =
            String::from_utf8_lossy(&full_body[..content_length.min(full_body.len())]).to_string();

        CapturedRequest {
            method,
            path,
            headers,
            body,
        }
    }

    fn start_test_server(
        status_line: &'static str,
        response_body: &'static str,
    ) -> (
        String,
        Arc<Mutex<Option<CapturedRequest>>>,
        thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        let captured = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let request = read_request(&mut stream);
            *captured_clone.lock().unwrap() = Some(request);

            let body_bytes = response_body.as_bytes();
            let response = format!(
                "{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                status_line,
                body_bytes.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            if !body_bytes.is_empty() {
                stream.write_all(body_bytes).unwrap();
            }
        });

        (format!("http://{}", addr), captured, handle)
    }

    fn start_hanging_server(delay: Duration) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let _request = read_request(&mut stream);
            thread::sleep(delay);
        });

        (format!("http://{}", addr), handle)
    }

    #[test]
    fn detect_webhook_type_matches() {
        assert_eq!(
            detect_webhook_type("https://discord.com/api/webhooks/123"),
            WebhookType::Discord
        );
        assert_eq!(
            detect_webhook_type("HTTPS://DISCORD.COM/API/WEBHOOKS/123"),
            WebhookType::Discord
        );
        assert_eq!(
            detect_webhook_type("https://discordapp.com/api/webhooks/123"),
            WebhookType::Discord
        );
        assert_eq!(
            detect_webhook_type("https://hooks.slack.com/services/123"),
            WebhookType::Slack
        );
        assert_eq!(
            detect_webhook_type("https://Hooks.Slack.com/services/123"),
            WebhookType::Slack
        );
        assert_eq!(
            detect_webhook_type("https://example.com/webhook"),
            WebhookType::Generic
        );
    }

    #[test]
    fn detect_webhook_type_handles_variants_and_non_matches() {
        assert_eq!(
            detect_webhook_type("https://discord.com/api/webhooks/123?wait=true"),
            WebhookType::Discord
        );
        assert_eq!(
            detect_webhook_type("https://hooks.slack.com/services/123?mode=test"),
            WebhookType::Slack
        );
        assert_eq!(
            detect_webhook_type("https://discord.com/api/users/123"),
            WebhookType::Generic
        );
        assert_eq!(
            detect_webhook_type("https://slack.com/hooks/123"),
            WebhookType::Generic
        );
    }

    #[test]
    fn detect_webhook_type_handles_mixed_case_query_params() {
        assert_eq!(
            detect_webhook_type("https://Discord.com/API/Webhooks/123?Wait=true"),
            WebhookType::Discord
        );
        assert_eq!(
            detect_webhook_type("https://Hooks.Slack.com/services/123?Mode=Test"),
            WebhookType::Slack
        );
    }

    #[test]
    fn format_discord_complete_payload_fields() {
        let payload =
            format_discord_complete("alpha", "./demo", "7", "2m 4s", "2026-01-26T10:11:12Z")
                .expect("discord payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let embed = &value["embeds"][0];

        assert_eq!(embed["title"], "✅ Gralph Complete");
        assert_eq!(embed["color"], 5763719);
        assert_eq!(embed["timestamp"], "2026-01-26T10:11:12Z");
        assert_eq!(embed["footer"]["text"], "Gralph CLI");
        assert!(embed["description"].as_str().unwrap().contains("alpha"));

        let fields = embed["fields"].as_array().expect("fields array");
        assert_eq!(fields[0]["name"], "Project");
        assert_eq!(fields[0]["value"], "`./demo`");
        assert_eq!(fields[1]["name"], "Iterations");
        assert_eq!(fields[1]["value"], "7");
        assert_eq!(fields[2]["name"], "Duration");
        assert_eq!(fields[2]["value"], "2m 4s");
    }

    #[test]
    fn format_slack_complete_payload_structure() {
        let payload = format_slack_complete("beta", "repo", "3", "14s", "2026-01-26T11:12:13Z")
            .expect("slack payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let attachment = &value["attachments"][0];

        assert_eq!(attachment["color"], "#57F287");
        let blocks = attachment["blocks"].as_array().expect("blocks array");
        assert_eq!(blocks[0]["type"], "header");
        assert_eq!(blocks[0]["text"]["type"], "plain_text");
        assert_eq!(blocks[0]["text"]["text"], "✅ Gralph Complete");
        assert_eq!(blocks[1]["type"], "section");
        assert!(blocks[1]["text"]["text"].as_str().unwrap().contains("beta"));

        let fields = blocks[2]["fields"].as_array().expect("fields array");
        assert_eq!(fields[0]["text"], "*Project:*\n`repo`");
        assert_eq!(fields[1]["text"], "*Iterations:*\n3");
        assert_eq!(fields[2]["text"], "*Duration:*\n14s");
        assert_eq!(blocks[3]["type"], "context");
        assert_eq!(
            blocks[3]["elements"][0]["text"],
            "Gralph CLI • 2026-01-26T11:12:13Z"
        );
    }

    #[test]
    fn format_generic_complete_payload_fields() {
        let payload = format_generic_complete(
            "gamma",
            "/tmp/project",
            "9",
            "1h 2m 3s",
            "2026-01-26T12:13:14Z",
        )
        .expect("generic payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");

        assert_eq!(value["event"], "complete");
        assert_eq!(value["status"], "success");
        assert_eq!(value["session"], "gamma");
        assert_eq!(value["project"], "/tmp/project");
        assert_eq!(value["iterations"], "9");
        assert_eq!(value["duration"], "1h 2m 3s");
        assert_eq!(value["timestamp"], "2026-01-26T12:13:14Z");
        assert!(value["message"].as_str().unwrap().contains("gamma"));
    }

    #[test]
    fn format_generic_complete_omits_failure_reason_fields() {
        let payload = format_generic_complete("delta", "repo", "2", "9s", "2026-01-26T12:13:14Z")
            .expect("generic payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let object = value.as_object().expect("payload object");

        assert!(!object.contains_key("reason"));
        assert!(!object.contains_key("max_iterations"));
        assert!(!object.contains_key("remaining_tasks"));
    }

    #[test]
    fn format_generic_complete_message_assembly() {
        let payload =
            format_generic_complete("alpha", "/srv/demo", "3", "5s", "2026-01-26T15:16:17Z")
                .expect("generic payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");

        assert_eq!(
            value["message"],
            "Gralph loop 'alpha' completed successfully after 3 iterations (5s)"
        );
    }

    #[test]
    fn emphasized_session_wraps_marker() {
        assert_eq!(emphasized_session("alpha", "**"), "**alpha**");
        assert_eq!(emphasized_session("beta", "*"), "*beta*");
    }

    #[test]
    fn format_complete_description_emphasizes_session() {
        let description = format_complete_description("alpha", "**");
        assert_eq!(
            description,
            "Session **alpha** has finished all tasks successfully."
        );
    }

    #[test]
    fn format_failure_description_maps_known_reasons() {
        let cases = [
            (
                "max_iterations",
                "Session **alpha** hit maximum iterations limit.",
            ),
            ("error", "Session **alpha** encountered an error."),
            ("manual_stop", "Session **alpha** was manually stopped."),
        ];

        for (reason, expected) in cases {
            let description = format_failure_description("alpha", reason, "**");
            assert_eq!(description, expected);
        }
    }

    #[test]
    fn build_generic_payload_optional_fields() {
        let payload = build_generic_payload(
            "complete",
            "success",
            "session",
            "repo",
            None,
            "1",
            None,
            None,
            "4s",
            "2026-01-26T15:16:17Z",
            "custom message".to_string(),
        );
        let object = payload.as_object().expect("payload object");

        assert!(!object.contains_key("reason"));
        assert!(!object.contains_key("max_iterations"));
        assert!(!object.contains_key("remaining_tasks"));
        assert_eq!(
            object.get("message").and_then(Value::as_str),
            Some("custom message")
        );

        let payload = build_generic_payload(
            "failed",
            "failure",
            "session",
            "repo",
            Some("timeout"),
            "2",
            Some("5"),
            Some("1"),
            "4s",
            "2026-01-26T15:16:17Z",
            "failure message".to_string(),
        );
        let object = payload.as_object().expect("payload object");

        assert_eq!(
            object.get("reason").and_then(Value::as_str),
            Some("timeout")
        );
        assert_eq!(
            object.get("max_iterations").and_then(Value::as_str),
            Some("5")
        );
        assert_eq!(
            object.get("remaining_tasks").and_then(Value::as_str),
            Some("1")
        );
        assert_eq!(
            object.get("message").and_then(Value::as_str),
            Some("failure message")
        );
    }

    #[test]
    fn notify_complete_rejects_empty_inputs() {
        let err = notify_complete(
            "",
            "https://example.com/webhook",
            Some("repo"),
            Some(1),
            Some(5),
            Some(3),
        )
        .expect_err("empty session name");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "session name is required")
            }
            _ => panic!("expected invalid input"),
        }

        let err = notify_complete("session", "  ", Some("repo"), Some(1), Some(5), Some(3))
            .expect_err("empty webhook url");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "webhook url is required")
            }
            _ => panic!("expected invalid input"),
        }
    }

    #[test]
    fn notify_failed_rejects_empty_inputs() {
        let err = notify_failed(
            "",
            "https://example.com/webhook",
            Some("error"),
            Some("repo"),
            Some(1),
            Some(2),
            Some(3),
            Some(5),
            Some(3),
        )
        .expect_err("empty session name");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "session name is required")
            }
            _ => panic!("expected invalid input"),
        }

        let err = notify_failed(
            "session",
            "\t",
            Some("error"),
            Some("repo"),
            Some(1),
            Some(2),
            Some(3),
            Some(5),
            Some(3),
        )
        .expect_err("empty webhook url");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "webhook url is required")
            }
            _ => panic!("expected invalid input"),
        }
    }

    #[test]
    fn notify_complete_defaults_unknown_when_optional_missing() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        notify_complete(
            "session",
            &format!("{}/complete", base),
            Some("repo"),
            None,
            None,
            Some(5),
        )
        .expect("notify complete");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");

        assert_eq!(value["event"], "complete");
        assert_eq!(value["status"], "success");
        assert_eq!(value["session"], "session");
        assert_eq!(value["project"], "repo");
        assert_eq!(value["iterations"], "unknown");
        assert_eq!(value["duration"], "unknown");
        assert!(value["message"].as_str().unwrap().contains("unknown"));

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_complete_formats_discord_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");
        let url = format!("{}/discord.com/api/webhooks/123", base);

        notify_complete("session", &url, Some("repo"), Some(4), Some(62), Some(5))
            .expect("notify complete");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");
        let embed = &value["embeds"][0];

        assert_eq!(embed["title"], "✅ Gralph Complete");
        assert_eq!(embed["fields"][0]["value"], "`repo`");
        assert_eq!(embed["fields"][1]["value"], "4");
        assert_eq!(embed["fields"][2]["value"], "1m 2s");

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_complete_formats_slack_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");
        let url = format!("{}/hooks.slack.com/services/123", base);

        notify_complete("session", &url, Some("repo"), Some(4), Some(62), Some(5))
            .expect("notify complete");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");
        let attachment = &value["attachments"][0];
        let blocks = attachment["blocks"].as_array().expect("blocks");
        let fields = blocks[2]["fields"].as_array().expect("fields");

        assert_eq!(attachment["color"], "#57F287");
        assert_eq!(blocks[0]["text"]["text"], "✅ Gralph Complete");
        assert!(blocks[1]["text"]["text"]
            .as_str()
            .unwrap()
            .contains("session"));
        assert_eq!(fields[0]["text"], "*Project:*\n`repo`");
        assert_eq!(fields[1]["text"], "*Iterations:*\n4");
        assert_eq!(fields[2]["text"], "*Duration:*\n1m 2s");

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_complete_handles_non_success_status() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 503 Service Unavailable", "");

        let err = notify_complete(
            "session",
            &format!("{}/status", base),
            None,
            None,
            None,
            Some(5),
        )
        .expect_err("non-success status");
        assert!(matches!(err, NotifyError::HttpStatus(503)));
        assert!(captured.lock().unwrap().is_some());

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_defaults_unknown_when_optional_missing() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        notify_failed(
            "session",
            &format!("{}/failed", base),
            Some("error"),
            Some("repo"),
            None,
            None,
            None,
            None,
            Some(5),
        )
        .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");

        assert_eq!(value["event"], "failed");
        assert_eq!(value["status"], "failure");
        assert_eq!(value["reason"], "error");
        assert_eq!(value["iterations"], "unknown");
        assert_eq!(value["max_iterations"], "unknown");
        assert_eq!(value["remaining_tasks"], "unknown");
        assert_eq!(value["duration"], "unknown");
        assert_eq!(
            value["message"],
            "Gralph loop 'session' failed due to an error after unknown iterations"
        );

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_defaults_reason_when_missing_for_generic_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        notify_failed(
            "session",
            &format!("{}/failed", base),
            None,
            Some("repo"),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
        )
        .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");

        assert_eq!(value["event"], "failed");
        assert_eq!(value["status"], "failure");
        assert_eq!(value["reason"], "unknown");

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_preserves_unknown_reason_for_generic_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        notify_failed(
            "session",
            &format!("{}/failed", base),
            Some("mystery"),
            Some("repo"),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
        )
        .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");

        assert_eq!(value["reason"], "mystery");
        assert_eq!(
            value["message"],
            "Gralph loop 'session' failed: mystery after 1 iterations"
        );

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_includes_generic_payload_fields_for_unknown_reason() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        notify_failed(
            "session",
            &format!("{}/failed", base),
            Some("mystery"),
            Some("repo"),
            Some(2),
            Some(5),
            Some(1),
            Some(3599),
            Some(5),
        )
        .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");

        assert_eq!(value["event"], "failed");
        assert_eq!(value["status"], "failure");
        assert_eq!(value["reason"], "mystery");
        assert_eq!(value["project"], "repo");
        assert_eq!(value["iterations"], "2");
        assert_eq!(value["max_iterations"], "5");
        assert_eq!(value["remaining_tasks"], "1");
        assert_eq!(value["duration"], "59m 59s");
        assert_eq!(
            value["message"],
            "Gralph loop 'session' failed: mystery after 2 iterations"
        );

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_allows_empty_reason_for_generic_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        notify_failed(
            "session",
            &format!("{}/failed", base),
            Some(""),
            Some("repo"),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
        )
        .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");

        assert_eq!(value["reason"], "");
        assert_eq!(
            value["message"],
            "Gralph loop 'session' failed:  after 1 iterations"
        );

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_defaults_unknown_for_discord_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");
        let url = format!("{}/discord.com/api/webhooks/123", base);

        notify_failed("session", &url, None, None, None, None, None, None, Some(5))
            .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");
        let embed = &value["embeds"][0];

        assert_eq!(embed["description"], "Session **session** failed: unknown");
        assert_eq!(embed["fields"][0]["value"], "`unknown`");
        assert_eq!(embed["fields"][1]["value"], "unknown");
        assert_eq!(embed["fields"][2]["value"], "unknown/unknown");
        assert_eq!(embed["fields"][3]["value"], "unknown");
        assert_eq!(embed["fields"][4]["value"], "unknown");

        handle.join().expect("server thread");
    }

    #[test]
    fn notify_failed_defaults_unknown_for_slack_payload() {
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");
        let url = format!("{}/hooks.slack.com/services/123", base);

        notify_failed("session", &url, None, None, None, None, None, None, Some(5))
            .expect("notify failed");

        let request = captured.lock().unwrap().clone().expect("captured request");
        let value: Value = serde_json::from_str(&request.body).expect("json payload");
        let attachment = &value["attachments"][0];
        let blocks = attachment["blocks"].as_array().expect("blocks");
        let fields = blocks[2]["fields"].as_array().expect("fields");

        assert_eq!(
            blocks[1]["text"]["text"],
            "Session *session* failed: unknown"
        );
        assert_eq!(fields[0]["text"], "*Project:*\n`unknown`");
        assert_eq!(fields[1]["text"], "*Reason:*\nunknown");
        assert_eq!(fields[2]["text"], "*Iterations:*\nunknown/unknown");
        assert_eq!(fields[3]["text"], "*Remaining Tasks:*\nunknown");
        assert_eq!(fields[4]["text"], "*Duration:*\nunknown");

        handle.join().expect("server thread");
    }

    #[test]
    fn format_discord_failed_reason_mappings() {
        let reasons = ["max_iterations", "error", "manual_stop", "timeout"];
        for reason in reasons {
            let payload = format_discord_failed(
                "alpha",
                "repo",
                reason,
                "2",
                "5",
                "1",
                "4m 1s",
                "2026-01-26T01:02:03Z",
            )
            .expect("discord payload");
            let value: Value = serde_json::from_str(&payload).expect("json payload");
            let embed = &value["embeds"][0];
            let description = embed["description"].as_str().expect("description");

            assert_eq!(
                description,
                format_failure_description("alpha", reason, "**")
            );
            assert_eq!(embed["fields"][1]["name"], "Reason");
            assert_eq!(embed["fields"][1]["value"], reason);
        }
    }

    #[test]
    fn format_discord_failed_manual_stop_payload() {
        let payload = format_discord_failed(
            "alpha",
            "repo",
            "manual_stop",
            "2",
            "5",
            "1",
            "4m 1s",
            "2026-01-26T01:02:03Z",
        )
        .expect("discord payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let embed = &value["embeds"][0];

        assert_eq!(
            embed["description"],
            "Session **alpha** was manually stopped."
        );
        assert_eq!(embed["fields"][1]["name"], "Reason");
        assert_eq!(embed["fields"][1]["value"], "manual_stop");
    }

    #[test]
    fn format_discord_failed_unknown_reason_message() {
        let payload = format_discord_failed(
            "alpha",
            "repo",
            "mystery",
            "2",
            "5",
            "1",
            "4m 1s",
            "2026-01-26T01:02:03Z",
        )
        .expect("discord payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let embed = &value["embeds"][0];

        assert_eq!(embed["description"], "Session **alpha** failed: mystery");
        assert_eq!(embed["fields"][1]["value"], "mystery");
    }

    #[test]
    fn format_slack_failed_reason_mappings() {
        let reasons = ["max_iterations", "error", "manual_stop", "timeout"];
        for reason in reasons {
            let payload = format_slack_failed(
                "beta",
                "repo",
                reason,
                "2",
                "5",
                "1",
                "4m 1s",
                "2026-01-26T01:02:03Z",
            )
            .expect("slack payload");
            let value: Value = serde_json::from_str(&payload).expect("json payload");
            let attachment = &value["attachments"][0];
            let blocks = attachment["blocks"].as_array().expect("blocks");
            let description = blocks[1]["text"]["text"].as_str().expect("description");

            assert_eq!(description, format_failure_description("beta", reason, "*"));
            assert_eq!(
                blocks[2]["fields"][1]["text"],
                format!("*Reason:*\n{}", reason)
            );
        }
    }

    #[test]
    fn format_slack_failed_manual_stop_payload() {
        let payload = format_slack_failed(
            "beta",
            "repo",
            "manual_stop",
            "2",
            "5",
            "1",
            "4m 1s",
            "2026-01-26T01:02:03Z",
        )
        .expect("slack payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let attachment = &value["attachments"][0];
        let blocks = attachment["blocks"].as_array().expect("blocks");

        assert_eq!(
            blocks[1]["text"]["text"],
            "Session *beta* was manually stopped."
        );
        assert_eq!(blocks[2]["fields"][1]["text"], "*Reason:*\nmanual_stop");
    }

    #[test]
    fn format_slack_failed_unknown_reason_payload() {
        let payload = format_slack_failed(
            "beta",
            "repo",
            "timeout",
            "2",
            "5",
            "1",
            "4m 1s",
            "2026-01-26T01:02:03Z",
        )
        .expect("slack payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");
        let attachment = &value["attachments"][0];
        let blocks = attachment["blocks"].as_array().expect("blocks");

        assert_eq!(blocks[1]["text"]["text"], "Session *beta* failed: timeout");
        assert_eq!(blocks[2]["fields"][1]["text"], "*Reason:*\ntimeout");
    }

    #[test]
    fn format_generic_failed_reason_mappings() {
        let cases = [
            (
                "max_iterations",
                "Gralph loop 'gamma' failed: hit max iterations (2/5) with 1 tasks remaining",
            ),
            (
                "error",
                "Gralph loop 'gamma' failed due to an error after 2 iterations",
            ),
            (
                "manual_stop",
                "Gralph loop 'gamma' was manually stopped after 2 iterations with 1 tasks remaining",
            ),
            (
                "timeout",
                "Gralph loop 'gamma' failed: timeout after 2 iterations",
            ),
        ];

        for (reason, expected) in cases {
            let payload = format_generic_failed(
                "gamma",
                "repo",
                reason,
                "2",
                "5",
                "1",
                "4m 1s",
                "2026-01-26T01:02:03Z",
            )
            .expect("generic payload");
            let value: Value = serde_json::from_str(&payload).expect("json payload");

            assert_eq!(value["event"], "failed");
            assert_eq!(value["status"], "failure");
            assert_eq!(value["reason"], reason);
            assert_eq!(value["max_iterations"], "5");
            assert_eq!(value["remaining_tasks"], "1");
            assert_eq!(value["message"], expected);
        }
    }

    #[test]
    fn format_generic_failed_includes_required_fields() {
        let payload = format_generic_failed(
            "delta",
            "/tmp/project",
            "timeout",
            "7",
            "10",
            "2",
            "45s",
            "2026-01-26T05:06:07Z",
        )
        .expect("generic payload");
        let value: Value = serde_json::from_str(&payload).expect("json payload");

        assert_eq!(value["event"], "failed");
        assert_eq!(value["status"], "failure");
        assert_eq!(value["session"], "delta");
        assert_eq!(value["project"], "/tmp/project");
        assert_eq!(value["iterations"], "7");
        assert_eq!(value["max_iterations"], "10");
        assert_eq!(value["remaining_tasks"], "2");
        assert_eq!(value["duration"], "45s");
        assert_eq!(value["timestamp"], "2026-01-26T05:06:07Z");
    }

    #[test]
    fn format_duration_handles_none_and_units() {
        assert_eq!(format_duration(None), "unknown");
        assert_eq!(format_duration(Some(0)), "0s");
        assert_eq!(format_duration(Some(65)), "1m 5s");
        assert_eq!(format_duration(Some(3600)), "1h 0m 0s");
        assert_eq!(format_duration(Some(3661)), "1h 1m 1s");
        assert_eq!(format_duration(Some(90061)), "25h 1m 1s");
    }

    #[test]
    fn format_duration_handles_boundaries() {
        assert_eq!(format_duration(Some(59)), "59s");
        assert_eq!(format_duration(Some(60)), "1m 0s");
        assert_eq!(format_duration(Some(3599)), "59m 59s");
        assert_eq!(format_duration(Some(3600)), "1h 0m 0s");
        assert_eq!(format_duration(Some(3601)), "1h 0m 1s");
    }

    #[test]
    fn format_failure_description_handles_unknown_reason() {
        let description = format_failure_description("alpha", "timeout", "**");
        assert_eq!(description, "Session **alpha** failed: timeout");
    }

    #[test]
    fn send_webhook_rejects_empty_payload() {
        let err = send_webhook("https://example.com", "", Some(5)).expect_err("empty payload");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "payload is required")
            }
            _ => panic!("expected invalid input"),
        }

        let err =
            send_webhook("https://example.com", "  ", Some(5)).expect_err("whitespace payload");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "payload is required")
            }
            _ => panic!("expected invalid input"),
        }
    }

    #[test]
    fn send_webhook_rejects_empty_url() {
        let err = send_webhook("", "{}", Some(5)).expect_err("empty url");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "webhook url is required")
            }
            _ => panic!("expected invalid input"),
        }

        let err = send_webhook("  ", "{}", Some(5)).expect_err("whitespace url");
        match err {
            NotifyError::InvalidInput(message) => {
                assert_eq!(message, "webhook url is required")
            }
            _ => panic!("expected invalid input"),
        }
    }

    #[test]
    fn send_webhook_handles_invalid_url() {
        let err = send_webhook("not a url", "{}", Some(5)).expect_err("invalid url");
        assert!(matches!(err, NotifyError::Http(_)));
    }

    #[test]
    fn send_webhook_posts_payload_and_headers() {
        let payload = "{\"hello\":\"world\"}";
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        send_webhook(&format!("{}/notify", base), payload, Some(5)).expect("send webhook");

        let request = captured.lock().unwrap().clone().expect("captured request");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/notify");
        assert_eq!(
            request.headers.get("content-type").map(String::as_str),
            Some("application/json")
        );
        assert_eq!(request.body, payload);

        handle.join().expect("server thread");
    }

    #[test]
    fn send_webhook_defaults_timeout_when_zero() {
        let payload = "{}";
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        send_webhook(&format!("{}/default", base), payload, Some(0)).expect("send webhook");

        assert!(captured.lock().unwrap().is_some());
        handle.join().expect("server thread");
    }

    #[test]
    fn send_webhook_defaults_timeout_when_none() {
        let payload = "{}";
        let (base, captured, handle) = start_test_server("HTTP/1.1 204 No Content", "");

        send_webhook(&format!("{}/default-none", base), payload, None).expect("send webhook");

        assert!(captured.lock().unwrap().is_some());
        handle.join().expect("server thread");
    }

    #[test]
    fn send_webhook_handles_non_success_status() {
        let payload = "{}";
        let (base, captured, handle) =
            start_test_server("HTTP/1.1 500 Internal Server Error", "oops");

        let err = send_webhook(&format!("{}/fail", base), payload, Some(5))
            .expect_err("non-success status");
        assert!(matches!(err, NotifyError::HttpStatus(500)));
        assert!(captured.lock().unwrap().is_some());

        handle.join().expect("server thread");
    }

    #[test]
    fn send_webhook_times_out_when_server_stalls() {
        let payload = "{}";
        let (base, handle) = start_hanging_server(Duration::from_millis(1500));

        let err = send_webhook(&format!("{}/timeout", base), payload, Some(1))
            .expect_err("timeout error");
        match err {
            NotifyError::Http(err) => assert!(err.is_timeout()),
            _ => panic!("expected http timeout"),
        }

        handle.join().expect("server thread");
    }
}
