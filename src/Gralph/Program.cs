using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Gralph.Backends;
using Gralph.Config;
using Gralph.Prd;

namespace Gralph;

public static class Program
{
    private const string Version = "1.1.0";

    private static readonly string[] CommandNames =
    [
        "start",
        "stop",
        "status",
        "logs",
        "resume",
        "prd",
        "worktree",
        "backends",
        "config",
        "server",
        "version",
        "help"
    ];

    private const string UsageText = "gralph - Autonomous AI coding loops\n\n" +
                                     "USAGE:\n" +
                                     "  gralph <command> [options]\n\n" +
                                     "COMMANDS:\n" +
                                     "  start <dir>          Start a new gralph loop\n" +
                                     "  stop <name>          Stop a running loop\n" +
                                     "  stop --all           Stop all loops\n" +
                                     "  status               Show status of all loops\n" +
                                     "  logs <name>          View logs for a loop\n" +
                                     "  resume [name]        Resume crashed/stopped loops\n" +
                                     "  prd check <file>     Validate PRD task blocks\n" +
                                     "  prd create           Generate a spec-compliant PRD\n" +
                                     "  worktree create <ID> Create task worktree\n" +
                                     "  worktree finish <ID> Finish task worktree\n" +
                                     "  backends             List available AI backends\n" +
                                     "  config               Manage configuration\n" +
                                     "  server               Start status API server\n" +
                                     "  version              Show version\n" +
                                     "  help                 Show this help message\n\n" +
                                     "START OPTIONS:\n" +
                                     "  --name, -n           Session name (default: directory name)\n" +
                                     "  --max-iterations     Max iterations before giving up (default: 30)\n" +
                                     "  --task-file, -f      Task file path (default: PRD.md)\n" +
                                     "  --completion-marker  Completion promise text (default: COMPLETE)\n" +
                                     "  --backend, -b        AI backend (default: claude). See `gralph backends`\n" +
                                     "  --model, -m          Model override (format depends on backend)\n" +
                                     "  --variant            Model variant override (backend-specific)\n" +
                                     "  --prompt-template    Path to custom prompt template file\n" +
                                     "  --webhook            Notification webhook URL\n" +
                                     "  --no-tmux            Run in foreground (blocks)\n" +
                                     "  --strict-prd         Validate PRD before starting the loop\n\n" +
                                     "PRD OPTIONS:\n" +
                                     "  --dir                Project directory (default: current)\n" +
                                     "  --output, -o         Output PRD file path (default: PRD.generated.md)\n" +
                                     "  --goal               Short description of what to build\n" +
                                     "  --constraints        Constraints or non-functional requirements\n" +
                                     "  --context            Extra context files (comma-separated)\n" +
                                     "  --sources            External URLs or references (comma-separated)\n" +
                                     "  --backend, -b         Backend for PRD generation (default: config/default)\n" +
                                     "  --model, -m           Model override for PRD generation\n" +
                                     "  --allow-missing-context Allow missing Context Bundle paths\n" +
                                     "  --multiline          Enable multiline prompts (interactive)\n" +
                                     "  --no-interactive     Disable interactive prompts\n" +
                                     "  --interactive        Force interactive prompts\n" +
                                     "  --force              Overwrite existing output file\n\n" +
                                     "SERVER OPTIONS:\n" +
                                     "  --host, -H           Host/IP to bind to (default: 127.0.0.1)\n" +
                                     "  --port, -p           Port number (default: 8080)\n" +
                                     "  --token, -t          Authentication token (required for non-localhost)\n" +
                                     "  --open               Disable token requirement (use with caution)\n\n" +
                                     "EXAMPLES:\n" +
                                     "  gralph start .\n" +
                                     "  gralph start ~/project --name myapp --max-iterations 50\n" +
                                     "  gralph status\n" +
                                     "  gralph logs myapp --follow\n" +
                                     "  gralph stop myapp\n" +
                                     "  gralph prd create --dir . --output PRD.new.md --goal \"Add a billing dashboard\"\n" +
                                     "  gralph worktree create C-1\n" +
                                     "  gralph worktree finish C-1\n" +
                                     "  gralph server --host 0.0.0.0 --port 8080";

