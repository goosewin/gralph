using Gralph.Backends;
using Gralph.Prd;

namespace Gralph.Commands;

public sealed class PrdCheckCommandHandler
{
    public int Execute(PrdCheckSettings settings)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        if (string.IsNullOrWhiteSpace(settings.FilePath))
        {
            Console.Error.WriteLine("Error: PRD file path is required.");
            return 1;
        }

        var filePath = Path.GetFullPath(settings.FilePath);
        if (!File.Exists(filePath))
        {
            Console.Error.WriteLine($"Error: Task file does not exist: {filePath}");
            return 1;
        }

        var baseDir = settings.BaseDir;
        if (string.IsNullOrWhiteSpace(baseDir))
        {
            baseDir = Path.GetDirectoryName(filePath) ?? Directory.GetCurrentDirectory();
        }

        var errors = new List<string>();
        if (!PrdValidator.Validate(filePath, baseDir, errors.Add, settings.AllowMissingContext))
        {
            foreach (var error in errors)
            {
                Console.Error.WriteLine(error);
            }

            return 1;
        }

        Console.WriteLine($"PRD validation passed: {filePath}");
        return 0;
    }
}

public sealed class PrdCheckSettings
{
    public string FilePath { get; init; } = string.Empty;
    public string? BaseDir { get; init; }
    public bool AllowMissingContext { get; init; }
}

public sealed class PrdCreateCommandHandler
{
    private readonly BackendRegistry _backendRegistry;

    public PrdCreateCommandHandler(BackendRegistry backendRegistry)
    {
        _backendRegistry = backendRegistry ?? throw new ArgumentNullException(nameof(backendRegistry));
    }

    public async Task<int> ExecuteAsync(PrdCreateSettings settings, CancellationToken cancellationToken)
    {
        if (settings is null)
        {
            throw new ArgumentNullException(nameof(settings));
        }

        return await PrdGenerator.GenerateAsync(settings, _backendRegistry, cancellationToken);
    }
}
