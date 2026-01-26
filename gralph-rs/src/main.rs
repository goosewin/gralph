use clap::Parser;

#[derive(Parser)]
#[command(name = "gralph", version, about = "Rust port of the gralph CLI")]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
