using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;

namespace Gralph.Prd;

public static class PrdValidator
{
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+", RegexOptions.Compiled);
    private static readonly Regex BlockEndRegex = new("^\\s*(---\\s*$|##\\s+)", RegexOptions.Compiled);
    private static readonly Regex OpenQuestionsRegex = new("^\\s*#+\\s+Open Questions\\b", RegexOptions.Compiled);
    private static readonly Regex UncheckedRegex = new("^\\s*-\\s*\\[\\s\\]", RegexOptions.Compiled);
    private static readonly StringComparison PathComparison = OperatingSystem.IsWindows()
        ? StringComparison.OrdinalIgnoreCase
        : StringComparison.Ordinal;

    public static PrdValidationResult ValidateFile(string taskFilePath, bool allowMissingContext = false, string? baseDirOverride = null)
    {
        var result = new PrdValidationResult();

        if (string.IsNullOrWhiteSpace(taskFilePath))
        {
            result.Add(new PrdValidationError(taskFilePath, "Task file is required"));
            return result;
        }

        if (!File.Exists(taskFilePath))
        {
            result.Add(new PrdValidationError(taskFilePath, $"Task file does not exist: {taskFilePath}"));
            return result;
        }

        var baseDir = ResolveBaseDir(taskFilePath, baseDirOverride);

        var lines = File.ReadAllLines(taskFilePath);

        for (var i = 0; i < lines.Length; i++)
        {
            if (OpenQuestionsRegex.IsMatch(lines[i]))
            {
                result.Add(new PrdValidationError(taskFilePath, "Open Questions section is not allowed"));
                break;
            }
        }

        result.AddRange(ValidateStrayUnchecked(taskFilePath, lines));

        foreach (var block in PrdParser.GetTaskBlocks(taskFilePath))
        {
            ValidateTaskBlock(taskFilePath, baseDir, block, allowMissingContext, result);
        }

        return result;
    }

    private static IEnumerable<PrdValidationError> ValidateStrayUnchecked(string taskFilePath, IReadOnlyList<string> lines)
    {
        var errors = new List<PrdValidationError>();
        var inBlock = false;

        for (var i = 0; i < lines.Count; i++)
        {
            var line = lines[i];
            if (TaskHeaderRegex.IsMatch(line))
            {
                inBlock = true;
            }
            else if (inBlock && BlockEndRegex.IsMatch(line))
            {
                inBlock = false;
            }

            if (!inBlock && UncheckedRegex.IsMatch(line))
            {
                errors.Add(new PrdValidationError(taskFilePath, "Unchecked task line outside task block", lineNumber: i + 1));
            }
        }

        return errors;
    }

    private static void ValidateTaskBlock(
        string taskFilePath,
        string baseDir,
        PrdTaskBlock block,
        bool allowMissingContext,
        PrdValidationResult result)
    {
        var taskLabel = !string.IsNullOrWhiteSpace(block.IdField)
            ? block.IdField
            : block.HeaderId ?? "unknown";

        var requiredFields = new[] { "ID", "Context Bundle", "DoD", "Checklist", "Dependencies" };
        foreach (var field in requiredFields)
        {
            if (!block.Fields.ContainsKey(field))
            {
                result.Add(new PrdValidationError(taskFilePath, $"Missing required field: {field}", taskLabel));
            }
        }

        if (block.DuplicateFields.Count > 0)
        {
            foreach (var duplicate in block.DuplicateFields.Distinct(StringComparer.Ordinal))
            {
                result.Add(new PrdValidationError(taskFilePath, $"Duplicate field: {duplicate}", taskLabel));
            }
        }

        if (block.UncheckedCount == 0)
        {
            result.Add(new PrdValidationError(taskFilePath, "Missing unchecked task line", taskLabel));
        }
        else if (block.UncheckedCount > 1)
        {
            result.Add(new PrdValidationError(taskFilePath, $"Multiple unchecked task lines ({block.UncheckedCount})", taskLabel));
        }

        if (allowMissingContext)
        {
            return;
        }

        if (block.ContextEntries.Count == 0)
        {
            result.Add(new PrdValidationError(taskFilePath, "Context Bundle must include at least one file path", taskLabel));
            return;
        }

        foreach (var entry in block.ContextEntries)
        {
            if (string.IsNullOrWhiteSpace(entry))
            {
                continue;
            }

            var trimmed = entry.Trim();
            string resolved;
            if (Path.IsPathRooted(trimmed))
            {
                resolved = Path.GetFullPath(trimmed);
                if (!IsSubPath(baseDir, resolved))
                {
                    result.Add(new PrdValidationError(taskFilePath, $"Context Bundle path outside repo: {trimmed}", taskLabel));
                    continue;
                }
            }
            else
            {
                resolved = Path.GetFullPath(Path.Combine(baseDir, trimmed));
            }

            if (!File.Exists(resolved) && !Directory.Exists(resolved))
            {
                result.Add(new PrdValidationError(taskFilePath, $"Context Bundle path not found: {trimmed}", taskLabel));
            }
        }
    }

    private static string ResolveBaseDir(string taskFilePath, string? baseDirOverride)
    {
        if (!string.IsNullOrWhiteSpace(baseDirOverride))
        {
            try
            {
                return NormalizePath(Path.GetFullPath(baseDirOverride));
            }
            catch
            {
                return NormalizePath(baseDirOverride);
            }
        }

        var fullPath = Path.GetFullPath(taskFilePath);
        var directory = Path.GetDirectoryName(fullPath);
        if (string.IsNullOrWhiteSpace(directory))
        {
            return NormalizePath(fullPath);
        }

        return NormalizePath(directory);
    }

    private static string NormalizePath(string path)
    {
        return path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
    }

    private static bool IsSubPath(string baseDir, string path)
    {
        if (string.IsNullOrWhiteSpace(baseDir))
        {
            return true;
        }

        if (!path.StartsWith(baseDir, PathComparison))
        {
            return false;
        }

        if (path.Length == baseDir.Length)
        {
            return true;
        }

        var next = path[baseDir.Length];
        return next == Path.DirectorySeparatorChar || next == Path.AltDirectorySeparatorChar;
    }
}
