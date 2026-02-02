//! Integration tests for PRD generation retry loop.
//!
//! These tests verify that:
//! 1. A mock backend returning invalid PRD first, valid second triggers retries.
//! 2. Retry limit exhaustion returns the best attempt.
//! 3. A valid first attempt returns immediately without retry.

use predicates::prelude::*;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod support;

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

fn valid_prd_content(context_file: &str) -> String {
    format!(
        r#"# Test PRD

## Overview

Test project.

## Problem Statement

- Test problem.

## Solution

Test solution.

---

## Implementation Tasks

### Task TEST-1

- **ID** TEST-1
- **Context Bundle** `{}`
- **DoD** Test done.
- **Checklist**
  * Item one.
- **Dependencies** None
- [ ] TEST-1 Test task
---

## Success Criteria

- Test passes.
"#,
        context_file
    )
}

fn invalid_prd_missing_context_bundle() -> String {
    r#"# Test PRD

## Overview

Test project.

---

## Implementation Tasks

### Task TEST-1

- **ID** TEST-1
- **DoD** Test done.
- **Checklist**
  * Item one.
- **Dependencies** None
- [ ] TEST-1 Test task
---
"#
    .to_string()
}

/// Creates a stateful shell script that returns different outputs on each invocation.
/// Uses a counter file to track the invocation number.
fn create_stateful_fake_cli(
    base_dir: &Path,
    name: &str,
    responses: &[&str],
) -> std::io::Result<PathBuf> {
    let bin_dir = base_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;

    let counter_file = base_dir.join("counter");
    fs::write(&counter_file, "0")?;

    // Write each response to a numbered file
    for (i, response) in responses.iter().enumerate() {
        let response_file = base_dir.join(format!("response_{}.txt", i));
        fs::write(&response_file, response)?;
    }

    let script_path = bin_dir.join(script_name(name));

    #[cfg(unix)]
    {
        let script = format!(
            r#"#!/bin/sh
COUNTER_FILE="{counter}"
RESPONSE_DIR="{response_dir}"
COUNT=$(cat "$COUNTER_FILE" 2>/dev/null || echo 0)
NEXT_COUNT=$((COUNT + 1))
echo "$NEXT_COUNT" > "$COUNTER_FILE"
RESPONSE_FILE="$RESPONSE_DIR/response_$COUNT.txt"
if [ -f "$RESPONSE_FILE" ]; then
    cat "$RESPONSE_FILE"
else
    # Return last available response if we exceed count
    LAST_IDX=$(($(ls -1 "$RESPONSE_DIR"/response_*.txt 2>/dev/null | wc -l) - 1))
    if [ $LAST_IDX -ge 0 ]; then
        cat "$RESPONSE_DIR/response_$LAST_IDX.txt"
    fi
fi
exit 0
"#,
            counter = counter_file.display(),
            response_dir = base_dir.display(),
        );
        fs::write(&script_path, script)?;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    #[cfg(windows)]
    {
        // Windows batch script version
        let script = format!(
            r#"@echo off
setlocal enabledelayedexpansion
set COUNTER_FILE={counter}
set RESPONSE_DIR={response_dir}
set /p COUNT=<"%COUNTER_FILE%"
if "%COUNT%"=="" set COUNT=0
set /a NEXT_COUNT=COUNT+1
echo !NEXT_COUNT!>"%COUNTER_FILE%"
set RESPONSE_FILE=%RESPONSE_DIR%\response_%COUNT%.txt
if exist "%RESPONSE_FILE%" (
    type "%RESPONSE_FILE%"
) else (
    for /f %%i in ('dir /b "%RESPONSE_DIR%\response_*.txt" 2^>nul ^| find /c /v ""') do set TOTAL=%%i
    set /a LAST_IDX=TOTAL-1
    if !LAST_IDX! geq 0 type "%RESPONSE_DIR%\response_!LAST_IDX!.txt"
)
exit /b 0
"#,
            counter = counter_file.display(),
            response_dir = base_dir.display(),
        );
        fs::write(&script_path, script)?;
    }

    Ok(bin_dir)
}

fn script_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.cmd", name)
    } else {
        name.to_string()
    }
}

