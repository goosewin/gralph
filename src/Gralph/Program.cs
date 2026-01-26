using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using System.Threading;
using Gralph.Backends;
using Gralph.Config;
using Gralph.Core;
using Gralph.Notify;
using Gralph.Prd;
using Gralph.Server;
using Gralph.State;

namespace Gralph;

public static class Program
{
    private const string Version = "1.1.0";
    private const string StartWorkerEnv = "GRALPH_START_WORKER";

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

        if (IsStartWorker())
        {
            options.NoTmux = true;
            options.NoTmuxSet = true;
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

        var sessionName = options.NameSet && !string.IsNullOrWhiteSpace(options.Name)
            ? options.Name
            : Path.GetFileName(targetDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        sessionName = SanitizeSessionName(sessionName);

        if (string.IsNullOrWhiteSpace(sessionName))
        {
            sessionName = "gralph";
        }

        var backendName = options.Backend;
        IBackend backend;
        try
        {
            backend = BackendLoader.Load(backendName);
        }
        catch (Exception ex) when (ex is ArgumentException or KeyNotFoundException)
        {
            return Fail(ex.Message);
        }

        if (!backend.IsInstalled())
        {
            return Fail($"Backend '{backend.Name}' is not installed. {backend.GetInstallHint()}");
        }

        var state = new StateStore(StatePaths.FromEnvironment());
        state.Init();

        if (!IsStartWorker())
        {
            var existing = state.GetSession(sessionName);
            if (existing != null)
            {
                var status = GetSessionString(existing, "status") ?? "unknown";
                if (string.Equals(status, "running", StringComparison.OrdinalIgnoreCase))
                {
                    if (TryGetSessionInt(existing, "pid", out var pid) && new ProcessInspector().IsAlive(pid))
                    {
                        return Fail($"Session '{sessionName}' is already running (PID: {pid}). Use 'gralph stop {sessionName}' first.");
                    }

                    Console.Error.WriteLine($"Warning: Session '{sessionName}' exists but appears stale. Restarting...");
                }
            }
        }

        var gralphDir = Path.Combine(targetDir, ".gralph");
        Directory.CreateDirectory(gralphDir);
        var logFile = Path.Combine(gralphDir, $"{sessionName}.log");
        var initialRemaining = CountRemainingTasks(taskFilePath);

        var sessionData = new Dictionary<string, object?>
        {
            ["dir"] = targetDir,
            ["task_file"] = options.TaskFile,
            ["pid"] = Environment.ProcessId,
            ["tmux_session"] = string.Empty,
            ["started_at"] = DateTimeOffset.Now.ToString("O"),
            ["iteration"] = 1,
            ["max_iterations"] = options.MaxIterations,
            ["status"] = "running",
            ["last_task_count"] = initialRemaining,
            ["completion_marker"] = options.CompletionMarker,
            ["log_file"] = logFile,
            ["backend"] = backend.Name,
            ["model"] = options.Model ?? string.Empty,
            ["variant"] = options.Variant ?? string.Empty,
            ["webhook"] = options.Webhook ?? string.Empty
        };

        if (!options.NoTmux && !IsStartWorker())
        {
            var process = StartBackgroundProcess(targetDir, sessionName, options);
            sessionData["pid"] = process.Id;
            state.SetSession(sessionName, sessionData);
            Console.WriteLine($"Started gralph session '{sessionName}' in background (PID: {process.Id}).");
            Console.WriteLine($"Logs: {logFile}");
            return 0;
        }

        state.SetSession(sessionName, sessionData);

        var loopOptions = new CoreLoopOptions(targetDir)
        {
            TaskFile = options.TaskFile,
            MaxIterations = options.MaxIterations,
            CompletionMarker = options.CompletionMarker,
            ModelOverride = options.Model,
            SessionName = sessionName,
            PromptTemplatePath = string.IsNullOrWhiteSpace(options.PromptTemplatePath)
                ? null
                : Path.GetFullPath(options.PromptTemplatePath),
            LogFilePath = logFile
        };

        var loop = new CoreLoop(config, backend);
        var result = loop.RunAsync(loopOptions, update =>
        {
            state.SetSession(sessionName, new Dictionary<string, object?>
            {
                ["iteration"] = update.Iteration,
                ["status"] = update.Status,
                ["last_task_count"] = update.RemainingTasks
            });
        }).GetAwaiter().GetResult();

        var finalStatus = result.Status switch
        {
            CoreLoopStatus.Complete => "complete",
            CoreLoopStatus.MaxIterations => "max_iterations",
            _ => "failed"
        };

        state.SetSession(sessionName, new Dictionary<string, object?>
        {
            ["iteration"] = result.Iterations,
            ["status"] = finalStatus,
            ["last_task_count"] = result.RemainingTasks
        });

        if (result.Status == CoreLoopStatus.Failed && !string.IsNullOrWhiteSpace(result.ErrorMessage))
        {
            Console.Error.WriteLine($"Error: {result.ErrorMessage}");
        }

        if (result.Status == CoreLoopStatus.Complete)
        {
            TryNotifyCompletion(config, options.Webhook, sessionName, targetDir, result);
        }
        else
        {
            var reason = result.Status == CoreLoopStatus.MaxIterations ? "max_iterations" : "error";
            TryNotifyFailure(
                config,
                options.Webhook,
                sessionWebhook: null,
                sessionName,
                targetDir,
                reason,
                result.Iterations,
                options.MaxIterations,
                result.RemainingTasks,
                result.Duration);
        }

        return result.Status == CoreLoopStatus.Complete ? 0 : 1;
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

        var state = new StateStore(StatePaths.FromEnvironment());
        state.Init();
        var inspector = new ProcessInspector();

        if (all)
        {
            var sessions = state.ListSessions();
            if (sessions.Count == 0)
            {
                Console.WriteLine("No sessions found.");
                return 0;
            }

            var stoppedCount = 0;
            foreach (var session in sessions)
            {
                var name = GetSessionString(session, "name") ?? string.Empty;
                if (string.IsNullOrWhiteSpace(name))
                {
                    continue;
                }

                var status = GetSessionString(session, "status") ?? "unknown";
                if (!string.Equals(status, "running", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                StopSession(session, name, inspector, state, quietWarnings: true);
                TryNotifyManualStop(session, name);
                stoppedCount++;
            }

            if (stoppedCount == 0)
            {
                Console.WriteLine("No running sessions to stop.");
            }
            else
            {
                Console.WriteLine($"Stopped {stoppedCount} session(s).");
            }

            return 0;
        }

        var sessionName = positional[0];
        var existing = state.GetSession(sessionName);
        if (existing is null)
        {
            return Fail($"Session not found: {sessionName}");
        }

        var existingStatus = GetSessionString(existing, "status") ?? "unknown";
        if (!string.Equals(existingStatus, "running", StringComparison.OrdinalIgnoreCase))
        {
            Console.Error.WriteLine($"Warning: Session '{sessionName}' is not running (status: {existingStatus}).");
        }

        StopSession(existing, sessionName, inspector, state, quietWarnings: false);
        TryNotifyManualStop(existing, sessionName);
        Console.WriteLine($"Stopped session: {sessionName}");
        return 0;
    }

    private static int HandleStatus(string[] args)
    {
        if (args.Length > 0)
        {
            return Fail($"Unknown option: {args[0]}");
        }

        var state = new StateStore(StatePaths.FromEnvironment());
        state.Init();
        _ = state.CleanupStale();

        var sessions = state.ListSessions();
        if (sessions.Count == 0)
        {
            Console.WriteLine("No sessions found.");
            Console.WriteLine();
            Console.WriteLine("Start a new loop with: gralph start <directory>");
            return 0;
        }

        var rows = new List<StatusRow>();
        foreach (var session in sessions.OrderBy(session => GetSessionString(session, "name")))
        {
            var name = GetSessionString(session, "name") ?? "unknown";
            var dir = GetSessionString(session, "dir") ?? string.Empty;
            var taskFile = GetSessionString(session, "task_file") ?? "PRD.md";
            var status = GetSessionString(session, "status") ?? "unknown";
            _ = TryGetSessionInt(session, "iteration", out var iteration);
            _ = TryGetSessionInt(session, "max_iterations", out var maxIterations);

            var displayDir = TruncateDirectory(dir, 40);
            var iterDisplay = maxIterations > 0 ? $"{iteration}/{maxIterations}" : iteration.ToString();
            var remaining = ResolveRemainingTasks(dir, taskFile, session);
            var remainingDisplay = FormatRemaining(remaining);

            rows.Add(new StatusRow(name, displayDir, iterDisplay, status, remainingDisplay));
        }

        PrintStatusTable(rows);
        Console.WriteLine();
        Console.WriteLine("Commands: gralph logs <name>, gralph stop <name>, gralph resume");
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

        var sessionName = positional[0];
        var state = new StateStore(StatePaths.FromEnvironment());
        state.Init();

        var session = state.GetSession(sessionName);
        if (session is null)
        {
            return Fail($"Session not found: {sessionName}");
        }

        var logFile = GetSessionString(session, "log_file") ?? string.Empty;
        if (string.IsNullOrWhiteSpace(logFile))
        {
            var sessionDir = GetSessionString(session, "dir") ?? string.Empty;
            if (!string.IsNullOrWhiteSpace(sessionDir))
            {
                logFile = Path.Combine(sessionDir, ".gralph", $"{sessionName}.log");
            }
        }

        if (string.IsNullOrWhiteSpace(logFile))
        {
            return Fail($"Cannot determine log file path for session: {sessionName}");
        }

        if (!File.Exists(logFile))
        {
            return Fail($"Log file does not exist: {logFile}");
        }

        var status = GetSessionString(session, "status") ?? "unknown";
        Console.WriteLine($"Session: {sessionName} (status: {status})");
        Console.WriteLine($"Log file: {logFile}");
        Console.WriteLine();

        var lines = ReadLastLines(logFile, follow ? 10 : 100);
        foreach (var line in lines)
        {
            Console.WriteLine(line);
        }

        if (follow)
        {
            FollowFile(logFile);
        }

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

        var targetSession = args.Length == 1 ? args[0] : string.Empty;
        var state = new StateStore(StatePaths.FromEnvironment());
        state.Init();

        var sessions = state.ListSessions();
        if (sessions.Count == 0)
        {
            Console.WriteLine("No sessions found.");
            return 0;
        }

        var inspector = new ProcessInspector();
        var resumedCount = 0;

        IEnumerable<JsonObject> targets;
        if (!string.IsNullOrWhiteSpace(targetSession))
        {
            var session = state.GetSession(targetSession);
            if (session is null)
            {
                return Fail($"Session not found: {targetSession}");
            }
            targets = [session];
        }
        else
        {
            targets = sessions;
        }

        foreach (var session in targets)
        {
            var sessionName = GetSessionString(session, "name") ?? targetSession;
            if (string.IsNullOrWhiteSpace(sessionName))
            {
                continue;
            }

            var status = GetSessionString(session, "status") ?? "unknown";
            var shouldResume = false;
            if (string.Equals(status, "running", StringComparison.OrdinalIgnoreCase))
            {
                if (!TryGetSessionInt(session, "pid", out var pid) || !inspector.IsAlive(pid))
                {
                    shouldResume = true;
                }
            }
            else if (string.Equals(status, "stale", StringComparison.OrdinalIgnoreCase)
                     || string.Equals(status, "stopped", StringComparison.OrdinalIgnoreCase))
            {
                shouldResume = true;
            }

            if (!shouldResume)
            {
                if (!string.IsNullOrWhiteSpace(targetSession))
                {
                    Console.Error.WriteLine($"Warning: Session '{sessionName}' is already running or completed (status: {status}).");
                }
                continue;
            }

            var dir = GetSessionString(session, "dir") ?? string.Empty;
            if (string.IsNullOrWhiteSpace(dir) || !Directory.Exists(dir))
            {
                Console.Error.WriteLine($"Warning: Skipping '{sessionName}': directory no longer exists: {dir}");
                continue;
            }

            var taskFile = GetSessionString(session, "task_file") ?? "PRD.md";
            var taskPath = Path.Combine(dir, taskFile);
            if (!File.Exists(taskPath))
            {
                Console.Error.WriteLine($"Warning: Skipping '{sessionName}': task file not found: {taskPath}");
                continue;
            }

            var backendName = GetSessionString(session, "backend") ?? "claude";
            IBackend backend;
            try
            {
                backend = BackendLoader.Load(backendName);
            }
            catch (Exception ex) when (ex is ArgumentException or KeyNotFoundException)
            {
                if (!string.IsNullOrWhiteSpace(targetSession))
                {
                    return Fail(ex.Message);
                }
                Console.Error.WriteLine($"Warning: Skipping '{sessionName}': {ex.Message}");
                continue;
            }

            if (!backend.IsInstalled())
            {
                if (!string.IsNullOrWhiteSpace(targetSession))
                {
                    return Fail($"Backend '{backend.Name}' is not installed. {backend.GetInstallHint()}");
                }

                Console.Error.WriteLine($"Warning: Skipping '{sessionName}': backend '{backend.Name}' not installed.");
                continue;
            }

            var options = BuildResumeOptions(session, backend.Name);
            var process = StartBackgroundProcess(dir, sessionName, options);
            var remaining = CountRemainingTasks(taskPath);

            state.SetSession(sessionName, new Dictionary<string, object?>
            {
                ["pid"] = process.Id,
                ["tmux_session"] = string.Empty,
                ["status"] = "running",
                ["last_task_count"] = remaining
            });

            resumedCount++;
            Console.WriteLine($"Resumed session '{sessionName}' in background (PID: {process.Id}).");
        }

        if (resumedCount == 0)
        {
            Console.WriteLine("No sessions to resume.");
            return 0;
        }

        Console.WriteLine();
        Console.WriteLine($"Resumed {resumedCount} session(s).");
        Console.WriteLine();
        Console.WriteLine("Commands: gralph status, gralph logs <name>, gralph stop <name>");
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

        if (positional.Count > 1)
        {
            return Fail("prd check accepts a single file path.");
        }

        var taskFile = positional[0];
        var validation = PrdValidator.ValidateFile(taskFile, allowMissingContext);
        if (!validation.IsValid)
        {
            foreach (var error in validation.Errors)
            {
                Console.Error.WriteLine(error.Format());
            }

            return Fail($"PRD validation failed: {taskFile}");
        }

        Console.WriteLine($"PRD validation passed: {taskFile}");
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

        var targetDir = string.IsNullOrWhiteSpace(options.Directory)
            ? Directory.GetCurrentDirectory()
            : options.Directory;

        if (!Directory.Exists(targetDir))
        {
            return Fail($"Directory does not exist: {targetDir}");
        }

        targetDir = Path.GetFullPath(targetDir);

        var outputPath = string.IsNullOrWhiteSpace(options.Output)
            ? "PRD.generated.md"
            : options.Output;

        if (!Path.IsPathRooted(outputPath))
        {
            outputPath = Path.Combine(targetDir, outputPath);
        }

        if (File.Exists(outputPath) && !options.Force)
        {
            return Fail($"Output file already exists: {outputPath}. Use --force to overwrite.");
        }

        var config = new ConfigService(ConfigPaths.FromEnvironment());
        config.Load(targetDir);

        var backendName = string.IsNullOrWhiteSpace(options.Backend)
            ? config.Get("defaults.backend", "claude")
            : options.Backend;
        var model = string.IsNullOrWhiteSpace(options.Model)
            ? config.Get("defaults.model", string.Empty)
            : options.Model;

        if (string.IsNullOrWhiteSpace(model))
        {
            model = config.Get($"{backendName}.default_model", string.Empty);
        }

        var interactive = options.Interactive ?? !Console.IsInputRedirected;

        if (interactive)
        {
            options.Goal = PromptRequired("Goal", options.Goal, options.Multiline);
            options.Constraints = PromptOptional("Constraints", options.Constraints, options.Multiline);
            options.Sources = PromptOptional("Sources (comma-separated URLs)", options.Sources, options.Multiline);
        }

        if (string.IsNullOrWhiteSpace(options.Goal))
        {
            return Fail("PRD goal is required. Provide --goal or run interactively.");
        }

        var contextFiles = BuildContextFileList(targetDir, options.Context, config.Get("defaults.context_files", string.Empty));
        var sourcesList = NormalizeCsvList(options.Sources);
        var constraintsText = string.IsNullOrWhiteSpace(options.Constraints) ? "None." : options.Constraints.Trim();
        var sourcesSection = sourcesList.Count > 0 ? string.Join("\n", sourcesList) : "None.";
        var warningsSection = sourcesList.Count == 0
            ? "No reliable external sources were provided. Verify requirements and stack assumptions before implementation."
            : string.Empty;
        var contextSection = contextFiles.Count > 0 ? string.Join("\n", contextFiles) : "None.";

        var stackSummary = PrdStackDetector.Detect(targetDir);
        var stackSummaryPrompt = stackSummary.FormatSummary(2);

        var templateText = PrdTemplate.GetTemplateText(targetDir);

        var promptBuilder = new StringBuilder();
        promptBuilder.AppendLine("You are generating a gralph PRD in markdown. The output must be spec-compliant and grounded in the repository.");
        promptBuilder.AppendLine();
        promptBuilder.AppendLine($"Project directory: {targetDir}");
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Goal:");
        promptBuilder.AppendLine(options.Goal.Trim());
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Constraints:");
        promptBuilder.AppendLine(constraintsText);
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Detected stack summary (from repository files):");
        promptBuilder.AppendLine(stackSummaryPrompt);
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Sources (authoritative URLs or references):");
        promptBuilder.AppendLine(sourcesSection);
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Warnings (only include in the PRD if Sources is empty):");
        promptBuilder.AppendLine(string.IsNullOrWhiteSpace(warningsSection) ? "None." : warningsSection);
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Context files (read these first if present):");
        promptBuilder.AppendLine(contextSection);
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Requirements:");
        promptBuilder.AppendLine("- Output only the PRD markdown with no commentary or code fences.");
        promptBuilder.AppendLine("- Use ASCII only.");
        promptBuilder.AppendLine("- Do not include an \"Open Questions\" section.");
        promptBuilder.AppendLine("- Do not use any checkboxes outside task blocks.");
        promptBuilder.AppendLine("- Context Bundle entries must be real files in the repo and must be selected from the Context files list above.");
        promptBuilder.AppendLine("- If a task creates new files, do not list the new files in Context Bundle; cite the closest existing files instead.");
        promptBuilder.AppendLine("- Use atomic, granular tasks grounded in the repo and context files.");
        promptBuilder.AppendLine("- Each task block must use a '### Task <ID>' header and include **ID**, **Context Bundle**, **DoD**, **Checklist**, **Dependencies**.");
        promptBuilder.AppendLine("- Each task block must contain exactly one unchecked task line like '- [ ] <ID> <summary>'.");
        promptBuilder.AppendLine("- If Sources is empty, include a 'Warnings' section with the warning text above and no checkboxes.");
        promptBuilder.AppendLine("- Do not invent stack, frameworks, or files not supported by the context files and stack summary.");
        promptBuilder.AppendLine();
        promptBuilder.AppendLine("Template:");
        promptBuilder.AppendLine(templateText);

        var prompt = promptBuilder.ToString();

        IBackend backend;
        try
        {
            backend = BackendLoader.Load(backendName);
        }
        catch (Exception ex) when (ex is ArgumentException or KeyNotFoundException)
        {
            return Fail(ex.Message);
        }

        if (!backend.IsInstalled())
        {
            return Fail($"Backend '{backend.Name}' is not installed. {backend.GetInstallHint()}");
        }

        var tmpOutput = Path.GetTempFileName();
        var tmpPrd = Path.GetTempFileName();
        var rawOutput = Path.GetTempFileName();

        BackendRunResult runResult;
        try
        {
            runResult = backend.RunIterationAsync(new BackendRunRequest(prompt, model, tmpOutput, rawOutput), CancellationToken.None)
                .GetAwaiter()
                .GetResult();
        }
        catch (Exception ex)
        {
            return Fail($"PRD generation failed: {ex.Message}");
        }

        if (runResult.ExitCode != 0)
        {
            Console.Error.WriteLine($"Warning: PRD generation failed (backend exit code {runResult.ExitCode}).");
            Console.Error.WriteLine($"Raw backend output saved to: {rawOutput}");
            return 1;
        }

        if (string.IsNullOrWhiteSpace(runResult.ParsedText))
        {
            Console.Error.WriteLine("Warning: PRD generation returned empty output.");
            Console.Error.WriteLine($"Raw backend output saved to: {rawOutput}");
            return 1;
        }

        File.WriteAllText(tmpPrd, runResult.ParsedText.TrimEnd() + "\n");

        PrdSanitizer.SanitizeFile(tmpPrd, targetDir, contextFiles);

        var validation = PrdValidator.ValidateFile(tmpPrd, options.AllowMissingContext, targetDir);
        if (!validation.IsValid)
        {
            Console.Error.WriteLine("Warning: Generated PRD failed validation.");
            foreach (var error in validation.Errors)
            {
                Console.Error.WriteLine(error.Format());
            }

            var invalidPath = outputPath;
            if (!options.Force)
            {
                invalidPath = outputPath.EndsWith(".md", StringComparison.OrdinalIgnoreCase)
                    ? outputPath[..^3] + ".invalid.md"
                    : outputPath + ".invalid";
            }

            File.Move(tmpPrd, invalidPath, true);
            Console.Error.WriteLine($"Saved invalid PRD to: {invalidPath}");
            return 1;
        }

        File.Move(tmpPrd, outputPath, true);
        TryDeleteFile(tmpOutput);
        TryDeleteFile(rawOutput);

        Console.WriteLine($"PRD created: {outputPath}");
        var relativeOutput = outputPath.StartsWith(targetDir, StringComparison.Ordinal)
            ? Path.GetRelativePath(targetDir, outputPath)
            : outputPath;
        Console.WriteLine("Next step:");
        var startCommand = new StringBuilder($"gralph start {targetDir} --task-file {relativeOutput} --no-tmux --backend {backendName}");
        if (!string.IsNullOrWhiteSpace(model))
        {
            startCommand.Append($" --model {model}");
        }
        startCommand.Append(" --strict-prd");
        Console.WriteLine($"  {startCommand}");

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

                var taskId = subArgs[0];
                if (!IsValidTaskId(taskId))
                {
                    return Fail($"Invalid task ID format: {taskId} (expected like A-1)");
                }

                var repoRoot = GetGitRoot();
                if (string.IsNullOrWhiteSpace(repoRoot))
                {
                    return Fail("Not a git repository (or any of the parent directories)");
                }

                var statusResult = RunProcess("git", ["-C", repoRoot, "status", "--porcelain"], repoRoot);
                if (statusResult.ExitCode != 0)
                {
                    return Fail($"Unable to check git status in {repoRoot}");
                }

                if (!string.IsNullOrWhiteSpace(statusResult.StdOut))
                {
                    return Fail("Git working tree is dirty. Commit or stash changes before running worktree commands.");
                }

                return subcommand.Equals("create", StringComparison.OrdinalIgnoreCase)
                    ? HandleWorktreeCreate(repoRoot, taskId)
                    : HandleWorktreeFinish(repoRoot, taskId);
            default:
                return Fail($"Unknown worktree subcommand: {subcommand}");
        }
    }

    private static int HandleWorktreeCreate(string repoRoot, string taskId)
    {
        var branchName = $"task-{taskId}";
        var worktreesDir = Path.Combine(repoRoot, ".worktrees");
        Directory.CreateDirectory(worktreesDir);
        var worktreePath = Path.Combine(worktreesDir, branchName);

        var branchResult = RunProcess("git", ["-C", repoRoot, "show-ref", "--verify", "--quiet", $"refs/heads/{branchName}"], repoRoot);
        if (branchResult.ExitCode == 0)
        {
            return Fail($"Branch already exists: {branchName}");
        }

        if (Directory.Exists(worktreePath) || File.Exists(worktreePath))
        {
            return Fail($"Worktree path already exists: {worktreePath}");
        }

        var addResult = RunProcess("git", ["-C", repoRoot, "worktree", "add", "-b", branchName, worktreePath], repoRoot);
        if (addResult.ExitCode != 0)
        {
            return Fail($"Failed to create worktree at {worktreePath}");
        }

        Console.WriteLine($"Created worktree {worktreePath} on branch {branchName}");
        return 0;
    }

    private static int HandleWorktreeFinish(string repoRoot, string taskId)
    {
        var branchName = $"task-{taskId}";
        var worktreesDir = Path.Combine(repoRoot, ".worktrees");
        var worktreePath = Path.Combine(worktreesDir, branchName);

        var branchResult = RunProcess("git", ["-C", repoRoot, "show-ref", "--verify", "--quiet", $"refs/heads/{branchName}"], repoRoot);
        if (branchResult.ExitCode != 0)
        {
            return Fail($"Branch does not exist: {branchName}");
        }

        if (!Directory.Exists(worktreePath))
        {
            return Fail($"Worktree path is missing: {worktreePath} (run 'gralph worktree create {taskId}' first)");
        }

        var currentBranchResult = RunProcess("git", ["-C", repoRoot, "rev-parse", "--abbrev-ref", "HEAD"], repoRoot);
        if (currentBranchResult.ExitCode == 0)
        {
            var currentBranch = currentBranchResult.StdOut.Trim();
            if (string.Equals(currentBranch, branchName, StringComparison.Ordinal))
            {
                return Fail($"Cannot finish while on branch {branchName}");
            }
        }

        var mergeResult = RunProcess("git", ["-C", repoRoot, "merge", "--no-ff", branchName], repoRoot);
        if (mergeResult.ExitCode != 0)
        {
            return Fail($"Failed to merge branch: {branchName}");
        }

        var removeResult = RunProcess("git", ["-C", repoRoot, "worktree", "remove", worktreePath], repoRoot);
        if (removeResult.ExitCode != 0)
        {
            return Fail($"Failed to remove worktree at {worktreePath}");
        }

        Console.WriteLine($"Finished worktree {worktreePath} and merged {branchName}");
        return 0;
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
        var config = new ConfigService(ConfigPaths.FromEnvironment());
        config.Load(Directory.GetCurrentDirectory());

        if (args.Length == 0)
        {
            PrintConfigList(config);
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
                PrintConfigList(config);
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

                var key = subArgs[0];
                if (!config.Exists(key))
                {
                    return Fail($"Config key not found: {key}");
                }

                Console.WriteLine(config.Get(key));
                return 0;
            case "set":
                if (subArgs.Length < 2)
                {
                    return Fail("config set requires a key and value.");
                }
                if (subArgs.Length > 2)
                {
                    return Fail("config set accepts a single key and value.");
                }

                var setKey = subArgs[0];
                var setValue = subArgs[1];
                config.Set(setKey, setValue);
                Console.WriteLine($"Updated config: {setKey}");
                return 0;
            default:
                return Fail($"Unknown config subcommand: {subcommand}");
        }
    }

    private static void PrintConfigList(ConfigService config)
    {
        var entries = config.ListMerged();
        foreach (var entry in entries)
        {
            Console.WriteLine($"{entry.Key}={entry.Value}");
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

        if (options.Port > 65535)
        {
            return Fail($"Port must be between 1 and 65535 (got {options.Port}).");
        }

        var host = string.IsNullOrWhiteSpace(options.Host) ? "127.0.0.1" : options.Host;
        if (!IsLocalhost(host) && string.IsNullOrWhiteSpace(options.Token) && !options.Open)
        {
            Console.Error.WriteLine($"Error: Token required when binding to non-localhost address ({host}).");
            Console.Error.WriteLine();
            Console.Error.WriteLine("For security, a token is required when exposing the server to the network.");
            Console.Error.WriteLine("Either:");
            Console.Error.WriteLine("  1. Provide a token: --token <your-secret-token>");
            Console.Error.WriteLine("  2. Explicitly disable security: --open (not recommended)");
            Console.Error.WriteLine("  3. Bind to localhost only: --host 127.0.0.1");
            return 1;
        }

        if (!IsLocalhost(host) && options.Open && string.IsNullOrWhiteSpace(options.Token))
        {
            Console.Error.WriteLine("Warning: Server exposed without authentication (--open flag used)");
            Console.Error.WriteLine("Anyone with network access can view and control your sessions!");
            Console.Error.WriteLine();
        }

        var state = new StateStore(StatePaths.FromEnvironment());
        state.Init();

        var serverOptions = new StatusServerOptions
        {
            Host = host,
            Port = options.Port,
            Token = options.Token ?? string.Empty,
            Open = options.Open
        };

        var server = new StatusServer(serverOptions, state);
        Console.WriteLine($"Starting gralph status server on {host}:{options.Port}...");
        Console.WriteLine("Endpoints:");
        Console.WriteLine("  GET  /status        - Get all sessions");
        Console.WriteLine("  GET  /status/:name  - Get specific session");
        Console.WriteLine("  POST /stop/:name    - Stop a session");
        Console.WriteLine(string.IsNullOrWhiteSpace(options.Token)
            ? "Authentication: None (use --token to enable)"
            : "Authentication: Bearer token required");
        Console.WriteLine();
        Console.WriteLine("Press Ctrl+C to stop");
        Console.WriteLine();
        return server.Run();
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

    private static bool IsLocalhost(string host)
    {
        if (string.IsNullOrWhiteSpace(host))
        {
            return true;
        }

        return host switch
        {
            "127.0.0.1" => true,
            "localhost" => true,
            "::1" => true,
            _ => false
        };
    }

    private static string PromptRequired(string label, string currentValue, bool multiline)
    {
        if (!string.IsNullOrWhiteSpace(currentValue))
        {
            return currentValue.Trim();
        }

        var value = PromptInput(label, string.Empty, multiline);
        while (string.IsNullOrWhiteSpace(value))
        {
            Console.WriteLine($"{label} is required.");
            value = PromptInput(label, string.Empty, multiline);
        }

        return value;
    }

    private static string PromptOptional(string label, string currentValue, bool multiline)
    {
        if (!string.IsNullOrWhiteSpace(currentValue))
        {
            return currentValue.Trim();
        }

        return PromptInput(label, string.Empty, multiline);
    }

    private static string PromptInput(string label, string defaultValue, bool multiline)
    {
        if (multiline)
        {
            Console.WriteLine($"{label} (finish with empty line):");
            var lines = new List<string>();
            while (true)
            {
                var line = Console.ReadLine();
                if (line is null)
                {
                    break;
                }
                if (string.IsNullOrEmpty(line))
                {
                    break;
                }
                lines.Add(line);
            }

            if (lines.Count == 0)
            {
                return defaultValue;
            }

            return string.Join("\n", lines).Trim();
        }

        Console.Write($"{label}: ");
        var input = Console.ReadLine();
        if (string.IsNullOrWhiteSpace(input))
        {
            return defaultValue;
        }

        return input.Trim();
    }

    private static List<string> NormalizeCsvList(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return new List<string>();
        }

        var items = value
            .Split(new[] { ',', '\n' }, StringSplitOptions.RemoveEmptyEntries)
            .Select(item => item.Trim())
            .Where(item => !string.IsNullOrWhiteSpace(item))
            .Distinct(StringComparer.Ordinal)
            .ToList();
        return items;
    }

    private static List<string> BuildContextFileList(string targetDir, string? userContext, string configContext)
    {
        var combined = new List<string>();
        combined.AddRange(NormalizeCsvList(configContext));
        combined.AddRange(NormalizeCsvList(userContext));

        var results = new List<string>();
        foreach (var entry in combined)
        {
            if (!TryResolveContextEntry(entry, targetDir, out var displayPath, out var fullPath))
            {
                continue;
            }

            if (!File.Exists(fullPath) && !Directory.Exists(fullPath))
            {
                continue;
            }

            if (!results.Contains(displayPath, StringComparer.Ordinal))
            {
                results.Add(displayPath);
            }
        }

        return results;
    }

    private static bool TryResolveContextEntry(string entry, string baseDir, out string displayPath, out string fullPath)
    {
        displayPath = string.Empty;
        fullPath = string.Empty;

        if (string.IsNullOrWhiteSpace(entry))
        {
            return false;
        }

        if (Path.IsPathRooted(entry))
        {
            fullPath = Path.GetFullPath(entry);
            if (!IsSubPath(baseDir, fullPath))
            {
                return false;
            }
            displayPath = Path.GetRelativePath(baseDir, fullPath);
            return true;
        }

        fullPath = Path.GetFullPath(Path.Combine(baseDir, entry));
        if (!IsSubPath(baseDir, fullPath))
        {
            return false;
        }

        displayPath = entry.Replace('\u005c', '/');
        return true;
    }

    private static void TryDeleteFile(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return;
        }

        try
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }
        catch
        {
        }
    }

    private static bool IsSubPath(string baseDir, string path)
    {
        if (string.IsNullOrWhiteSpace(baseDir))
        {
            return true;
        }

        var comparison = OperatingSystem.IsWindows()
            ? StringComparison.OrdinalIgnoreCase
            : StringComparison.Ordinal;
        var normalizedBase = baseDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        if (!path.StartsWith(normalizedBase, comparison))
        {
            return false;
        }

        if (path.Length == normalizedBase.Length)
        {
            return true;
        }

        var next = path[normalizedBase.Length];
        return next == Path.DirectorySeparatorChar || next == Path.AltDirectorySeparatorChar;
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

    private static bool IsStartWorker()
    {
        return string.Equals(Environment.GetEnvironmentVariable(StartWorkerEnv), "1", StringComparison.Ordinal);
    }

    private static string SanitizeSessionName(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return string.Empty;
        }

        return Regex.Replace(value.Trim(), "[^a-zA-Z0-9_-]", "-");
    }

    private static int CountRemainingTasks(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return 0;
        }

        var blocks = PrdParser.GetTaskBlocks(taskFilePath);
        if (blocks.Count > 0)
        {
            return blocks.Sum(block => block.UncheckedCount);
        }

        var count = 0;
        foreach (var line in File.ReadLines(taskFilePath))
        {
            if (line.AsSpan().TrimStart().StartsWith("- [ ]", StringComparison.Ordinal))
            {
                count++;
            }
        }

        return count;
    }

