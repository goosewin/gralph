using System.Text.RegularExpressions;
using Gralph.Core;

namespace Gralph.Prd;

public static class PrdValidator
{
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+", RegexOptions.Compiled);
    private static readonly Regex BlockTerminatorRegex = new("^\\s*(---\\s*$|##\\s+)", RegexOptions.Compiled);
    private static readonly Regex UncheckedRegex = new("^\\s*-\\s*\\[\\s\\]", RegexOptions.Compiled);
    private static readonly Regex OpenQuestionsRegex = new("^\\s*#+\\s+Open Questions\\b", RegexOptions.Compiled | RegexOptions.IgnoreCase);
    private static readonly Regex IdRegex = new("^\\s*-\\s*\\*\\*ID\\*\\*\\s+", RegexOptions.Compiled);
    private static readonly Regex ContextRegex = new("^\\s*-\\s*\\*\\*Context Bundle\\*\\*", RegexOptions.Compiled);
    private static readonly Regex DodRegex = new("^\\s*-\\s*\\*\\*DoD\\*\\*", RegexOptions.Compiled);
    private static readonly Regex ChecklistRegex = new("^\\s*-\\s*\\*\\*Checklist\\*\\*", RegexOptions.Compiled);
    private static readonly Regex DependenciesRegex = new("^\\s*-\\s*\\*\\*Dependencies\\*\\*", RegexOptions.Compiled);
    private static readonly Regex BacktickRegex = new("`([^`]+)`", RegexOptions.Compiled);

    public static bool Validate(string taskFilePath, string projectDir, Action<string> emitError)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            emitError($"PRD validation error: Task file does not exist: {taskFilePath}");
            return false;
        }

        if (string.IsNullOrWhiteSpace(projectDir))
        {
            emitError("PRD validation error: project directory is required");
            return false;
        }

        var hasErrors = false;

        foreach (var line in File.ReadLines(taskFilePath))
        {
            if (OpenQuestionsRegex.IsMatch(line))
            {
                emitError($"PRD validation error: {taskFilePath}: Open Questions section is not allowed");
                hasErrors = true;
                break;
            }
        }

        if (HasStrayUnchecked(taskFilePath))
        {
            emitError($"PRD validation error: {taskFilePath}: unchecked task outside a task block");
            hasErrors = true;
        }

        var blocks = TaskBlockParser.GetTaskBlocks(taskFilePath);
        foreach (var block in blocks)
        {
            if (!ValidateBlock(block, taskFilePath, projectDir, emitError))
            {
                hasErrors = true;
            }
        }

        return !hasErrors;
    }

    private static bool HasStrayUnchecked(string taskFilePath)
    {
        var inBlock = false;
        foreach (var line in File.ReadLines(taskFilePath))
        {
            if (TaskHeaderRegex.IsMatch(line))
            {
                inBlock = true;
                continue;
            }

            if (inBlock && BlockTerminatorRegex.IsMatch(line))
            {
                inBlock = false;
                continue;
            }

            if (!inBlock && UncheckedRegex.IsMatch(line))
            {
                return true;
            }
        }

        return false;
    }

    private static bool ValidateBlock(string block, string taskFilePath, string projectDir, Action<string> emitError)
    {
        var lines = block.Split('\n');
        var taskLabel = ExtractTaskLabel(lines.FirstOrDefault() ?? "(unknown task)");
        var uncheckedCount = lines.Count(line => UncheckedRegex.IsMatch(line));

        var hasId = lines.Any(line => IdRegex.IsMatch(line));
        var hasContext = lines.Any(line => ContextRegex.IsMatch(line));
        var hasDod = lines.Any(line => DodRegex.IsMatch(line));
        var hasChecklist = lines.Any(line => ChecklistRegex.IsMatch(line));
        var hasDependencies = lines.Any(line => DependenciesRegex.IsMatch(line));

        var valid = true;

        if (!hasId)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: missing ID field");
            valid = false;
        }

        if (!hasContext)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: missing Context Bundle field");
            valid = false;
        }

        if (!hasDod)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: missing DoD field");
            valid = false;
        }

        if (!hasChecklist)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: missing Checklist field");
            valid = false;
        }

        if (!hasDependencies)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: missing Dependencies field");
            valid = false;
        }

        if (uncheckedCount != 1)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: expected exactly one unchecked task line, found {uncheckedCount}");
            valid = false;
        }

        if (hasContext)
        {
            var contextLine = lines.FirstOrDefault(line => ContextRegex.IsMatch(line));
            if (!string.IsNullOrWhiteSpace(contextLine))
            {
                foreach (Match match in BacktickRegex.Matches(contextLine))
                {
                    var entry = match.Groups[1].Value.Trim();
                    if (string.IsNullOrWhiteSpace(entry))
                    {
                        continue;
                    }

                    if (!ContextEntryExists(entry, projectDir))
                    {
                        emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Context Bundle path not found: {entry}");
                        valid = false;
                    }
                }
            }
        }

        return valid;
    }

    private static string ExtractTaskLabel(string headerLine)
    {
        if (string.IsNullOrWhiteSpace(headerLine))
        {
            return "(unknown task)";
        }

        return headerLine.Trim();
    }

    private static bool ContextEntryExists(string entry, string projectDir)
    {
        if (Path.IsPathRooted(entry))
        {
            var full = Path.GetFullPath(entry);
            var projectFull = Path.GetFullPath(projectDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
            if (!full.StartsWith(projectFull + Path.DirectorySeparatorChar, StringComparison.Ordinal))
            {
                return false;
            }

            return File.Exists(full) || Directory.Exists(full);
        }

        var combined = Path.Combine(projectDir, entry);
        return File.Exists(combined) || Directory.Exists(combined);
    }
}
