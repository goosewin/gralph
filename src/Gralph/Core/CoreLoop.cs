using System.Globalization;
using Gralph.Backends;
using Gralph.Configuration;

namespace Gralph.Core;

public sealed class CoreLoop
{
    private readonly BackendRegistry _backendRegistry;

    public CoreLoop(BackendRegistry backendRegistry)
    {
        _backendRegistry = backendRegistry;
    }

    public async Task<LoopResult> RunAsync(CoreLoopOptions options, CancellationToken cancellationToken)
    {
        if (options is null)
        {
            throw new ArgumentNullException(nameof(options));
        }

        if (string.IsNullOrWhiteSpace(options.ProjectDir))
        {
            throw new ArgumentException("Project directory is required.", nameof(options));
        }

        if (options.MaxIterations <= 0)
        {
            throw new ArgumentException("MaxIterations must be a positive integer.", nameof(options));
        }

        var projectDir = Path.GetFullPath(options.ProjectDir);
        if (!Directory.Exists(projectDir))
        {
            throw new DirectoryNotFoundException($"Project directory does not exist: {projectDir}");
        }

        var taskFile = string.IsNullOrWhiteSpace(options.TaskFile) ? "PRD.md" : options.TaskFile;
        var taskFilePath = Path.Combine(projectDir, taskFile);
        if (!File.Exists(taskFilePath))
        {
            throw new FileNotFoundException($"Task file does not exist: {taskFilePath}");
        }

        Config.Load(projectDir);

        var completionMarker = string.IsNullOrWhiteSpace(options.CompletionMarker)
            ? Config.Get("defaults.completion_marker", "COMPLETE")
            : options.CompletionMarker;

        var logDir = Path.Combine(projectDir, ".gralph");
        Directory.CreateDirectory(logDir);
        CleanupOldLogs(logDir, Config.Get("logging.retain_days", "7"));

        var logName = string.IsNullOrWhiteSpace(options.SessionName) ? "gralph" : options.SessionName;
        var logFile = Path.Combine(logDir, $"{logName}.log");

        var startTime = DateTimeOffset.UtcNow;
        WriteLog(logFile, $"Starting gralph loop in {projectDir}");
        WriteLog(logFile, $"Task file: {taskFile}");
        WriteLog(logFile, $"Max iterations: {options.MaxIterations}");
        WriteLog(logFile, $"Completion marker: {completionMarker}");
        if (!string.IsNullOrWhiteSpace(options.Model))
        {
            WriteLog(logFile, $"Model: {options.Model}");
        }

        WriteLog(logFile, $"Started at: {DateTimeOffset.Now:O}");
        var initialRemaining = TaskBlockParser.CountRemainingTasks(taskFilePath);
        WriteLog(logFile, $"Initial remaining tasks: {initialRemaining}");

        for (var iteration = 1; iteration <= options.MaxIterations; iteration++)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var remainingBefore = TaskBlockParser.CountRemainingTasks(taskFilePath);
            WriteLog(logFile, string.Empty);
            WriteLog(logFile, $"=== Iteration {iteration}/{options.MaxIterations} (Remaining: {remainingBefore}) ===");

            if (remainingBefore == 0)
            {
                WriteLog(logFile, "Zero tasks remaining before iteration, verifying completion...");
            }

            options.StateCallback?.Invoke(new LoopStateUpdate(logName, iteration, "running", remainingBefore));

            options.CurrentIteration = iteration;
            var iterationResult = await RunIterationAsync(
                options,
                projectDir,
                taskFile,
                taskFilePath,
                completionMarker,
                logFile,
                cancellationToken);

            if (!iterationResult.Success)
            {
                WriteLog(logFile, $"Iteration failed with exit code {iterationResult.ExitCode}.");
                if (!string.IsNullOrWhiteSpace(iterationResult.OutputFile) && File.Exists(iterationResult.OutputFile))
                {
                    WriteLog(logFile, $"Raw backend output: {iterationResult.OutputFile}");
                }

                options.StateCallback?.Invoke(new LoopStateUpdate(logName, iteration, "failed", remainingBefore));

                return new LoopResult
                {
                    Completed = false,
                    Iterations = iteration,
                    RemainingTasks = remainingBefore,
                    Duration = DateTimeOffset.UtcNow - startTime,
                    LogFile = logFile
                };
            }

            if (CompletionDetector.IsComplete(taskFilePath, iterationResult.ResultText, completionMarker))
            {
                var duration = DateTimeOffset.UtcNow - startTime;
                WriteLog(logFile, string.Empty);
                WriteLog(logFile, $"Gralph complete after {iteration} iterations.");
                WriteLog(logFile, $"Duration: {duration.TotalSeconds.ToString(CultureInfo.InvariantCulture)}s");
                WriteLog(logFile, $"FINISHED: {DateTimeOffset.Now:O}");
                options.StateCallback?.Invoke(new LoopStateUpdate(logName, iteration, "complete", 0));

                return new LoopResult
                {
                    Completed = true,
                    Iterations = iteration,
                    RemainingTasks = 0,
                    Duration = duration,
                    LogFile = logFile
                };
            }

            var remainingAfter = TaskBlockParser.CountRemainingTasks(taskFilePath);
            WriteLog(logFile, $"Tasks remaining after iteration: {remainingAfter}");
            options.StateCallback?.Invoke(new LoopStateUpdate(logName, iteration, "running", remainingAfter));

            if (iteration < options.MaxIterations)
            {
                await Task.Delay(TimeSpan.FromSeconds(2), cancellationToken);
            }
        }

