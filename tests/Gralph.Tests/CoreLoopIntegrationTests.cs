using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Gralph.Backends;
using Gralph.Config;
using Gralph.Core;
using Xunit;

namespace Gralph.Tests;

public sealed class CoreLoopIntegrationTests
{
    [Fact]
    public async Task RunAsync_Completes_WhenPromiseAndNoRemainingTasks()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.CreateSubdirectory("project");
        var taskPath = Path.Combine(projectDir, "PRD.md");
        File.WriteAllText(taskPath, "# Tasks\n\n- [x] Done\n");

        using var env = new EnvironmentScope("GRALPH_CONFIG_DIR", "GRALPH_GLOBAL_CONFIG", "GRALPH_DEFAULT_CONFIG");
        var config = BuildConfig(env, projectDir);

        var backend = new FakeBackend("All done\n<promise>COMPLETE</promise>");
        var loop = new CoreLoop(config, backend);
        var options = new CoreLoopOptions(projectDir)
        {
            TaskFile = "PRD.md",
            MaxIterations = 1,
            CompletionMarker = "COMPLETE",
            LogFilePath = Path.Combine(projectDir, "loop.log"),
            SessionName = "test"
        };

        var result = await loop.RunAsync(options);

        Assert.Equal(CoreLoopStatus.Complete, result.Status);
        Assert.Equal(1, backend.RunCount);
    }

    [Fact]
    public async Task RunAsync_DoesNotComplete_WhenTasksRemain()
    {
        using var temp = new TempDirectory();
        var projectDir = temp.CreateSubdirectory("project");
        var taskPath = Path.Combine(projectDir, "PRD.md");
        File.WriteAllText(taskPath, "# Tasks\n\n- [ ] Pending\n");

        using var env = new EnvironmentScope("GRALPH_CONFIG_DIR", "GRALPH_GLOBAL_CONFIG", "GRALPH_DEFAULT_CONFIG");
        var config = BuildConfig(env, projectDir);

        var backend = new FakeBackend("<promise>COMPLETE</promise>");
        var loop = new CoreLoop(config, backend);
        var options = new CoreLoopOptions(projectDir)
        {
            TaskFile = "PRD.md",
            MaxIterations = 1,
            CompletionMarker = "COMPLETE",
            LogFilePath = Path.Combine(projectDir, "loop.log"),
            SessionName = "test"
        };

        var result = await loop.RunAsync(options);

        Assert.Equal(CoreLoopStatus.MaxIterations, result.Status);
        Assert.Equal(1, result.RemainingTasks);
    }

    private static ConfigService BuildConfig(EnvironmentScope env, string projectDir)
    {
        var repoRoot = FindRepoRoot();
        var configDir = Path.Combine(projectDir, ".config");
        var globalConfig = Path.Combine(configDir, "config.yaml");
        var defaultConfig = Path.Combine(repoRoot, "config", "default.yaml");

        Directory.CreateDirectory(configDir);
        env.Set("GRALPH_CONFIG_DIR", configDir);
        env.Set("GRALPH_GLOBAL_CONFIG", globalConfig);
        env.Set("GRALPH_DEFAULT_CONFIG", defaultConfig);

        var config = new ConfigService(ConfigPaths.FromEnvironment());
        config.Load(projectDir);
        return config;
    }

    private static string FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null)
        {
            var candidate = Path.Combine(dir.FullName, "config", "default.yaml");
            if (File.Exists(candidate))
            {
                return dir.FullName;
            }

            dir = dir.Parent;
        }

        throw new InvalidOperationException("Repository root not found.");
    }

    private sealed class FakeBackend : IBackend
    {
        private readonly Queue<string> _responses;

        public FakeBackend(params string[] responses)
        {
            _responses = new Queue<string>(responses ?? Array.Empty<string>());
        }

        public string Name => "mock";
        public int RunCount { get; private set; }
        public bool IsInstalled() => true;
        public string GetInstallHint() => string.Empty;
        public IReadOnlyList<string> GetModels() => new[] { "mock-model" };
        public string GetDefaultModel() => "mock-model";
        public string ParseText(string rawResponse) => rawResponse;

        public Task<BackendRunResult> RunIterationAsync(BackendRunRequest request, CancellationToken cancellationToken)
        {
            RunCount++;
            var response = _responses.Count > 0 ? _responses.Dequeue() : string.Empty;
            return Task.FromResult(new BackendRunResult(0, response, response));
        }
    }
}
