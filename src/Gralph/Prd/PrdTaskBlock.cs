using System.Collections.Generic;

namespace Gralph.Prd;

public sealed class PrdTaskBlock
{
    public PrdTaskBlock(
        string headerLine,
        string rawText,
        int startLine,
        int endLine,
        string? headerId,
        string? idField,
        IReadOnlyDictionary<string, string?> fields,
        IReadOnlyList<string> contextEntries,
        int uncheckedCount,
        IReadOnlyList<string> duplicateFields)
    {
        HeaderLine = headerLine;
        RawText = rawText;
        StartLine = startLine;
        EndLine = endLine;
        HeaderId = headerId;
        IdField = idField;
        Fields = fields;
        ContextEntries = contextEntries;
        UncheckedCount = uncheckedCount;
        DuplicateFields = duplicateFields;
    }

    public string HeaderLine { get; }
    public string RawText { get; }
    public int StartLine { get; }
    public int EndLine { get; }
    public string? HeaderId { get; }
    public string? IdField { get; }
    public IReadOnlyDictionary<string, string?> Fields { get; }
    public IReadOnlyList<string> ContextEntries { get; }
    public int UncheckedCount { get; }
    public IReadOnlyList<string> DuplicateFields { get; }
}