/// Test: Valid first attempt returns immediately without retry.
///
/// When the backend returns a valid PRD on the first attempt, the retry loop
/// should exit immediately and produce a valid PRD.
#[test]
fn prd_create_valid_first_attempt_no_retry() {
    let temp = tempfile::tempdir().unwrap();
    let mut test_env = prepare_env(temp.path());

    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("ARCHITECTURE.md"), "arch\n").unwrap();
    fs::write(project.join("PROCESS.md"), "process\n").unwrap();

    let prd_output = project.join("PRD.generated.md");
    let valid_prd = valid_prd_content("ARCHITECTURE.md");

    // Create fake CLI that returns valid PRD on first call
    let fake_dir = temp.path().join("fake");
    let bin_dir = create_stateful_fake_cli(&fake_dir, "codex", &[&valid_prd]).unwrap();
    test_env.prepend_path(bin_dir);

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
        "--max-retries",
        "3",
        "--output",
    ])
    .arg(&prd_output);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PRD created:"));

    let generated = fs::read_to_string(&prd_output).unwrap();
    assert!(generated.contains("### Task TEST-1"));
    assert!(generated.contains("**Context Bundle**"));

    // Verify counter shows only 1 call was made (no retry)
    let counter = fs::read_to_string(fake_dir.join("counter")).unwrap();
    assert_eq!(counter.trim(), "1", "Expected exactly 1 backend call");
}

/// Test: Mock backend returning invalid PRD first, valid second triggers retries.
///
/// When the first attempt fails validation, the retry loop should retry and
/// succeed when the backend returns a valid PRD on the second attempt.
#[test]
fn prd_create_retries_on_validation_failure() {
    let temp = tempfile::tempdir().unwrap();
    let mut test_env = prepare_env(temp.path());

    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("ARCHITECTURE.md"), "arch\n").unwrap();
    fs::write(project.join("PROCESS.md"), "process\n").unwrap();

    let prd_output = project.join("PRD.generated.md");
    let invalid_prd = invalid_prd_missing_context_bundle();
    let valid_prd = valid_prd_content("ARCHITECTURE.md");

    // Create fake CLI that returns invalid PRD first, valid PRD second
    let fake_dir = temp.path().join("fake");
    let bin_dir =
        create_stateful_fake_cli(&fake_dir, "codex", &[&invalid_prd, &valid_prd]).unwrap();
    test_env.prepend_path(bin_dir);

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
        "--max-retries",
        "3",
        "--output",
    ])
    .arg(&prd_output);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PRD created:"));

    let generated = fs::read_to_string(&prd_output).unwrap();
    assert!(generated.contains("### Task TEST-1"));
    assert!(generated.contains("**Context Bundle**"));

    // Verify counter shows 2 calls were made (1 initial + 1 retry)
    let counter = fs::read_to_string(fake_dir.join("counter")).unwrap();
    assert_eq!(counter.trim(), "2", "Expected 2 backend calls (1 retry)");
}

/// Test: Retry limit exhaustion returns best attempt.
///
/// When all retry attempts return invalid PRDs, the function should return
/// an error but save the best attempt to an .invalid.md file.
#[test]
fn prd_create_retry_exhaustion_returns_best_attempt() {
    let temp = tempfile::tempdir().unwrap();
    let mut test_env = prepare_env(temp.path());

    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("ARCHITECTURE.md"), "arch\n").unwrap();
    fs::write(project.join("PROCESS.md"), "process\n").unwrap();

    let prd_output = project.join("PRD.generated.md");
    let invalid_output = project.join("PRD.generated.invalid.md");
    let invalid_prd = invalid_prd_missing_context_bundle();

    // Create fake CLI that always returns invalid PRD
    let fake_dir = temp.path().join("fake");
    let bin_dir = create_stateful_fake_cli(
        &fake_dir,
        "codex",
        &[&invalid_prd, &invalid_prd, &invalid_prd, &invalid_prd],
    )
    .unwrap();
    test_env.prepend_path(bin_dir);

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
        "--max-retries",
        "2", // 3 total attempts
        "--allow-missing-context",
        "--output",
    ])
    .arg(&prd_output);

    // Should fail but still save best attempt to .invalid.md file
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Retry limit reached"))
        .stderr(predicate::str::contains(
            "failed validation after 2 retries",
        ));

    // The invalid PRD file should be written (best attempt)
    assert!(invalid_output.exists());
    let generated = fs::read_to_string(&invalid_output).unwrap();
    assert!(generated.contains("### Task TEST-1"));

    // The original output path should NOT exist
    assert!(!prd_output.exists());

    // Verify counter shows 3 calls were made (1 initial + 2 retries)
    let counter = fs::read_to_string(fake_dir.join("counter")).unwrap();
    assert_eq!(counter.trim(), "3", "Expected 3 backend calls");
}
