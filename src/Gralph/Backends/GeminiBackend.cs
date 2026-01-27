using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace Gralph.Backends;

public sealed class GeminiBackend : IBackend
{
    private static readonly IReadOnlyList<string> Models = new[] { "gemini-1.5-pro" };

    public string Name => "gemini";

    public bool IsInstalled()
    {
        return CommandExists("gemini");
    }

    public string GetInstallHint()
    {
        return "npm install -g @google/gemini-cli";
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

        EnsureOutputDirectories(request);

        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = "gemini",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false
            }
        };

        process.StartInfo.ArgumentList.Add("--headless");

        if (!string.IsNullOrWhiteSpace(request.ModelOverride))
        {
            process.StartInfo.ArgumentList.Add("--model");
            process.StartInfo.ArgumentList.Add(request.ModelOverride);
        }

        process.StartInfo.ArgumentList.Add(request.Prompt);

        process.Start();

        var stdoutTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
        var stderrTask = process.StandardError.ReadToEndAsync(cancellationToken);

        await process.WaitForExitAsync(cancellationToken);

        var stdout = await stdoutTask;
        var stderr = await stderrTask;
        var combined = CombineOutput(stdout, stderr);

        await File.WriteAllTextAsync(request.OutputPath, combined, cancellationToken);

        if (!string.IsNullOrWhiteSpace(request.RawOutputPath))
        {
            await File.WriteAllTextAsync(request.RawOutputPath, combined, cancellationToken);
        }

        var parsedText = ParseText(combined);

        return new BackendRunResult(process.ExitCode, parsedText, combined);
    }

    public string ParseText(string rawResponse)
    {
        return string.IsNullOrWhiteSpace(rawResponse) ? string.Empty : rawResponse.TrimEnd();
    }

    private static void EnsureOutputDirectories(BackendRunRequest request)
    {
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
