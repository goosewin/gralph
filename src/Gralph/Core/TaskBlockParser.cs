using System.Text;
using System.Text.RegularExpressions;

namespace Gralph.Core;

public static class TaskBlockParser
{
    private static readonly Regex TaskHeaderRegex = new("^\\s*###\\s+Task\\s+", RegexOptions.Compiled);
    private static readonly Regex BlockTerminatorRegex = new("^\\s*(---\\s*$|##\\s+)", RegexOptions.Compiled);
    private static readonly Regex UncheckedRegex = new("^\\s*-\\s*\\[\\s\\]", RegexOptions.Compiled);

    public static IReadOnlyList<string> GetTaskBlocks(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return Array.Empty<string>();
        }

        var blocks = new List<string>();
        var builder = new StringBuilder();
        var inBlock = false;

        foreach (var line in File.ReadLines(taskFilePath))
        {
            if (TaskHeaderRegex.IsMatch(line))
            {
                if (inBlock)
                {
                    blocks.Add(builder.ToString());
                    builder.Clear();
                }

                inBlock = true;
                builder.Append(line);
                continue;
            }

            if (inBlock && BlockTerminatorRegex.IsMatch(line))
            {
                blocks.Add(builder.ToString());
                builder.Clear();
                inBlock = false;
                continue;
            }

            if (inBlock)
            {
                builder.AppendLine();
                builder.Append(line);
            }
        }

        if (inBlock)
        {
            blocks.Add(builder.ToString());
        }

        return blocks;
    }

    public static string GetNextUncheckedTaskBlock(string taskFilePath)
    {
        foreach (var block in GetTaskBlocks(taskFilePath))
        {
            if (UncheckedRegex.IsMatch(block))
            {
                return block;
            }
        }

        return string.Empty;
    }

    public static int CountRemainingTasks(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return 0;
        }

        var lines = File.ReadLines(taskFilePath);
        if (lines.Any(line => TaskHeaderRegex.IsMatch(line)))
        {
            var total = 0;
            foreach (var block in GetTaskBlocks(taskFilePath))
            {
                total += UncheckedRegex.Matches(block).Count;
            }

            return total;
        }

        return lines.Count(line => UncheckedRegex.IsMatch(line));
    }

    public static string GetFirstUncheckedLine(string taskFilePath)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return string.Empty;
        }

        foreach (var line in File.ReadLines(taskFilePath))
        {
            if (UncheckedRegex.IsMatch(line))
            {
                return line;
            }
        }

        return string.Empty;
    }
}
