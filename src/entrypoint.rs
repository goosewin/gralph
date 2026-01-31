use crate::app::{Deps, exit_code_for, run};
use crate::cli;
use clap::Parser;
use std::process::ExitCode;

pub fn cli_entrypoint() -> ExitCode {
    cli_entrypoint_from(std::env::args_os())
}

pub(crate) fn cli_entrypoint_from<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = cli::Cli::parse_from(args);
    let deps = Deps::real();
    exit_code_for(run(cli, &deps))
}
