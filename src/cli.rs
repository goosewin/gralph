use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gralph", version, about = "Rust port of the gralph CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Start(StartArgs),
    Stop(StopArgs),
    Status,
    Logs(LogsArgs),
    Resume(ResumeArgs),
    Prd(PrdArgs),
    Worktree(WorktreeArgs),
    Backends,
    Config(ConfigArgs),
    Server(ServerArgs),
    Version,
    #[command(hide = true)]
    RunLoop(RunLoopArgs),
}

#[derive(Args, Debug, Clone)]
pub struct StartArgs {
    pub dir: PathBuf,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(long)]
    pub max_iterations: Option<u32>,
    #[arg(short = 'f', long)]
    pub task_file: Option<String>,
    #[arg(long)]
    pub completion_marker: Option<String>,
    #[arg(short = 'b', long)]
    pub backend: Option<String>,
    #[arg(short = 'm', long)]
    pub model: Option<String>,
    #[arg(long)]
    pub variant: Option<String>,
    #[arg(long)]
    pub prompt_template: Option<PathBuf>,
    #[arg(long)]
    pub webhook: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub no_tmux: bool,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub strict_prd: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RunLoopArgs {
    pub dir: PathBuf,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub max_iterations: Option<u32>,
    #[arg(long)]
    pub task_file: Option<String>,
    #[arg(long)]
    pub completion_marker: Option<String>,
    #[arg(long)]
    pub backend: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub variant: Option<String>,
    #[arg(long)]
    pub prompt_template: Option<PathBuf>,
    #[arg(long)]
    pub webhook: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub strict_prd: bool,
}

#[derive(Args, Debug)]
pub struct StopArgs {
    pub name: Option<String>,
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct LogsArgs {
    pub name: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub follow: bool,
}

#[derive(Args, Debug)]
pub struct ResumeArgs {
    pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct PrdArgs {
    #[command(subcommand)]
    pub command: PrdCommand,
}

#[derive(Subcommand, Debug)]
pub enum PrdCommand {
    Check(PrdCheckArgs),
    Create(PrdCreateArgs),
}

#[derive(Args, Debug)]
pub struct PrdCheckArgs {
    pub file: PathBuf,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub allow_missing_context: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PrdCreateArgs {
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub goal: Option<String>,
    #[arg(long)]
    pub constraints: Option<String>,
    #[arg(long)]
    pub context: Option<String>,
    #[arg(long)]
    pub sources: Option<String>,
    #[arg(short = 'b', long)]
    pub backend: Option<String>,
    #[arg(short = 'm', long)]
    pub model: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub allow_missing_context: bool,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub multiline: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "interactive")]
    pub no_interactive: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "no_interactive")]
    pub interactive: bool,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct WorktreeArgs {
    #[command(subcommand)]
    pub command: WorktreeCommand,
}

#[derive(Subcommand, Debug)]
pub enum WorktreeCommand {
    Create(WorktreeCreateArgs),
    Finish(WorktreeFinishArgs),
}

#[derive(Args, Debug)]
pub struct WorktreeCreateArgs {
    pub id: String,
}

#[derive(Args, Debug)]
pub struct WorktreeFinishArgs {
    pub id: String,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    Get(ConfigGetArgs),
    Set(ConfigSetArgs),
    List,
}

#[derive(Args, Debug)]
pub struct ConfigGetArgs {
    pub key: String,
}

#[derive(Args, Debug)]
pub struct ConfigSetArgs {
    pub key: String,
    pub value: String,
}

#[derive(Args, Debug)]
pub struct ServerArgs {
    #[arg(short = 'H', long)]
    pub host: Option<String>,
    #[arg(short = 'p', long)]
    pub port: Option<u16>,
    #[arg(short = 't', long)]
    pub token: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub open: bool,
}
