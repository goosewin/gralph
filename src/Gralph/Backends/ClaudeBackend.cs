using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Gralph.Backends;

public sealed class ClaudeBackend : IBackend
{
    private static readonly IReadOnlyList<string> Models = new[] { "claude-opus-4-5" };

    public string Name => "claude";

    public bool IsInstalled()
    {
        return CommandExists("claude");
    }

    public string GetInstallHint()
    {
        return "npm install -g @anthropic-ai/claude-code";
    }

    public IReadOnlyList<string> GetModels()
    {
        return Models;
    }

    public string GetDefaultModel()
    {
        return Models[0];
    }

    public async Task<BackendRunResult> RunIterationAsync(BackendRunRequest request, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(request.Prompt))
        {
            throw new ArgumentException("Prompt is required.", nameof(request));
        }

        if (string.IsNullOrWhiteSpace(request.OutputPath))
        {
            throw new ArgumentException("Output path is required.", nameof(request));
        }

        var outputDir = Path.GetDirectoryName(request.OutputPath);
        if (!string.IsNullOrWhiteSpace(outputDir))
        {
            Directory.CreateDirectory(outputDir);
        }

        if (!string.IsNullOrWhiteSpace(request.RawOutputPath))
        {
            var rawDir = Path.GetDirectoryName(request.RawOutputPath);
            if (!string.IsNullOrWhiteSpace(rawDir))
            {
                Directory.CreateDirectory(rawDir);
            }
        }

        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = "claude",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false
            }
        };

        process.StartInfo.ArgumentList.Add("--dangerously-skip-permissions");
        process.StartInfo.ArgumentList.Add("--verbose");
        process.StartInfo.ArgumentList.Add("--print");
        process.StartInfo.ArgumentList.Add("--output-format");
        process.StartInfo.ArgumentList.Add("stream-json");

        if (!string.IsNullOrWhiteSpace(request.ModelOverride))
        {
            process.StartInfo.ArgumentList.Add("--model");
            process.StartInfo.ArgumentList.Add(request.ModelOverride);
        }

        process.StartInfo.ArgumentList.Add("-p");
        process.StartInfo.ArgumentList.Add(request.Prompt);
        process.StartInfo.Environment["IS_SANDBOX"] = "1";

        process.Start();

        var stdoutTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
        var stderrTask = process.StandardError.ReadToEndAsync(cancellationToken);

        await process.WaitForExitAsync(cancellationToken);

        var stdout = await stdoutTask;
        var stderr = await stderrTask;
        var combined = CombineOutput(stdout, stderr);
        var jsonStream = ExtractJsonLines(combined);

        await File.WriteAllTextAsync(request.OutputPath, jsonStream, cancellationToken);

        if (!string.IsNullOrWhiteSpace(request.RawOutputPath))
        {
            await File.WriteAllTextAsync(request.RawOutputPath, combined, cancellationToken);
        }

        var parsedText = ParseText(jsonStream);

        return new BackendRunResult(process.ExitCode, parsedText, jsonStream);
    }

    public string ParseText(string rawResponse)
    {
        if (string.IsNullOrWhiteSpace(rawResponse))
        {
            return string.Empty;
        }

        string? finalResult = null;
        var builder = new StringBuilder();

        var parseErrors = 0;
        foreach (var line in SplitLines(rawResponse))
        {
            if (!line.AsSpan().TrimStart().StartsWith("{", StringComparison.Ordinal))
            {
                continue;
            }

            try
            {
                using var doc = JsonDocument.Parse(line);
                if (!doc.RootElement.TryGetProperty("type", out var typeElement))
                {
                    continue;
                }

                var type = typeElement.GetString();
                if (string.Equals(type, "result", StringComparison.OrdinalIgnoreCase))
                {
                    if (doc.RootElement.TryGetProperty("result", out var resultElement))
                    {
                        finalResult = resultElement.GetString() ?? string.Empty;
                    }
                }
                else if (string.Equals(type, "assistant", StringComparison.OrdinalIgnoreCase))
                {
                    AppendAssistantText(doc.RootElement, builder);
                }
            }
            catch (JsonException)
            {
                parseErrors++;
            }
        }

        if (parseErrors > 0)
        {
            Console.Error.WriteLine($"Warning: Failed to parse {parseErrors} Claude JSON line(s).");
        }

        return finalResult ?? builder.ToString().TrimEnd();
    }

    private static void AppendAssistantText(JsonElement root, StringBuilder builder)
    {
        if (!root.TryGetProperty("message", out var messageElement))
        {
            return;
        }

        if (!messageElement.TryGetProperty("content", out var contentElement)
            || contentElement.ValueKind != JsonValueKind.Array)
        {
            return;
        }

        foreach (var item in contentElement.EnumerateArray())
        {
            if (!item.TryGetProperty("type", out var typeElement))
            {
                continue;
            }

            if (!string.Equals(typeElement.GetString(), "text", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (!item.TryGetProperty("text", out var textElement))
            {
                continue;
            }

            var text = textElement.GetString();
            if (string.IsNullOrEmpty(text))
            {
                continue;
            }

            if (builder.Length > 0)
            {
                builder.Append("\n\n");
            }

            builder.Append(text);
        }
    }

    private static string CombineOutput(string stdout, string stderr)
    {
        if (string.IsNullOrEmpty(stderr))
        {
            return stdout ?? string.Empty;
        }

        if (string.IsNullOrEmpty(stdout))
        {
            return stderr;
        }

        return stdout + "\n" + stderr;
    }

    private static string ExtractJsonLines(string content)
    {
        if (string.IsNullOrWhiteSpace(content))
        {
            return string.Empty;
        }

        var builder = new StringBuilder();
        foreach (var line in SplitLines(content))
        {
            if (!line.AsSpan().TrimStart().StartsWith("{", StringComparison.Ordinal))
            {
                continue;
            }

            if (builder.Length > 0)
            {
                builder.Append('\n');
            }

            builder.Append(line);
        }

        return builder.ToString();
    }

    private static IEnumerable<string> SplitLines(string content)
    {
        using var reader = new StringReader(content ?? string.Empty);
        string? line;
        while ((line = reader.ReadLine()) != null)
        {
            yield return line;
        }
    }

    private static bool CommandExists(string command)
    {
        if (string.IsNullOrWhiteSpace(command))
        {
            return false;
        }

        var path = Environment.GetEnvironmentVariable("PATH");
        if (string.IsNullOrWhiteSpace(path))
        {
            return false;
        }

        var extension = OperatingSystem.IsWindows() ? ".exe" : string.Empty;
        foreach (var dir in path.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            var candidate = Path.Combine(dir, command + extension);
            if (File.Exists(candidate))
            {
                return true;
            }
        }

        return false;
    }
}
