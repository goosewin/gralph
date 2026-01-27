mod support;

use gralph_rs::backend::claude::ClaudeBackend;
use gralph_rs::backend::{Backend, BackendError};
use std::fs;

#[test]
fn claude_run_iteration_writes_json_output_and_args() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("claude.out");
    let script = render_claude_script();
    let fake = support::FakeCli::new_script("claude", &script).unwrap();
    let _guard = fake.prepend_to_path().unwrap();

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
fn claude_run_iteration_reports_failure_exit() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("claude.err");
    let fake = support::FakeCli::new("claude", "", "", 2).unwrap();
    let _guard = fake.prepend_to_path().unwrap();

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
