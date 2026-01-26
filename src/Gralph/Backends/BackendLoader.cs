using System;
using System.Collections.Generic;
using System.Linq;

namespace Gralph.Backends;

public static class BackendLoader
{
    private static readonly IReadOnlyDictionary<string, Func<IBackend>> Factories =
        new Dictionary<string, Func<IBackend>>(StringComparer.OrdinalIgnoreCase)
        {
            { "claude", () => new ClaudeBackend() },
            { "opencode", () => new OpenCodeBackend() },
            { "gemini", () => new GeminiBackend() },
            { "codex", () => new CodexBackend() }
        };

    public static IReadOnlyList<string> ListAvailable()
    {
        return Factories.Keys.OrderBy(name => name, StringComparer.OrdinalIgnoreCase).ToArray();
    }

    public static IBackend Load(string backendName)
    {
        if (string.IsNullOrWhiteSpace(backendName))
        {
            throw new ArgumentException("Backend name is required.", nameof(backendName));
        }

        if (!Factories.TryGetValue(backendName, out var factory))
        {
            var available = string.Join(", ", ListAvailable());
            throw new KeyNotFoundException($"Backend '{backendName}' not found. Available: {available}.");
        }

        return factory();
    }
}
