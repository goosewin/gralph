using System;
using System.IO;
using Gralph;
using Xunit;

namespace Gralph.Tests;

public sealed class CliArgumentParsingTests
{
    [Fact]
    public async Task StartWithoutDirectory_ReturnsError()
    {
        var (result, error) = await CaptureErrorAsync(() => Program.Main(new[] { "start" }));

        Assert.Equal(1, result);
        Assert.Contains("Directory is required", error, StringComparison.Ordinal);
    }

    [Fact]
    public async Task StartWithUnknownOption_ReturnsError()
    {
        var (result, error) = await CaptureErrorAsync(() => Program.Main(new[] { "start", "--unknown" }));

        Assert.Equal(1, result);
        Assert.Contains("Unknown option", error, StringComparison.Ordinal);
    }

    private static async Task<(int ExitCode, string Error)> CaptureErrorAsync(Func<Task<int>> action)
    {
        var originalError = Console.Error;
        try
        {
            using var writer = new StringWriter();
            Console.SetError(writer);
            var exitCode = await action();
            writer.Flush();
            return (exitCode, writer.ToString());
        }
        finally
        {
            Console.SetError(originalError);
        }
    }
}
