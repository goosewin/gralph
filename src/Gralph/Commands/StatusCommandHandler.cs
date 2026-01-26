using Gralph.Core;
using Gralph.State;
using Spectre.Console;

namespace Gralph.Commands;

public sealed class StatusCommandHandler
{
    private readonly StateStore _stateStore;

    public StatusCommandHandler(StateStore stateStore)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
    }

    public int Execute()
    {
        _stateStore.CleanupStale();
        var sessions = _stateStore.ListSessions()
            .OrderBy(session => session.Name ?? string.Empty, StringComparer.OrdinalIgnoreCase)
            .ToList();

        if (sessions.Count == 0)
        {
            AnsiConsole.MarkupLine("[grey]No sessions found[/]");
            AnsiConsole.WriteLine();
            AnsiConsole.MarkupLine("Start a new loop with: [bold]gralph start <directory>[/]");
            return 0;
        }

        var table = new Table();
        table.Border(TableBorder.Rounded);
        table.AddColumn("NAME");
        table.AddColumn("DIR");
        table.AddColumn("ITERATION");
        table.AddColumn("STATUS");
        table.AddColumn("REMAINING");

        foreach (var session in sessions)
        {
            var name = session.Name ?? "unknown";
            var dir = session.Dir ?? string.Empty;
            var displayDir = TruncatePath(dir, 40);
            var iteration = session.Iteration ?? 0;
            var maxIterations = session.MaxIterations ?? 0;
            var status = session.Status ?? "unknown";
            var remaining = ResolveRemainingTasks(session);

            table.AddRow(
                Markup.Escape(name),
                Markup.Escape(displayDir),
                Markup.Escape($"{iteration}/{maxIterations}"),
                FormatStatus(status),
                Markup.Escape(FormatRemaining(remaining)));
        }

        AnsiConsole.Write(table);
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("Commands: [bold]gralph logs <name>[/], [bold]gralph stop <name>[/], [bold]gralph resume[/]");
        return 0;
    }

    private static int? ResolveRemainingTasks(SessionState session)
    {
        if (string.IsNullOrWhiteSpace(session.Dir))
        {
            return session.LastTaskCount;
        }

        var taskFile = string.IsNullOrWhiteSpace(session.TaskFile) ? "PRD.md" : session.TaskFile;
        var taskFilePath = Path.Combine(session.Dir, taskFile);
        if (File.Exists(taskFilePath))
        {
            return TaskBlockParser.CountRemainingTasks(taskFilePath);
        }

        return session.LastTaskCount;
    }

    private static string FormatRemaining(int? remaining)
    {
        if (!remaining.HasValue)
        {
            return "?";
        }

        return remaining.Value switch
        {
            0 => "0 tasks",
            1 => "1 task",
            _ => $"{remaining.Value} tasks"
        };
    }

    private static string FormatStatus(string status)
    {
        var normalized = status.Trim().ToLowerInvariant();
        return normalized switch
        {
            "complete" or "completed" => $"[green]{Markup.Escape(status)}[/]",
            "running" => $"[yellow]{Markup.Escape(status)}[/]",
            "failed" or "stale" or "stopped" => $"[red]{Markup.Escape(status)}[/]",
            _ => Markup.Escape(status)
        };
    }

    private static string TruncatePath(string path, int maxLength)
    {
        if (string.IsNullOrWhiteSpace(path) || path.Length <= maxLength)
        {
            return path;
        }

        if (maxLength <= 3)
        {
            return path[..maxLength];
        }

        return "..." + path[^Math.Min(path.Length, maxLength - 3)..];
    }
}
