using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Gralph.Configuration;
using Xunit;

namespace Gralph.Tests;

public sealed class ConfigStoreTests
{
    [Fact]
    public void LoadDefaultConfigIncludesMaxIterations()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out _);
        var store = new ConfigStore();

        store.Load();

        var maxIterations = store.Get("defaults.max_iterations", string.Empty);
        Assert.Equal("30", maxIterations);
    }

    [Fact]
    public void GetReturnsDefaultForMissingKey()
    {
        var store = new ConfigStore();

        var result = store.Get("nonexistent.key", "fallback_value");
        Assert.Equal("fallback_value", result);
    }

    [Fact]
    public void GetReturnsEmptyForMissingKeyWithoutDefault()
    {
        var store = new ConfigStore();

        var result = store.Get("nonexistent.key", string.Empty);
        Assert.Equal(string.Empty, result);
    }

    [Fact]
    public void SetConfigCreatesDirectory()
    {
        using var temp = new TempDirectory();
        var configPath = System.IO.Path.Combine(temp.Path, "config", "config.yaml");

        ConfigFileEditor.SetValue(configPath, "test.key", "test_value");

        Assert.True(Directory.Exists(System.IO.Path.GetDirectoryName(configPath)!));
    }

    [Fact]
    public void SetConfigWritesSimpleKey()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        ConfigFileEditor.SetValue(globalConfig, "simple_key", "simple_value");
        store.Load();

        var result = store.Get("simple_key", string.Empty);
        Assert.Equal("simple_value", result);
    }

    [Fact]
    public void SetConfigWritesNestedKey()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        ConfigFileEditor.SetValue(globalConfig, "parent.child", "nested_value");
        store.Load();

        var result = store.Get("parent.child", string.Empty);
        Assert.Equal("nested_value", result);
    }

    [Fact]
    public void SetConfigUpdatesExistingKey()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        ConfigFileEditor.SetValue(globalConfig, "update_test", "original");
        ConfigFileEditor.SetValue(globalConfig, "update_test", "updated");
        store.Load();

        var result = store.Get("update_test", string.Empty);
        Assert.Equal("updated", result);
    }

    [Fact]
    public void ExistsReturnsTrueForExistingKey()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        ConfigFileEditor.SetValue(globalConfig, "exists_test", "value");
        store.Load();

        Assert.True(store.Exists("exists_test"));
    }

    [Fact]
    public void ExistsReturnsFalseForMissingKey()
    {
        var store = new ConfigStore();

        Assert.False(store.Exists("definitely_not_exists_xyz"));
    }

    [Fact]
    public void EnvOverrideTakesPrecedence()
    {
        using var env = new EnvVarScope("GRALPH_TEST_ENV_KEY", "env_override_value");
        var store = new ConfigStore();

        var result = store.Get("test.env.key", "default");
        Assert.Equal("env_override_value", result);
    }

    [Fact]
    public void LegacyEnvOverrideIsHonored()
    {
        using var env = new EnvVarScope("GRALPH_MAX_ITERATIONS", "99");
        var store = new ConfigStore();

        store.Load();
        var result = store.Get("defaults.max_iterations", string.Empty);
        Assert.Equal("99", result);
    }

    [Fact]
    public void ListIncludesConfiguredKeys()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        ConfigFileEditor.SetValue(globalConfig, "list_test_a", "value_a");
        ConfigFileEditor.SetValue(globalConfig, "list_test_b", "value_b");
        store.Load();

        var list = store.List().ToList();
        Assert.Contains("list_test_a=value_a", list);
        Assert.Contains("list_test_b=value_b", list);
    }

    [Fact]
    public void YamlParserIgnoresComments()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(globalConfig)!);
        File.WriteAllText(globalConfig, "# comment\ncomment_test: actual_value  # inline comment\n");

        store.Load();
        var result = store.Get("comment_test", string.Empty);
        Assert.Equal("actual_value", result);
    }

    [Fact]
    public void YamlParserFlattensSimpleArrays()
    {
        using var temp = new TempDirectory();
        using var env = CreateConfigEnv(temp, out var globalConfig);
        var store = new ConfigStore();

        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(globalConfig)!);
        File.WriteAllText(globalConfig, "test:\n  flags:\n    - --headless\n    - \"--verbose\"  # inline comment\n");

        store.Load();
        var result = store.Get("test.flags", string.Empty);
        Assert.Equal("--headless,--verbose", result);
    }

    private static IDisposable CreateConfigEnv(TempDirectory temp, out string globalConfig)
    {
        var repoRoot = TestPaths.FindRepoRoot();
        var defaultConfig = System.IO.Path.Combine(repoRoot, "config", "default.yaml");
        globalConfig = System.IO.Path.Combine(temp.Path, "config", "config.yaml");

        var scopes = new List<EnvVarScope>
        {
            new EnvVarScope("GRALPH_DEFAULT_CONFIG", defaultConfig),
            new EnvVarScope("GRALPH_GLOBAL_CONFIG", globalConfig),
            new EnvVarScope("GRALPH_CONFIG_DIR", System.IO.Path.Combine(temp.Path, "config"))
        };

        return new CompositeEnvVarScope(scopes);
    }

    private sealed class CompositeEnvVarScope : IDisposable
    {
        private readonly IReadOnlyList<EnvVarScope> _scopes;

        public CompositeEnvVarScope(IReadOnlyList<EnvVarScope> scopes)
        {
            _scopes = scopes;
        }

        public void Dispose()
        {
            foreach (var scope in _scopes)
            {
                scope.Dispose();
            }
        }
    }
}
