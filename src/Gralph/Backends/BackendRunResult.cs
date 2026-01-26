namespace Gralph.Backends;

public sealed record BackendRunResult(int ExitCode, string ParsedText, string RawResponse);
