using System.CommandLine;
using Gralph.Backends;
using Gralph.Commands;
using Gralph.Prd;
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
var stopNameArgument = new Argument<string>("name") { Arity = ArgumentArity.ZeroOrOne };
var stopAllOption = new Option<bool>("--all", "Stop all loops");
stop.Add(stopNameArgument);
stop.Add(stopAllOption);
stop.SetAction(parseResult =>
{
    var handler = new StopCommandHandler(new StateStore());
    var exitCode = handler.Execute(new StopCommandSettings
    {
        Name = parseResult.GetValue(stopNameArgument),
        All = parseResult.GetValue(stopAllOption)
    });

    Environment.ExitCode = exitCode;
});

var status = new Command("status", "Show status of all loops");
status.SetAction(_ =>
{
    var handler = new StatusCommandHandler(new StateStore());
    var exitCode = handler.Execute();
    Environment.ExitCode = exitCode;
});

var logs = new Command("logs", "View logs for a loop");
var logsNameArgument = new Argument<string>("name") { Arity = ArgumentArity.ExactlyOne };
var logsFollowOption = new Option<bool>("--follow", "Follow log output");
logs.Add(logsNameArgument);
logs.Add(logsFollowOption);
logs.SetAction(parseResult =>
{
    var handler = new LogsCommandHandler(new StateStore());
    var exitCode = handler.Execute(new LogsCommandSettings
    {
        Name = parseResult.GetValue(logsNameArgument),
        Follow = parseResult.GetValue(logsFollowOption)
    });

    Environment.ExitCode = exitCode;
});

var resume = new Command("resume", "Resume crashed/stopped loops");
var resumeNameArgument = new Argument<string>("name") { Arity = ArgumentArity.ZeroOrOne };
resume.Add(resumeNameArgument);
resume.SetAction(parseResult =>
{
    var handler = new ResumeCommandHandler(BackendRegistry.CreateDefault(), new StateStore());
    var exitCode = handler.Execute(new ResumeCommandSettings
    {
        Name = parseResult.GetValue(resumeNameArgument)
    });

    Environment.ExitCode = exitCode;
});

var prd = new Command("prd", "Validate or generate PRDs");
var prdCheck = new Command("check", "Validate PRD task blocks");
var prdCheckFileArgument = new Argument<string>("file") { Arity = ArgumentArity.ExactlyOne };
var prdCheckAllowMissingContextOption = new Option<bool>("--allow-missing-context", "Allow missing Context Bundle paths");
prdCheck.Add(prdCheckFileArgument);
prdCheck.Add(prdCheckAllowMissingContextOption);
prdCheck.SetAction(parseResult =>
{
    var handler = new PrdCheckCommandHandler();
    var exitCode = handler.Execute(new PrdCheckSettings
    {
        FilePath = parseResult.GetValue(prdCheckFileArgument),
        AllowMissingContext = parseResult.GetValue(prdCheckAllowMissingContextOption)
    });

    Environment.ExitCode = exitCode;
});
var prdCreate = new Command("create", "Generate a spec-compliant PRD");
var prdCreateDirOption = new Option<string?>("--dir", "Project directory (default: current)");
var prdCreateOutputOption = new Option<string?>("--output", "Output PRD file path (default: PRD.generated.md)");
var prdCreateOutputShortOption = new Option<string?>("-o", "Output PRD file path (default: PRD.generated.md)");
var prdCreateGoalOption = new Option<string?>("--goal", "Short description of what to build");
var prdCreateConstraintsOption = new Option<string?>("--constraints", "Constraints or non-functional requirements");
var prdCreateContextOption = new Option<string?>("--context", "Extra context files (comma-separated)");
var prdCreateSourcesOption = new Option<string?>("--sources", "External URLs or references (comma-separated)");
var prdCreateBackendOption = new Option<string?>("--backend", "Backend for PRD generation");
var prdCreateBackendShortOption = new Option<string?>("-b", "Backend for PRD generation");
var prdCreateModelOption = new Option<string?>("--model", "Model override for PRD generation");
var prdCreateModelShortOption = new Option<string?>("-m", "Model override for PRD generation");
var prdCreateAllowMissingContextOption = new Option<bool>("--allow-missing-context", "Allow missing Context Bundle paths");
var prdCreateMultilineOption = new Option<bool>("--multiline", "Enable multiline prompts (interactive)");
var prdCreateNoInteractiveOption = new Option<bool>("--no-interactive", "Disable interactive prompts");
var prdCreateInteractiveOption = new Option<bool>("--interactive", "Force interactive prompts");
var prdCreateForceOption = new Option<bool>("--force", "Overwrite existing output file");

