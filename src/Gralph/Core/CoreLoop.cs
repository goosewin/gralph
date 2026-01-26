using System;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using Gralph.Backends;
using Gralph.Config;
using Gralph.Prd;

namespace Gralph.Core;

public enum CoreLoopStatus
{
    Complete,
    MaxIterations,
    Failed
}

public sealed record CoreLoopOptions(string ProjectDir)
{
    public string TaskFile { get; init; } = "PRD.md";
    public int MaxIterations { get; init; } = 30;
    public string CompletionMarker { get; init; } = "COMPLETE";
    public string? ModelOverride { get; init; }
    public string? SessionName { get; init; }
    public string? PromptTemplate { get; init; }
    public string? PromptTemplatePath { get; init; }
    public string? LogFilePath { get; init; }
}

public sealed record LoopStateUpdate(string SessionName, int Iteration, string Status, int RemainingTasks);

public sealed record CoreLoopResult(CoreLoopStatus Status, int Iterations, int RemainingTasks, TimeSpan Duration, string? ErrorMessage = null);

public sealed class CoreLoop
{
    private const string DefaultPromptTemplate = "Read {task_file} carefully. Find any task marked '- [ ]' (unchecked).\n\n" +
                                                 "If unchecked tasks exist:\n" +
                                                 "- Complete ONE task fully\n" +
                                                 "- Mark it '- [x]' in {task_file}\n" +
                                                 "- Commit changes\n" +
                                                 "- Exit normally (do NOT output completion promise)\n\n" +
                                                 "If ZERO '- [ ]' remain (all complete):\n" +
                                                 "- Verify by searching the file\n" +
                                                 "- Output ONLY: <promise>{completion_marker}</promise>\n\n" +
                                                 "CRITICAL: Never mention the promise unless outputting it as the completion signal.\n\n" +
                                                 "{context_files_section}" +
                                                 "Task Block:\n" +
                                                 "{task_block}\n\n" +
                                                 "Iteration: {iteration}/{max_iterations}";

    private static readonly Regex UncheckedRegex = new("^\\s*-\\s*\\[\\s\\]", RegexOptions.Compiled);
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+", RegexOptions.Compiled);
    private static readonly Regex PromiseRegex = new("^\\s*<promise>(?<marker>[^<]+)</promise>\\s*$", RegexOptions.Compiled | RegexOptions.IgnoreCase);
    private static readonly Regex NegatedPromiseRegex = new("(cannot|can't|won't|will not|do not|don't|should not|shouldn't|must not|mustn't)[^<]*<promise>", RegexOptions.Compiled | RegexOptions.IgnoreCase);

    private readonly ConfigService _config;
    private readonly IBackend _backend;

    public CoreLoop(ConfigService config, IBackend backend)
    {
        _config = config ?? throw new ArgumentNullException(nameof(config));
        _backend = backend ?? throw new ArgumentNullException(nameof(backend));
    }