    public static int Main(string[] args)
    {
        args ??= Array.Empty<string>();

        if (args.Length == 0)
        {
            PrintUsage();
            return 0;
        }

        if (args.Any(IsHelpFlag))
        {
            PrintUsage();
            return 0;
        }

        if (args.Length == 1 && IsVersionFlag(args[0]))
        {
            PrintVersion();
            return 0;
        }

        var command = args[0];
        var commandArgs = args.Skip(1).ToArray();

        if (string.Equals(command, "help", StringComparison.OrdinalIgnoreCase))
        {
            PrintUsage();
            return 0;
        }

        if (string.Equals(command, "version", StringComparison.OrdinalIgnoreCase))
        {
            PrintVersion();
            return 0;
        }

        try
        {
            return command.ToLowerInvariant() switch
            {
                "start" => HandleStart(commandArgs),
                "stop" => HandleStop(commandArgs),
                "status" => HandleStatus(commandArgs),
                "logs" => HandleLogs(commandArgs),
                "resume" => HandleResume(commandArgs),
                "prd" => HandlePrd(commandArgs),
                "worktree" => HandleWorktree(commandArgs),
                "backends" => HandleBackends(commandArgs),
                "config" => HandleConfig(commandArgs),
                "server" => HandleServer(commandArgs),
                _ => HandleUnknownCommand(command)
            };
        }
        catch (ArgumentException ex)
        {
            return Fail(ex.Message);
        }
    }

    private static int HandleStart(string[] args)
    {
        var options = new StartOptions();
        var positional = new List<string>();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            switch (arg)
            {
                case "-n":
                case "--name":
                    options.Name = RequireValue(args, ref i, "--name");
                    options.NameSet = true;
                    break;
                case var _ when arg.StartsWith("--name=", StringComparison.Ordinal):
                    options.Name = arg["--name=".Length..];
                    options.NameSet = true;
                    break;
                case "--max-iterations":
                    options.MaxIterations = RequireInt(args, ref i, "--max-iterations");
                    options.MaxIterationsSet = true;
                    break;
                case var _ when arg.StartsWith("--max-iterations=", StringComparison.Ordinal):
                    options.MaxIterations = ParsePositiveInt(arg["--max-iterations=".Length..], "--max-iterations");
                    options.MaxIterationsSet = true;
                    break;
                case "-f":
                case "--task-file":
                    options.TaskFile = RequireValue(args, ref i, "--task-file");
                    options.TaskFileSet = true;
                    break;
                case var _ when arg.StartsWith("--task-file=", StringComparison.Ordinal):
                    options.TaskFile = arg["--task-file=".Length..];
                    options.TaskFileSet = true;
                    break;
                case "--completion-marker":
                    options.CompletionMarker = RequireValue(args, ref i, "--completion-marker");
                    options.CompletionMarkerSet = true;
                    break;
                case var _ when arg.StartsWith("--completion-marker=", StringComparison.Ordinal):
                    options.CompletionMarker = arg["--completion-marker=".Length..];
                    options.CompletionMarkerSet = true;
                    break;
                case "-b":
                case "--backend":
                    options.Backend = RequireValue(args, ref i, "--backend");
                    options.BackendSet = true;
                    break;
                case var _ when arg.StartsWith("--backend=", StringComparison.Ordinal):
                    options.Backend = arg["--backend=".Length..];
                    options.BackendSet = true;
                    break;
                case "-m":
                case "--model":
                    options.Model = RequireValue(args, ref i, "--model");
                    options.ModelSet = true;
                    break;
                case var _ when arg.StartsWith("--model=", StringComparison.Ordinal):
                    options.Model = arg["--model=".Length..];
                    options.ModelSet = true;
                    break;
                case "--variant":
                    options.Variant = RequireValue(args, ref i, "--variant");
                    options.VariantSet = true;
                    break;
                case var _ when arg.StartsWith("--variant=", StringComparison.Ordinal):
                    options.Variant = arg["--variant=".Length..];
                    options.VariantSet = true;
                    break;
                case "--prompt-template":
                    options.PromptTemplatePath = RequireValue(args, ref i, "--prompt-template");
                    options.PromptTemplateSet = true;
                    break;
                case var _ when arg.StartsWith("--prompt-template=", StringComparison.Ordinal):
                    options.PromptTemplatePath = arg["--prompt-template=".Length..];
                    options.PromptTemplateSet = true;
                    break;
                case "--webhook":
                    options.Webhook = RequireValue(args, ref i, "--webhook");
                    options.WebhookSet = true;
                    break;
                case var _ when arg.StartsWith("--webhook=", StringComparison.Ordinal):
                    options.Webhook = arg["--webhook=".Length..];
                    options.WebhookSet = true;
                    break;
                case "--no-tmux":
                    options.NoTmux = true;
                    options.NoTmuxSet = true;
                    break;
                case "--strict-prd":
                    options.StrictPrd = true;
                    break;
                default:
                    if (arg.StartsWith("-", StringComparison.Ordinal))
                    {
                        return Fail($"Unknown option: {arg}");
                    }

                    positional.Add(arg);
                    break;
            }
        }

