using System.Diagnostics;
using System.Text.RegularExpressions;
using Gralph.Backends;
using Gralph.Configuration;
using Gralph.Core;
using Gralph.Prd;
using Gralph.State;

namespace Gralph.Commands;

public sealed class StartCommandHandler
{
    private static readonly Regex SessionNameSanitizer = new("[^a-zA-Z0-9_-]", RegexOptions.Compiled);
    private readonly BackendRegistry _backendRegistry;
    private readonly StateStore _stateStore;

    public StartCommandHandler(BackendRegistry backendRegistry, StateStore stateStore)
    {
        _backendRegistry = backendRegistry ?? throw new ArgumentNullException(nameof(backendRegistry));
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
    }

    public async Task<int> ExecuteAsync(StartCommandSettings settings, CancellationToken cancellationToken)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        if (string.IsNullOrWhiteSpace(settings.Directory))
        {
            Console.Error.WriteLine("Error: Directory is required. Usage: gralph start <directory>");
            return 1;
        }

        var projectDir = Path.GetFullPath(settings.Directory);
        if (!Directory.Exists(projectDir))
        {
            Console.Error.WriteLine($"Error: Directory does not exist: {projectDir}");
            return 1;
        }

        Config.Load(projectDir);

        var maxIterations = ResolveMaxIterations(settings.MaxIterations);
        if (maxIterations <= 0)
        {
            Console.Error.WriteLine("Error: max_iterations must be a positive integer");
            return 1;
        }

        var taskFile = ResolveTaskFile(settings.TaskFile);
        if (string.IsNullOrWhiteSpace(taskFile))
        {
            Console.Error.WriteLine("Error: task file cannot be empty");
            return 1;
        }

        var taskFilePath = Path.Combine(projectDir, taskFile);
        if (!File.Exists(taskFilePath))
        {
            Console.Error.WriteLine($"Error: Task file does not exist: {taskFilePath}");
            return 1;
        }

        var completionMarker = ResolveCompletionMarker(settings.CompletionMarker);
        if (string.IsNullOrWhiteSpace(completionMarker))
        {
            Console.Error.WriteLine("Error: completion marker cannot be empty");
            return 1;
        }

        var backendName = ResolveBackendName(settings.Backend);
        if (!_backendRegistry.TryGet(backendName, out var backend) || backend is null)
        {
            var available = string.Join(", ", _backendRegistry.List().Select(item => item.Name));
            Console.Error.WriteLine($"Error: Unknown backend '{backendName}'. Available backends: {available}");
            return 1;
        }

        if (!backend.IsInstalled())
        {
            Console.Error.WriteLine($"Error: Backend '{backendName}' CLI is not installed");
            Console.Error.WriteLine($"Install with: {backend.GetInstallHint()}");
            return 1;
        }

        var model = ResolveModel(settings.Model, backendName, backend);

        if (settings.StrictPrd && !PrdValidator.Validate(taskFilePath, projectDir, Console.Error.WriteLine))
        {
            Console.Error.WriteLine("Error: PRD validation failed.");
            return 1;
        }

        var sessionName = ResolveSessionName(settings.Name, projectDir);
        if (!settings.BackgroundChild && !EnsureSessionAvailable(sessionName))
        {
            return 1;
        }

        var gralphDir = Path.Combine(projectDir, ".gralph");
        Directory.CreateDirectory(gralphDir);

        if (!string.IsNullOrWhiteSpace(settings.PromptTemplatePath))
        {
            if (!CopyPromptTemplate(settings.PromptTemplatePath, gralphDir))
            {
                return 1;
            }
        }

        var logFile = Path.Combine(gralphDir, $"{sessionName}.log");
        var initialRemaining = TaskBlockParser.CountRemainingTasks(taskFilePath);

        if (!settings.NoTmux && !settings.BackgroundChild)
        {
            return StartBackgroundLoop(new StartLoopRequest
            {
                ProjectDir = projectDir,
                TaskFile = taskFile,
                SessionName = sessionName,
                MaxIterations = maxIterations,
                CompletionMarker = completionMarker,
                BackendName = backendName,
                Model = model,
                Variant = settings.Variant,
                Webhook = settings.Webhook,
                InitialRemaining = initialRemaining,
                LogFile = logFile
            });
        }

        var pid = Environment.ProcessId;
        UpsertSession(sessionName, projectDir, taskFile, pid, null, maxIterations, completionMarker, initialRemaining, logFile, backendName, model, settings.Variant, settings.Webhook);

