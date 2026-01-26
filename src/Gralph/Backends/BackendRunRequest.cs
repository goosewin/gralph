namespace Gralph.Backends;

public sealed record BackendRunRequest(string Prompt, string? ModelOverride, string OutputPath, string? RawOutputPath = null);
