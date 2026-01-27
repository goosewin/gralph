use clap::CommandFactory;
use clap_complete::{
    generate,
    shells::{Bash, Zsh},
};
use std::fs::File;
use std::path::PathBuf;

#[path = "src/cli.rs"]
mod cli;

fn main() {
    println!("cargo:rerun-if-changed=src/cli.rs");
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let completions_dir = manifest_dir.join("completions");
    let _ = std::fs::create_dir_all(&completions_dir);

    let mut cmd = cli::Cli::command();
    if let Ok(mut file) = File::create(completions_dir.join("gralph.bash")) {
        let _ = generate(Bash, &mut cmd, "gralph", &mut file);
    }

    let mut cmd = cli::Cli::command();
    if let Ok(mut file) = File::create(completions_dir.join("gralph.zsh")) {
        let _ = generate(Zsh, &mut cmd, "gralph", &mut file);
    }
}
