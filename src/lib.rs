pub mod backend;
pub mod cli;
pub mod config;
pub mod core;
mod entrypoint;
pub mod notify;
pub mod prd;
pub mod server;
pub mod state;
pub mod task;
pub mod update;
pub mod version;
mod verifier;

pub mod app;
pub use app::{exit_code_for, run, Deps};
pub use entrypoint::cli_entrypoint;
#[cfg(test)]
pub(crate) use entrypoint::cli_entrypoint_from;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::cli_entrypoint_from;
    use crate::{backend, config, core, notify, prd, server, state, task, update, version};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::ExitCode;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = crate::test_support::env_lock();
        clear_env_overrides();
        guard
    }

    fn clear_env_overrides() {
        for key in [
            "GRALPH_DEFAULT_CONFIG",
            "GRALPH_GLOBAL_CONFIG",
            "GRALPH_PROJECT_CONFIG_NAME",
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT",
        ] {
            remove_env(key);
        }
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env(key: &str) {
        unsafe {
            env::remove_var(key);
        }
    }

    fn set_state_env(root: &Path) -> PathBuf {
        let state_dir = root.join("state");
        set_env("GRALPH_STATE_DIR", &state_dir);
        set_env("GRALPH_STATE_FILE", state_dir.join("state.json"));
        set_env("GRALPH_LOCK_FILE", state_dir.join("state.lock"));
        set_env("GRALPH_LOCK_TIMEOUT", "1");
        state_dir
    }

    fn write_prd(path: &Path) {
        let contents = "# PRD\n\n### Task T-1\n\n- **ID** T-1\n- **Context Bundle** `README.md`\n- **DoD** Do it.\n- **Checklist**\n  * Item\n- **Dependencies** None\n- [ ] T-1 Do it\n";
        write_file(path, contents);
    }

    #[test]
    fn lib_wiring_resolves_backend() {
        assert!(backend::backend_from_name("codex").is_ok());
    }

    #[test]
    fn lib_exposes_expected_modules() {
        let _ = backend::backend_from_name;
        let _ = config::Config::load;
        let _ = core::count_remaining_tasks;
        let _ = notify::notify_failed;
        let _ = prd::prd_detect_stack;
        let _ = server::ServerConfig::from_env;
        let _ = state::StateStore::new_from_env;
        let _ = task::task_blocks_from_contents;
        let _ = update::check_for_update;
        let _ = version::VERSION;
        let _ = version::VERSION_TAG;
    }

    #[test]
    fn cli_entrypoint_from_runs_intro_without_args() {
        let code = cli_entrypoint_from(["gralph"]);
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cli_entrypoint_from_runs_version_command() {
        let code = cli_entrypoint_from(["gralph", "version"]);
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cli_entrypoint_from_runs_config_list() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        write_file(&default_path, "defaults:\n  task_file: PRD.md\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", temp.path().join("missing-global.yaml"));
        set_env(
            "GRALPH_PROJECT_CONFIG_NAME",
            temp.path().join("missing-project.yaml"),
        );

        let code = cli_entrypoint_from(["gralph", "config", "list"]);
        assert_eq!(code, ExitCode::SUCCESS);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_runs_config_get() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        write_file(&default_path, "defaults:\n  task_file: PRD.md\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", temp.path().join("missing-global.yaml"));
        set_env(
            "GRALPH_PROJECT_CONFIG_NAME",
            temp.path().join("missing-project.yaml"),
        );

        let code = cli_entrypoint_from(["gralph", "config", "get", "defaults.task_file"]);
        assert_eq!(code, ExitCode::SUCCESS);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_runs_config_default_list() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        write_file(&default_path, "defaults:\n  task_file: PRD.md\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", temp.path().join("missing-global.yaml"));
        set_env(
            "GRALPH_PROJECT_CONFIG_NAME",
            temp.path().join("missing-project.yaml"),
        );

        let code = cli_entrypoint_from(["gralph", "config"]);
        assert_eq!(code, ExitCode::SUCCESS);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_reports_missing_config_key() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let default_path = temp.path().join("default.yaml");
        write_file(&default_path, "defaults:\n  task_file: PRD.md\n");
        set_env("GRALPH_DEFAULT_CONFIG", &default_path);
        set_env("GRALPH_GLOBAL_CONFIG", temp.path().join("missing-global.yaml"));
        set_env(
            "GRALPH_PROJECT_CONFIG_NAME",
            temp.path().join("missing-project.yaml"),
        );

        let code = cli_entrypoint_from(["gralph", "config", "get", "missing.key"]);
        assert_eq!(code, ExitCode::FAILURE);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_runs_backends_with_empty_path() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let original_path = env::var_os("PATH");
        set_env("PATH", temp.path());

        let code = cli_entrypoint_from(["gralph", "backends"]);
        assert_eq!(code, ExitCode::SUCCESS);

        if let Some(path) = original_path {
            set_env("PATH", path);
        } else {
            remove_env("PATH");
        }
        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_runs_backends_with_stubbed_path() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        for cmd in ["claude", "opencode", "gemini", "codex"] {
            write_file(&temp.path().join(cmd), "stub");
        }
        let original_path = env::var_os("PATH");
        set_env("PATH", temp.path());

        let code = cli_entrypoint_from(["gralph", "backends"]);
        assert_eq!(code, ExitCode::SUCCESS);

        if let Some(path) = original_path {
            set_env("PATH", path);
        } else {
            remove_env("PATH");
        }
        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_runs_status_with_session() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let prd_path = temp.path().join("PRD.md");
        write_prd(&prd_path);

        let store = state::StateStore::new_from_env();
        store.init_state().unwrap();
        let dir_string = temp.path().to_string_lossy().to_string();
        store
            .set_session(
                "demo",
                &[
                    ("dir", &dir_string),
                    ("task_file", "PRD.md"),
                    ("iteration", "2"),
                    ("max_iterations", "5"),
                    ("status", "running"),
                    ("last_task_count", "1"),
                ],
            )
            .unwrap();

        let code = cli_entrypoint_from(["gralph", "status"]);
        assert_eq!(code, ExitCode::SUCCESS);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_reports_no_sessions() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = state::StateStore::new_from_env();
        store.init_state().unwrap();

        let code = cli_entrypoint_from(["gralph", "status"]);
        assert_eq!(code, ExitCode::SUCCESS);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_reports_missing_log_file() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = state::StateStore::new_from_env();
        store.init_state().unwrap();
        let dir_string = temp.path().to_string_lossy().to_string();
        let log_path = temp.path().join("missing.log");
        let log_string = log_path.to_string_lossy().to_string();
        store
            .set_session(
                "demo",
                &[("dir", &dir_string), ("log_file", &log_string)],
            )
            .unwrap();

        let code = cli_entrypoint_from(["gralph", "logs", "demo"]);
        assert_eq!(code, ExitCode::FAILURE);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_reports_missing_verifier_dir() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing-dir");
        let missing_str = missing.to_string_lossy().to_string();

        let code = cli_entrypoint_from(["gralph", "verifier", &missing_str]);
        assert_eq!(code, ExitCode::FAILURE);

        clear_env_overrides();
    }

    #[test]
    fn cli_entrypoint_from_stops_named_session() {
        let _guard = env_guard();
        let temp = tempfile::tempdir().unwrap();
        set_state_env(temp.path());
        let store = state::StateStore::new_from_env();
        store.init_state().unwrap();
        let dir_string = temp.path().to_string_lossy().to_string();
        store
            .set_session(
                "demo",
                &[
                    ("dir", &dir_string),
                    ("status", "running"),
                    ("pid", "0"),
                    ("tmux_session", ""),
                ],
            )
            .unwrap();

        let code = cli_entrypoint_from(["gralph", "stop", "demo"]);
        assert_eq!(code, ExitCode::SUCCESS);

        clear_env_overrides();
    }
}
