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
}

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotifyError::InvalidInput(message) => write!(f, "invalid input: {}", message),
            NotifyError::Http(err) => write!(f, "http error: {}", err),
            NotifyError::HttpStatus(code) => write!(f, "webhook returned HTTP {}", code),
        }
    }
}

impl Error for NotifyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotifyError::Http(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for NotifyError {
    fn from(error: reqwest::Error) -> Self {
        NotifyError::Http(error)
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
}
