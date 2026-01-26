using System.IO;
using Gralph.Prd;
using Xunit;

namespace Gralph.Tests;

public sealed class PrdValidatorTests
{
    [Fact]
    public void ValidateFile_AcceptsValidPrd()
    {
        using var temp = new TempDirectory();
        var contextPath = Path.Combine(temp.Path, "context.txt");
        File.WriteAllText(contextPath, "context");

        var prdPath = Path.Combine(temp.Path, "prd.md");
        File.WriteAllText(prdPath, """
# PRD

### Task D-1
- **ID** D-1
- **Context Bundle** `context.txt`
- **DoD** Implement the feature.
- **Checklist**
  * Done.
- **Dependencies** None
- [ ] D-1 Implement
""");

        var result = PrdValidator.ValidateFile(prdPath, baseDirOverride: temp.Path);
        Assert.True(result.IsValid);
    }

    [Fact]
    public void ValidateFile_FlagsMissingField()
    {
        using var temp = new TempDirectory();
        var contextPath = Path.Combine(temp.Path, "context.txt");
        File.WriteAllText(contextPath, "context");

        var prdPath = Path.Combine(temp.Path, "missing.md");
        File.WriteAllText(prdPath, """
# PRD

### Task D-2
- **ID** D-2
- **Context Bundle** `context.txt`
- **Checklist**
  * Missing DoD.
- **Dependencies** None
- [ ] D-2 Missing DoD
""");

        var result = PrdValidator.ValidateFile(prdPath, baseDirOverride: temp.Path);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.Message == "Missing required field: DoD");
    }

    [Fact]
    public void ValidateFile_FlagsStrayUnchecked()
    {
        using var temp = new TempDirectory();
        var contextPath = Path.Combine(temp.Path, "context.txt");
        File.WriteAllText(contextPath, "context");

        var prdPath = Path.Combine(temp.Path, "stray.md");
        File.WriteAllText(prdPath, """
# PRD

- [ ] Stray unchecked

### Task D-3
- **ID** D-3
- **Context Bundle** `context.txt`
- **DoD** Fix validation.
- **Checklist**
  * Done.
- **Dependencies** None
- [ ] D-3 Fix
""");

        var result = PrdValidator.ValidateFile(prdPath, baseDirOverride: temp.Path);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.Message == "Unchecked task line outside task block");
    }

    [Fact]
    public void ValidateFile_FlagsMultipleUnchecked()
    {
        using var temp = new TempDirectory();
        var contextPath = Path.Combine(temp.Path, "context.txt");
        File.WriteAllText(contextPath, "context");

        var prdPath = Path.Combine(temp.Path, "multi.md");
        File.WriteAllText(prdPath, """
# PRD

### Task D-4
- **ID** D-4
- **Context Bundle** `context.txt`
- **DoD** Fix validation.
- **Checklist**
  * Done.
- **Dependencies** None
- [ ] D-4 Fix
- [ ] D-4 Extra
""");

        var result = PrdValidator.ValidateFile(prdPath, baseDirOverride: temp.Path);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.Message.StartsWith("Multiple unchecked task lines"));
    }

    [Fact]
    public void ValidateFile_AllowsMissingContextWhenFlagSet()
    {
        using var temp = new TempDirectory();
        var prdPath = Path.Combine(temp.Path, "allow.md");
        File.WriteAllText(prdPath, """
# PRD

### Task D-5
- **ID** D-5
- **Context Bundle** `missing.txt`
- **DoD** Skip context validation.
- **Checklist**
  * Done.
- **Dependencies** None
- [ ] D-5 Skip
""");

        var result = PrdValidator.ValidateFile(prdPath, allowMissingContext: true, baseDirOverride: temp.Path);
        Assert.True(result.IsValid);
    }
}
