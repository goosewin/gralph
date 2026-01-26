using Gralph.Configuration;

namespace Gralph.Commands;

public sealed class ConfigCommandHandler
{
    public int ExecuteList(string? projectDir)
    {
        Config.Load(projectDir);
        foreach (var entry in Config.List())
        {
            Console.WriteLine(entry);
        }

        return 0;
    }

    public int ExecuteGet(string? projectDir, string? key)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            Console.Error.WriteLine("Usage: gralph config get <key>");
            return 1;
        }

        Config.Load(projectDir);
        if (!Config.Exists(key))
        {
            Console.Error.WriteLine($"Config key not found: {key}");
            return 1;
        }

        Console.WriteLine(Config.Get(key));
        return 0;
    }

    public int ExecuteSet(string? key, string? value)
    {
        if (string.IsNullOrWhiteSpace(key) || string.IsNullOrWhiteSpace(value))
        {
            Console.Error.WriteLine("Usage: gralph config set <key> <value>");
            return 1;
        }

        try
        {
            ConfigFileEditor.SetValue(ConfigPaths.GlobalConfigPath, key, value);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Failed to set config: {key}");
            Console.Error.WriteLine(ex.Message);
            return 1;
        }

        Console.WriteLine($"Updated config: {key}");
        return 0;
    }
}
