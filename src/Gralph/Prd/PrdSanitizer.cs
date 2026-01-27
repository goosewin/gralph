using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;

namespace Gralph.Prd;

public static class PrdSanitizer
{
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+", RegexOptions.Compiled);
    private static readonly Regex BlockEndRegex = new("^\\s*(---\\s*$|##\\s+)", RegexOptions.Compiled);
    private static readonly Regex OpenQuestionsRegex = new("^\\s*##\\s+Open Questions\\b", RegexOptions.Compiled | RegexOptions.IgnoreCase);
    private static readonly Regex HeadingRegex = new("^\\s*#", RegexOptions.Compiled);
    private static readonly Regex UncheckedRegex = new("^(\\s*)-\\s*\\[\\s\\]\\s*(.*)$", RegexOptions.Compiled);
    private static readonly Regex ContextHeaderRegex = new("^(\\s*)-\\s*\\*\\*Context Bundle\\*\\*", RegexOptions.Compiled);
    private static readonly Regex FieldRegex = new("^\\s*-\\s*\\*\\*[^*]+\\*\\*", RegexOptions.Compiled);
    private static readonly Regex BacktickRegex = new("`([^`]*)`", RegexOptions.Compiled);

    public static void SanitizeFile(string filePath, string baseDir, IReadOnlyCollection<string> allowedContext)
    {
        if (string.IsNullOrWhiteSpace(filePath) || !File.Exists(filePath))
        {
            return;
        }

        var lines = File.ReadAllLines(filePath);
        var output = new List<string>();
        var blockLines = new List<string>();
        var inBlock = false;
        var inOpenQuestions = false;
        var started = false;

        foreach (var line in lines)
        {
            if (!started)
            {
                if (!HeadingRegex.IsMatch(line))
                {
                    continue;
                }
                started = true;
            }

            if (OpenQuestionsRegex.IsMatch(line))
            {
                inOpenQuestions = true;
                continue;
            }

            if (inOpenQuestions)
            {
                if (line.TrimStart().StartsWith("##", StringComparison.Ordinal))
                {
                    inOpenQuestions = false;
                }
                else
                {
                    continue;
                }
            }

            if (TaskHeaderRegex.IsMatch(line))
            {
                if (inBlock && blockLines.Count > 0)
                {
                    output.AddRange(SanitizeTaskBlock(blockLines, baseDir, allowedContext));
                    blockLines.Clear();
                }
                inBlock = true;
                blockLines.Add(line);
                continue;
            }

            if (inBlock && BlockEndRegex.IsMatch(line))
            {
                if (blockLines.Count > 0)
                {
                    output.AddRange(SanitizeTaskBlock(blockLines, baseDir, allowedContext));
                    blockLines.Clear();
                }
                inBlock = false;
            }

            if (inBlock)
            {
                blockLines.Add(line);
                continue;
            }

            output.Add(SanitizeUncheckedLine(line));
        }

        if (inBlock && blockLines.Count > 0)
        {
            output.AddRange(SanitizeTaskBlock(blockLines, baseDir, allowedContext));
        }

        File.WriteAllText(filePath, string.Join("\n", output));
    }

    private static IEnumerable<string> SanitizeTaskBlock(
        IReadOnlyList<string> blockLines,
        string baseDir,
        IReadOnlyCollection<string> allowedContext)
    {
        var output = new List<string>();
        var validEntries = ResolveContextEntries(blockLines, baseDir, allowedContext);
        var contextLine = BuildContextLine(validEntries);

        var inContext = false;
        var uncheckedSeen = false;

        foreach (var line in blockLines)
        {
            var match = ContextHeaderRegex.Match(line);
            if (match.Success)
            {
                var indent = match.Groups[1].Value;
                output.Add($"{indent}{contextLine}");
                inContext = true;
                continue;
            }

            if (inContext)
            {
                if (FieldRegex.IsMatch(line))
                {
                    inContext = false;
                }
                else
                {
                    continue;
                }
            }

            var uncheckedMatch = UncheckedRegex.Match(line);
            if (uncheckedMatch.Success)
            {
                if (uncheckedSeen)
                {
                    output.Add($"{uncheckedMatch.Groups[1].Value}- {uncheckedMatch.Groups[2].Value}");
                }
                else
                {
                    uncheckedSeen = true;
                    output.Add(line);
                }
                continue;
            }

            output.Add(line);
        }

        return output;
    }

