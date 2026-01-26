using System.Text.RegularExpressions;
using System.Linq;
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
    private static readonly Regex FieldRegex = new("^\\s*-\\s*\\*\\*[^*]+\\*\\*", RegexOptions.Compiled);
    private static readonly Regex BacktickRegex = new("`([^`]+)`", RegexOptions.Compiled);

    public static bool Validate(string taskFilePath, string projectDir, Action<string> emitError, bool allowMissingContext = false)
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

        if (HasStrayUnchecked(taskFilePath, emitError))
        {
            hasErrors = true;
        }

        var blocks = TaskBlockParser.GetTaskBlocks(taskFilePath);
        foreach (var block in blocks)
        {
            if (!ValidateBlock(block, taskFilePath, projectDir, emitError, allowMissingContext))
            {
                hasErrors = true;
            }
        }

        return !hasErrors;
    }

    private static bool HasStrayUnchecked(string taskFilePath, Action<string> emitError)
    {
        var inBlock = false;
        var lineNumber = 0;
        var errors = 0;
        foreach (var line in File.ReadLines(taskFilePath))
        {
            lineNumber++;
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
                emitError($"PRD validation error: {taskFilePath}: line {lineNumber}: Unchecked task line outside task block");
                errors++;
            }
        }

        return errors > 0;
    }

    private static bool ValidateBlock(string block, string taskFilePath, string projectDir, Action<string> emitError, bool allowMissingContext)
    {
        var lines = block.Split('\n');
        var taskLabel = ExtractTaskLabel(lines);
        var uncheckedCount = lines.Count(line => UncheckedRegex.IsMatch(line));

        var hasId = lines.Any(line => IdRegex.IsMatch(line));
        var hasContext = lines.Any(line => ContextRegex.IsMatch(line));
        var hasDod = lines.Any(line => DodRegex.IsMatch(line));
        var hasChecklist = lines.Any(line => ChecklistRegex.IsMatch(line));
        var hasDependencies = lines.Any(line => DependenciesRegex.IsMatch(line));

        var valid = true;

        if (!hasId)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Missing required field: ID");
            valid = false;
        }

        if (!hasContext)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Missing required field: Context Bundle");
            valid = false;
        }

        if (!hasDod)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Missing required field: DoD");
            valid = false;
        }

        if (!hasChecklist)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Missing required field: Checklist");
            valid = false;
        }

        if (!hasDependencies)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Missing required field: Dependencies");
            valid = false;
        }

        if (uncheckedCount == 0)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Missing unchecked task line");
            valid = false;
        }
        else if (uncheckedCount > 1)
        {
            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Multiple unchecked task lines ({uncheckedCount})");
            valid = false;
        }

        if (hasContext && !allowMissingContext)
        {
            var contextEntries = ExtractContextEntries(lines);
            if (contextEntries.Count == 0)
            {
                emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Context Bundle must include at least one file path");
                valid = false;
            }
            else
            {
                foreach (var entry in contextEntries)
                {
                    if (!ContextEntryExists(entry, projectDir, out var outsideRepo))
                    {
                        if (outsideRepo)
                        {
                            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Context Bundle path outside repo: {entry}");
                        }
                        else
                        {
                            emitError($"PRD validation error: {taskFilePath}: {taskLabel}: Context Bundle path not found: {entry}");
                        }
                        valid = false;
                    }
                }
            }
        }

        return valid;
    }

    private static string ExtractTaskLabel(IReadOnlyList<string> lines)
    {
        var idField = lines.FirstOrDefault(line => IdRegex.IsMatch(line));
        if (!string.IsNullOrWhiteSpace(idField))
        {
            var trimmed = IdRegex.Replace(idField, string.Empty).Trim();
            if (!string.IsNullOrWhiteSpace(trimmed))
            {
                return trimmed;
            }
        }

        var header = lines.FirstOrDefault(line => TaskHeaderRegex.IsMatch(line));
        if (!string.IsNullOrWhiteSpace(header))
        {
            return TaskHeaderRegex.Replace(header, string.Empty).Trim();
        }

        return "unknown";
    }

    private static List<string> ExtractContextEntries(IReadOnlyList<string> lines)
    {
        var entries = new List<string>();
        var inContext = false;

        foreach (var line in lines)
        {
            if (ContextRegex.IsMatch(line))
            {
                inContext = true;
            }
            else if (inContext && FieldRegex.IsMatch(line))
            {
                break;
            }

            if (!inContext)
            {
                continue;
            }

            foreach (Match match in BacktickRegex.Matches(line))
            {
                var entry = match.Groups[1].Value.Trim();
                if (!string.IsNullOrWhiteSpace(entry))
                {
                    entries.Add(entry);
                }
            }
        }

        return entries;
    }

    private static bool ContextEntryExists(string entry, string projectDir, out bool outsideRepo)
    {
        outsideRepo = false;
        if (Path.IsPathRooted(entry))
        {
            var full = Path.GetFullPath(entry);
            var projectFull = Path.GetFullPath(projectDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
            if (!full.StartsWith(projectFull + Path.DirectorySeparatorChar, StringComparison.Ordinal))
            {
                outsideRepo = true;
                return false;
            }

            return File.Exists(full) || Directory.Exists(full);
        }

        var combined = Path.Combine(projectDir, entry);
        return File.Exists(combined) || Directory.Exists(combined);
    }
}
