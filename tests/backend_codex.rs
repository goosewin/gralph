mod support;

use gralph_rs::backend::codex::CodexBackend;
use gralph_rs::backend::{Backend, BackendError};
use std::env;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore]
fn codex_cli_smoke() {
    let backend = CodexBackend::new();
    assert!(backend.check_installed());
}

/// Test that verifies the backend correctly passes arguments to the CLI.
/// Uses a subprocess to isolate environment modifications from the test runner.
#[test]
fn codex_run_iteration_writes_output_and_args() {
    // Run the actual test in a subprocess to isolate env changes
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args([
            "test",
            "--test",
            "backend_codex",
            "codex_run_iteration_writes_output_and_args_impl",
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
fn codex_run_iteration_writes_output_and_args_impl() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("codex.out");
    let script = render_args_script();
    let fake = support::FakeCli::new_script("codex", &script).unwrap();

    // Safe to modify PATH here since we're in an isolated subprocess
    prepend_path(fake.bin_dir());

    let backend = CodexBackend::with_command(fake.command());
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
    assert!(output.contains("args:--quiet --auto-approve --model test-model prompt"));

    let parsed = backend.parse_text(&output_path).unwrap();
    assert_eq!(parsed, output);
}

/// Test that verifies the backend reports command failures correctly.
#[test]
fn codex_run_iteration_reports_failure_exit() {
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args([
            "test",
            "--test",
            "backend_codex",
            "codex_run_iteration_reports_failure_exit_impl",
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
fn codex_run_iteration_reports_failure_exit_impl() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("codex.err");
    let fake = support::FakeCli::new("codex", "", "", 4).unwrap();

    prepend_path(fake.bin_dir());

    let backend = CodexBackend::with_command(fake.command());
    let result = backend.run_iteration("prompt", None, None, &output_path, temp.path());

    match result {
        Err(BackendError::Command(_)) => {}
        other => panic!("expected BackendError::Command, got {other:?}"),
    }
}

fn render_args_script() -> String {
    if cfg!(windows) {
        "@echo off\r\necho args:%*\r\nexit /b 0\r\n".to_string()
    } else {
        "#!/bin/sh\nprintf '%s\\n' \"args:$*\"\nexit 0\n".to_string()
    }
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
