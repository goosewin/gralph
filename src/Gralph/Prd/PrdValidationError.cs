namespace Gralph.Prd;

public sealed class PrdValidationError
{
    public PrdValidationError(string filePath, string message, string? taskLabel = null, int? lineNumber = null)
    {
        FilePath = filePath;
        Message = message;
        TaskLabel = taskLabel;
        LineNumber = lineNumber;
    }

    public string FilePath { get; }
    public string Message { get; }
    public string? TaskLabel { get; }
    public int? LineNumber { get; }

    public string Format()
    {
        if (LineNumber.HasValue)
        {
            return $"PRD validation error: {FilePath}: line {LineNumber.Value}: {Message}";
        }

        if (!string.IsNullOrWhiteSpace(TaskLabel))
        {
            return $"PRD validation error: {FilePath}: {TaskLabel}: {Message}";
        }

        return $"PRD validation error: {FilePath}: {Message}";
    }
}
