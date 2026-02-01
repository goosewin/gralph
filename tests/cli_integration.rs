use predicates::prelude::*;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod support;
use support::FakeCli;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const ENV_KEYS: [&str; 8] = [
    "GRALPH_DEFAULT_CONFIG",
    "GRALPH_GLOBAL_CONFIG",
    "GRALPH_PROJECT_CONFIG_NAME",
    "GRALPH_CONFIG_DIR",
    "GRALPH_STATE_DIR",
    "GRALPH_STATE_FILE",
    "GRALPH_LOCK_FILE",
    "GRALPH_LOCK_TIMEOUT",
];

struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    originals: Vec<(String, Option<OsString>)>,
}

impl EnvGuard {
    fn new(keys: &[&str]) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let originals = keys
            .iter()
            .map(|key| ((*key).to_string(), env::var_os(key)))
            .collect();
        Self {
            _lock: lock,
            originals,
        }
    }

    fn set(&self, key: &str, value: impl AsRef<OsStr>) {
        unsafe {
            env::set_var(key, value);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.originals.drain(..) {
            match value {
                Some(original) => unsafe {
                    env::set_var(&key, original);
                },
                None => unsafe {
                    env::remove_var(&key);
                },
            }
        }
    }
}

fn prepare_env(base: &Path) -> EnvGuard {
    let guard = EnvGuard::new(&ENV_KEYS);
    let config_dir = base.join("config");
    fs::create_dir_all(&config_dir).unwrap();
    let default_path = config_dir.join("default.yaml");
    let global_path = config_dir.join("global.yaml");
    fs::write(
        &default_path,
        "defaults:\n  context_files: ARCHITECTURE.md,PROCESS.md\n",
    )
    .unwrap();
    fs::write(&global_path, "defaults: {}\n").unwrap();

    guard.set("GRALPH_DEFAULT_CONFIG", &default_path);
    guard.set("GRALPH_GLOBAL_CONFIG", &global_path);
    guard.set("GRALPH_CONFIG_DIR", &config_dir);
    guard.set("GRALPH_PROJECT_CONFIG_NAME", "missing.yaml");
    guard.set("GRALPH_STATE_DIR", base.join("state"));
    guard.set("GRALPH_STATE_FILE", base.join("state").join("state.json"));
    guard.set("GRALPH_LOCK_FILE", base.join("state").join("state.lock"));
    guard.set("GRALPH_LOCK_TIMEOUT", "1");
    guard
}

fn temp_path(base: &Path, name: &str) -> PathBuf {
    base.join(name)
}

#[test]
fn cli_help_shows_overview() {
    let temp = tempfile::tempdir().unwrap();
    let _guard = prepare_env(temp.path());

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Autonomous AI coding loops"))
        .stdout(predicate::str::contains("START OPTIONS"));
}

#[test]
fn cli_rejects_invalid_args() {
    let temp = tempfile::tempdir().unwrap();
    let _guard = prepare_env(temp.path());

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    cmd.arg("--definitely-invalid");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn cli_init_writes_context_files() {
    let temp = tempfile::tempdir().unwrap();
    let _guard = prepare_env(temp.path());
    let target = temp_path(temp.path(), "project");
    fs::create_dir_all(&target).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    cmd.args(["init", "--dir"])
        .arg(&target)
        .current_dir(temp.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Init summary:"));

    assert!(target.join("ARCHITECTURE.md").is_file());
    assert!(target.join("PROCESS.md").is_file());
}

#[test]
fn cli_prd_check_reports_missing_file() {
    let temp = tempfile::tempdir().unwrap();
    let _guard = prepare_env(temp.path());
    let missing = temp_path(temp.path(), "missing.md");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    cmd.args(["prd", "check"]).arg(&missing);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task file does not exist"));
}

#[test]
fn cli_prd_create_writes_generated_file() {
    let temp = tempfile::tempdir().unwrap();
    let _guard = prepare_env(temp.path());
    let project = temp_path(temp.path(), "project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("ARCHITECTURE.md"), "arch\n").unwrap();
    fs::write(project.join("PROCESS.md"), "process\n").unwrap();

    let prd_output = project.join("PRD.generated.md");
    let prd_contents = "# Project Requirements Document\n\n## Overview\n\nTest.\n\n## Implementation Tasks\n\n### Task TST-1\n\n- **ID** TST-1\n- **Context Bundle** `ARCHITECTURE.md`\n- **DoD** Example.\n- **Checklist**\n  * Example\n- **Dependencies** None\n- [ ] TST-1 Example task\n";
    let fake = FakeCli::new("codex", prd_contents, "", 0).unwrap();
    let _path_guard = fake.prepend_to_path().unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    cmd.current_dir(&project);
    cmd.args([
        "prd",
        "create",
        "--goal",
        "Test PRD",
        "--backend",
        "codex",
        "--output",
    ])
    .arg(&prd_output);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PRD created:"));

    let generated = fs::read_to_string(&prd_output).unwrap();
    assert!(generated.contains("### Task TST-1"));
}