        if (positional.Count == 0)
        {
            return Fail("Directory is required. Usage: gralph start <directory>");
        }

        var targetDir = positional[0];
        if (!Directory.Exists(targetDir))
        {
            return Fail($"Directory does not exist: {targetDir}");
        }

        targetDir = Path.GetFullPath(targetDir);

        var config = new ConfigService(ConfigPaths.FromEnvironment());
        config.Load(targetDir);

        options.MaxIterations = options.MaxIterationsSet
            ? options.MaxIterations
            : ParsePositiveInt(config.Get("defaults.max_iterations", "30"), "defaults.max_iterations");
        options.TaskFile = options.TaskFileSet ? options.TaskFile : config.Get("defaults.task_file", "PRD.md");
        options.CompletionMarker = options.CompletionMarkerSet
            ? options.CompletionMarker
            : config.Get("defaults.completion_marker", "COMPLETE");
        options.Backend = options.BackendSet ? options.Backend : config.Get("defaults.backend", "claude");
        options.Model = options.ModelSet ? options.Model : config.Get("defaults.model", string.Empty);

        if (string.Equals(options.Backend, "opencode", StringComparison.OrdinalIgnoreCase)
            && string.IsNullOrWhiteSpace(options.Model))
        {
            options.Model = config.Get("opencode.default_model", string.Empty);
        }

        if (string.IsNullOrWhiteSpace(options.CompletionMarker))
        {
            return Fail("Configured completion marker cannot be empty");
        }

        var taskFilePath = Path.Combine(targetDir, options.TaskFile);
        if (!File.Exists(taskFilePath))
        {
            return Fail($"Task file does not exist: {taskFilePath}");
        }

        if (!string.IsNullOrWhiteSpace(options.PromptTemplatePath))
        {
            var templatePath = Path.GetFullPath(options.PromptTemplatePath);
            if (!File.Exists(templatePath))
            {
                return Fail($"Prompt template file does not exist: {templatePath}");
            }
        }

        if (options.StrictPrd)
        {
            var validation = PrdValidator.ValidateFile(taskFilePath, allowMissingContext: false, baseDirOverride: targetDir);
            if (!validation.IsValid)
            {
                foreach (var error in validation.Errors)
                {
                    Console.Error.WriteLine(error.Format());
                }

                return Fail($"PRD validation failed: {taskFilePath}");
            }
        }

