pub mod backend;
pub mod cli;
pub mod config;
pub mod core;
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
use clap::Parser;
use std::process::ExitCode;

pub fn cli_entrypoint() -> ExitCode {
    cli_entrypoint_from(std::env::args_os())
}

fn cli_entrypoint_from<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = cli::Cli::parse_from(args);
    let deps = Deps::real();
    exit_code_for(run(cli, &deps))
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::cli_entrypoint_from;
    use crate::{backend, config, core, notify, prd, server, state, task, update, version};
    use std::process::ExitCode;

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
}
