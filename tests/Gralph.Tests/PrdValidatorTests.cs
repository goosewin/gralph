using System;
using System.Collections.Generic;
using System.IO;
using Gralph.Prd;
using Xunit;

namespace Gralph.Tests;

public sealed class PrdValidatorTests
{
    [Fact]
    public void ValidateAcceptsValidPrd()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.Path;
        var contextDir = System.IO.Path.Combine(projectDir, "lib");
        Directory.CreateDirectory(contextDir);
        File.WriteAllText(System.IO.Path.Combine(contextDir, "context.txt"), "context");

        var prdFile = System.IO.Path.Combine(projectDir, "prd-valid.md");
        File.WriteAllText(prdFile, "# PRD\n\n### Task D-1\n- **ID** D-1\n- **Context Bundle** `lib/context.txt`\n- **DoD** Implement the feature.\n- **Checklist**\n  * Task implemented.\n- **Dependencies** None\n- [ ] D-1 Implement PRD validation\n");

        var errors = new List<string>();
        var ok = PrdValidator.Validate(prdFile, projectDir, errors.Add);

        Assert.True(ok);
        Assert.Empty(errors);
    }

    [Fact]
    public void ValidateReportsMissingField()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.Path;
        var contextDir = System.IO.Path.Combine(projectDir, "lib");
        Directory.CreateDirectory(contextDir);
        File.WriteAllText(System.IO.Path.Combine(contextDir, "context.txt"), "context");

        var prdFile = System.IO.Path.Combine(projectDir, "prd-missing-field.md");
        File.WriteAllText(prdFile, "# PRD\n\n### Task D-2\n- **ID** D-2\n- **Context Bundle** `lib/context.txt`\n- **Checklist**\n  * Missing DoD field.\n- **Dependencies** D-1\n- [ ] D-2 Missing DoD\n");

        var errors = new List<string>();
        var ok = PrdValidator.Validate(prdFile, projectDir, errors.Add);

        Assert.False(ok);
        Assert.Contains(errors, message => message.Contains("Missing required field: DoD", StringComparison.Ordinal));
    }

    [Fact]
    public void ValidateRejectsMultipleUncheckedTaskLines()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.Path;
        var contextDir = System.IO.Path.Combine(projectDir, "lib");
        Directory.CreateDirectory(contextDir);
        File.WriteAllText(System.IO.Path.Combine(contextDir, "context.txt"), "context");

        var prdFile = System.IO.Path.Combine(projectDir, "prd-multiple-unchecked.md");
        File.WriteAllText(prdFile, "# PRD\n\n### Task D-3\n- **ID** D-3\n- **Context Bundle** `lib/context.txt`\n- **DoD** Add strict PRD validation.\n- **Checklist**\n  * Validation added.\n- **Dependencies** D-2\n- [ ] D-3 Add strict PRD validation\n- [ ] D-3 Update error handling\n");

        var errors = new List<string>();
        var ok = PrdValidator.Validate(prdFile, projectDir, errors.Add);

        Assert.False(ok);
        Assert.Contains(errors, message => message.Contains("Multiple unchecked task lines", StringComparison.Ordinal));
    }

    [Fact]
    public void ValidateRejectsStrayUncheckedCheckbox()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.Path;
        var contextDir = System.IO.Path.Combine(projectDir, "context");
        Directory.CreateDirectory(contextDir);
        File.WriteAllText(System.IO.Path.Combine(contextDir, "valid.txt"), "context");

        var prdFile = System.IO.Path.Combine(projectDir, "prd-stray-checkbox.md");
        File.WriteAllText(prdFile, "# PRD\n\n- [ ] Stray unchecked outside task block\n\n### Task D-4\n- **ID** D-4\n- **Context Bundle** `context/valid.txt`\n- **DoD** Fix validation.\n- **Checklist**\n  * Add guard.\n- **Dependencies** None\n- [ ] D-4 Add guard\n");

        var errors = new List<string>();
        var ok = PrdValidator.Validate(prdFile, projectDir, errors.Add);

        Assert.False(ok);
        Assert.Contains(errors, message => message.Contains("Unchecked task line outside task block", StringComparison.Ordinal));
    }

    [Fact]
    public void ValidateRejectsMissingContextBundlePath()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.Path;

        var prdFile = System.IO.Path.Combine(projectDir, "prd-missing-context.md");
        File.WriteAllText(prdFile, "# PRD\n\n### Task D-5\n- **ID** D-5\n- **Context Bundle** `missing/file.txt`\n- **DoD** Ensure context exists.\n- **Checklist**\n  * Validation fails.\n- **Dependencies** None\n- [ ] D-5 Missing context\n");

        var errors = new List<string>();
        var ok = PrdValidator.Validate(prdFile, projectDir, errors.Add);

        Assert.False(ok);
        Assert.Contains(errors, message => message.Contains("Context Bundle path not found", StringComparison.Ordinal));
    }

    [Fact]
    public void ValidateAllowsMissingContextWhenFlagged()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.Path;

        var prdFile = System.IO.Path.Combine(projectDir, "prd-allow-missing-context.md");
        File.WriteAllText(prdFile, "# PRD\n\n### Task D-6\n- **ID** D-6\n- **Context Bundle** `missing/ok.txt`\n- **DoD** Skip context validation.\n- **Checklist**\n  * Validation passes.\n- **Dependencies** None\n- [ ] D-6 Allow missing context\n");

        var errors = new List<string>();
        var ok = PrdValidator.Validate(prdFile, projectDir, errors.Add, allowMissingContext: true);

        Assert.True(ok);
        Assert.Empty(errors);
    }
}
