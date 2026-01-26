using Gralph.State;

namespace Gralph.Commands;

public sealed class LogsCommandHandler
{
    private const int DefaultTailLines = 100;
    private static readonly TimeSpan FollowPollInterval = TimeSpan.FromMilliseconds(250);
    private readonly StateStore _stateStore;

    public LogsCommandHandler(StateStore stateStore)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
    }

    public int Execute(LogsCommandSettings settings)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        if (string.IsNullOrWhiteSpace(settings.Name))
        {
            Console.Error.WriteLine("Error: Session name is required. Usage: gralph logs <name> [--follow]");
            return 1;
        }

        var sessionName = settings.Name.Trim();
        var session = _stateStore.GetSession(sessionName);
        if (session is null)
        {
            Console.Error.WriteLine($"Error: Session not found: {sessionName}");
            return 1;
        }

        var logFile = ResolveLogFile(sessionName, session);
        if (string.IsNullOrWhiteSpace(logFile))
        {
            Console.Error.WriteLine($"Error: Cannot determine log file path for session: {sessionName}");
            return 1;
        }

        if (!File.Exists(logFile))
        {
            Console.Error.WriteLine($"Error: Log file does not exist: {logFile}");
            return 1;
        }

        var status = session.Status ?? "unknown";
        Console.WriteLine($"Session: {sessionName} (status: {status})");
        Console.WriteLine($"Log file: {logFile}");
        Console.WriteLine();

        if (settings.Follow)
        {
            TailFollow(logFile, DefaultTailLines);
            return 0;
        }

        foreach (var line in ReadLastLines(logFile, DefaultTailLines))
        {
            Console.WriteLine(line);
        }

        return 0;
    }

    private static string? ResolveLogFile(string sessionName, SessionState session)
    {
        if (!string.IsNullOrWhiteSpace(session.LogFile))
        {
            return session.LogFile;
        }

        if (string.IsNullOrWhiteSpace(session.Dir))
        {
            return null;
        }

        return Path.Combine(session.Dir, ".gralph", $"{sessionName}.log");
    }

    private static IEnumerable<string> ReadLastLines(string filePath, int lineCount)
    {
        var buffer = new Queue<string>(lineCount);
        foreach (var line in File.ReadLines(filePath))
        {
            if (buffer.Count == lineCount)
            {
                buffer.Dequeue();
            }

            buffer.Enqueue(line);
        }

        return buffer;
    }

    private static void TailFollow(string filePath, int lineCount)
    {
        foreach (var line in ReadLastLines(filePath, lineCount))
        {
            Console.WriteLine(line);
        }

        var cancelled = false;
        Console.CancelKeyPress += (_, args) =>
        {
            args.Cancel = true;
            cancelled = true;
        };

        using var stream = new FileStream(filePath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
        using var reader = new StreamReader(stream);
        stream.Seek(0, SeekOrigin.End);

        while (!cancelled)
        {
            var line = reader.ReadLine();
            if (line is not null)
            {
                Console.WriteLine(line);
            }
            else
            {
                Thread.Sleep(FollowPollInterval);
            }
        }
    }
}

public sealed class LogsCommandSettings
{
    public string? Name { get; init; }
    public bool Follow { get; init; }
}