    public async Task<CoreLoopResult> RunAsync(
        CoreLoopOptions options,
        Action<LoopStateUpdate>? stateCallback = null,
        CancellationToken cancellationToken = default)
    {
        if (options is null)
        {
            throw new ArgumentNullException(nameof(options));
        }

        if (string.IsNullOrWhiteSpace(options.ProjectDir))
        {
            return new CoreLoopResult(CoreLoopStatus.Failed, 0, 0, TimeSpan.Zero, "Project directory is required.");
        }

        if (!Directory.Exists(options.ProjectDir))
        {
            return new CoreLoopResult(CoreLoopStatus.Failed, 0, 0, TimeSpan.Zero, $"Project directory does not exist: {options.ProjectDir}");
        }

        if (options.MaxIterations <= 0)
        {
            return new CoreLoopResult(CoreLoopStatus.Failed, 0, 0, TimeSpan.Zero, "Max iterations must be a positive integer.");
        }

        if (!_backend.IsInstalled())
        {
            var hint = _backend.GetInstallHint();
            return new CoreLoopResult(CoreLoopStatus.Failed, 0, 0, TimeSpan.Zero, $"Backend '{_backend.Name}' is not installed. {hint}");
        }

        var projectDir = Path.GetFullPath(options.ProjectDir);
        var taskFile = string.IsNullOrWhiteSpace(options.TaskFile) ? "PRD.md" : options.TaskFile.Trim();
        var fullTaskPath = Path.Combine(projectDir, taskFile);
        if (!File.Exists(fullTaskPath))
        {
            return new CoreLoopResult(CoreLoopStatus.Failed, 0, 0, TimeSpan.Zero, $"Task file does not exist: {fullTaskPath}");
        }

        var logFile = ResolveLogFile(options, projectDir);
        var startTime = DateTimeOffset.UtcNow;

        LogLine(logFile, $"Starting gralph loop in {projectDir}");
        LogLine(logFile, $"Task file: {taskFile}");
        LogLine(logFile, $"Max iterations: {options.MaxIterations}");
        LogLine(logFile, $"Completion marker: {options.CompletionMarker}");
        if (!string.IsNullOrWhiteSpace(options.ModelOverride))
        {
            LogLine(logFile, $"Model: {options.ModelOverride}");
        }
        LogLine(logFile, $"Started at: {DateTimeOffset.Now:O}");

        var initialRemaining = CountRemainingTasks(fullTaskPath);
        LogLine(logFile, $"Initial remaining tasks: {initialRemaining}");

        var promptTemplate = ResolvePromptTemplate(options, projectDir);
        var contextFiles = _config.Get("defaults.context_files", string.Empty);
        var normalizedContextFiles = NormalizeContextFiles(contextFiles);

        for (var iteration = 1; iteration <= options.MaxIterations; iteration++)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var remainingBefore = CountRemainingTasks(fullTaskPath);
            LogLine(logFile, string.Empty);
            LogLine(logFile, $"=== Iteration {iteration}/{options.MaxIterations} (Remaining: {remainingBefore}) ===");

            stateCallback?.Invoke(new LoopStateUpdate(options.SessionName ?? string.Empty, iteration, "running", remainingBefore));

            var iterationResult = await RunIterationAsync(
                projectDir,
                taskFile,
                fullTaskPath,
                iteration,
                options.MaxIterations,
                options.CompletionMarker,
                options.ModelOverride,
                logFile,
                promptTemplate,
                normalizedContextFiles,
                cancellationToken);

            if (iterationResult.ExitCode != 0 || string.IsNullOrWhiteSpace(iterationResult.ParsedText))
            {
                LogLine(logFile, $"Iteration failed with exit code {iterationResult.ExitCode}.");
                stateCallback?.Invoke(new LoopStateUpdate(options.SessionName ?? string.Empty, iteration, "failed", remainingBefore));
                var duration = DateTimeOffset.UtcNow - startTime;
                return new CoreLoopResult(CoreLoopStatus.Failed, iteration, remainingBefore, duration, "Backend iteration failed.");
            }

            if (IsCompletion(fullTaskPath, iterationResult.ParsedText, options.CompletionMarker))
            {
                var duration = DateTimeOffset.UtcNow - startTime;
                LogLine(logFile, string.Empty);
                LogLine(logFile, $"Gralph complete after {iteration} iterations.");
                LogLine(logFile, $"Duration: {Math.Round(duration.TotalSeconds)}s");
                LogLine(logFile, $"FINISHED: {DateTimeOffset.Now:O}");
                stateCallback?.Invoke(new LoopStateUpdate(options.SessionName ?? string.Empty, iteration, "complete", 0));
                return new CoreLoopResult(CoreLoopStatus.Complete, iteration, 0, duration);
            }

            var remainingAfter = CountRemainingTasks(fullTaskPath);
            LogLine(logFile, $"Tasks remaining after iteration: {remainingAfter}");
            stateCallback?.Invoke(new LoopStateUpdate(options.SessionName ?? string.Empty, iteration, "running", remainingAfter));

            if (iteration < options.MaxIterations)
            {
                await Task.Delay(TimeSpan.FromSeconds(2), cancellationToken);
            }
        }

