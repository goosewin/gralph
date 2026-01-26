using System.Diagnostics;
using Gralph.State;

namespace Gralph.Commands;

public sealed class StopCommandHandler
{
    private readonly StateStore _stateStore;

    public StopCommandHandler(StateStore stateStore)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
    }

    public int Execute(StopCommandSettings settings)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        if (settings.All)
        {
            return StopAllSessions();
        }

        if (string.IsNullOrWhiteSpace(settings.Name))
        {
            Console.Error.WriteLine("Error: Session name is required. Usage: gralph stop <name> or gralph stop --all");
            return 1;
        }

        return StopSession(settings.Name.Trim());
    }

    private int StopAllSessions()
    {
        var sessions = _stateStore.ListSessions();
        var stoppedCount = 0;

        foreach (var session in sessions)
        {
            if (!string.Equals(session.Status, "running", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (!string.IsNullOrWhiteSpace(session.Name))
            {
                StopSession(session.Name, quiet: true);
                stoppedCount++;
            }
        }

        if (stoppedCount == 0)
        {
            Console.WriteLine("No running sessions to stop");
            return 0;
        }

        Console.WriteLine($"Stopped {stoppedCount} session(s)");
        return 0;
    }

    private int StopSession(string sessionName, bool quiet = false)
    {
        var session = _stateStore.GetSession(sessionName);
        if (session is null)
        {
            if (!quiet)
            {
                Console.Error.WriteLine($"Error: Session not found: {sessionName}");
            }

            return 1;
        }

        var status = session.Status ?? "unknown";
        if (!string.Equals(status, "running", StringComparison.OrdinalIgnoreCase) && !quiet)
        {
            Console.WriteLine($"Warning: Session '{sessionName}' is not running (status: {status})");
        }

        if (session.Pid is not null and > 0)
        {
            if (TryKillProcess(session.Pid.Value, out var error))
            {
                if (!quiet)
                {
                    Console.WriteLine($"Stopped process {session.Pid.Value}");
                }
            }
            else if (!quiet)
            {
                Console.WriteLine(error ?? "Warning: Process not found (may have already exited)");
            }
        }

        _stateStore.SetSession(sessionName, existing =>
        {
            existing.Status = "stopped";
            existing.Pid = null;
            existing.TmuxSession = null;
        });

        if (!quiet)
        {
            Console.WriteLine($"Stopped session: {sessionName}");
        }

        return 0;
    }

    private static bool TryKillProcess(int pid, out string? error)
    {
        error = null;
        try
        {
            var process = Process.GetProcessById(pid);
            if (process.HasExited)
            {
                error = "Warning: Process not found (may have already exited)";
                return false;
            }

            process.Kill(true);
            process.WaitForExit(TimeSpan.FromSeconds(5));
            return true;
        }
        catch (ArgumentException)
        {
            error = "Warning: Process not found (may have already exited)";
            return false;
        }
        catch (InvalidOperationException)
        {
            error = "Warning: Process not found (may have already exited)";
            return false;
        }
        catch (Exception ex)
        {
            error = $"Warning: Failed to stop process {pid}: {ex.Message}";
            return false;
        }
    }
}

public sealed class StopCommandSettings
{
    public string? Name { get; init; }
    public bool All { get; init; }
}
