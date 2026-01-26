using System;
using System.Collections.Generic;
using System.IO;

namespace Gralph.Tests;

internal sealed class TempDirectory : IDisposable
{
    public TempDirectory()
    {
        Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "gralph-tests-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path);
    }

    public string Path { get; }

    public string CreateSubdirectory(string name)
    {
        var path = System.IO.Path.Combine(Path, name);
        Directory.CreateDirectory(path);
        return path;
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

internal sealed class EnvironmentScope : IDisposable
{
    private readonly Dictionary<string, string?> _originalValues = new(StringComparer.Ordinal);

    public EnvironmentScope(params string[] keys)
    {
        foreach (var key in keys)
        {
            _originalValues[key] = Environment.GetEnvironmentVariable(key);
        }
    }

    public void Set(string key, string? value)
    {
        Environment.SetEnvironmentVariable(key, value);
    }

    public void Dispose()
    {
        foreach (var (key, value) in _originalValues)
        {
            Environment.SetEnvironmentVariable(key, value);
        }
    }
}
