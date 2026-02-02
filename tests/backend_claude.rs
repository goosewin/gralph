mod support;

use gralph_rs::backend::claude::ClaudeBackend;
use gralph_rs::backend::{Backend, BackendError};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn claude_run_iteration_writes_json_output_and_args() {
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args([
            "test",
            "--test",
            "backend_claude",
            "claude_run_iteration_writes_json_output_and_args_impl",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ])
        .status()
        .expect("Failed to run subprocess test");
    assert!(status.success(), "Subprocess test failed");
}

#[test]
#[ignore] // Run only via subprocess from the main test
fn claude_run_iteration_writes_json_output_and_args_impl() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("claude.out");
    let script = render_claude_script();
    let fake = support::FakeCli::new_script("claude", &script).unwrap();

    prepend_path(fake.bin_dir());

    let backend = ClaudeBackend::with_command(fake.command());
    backend
        .run_iteration(
            "prompt",
            Some("test-model"),
            None,
            &output_path,
            temp.path(),
        )
        .unwrap();

    let output = fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("args:--dangerously-skip-permissions --verbose --print --output-format stream-json -p prompt --model test-model"));
    assert!(output.contains("env:1"));

    let parsed = backend.parse_text(&output_path).unwrap();
    assert_eq!(parsed, "done");
}

#[test]
fn claude_run_iteration_orders_args_and_places_model_flag() {
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args([
            "test",
            "--test",
            "backend_claude",
            "claude_run_iteration_orders_args_and_places_model_flag_impl",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ])
        .status()
        .expect("Failed to run subprocess test");
    assert!(status.success(), "Subprocess test failed");
}

#[test]
#[ignore] // Run only via subprocess from the main test
fn claude_run_iteration_orders_args_and_places_model_flag_impl() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("claude.args");
    let script = render_claude_script();
    let fake = support::FakeCli::new_script("claude", &script).unwrap();

    prepend_path(fake.bin_dir());

    let backend = ClaudeBackend::with_command(fake.command());
    backend
        .run_iteration(
            "final-prompt",
            Some("model-x"),
            None,
            &output_path,
            temp.path(),
        )
        .unwrap();

    let output = fs::read_to_string(&output_path).unwrap();
    let args = extract_args_from_output(&output).expect("args in output");
    assert_eq!(
        args,
        "--dangerously-skip-permissions --verbose --print --output-format stream-json -p final-prompt --model model-x"
    );
}

#[test]
fn claude_parse_text_falls_back_to_raw_when_no_result_entries() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("claude.raw");
    let contents = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hello\"}]}}\nnot-json\n";
    fs::write(&output_path, contents).unwrap();

    let backend = ClaudeBackend::new();
    let parsed = backend.parse_text(&output_path).unwrap();
    assert_eq!(parsed, contents);
}

#[test]
fn claude_run_iteration_reports_failure_exit() {
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args([
            "test",
            "--test",
            "backend_claude",
            "claude_run_iteration_reports_failure_exit_impl",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ])
        .status()
        .expect("Failed to run subprocess test");
    assert!(status.success(), "Subprocess test failed");
}

#[test]
#[ignore] // Run only via subprocess from the main test
fn claude_run_iteration_reports_failure_exit_impl() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("claude.err");
    let fake = support::FakeCli::new("claude", "", "", 2).unwrap();

    prepend_path(fake.bin_dir());

    let backend = ClaudeBackend::with_command(fake.command());
    let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

    assert!(
        matches!(result, Err(BackendError::Command(message)) if message.contains("claude exited with"))
    );
}

fn render_claude_script() -> String {
    if cfg!(windows) {
        "@echo off\r\necho {\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"args:%* env:%IS_SANDBOX%\"}]}}\r\necho {\"type\":\"result\",\"result\":\"done\"}\r\nexit /b 0\r\n".to_string()
    } else {
        r#"#!/bin/sh
echo "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"args:$* env:$IS_SANDBOX\"}]}}"
echo "{\"type\":\"result\",\"result\":\"done\"}"
exit 0
"#
            .to_string()
    }
}

fn extract_args_from_output(output: &str) -> Option<String> {
    for line in output.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("assistant") {
            continue;
        }
        let text = value.pointer("/message/content/0/text")?.as_str()?;
        let rest = text.strip_prefix("args:")?;
        let (args, _) = rest.split_once(" env:")?;
        return Some(args.to_string());
    }
    None
}

fn prepend_path(dir: &std::path::Path) {
    let mut paths: Vec<PathBuf> = vec![dir.to_path_buf()];
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }
    if let Ok(joined) = env::join_paths(&paths) {
        // Safe in subprocess - this process is isolated
        unsafe { env::set_var("PATH", joined) };
    }
}