        var coreLoop = new CoreLoop(_backendRegistry);
        var loopOptions = new CoreLoopOptions
        {
            ProjectDir = projectDir,
            TaskFile = taskFile,
            MaxIterations = maxIterations,
            CompletionMarker = completionMarker,
            Model = model,
            SessionName = sessionName,
            BackendName = backendName,
            StateCallback = update => UpdateSessionState(sessionName, update)
        };

        var result = await coreLoop.RunAsync(loopOptions, cancellationToken);
        var status = result.Completed ? "complete" : "failed";
        _stateStore.SetSession(sessionName, session =>
        {
            session.Status = status;
            session.LastTaskCount = result.RemainingTasks;
            if (!string.IsNullOrWhiteSpace(result.LogFile))
            {
                session.LogFile = result.LogFile;
            }
        });

        return result.Completed ? 0 : 1;
    }

    private int StartBackgroundLoop(StartLoopRequest request)
    {
        try
        {
            var processPath = Environment.ProcessPath ?? Process.GetCurrentProcess().MainModule?.FileName;
            if (string.IsNullOrWhiteSpace(processPath))
            {
                Console.Error.WriteLine("Error: Unable to determine executable path for background mode.");
                return 1;
            }

            var startInfo = new ProcessStartInfo
            {
                FileName = processPath,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            startInfo.ArgumentList.Add("start");
            startInfo.ArgumentList.Add(request.ProjectDir);
            startInfo.ArgumentList.Add("--no-tmux");
            startInfo.ArgumentList.Add("--background-child");
            startInfo.ArgumentList.Add("--name");
            startInfo.ArgumentList.Add(request.SessionName);
            startInfo.ArgumentList.Add("--max-iterations");
            startInfo.ArgumentList.Add(request.MaxIterations.ToString());
            startInfo.ArgumentList.Add("--task-file");
            startInfo.ArgumentList.Add(request.TaskFile);
            startInfo.ArgumentList.Add("--completion-marker");
            startInfo.ArgumentList.Add(request.CompletionMarker);
            startInfo.ArgumentList.Add("--backend");
            startInfo.ArgumentList.Add(request.BackendName);

            if (!string.IsNullOrWhiteSpace(request.Model))
            {
                startInfo.ArgumentList.Add("--model");
                startInfo.ArgumentList.Add(request.Model);
            }

            if (!string.IsNullOrWhiteSpace(request.Variant))
            {
                startInfo.ArgumentList.Add("--variant");
                startInfo.ArgumentList.Add(request.Variant);
            }

            if (!string.IsNullOrWhiteSpace(request.Webhook))
            {
                startInfo.ArgumentList.Add("--webhook");
                startInfo.ArgumentList.Add(request.Webhook);
            }

            var process = Process.Start(startInfo);
            if (process is null)
            {
                Console.Error.WriteLine("Error: Failed to start background process.");
                return 1;
            }

            UpsertSession(
                request.SessionName,
                request.ProjectDir,
                request.TaskFile,
                process.Id,
                null,
                request.MaxIterations,
                request.CompletionMarker,
                request.InitialRemaining,
                request.LogFile,
                request.BackendName,
                request.Model,
                request.Variant,
                request.Webhook);

            Console.WriteLine($"Started gralph loop '{request.SessionName}' in background (PID: {process.Id}).");
            Console.WriteLine("Use `gralph status` to view running loops.");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: Failed to start background loop: {ex.Message}");
            return 1;
        }
    }

    private bool EnsureSessionAvailable(string sessionName)
    {
        var existing = _stateStore.GetSession(sessionName);
        if (existing is null)
        {
            return true;
        }

        if (!string.Equals(existing.Status, "running", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (existing.Pid is null or <= 0)
        {
            return true;
        }

        if (IsProcessAlive(existing.Pid.Value))
        {
            Console.Error.WriteLine($"Error: Session '{sessionName}' is already running (PID: {existing.Pid}).");
            return false;
        }

        Console.Error.WriteLine($"Warning: Session '{sessionName}' appears stale. Restarting...");
        return true;
    }

    private void UpdateSessionState(string sessionName, LoopStateUpdate update)
    {
        _stateStore.SetSession(sessionName, session =>
        {
            session.Iteration = update.Iteration;
            session.Status = update.Status;
            session.LastTaskCount = update.RemainingTasks;
        });
    }

    private void UpsertSession(
        string sessionName,
        string projectDir,
        string taskFile,
        int pid,
        string? tmuxSession,
        int maxIterations,
        string completionMarker,
        int remainingTasks,
        string logFile,
        string backendName,
        string? model,
        string? variant,
        string? webhook)
    {
        _stateStore.SetSession(sessionName, session =>
        {
            session.Name = sessionName;
            session.Dir = projectDir;
            session.TaskFile = taskFile;
            session.Pid = pid;
            session.TmuxSession = tmuxSession;
            session.StartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            session.Iteration = session.Iteration is null or <= 0 ? 1 : session.Iteration;
            session.MaxIterations = maxIterations;
            session.Status = "running";
            session.LastTaskCount = remainingTasks;
            session.CompletionMarker = completionMarker;
            session.LogFile = logFile;
            session.Backend = backendName;
            session.Model = model;
            session.Variant = variant;
            session.Webhook = webhook;
        });
    }

    private static bool IsProcessAlive(int pid)
    {
        try
        {
            var process = Process.GetProcessById(pid);
            return !process.HasExited;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
    }

    private static string ResolveSessionName(string? providedName, string projectDir)
    {
        var name = string.IsNullOrWhiteSpace(providedName)
            ? Path.GetFileName(projectDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar))
            : providedName.Trim();

        if (string.IsNullOrWhiteSpace(name))
        {
            name = "gralph";
        }

        name = SessionNameSanitizer.Replace(name, "-");
        return string.IsNullOrWhiteSpace(name) ? "gralph" : name;
    }

    private static int ResolveMaxIterations(int? provided)
    {
        if (provided.HasValue)
        {
            return provided.Value;
        }

        var raw = Config.Get("defaults.max_iterations", "30");
        return int.TryParse(raw, out var parsed) ? parsed : 30;
    }

    private static string ResolveTaskFile(string? provided)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return provided.Trim();
        }

        return Config.Get("defaults.task_file", "PRD.md");
    }

    private static string ResolveCompletionMarker(string? provided)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return provided.Trim();
        }

        return Config.Get("defaults.completion_marker", "COMPLETE");
    }

    private static string ResolveBackendName(string? provided)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return provided.Trim();
        }

        return Config.Get("defaults.backend", BackendRegistry.DefaultBackendName);
    }

    private static string? ResolveModel(string? provided, string backendName, IBackend backend)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return provided.Trim();
        }

        var model = Config.Get("defaults.model", string.Empty);
        if (string.IsNullOrWhiteSpace(model) && string.Equals(backendName, "opencode", StringComparison.OrdinalIgnoreCase))
        {
            model = Config.Get("opencode.default_model", string.Empty);
        }

        if (string.IsNullOrWhiteSpace(model))
        {
            model = backend.DefaultModel ?? string.Empty;
        }

        return string.IsNullOrWhiteSpace(model) ? null : model;
    }

    private static bool CopyPromptTemplate(string templatePath, string gralphDir)
    {
        var resolvedPath = Path.IsPathRooted(templatePath)
            ? templatePath
            : Path.GetFullPath(templatePath, Environment.CurrentDirectory);

        if (!File.Exists(resolvedPath))
        {
            Console.Error.WriteLine($"Error: Prompt template file does not exist: {resolvedPath}");
            return false;
        }

        var destination = Path.Combine(gralphDir, "prompt-template.txt");
        try
        {
            File.Copy(resolvedPath, destination, true);
            return true;
        }
        catch (IOException ex)
        {
            Console.Error.WriteLine($"Error: Failed to copy prompt template: {ex.Message}");
            return false;
        }
    }
}

public sealed class StartCommandSettings
{
    public string Directory { get; init; } = string.Empty;
    public string? Name { get; init; }
    public int? MaxIterations { get; init; }
    public string? TaskFile { get; init; }
    public string? CompletionMarker { get; init; }
    public string? Backend { get; init; }
    public string? Model { get; init; }
    public string? Variant { get; init; }
    public string? PromptTemplatePath { get; init; }
    public string? Webhook { get; init; }
    public bool NoTmux { get; init; }
    public bool StrictPrd { get; init; }
    public bool BackgroundChild { get; init; }
}

internal sealed class StartLoopRequest
{
    public string ProjectDir { get; init; } = string.Empty;
    public string TaskFile { get; init; } = "PRD.md";
    public string SessionName { get; init; } = "gralph";
    public int MaxIterations { get; init; }
    public string CompletionMarker { get; init; } = "COMPLETE";
    public string BackendName { get; init; } = BackendRegistry.DefaultBackendName;
    public string? Model { get; init; }
    public string? Variant { get; init; }
    public string? Webhook { get; init; }
    public int InitialRemaining { get; init; }
    public string LogFile { get; init; } = string.Empty;
}
