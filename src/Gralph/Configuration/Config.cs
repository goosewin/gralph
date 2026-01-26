namespace Gralph.Configuration;

public static class Config
{
    private static readonly ConfigStore Store = new();

    public static ConfigStore Current => Store;

    public static IReadOnlyDictionary<string, string> Cache => Store.Cache;

    public static void Load(string? projectDir = null) => Store.Load(projectDir);

    public static string Get(string key, string? defaultValue = null) => Store.Get(key, defaultValue);

    public static bool Exists(string key) => Store.Exists(key);

    public static IEnumerable<string> List() => Store.List();
}
