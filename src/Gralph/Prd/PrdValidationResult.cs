using System.Collections.Generic;

namespace Gralph.Prd;

public sealed class PrdValidationResult
{
    private readonly List<PrdValidationError> _errors = new();

    public IReadOnlyList<PrdValidationError> Errors => _errors;

    public bool IsValid => _errors.Count == 0;

    public void Add(PrdValidationError error)
    {
        _errors.Add(error);
    }

    public void AddRange(IEnumerable<PrdValidationError> errors)
    {
        _errors.AddRange(errors);
    }
}
