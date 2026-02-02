use predicates::prelude::*;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod support;
use support::FakeCli;

/// Environment configuration to apply to child commands.
/// Instead of modifying the parent process environment (which is unsafe in multi-threaded contexts),
/// we collect the env vars and apply them directly to the Command.
struct TestEnv {
    vars: Vec<(String, OsString)>,
    path_prefix: Option<PathBuf>,
}

impl TestEnv {
    fn new() -> Self {
        Self {
            vars: Vec::new(),
            path_prefix: None,
        }
    }

    fn set(&mut self, key: &str, value: impl Into<OsString>) {
        self.vars.push((key.to_string(), value.into()));
    }

    fn prepend_path(&mut self, dir: PathBuf) {
        self.path_prefix = Some(dir);
    }

    /// Apply environment variables to a Command
    fn apply(&self, cmd: &mut assert_cmd::Command) {
        for (key, value) in &self.vars {
            cmd.env(key, value);
        }
        if let Some(prefix) = &self.path_prefix {
            let mut paths = vec![prefix.clone()];
            if let Some(existing) = env::var_os("PATH") {
                paths.extend(env::split_paths(&existing));
            }
            if let Ok(joined) = env::join_paths(&paths) {
                cmd.env("PATH", joined);
            }
        }
    }
}

fn prepare_env(base: &Path) -> TestEnv {
    let mut env = TestEnv::new();
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

    env.set("GRALPH_DEFAULT_CONFIG", &default_path);
    env.set("GRALPH_GLOBAL_CONFIG", &global_path);
    env.set("GRALPH_CONFIG_DIR", &config_dir);
    env.set("GRALPH_PROJECT_CONFIG_NAME", "missing.yaml");
    env.set("GRALPH_STATE_DIR", base.join("state"));
    env.set("GRALPH_STATE_FILE", base.join("state").join("state.json"));
    env.set("GRALPH_LOCK_FILE", base.join("state").join("state.lock"));
    env.set("GRALPH_LOCK_TIMEOUT", "1");
    env
}

fn temp_path(base: &Path, name: &str) -> PathBuf {
    base.join(name)
}

#[test]
fn cli_help_shows_overview() {
    let temp = tempfile::tempdir().unwrap();
    let test_env = prepare_env(temp.path());

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    test_env.apply(&mut cmd);
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Autonomous AI coding loops"))
        .stdout(predicate::str::contains("START OPTIONS"));
}

#[test]
fn cli_rejects_invalid_args() {
    let temp = tempfile::tempdir().unwrap();
    let test_env = prepare_env(temp.path());

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    test_env.apply(&mut cmd);
    cmd.arg("--definitely-invalid");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn cli_init_writes_context_files() {
    let temp = tempfile::tempdir().unwrap();
    let test_env = prepare_env(temp.path());
    let target = temp_path(temp.path(), "project");
    fs::create_dir_all(&target).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    test_env.apply(&mut cmd);
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
    let test_env = prepare_env(temp.path());
    let missing = temp_path(temp.path(), "missing.md");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    test_env.apply(&mut cmd);
    cmd.args(["prd", "check"]).arg(&missing);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task file does not exist"));
}

#[test]
fn cli_prd_create_spawns_background_session() {
    let temp = tempfile::tempdir().unwrap();
    let mut test_env = prepare_env(temp.path());
    let project = temp_path(temp.path(), "project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("ARCHITECTURE.md"), "arch\n").unwrap();
    fs::write(project.join("PROCESS.md"), "process\n").unwrap();

    let prd_output = project.join("PRD.generated.md");
    let prd_contents = "# Project Requirements Document\n\n## Overview\n\nTest.\n\n## Implementation Tasks\n\n### Task TST-1\n\n- **ID** TST-1\n- **Context Bundle** `ARCHITECTURE.md`\n- **DoD** Example.\n- **Checklist**\n  * Example\n- **Dependencies** None\n- [ ] TST-1 Example task\n";
    let fake = FakeCli::new("codex", prd_contents, "", 0).unwrap();
    test_env.prepend_path(fake.bin_dir().to_path_buf());

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gralph");
    cmd.current_dir(&project);
    test_env.apply(&mut cmd);
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

    // prd create now spawns a background tmux session and returns immediately
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PRD generation started in background"))
        .stdout(predicate::str::contains("Session:"))
        .stdout(predicate::str::contains("Tmux session:"))
        .stdout(predicate::str::contains("Output:"))
        .stdout(predicate::str::contains("Logs:"))
        .stdout(predicate::str::contains("Check status: gralph status"));

    // The .gralph directory should be created for logs
    assert!(project.join(".gralph").is_dir());
}
