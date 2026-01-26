using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace Gralph.Backends;

public interface IBackend
{
    string Name { get; }
    bool IsInstalled();
    string GetInstallHint();
    IReadOnlyList<string> GetModels();
    string GetDefaultModel();
    Task<BackendRunResult> RunIterationAsync(BackendRunRequest request, CancellationToken cancellationToken);
    string ParseText(string rawResponse);
}
