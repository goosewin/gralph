using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using YamlDotNet.Serialization;

namespace Gralph.Config;

public static class YamlConfig
{
    public static IDictionary<string, object?> Load(string path)
    {
        using var reader = new StreamReader(path);
        var deserializer = new DeserializerBuilder().Build();
        var yamlObject = deserializer.Deserialize(reader);
        var normalized = NormalizeYamlObject(yamlObject);
        return normalized as IDictionary<string, object?> ?? new Dictionary<string, object?>(StringComparer.Ordinal);
    }

    public static Dictionary<string, string> Flatten(IDictionary<string, object?> root)
    {
        var result = new Dictionary<string, string>(StringComparer.Ordinal);
        FlattenNode(root, string.Empty, result);
        return result;
    }

    public static void Save(string path, IDictionary<string, object?> root)
    {
        var serializer = new SerializerBuilder().Build();
        using var writer = new StreamWriter(path);
        serializer.Serialize(writer, root);
    }

    public static IDictionary<string, object?> NormalizeYamlObject(object? node)
    {
        if (node is null)
        {
            return new Dictionary<string, object?>(StringComparer.Ordinal);
        }

        if (node is IDictionary<string, object?> stringMap)
        {
            var normalized = new Dictionary<string, object?>(StringComparer.Ordinal);
            foreach (var (key, value) in stringMap)
            {
                normalized[key] = NormalizeYamlValue(value);
            }
            return normalized;
        }

        if (node is IDictionary<object, object> map)
        {
            var normalized = new Dictionary<string, object?>(StringComparer.Ordinal);
            foreach (var entry in map)
            {
                var key = Convert.ToString(entry.Key, CultureInfo.InvariantCulture) ?? string.Empty;
                normalized[key] = NormalizeYamlValue(entry.Value);
            }
            return normalized;
        }

        return new Dictionary<string, object?>(StringComparer.Ordinal);
    }

    private static object? NormalizeYamlValue(object? value)
    {
        if (value is null)
        {
            return null;
        }

        if (value is IDictionary<object, object> map)
        {
            var normalized = new Dictionary<string, object?>(StringComparer.Ordinal);
            foreach (var entry in map)
            {
                var key = Convert.ToString(entry.Key, CultureInfo.InvariantCulture) ?? string.Empty;
                normalized[key] = NormalizeYamlValue(entry.Value);
            }
            return normalized;
        }

        if (value is IDictionary<string, object?> stringMap)
        {
            var normalized = new Dictionary<string, object?>(StringComparer.Ordinal);
            foreach (var (key, child) in stringMap)
            {
                normalized[key] = NormalizeYamlValue(child);
            }
            return normalized;
        }

        if (value is IList list)
        {
            var normalized = new List<object?>();
            foreach (var item in list)
            {
                normalized.Add(NormalizeYamlValue(item));
            }
            return normalized;
        }

        return value;
    }

    private static void FlattenNode(object? node, string prefix, Dictionary<string, string> result)
    {
        if (node is IDictionary<string, object?> map)
        {
            foreach (var (key, value) in map)
            {
                var nextPrefix = string.IsNullOrEmpty(prefix) ? key : $"{prefix}.{key}";
                FlattenNode(value, nextPrefix, result);
            }
            return;
        }

        if (node is IList list)
        {
            var items = new List<string>();
            foreach (var item in list)
            {
                items.Add(ScalarToString(item));
            }

            if (!string.IsNullOrEmpty(prefix))
            {
                result[prefix] = string.Join(",", items);
            }
            return;
        }

        if (!string.IsNullOrEmpty(prefix))
        {
            result[prefix] = ScalarToString(node);
        }
    }

    private static string ScalarToString(object? value)
    {
        if (value is null)
        {
            return string.Empty;
        }

        if (value is bool boolValue)
        {
            return boolValue ? "true" : "false";
        }

        if (value is IFormattable formattable)
        {
            return formattable.ToString(null, CultureInfo.InvariantCulture) ?? string.Empty;
        }

        return value.ToString() ?? string.Empty;
    }
}
