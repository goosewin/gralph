using System.Diagnostics;
using System.Text;
using System.Text.Json;

namespace Gralph.Backends;

public sealed class ClaudeBackend : IBackend
{
    private static readonly IReadOnlyList<string> ClaudeModels = new[]
    {
        "claude-opus-4-5"
    };

    public string Name => "claude";

    public IReadOnlyList<string> Models => ClaudeModels;

    public string? DefaultModel => "claude-opus-4-5";

    public bool IsInstalled()
    {
        return ExecutableLocator.CommandExists("claude");
    }

    public string GetInstallHint()
    {
        return "npm install -g @anthropic-ai/claude-code";
    }

    public async Task<int> RunIterationAsync(string prompt, string? modelOverride, string outputFile, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(prompt))
        {
            throw new ArgumentException("Prompt is required.", nameof(prompt));
        }

        if (string.IsNullOrWhiteSpace(outputFile))
        {
            throw new ArgumentException("Output file path is required.", nameof(outputFile));
        }

        var outputDirectory = Path.GetDirectoryName(outputFile);
        if (!string.IsNullOrWhiteSpace(outputDirectory))
        {
            Directory.CreateDirectory(outputDirectory);
        }

        using var outputWriter = new StreamWriter(outputFile, false, new UTF8Encoding(false));
        outputWriter.AutoFlush = true;
        var writeLock = new object();

        using var process = new Process();
        process.StartInfo = BuildStartInfo(prompt, modelOverride);

        if (!process.Start())
        {
            return 1;
        }

        var stdoutTask = ReadStreamAsync(process.StandardOutput, outputWriter, writeLock, cancellationToken);
        var stderrTask = ReadStreamAsync(process.StandardError, outputWriter, writeLock, cancellationToken);

        await Task.WhenAll(stdoutTask, stderrTask, process.WaitForExitAsync(cancellationToken));

        return process.ExitCode;
    }

    public string ParseText(string responseFile)
    {
        if (string.IsNullOrWhiteSpace(responseFile) || !File.Exists(responseFile))
        {
            return string.Empty;
        }

        string? result = null;

        foreach (var line in File.ReadLines(responseFile))
        {
            if (!IsJsonLine(line))
            {
                continue;
            }

            if (TryExtractResultText(line, out var text))
            {
                result = text;
            }
        }

        return !string.IsNullOrWhiteSpace(result) ? result : File.ReadAllText(responseFile);
    }

    private static ProcessStartInfo BuildStartInfo(string prompt, string? modelOverride)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "claude",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false
        };

        startInfo.ArgumentList.Add("--dangerously-skip-permissions");
        startInfo.ArgumentList.Add("--verbose");
        startInfo.ArgumentList.Add("--print");
        startInfo.ArgumentList.Add("--output-format");
        startInfo.ArgumentList.Add("stream-json");

        if (!string.IsNullOrWhiteSpace(modelOverride))
        {
            startInfo.ArgumentList.Add("--model");
            startInfo.ArgumentList.Add(modelOverride);
        }

        startInfo.ArgumentList.Add("-p");
        startInfo.ArgumentList.Add(prompt);
        startInfo.Environment["IS_SANDBOX"] = "1";

        return startInfo;
    }

    private static async Task ReadStreamAsync(StreamReader reader, StreamWriter outputWriter, object writeLock, CancellationToken cancellationToken)
    {
        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var line = await reader.ReadLineAsync();
            if (line is null)
            {
                break;
            }

            if (!IsJsonLine(line))
            {
                continue;
            }

            lock (writeLock)
            {
                outputWriter.WriteLine(line);
            }

            if (TryExtractAssistantText(line, out var text))
            {
                Console.Out.Write(text.Replace("\n", Environment.NewLine));
                Console.Out.WriteLine();
                Console.Out.WriteLine();
            }
        }
    }

    private static bool TryExtractAssistantText(string jsonLine, out string text)
    {
        text = string.Empty;

        try
        {
            using var document = JsonDocument.Parse(jsonLine);
            var root = document.RootElement;

            if (!root.TryGetProperty("type", out var typeElement))
            {
                return false;
            }

            if (!string.Equals(typeElement.GetString(), "assistant", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            if (!root.TryGetProperty("message", out var messageElement))
            {
                return false;
            }

            if (!messageElement.TryGetProperty("content", out var contentElement))
            {
                return false;
            }

            if (contentElement.ValueKind != JsonValueKind.Array)
            {
                return false;
            }

            var builder = new StringBuilder();
            foreach (var item in contentElement.EnumerateArray())
            {
                if (!item.TryGetProperty("type", out var contentType))
                {
                    continue;
                }

                if (!string.Equals(contentType.GetString(), "text", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (item.TryGetProperty("text", out var textElement))
                {
                    builder.Append(textElement.GetString());
                }
            }

            text = builder.ToString();
            return !string.IsNullOrEmpty(text);
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool TryExtractResultText(string jsonLine, out string text)
    {
        text = string.Empty;

        try
        {
            using var document = JsonDocument.Parse(jsonLine);
            var root = document.RootElement;

            if (!root.TryGetProperty("type", out var typeElement))
            {
                return false;
            }

            if (!string.Equals(typeElement.GetString(), "result", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            if (!root.TryGetProperty("result", out var resultElement))
            {
                return false;
            }

            if (resultElement.ValueKind != JsonValueKind.String)
            {
                return false;
            }

            text = resultElement.GetString() ?? string.Empty;
            return !string.IsNullOrEmpty(text);
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool IsJsonLine(string line)
    {
        return !string.IsNullOrWhiteSpace(line) && line.TrimStart().StartsWith("{", StringComparison.Ordinal);
    }
}