        var finalRemaining = CountRemainingTasks(fullTaskPath);
        var finalDuration = DateTimeOffset.UtcNow - startTime;
        LogLine(logFile, string.Empty);
        LogLine(logFile, $"Hit max iterations ({options.MaxIterations})");
        LogLine(logFile, $"Remaining tasks: {finalRemaining}");
        LogLine(logFile, $"Duration: {Math.Round(finalDuration.TotalSeconds)}s");
        LogLine(logFile, $"FINISHED: {DateTimeOffset.Now:O}");
        stateCallback?.Invoke(new LoopStateUpdate(options.SessionName ?? string.Empty, options.MaxIterations, "max_iterations", finalRemaining));
        return new CoreLoopResult(CoreLoopStatus.MaxIterations, options.MaxIterations, finalRemaining, finalDuration);
    }

    private async Task<BackendRunResult> RunIterationAsync(
        string projectDir,
        string taskFile,
        string fullTaskPath,
        int iteration,
        int maxIterations,
        string completionMarker,
        string? modelOverride,
        string logFile,
        string promptTemplate,
        string normalizedContextFiles,
        CancellationToken cancellationToken)
    {
        var taskBlock = GetNextUncheckedTaskBlock(fullTaskPath);
        if (string.IsNullOrWhiteSpace(taskBlock))
        {
            taskBlock = "No task block available.";
        }

        var prompt = RenderPromptTemplate(
            promptTemplate,
            taskFile,
            completionMarker,
            iteration,
            maxIterations,
            taskBlock,
            normalizedContextFiles);

        var outputPath = Path.Combine(Path.GetTempPath(), $"gralph-iter-{Guid.NewGuid():N}.json");
        var rawOutputPath = string.IsNullOrWhiteSpace(logFile) ? null : GetRawLogPath(logFile);

        var request = new BackendRunRequest(prompt, modelOverride, outputPath, rawOutputPath);
        var previousDir = Directory.GetCurrentDirectory();
        try
        {
            Directory.SetCurrentDirectory(projectDir);
            var result = await _backend.RunIterationAsync(request, cancellationToken);
            return result;
        }
        finally
        {
            Directory.SetCurrentDirectory(previousDir);
            if (File.Exists(outputPath))
            {
                File.Delete(outputPath);
            }
        }
    }

    private static string ResolveLogFile(CoreLoopOptions options, string projectDir)
    {
        if (!string.IsNullOrWhiteSpace(options.LogFilePath))
        {
            var logDir = Path.GetDirectoryName(options.LogFilePath);
            if (!string.IsNullOrWhiteSpace(logDir))
            {
                Directory.CreateDirectory(logDir);
            }
            return options.LogFilePath;
        }

        var logDirectory = Path.Combine(projectDir, ".gralph");
        Directory.CreateDirectory(logDirectory);
        var logName = string.IsNullOrWhiteSpace(options.SessionName) ? "gralph" : options.SessionName.Trim();
        return Path.Combine(logDirectory, $"{logName}.log");
    }

    private static string GetRawLogPath(string logFile)
    {
        return logFile.EndsWith(".log", StringComparison.OrdinalIgnoreCase)
            ? logFile[..^4] + ".raw.log"
            : logFile + ".raw.log";
    }

    private static void LogLine(string logFile, string message)
    {
        if (string.IsNullOrWhiteSpace(logFile))
        {
            return;
        }

        var line = message ?? string.Empty;
        File.AppendAllText(logFile, line + Environment.NewLine);
    }

    private static string ResolvePromptTemplate(CoreLoopOptions options, string projectDir)
    {
        if (!string.IsNullOrWhiteSpace(options.PromptTemplate))
        {
            return options.PromptTemplate;
        }

        var templatePath = options.PromptTemplatePath;
        if (string.IsNullOrWhiteSpace(templatePath))
        {
            templatePath = Path.Combine(projectDir, ".gralph", "prompt-template.txt");
        }

        if (File.Exists(templatePath))
        {
            return File.ReadAllText(templatePath);
        }

        return DefaultPromptTemplate;
    }

    private static string NormalizeContextFiles(string raw)
    {
        if (string.IsNullOrWhiteSpace(raw))
        {
            return string.Empty;
        }

        var entries = raw.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Where(entry => !string.IsNullOrWhiteSpace(entry))
            .ToArray();

        return entries.Length == 0 ? string.Empty : string.Join(Environment.NewLine, entries);
    }

    private static string RenderPromptTemplate(
        string template,
        string taskFile,
        string completionMarker,
        int iteration,
        int maxIterations,
        string taskBlock,
        string normalizedContextFiles)
    {
        var contextFilesSection = string.Empty;
        if (!string.IsNullOrWhiteSpace(normalizedContextFiles))
        {
            contextFilesSection = "Context Files (read these first):\n" + normalizedContextFiles + "\n\n";
        }

        var rendered = template;
        rendered = rendered.Replace("{task_file}", taskFile, StringComparison.Ordinal);
        rendered = rendered.Replace("{completion_marker}", completionMarker, StringComparison.Ordinal);
        rendered = rendered.Replace("{iteration}", iteration.ToString(), StringComparison.Ordinal);
        rendered = rendered.Replace("{max_iterations}", maxIterations.ToString(), StringComparison.Ordinal);
        rendered = rendered.Replace("{task_block}", taskBlock, StringComparison.Ordinal);
        rendered = rendered.Replace("{context_files}", normalizedContextFiles, StringComparison.Ordinal);
        rendered = rendered.Replace("{context_files_section}", contextFilesSection, StringComparison.Ordinal);
        return rendered;
    }

    private static int CountRemainingTasks(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return 0;
        }

        var taskBlocks = PrdParser.GetTaskBlocks(taskFilePath);
        if (taskBlocks.Count > 0)
        {
            return taskBlocks.Sum(block => block.UncheckedCount);
        }

        var count = 0;
        foreach (var line in File.ReadLines(taskFilePath))
        {
            if (UncheckedRegex.IsMatch(line))
            {
                count++;
            }
        }
        return count;
    }

    private static string? GetNextUncheckedTaskBlock(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return null;
        }

        foreach (var block in PrdParser.GetTaskBlocks(taskFilePath))
        {
            if (block.UncheckedCount > 0)
            {
                return block.RawText;
            }
        }

        if (!File.ReadLines(taskFilePath).Any(line => TaskHeaderRegex.IsMatch(line)))
        {
            foreach (var line in File.ReadLines(taskFilePath))
            {
                if (UncheckedRegex.IsMatch(line))
                {
                    return line;
                }
            }
        }

        return null;
    }

    private static bool IsCompletion(string taskFilePath, string result, string completionMarker)
    {
        if (string.IsNullOrWhiteSpace(result))
        {
            return false;
        }

        if (CountRemainingTasks(taskFilePath) > 0)
        {
            return false;
        }

        var lastLine = result
            .Split(new[] { "\r\n", "\n" }, StringSplitOptions.None)
            .Reverse()
            .FirstOrDefault(line => !string.IsNullOrWhiteSpace(line));

        if (string.IsNullOrWhiteSpace(lastLine))
        {
            return false;
        }

        var match = PromiseRegex.Match(lastLine);
        if (!match.Success)
        {
            return false;
        }

        var marker = match.Groups["marker"].Value.Trim();
        if (!string.Equals(marker, completionMarker, StringComparison.Ordinal))
        {
            return false;
        }

        if (NegatedPromiseRegex.IsMatch(lastLine))
        {
            return false;
        }

        return true;
    }
}
