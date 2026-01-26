using System;
using System.IO;
using Gralph;
using Xunit;

namespace Gralph.Tests;

public sealed class CliArgumentParsingTests
{
    [Fact]
    public void StartWithoutDirectory_ReturnsError()
    {
        var result = CaptureError(() => Program.Main(new[] { "start" }), out var error);

        Assert.Equal(1, result);
        Assert.Contains("Directory is required", error, StringComparison.Ordinal);
    }

    [Fact]
    public void StartWithUnknownOption_ReturnsError()
    {
        var result = CaptureError(() => Program.Main(new[] { "start", "--unknown" }), out var error);

        Assert.Equal(1, result);
        Assert.Contains("Unknown option", error, StringComparison.Ordinal);
    }

    private static int CaptureError(Func<int> action, out string error)
    {
        var originalError = Console.Error;
        try
        {
            using var writer = new StringWriter();
            Console.SetError(writer);
            var exitCode = action();
            writer.Flush();
            error = writer.ToString();
            return exitCode;
        }
        finally
        {
            Console.SetError(originalError);
        }
    }
}
