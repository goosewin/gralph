using YamlDotNet.RepresentationModel;

namespace Gralph.Configuration;

public static class ConfigFileEditor
{
    public static void SetValue(string configPath, string key, string value)
    {
        if (string.IsNullOrWhiteSpace(configPath))
        {
            throw new ArgumentException("Config path is required.", nameof(configPath));
        }

        if (string.IsNullOrWhiteSpace(key))
        {
            throw new ArgumentException("Config key is required.", nameof(key));
        }

        var directory = Path.GetDirectoryName(configPath);
        if (!string.IsNullOrWhiteSpace(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var yaml = new YamlStream();
        YamlMappingNode root;

        if (File.Exists(configPath))
        {
            using var reader = new StreamReader(configPath);
            yaml.Load(reader);
        }

        if (yaml.Documents.Count == 0 || yaml.Documents[0].RootNode is not YamlMappingNode mapping)
        {
            root = new YamlMappingNode();
            yaml.Documents.Clear();
            yaml.Documents.Add(new YamlDocument(root));
        }
        else
        {
            root = mapping;
        }

        var parts = key.Split('.', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (parts.Length == 0)
        {
            throw new ArgumentException("Config key is required.", nameof(key));
        }

        var current = root;
        for (var index = 0; index < parts.Length; index++)
        {
            var part = parts[index];
            if (index == parts.Length - 1)
            {
                current.Children[new YamlScalarNode(part)] = new YamlScalarNode(value);
                break;
            }

            if (!current.Children.TryGetValue(new YamlScalarNode(part), out var child) || child is not YamlMappingNode childMapping)
            {
                childMapping = new YamlMappingNode();
                current.Children[new YamlScalarNode(part)] = childMapping;
            }

            current = childMapping;
        }

        using var writer = new StreamWriter(configPath);
        yaml.Save(writer, false);
    }
}
