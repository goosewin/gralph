using YamlDotNet.RepresentationModel;

namespace Gralph.Configuration;

public static class YamlConfigParser
{
    public static IReadOnlyDictionary<string, string> ParseFile(string path)
    {
        var result = new Dictionary<string, string>(StringComparer.Ordinal);

        if (!File.Exists(path))
        {
            return result;
        }

        using var reader = new StreamReader(path);
        var yaml = new YamlStream();
        yaml.Load(reader);

        if (yaml.Documents.Count == 0)
        {
            return result;
        }

        var root = yaml.Documents[0].RootNode;
        FlattenNode(root, string.Empty, result);
        return result;
    }

    private static void FlattenNode(YamlNode node, string prefix, IDictionary<string, string> result)
    {
        switch (node)
        {
            case YamlMappingNode mapping:
                foreach (var entry in mapping.Children)
                {
                    if (entry.Key is not YamlScalarNode keyNode)
                    {
                        continue;
                    }

                    var key = keyNode.Value;
                    if (string.IsNullOrWhiteSpace(key))
                    {
                        continue;
                    }

                    var nextPrefix = string.IsNullOrEmpty(prefix) ? key : $"{prefix}.{key}";
                    FlattenNode(entry.Value, nextPrefix, result);
                }

                break;
            case YamlSequenceNode sequence:
                if (string.IsNullOrEmpty(prefix))
                {
                    break;
                }

                var items = new List<string>();
                foreach (var item in sequence.Children)
                {
                    var value = item switch
                    {
                        YamlScalarNode scalar => scalar.Value,
                        _ => item.ToString()
                    };

                    if (!string.IsNullOrWhiteSpace(value))
                    {
                        items.Add(value.Trim());
                    }
                }

                if (items.Count > 0)
                {
                    result[prefix] = string.Join(',', items);
                }

                break;
            case YamlScalarNode scalar:
                if (!string.IsNullOrEmpty(prefix))
                {
                    result[prefix] = scalar.Value ?? string.Empty;
                }

                break;
        }
    }
}
