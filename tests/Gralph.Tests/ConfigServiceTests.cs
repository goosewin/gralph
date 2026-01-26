using System;
using System.IO;
using Gralph.Config;
using Xunit;

namespace Gralph.Tests;

public sealed class ConfigServiceTests
{
    [Fact]
    public void Load_MergesDefaultGlobalProject()
    {
        using var temp = new TempDirectory();
        var configDir = temp.CreateSubdirectory("config");
        var projectDir = temp.CreateSubdirectory("project");
        var defaultConfig = Path.Combine(configDir, "default.yaml");
        var globalConfig = Path.Combine(configDir, "config.yaml");
        var projectConfig = Path.Combine(projectDir, ".gralph.yaml");

        File.WriteAllText(defaultConfig, "defaults:\n  max_iterations: 3\n");
        File.WriteAllText(globalConfig, "defaults:\n  max_iterations: 5\n");
        File.WriteAllText(projectConfig, "defaults:\n  max_iterations: 7\n");

        using var env = new EnvironmentScope(
            "GRALPH_CONFIG_DIR",
            "GRALPH_GLOBAL_CONFIG",
            "GRALPH_DEFAULT_CONFIG",
            "GRALPH_PROJECT_CONFIG_NAME");
        env.Set("GRALPH_CONFIG_DIR", configDir);
        env.Set("GRALPH_GLOBAL_CONFIG", globalConfig);
        env.Set("GRALPH_DEFAULT_CONFIG", defaultConfig);
        env.Set("GRALPH_PROJECT_CONFIG_NAME", ".gralph.yaml");

        var paths = ConfigPaths.FromEnvironment();
        var service = new ConfigService(paths);
        service.Load(projectDir);

        var value = service.Get("defaults.max_iterations");
        Assert.Equal("7", value);
    }

    [Fact]
    public void Get_UsesLegacyEnvironmentOverride()
    {
        using var temp = new TempDirectory();
        var configDir = temp.CreateSubdirectory("config");
        var defaultConfig = Path.Combine(configDir, "default.yaml");
        File.WriteAllText(defaultConfig, "defaults:\n  max_iterations: 3\n");

        using var env = new EnvironmentScope(
            "GRALPH_CONFIG_DIR",
            "GRALPH_DEFAULT_CONFIG",
            "GRALPH_MAX_ITERATIONS",
            "GRALPH_DEFAULTS_MAX_ITERATIONS");
        env.Set("GRALPH_CONFIG_DIR", configDir);
        env.Set("GRALPH_DEFAULT_CONFIG", defaultConfig);
        env.Set("GRALPH_MAX_ITERATIONS", "99");
        env.Set("GRALPH_DEFAULTS_MAX_ITERATIONS", "12");

        var service = new ConfigService(ConfigPaths.FromEnvironment());
        service.Load(null);

        var value = service.Get("defaults.max_iterations");
        Assert.Equal("99", value);
    }

    [Fact]
    public void Get_UsesEnvironmentOverride()
    {
        using var temp = new TempDirectory();
        var configDir = temp.CreateSubdirectory("config");
        var defaultConfig = Path.Combine(configDir, "default.yaml");
        File.WriteAllText(defaultConfig, "defaults:\n  max_iterations: 3\n");

        using var env = new EnvironmentScope(
            "GRALPH_CONFIG_DIR",
            "GRALPH_DEFAULT_CONFIG",
            "GRALPH_DEFAULTS_MAX_ITERATIONS");
        env.Set("GRALPH_CONFIG_DIR", configDir);
        env.Set("GRALPH_DEFAULT_CONFIG", defaultConfig);
        env.Set("GRALPH_DEFAULTS_MAX_ITERATIONS", "42");

        var service = new ConfigService(ConfigPaths.FromEnvironment());
        service.Load(null);

        var value = service.Get("defaults.max_iterations");
        Assert.Equal("42", value);
    }

    [Fact]
    public void Set_WritesNestedKey()
    {
        using var temp = new TempDirectory();
        var configDir = temp.CreateSubdirectory("config");
        var globalConfig = Path.Combine(configDir, "config.yaml");
        var defaultConfig = Path.Combine(configDir, "default.yaml");
        File.WriteAllText(defaultConfig, "defaults:\n  max_iterations: 3\n");

        using var env = new EnvironmentScope(
            "GRALPH_CONFIG_DIR",
            "GRALPH_GLOBAL_CONFIG",
            "GRALPH_DEFAULT_CONFIG");
        env.Set("GRALPH_CONFIG_DIR", configDir);
        env.Set("GRALPH_GLOBAL_CONFIG", globalConfig);
        env.Set("GRALPH_DEFAULT_CONFIG", defaultConfig);

        var service = new ConfigService(ConfigPaths.FromEnvironment());
        service.Set("parent.child", "value");

        var loaded = YamlConfig.Load(globalConfig);
        var flattened = YamlConfig.Flatten(loaded);
        Assert.True(flattened.TryGetValue("parent.child", out var value));
        Assert.Equal("value", value);
    }
}
