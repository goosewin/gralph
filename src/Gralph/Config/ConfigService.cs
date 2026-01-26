using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Gralph.Config;

public sealed class ConfigService
{
    private readonly ConfigPaths _paths;
    private readonly Dictionary<string, string> _cache = new(StringComparer.Ordinal);

    public ConfigService(ConfigPaths paths)
    {
        _paths = paths;
    }

    public void Load(string? projectDir)
    {
        _cache.Clear();

        var configFiles = new List<string>();

        if (File.Exists(_paths.DefaultConfigPath))
        {
            configFiles.Add(_paths.DefaultConfigPath);
        }

        if (File.Exists(_paths.GlobalConfigPath))
        {
            configFiles.Add(_paths.GlobalConfigPath);
        }

        var projectConfig = string.Empty;
        if (!string.IsNullOrWhiteSpace(projectDir) && Directory.Exists(projectDir))
        {
            projectConfig = Path.Combine(projectDir, _paths.ProjectConfigName);
        }

        if (!string.IsNullOrWhiteSpace(projectConfig) && File.Exists(projectConfig))
        {
            configFiles.Add(projectConfig);
        }

        foreach (var configFile in configFiles)
        {
            var configData = YamlConfig.Load(configFile);
            var flatConfig = YamlConfig.Flatten(configData);
            foreach (var (key, value) in flatConfig)
            {
                _cache[key] = value;
            }
        }
    }

    public string Get(string key, string? defaultValue = null)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            return defaultValue ?? string.Empty;
        }

        if (TryLegacyEnvOverride(key, out var legacyValue))
        {
            return legacyValue;
        }

        var envKey = $"GRALPH_{key.ToUpperInvariant().Replace('.', '_')}";
        var envValue = Environment.GetEnvironmentVariable(envKey);
        if (!string.IsNullOrEmpty(envValue))
        {
            return envValue;
        }

        if (_cache.TryGetValue(key, out var value))
        {
            return value;
        }

        return defaultValue ?? string.Empty;
    }

    public bool Exists(string key)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            return false;
        }

        if (TryLegacyEnvOverride(key, out _))
        {
            return true;
        }

        var envKey = $"GRALPH_{key.ToUpperInvariant().Replace('.', '_')}";
        var envValue = Environment.GetEnvironmentVariable(envKey);
        if (!string.IsNullOrEmpty(envValue))
        {
            return true;
        }

        return _cache.ContainsKey(key);
    }

    public IReadOnlyList<KeyValuePair<string, string>> ListMerged()
    {
        return _cache.OrderBy(pair => pair.Key, StringComparer.Ordinal).ToList();
    }

    public void Set(string key, string value)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            throw new ArgumentException("Configuration key is required.", nameof(key));
        }

        Directory.CreateDirectory(_paths.ConfigDir);

        var root = File.Exists(_paths.GlobalConfigPath)
            ? YamlConfig.Load(_paths.GlobalConfigPath)
            : new Dictionary<string, object?>(StringComparer.Ordinal);

        var parts = key.Split('.', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (parts.Length == 0)
        {
            throw new ArgumentException("Invalid configuration key.", nameof(key));
        }

        SetNestedValue(root, parts, value);

        YamlConfig.Save(_paths.GlobalConfigPath, root);
        _cache[key] = value;
    }

    private static void SetNestedValue(IDictionary<string, object?> root, string[] parts, string value)
    {
        var current = root;
        for (var i = 0; i < parts.Length; i++)
        {
            var part = parts[i];
            if (i == parts.Length - 1)
            {
                current[part] = value;
                return;
            }

            if (!current.TryGetValue(part, out var next) || next is not IDictionary<string, object?> nextDict)
            {
                nextDict = new Dictionary<string, object?>(StringComparer.Ordinal);
                current[part] = nextDict;
            }

            current = nextDict;
        }
    }

    private static bool TryLegacyEnvOverride(string key, out string value)
    {
        var legacyEnv = key switch
        {
            "defaults.max_iterations" => "GRALPH_MAX_ITERATIONS",
            "defaults.task_file" => "GRALPH_TASK_FILE",
            "defaults.completion_marker" => "GRALPH_COMPLETION_MARKER",
            "defaults.backend" => "GRALPH_BACKEND",
            "defaults.model" => "GRALPH_MODEL",
            _ => string.Empty
        };

        if (!string.IsNullOrEmpty(legacyEnv))
        {
            var legacyValue = Environment.GetEnvironmentVariable(legacyEnv);
            if (!string.IsNullOrEmpty(legacyValue))
            {
                value = legacyValue;
                return true;
            }
        }

        value = string.Empty;
        return false;
    }
}
