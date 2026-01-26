using System.Diagnostics;
using Gralph.Backends;
using Gralph.Core;
using Gralph.State;

namespace Gralph.Commands;

public sealed class ResumeCommandHandler
{
    private readonly BackendRegistry _backendRegistry;
    private readonly StateStore _stateStore;

    public ResumeCommandHandler(BackendRegistry backendRegistry, StateStore stateStore)
    {
        _backendRegistry = backendRegistry ?? throw new ArgumentNullException(nameof(backendRegistry));
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
    }

    public int Execute(ResumeCommandSettings settings)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        var sessions = _stateStore.ListSessions();
        if (sessions.Count == 0)
        {
            Console.WriteLine("No sessions found");
            return 0;
        }

        var targetNames = ResolveTargetNames(settings.Name, sessions);
        if (targetNames is null)
        {
            return 1;
        }

        var resumed = 0;
        foreach (var sessionName in targetNames)
        {
            var session = _stateStore.GetSession(sessionName);
            if (session is null)
            {
                if (!string.IsNullOrWhiteSpace(settings.Name))
                {
                    Console.Error.WriteLine($"Error: Session not found: {sessionName}");
                    return 1;
                }

                continue;
            }

            if (!ShouldResume(session))
            {
                if (!string.IsNullOrWhiteSpace(settings.Name))
                {
                    var status = session.Status ?? "unknown";
                    var pidText = session.Pid?.ToString() ?? "none";
                    Console.WriteLine($"Warning: Session '{sessionName}' is already running (PID: {pidText}) or completed (status: {status})");
                }

                continue;
            }

            if (string.IsNullOrWhiteSpace(session.Dir) || !Directory.Exists(session.Dir))
            {
                Console.WriteLine($"Warning: Skipping '{sessionName}': directory no longer exists: {session.Dir}");
                continue;
            }

            var taskFile = string.IsNullOrWhiteSpace(session.TaskFile) ? "PRD.md" : session.TaskFile;
            var taskFilePath = Path.Combine(session.Dir, taskFile);
            if (!File.Exists(taskFilePath))
            {
                Console.WriteLine($"Warning: Skipping '{sessionName}': task file not found: {taskFilePath}");
                continue;
            }

            var backendName = string.IsNullOrWhiteSpace(session.Backend) ? BackendRegistry.DefaultBackendName : session.Backend;
            if (!_backendRegistry.TryGet(backendName, out var backend) || backend is null)
            {
                Console.WriteLine($"Warning: Skipping '{sessionName}': unknown backend '{backendName}'");
                continue;
            }

            if (!backend.IsInstalled())
            {
                Console.WriteLine($"Warning: Backend '{backendName}' CLI is not installed for session '{sessionName}'");
                Console.WriteLine($"Install with: {backend.GetInstallHint()}");
                continue;
            }

            var maxIterations = session.MaxIterations ?? 30;
            var completionMarker = string.IsNullOrWhiteSpace(session.CompletionMarker) ? "COMPLETE" : session.CompletionMarker;
            var remainingTasks = TaskBlockParser.CountRemainingTasks(taskFilePath);
            var logFile = ResolveLogFile(sessionName, session.Dir);

            var pid = StartBackgroundLoop(new ResumeLoopRequest
            {
                ProjectDir = session.Dir,
                TaskFile = taskFile,
                SessionName = sessionName,
                MaxIterations = maxIterations,
                CompletionMarker = completionMarker,
                BackendName = backendName,
                Model = session.Model,
                Variant = session.Variant,
                Webhook = session.Webhook
            });

            if (pid is null)
            {
                continue;
            }

            _stateStore.SetSession(sessionName, existing =>
            {
                existing.Name = sessionName;
                existing.Dir = session.Dir;
                existing.TaskFile = taskFile;
                existing.Pid = pid;
                existing.TmuxSession = null;
                existing.StartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                existing.Iteration = existing.Iteration is null or <= 0 ? 1 : existing.Iteration;
                existing.MaxIterations = maxIterations;
                existing.Status = "running";
                existing.LastTaskCount = remainingTasks;
                existing.CompletionMarker = completionMarker;
                existing.LogFile = logFile;
                existing.Backend = backendName;
                existing.Model = session.Model;
                existing.Variant = session.Variant;
                existing.Webhook = session.Webhook;
            });

            resumed++;
            Console.WriteLine($"Resumed session: {sessionName} (PID: {pid})");
        }

        if (resumed == 0)
        {
            Console.WriteLine("No sessions to resume");
            return 0;
        }

        Console.WriteLine();
        Console.WriteLine($"Resumed {resumed} session(s)");
        Console.WriteLine();
        Console.WriteLine("Commands: gralph status, gralph logs <name>, gralph stop <name>");
        return 0;
    }

    private static IReadOnlyList<string>? ResolveTargetNames(string? provided, IReadOnlyList<SessionState> sessions)
    {
        if (!string.IsNullOrWhiteSpace(provided))
        {
            return new[] { provided.Trim() };
        }

        return sessions
            .Select(session => session.Name)
            .Where(name => !string.IsNullOrWhiteSpace(name))
            .Select(name => name!)
            .ToList();
    }

    private static bool ShouldResume(SessionState session)
    {
        var status = session.Status ?? "unknown";
        if (string.Equals(status, "running", StringComparison.OrdinalIgnoreCase))
        {
            if (session.Pid is null or <= 0)
            {
                return true;
            }

            return !IsProcessAlive(session.Pid.Value);
        }

        return string.Equals(status, "stale", StringComparison.OrdinalIgnoreCase)
            || string.Equals(status, "stopped", StringComparison.OrdinalIgnoreCase);
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

    private static string ResolveLogFile(string sessionName, string projectDir)
    {
        return Path.Combine(projectDir, ".gralph", $"{sessionName}.log");
    }

    private static int? StartBackgroundLoop(ResumeLoopRequest request)
    {
        try
        {
            var processPath = Environment.ProcessPath ?? Process.GetCurrentProcess().MainModule?.FileName;
            if (string.IsNullOrWhiteSpace(processPath))
            {
                Console.Error.WriteLine("Error: Unable to determine executable path for background mode.");
                return null;
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
                return null;
            }

            return process.Id;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: Failed to resume background loop: {ex.Message}");
            return null;
        }
    }
}

public sealed class ResumeCommandSettings
{
    public string? Name { get; init; }
}

internal sealed class ResumeLoopRequest
{
    public string ProjectDir { get; init; } = string.Empty;
    public string TaskFile { get; init; } = "PRD.md";
    public string SessionName { get; init; } = "gralph";
    public int MaxIterations { get; init; } = 30;
    public string CompletionMarker { get; init; } = "COMPLETE";
    public string BackendName { get; init; } = BackendRegistry.DefaultBackendName;
    public string? Model { get; init; }
    public string? Variant { get; init; }
    public string? Webhook { get; init; }
}
