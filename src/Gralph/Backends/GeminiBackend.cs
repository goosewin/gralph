using System.Diagnostics;
using System.Text;

namespace Gralph.Backends;

public sealed class GeminiBackend : IBackend
{
    private static readonly IReadOnlyList<string> GeminiModels = new[]
    {
        "gemini-1.5-pro"
    };

    public string Name => "gemini";

    public IReadOnlyList<string> Models => GeminiModels;

    public string? DefaultModel => "gemini-1.5-pro";

    public bool IsInstalled()
    {
        return ExecutableLocator.CommandExists("gemini");
    }

    public string GetInstallHint()
    {
        return "npm install -g @google/gemini-cli";
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

        return File.ReadAllText(responseFile);
    }

    private static ProcessStartInfo BuildStartInfo(string prompt, string? modelOverride)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "gemini",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false
        };

        startInfo.ArgumentList.Add("--headless");

        if (!string.IsNullOrWhiteSpace(modelOverride))
        {
            startInfo.ArgumentList.Add("--model");
            startInfo.ArgumentList.Add(modelOverride);
        }

        startInfo.ArgumentList.Add(prompt);

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

            lock (writeLock)
            {
                outputWriter.WriteLine(line);
            }

            Console.Out.WriteLine(line);
        }
    }
}
