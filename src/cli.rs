use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

pub const ASCII_BANNER: &str = r#"  ___  ____    __    __    ____  _   _
 / __)(  _ \  /__\  (  )  (  _ \( )_( )
( (_-. )   / /(__)\  )(__  )___/ ) _ (
 \___/(_)\_)(__)(__)(____)(__)  (_) (_)
"#;

const ROOT_AFTER_HELP: &str = r#"START OPTIONS:
  --name, -n          Session name (default: directory name)
  --max-iterations    Max iterations before giving up (default: 30)
  --task-file, -f     Task file path (default: PRD.md)
  --completion-marker Completion promise text (default: COMPLETE)
  --backend, -b       AI backend (default: claude). See `gralph backends`
  --model, -m         Model override (format depends on backend)
  --variant           Model variant override (backend-specific)
  --prompt-template   Path to custom prompt template file
  --webhook           Notification webhook URL
  --no-worktree       Disable automatic worktree creation
  --no-tmux           Run in foreground (blocks)
  --strict-prd        Validate PRD before starting the loop

PRD OPTIONS:
  --dir               Project directory (default: current)
  --output, -o        Output PRD file path (default: PRD.generated.md)
  --goal              Short description of what to build
  --constraints       Constraints or non-functional requirements
  --context           Extra context files (comma-separated)
  --sources           External URLs or references (comma-separated)
  --backend, -b        Backend for PRD generation (default: config/default)
  --model, -m          Model override for PRD generation
  --variant           Model variant override (backend-specific)
  --allow-missing-context Allow missing Context Bundle paths
  --multiline         Enable multiline prompts (interactive)
  --no-interactive    Disable interactive prompts
  --interactive       Force interactive prompts
  --force             Overwrite existing output file

INIT OPTIONS:
  --dir               Target directory (default: current)
  --force             Overwrite existing files

SERVER OPTIONS:
  --host, -H            Host/IP to bind to (default: 127.0.0.1)
  --port, -p            Port number (default: 8080)
  --token, -t           Authentication token (required for non-localhost)
  --open                Disable token requirement (use with caution)

EXAMPLES:
  gralph start .
  gralph start ~/project --name myapp --max-iterations 50
  gralph status
  gralph logs myapp --follow
  gralph stop myapp
  gralph prd create --dir . --output PRD.new.md --goal "Add a billing dashboard"
  gralph init --dir .
  gralph worktree create C-1
  gralph worktree finish C-1
  gralph verifier --dir .
  gralph server --host 0.0.0.0 --port 8080
"#;

#[derive(Parser, Debug)]
#[command(
    name = "gralph",
    version,
    about = "Autonomous AI coding loops",
    long_about = "Autonomous AI coding loops",
    before_help = ASCII_BANNER,
    after_help = ROOT_AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "Start a new gralph loop")]
    Start(StartArgs),
    #[command(about = "Stop a running loop")]
    Stop(StopArgs),
    #[command(about = "Show status of all loops")]
    Status,
    #[command(about = "View logs for a loop")]
    Logs(LogsArgs),
    #[command(about = "Resume crashed/stopped loops")]
    Resume(ResumeArgs),
    #[command(about = "Initialize shared context files")]
    Init(InitArgs),
    #[command(about = "Generate or validate PRDs")]
    Prd(PrdArgs),
    #[command(about = "Manage task worktrees")]
    Worktree(WorktreeArgs),
    #[command(about = "List available AI backends")]
    Backends,
    #[command(about = "Manage configuration")]
    Config(ConfigArgs),
    #[command(about = "Run verifier quality gates")]
    Verifier(VerifierArgs),
    #[command(about = "Start status API server")]
    Server(ServerArgs),
    #[command(about = "Show version")]
    Version,
    #[command(about = "Install the latest release")]
    Update,
    #[command(hide = true)]
    RunLoop(RunLoopArgs),
}

