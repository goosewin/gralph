using System.IO;
using Gralph.Prd;
using Xunit;

namespace Gralph.Tests;

public sealed class StackDetectorTests
{
    [Fact]
    public void DetectsMultipleStacksWhenMarkersPresent()
    {
        using var temp = new TempDirectory();
        File.WriteAllText(System.IO.Path.Combine(temp.Path, "package.json"), "{\n  \"name\": \"stack-detect\",\n  \"version\": \"1.0.0\"\n}\n");
        File.WriteAllText(System.IO.Path.Combine(temp.Path, "go.mod"), "module example.com/stack\n\ngo 1.21\n");

        var result = StackDetector.Detect(temp.Path);

        Assert.Contains("Node.js", result.StackIds);
        Assert.Contains("Go", result.StackIds);
        Assert.True(result.StackIds.Count > 1);
    }
}
