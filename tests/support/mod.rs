use std::fs;
use std::io;
use std::path::Path;
use tempfile::TempDir;

pub struct FakeCli {
    temp_dir: TempDir,
    bin_name: String,
}

impl FakeCli {
    pub fn new(name: &str, stdout: &str, stderr: &str, exit_code: i32) -> io::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let bin_name = name.to_string();
        let bin_path = temp_dir.path().join(script_name(name));
        let script = render_script(stdout, stderr, exit_code);
        fs::write(&bin_path, script)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin_path, perms)?;
        }

        Ok(Self { temp_dir, bin_name })
    }

    #[allow(dead_code)]
    pub fn new_script(name: &str, script: &str) -> io::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let bin_name = name.to_string();
        let bin_path = temp_dir.path().join(script_name(name));
        fs::write(&bin_path, script)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin_path, perms)?;
        }

        Ok(Self { temp_dir, bin_name })
    }

    /// Returns the command name (for use with PATH-based lookup).
    pub fn command(&self) -> String {
        script_name(&self.bin_name)
    }

    /// Returns the directory containing the fake CLI binary.
    /// Use this with TestEnv::prepend_path() or Command::env("PATH", ...) to make the fake CLI available.
    pub fn bin_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Returns the full path to the fake CLI binary.
    /// Use this when you want to run the CLI directly without modifying PATH.
    pub fn bin_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join(script_name(&self.bin_name))
    }
}

fn script_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.cmd", name)
    } else {
        name.to_string()
    }
}

fn render_script(stdout: &str, stderr: &str, exit_code: i32) -> String {
    if cfg!(windows) {
        render_windows_script(stdout, stderr, exit_code)
    } else {
        render_unix_script(stdout, stderr, exit_code)
    }
}

fn render_unix_script(stdout: &str, stderr: &str, exit_code: i32) -> String {
    format!(
        "#!/bin/sh\ncat <<'STDOUT'\n{stdout}\nSTDOUT\ncat <<'STDERR' 1>&2\n{stderr}\nSTDERR\nexit {exit_code}\n"
    )
}

fn render_windows_script(stdout: &str, stderr: &str, exit_code: i32) -> String {
    let mut script = String::from("@echo off\r\n");
    for line in stdout.lines() {
        script.push_str("echo ");
        script.push_str(line);
        script.push_str("\r\n");
    }
    for line in stderr.lines() {
        // Use parentheses to avoid trailing space before redirect
        script.push_str("(echo ");
        script.push_str(line);
        script.push_str(")1>&2\r\n");
    }
    script.push_str(&format!("exit /b {}\r\n", exit_code));
    script
}
