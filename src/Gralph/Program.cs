using System.CommandLine;

const string Version = "1.1.0";

var helpText = """
  ___  ____    __    __    ____  _   _
 / __)(  _ \  /__\  (  )  (  _ \( )_( )
( (_-. )   / /(__)\  )(__  )___/ ) _ (
 \___/(_)\_)(__)(__)(____)(__)  (_) (_)

gralph - Autonomous AI coding loops

USAGE:
  gralph <command> [options]

COMMANDS:
  start <dir>         Start a new gralph loop
  stop <name>         Stop a running loop
  stop --all          Stop all loops
  status              Show status of all loops
  logs <name>         View logs for a loop
  resume [name]       Resume crashed/stopped loops
  prd check <file>    Validate PRD task blocks
  prd create          Generate a spec-compliant PRD
  worktree create <ID> Create task worktree
  worktree finish <ID> Finish task worktree
  backends            List available AI backends
  config              Manage configuration
  server              Start status API server
  version             Show version
  help                Show this help message

START OPTIONS:
  --name, -n          Session name (default: directory name)
  --max-iterations    Max iterations before giving up (default: 30)
  --task-file, -f     Task file path (default: PRD.md)
  --completion-marker Completion promise text (default: COMPLETE)
  --backend, -b       AI backend (default: claude). See `gralph backends`
  --model, -m         Model override (format depends on backend)
  --variant           Model variant override (backend-specific)
  --prompt-template   Path to custom prompt template file
  --webhook           Notification webhook URL
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
  --allow-missing-context Allow missing Context Bundle paths
  --multiline         Enable multiline prompts (interactive)
  --no-interactive    Disable interactive prompts
  --interactive       Force interactive prompts
  --force             Overwrite existing output file

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
  gralph worktree create C-1
  gralph worktree finish C-1
  gralph server --host 0.0.0.0 --port 8080
""";

var versionOption = new Option<bool>("--version", new[] { "-v" })
{
    Arity = ArgumentArity.Zero,
    Recursive = true
};

var helpOption = new Option<bool>("--help", new[] { "-h" })
{
    Arity = ArgumentArity.Zero,
    Recursive = true
};

var root = new RootCommand("gralph - Autonomous AI coding loops");
root.Add(versionOption);
root.Add(helpOption);
root.SetAction(_ => Console.WriteLine(helpText));

var start = new Command("start", "Start a new gralph loop");
start.Add(new Argument<string>("dir") { Arity = ArgumentArity.ExactlyOne });
start.SetAction(_ => Console.WriteLine("start is not implemented yet."));

var stop = new Command("stop", "Stop a running loop");
stop.Add(new Argument<string>("name") { Arity = ArgumentArity.ZeroOrOne });
stop.SetAction(_ => Console.WriteLine("stop is not implemented yet."));

var status = new Command("status", "Show status of all loops");
status.SetAction(_ => Console.WriteLine("status is not implemented yet."));

var logs = new Command("logs", "View logs for a loop");
logs.Add(new Argument<string>("name") { Arity = ArgumentArity.ExactlyOne });
logs.SetAction(_ => Console.WriteLine("logs is not implemented yet."));

var resume = new Command("resume", "Resume crashed/stopped loops");
resume.Add(new Argument<string>("name") { Arity = ArgumentArity.ZeroOrOne });
resume.SetAction(_ => Console.WriteLine("resume is not implemented yet."));

var prd = new Command("prd", "Validate or generate PRDs");
var prdCheck = new Command("check", "Validate PRD task blocks");
prdCheck.Add(new Argument<string>("file") { Arity = ArgumentArity.ExactlyOne });
prdCheck.SetAction(_ => Console.WriteLine("prd check is not implemented yet."));
var prdCreate = new Command("create", "Generate a spec-compliant PRD");
prdCreate.SetAction(_ => Console.WriteLine("prd create is not implemented yet."));
prd.Add(prdCheck);
prd.Add(prdCreate);

var worktree = new Command("worktree", "Manage git worktrees");
var worktreeCreate = new Command("create", "Create task worktree");
worktreeCreate.Add(new Argument<string>("id") { Arity = ArgumentArity.ExactlyOne });
worktreeCreate.SetAction(_ => Console.WriteLine("worktree create is not implemented yet."));
var worktreeFinish = new Command("finish", "Finish task worktree");
worktreeFinish.Add(new Argument<string>("id") { Arity = ArgumentArity.ExactlyOne });
worktreeFinish.SetAction(_ => Console.WriteLine("worktree finish is not implemented yet."));
worktree.Add(worktreeCreate);
worktree.Add(worktreeFinish);

var backends = new Command("backends", "List available AI backends");
backends.SetAction(_ => Console.WriteLine("backends is not implemented yet."));

var config = new Command("config", "Manage configuration");
config.SetAction(_ => Console.WriteLine("config is not implemented yet."));

var server = new Command("server", "Start status API server");
server.SetAction(_ => Console.WriteLine("server is not implemented yet."));

var version = new Command("version", "Show version");
version.SetAction(_ => Console.WriteLine(Version));

var help = new Command("help", "Show this help message");
help.SetAction(_ => Console.WriteLine(helpText));

root.Add(start);
root.Add(stop);
root.Add(status);
root.Add(logs);
root.Add(resume);
root.Add(prd);
root.Add(worktree);
root.Add(backends);
root.Add(config);
root.Add(server);
root.Add(version);
root.Add(help);

if (args.Length == 0)
{
    Console.WriteLine(helpText);
    return 0;
}

var parseResult = root.Parse(args, new ParserConfiguration { EnablePosixBundling = true });

if (parseResult.GetValue(helpOption))
{
    Console.WriteLine(helpText);
    return 0;
}

if (parseResult.GetValue(versionOption))
{
    Console.WriteLine(Version);
    return 0;
}

var invocation = new InvocationConfiguration
{
    Output = Console.Out,
    Error = Console.Error
};

return await parseResult.InvokeAsync(invocation, CancellationToken.None);
