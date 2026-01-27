using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;

namespace Gralph.Prd;

public static class PrdParser
{
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+(.+)$", RegexOptions.Compiled);
    private static readonly Regex BlockEndRegex = new("^\\s*(---\\s*$|##\\s+)", RegexOptions.Compiled);
    private static readonly Regex FieldRegex = new("^\\s*-\\s*\\*\\*(?<name>[^*]+)\\*\\*(?<rest>.*)$", RegexOptions.Compiled);
    private static readonly Regex UncheckedRegex = new("^\\s*-\\s*\\[\\s\\]", RegexOptions.Compiled);
    private static readonly Regex ContextHeaderRegex = new("^\\s*-\\s*\\*\\*Context Bundle\\*\\*", RegexOptions.Compiled);
    private static readonly Regex BacktickRegex = new("`([^`]*)`", RegexOptions.Compiled);

    public static IReadOnlyList<PrdTaskBlock> GetTaskBlocks(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return Array.Empty<PrdTaskBlock>();
        }

        var blocks = new List<PrdTaskBlock>();
        var currentLines = new List<string>();
        var inBlock = false;
        var startLine = 0;
        var lineNumber = 0;

        foreach (var line in File.ReadLines(taskFilePath))
        {
            lineNumber++;
            if (TaskHeaderRegex.IsMatch(line))
            {
                if (inBlock && currentLines.Count > 0)
                {
                    blocks.Add(BuildBlock(currentLines, startLine, lineNumber - 1));
                }

                inBlock = true;
                startLine = lineNumber;
                currentLines = new List<string> { line };
                continue;
            }

            if (inBlock && BlockEndRegex.IsMatch(line))
            {
                if (currentLines.Count > 0)
                {
                    blocks.Add(BuildBlock(currentLines, startLine, lineNumber - 1));
                }

                inBlock = false;
                currentLines = new List<string>();
                continue;
            }

            if (inBlock)
            {
                currentLines.Add(line);
            }
        }

        if (inBlock && currentLines.Count > 0)
        {
            blocks.Add(BuildBlock(currentLines, startLine, lineNumber));
        }

        return blocks;
    }

    private static PrdTaskBlock BuildBlock(IReadOnlyList<string> lines, int startLine, int endLine)
    {
        var headerLine = lines.Count > 0 ? lines[0] : string.Empty;
        var headerId = ExtractHeaderId(headerLine);
        var (fields, duplicateFields) = ExtractFields(lines);
        fields.TryGetValue("ID", out var idField);
        var contextEntries = ExtractContextEntries(lines);
        var uncheckedCount = lines.Count(line => UncheckedRegex.IsMatch(line));
        var rawText = string.Join("\n", lines);

        return new PrdTaskBlock(
            headerLine,
            rawText,
            startLine,
            endLine,
            headerId,
            idField,
            fields,
            contextEntries,
            uncheckedCount,
            duplicateFields);
    }

    private static string? ExtractHeaderId(string headerLine)
    {
        var match = TaskHeaderRegex.Match(headerLine);
        if (!match.Success)
        {
            return null;
        }

        return match.Groups[1].Value.Trim();
    }

    private static (Dictionary<string, string?> Fields, List<string> Duplicates) ExtractFields(IReadOnlyList<string> lines)
    {
        var fields = new Dictionary<string, string?>(StringComparer.Ordinal);
        var duplicates = new List<string>();
        foreach (var line in lines)
        {
            var match = FieldRegex.Match(line);
            if (!match.Success)
            {
                continue;
            }

            var name = match.Groups["name"].Value.Trim();
            if (string.IsNullOrEmpty(name))
            {
                continue;
            }

            if (fields.ContainsKey(name))
            {
                duplicates.Add(name);
                continue;
            }

            var rest = match.Groups["rest"].Value.Trim();
            fields[name] = string.IsNullOrEmpty(rest) ? null : rest;
        }

        return (fields, duplicates);
    }

    private static List<string> ExtractContextEntries(IReadOnlyList<string> lines)
    {
        var entries = new List<string>();
        var inContext = false;
        var contextIndent = 0;

        foreach (var line in lines)
        {
            if (!inContext && ContextHeaderRegex.IsMatch(line))
            {
                inContext = true;
                contextIndent = GetIndentation(line);
                AddBacktickEntries(line, entries);
                continue;
            }

            if (inContext)
            {
                if (FieldRegex.IsMatch(line))
                {
                    var indent = GetIndentation(line);
                    if (indent <= contextIndent)
                    {
                        break;
                    }
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

    private static int GetIndentation(string line)
    {
        if (string.IsNullOrEmpty(line))
        {
            return 0;
        }

        var count = 0;
        foreach (var ch in line)
        {
            if (ch == ' ' || ch == '\t')
            {
                count++;
                continue;
            }

            break;
        }

        return count;
    }
}
