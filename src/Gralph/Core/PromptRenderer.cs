using System.Globalization;

namespace Gralph.Core;

public static class PromptRenderer
{
    public static string Render(
        string template,
        string taskFile,
        string completionMarker,
        int iteration,
        int maxIterations,
        string? taskBlock,
        string? contextFiles)
    {
        var normalizedTaskBlock = string.IsNullOrWhiteSpace(taskBlock)
            ? "No task block available."
            : taskBlock;

        var normalizedContextFiles = string.IsNullOrWhiteSpace(contextFiles)
            ? string.Empty
            : contextFiles;

        var contextSection = string.Empty;
        if (!string.IsNullOrWhiteSpace(normalizedContextFiles))
        {
            contextSection =
                "Context Files (read these first):" + Environment.NewLine +
                normalizedContextFiles + Environment.NewLine;
        }

        return template
            .Replace("{task_file}", taskFile, StringComparison.Ordinal)
            .Replace("{completion_marker}", completionMarker, StringComparison.Ordinal)
            .Replace("{iteration}", iteration.ToString(CultureInfo.InvariantCulture), StringComparison.Ordinal)
            .Replace("{max_iterations}", maxIterations.ToString(CultureInfo.InvariantCulture), StringComparison.Ordinal)
            .Replace("{task_block}", normalizedTaskBlock, StringComparison.Ordinal)
            .Replace("{context_files}", normalizedContextFiles, StringComparison.Ordinal)
            .Replace("{context_files_section}", contextSection, StringComparison.Ordinal);
    }

    public static string NormalizeContextFiles(string? raw)
    {
        if (string.IsNullOrWhiteSpace(raw))
        {
            return string.Empty;
        }

        var parts = raw.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        return string.Join(Environment.NewLine, parts.Where(part => !string.IsNullOrWhiteSpace(part)));
    }
}