prdCreate.Add(prdCreateDirOption);
prdCreate.Add(prdCreateOutputOption);
prdCreate.Add(prdCreateOutputShortOption);
prdCreate.Add(prdCreateGoalOption);
prdCreate.Add(prdCreateConstraintsOption);
prdCreate.Add(prdCreateContextOption);
prdCreate.Add(prdCreateSourcesOption);
prdCreate.Add(prdCreateBackendOption);
prdCreate.Add(prdCreateBackendShortOption);
prdCreate.Add(prdCreateModelOption);
prdCreate.Add(prdCreateModelShortOption);
prdCreate.Add(prdCreateAllowMissingContextOption);
prdCreate.Add(prdCreateMultilineOption);
prdCreate.Add(prdCreateNoInteractiveOption);
prdCreate.Add(prdCreateInteractiveOption);
prdCreate.Add(prdCreateForceOption);

prdCreate.SetAction(parseResult =>
{
    var noInteractive = parseResult.GetValue(prdCreateNoInteractiveOption);
    var interactive = parseResult.GetValue(prdCreateInteractiveOption);
    if (noInteractive && interactive)
    {
        Console.Error.WriteLine("Error: --interactive and --no-interactive cannot be used together.");
        Environment.ExitCode = 1;
        return;
    }

    bool? interactiveSetting = null;
    if (noInteractive)
    {
        interactiveSetting = false;
    }
    else if (interactive)
    {
        interactiveSetting = true;
    }

    var handler = new PrdCreateCommandHandler(BackendRegistry.CreateDefault());
    var exitCode = handler.ExecuteAsync(new PrdCreateSettings
    {
        Directory = parseResult.GetValue(prdCreateDirOption),
        Output = parseResult.GetValue(prdCreateOutputOption) ?? parseResult.GetValue(prdCreateOutputShortOption),
        Goal = parseResult.GetValue(prdCreateGoalOption),
        Constraints = parseResult.GetValue(prdCreateConstraintsOption),
        Context = parseResult.GetValue(prdCreateContextOption),
        Sources = parseResult.GetValue(prdCreateSourcesOption),
        Backend = parseResult.GetValue(prdCreateBackendOption) ?? parseResult.GetValue(prdCreateBackendShortOption),
        Model = parseResult.GetValue(prdCreateModelOption) ?? parseResult.GetValue(prdCreateModelShortOption),
        AllowMissingContext = parseResult.GetValue(prdCreateAllowMissingContextOption),
        Multiline = parseResult.GetValue(prdCreateMultilineOption),
        Force = parseResult.GetValue(prdCreateForceOption),
        Interactive = interactiveSetting
    }, CancellationToken.None).GetAwaiter().GetResult();

    Environment.ExitCode = exitCode;
});
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
var serverHostOption = new Option<string?>("--host", "Host/IP to bind to");
var serverHostShortOption = new Option<string?>("-H", "Host/IP to bind to");
var serverPortOption = new Option<int?>("--port", "Port number");
var serverPortShortOption = new Option<int?>("-p", "Port number");
var serverTokenOption = new Option<string?>("--token", "Authentication token");
var serverTokenShortOption = new Option<string?>("-t", "Authentication token");
var serverOpenOption = new Option<bool>("--open", "Disable token requirement (use with caution)");
server.Add(serverHostOption);
server.Add(serverHostShortOption);
server.Add(serverPortOption);
server.Add(serverPortShortOption);
server.Add(serverTokenOption);
server.Add(serverTokenShortOption);
server.Add(serverOpenOption);
server.SetAction(parseResult =>
{
    var handler = new ServerCommandHandler(new StateStore());
    var exitCode = handler.ExecuteAsync(new ServerCommandSettings
    {
        Host = parseResult.GetValue(serverHostOption) ?? parseResult.GetValue(serverHostShortOption),
        Port = parseResult.GetValue(serverPortOption) ?? parseResult.GetValue(serverPortShortOption),
        Token = parseResult.GetValue(serverTokenOption) ?? parseResult.GetValue(serverTokenShortOption),
        Open = parseResult.GetValue(serverOpenOption)
    }, CancellationToken.None).GetAwaiter().GetResult();

    Environment.ExitCode = exitCode;
});

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
