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

fn format_discord_complete(
    session_name: &str,
    project_dir: &str,
    iterations: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let payload = json!({
        "embeds": [{
            "title": "✅ Gralph Complete",
            "description": format!("Session **{}** has finished all tasks successfully.", session_name),
            "color": 5763719,
            "fields": [
                {
                    "name": "Project",
                    "value": format!("`{}`", project_dir),
                    "inline": false
                },
                {
                    "name": "Iterations",
                    "value": iterations,
                    "inline": true
                },
                {
                    "name": "Duration",
                    "value": duration_str,
                    "inline": true
                }
            ],
            "footer": {
                "text": "Gralph CLI"
            },
            "timestamp": timestamp
        }]
    });
    Ok(serde_json::to_string_pretty(&payload)?)
}

fn format_slack_complete(
    session_name: &str,
    project_dir: &str,
    iterations: &str,
    duration_str: &str,
    timestamp: &str,
) -> Result<String, NotifyError> {
    let payload = json!({
        "attachments": [{
            "color": "#57F287",
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": "✅ Gralph Complete",
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("Session *{}* has finished all tasks successfully.", session_name)
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        {
                            "type": "mrkdwn",
                            "text": format!("*Project:*\n`{}`", project_dir)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Iterations:*\n{}", iterations)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Duration:*\n{}", duration_str)
                        }
                    ]
                },
                {
                    "type": "context",
                    "elements": [{
                        "type": "mrkdwn",
                        "text": format!("Gralph CLI • {}", timestamp)
                    }]
                }
            ]
        }]
    });
    Ok(serde_json::to_string_pretty(&payload)?)
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
    let description = match failure_reason {
        "max_iterations" => format!("Session *{}* hit maximum iterations limit.", session_name),
        "error" => format!("Session *{}* encountered an error.", session_name),
        "manual_stop" => format!("Session *{}* was manually stopped.", session_name),
        _ => format!("Session *{}* failed: {}", session_name, failure_reason),
    };

    let payload = json!({
        "attachments": [{
            "color": "#ED4245",
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": "❌ Gralph Failed",
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": description
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        {
                            "type": "mrkdwn",
                            "text": format!("*Project:*\n`{}`", project_dir)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Reason:*\n{}", failure_reason)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Iterations:*\n{}/{}", iterations, max_iterations)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Remaining Tasks:*\n{}", remaining_tasks)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Duration:*\n{}", duration_str)
                        }
                    ]
                },
                {
                    "type": "context",
                    "elements": [{
                        "type": "mrkdwn",
                        "text": format!("Gralph CLI • {}", timestamp)
                    }]
                }
            ]
        }]
    });
    Ok(serde_json::to_string_pretty(&payload)?)
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
    let description = match failure_reason {
        "max_iterations" => format!("Session **{}** hit maximum iterations limit.", session_name),
        "error" => format!("Session **{}** encountered an error.", session_name),
        "manual_stop" => format!("Session **{}** was manually stopped.", session_name),
        _ => format!("Session **{}** failed: {}", session_name, failure_reason),
    };

    let payload = json!({
        "embeds": [{
            "title": "❌ Gralph Failed",
            "description": description,
            "color": 15548997,
            "fields": [
                {
                    "name": "Project",
                    "value": format!("`{}`", project_dir),
                    "inline": false
                },
                {
                    "name": "Reason",
                    "value": failure_reason,
                    "inline": true
                },
                {
                    "name": "Iterations",
                    "value": format!("{}/{}", iterations, max_iterations),
                    "inline": true
                },
                {
                    "name": "Remaining Tasks",
                    "value": remaining_tasks,
                    "inline": true
                },
                {
                    "name": "Duration",
                    "value": duration_str,
                    "inline": true
                }
            ],
            "footer": {
                "text": "Gralph CLI"
            },
            "timestamp": timestamp
        }]
    });
    Ok(serde_json::to_string_pretty(&payload)?)
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
    let payload = json!({
        "event": "complete",
        "status": "success",
        "session": session_name,
        "project": project_dir,
        "iterations": iterations,
        "duration": duration_str,
        "timestamp": timestamp,
        "message": message
    });
    Ok(serde_json::to_string_pretty(&payload)?)
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
    let payload = json!({
        "event": "failed",
        "status": "failure",
        "session": session_name,
        "project": project_dir,
        "reason": failure_reason,
        "iterations": iterations,
        "max_iterations": max_iterations,
        "remaining_tasks": remaining_tasks,
        "duration": duration_str,
        "timestamp": timestamp,
        "message": message
    });
    Ok(serde_json::to_string_pretty(&payload)?)
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
        let (header_bytes, mut body_bytes) = buffer.split_at(header_end);
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

    #[test]
    fn detect_webhook_type_matches() {
        assert_eq!(
            detect_webhook_type("https://discord.com/api/webhooks/123"),
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
            detect_webhook_type("https://example.com/webhook"),
            WebhookType::Generic
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
}
