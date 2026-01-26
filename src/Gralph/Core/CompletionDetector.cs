using System.Text.RegularExpressions;

namespace Gralph.Core;

public static class CompletionDetector
{
    private static readonly Regex NegationRegex =
        new("(cannot|can't|won't|will not|do not|don't|should not|shouldn't|must not|mustn't)[^<]*<promise>",
            RegexOptions.IgnoreCase | RegexOptions.Compiled);

    public static bool IsComplete(string taskFilePath, string result, string completionMarker)
    {
        if (string.IsNullOrWhiteSpace(taskFilePath) || !File.Exists(taskFilePath))
        {
            return false;
        }

        if (string.IsNullOrWhiteSpace(result))
        {
            return false;
        }

        var remaining = TaskBlockParser.CountRemainingTasks(taskFilePath);
        if (remaining > 0)
        {
            return false;
        }

        var promiseLine = GetLastNonEmptyLine(result);
        if (string.IsNullOrWhiteSpace(promiseLine))
        {
            return false;
        }

        var promisePattern = $"^\\s*<promise>{Regex.Escape(completionMarker)}</promise>\\s*$";
        if (!Regex.IsMatch(promiseLine, promisePattern))
        {
            return false;
        }

        if (NegationRegex.IsMatch(promiseLine))
        {
            return false;
        }

        return true;
    }

    private static string GetLastNonEmptyLine(string text)
    {
        string? last = null;
        using var reader = new StringReader(text);
        while (reader.ReadLine() is { } line)
        {
            if (!string.IsNullOrWhiteSpace(line))
            {
                last = line;
            }
        }

        return last ?? string.Empty;
    }
}