        var finalRemaining = TaskBlockParser.CountRemainingTasks(taskFilePath);
        var totalDuration = DateTimeOffset.UtcNow - startTime;
        WriteLog(logFile, string.Empty);
        WriteLog(logFile, $"Hit max iterations ({options.MaxIterations})");
        WriteLog(logFile, $"Remaining tasks: {finalRemaining}");
        WriteLog(logFile, $"Duration: {totalDuration.TotalSeconds.ToString(CultureInfo.InvariantCulture)}s");
        WriteLog(logFile, $"FINISHED: {DateTimeOffset.Now:O}");
        options.StateCallback?.Invoke(new LoopStateUpdate(logName, options.MaxIterations, "max_iterations", finalRemaining));

        return new LoopResult
        {
            Completed = false,
            Iterations = options.MaxIterations,
            RemainingTasks = finalRemaining,
            Duration = totalDuration,
            LogFile = logFile
        };
    }

    private async Task<IterationRunResult> RunIterationAsync(
        CoreLoopOptions options,
        string projectDir,
        string taskFile,
        string taskFilePath,
        string completionMarker,
        string logFile,
        CancellationToken cancellationToken)
    {
        var backendName = string.IsNullOrWhiteSpace(options.BackendName)
            ? Config.Get("defaults.backend", BackendRegistry.DefaultBackendName)
            : options.BackendName;

        var backend = _backendRegistry.Get(backendName);
        if (!backend.IsInstalled())
        {
            WriteLog(logFile, $"Error: Backend '{backend.Name}' CLI is not installed");
            WriteLog(logFile, $"Install with: {backend.GetInstallHint()}");
            return new IterationRunResult { Success = false, ExitCode = 1 };
        }

        var promptTemplate = ResolvePromptTemplate(options, projectDir);

        var taskBlock = TaskBlockParser.GetNextUncheckedTaskBlock(taskFilePath);
        if (string.IsNullOrWhiteSpace(taskBlock))
        {
            var remaining = TaskBlockParser.CountRemainingTasks(taskFilePath);
            if (remaining > 0)
            {
                taskBlock = TaskBlockParser.GetFirstUncheckedLine(taskFilePath);
            }
        }

        var contextFilesRaw = Config.Get("defaults.context_files", string.Empty);
        var normalizedContextFiles = PromptRenderer.NormalizeContextFiles(contextFilesRaw);
        var prompt = PromptRenderer.Render(
            promptTemplate,
            taskFile,
            completionMarker,
            options.CurrentIteration,
            options.MaxIterations,
            taskBlock,
            normalizedContextFiles);

        var rawOutputFile = GetRawOutputFile(logFile);
        EnsureDirectory(rawOutputFile);

        var originalDirectory = Environment.CurrentDirectory;
        try
        {
            Environment.CurrentDirectory = projectDir;
            var exitCode = await backend.RunIterationAsync(prompt, options.Model, rawOutputFile, cancellationToken);

            if (exitCode != 0)
            {
                return new IterationRunResult
                {
                    Success = false,
                    ExitCode = exitCode,
                    OutputFile = rawOutputFile
                };
            }

            if (!File.Exists(rawOutputFile) || new FileInfo(rawOutputFile).Length == 0)
            {
                WriteLog(logFile, $"Error: backend '{backend.Name}' produced no JSON output.");
                return new IterationRunResult
                {
                    Success = false,
                    ExitCode = 1,
                    OutputFile = rawOutputFile
                };
            }

            var result = backend.ParseText(rawOutputFile);
            if (string.IsNullOrWhiteSpace(result))
            {
                WriteLog(logFile, $"Error: backend '{backend.Name}' returned no parsed result.");
                return new IterationRunResult
                {
                    Success = false,
                    ExitCode = 1,
                    OutputFile = rawOutputFile
                };
            }

            return new IterationRunResult
            {
                Success = true,
                ExitCode = 0,
                ResultText = result,
                OutputFile = rawOutputFile
            };
        }
        finally
        {
            Environment.CurrentDirectory = originalDirectory;
        }
    }

    private static string ResolvePromptTemplate(CoreLoopOptions options, string projectDir)
    {
        if (!string.IsNullOrWhiteSpace(options.PromptTemplate))
        {
            return options.PromptTemplate;
        }

        var envTemplate = Environment.GetEnvironmentVariable("GRALPH_PROMPT_TEMPLATE_FILE");
        if (!string.IsNullOrWhiteSpace(envTemplate) && File.Exists(envTemplate))
        {
            return File.ReadAllText(envTemplate);
        }

        var projectTemplate = Path.Combine(projectDir, ".gralph", "prompt-template.txt");
        if (File.Exists(projectTemplate))
        {
            return File.ReadAllText(projectTemplate);
        }

        return PromptTemplates.Default;
    }

    private static void CleanupOldLogs(string logDir, string retainDaysRaw)
    {
        if (string.IsNullOrWhiteSpace(logDir) || !Directory.Exists(logDir))
        {
            return;
        }

        var retainDays = 7;
        if (!string.IsNullOrWhiteSpace(retainDaysRaw)
            && int.TryParse(retainDaysRaw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            && parsed > 0)
        {
            retainDays = parsed;
        }

        var threshold = DateTime.UtcNow.AddDays(-retainDays);
        foreach (var file in Directory.EnumerateFiles(logDir, "*.log", SearchOption.TopDirectoryOnly))
        {
            try
            {
                var info = new FileInfo(file);
                if (info.LastWriteTimeUtc < threshold)
                {
                    info.Delete();
                }
            }
            catch (IOException)
            {
                continue;
            }
        }
    }

    private static void WriteLog(string logFile, string message)
    {
        var line = message ?? string.Empty;
        File.AppendAllText(logFile, line + Environment.NewLine);
        if (!string.IsNullOrEmpty(line))
        {
            Console.WriteLine(line);
        }
    }

    private static void EnsureDirectory(string filePath)
    {
        var directory = Path.GetDirectoryName(filePath);
        if (!string.IsNullOrWhiteSpace(directory))
        {
            Directory.CreateDirectory(directory);
        }
    }

    private static string GetRawOutputFile(string logFile)
    {
        if (logFile.EndsWith(".log", StringComparison.OrdinalIgnoreCase))
        {
            return Path.Combine(
                Path.GetDirectoryName(logFile) ?? string.Empty,
                Path.GetFileNameWithoutExtension(logFile) + ".raw.log");
        }

        return logFile + ".raw.log";
    }
}

