using System;
using System.IO;

namespace Gralph.Tests;

internal sealed class TempDirectory : IDisposable
{
    public string Path { get; }

    public TempDirectory()
    {
        Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "gralph-tests-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path);
    }

    public void Dispose()
    {
        try
        {
            if (Directory.Exists(Path))
            {
                Directory.Delete(Path, true);
            }
        }
        catch (IOException)
        {
        }
        catch (UnauthorizedAccessException)
        {
        }
    }
}

internal sealed class EnvVarScope : IDisposable
{
    private readonly string _key;
    private readonly string? _previous;
    private readonly bool _hadValue;

    public EnvVarScope(string key, string? value)
    {
        _key = key;
        _previous = Environment.GetEnvironmentVariable(key);
        _hadValue = _previous is not null;
        Environment.SetEnvironmentVariable(key, value);
    }

    public void Dispose()
    {
        Environment.SetEnvironmentVariable(_key, _hadValue ? _previous : null);
    }
}

internal static class TestPaths
{
    public static string FindRepoRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var candidate = System.IO.Path.Combine(current.FullName, "config", "default.yaml");
            if (File.Exists(candidate))
            {
                return current.FullName;
            }

            current = current.Parent;
        }

        throw new InvalidOperationException("Unable to locate repository root from test base directory.");
    }
}
