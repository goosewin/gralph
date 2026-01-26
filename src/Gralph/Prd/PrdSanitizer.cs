using System.Text.RegularExpressions;
using System.Linq;

namespace Gralph.Prd;

public static class PrdSanitizer
{
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+", RegexOptions.Compiled);
    private static readonly Regex BlockTerminatorRegex = new("^\\s*(---\\s*$|##\\s+)", RegexOptions.Compiled);
    private static readonly Regex UncheckedRegex = new("^(\\s*)-\\s*\\[\\s\\]\\s*(.*)$", RegexOptions.Compiled);
    private static readonly Regex OpenQuestionsRegex = new("^\\s*##\\s+Open Questions\\b", RegexOptions.Compiled | RegexOptions.IgnoreCase);
    private static readonly Regex HeaderRegex = new("^\\s*#+\\s+", RegexOptions.Compiled);
    private static readonly Regex ContextRegex = new("^\\s*-\\s*\\*\\*Context Bundle\\*\\*", RegexOptions.Compiled);
    private static readonly Regex FieldRegex = new("^\\s*-\\s*\\*\\*[^*]+\\*\\*", RegexOptions.Compiled);
    private static readonly Regex BacktickRegex = new("`([^`]+)`", RegexOptions.Compiled);

    public static void SanitizeGeneratedFile(string filePath, string baseDir, IReadOnlyCollection<string>? allowedContext)
    {
        if (string.IsNullOrWhiteSpace(filePath) || !File.Exists(filePath))
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(baseDir))
        {
            baseDir = Path.GetDirectoryName(filePath) ?? string.Empty;
        }

        var normalizedAllowed = new HashSet<string>(StringComparer.Ordinal);
        if (allowedContext is not null)
        {
            foreach (var entry in allowedContext)
            {
                if (!string.IsNullOrWhiteSpace(entry))
                {
                    normalizedAllowed.Add(entry.Trim());
                }
            }
        }

        var output = new List<string>();
        var blockLines = new List<string>();
        var inBlock = false;
        var started = false;
        var inOpenQuestions = false;

        foreach (var line in File.ReadLines(filePath))
        {
            if (!started)
            {
                if (HeaderRegex.IsMatch(line))
                {
                    started = true;
                }
                else
                {
                    continue;
                }
            }

            if (OpenQuestionsRegex.IsMatch(line))
            {
                inOpenQuestions = true;
                continue;
            }

            if (inOpenQuestions)
            {
                if (HeaderRegex.IsMatch(line))
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
                if (inBlock)
                {
                    output.AddRange(SanitizeBlock(blockLines, baseDir, normalizedAllowed));
                    blockLines.Clear();
                }

                inBlock = true;
                blockLines.Add(line);
                continue;
            }

            if (inBlock && BlockTerminatorRegex.IsMatch(line))
            {
                output.AddRange(SanitizeBlock(blockLines, baseDir, normalizedAllowed));
                blockLines.Clear();
                inBlock = false;
            }

            if (inBlock)
            {
                blockLines.Add(line);
            }
            else
            {
                output.Add(SanitizeUncheckedOutsideBlock(line));
            }
        }

        if (inBlock)
        {
            output.AddRange(SanitizeBlock(blockLines, baseDir, normalizedAllowed));
        }

        File.WriteAllLines(filePath, output);
    }

    private static string SanitizeUncheckedOutsideBlock(string line)
    {
        var match = UncheckedRegex.Match(line);
        if (!match.Success)
        {
            return line;
        }

        var indent = match.Groups[1].Value;
        var rest = match.Groups[2].Value;
        return string.IsNullOrWhiteSpace(rest) ? $"{indent}-" : $"{indent}- {rest}";
    }

    private static IEnumerable<string> SanitizeBlock(IReadOnlyList<string> lines, string baseDir, HashSet<string> allowedContext)
    {
        var sanitized = new List<string>();
        var contextEntries = ExtractContextEntries(lines);
        var validEntries = FilterContextEntries(contextEntries, baseDir, allowedContext);
        var contextLine = BuildContextLine(validEntries, baseDir, allowedContext);

        var inContextBlock = false;
        var uncheckedSeen = false;

        foreach (var line in lines)
        {
            if (ContextRegex.IsMatch(line))
            {
                var indentMatch = Regex.Match(line, "^(\\s*)-");
                var indent = indentMatch.Success ? indentMatch.Groups[1].Value : string.Empty;
                sanitized.Add($"{indent}{contextLine}");
                inContextBlock = true;
                continue;
            }

            if (inContextBlock)
            {
                if (FieldRegex.IsMatch(line))
                {
                    inContextBlock = false;
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
                    var indent = uncheckedMatch.Groups[1].Value;
                    var rest = uncheckedMatch.Groups[2].Value;
                    sanitized.Add(string.IsNullOrWhiteSpace(rest) ? $"{indent}-" : $"{indent}- {rest}");
                    continue;
                }

                uncheckedSeen = true;
            }

            sanitized.Add(line);
        }

        return sanitized;
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

    private static List<string> FilterContextEntries(IEnumerable<string> entries, string baseDir, HashSet<string> allowedContext)
    {
        var valid = new List<string>();
        var seen = new HashSet<string>(StringComparer.Ordinal);

        foreach (var entry in entries)
        {
            var display = NormalizeDisplayPath(entry, baseDir);
            if (string.IsNullOrWhiteSpace(display))
            {
                continue;
            }

            if (!ContextEntryExists(display, baseDir))
            {
                continue;
            }

            if (allowedContext.Count > 0 && !allowedContext.Contains(display))
            {
                continue;
            }

            if (seen.Add(display))
            {
                valid.Add(display);
            }
        }

        return valid;
    }

    private static string BuildContextLine(List<string> validEntries, string baseDir, HashSet<string> allowedContext)
    {
        if (validEntries.Count == 0)
        {
            var fallback = PickFallbackContext(baseDir, allowedContext);
            if (!string.IsNullOrWhiteSpace(fallback))
            {
                validEntries.Add(fallback);
            }
        }

        if (validEntries.Count == 0)
        {
            return "- **Context Bundle**";
        }

        var formatted = string.Join(", ", validEntries.Select(entry => $"`{entry}`"));
        return $"- **Context Bundle** {formatted}";
    }

    private static string PickFallbackContext(string baseDir, HashSet<string> allowedContext)
    {
        if (allowedContext.Count > 0)
        {
            foreach (var entry in allowedContext)
            {
                if (ContextEntryExists(entry, baseDir))
                {
                    return entry;
                }
            }
        }

        var readme = Path.Combine(baseDir, "README.md");
        if (File.Exists(readme))
        {
            return "README.md";
        }

        return string.Empty;
    }

    private static string NormalizeDisplayPath(string entry, string baseDir)
    {
        if (string.IsNullOrWhiteSpace(entry))
        {
            return string.Empty;
        }

        if (Path.IsPathRooted(entry))
        {
            var full = Path.GetFullPath(entry);
            var baseFull = Path.GetFullPath(baseDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
            if (full.StartsWith(baseFull + Path.DirectorySeparatorChar, StringComparison.Ordinal))
            {
                return full.Substring(baseFull.Length + 1);
            }

            return string.Empty;
        }

        return entry;
    }

    private static bool ContextEntryExists(string entry, string baseDir)
    {
        if (string.IsNullOrWhiteSpace(entry))
        {
            return false;
        }

        var resolved = Path.IsPathRooted(entry)
            ? entry
            : Path.Combine(baseDir, entry);

        return File.Exists(resolved) || Directory.Exists(resolved);
    }
}