public sealed class CoreLoopOptions
{
    public string ProjectDir { get; init; } = string.Empty;
    public string TaskFile { get; init; } = "PRD.md";
    public int MaxIterations { get; init; } = 30;
    public string CompletionMarker { get; init; } = "COMPLETE";
    public string? Model { get; init; }
    public string? SessionName { get; init; }
    public string? PromptTemplate { get; init; }
    public string? BackendName { get; init; }
    public Action<LoopStateUpdate>? StateCallback { get; init; }

    internal int CurrentIteration { get; set; }
}

public sealed class LoopResult
{
    public bool Completed { get; init; }
    public int Iterations { get; init; }
    public int RemainingTasks { get; init; }
    public TimeSpan Duration { get; init; }
    public string? LogFile { get; init; }
}

public sealed class LoopStateUpdate
{
    public LoopStateUpdate(string? sessionName, int iteration, string status, int remainingTasks)
    {
        SessionName = sessionName;
        Iteration = iteration;
        Status = status;
        RemainingTasks = remainingTasks;
    }

    public string? SessionName { get; }
    public int Iteration { get; }
    public string Status { get; }
    public int RemainingTasks { get; }
}

internal sealed class IterationRunResult
{
    public bool Success { get; init; }
    public int ExitCode { get; init; }
    public string ResultText { get; init; } = string.Empty;
    public string OutputFile { get; init; } = string.Empty;
}
