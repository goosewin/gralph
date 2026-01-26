using System;
using Gralph.Backends;
using Xunit;

namespace Gralph.Tests;

public sealed class BackendIntegrationTests
{
    [Fact]
    public void ListAvailable_IncludesExpectedBackends()
    {
        var available = BackendLoader.ListAvailable();

        Assert.Contains("claude", available);
        Assert.Contains("opencode", available);
        Assert.Contains("gemini", available);
        Assert.Contains("codex", available);
    }

    [Fact]
    public void ClaudeBackend_ParseText_UsesResultField()
    {
        var backend = new ClaudeBackend();
        var raw = string.Join("\n", new[]
        {
            "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Hello\"}]}}",
            "{\"type\":\"result\",\"result\":\"Final output\"}"
        });

        var parsed = backend.ParseText(raw);

        Assert.Equal("Final output", parsed);
    }

    [Fact]
    public void ClaudeBackend_ParseText_UsesAssistantTextWhenNoResult()
    {
        var backend = new ClaudeBackend();
        var raw = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Only assistant\"}]}}";

        var parsed = backend.ParseText(raw);

        Assert.Equal("Only assistant", parsed);
    }

    [Fact]
    public void OpenCodeBackend_ParseText_TrimsOutput()
    {
        AssertTrimmedParse(new OpenCodeBackend());
    }

    [Fact]
    public void GeminiBackend_ParseText_TrimsOutput()
    {
        AssertTrimmedParse(new GeminiBackend());
    }

    [Fact]
    public void CodexBackend_ParseText_TrimsOutput()
    {
        AssertTrimmedParse(new CodexBackend());
    }

    private static void AssertTrimmedParse(IBackend backend)
    {
        var raw = "Hello world  \n";

        var parsed = backend.ParseText(raw);

        Assert.Equal("Hello world", parsed);
    }
}
