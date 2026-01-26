namespace Gralph.State;

public static class StatePaths
{
    public static string StateDir
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_STATE_DIR");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            var home = GetHomeDirectory();
            return Path.Combine(home, ".config", "gralph");
        }
    }

    public static string StateFile
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_STATE_FILE");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            return Path.Combine(StateDir, "state.json");
        }
    }

    public static string LockFile
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_LOCK_FILE");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            return Path.Combine(StateDir, "state.lock");
        }
    }

    public static string LockDir
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_LOCK_DIR");
            if (!string.IsNullOrWhiteSpace(env))
            {
                return env;
            }

            return LockFile + ".dir";
        }
    }

    public static int LockTimeoutSeconds
    {
        get
        {
            var env = Environment.GetEnvironmentVariable("GRALPH_LOCK_TIMEOUT");
            if (!string.IsNullOrWhiteSpace(env) && int.TryParse(env, out var value) && value > 0)
            {
                return value;
            }

            return 10;
        }
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