    private static void StopSession(JsonObject session, string sessionName, IProcessInspector inspector, StateStore state, bool quietWarnings)
    {
        if (TryGetSessionInt(session, "pid", out var pid))
        {
            if (inspector.IsAlive(pid))
            {
                try
                {
                    using var process = Process.GetProcessById(pid);
                    process.Kill(true);
                }
                catch (Exception ex) when (ex is ArgumentException or InvalidOperationException)
                {
                    if (!quietWarnings)
                    {
                        Console.Error.WriteLine($"Warning: Failed to kill process {pid}: {ex.Message}");
                    }
                }
                catch (System.ComponentModel.Win32Exception ex)
                {
                    if (!quietWarnings)
                    {
                        Console.Error.WriteLine($"Warning: Failed to kill process {pid}: {ex.Message}");
                    }
                }
            }
            else if (!quietWarnings)
            {
                Console.Error.WriteLine("Warning: Process not found (may have already exited).");
            }
        }

        state.SetSession(sessionName, new Dictionary<string, object?>
        {
            ["status"] = "stopped",
            ["pid"] = string.Empty,
            ["tmux_session"] = string.Empty
        });
    }

    private static StartOptions BuildResumeOptions(JsonObject session, string backendName)
    {
        var options = new StartOptions
        {
            Backend = backendName,
            BackendSet = true,
            NoTmux = true,
            NoTmuxSet = true
        };

        if (TryGetSessionInt(session, "max_iterations", out var maxIterations))
        {
            options.MaxIterations = maxIterations;
            options.MaxIterationsSet = true;
        }

        var taskFile = GetSessionString(session, "task_file");
        if (!string.IsNullOrWhiteSpace(taskFile))
        {
            options.TaskFile = taskFile;
            options.TaskFileSet = true;
        }

        var completionMarker = GetSessionString(session, "completion_marker");
        if (!string.IsNullOrWhiteSpace(completionMarker))
        {
            options.CompletionMarker = completionMarker;
            options.CompletionMarkerSet = true;
        }

        var model = GetSessionString(session, "model");
        if (!string.IsNullOrWhiteSpace(model))
        {
            options.Model = model;
            options.ModelSet = true;
        }

        var variant = GetSessionString(session, "variant");
        if (!string.IsNullOrWhiteSpace(variant))
        {
            options.Variant = variant;
            options.VariantSet = true;
        }

        var webhook = GetSessionString(session, "webhook");
        if (!string.IsNullOrWhiteSpace(webhook))
        {
            options.Webhook = webhook;
            options.WebhookSet = true;
        }

        return options;
    }