#[derive(Args, Debug, Clone)]
pub struct StartArgs {
    #[arg(value_name = "DIR", help = "Project directory to run the loop in")]
    pub dir: PathBuf,
    #[arg(short, long, help = "Session name (default: directory name)")]
    pub name: Option<String>,
    #[arg(long, help = "Max iterations before giving up (default: 30)")]
    pub max_iterations: Option<u32>,
    #[arg(short = 'f', long, help = "Task file path (default: PRD.md)")]
    pub task_file: Option<String>,
    #[arg(long, help = "Completion promise text (default: COMPLETE)")]
    pub completion_marker: Option<String>,
    #[arg(short = 'b', long, help = "AI backend (default: claude)")]
    pub backend: Option<String>,
    #[arg(short = 'm', long, help = "Model override (format depends on backend)")]
    pub model: Option<String>,
    #[arg(long, help = "Model variant override (backend-specific)")]
    pub variant: Option<String>,
    #[arg(long, help = "Path to custom prompt template file")]
    pub prompt_template: Option<PathBuf>,
    #[arg(long, help = "Notification webhook URL")]
    pub webhook: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Disable automatic worktree creation")]
    pub no_worktree: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Run in foreground (blocks)")]
    pub no_tmux: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Validate PRD before starting the loop")]
    pub strict_prd: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RunLoopArgs {
    #[arg(value_name = "DIR")]
    pub dir: PathBuf,
    #[arg(long, help = "Session name")]
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
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Disable automatic worktree creation")]
    pub no_worktree: bool,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub strict_prd: bool,
}

#[derive(Args, Debug)]
pub struct StopArgs {
    #[arg(value_name = "NAME", help = "Session name")]
    pub name: Option<String>,
    #[arg(short, long, action = clap::ArgAction::SetTrue, help = "Stop all loops")]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct LogsArgs {
    #[arg(value_name = "NAME", help = "Session name")]
    pub name: String,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Follow log output")]
    pub follow: bool,
}

#[derive(Args, Debug)]
pub struct ResumeArgs {
    #[arg(value_name = "NAME", help = "Session name")]
    pub name: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    #[arg(long, help = "Target directory (default: current)")]
    pub dir: Option<PathBuf>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Overwrite existing files")]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct PrdArgs {
    #[command(subcommand)]
    pub command: PrdCommand,
}

#[derive(Subcommand, Debug)]
pub enum PrdCommand {
    #[command(about = "Validate PRD task blocks")]
    Check(PrdCheckArgs),
    #[command(about = "Generate a spec-compliant PRD")]
    Create(PrdCreateArgs),
}

#[derive(Args, Debug)]
pub struct PrdCheckArgs {
    #[arg(value_name = "FILE", help = "PRD file to validate")]
    pub file: PathBuf,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Allow missing Context Bundle paths")]
    pub allow_missing_context: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PrdCreateArgs {
    #[arg(long, help = "Project directory (default: current)")]
    pub dir: Option<PathBuf>,
    #[arg(
        short = 'o',
        long,
        help = "Output PRD file path (default: PRD.generated.md)"
    )]
    pub output: Option<PathBuf>,
    #[arg(long, help = "Short description of what to build")]
    pub goal: Option<String>,
    #[arg(long, help = "Constraints or non-functional requirements")]
    pub constraints: Option<String>,
    #[arg(long, help = "Extra context files (comma-separated)")]
    pub context: Option<String>,
    #[arg(long, help = "External URLs or references (comma-separated)")]
    pub sources: Option<String>,
    #[arg(
        short = 'b',
        long,
        help = "Backend for PRD generation (default: config/default)"
    )]
    pub backend: Option<String>,
    #[arg(short = 'm', long, help = "Model override for PRD generation")]
    pub model: Option<String>,
    #[arg(long, help = "Model variant override (backend-specific)")]
    pub variant: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Allow missing Context Bundle paths")]
    pub allow_missing_context: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Enable multiline prompts (interactive)")]
    pub multiline: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "interactive", help = "Disable interactive prompts")]
    pub no_interactive: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "no_interactive", help = "Force interactive prompts")]
    pub interactive: bool,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Overwrite existing output file")]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct WorktreeArgs {
    #[command(subcommand)]
    pub command: WorktreeCommand,
}

#[derive(Subcommand, Debug)]
pub enum WorktreeCommand {
    #[command(about = "Create task worktree")]
    Create(WorktreeCreateArgs),
    #[command(about = "Finish task worktree")]
    Finish(WorktreeFinishArgs),
}

#[derive(Args, Debug)]
pub struct WorktreeCreateArgs {
    #[arg(value_name = "ID", help = "Task ID (e.g. C-1)")]
    pub id: String,
}

#[derive(Args, Debug)]
pub struct WorktreeFinishArgs {
    #[arg(value_name = "ID", help = "Task ID (e.g. C-1)")]
    pub id: String,
}

#[derive(Args, Debug, Clone)]
pub struct VerifierArgs {
    #[arg(
        value_name = "DIR",
        help = "Project directory to verify (default: current)"
    )]
    pub dir: Option<PathBuf>,
    #[arg(long, help = "Override test command")]
    pub test_command: Option<String>,
    #[arg(long, help = "Override coverage command")]
    pub coverage_command: Option<String>,
    #[arg(long, help = "Minimum coverage percent (default: 90)")]
    pub coverage_min: Option<f64>,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    #[command(about = "Get config value")]
    Get(ConfigGetArgs),
    #[command(about = "Set config value")]
    Set(ConfigSetArgs),
    #[command(about = "List config values")]
    List,
}

#[derive(Args, Debug)]
pub struct ConfigGetArgs {
    #[arg(value_name = "KEY", help = "Config key")]
    pub key: String,
}

#[derive(Args, Debug)]
pub struct ConfigSetArgs {
    #[arg(value_name = "KEY", help = "Config key")]
    pub key: String,
    #[arg(value_name = "VALUE", help = "Config value")]
    pub value: String,
}

#[derive(Args, Debug)]
pub struct ServerArgs {
    #[arg(short = 'H', long, help = "Host/IP to bind to (default: 127.0.0.1)")]
    pub host: Option<String>,
    #[arg(short = 'p', long, help = "Port number (default: 8080)")]
    pub port: Option<u16>,
    #[arg(
        short = 't',
        long,
        help = "Authentication token (required for non-localhost)"
    )]
    pub token: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Disable token requirement (use with caution)")]
    pub open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_update_command() {
        let cli = Cli::parse_from(["gralph", "update"]);
        match cli.command {
            Some(Command::Update) => {}
            other => panic!("Expected update command, got: {other:?}"),
        }
    }
}
