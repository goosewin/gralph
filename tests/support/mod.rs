use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct FakeCli {
    temp_dir: TempDir,
    bin_name: String,
    bin_path: PathBuf,
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

        Ok(Self {
            temp_dir,
            bin_name,
            bin_path,
        })
    }

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

        Ok(Self {
            temp_dir,
            bin_name,
            bin_path,
        })
    }

    pub fn command(&self) -> &str {
        &self.bin_name
    }

    pub fn bin_path(&self) -> &Path {
        &self.bin_path
    }

    pub fn prepend_to_path(&self) -> io::Result<PathGuard> {
        prepend_to_path(self.temp_dir.path())
    }
}

pub struct PathGuard {
    original: Option<OsString>,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => env::set_var("PATH", value),
            None => env::remove_var("PATH"),
        }
    }
}

fn prepend_to_path(dir: &Path) -> io::Result<PathGuard> {
    let original = env::var_os("PATH");
    let mut paths = Vec::new();
    paths.push(dir.to_path_buf());
    if let Some(existing) = &original {
        paths.extend(env::split_paths(existing));
    }
    let joined =
        env::join_paths(paths).map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    env::set_var("PATH", joined);
    Ok(PathGuard { original })
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
        script.push_str("echo ");
        script.push_str(line);
        script.push_str(" 1>&2\r\n");
    }
    script.push_str(&format!("exit /b {}\r\n", exit_code));
    script
}
