using Gralph.Backends;

namespace Gralph.Commands;

public sealed class BackendsCommandHandler
{
    private readonly BackendRegistry _backendRegistry;

    public BackendsCommandHandler(BackendRegistry backendRegistry)
    {
        _backendRegistry = backendRegistry ?? throw new ArgumentNullException(nameof(backendRegistry));
    }

    public int Execute()
    {
        var backends = _backendRegistry.List()
            .OrderBy(backend => backend.Name, StringComparer.OrdinalIgnoreCase)
            .ToList();

        Console.WriteLine("Available AI backends:");
        Console.WriteLine();

        foreach (var backend in backends)
        {
            var installed = backend.IsInstalled();
            var status = installed ? "installed" : "not installed";
            Console.WriteLine($"  {backend.Name} ({status})");

            if (installed)
            {
                Console.WriteLine($"      Models: {string.Join(' ', backend.Models)}");
            }
            else
            {
                Console.WriteLine($"      Install: {backend.GetInstallHint()}");
            }

            Console.WriteLine();
        }

        Console.WriteLine("Usage: gralph start <dir> --backend <name>");
        return 0;
    }
}
