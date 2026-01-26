using System.CommandLine;
using Gralph.Backends;
using Gralph.Commands;
using Gralph.State;

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
var startDirArgument = new Argument<string>("dir") { Arity = ArgumentArity.ExactlyOne };
var startNameOption = new Option<string?>("--name", "Session name (default: directory name)");
var startNameShortOption = new Option<string?>("-n", "Session name (default: directory name)");
var startMaxIterationsOption = new Option<int?>("--max-iterations", "Max iterations before giving up");
var startTaskFileOption = new Option<string?>("--task-file", "Task file path");
var startTaskFileShortOption = new Option<string?>("-f", "Task file path");
var startCompletionMarkerOption = new Option<string?>("--completion-marker", "Completion promise text");
var startBackendOption = new Option<string?>("--backend", "AI backend");
var startBackendShortOption = new Option<string?>("-b", "AI backend");
var startModelOption = new Option<string?>("--model", "Model override");
var startModelShortOption = new Option<string?>("-m", "Model override");
var startVariantOption = new Option<string?>("--variant", "Model variant override");
var startPromptTemplateOption = new Option<string?>("--prompt-template", "Path to custom prompt template file");
var startWebhookOption = new Option<string?>("--webhook", "Notification webhook URL");
var startNoTmuxOption = new Option<bool>("--no-tmux", "Run in foreground (blocks)");
var startStrictPrdOption = new Option<bool>("--strict-prd", "Validate PRD before starting the loop");
var startBackgroundChildOption = new Option<bool>("--background-child");

start.Add(startDirArgument);
start.Add(startNameOption);
start.Add(startNameShortOption);
start.Add(startMaxIterationsOption);
start.Add(startTaskFileOption);
start.Add(startTaskFileShortOption);
start.Add(startCompletionMarkerOption);
start.Add(startBackendOption);
start.Add(startBackendShortOption);
start.Add(startModelOption);
start.Add(startModelShortOption);
start.Add(startVariantOption);
start.Add(startPromptTemplateOption);
start.Add(startWebhookOption);
start.Add(startNoTmuxOption);
start.Add(startStrictPrdOption);
start.Add(startBackgroundChildOption);

start.SetAction(parseResult =>
{
    var handler = new StartCommandHandler(BackendRegistry.CreateDefault(), new StateStore());
    var exitCode = handler.ExecuteAsync(new StartCommandSettings
    {
        Directory = parseResult.GetValue(startDirArgument),
        Name = parseResult.GetValue(startNameOption) ?? parseResult.GetValue(startNameShortOption),
        MaxIterations = parseResult.GetValue(startMaxIterationsOption),
        TaskFile = parseResult.GetValue(startTaskFileOption) ?? parseResult.GetValue(startTaskFileShortOption),
        CompletionMarker = parseResult.GetValue(startCompletionMarkerOption),
        Backend = parseResult.GetValue(startBackendOption) ?? parseResult.GetValue(startBackendShortOption),
        Model = parseResult.GetValue(startModelOption) ?? parseResult.GetValue(startModelShortOption),
        Variant = parseResult.GetValue(startVariantOption),
        PromptTemplatePath = parseResult.GetValue(startPromptTemplateOption),
        Webhook = parseResult.GetValue(startWebhookOption),
        NoTmux = parseResult.GetValue(startNoTmuxOption),
        StrictPrd = parseResult.GetValue(startStrictPrdOption),
        BackgroundChild = parseResult.GetValue(startBackgroundChildOption)
    }, CancellationToken.None).GetAwaiter().GetResult();

    Environment.ExitCode = exitCode;
});

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
