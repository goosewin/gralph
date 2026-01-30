use clap::Parser;
use gralph_rs::{cli::Cli, exit_code_for, run, Deps};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let deps = Deps::real();
    exit_code_for(run(cli, &deps))
}