    private static IReadOnlyList<string> ResolveContextEntries(
        IReadOnlyList<string> blockLines,
        string baseDir,
        IReadOnlyCollection<string> allowedContext)
    {
        var entries = ExtractContextEntries(blockLines);
        var validEntries = new List<string>();
        var allowedSet = BuildAllowedSet(allowedContext, baseDir);

        foreach (var entry in entries)
        {
            var trimmed = entry.Trim();
            if (string.IsNullOrWhiteSpace(trimmed))
            {
                continue;
            }

            if (!TryResolvePath(trimmed, baseDir, out var displayPath, out var fullPath))
            {
                continue;
            }

            if (!File.Exists(fullPath) && !Directory.Exists(fullPath))
            {
                continue;
            }

            if (allowedSet.Count > 0 && !allowedSet.Contains(NormalizePath(displayPath)))
            {
                continue;
            }

            if (!validEntries.Contains(displayPath, StringComparer.Ordinal))
            {
                validEntries.Add(displayPath);
            }
        }

        if (validEntries.Count == 0)
        {
            var fallback = PickFallbackContext(baseDir, allowedContext);
            if (!string.IsNullOrWhiteSpace(fallback))
            {
                validEntries.Add(fallback);
            }
        }

        return validEntries;
    }

    private static IReadOnlyList<string> ExtractContextEntries(IReadOnlyList<string> blockLines)
    {
        var entries = new List<string>();
        var inContext = false;
        foreach (var line in blockLines)
        {
            if (!inContext && ContextHeaderRegex.IsMatch(line))
            {
                inContext = true;
                AddBacktickEntries(line, entries);
                continue;
            }

            if (inContext)
            {
                if (FieldRegex.IsMatch(line))
                {
                    break;
                }

                AddBacktickEntries(line, entries);
            }
        }

        return entries;
    }

    private static void AddBacktickEntries(string line, ICollection<string> entries)
    {
        foreach (Match match in BacktickRegex.Matches(line))
        {
            if (match.Success)
            {
                entries.Add(match.Groups[1].Value);
            }
        }
    }

    private static string BuildContextLine(IReadOnlyList<string> entries)
    {
        if (entries.Count == 0)
        {
            return "- **Context Bundle**";
        }

        var formatted = string.Join(", ", entries.Select(entry => $"`{entry}`"));
        return $"- **Context Bundle** {formatted}";
    }

    private static string SanitizeUncheckedLine(string line)
    {
        var match = UncheckedRegex.Match(line);
        if (!match.Success)
        {
            return line;
        }

        return $"{match.Groups[1].Value}- {match.Groups[2].Value}";
    }

    private static HashSet<string> BuildAllowedSet(IReadOnlyCollection<string> allowedContext, string baseDir)
    {
        var comparer = OperatingSystem.IsWindows()
            ? StringComparer.OrdinalIgnoreCase
            : StringComparer.Ordinal;
        var allowed = new HashSet<string>(comparer);
        foreach (var entry in allowedContext)
        {
            if (string.IsNullOrWhiteSpace(entry))
            {
                continue;
            }

            if (TryResolvePath(entry, baseDir, out var displayPath, out _))
            {
                allowed.Add(NormalizePath(displayPath));
            }
        }

        return allowed;
    }

    private static string PickFallbackContext(string baseDir, IReadOnlyCollection<string> allowedContext)
    {
        foreach (var entry in allowedContext)
        {
            if (string.IsNullOrWhiteSpace(entry))
            {
                continue;
            }

            if (!TryResolvePath(entry, baseDir, out var displayPath, out var fullPath))
            {
                continue;
            }

            if (File.Exists(fullPath) || Directory.Exists(fullPath))
            {
                return displayPath;
            }
        }

        var readme = Path.Combine(baseDir, "README.md");
        if (File.Exists(readme))
        {
            return "README.md";
        }

        return string.Empty;
    }

    private static bool TryResolvePath(string entry, string baseDir, out string displayPath, out string fullPath)
    {
        displayPath = string.Empty;
        fullPath = string.Empty;

        if (string.IsNullOrWhiteSpace(entry))
        {
            return false;
        }

        if (string.IsNullOrWhiteSpace(baseDir))
        {
            return false;
        }

        var baseFullPath = Path.GetFullPath(baseDir);

        if (Path.IsPathRooted(entry))
        {
            fullPath = Path.GetFullPath(entry);
        }
        else
        {
            fullPath = Path.GetFullPath(Path.Combine(baseFullPath, entry));
        }

        if (!IsSubPath(baseFullPath, fullPath))
        {
            return false;
        }

        displayPath = NormalizePath(Path.GetRelativePath(baseFullPath, fullPath));
        return true;
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

        var normalizedBase = Path.GetFullPath(baseDir)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var normalizedPath = Path.GetFullPath(path);
        if (!normalizedPath.StartsWith(normalizedBase, comparison))
        {
            return false;
        }

        if (normalizedPath.Length == normalizedBase.Length)
        {
            return true;
        }

        var next = normalizedPath[normalizedBase.Length];
        return next == Path.DirectorySeparatorChar || next == Path.AltDirectorySeparatorChar;
    }

    private static string NormalizePath(string path)
    {
        return path.Replace('\u005c', '/');
    }
}
