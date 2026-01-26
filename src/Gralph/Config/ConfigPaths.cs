using System;
using System.IO;

namespace Gralph.Config;

public sealed class ConfigPaths
{
    public string ConfigDir { get; }
    public string GlobalConfigPath { get; }
    public string DefaultConfigPath { get; }
    public string ProjectConfigName { get; }

    private ConfigPaths(string configDir, string globalConfigPath, string defaultConfigPath, string projectConfigName)
    {
        ConfigDir = configDir;
        GlobalConfigPath = globalConfigPath;
        DefaultConfigPath = defaultConfigPath;
        ProjectConfigName = projectConfigName;
    }

    public static ConfigPaths FromEnvironment()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (string.IsNullOrWhiteSpace(home))
        {
            home = Environment.GetEnvironmentVariable("HOME") ?? string.Empty;
        }

        var configDir = Environment.GetEnvironmentVariable("GRALPH_CONFIG_DIR")
            ?? Path.Combine(home, ".config", "gralph");

        var globalConfig = Environment.GetEnvironmentVariable("GRALPH_GLOBAL_CONFIG")
            ?? Path.Combine(configDir, "config.yaml");

        var projectConfigName = Environment.GetEnvironmentVariable("GRALPH_PROJECT_CONFIG_NAME")
            ?? ".gralph.yaml";

        var baseDir = AppContext.BaseDirectory;
        var defaultConfigDir = Path.GetFullPath(Path.Combine(baseDir, "..", "config"));

        if (!Directory.Exists(defaultConfigDir))
        {
            var installedConfigDir = Path.Combine(configDir, "config");
            if (Directory.Exists(installedConfigDir))
            {
                defaultConfigDir = installedConfigDir;
            }
        }

        var defaultConfig = Environment.GetEnvironmentVariable("GRALPH_DEFAULT_CONFIG")
            ?? Path.Combine(defaultConfigDir, "default.yaml");

        return new ConfigPaths(configDir, globalConfig, defaultConfig, projectConfigName);
    }
}