    private static int ResolveRemainingTasks(string dir, string taskFile, JsonObject session)
    {
        if (!string.IsNullOrWhiteSpace(dir) && Directory.Exists(dir))
        {
            var path = Path.Combine(dir, taskFile);
            if (File.Exists(path))
            {
                return CountRemainingTasks(path);
            }
        }

        if (TryGetSessionInt(session, "last_task_count", out var lastCount))
        {
            return lastCount;
        }

        var lastCountString = GetSessionString(session, "last_task_count");
        if (!string.IsNullOrWhiteSpace(lastCountString)
            && int.TryParse(lastCountString, out var parsed))
        {
            return parsed;
        }

        return -1;
    }

    private static void TryNotifyCompletion(
        ConfigService config,
        string? optionWebhook,
        string sessionName,
        string projectDir,
        CoreLoopResult result)
    {
        if (!IsConfigEnabled(config, "notifications.on_complete", defaultValue: false))
        {
            return;
        }

        var webhookUrl = ResolveWebhookUrl(config, optionWebhook, sessionWebhook: null);
        if (string.IsNullOrWhiteSpace(webhookUrl))
        {
            return;
        }

        try
        {
            var notification = new CompletionNotification(sessionName, projectDir, result.Iterations, result.Duration);
            var success = WebhookNotifier.NotifyCompleteAsync(notification, webhookUrl).GetAwaiter().GetResult();
            if (!success)
            {
                Console.Error.WriteLine("Warning: Failed to send completion notification");
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Warning: Failed to send completion notification: {ex.Message}");
        }
    }

    private static void TryNotifyFailure(
        ConfigService config,
        string? optionWebhook,
        string? sessionWebhook,
        string sessionName,
        string projectDir,
        string reason,
        int iterations,
        int maxIterations,
        int remainingTasks,
        TimeSpan? duration)
    {
        if (!IsConfigEnabled(config, "notifications.on_fail", defaultValue: false))
        {
            return;
        }

        var webhookUrl = ResolveWebhookUrl(config, optionWebhook, sessionWebhook);
        if (string.IsNullOrWhiteSpace(webhookUrl))
        {
            return;
        }

        try
        {
            var notification = new FailureNotification(sessionName, projectDir, reason, iterations, maxIterations, remainingTasks, duration);
            var success = WebhookNotifier.NotifyFailedAsync(notification, webhookUrl).GetAwaiter().GetResult();
            if (!success)
            {
                Console.Error.WriteLine("Warning: Failed to send failure notification");
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Warning: Failed to send failure notification: {ex.Message}");
        }
    }

    private static void TryNotifyManualStop(JsonObject session, string sessionName)
    {
        var projectDir = GetSessionString(session, "dir") ?? string.Empty;
        var config = new ConfigService(ConfigPaths.FromEnvironment());
        config.Load(string.IsNullOrWhiteSpace(projectDir) ? null : projectDir);

        if (!IsConfigEnabled(config, "notifications.on_fail", defaultValue: false))
        {
            return;
        }

        var sessionWebhook = GetSessionString(session, "webhook");
        var webhookUrl = ResolveWebhookUrl(config, optionWebhook: null, sessionWebhook);
        if (string.IsNullOrWhiteSpace(webhookUrl))
        {
            return;
        }

        var iterations = TryGetSessionInt(session, "iteration", out var iter) ? iter : -1;
        var maxIterations = TryGetSessionInt(session, "max_iterations", out var maxIter) ? maxIter : -1;
        var remainingTasks = TryGetSessionInt(session, "last_task_count", out var remaining) ? remaining : -1;
        var duration = ResolveSessionDuration(session);

        try
        {
            var notification = new FailureNotification(
                sessionName,
                projectDir,
                "manual_stop",
                iterations,
                maxIterations,
                remainingTasks,
                duration);

            var success = WebhookNotifier.NotifyFailedAsync(notification, webhookUrl).GetAwaiter().GetResult();
            if (!success)
            {
                Console.Error.WriteLine("Warning: Failed to send stop notification");
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Warning: Failed to send stop notification: {ex.Message}");
        }
    }

    private static string ResolveWebhookUrl(ConfigService config, string? optionWebhook, string? sessionWebhook)
    {
        if (!string.IsNullOrWhiteSpace(optionWebhook))
        {
            return optionWebhook.Trim();
        }

        if (!string.IsNullOrWhiteSpace(sessionWebhook))
        {
            return sessionWebhook.Trim();
        }

        return config.Get("notifications.webhook", string.Empty).Trim();
    }

    private static bool IsConfigEnabled(ConfigService config, string key, bool defaultValue)
    {
        var rawValue = config.Get(key, defaultValue ? "true" : "false");
        return string.Equals(rawValue, "true", StringComparison.OrdinalIgnoreCase);
    }

    private static TimeSpan? ResolveSessionDuration(JsonObject session)
    {
        var startedAt = GetSessionString(session, "started_at");
        if (string.IsNullOrWhiteSpace(startedAt))
        {
            return null;
        }

        if (!DateTimeOffset.TryParse(startedAt, out var startTime))
        {
            return null;
        }

        var duration = DateTimeOffset.Now - startTime;
        if (duration < TimeSpan.Zero)
        {
            return TimeSpan.Zero;
        }

        return duration;
    }

    private static string FormatRemaining(int remaining)
    {
        return remaining switch
        {
            0 => "0 tasks",
            1 => "1 task",
            > 1 => $"{remaining} tasks",
            _ => "?"
        };
    }

    private static string TruncateDirectory(string dir, int maxLength)
    {
        if (string.IsNullOrWhiteSpace(dir) || dir.Length <= maxLength)
        {
            return dir;
        }

        if (maxLength <= 3)
        {
            return dir[..maxLength];
        }

        var tailLength = maxLength - 3;
        return $"...{dir[^tailLength..]}";
    }

    private static void PrintStatusTable(IReadOnlyList<StatusRow> rows)
    {
        var nameWidth = Math.Max("NAME".Length, rows.Max(row => row.Name.Length));
        var dirWidth = Math.Max("DIR".Length, rows.Max(row => row.Dir.Length));
        var iterWidth = Math.Max("ITERATION".Length, rows.Max(row => row.Iteration.Length));
        var statusWidth = Math.Max("STATUS".Length, rows.Max(row => row.Status.Length));
        var remainingWidth = Math.Max("REMAINING".Length, rows.Max(row => row.Remaining.Length));

        Console.WriteLine($"{Pad("NAME", nameWidth)}  {Pad("DIR", dirWidth)}  {Pad("ITERATION", iterWidth)}  {Pad("STATUS", statusWidth)}  {Pad("REMAINING", remainingWidth)}");
        Console.WriteLine($"{new string('-', nameWidth)}  {new string('-', dirWidth)}  {new string('-', iterWidth)}  {new string('-', statusWidth)}  {new string('-', remainingWidth)}");

        foreach (var row in rows)
        {
            Console.WriteLine($"{Pad(row.Name, nameWidth)}  {Pad(row.Dir, dirWidth)}  {Pad(row.Iteration, iterWidth)}  {Pad(row.Status, statusWidth)}  {Pad(row.Remaining, remainingWidth)}");
        }
    }

    private static string Pad(string value, int width)
    {
        value ??= string.Empty;
        return value.Length >= width ? value : value.PadRight(width);
    }

    private static IReadOnlyList<string> ReadLastLines(string path, int count)
    {
        if (count <= 0)
        {
            return Array.Empty<string>();
        }

        var queue = new Queue<string>();
        foreach (var line in File.ReadLines(path))
        {
            if (queue.Count == count)
            {
                queue.Dequeue();
            }
            queue.Enqueue(line);
        }

        return queue.ToList();
    }

    private static void FollowFile(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
        stream.Seek(0, SeekOrigin.End);
        using var reader = new StreamReader(stream);

        while (true)
        {
            var line = reader.ReadLine();
            if (line is not null)
            {
                Console.WriteLine(line);
                continue;
            }

            Thread.Sleep(250);
            if (stream.Length < stream.Position)
            {
                stream.Seek(0, SeekOrigin.End);
            }
        }
    }

    private static Process StartBackgroundProcess(string targetDir, string sessionName, StartOptions options)
    {
        var exePath = Environment.ProcessPath
            ?? Process.GetCurrentProcess().MainModule?.FileName
            ?? throw new InvalidOperationException("Unable to resolve gralph executable path.");

        var startInfo = new ProcessStartInfo
        {
            FileName = exePath,
            UseShellExecute = false,
            CreateNoWindow = true,
            WorkingDirectory = targetDir
        };

        startInfo.ArgumentList.Add("start");
        startInfo.ArgumentList.Add(targetDir);
        startInfo.ArgumentList.Add("--name");
        startInfo.ArgumentList.Add(sessionName);
        startInfo.ArgumentList.Add("--max-iterations");
        startInfo.ArgumentList.Add(options.MaxIterations.ToString());
        startInfo.ArgumentList.Add("--task-file");
        startInfo.ArgumentList.Add(options.TaskFile);
        startInfo.ArgumentList.Add("--completion-marker");
        startInfo.ArgumentList.Add(options.CompletionMarker);
        startInfo.ArgumentList.Add("--backend");
        startInfo.ArgumentList.Add(options.Backend);
        startInfo.ArgumentList.Add("--no-tmux");

        if (!string.IsNullOrWhiteSpace(options.Model))
        {
            startInfo.ArgumentList.Add("--model");
            startInfo.ArgumentList.Add(options.Model);
        }

        if (!string.IsNullOrWhiteSpace(options.Variant))
        {
            startInfo.ArgumentList.Add("--variant");
            startInfo.ArgumentList.Add(options.Variant);
        }

        if (!string.IsNullOrWhiteSpace(options.PromptTemplatePath))
        {
            startInfo.ArgumentList.Add("--prompt-template");
            startInfo.ArgumentList.Add(Path.GetFullPath(options.PromptTemplatePath));
        }

        if (!string.IsNullOrWhiteSpace(options.Webhook))
        {
            startInfo.ArgumentList.Add("--webhook");
            startInfo.ArgumentList.Add(options.Webhook);
        }

        if (options.StrictPrd)
        {
            startInfo.ArgumentList.Add("--strict-prd");
        }

        startInfo.Environment[StartWorkerEnv] = "1";

        var process = new Process { StartInfo = startInfo };
        if (!process.Start())
        {
            throw new InvalidOperationException("Failed to spawn background process.");
        }

        return process;
    }

    private static string? GetSessionString(JsonObject session, string key)
    {
        if (!session.TryGetPropertyValue(key, out var node) || node is null)
        {
            return null;
        }

        if (node is JsonValue value && value.TryGetValue<string>(out var stringValue))
        {
            return stringValue;
        }

        return node.ToString();
    }

    private static bool TryGetSessionInt(JsonObject session, string key, out int result)
    {
        result = 0;
        if (!session.TryGetPropertyValue(key, out var node) || node is null)
        {
            return false;
        }

        if (node is JsonValue value)
        {
            if (value.TryGetValue<int>(out var intValue))
            {
                result = intValue;
                return true;
            }

            if (value.TryGetValue<long>(out var longValue)
                && longValue is > int.MinValue and < int.MaxValue)
            {
                result = (int)longValue;
                return true;
            }

            if (value.TryGetValue<string>(out var stringValue)
                && int.TryParse(stringValue, out var parsed))
            {
                result = parsed;
                return true;
            }
        }

        return false;
    }

    private static bool IsValidTaskId(string taskId)
    {
        if (string.IsNullOrWhiteSpace(taskId))
        {
            return false;
        }

        return Regex.IsMatch(taskId, "^[A-Za-z]+-[0-9]+$");
    }

    private static string? GetGitRoot()
    {
        var result = RunProcess("git", ["rev-parse", "--show-toplevel"], Environment.CurrentDirectory);
        if (result.ExitCode != 0)
        {
            return null;
        }

        var path = result.StdOut.Trim();
        return string.IsNullOrWhiteSpace(path) ? null : path;
    }

    private static (int ExitCode, string StdOut, string StdErr) RunProcess(string fileName, IReadOnlyCollection<string> args, string? workingDirectory)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = fileName,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            WorkingDirectory = string.IsNullOrWhiteSpace(workingDirectory) ? Environment.CurrentDirectory : workingDirectory
        };

        foreach (var arg in args)
        {
            startInfo.ArgumentList.Add(arg);
        }

        using var process = new Process { StartInfo = startInfo };
        try
        {
            if (!process.Start())
            {
                return (-1, string.Empty, "Failed to start process.");
            }
        }
        catch (Exception ex) when (ex is InvalidOperationException or System.ComponentModel.Win32Exception)
        {
            return (-1, string.Empty, ex.Message);
        }

        var stdOut = process.StandardOutput.ReadToEnd();
        var stdErr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        return (process.ExitCode, stdOut.TrimEnd(), stdErr.TrimEnd());
    }

    private static void PrintUsage()
    {
        Console.WriteLine(UsageText);
    }

    private static void PrintVersion()
    {
        Console.WriteLine($"gralph {Version}");
    }

    private readonly record struct StatusRow(string Name, string Dir, string Iteration, string Status, string Remaining);

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
