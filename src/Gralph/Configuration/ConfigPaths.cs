namespace Gralph.Configuration;

public static class ConfigPaths
{
    public static string ConfigDir
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_CONFIG_DIR");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            var home = GetHomeDirectory();
            return Path.Combine(home, ".config", "gralph");
        }
    }

    public static string GlobalConfigPath
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_GLOBAL_CONFIG");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            return Path.Combine(ConfigDir, "config.yaml");
        }
    }

    public static string DefaultConfigPath
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_DEFAULT_CONFIG");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            var dir = ResolveDefaultConfigDir();
            return Path.Combine(dir, "default.yaml");
        }
    }

    public static string ProjectConfigName
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_PROJECT_CONFIG_NAME");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            return ".gralph.yaml";
        }
    }

    public static string? GetProjectConfigPath(string? projectDir)
    {
        if (string.IsNullOrWhiteSpace(projectDir))
        {
            return null;
        }

        if (!Directory.Exists(projectDir))
        {
            return null;
        }

        return Path.Combine(projectDir, ProjectConfigName);
    }

    private static string ResolveDefaultConfigDir()
    {
        var baseDir = AppContext.BaseDirectory;
        var candidate = FindAncestorConfigDir(baseDir);
        if (candidate is not null)
        {
            return candidate;
        }

        var home = GetHomeDirectory();
        var altDir = Path.Combine(home, ".config", "gralph", "config");
        if (Directory.Exists(altDir))
        {
            return altDir;
        }

        return Path.Combine(baseDir, "config");
    }

    private static string? FindAncestorConfigDir(string startDirectory)
    {
        var current = new DirectoryInfo(startDirectory);
        while (current is not null)
        {
            var candidate = Path.Combine(current.FullName, "config", "default.yaml");
            if (File.Exists(candidate))
            {
                return Path.Combine(current.FullName, "config");
            }

            current = current.Parent;
        }

        return null;
    }

    private static string GetHomeDirectory()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (!string.IsNullOrWhiteSpace(home))
        {
            return home;
        }

        return Environment.GetEnvironmentVariable("HOME") ?? ".";
    }
}