        Console.WriteLine("start command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandleStop(string[] args)
    {
        var all = false;
        var positional = new List<string>();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            switch (arg)
            {
                case "--all":
                case "-a":
                    all = true;
                    break;
                default:
                    if (arg.StartsWith("-", StringComparison.Ordinal))
                    {
                        return Fail($"Unknown option: {arg}");
                    }

                    positional.Add(arg);
                    break;
            }
        }

        if (!all && positional.Count == 0)
        {
            return Fail("Session name is required. Usage: gralph stop <name> or gralph stop --all");
        }

        if (all && positional.Count > 0)
        {
            return Fail("Stop does not accept a session name when using --all.");
        }

        Console.WriteLine("stop command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandleStatus(string[] args)
    {
        if (args.Length > 0)
        {
            return Fail($"Unknown option: {args[0]}");
        }

        Console.WriteLine("status command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandleLogs(string[] args)
    {
        var follow = false;
        var positional = new List<string>();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            switch (arg)
            {
                case "--follow":
                    follow = true;
                    break;
                default:
                    if (arg.StartsWith("-", StringComparison.Ordinal))
                    {
                        return Fail($"Unknown option: {arg}");
                    }

                    positional.Add(arg);
                    break;
            }
        }

        if (positional.Count == 0)
        {
            return Fail("Session name is required. Usage: gralph logs <name> [--follow]");
        }

        Console.WriteLine(follow
            ? "logs command (--follow) is registered but not implemented in this build."
            : "logs command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandleResume(string[] args)
    {
        if (args.Length > 1)
        {
            return Fail("Resume accepts at most one session name.");
        }

        if (args.Length == 1 && args[0].StartsWith("-", StringComparison.Ordinal))
        {
            return Fail($"Unknown option: {args[0]}");
        }

        Console.WriteLine("resume command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandlePrd(string[] args)
    {
        if (args.Length == 0)
        {
            return Fail("PRD command requires a subcommand: check or create.");
        }

        var subcommand = args[0];
        var subArgs = args.Skip(1).ToArray();

        return subcommand.ToLowerInvariant() switch
        {
            "check" => HandlePrdCheck(subArgs),
            "create" => HandlePrdCreate(subArgs),
            _ => Fail($"Unknown prd subcommand: {subcommand}")
        };
    }

    private static int HandlePrdCheck(string[] args)
    {
        var allowMissingContext = false;
        var positional = new List<string>();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            switch (arg)
            {
                case "--allow-missing-context":
                    allowMissingContext = true;
                    break;
                default:
                    if (arg.StartsWith("-", StringComparison.Ordinal))
                    {
                        return Fail($"Unknown option: {arg}");
                    }
                    positional.Add(arg);
                    break;
            }
        }

        if (positional.Count == 0)
        {
            return Fail("PRD file path is required. Usage: gralph prd check <file>");
        }

        _ = allowMissingContext;
        Console.WriteLine("prd check command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandlePrdCreate(string[] args)
    {
        var options = new PrdCreateOptions();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            switch (arg)
            {
                case "--dir":
                    options.Directory = RequireValue(args, ref i, "--dir");
                    break;
                case var _ when arg.StartsWith("--dir=", StringComparison.Ordinal):
                    options.Directory = arg["--dir=".Length..];
                    break;
                case "--output":
                case "-o":
                    options.Output = RequireValue(args, ref i, "--output");
                    break;
                case var _ when arg.StartsWith("--output=", StringComparison.Ordinal):
                    options.Output = arg["--output=".Length..];
                    break;
                case "--goal":
                    options.Goal = RequireValue(args, ref i, "--goal");
                    break;
                case var _ when arg.StartsWith("--goal=", StringComparison.Ordinal):
                    options.Goal = arg["--goal=".Length..];
                    break;
                case "--constraints":
                    options.Constraints = RequireValue(args, ref i, "--constraints");
                    break;
                case var _ when arg.StartsWith("--constraints=", StringComparison.Ordinal):
                    options.Constraints = arg["--constraints=".Length..];
                    break;
                case "--context":
                    options.Context = RequireValue(args, ref i, "--context");
                    break;
                case var _ when arg.StartsWith("--context=", StringComparison.Ordinal):
                    options.Context = arg["--context=".Length..];
                    break;
                case "--sources":
                    options.Sources = RequireValue(args, ref i, "--sources");
                    break;
                case var _ when arg.StartsWith("--sources=", StringComparison.Ordinal):
                    options.Sources = arg["--sources=".Length..];
                    break;
                case "--backend":
                case "-b":
                    options.Backend = RequireValue(args, ref i, "--backend");
                    break;
                case var _ when arg.StartsWith("--backend=", StringComparison.Ordinal):
                    options.Backend = arg["--backend=".Length..];
                    break;
                case "--model":
                case "-m":
                    options.Model = RequireValue(args, ref i, "--model");
                    break;
                case var _ when arg.StartsWith("--model=", StringComparison.Ordinal):
                    options.Model = arg["--model=".Length..];
                    break;
                case "--allow-missing-context":
                    options.AllowMissingContext = true;
                    break;
                case "--multiline":
                    options.Multiline = true;
                    break;
                case "--interactive":
                    options.Interactive = true;
                    break;
                case "--no-interactive":
                case "--non-interactive":
                    options.Interactive = false;
                    break;
                case "--force":
                    options.Force = true;
                    break;
                default:
                    if (arg.StartsWith("-", StringComparison.Ordinal))
                    {
                        return Fail($"Unknown option: {arg}");
                    }

                    return Fail($"Unexpected argument: {arg}");
            }
        }

        Console.WriteLine("prd create command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandleWorktree(string[] args)
    {
        if (args.Length == 0)
        {
            return Fail("Worktree command requires a subcommand: create or finish.");
        }

        var subcommand = args[0];
        var subArgs = args.Skip(1).ToArray();

        switch (subcommand.ToLowerInvariant())
        {
            case "create":
            case "finish":
                if (subArgs.Length == 0)
                {
                    return Fail($"Worktree {subcommand} requires an ID. Usage: gralph worktree {subcommand} <ID>");
                }

                if (subArgs.Length > 1)
                {
                    return Fail($"Worktree {subcommand} accepts a single ID.");
                }

                if (subArgs[0].StartsWith("-", StringComparison.Ordinal))
                {
                    return Fail($"Unknown option: {subArgs[0]}");
                }

                Console.WriteLine($"worktree {subcommand} command is registered but not implemented in this build.");
                return 0;
            default:
                return Fail($"Unknown worktree subcommand: {subcommand}");
        }
    }

    private static int HandleBackends(string[] args)
    {
        if (args.Length > 0)
        {
            return Fail($"Unknown option: {args[0]}");
        }

        Console.WriteLine("Available AI backends:\n");
        foreach (var backendName in BackendLoader.ListAvailable())
        {
            var backend = BackendLoader.Load(backendName);
            var installed = backend.IsInstalled();
            Console.WriteLine($"  {backend.Name} {(installed ? "(installed)" : "(not installed)")}");
            if (installed)
            {
                var models = backend.GetModels();
                if (models.Count > 0)
                {
                    Console.WriteLine($"      Models: {string.Join(", ", models)}");
                }
            }
            else
            {
                Console.WriteLine($"      Install: {backend.GetInstallHint()}");
            }

            Console.WriteLine();
        }

        Console.WriteLine("Usage: gralph start <dir> --backend <name>");
        return 0;
    }

    private static int HandleConfig(string[] args)
    {
        if (args.Length == 0)
        {
            Console.WriteLine("config command is registered but not implemented in this build.");
            return 0;
        }

        var subcommand = args[0];
        var subArgs = args.Skip(1).ToArray();

        switch (subcommand.ToLowerInvariant())
        {
            case "list":
                if (subArgs.Length > 0)
                {
                    return Fail("config list does not take additional arguments.");
                }
                Console.WriteLine("config list command is registered but not implemented in this build.");
                return 0;
            case "get":
                if (subArgs.Length == 0)
                {
                    return Fail("config get requires a key.");
                }
                if (subArgs.Length > 1)
                {
                    return Fail("config get accepts a single key.");
                }
                Console.WriteLine("config get command is registered but not implemented in this build.");
                return 0;
            case "set":
                if (subArgs.Length < 2)
                {
                    return Fail("config set requires a key and value.");
                }
                Console.WriteLine("config set command is registered but not implemented in this build.");
                return 0;
            default:
                return Fail($"Unknown config subcommand: {subcommand}");
        }
    }

    private static int HandleServer(string[] args)
    {
        var options = new ServerOptions();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            switch (arg)
            {
                case "--host":
                case "-H":
                    options.Host = RequireValue(args, ref i, "--host");
                    break;
                case var _ when arg.StartsWith("--host=", StringComparison.Ordinal):
                    options.Host = arg["--host=".Length..];
                    break;
                case "--port":
                case "-p":
                    options.Port = RequireInt(args, ref i, "--port");
                    break;
                case var _ when arg.StartsWith("--port=", StringComparison.Ordinal):
                    options.Port = ParsePositiveInt(arg["--port=".Length..], "--port");
                    break;
                case "--token":
                case "-t":
                    options.Token = RequireValue(args, ref i, "--token");
                    break;
                case var _ when arg.StartsWith("--token=", StringComparison.Ordinal):
                    options.Token = arg["--token=".Length..];
                    break;
                case "--open":
                    options.Open = true;
                    break;
                default:
                    if (arg.StartsWith("-", StringComparison.Ordinal))
                    {
                        return Fail($"Unknown option: {arg}");
                    }

                    return Fail($"Unexpected argument: {arg}");
            }
        }

        _ = options;
        Console.WriteLine("server command is registered but not implemented in this build.");
        return 0;
    }

    private static int HandleUnknownCommand(string command)
    {
        var suggestion = SuggestCommand(command);
        if (!string.IsNullOrWhiteSpace(suggestion))
        {
            Console.Error.WriteLine($"Unknown command: {command}. Did you mean '{suggestion}'?");
        }
        else
        {
            Console.Error.WriteLine($"Unknown command: {command}.");
        }

        Console.Error.WriteLine("Run 'gralph help' to see available commands.");
        return 1;
    }

    private static string? SuggestCommand(string command)
    {
        if (string.IsNullOrWhiteSpace(command))
        {
            return null;
        }

        var best = string.Empty;
        var bestDistance = int.MaxValue;
        foreach (var candidate in CommandNames)
        {
            var distance = LevenshteinDistance(command, candidate);
            if (distance < bestDistance)
            {
                bestDistance = distance;
                best = candidate;
            }
        }

        return bestDistance <= 3 ? best : null;
    }

    private static int LevenshteinDistance(string source, string target)
    {
        var a = source ?? string.Empty;
        var b = target ?? string.Empty;
        var costs = new int[b.Length + 1];

        for (var j = 0; j <= b.Length; j++)
        {
            costs[j] = j;
        }

        for (var i = 1; i <= a.Length; i++)
        {
            costs[0] = i;
            var previousCost = i - 1;
            for (var j = 1; j <= b.Length; j++)
            {
                var currentCost = costs[j];
                var cost = a[i - 1] == b[j - 1] ? 0 : 1;
                costs[j] = Math.Min(
                    Math.Min(costs[j] + 1, costs[j - 1] + 1),
                    previousCost + cost);
                previousCost = currentCost;
            }
        }

        return costs[b.Length];
    }

    private static string RequireValue(string[] args, ref int index, string optionName)
    {
        if (index + 1 >= args.Length || args[index + 1].StartsWith("-", StringComparison.Ordinal))
        {
            throw new ArgumentException($"Option {optionName} requires a value.");
        }

        index++;
        return args[index];
    }

    private static int RequireInt(string[] args, ref int index, string optionName)
    {
        var value = RequireValue(args, ref index, optionName);
        return ParsePositiveInt(value, optionName);
    }

    private static int ParsePositiveInt(string value, string optionName)
    {
        if (!int.TryParse(value, out var result) || result <= 0)
        {
            throw new ArgumentException($"Option {optionName} must be a positive integer.");
        }

        return result;
    }

    private static int Fail(string message)
    {
        Console.Error.WriteLine($"Error: {message}");
        return 1;
    }

    private static bool IsHelpFlag(string arg)
    {
        return string.Equals(arg, "-h", StringComparison.Ordinal)
            || string.Equals(arg, "--help", StringComparison.Ordinal);
    }

    private static bool IsVersionFlag(string arg)
    {
        return string.Equals(arg, "-v", StringComparison.Ordinal)
            || string.Equals(arg, "--version", StringComparison.Ordinal);
    }

    private static void PrintUsage()
    {
        Console.WriteLine(UsageText);
    }

    private static void PrintVersion()
    {
        Console.WriteLine($"gralph {Version}");
    }

    private sealed class StartOptions
    {
        public string Name { get; set; } = string.Empty;
        public bool NameSet { get; set; }
        public int MaxIterations { get; set; } = 30;
        public bool MaxIterationsSet { get; set; }
        public string TaskFile { get; set; } = "PRD.md";
        public bool TaskFileSet { get; set; }
        public string CompletionMarker { get; set; } = "COMPLETE";
        public bool CompletionMarkerSet { get; set; }
        public string Backend { get; set; } = "";
        public bool BackendSet { get; set; }
        public string Model { get; set; } = string.Empty;
        public bool ModelSet { get; set; }
        public string Variant { get; set; } = string.Empty;
        public bool VariantSet { get; set; }
        public string PromptTemplatePath { get; set; } = string.Empty;
        public bool PromptTemplateSet { get; set; }
        public string Webhook { get; set; } = string.Empty;
        public bool WebhookSet { get; set; }
        public bool NoTmux { get; set; }
        public bool NoTmuxSet { get; set; }
        public bool StrictPrd { get; set; }
    }

    private sealed class PrdCreateOptions
    {
        public string Directory { get; set; } = System.IO.Directory.GetCurrentDirectory();
        public string Output { get; set; } = "PRD.generated.md";
        public string Goal { get; set; } = string.Empty;
        public string Constraints { get; set; } = string.Empty;
        public string Context { get; set; } = string.Empty;
        public string Sources { get; set; } = string.Empty;
        public string Backend { get; set; } = string.Empty;
        public string Model { get; set; } = string.Empty;
        public bool AllowMissingContext { get; set; }
        public bool Multiline { get; set; }
        public bool? Interactive { get; set; }
        public bool Force { get; set; }
    }

    private sealed class ServerOptions
    {
        public string Host { get; set; } = "127.0.0.1";
        public int Port { get; set; } = 8080;
        public string Token { get; set; } = string.Empty;
        public bool Open { get; set; }
    }
}
