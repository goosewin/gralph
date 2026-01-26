using System.Collections.ObjectModel;

namespace Gralph.Backends;

public sealed class BackendRegistry
{
    private readonly Dictionary<string, IBackend> _backends;

    public BackendRegistry(IEnumerable<IBackend> backends)
    {
        _backends = new Dictionary<string, IBackend>(StringComparer.OrdinalIgnoreCase);

        foreach (var backend in backends)
        {
            if (string.IsNullOrWhiteSpace(backend.Name))
            {
                throw new ArgumentException("Backend name cannot be empty.", nameof(backends));
            }

            if (!_backends.TryAdd(backend.Name, backend))
            {
                throw new ArgumentException($"Duplicate backend name '{backend.Name}'.", nameof(backends));
            }
        }
    }

    public static string DefaultBackendName => "claude";

    public static BackendRegistry CreateDefault()
    {
        return new BackendRegistry(new IBackend[]
        {
            new ClaudeBackend(),
            new OpenCodeBackend(),
            new GeminiBackend(),
            new CodexBackend()
        });
    }

    public IReadOnlyList<IBackend> List()
    {
        return new ReadOnlyCollection<IBackend>(_backends.Values.ToList());
    }

    public bool TryGet(string name, out IBackend? backend)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            backend = null;
            return false;
        }

        return _backends.TryGetValue(name, out backend);
    }

    public IBackend Get(string name)
    {
        if (TryGet(name, out var backend) && backend is not null)
        {
            return backend;
        }

        var available = string.Join(", ", _backends.Keys.OrderBy(value => value, StringComparer.OrdinalIgnoreCase));
        throw new InvalidOperationException($"Backend '{name}' not found. Available backends: {available}");
    }
}
