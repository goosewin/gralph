using System.IO;

namespace Gralph.Backends;

internal static class ExecutableLocator
{
    public static bool CommandExists(string command)
    {
        return FindOnPath(command) is not null;
    }

    public static string? FindOnPath(string command)
    {
        if (string.IsNullOrWhiteSpace(command))
        {
            return null;
        }

        var paths = (Environment.GetEnvironmentVariable("PATH") ?? string.Empty)
            .Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

        if (OperatingSystem.IsWindows())
        {
            return FindWindowsExecutable(command, paths);
        }

        return FindUnixExecutable(command, paths);
    }

    private static string? FindUnixExecutable(string command, string[] paths)
    {
        if (Path.IsPathRooted(command))
        {
            return File.Exists(command) ? command : null;
        }

        foreach (var path in paths)
        {
            var candidate = Path.Combine(path, command);
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        return null;
    }

    private static string? FindWindowsExecutable(string command, string[] paths)
    {
        if (Path.IsPathRooted(command))
        {
            return File.Exists(command) ? command : null;
        }

        var pathExt = (Environment.GetEnvironmentVariable("PATHEXT") ?? ".EXE;.CMD;.BAT")
            .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        var hasExtension = Path.HasExtension(command);

        foreach (var path in paths)
        {
            if (hasExtension)
            {
                var candidate = Path.Combine(path, command);
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                continue;
            }

            foreach (var ext in pathExt)
            {
                var candidate = Path.Combine(path, command + ext.ToLowerInvariant());
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                candidate = Path.Combine(path, command + ext.ToUpperInvariant());
                if (File.Exists(candidate))
                {
                    return candidate;
                }
            }
        }

        return null;
    }
}
