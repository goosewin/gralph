using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization.Metadata;
using System.Threading;

namespace Gralph.State;

public enum StaleCleanupMode
{
    Mark,
    Remove
}

public sealed class StateStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        TypeInfoResolver = new DefaultJsonTypeInfoResolver()
    };

    private readonly StatePaths _paths;
    private readonly IProcessInspector _processInspector;

    public StateStore(StatePaths paths, IProcessInspector? processInspector = null)
    {
        _paths = paths;
        _processInspector = processInspector ?? new ProcessInspector();
    }

    public void Init()
    {
        using var handle = AcquireLock();
        InitializeStateFile();
    }

    public JsonObject? GetSession(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return null;
        }

        return WithLock(() =>
        {
            var state = LoadState();
            if (!state.Sessions.TryGetPropertyValue(name, out var sessionNode) || sessionNode is not JsonObject session)
            {
                return null;
            }

            return (JsonObject)session.DeepClone();
        });
    }

    public void SetSession(string name, IDictionary<string, object?> values)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("Session name is required.", nameof(name));
        }

        WithLock(() =>
        {
            var state = LoadState();
            var session = state.Sessions.TryGetPropertyValue(name, out var sessionNode) && sessionNode is JsonObject existing
                ? existing
                : new JsonObject();

            session["name"] = name;
            foreach (var (key, value) in values)
            {
                if (string.IsNullOrWhiteSpace(key))
                {
                    continue;
                }

                session[key] = ToJsonNode(value);
            }

            state.Sessions[name] = session;
            SaveState(state.Root);
        });
    }

    public IReadOnlyList<JsonObject> ListSessions()
    {
        return WithLock(() =>
        {
            var state = LoadState();
            var results = new List<JsonObject>();
            foreach (var (name, node) in state.Sessions)
            {
                if (node is not JsonObject session)
                {
                    continue;
                }

                if (!session.ContainsKey("name"))
                {
                    session = (JsonObject)session.DeepClone();
                    session["name"] = name;
                }

                results.Add((JsonObject)session.DeepClone());
            }

            return results;
        });
    }

    public bool DeleteSession(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return false;
        }

        return WithLock(() =>
        {
            var state = LoadState();
            if (!state.Sessions.Remove(name))
            {
                return false;
            }

            SaveState(state.Root);
            return true;
        });
    }

    public IReadOnlyList<string> CleanupStale(StaleCleanupMode mode = StaleCleanupMode.Mark)
    {
        return WithLock(() =>
        {
            var state = LoadState();
            var cleaned = new List<string>();
            var names = new List<string>();
            foreach (var (name, _) in state.Sessions)
            {
                names.Add(name);
            }

            foreach (var name in names)
            {
                if (!state.Sessions.TryGetPropertyValue(name, out var node) || node is not JsonObject session)
                {
                    continue;
                }

                var status = GetString(session, "status");
                if (!string.Equals(status, "running", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (!TryGetInt(session, "pid", out var pid))
                {
                    continue;
                }

                if (_processInspector.IsAlive(pid))
                {
                    continue;
                }

                cleaned.Add(name);

                if (mode == StaleCleanupMode.Remove)
                {
                    state.Sessions.Remove(name);
                }
                else
                {
                    session["status"] = "stale";
                }
            }

            if (cleaned.Count > 0)
            {
                SaveState(state.Root);
            }

            return cleaned;
        });
    }

    private T WithLock<T>(Func<T> action)
    {
        using var handle = AcquireLock();
        return action();
    }

    private void WithLock(Action action)
    {
        using var handle = AcquireLock();
        action();
    }

    private FileStream AcquireLock()
    {
        Directory.CreateDirectory(_paths.StateDir);
        var deadline = DateTime.UtcNow + _paths.LockTimeout;

        while (true)
        {
            try
            {
                return new FileStream(_paths.LockFilePath, FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);
            }
            catch (IOException)
            {
                if (DateTime.UtcNow >= deadline)
                {
                    throw new TimeoutException($"Failed to acquire state lock within {_paths.LockTimeout.TotalSeconds:0.##}s.");
                }

                Thread.Sleep(100);
            }
        }
    }

    private void InitializeStateFile()
    {
        Directory.CreateDirectory(_paths.StateDir);
        if (!File.Exists(_paths.StateFilePath))
        {
            SaveState(CreateEmptyState());
            return;
        }

        try
        {
            LoadState();
        }
        catch (JsonException)
        {
            SaveState(CreateEmptyState());
        }
    }

    private StateSnapshot LoadState()
    {
        if (!File.Exists(_paths.StateFilePath))
        {
            SaveState(CreateEmptyState());
        }

        var content = File.ReadAllText(_paths.StateFilePath);
        if (string.IsNullOrWhiteSpace(content))
        {
            var emptyState = CreateEmptyState();
            SaveState(emptyState);
            return new StateSnapshot(emptyState);
        }

        var node = JsonNode.Parse(content) as JsonObject ?? CreateEmptyState();
        if (node["sessions"] is not JsonObject sessions)
        {
            sessions = new JsonObject();
            node["sessions"] = sessions;
        }

        return new StateSnapshot(node, sessions);
    }

    private void SaveState(JsonObject root)
    {
        if (root is null)
        {
            throw new ArgumentNullException(nameof(root));
        }

        Directory.CreateDirectory(_paths.StateDir);
        var json = root.ToJsonString(JsonOptions);
        if (string.IsNullOrWhiteSpace(json))
        {
            throw new InvalidOperationException("Refusing to write empty state content.");
        }

        var tempFile = $"{_paths.StateFilePath}.tmp.{Environment.ProcessId}.{Guid.NewGuid():N}";
        File.WriteAllText(tempFile, json);

        try
        {
            if (File.Exists(_paths.StateFilePath))
            {
                File.Replace(tempFile, _paths.StateFilePath, null);
            }
            else
            {
                File.Move(tempFile, _paths.StateFilePath);
            }
        }
        catch
        {
            if (File.Exists(tempFile))
            {
                File.Delete(tempFile);
            }
            throw;
        }
    }

    private static JsonObject CreateEmptyState()
    {
        return new JsonObject
        {
            ["sessions"] = new JsonObject()
        };
    }

    private static JsonNode? ToJsonNode(object? value)
    {
        if (value is null)
        {
            return null;
        }

        if (value is JsonNode node)
        {
            return node;
        }

        return JsonValue.Create(value);
    }

    private static string? GetString(JsonObject session, string key)
    {
        if (!session.TryGetPropertyValue(key, out var node) || node is null)
        {
            return null;
        }

        if (node is JsonValue value && value.TryGetValue<string>(out var stringValue))
        {
            return stringValue;
        }

        return node.ToString();
    }

    private static bool TryGetInt(JsonObject session, string key, out int result)
    {
        result = 0;
        if (!session.TryGetPropertyValue(key, out var node) || node is null)
        {
            return false;
        }

        if (node is JsonValue value)
        {
            if (value.TryGetValue<int>(out var intValue))
            {
                result = intValue;
                return true;
            }

            if (value.TryGetValue<long>(out var longValue))
            {
                if (longValue is > int.MinValue and < int.MaxValue)
                {
                    result = (int)longValue;
                    return true;
                }
            }

            if (value.TryGetValue<string>(out var stringValue)
                && int.TryParse(stringValue, out var parsed))
            {
                result = parsed;
                return true;
            }
        }

        return false;
    }

    private readonly record struct StateSnapshot(JsonObject Root, JsonObject Sessions)
    {
        public StateSnapshot(JsonObject root)
            : this(root, root["sessions"] as JsonObject ?? new JsonObject())
        {
        }
    }
}
