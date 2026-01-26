namespace Gralph.Backends;

public interface IBackend
{
    string Name { get; }
    IReadOnlyList<string> Models { get; }
    string? DefaultModel { get; }
    bool IsInstalled();
    string GetInstallHint();
    Task<int> RunIterationAsync(string prompt, string? modelOverride, string outputFile, CancellationToken cancellationToken);
    string ParseText(string responseFile);
}
