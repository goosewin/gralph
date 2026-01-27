mod support;

use gralph_rs::backend::opencode::OpenCodeBackend;
use gralph_rs::backend::{Backend, BackendError};
use std::fs;

#[test]
#[ignore]
fn opencode_cli_smoke() {
    let backend = OpenCodeBackend::new();
    assert!(backend.check_installed());
}

#[test]
fn opencode_run_iteration_writes_output_and_args() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("opencode.out");
    let script = render_args_env_script();
    let fake = support::FakeCli::new_script("opencode", &script).unwrap();
    let _guard = fake.prepend_to_path().unwrap();
    let _env_guard = EnvGuard::new("TEST_BACKEND_ENV", "ok");

    let backend = OpenCodeBackend::with_command(fake.command());
    backend
        .run_iteration("prompt", Some("test-model"), &output_path)
        .unwrap();

    let output = fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("args:run --model test-model prompt"));
    assert!(output.contains("env:ok"));

    let parsed = backend.parse_text(&output_path).unwrap();
    assert_eq!(parsed, output);
}

#[test]
fn opencode_run_iteration_reports_failure_exit() {
    let temp = tempfile::tempdir().unwrap();
    let output_path = temp.path().join("opencode.err");
    let fake = support::FakeCli::new("opencode", "", "", 9).unwrap();
    let _guard = fake.prepend_to_path().unwrap();

    let backend = OpenCodeBackend::with_command(fake.command());
    let result = backend.run_iteration("prompt", None, &output_path);

    assert!(
        matches!(result, Err(BackendError::Command(message)) if message.contains("opencode exited with"))
    );
}

fn render_args_env_script() -> String {
    if cfg!(windows) {
        "@echo off\r\necho args:%*\r\necho env:%TEST_BACKEND_ENV%\r\nexit /b 0\r\n".to_string()
    } else {
        "#!/bin/sh\nprintf '%s\\n' \"args:$*\" \"env:$TEST_BACKEND_ENV\"\nexit 0\n".to_string()
    }
}

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn new(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}
