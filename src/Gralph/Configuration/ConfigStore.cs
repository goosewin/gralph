namespace Gralph.Configuration;

public sealed class ConfigStore
{
    private static readonly IReadOnlyDictionary<string, string> LegacyOverrides =
        new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["defaults.max_iterations"] = "GRALPH_MAX_ITERATIONS",
            ["defaults.task_file"] = "GRALPH_TASK_FILE",
            ["defaults.completion_marker"] = "GRALPH_COMPLETION_MARKER",
            ["defaults.backend"] = "GRALPH_BACKEND",
            ["defaults.model"] = "GRALPH_MODEL"
        };

    private readonly Dictionary<string, string> _cache = new(StringComparer.Ordinal);

    public IReadOnlyDictionary<string, string> Cache => _cache;

    public void Load(string? projectDir = null)
    {
        _cache.Clear();

        var configFiles = new List<string>();

        var defaultConfig = ConfigPaths.DefaultConfigPath;
        if (File.Exists(defaultConfig))
        {
            configFiles.Add(defaultConfig);
        }

        var globalConfig = ConfigPaths.GlobalConfigPath;
        if (File.Exists(globalConfig))
        {
            configFiles.Add(globalConfig);
        }

        var projectConfig = ConfigPaths.GetProjectConfigPath(projectDir);
        if (!string.IsNullOrWhiteSpace(projectConfig) && File.Exists(projectConfig))
        {
            configFiles.Add(projectConfig);
        }

        foreach (var configFile in configFiles)
        {
            var values = YamlConfigParser.ParseFile(configFile);
            foreach (var pair in values)
            {
                _cache[pair.Key] = pair.Value;
            }
        }
    }

    public string Get(string key, string? defaultValue = null)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            return defaultValue ?? string.Empty;
        }

        if (TryGetLegacyOverride(key, out var legacyValue))
        {
            return legacyValue;
        }

        var envKey = ToEnvKey(key);
        var envValue = Environment.GetEnvironmentVariable(envKey);
        if (!string.IsNullOrWhiteSpace(envValue))
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

        if (TryGetLegacyOverride(key, out _))
        {
            return true;
        }

        var envKey = ToEnvKey(key);
        var envValue = Environment.GetEnvironmentVariable(envKey);
        if (!string.IsNullOrWhiteSpace(envValue))
        {
            return true;
        }

        return _cache.ContainsKey(key);
    }

    public IEnumerable<string> List()
    {
        return _cache
            .OrderBy(pair => pair.Key, StringComparer.Ordinal)
            .Select(pair => $"{pair.Key}={pair.Value}");
    }

    private static bool TryGetLegacyOverride(string key, out string value)
    {
        value = string.Empty;
        if (!LegacyOverrides.TryGetValue(key, out var envKey))
        {
            return false;
        }

        var envValue = Environment.GetEnvironmentVariable(envKey);
        if (string.IsNullOrWhiteSpace(envValue))
        {
            return false;
        }

        value = envValue;
        return true;
    }

    private static string ToEnvKey(string key)
    {
        return $"GRALPH_{key.Replace('.', '_').ToUpperInvariant()}";
    }
}
