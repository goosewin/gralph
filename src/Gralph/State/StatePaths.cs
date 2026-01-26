using System;
using System.Globalization;
using System.IO;

namespace Gralph.State;

public sealed class StatePaths
{
    public string StateDir { get; }
    public string StateFilePath { get; }
    public string LockFilePath { get; }
    public TimeSpan LockTimeout { get; }

    private StatePaths(string stateDir, string stateFilePath, string lockFilePath, TimeSpan lockTimeout)
    {
        StateDir = stateDir;
        StateFilePath = stateFilePath;
        LockFilePath = lockFilePath;
        LockTimeout = lockTimeout;
    }

    public static StatePaths FromEnvironment()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (string.IsNullOrWhiteSpace(home))
        {
            home = Environment.GetEnvironmentVariable("HOME") ?? string.Empty;
        }

        var stateDir = Environment.GetEnvironmentVariable("GRALPH_STATE_DIR")
            ?? Path.Combine(home, ".config", "gralph");

        var stateFile = Environment.GetEnvironmentVariable("GRALPH_STATE_FILE")
            ?? Path.Combine(stateDir, "state.json");

        var lockFile = Environment.GetEnvironmentVariable("GRALPH_LOCK_FILE")
            ?? Path.Combine(stateDir, "state.lock");

        var timeoutSeconds = Environment.GetEnvironmentVariable("GRALPH_LOCK_TIMEOUT");
        var timeout = 10.0;
        if (!string.IsNullOrWhiteSpace(timeoutSeconds)
            && double.TryParse(timeoutSeconds, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed))
        {
            timeout = parsed;
        }

        return new StatePaths(stateDir, stateFile, lockFile, TimeSpan.FromSeconds(timeout));
    }
}
