using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Gralph.State;

public sealed class StateStore
{
    private readonly string _stateDir;
    private readonly string _stateFile;
    private readonly string _lockFile;
    private readonly string _lockDir;
    private readonly TimeSpan _lockTimeout;
    private readonly JsonSerializerOptions _jsonOptions;

    public StateStore()
        : this(StatePaths.StateDir, StatePaths.StateFile, StatePaths.LockFile, StatePaths.LockDir, TimeSpan.FromSeconds(StatePaths.LockTimeoutSeconds))
    {
    }

    public StateStore(string stateDir, string stateFile, string lockFile, string lockDir, TimeSpan lockTimeout)
    {
        _stateDir = stateDir;
        _stateFile = stateFile;
        _lockFile = lockFile;
        _lockDir = lockDir;
        _lockTimeout = lockTimeout <= TimeSpan.Zero ? TimeSpan.FromSeconds(10) : lockTimeout;
        _jsonOptions = new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            WriteIndented = false
        };
    }

    public SessionState? GetSession(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return null;
        }

        return WithLock(() =>
        {
            var state = LoadStateUnlocked();
            if (!state.Sessions.TryGetValue(name, out var session))
            {
                return null;
            }

            if (string.IsNullOrWhiteSpace(session.Name))
            {
                session.Name = name;
            }

            return session;
        });
    }

    public void SetSession(string name, Action<SessionState> update)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("Session name is required", nameof(name));
        }

        if (update is null)
        {
            throw new ArgumentNullException(nameof(update));
        }

        WithLock(() =>
        {
            var state = LoadStateUnlocked();
            if (!state.Sessions.TryGetValue(name, out var session))
            {
                session = new SessionState { Name = name };
            }

            update(session);

            if (string.IsNullOrWhiteSpace(session.Name))
            {
                session.Name = name;
            }

            state.Sessions[name] = session;
            WriteStateUnlocked(state);
        });
    }

    public void SetSession(SessionState session)
    {
        if (session is null)
        {
            throw new ArgumentNullException(nameof(session));
        }

        var name = session.Name;
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("Session name is required", nameof(session));
        }

        SetSession(name, existing =>
        {
            existing.Name = name;
            existing.Dir = session.Dir;
            existing.TaskFile = session.TaskFile;
            existing.Pid = session.Pid;
            existing.TmuxSession = session.TmuxSession;
            existing.StartedAt = session.StartedAt;
            existing.Iteration = session.Iteration;
            existing.MaxIterations = session.MaxIterations;
            existing.Status = session.Status;
            existing.LastTaskCount = session.LastTaskCount;
            existing.CompletionMarker = session.CompletionMarker;
            existing.LogFile = session.LogFile;
            existing.Backend = session.Backend;
            existing.Model = session.Model;
            existing.Variant = session.Variant;
            existing.Webhook = session.Webhook;
        });
    }

    public IReadOnlyList<SessionState> ListSessions()
    {
        return WithLock(() =>
        {
            var state = LoadStateUnlocked();
            var sessions = new List<SessionState>();
            foreach (var pair in state.Sessions)
            {
                var session = pair.Value;
                if (string.IsNullOrWhiteSpace(session.Name))
                {
                    session.Name = pair.Key;
                }

                sessions.Add(session);
            }

            return sessions;
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
            var state = LoadStateUnlocked();
            if (!state.Sessions.Remove(name))
            {
                return false;
            }

            WriteStateUnlocked(state);
            return true;
        });
    }

    public IReadOnlyList<string> CleanupStale(string mode = "mark")
    {
        return WithLock(() =>
        {
            var state = LoadStateUnlocked();
            var cleaned = new List<string>();
            var remove = string.Equals(mode, "remove", StringComparison.OrdinalIgnoreCase);

            foreach (var pair in state.Sessions.ToList())
            {
                var name = pair.Key;
                var session = pair.Value;
                if (!string.Equals(session.Status, "running", StringComparison.Ordinal))
                {
                    continue;
                }

                if (session.Pid is null or <= 0)
                {
                    continue;
                }

                if (IsProcessAlive(session.Pid.Value))
                {
                    continue;
                }

                cleaned.Add(name);
                if (remove)
                {
                    state.Sessions.Remove(name);
                }
                else
                {
                    session.Status = "stale";
                    state.Sessions[name] = session;
                }
            }

            if (cleaned.Count > 0)
            {
                WriteStateUnlocked(state);
            }

            return cleaned;
        });
    }

    private T WithLock<T>(Func<T> func)
    {
        using var stateLock = AcquireLock();
        return func();
    }

    private void WithLock(Action action)
    {
        using var stateLock = AcquireLock();
        action();
    }

    private StateLock AcquireLock()
    {
        if (!StateLock.TryAcquire(_stateDir, _lockFile, _lockDir, _lockTimeout, out var stateLock))
        {
            throw new IOException($"Failed to acquire state lock within {_lockTimeout.TotalSeconds:N0}s");
        }

        return stateLock!;
    }

    private StateFileModel LoadStateUnlocked()
    {
        EnsureInitializedUnlocked();

        if (!File.Exists(_stateFile))
        {
            return new StateFileModel();
        }

        var json = File.ReadAllText(_stateFile);
        if (string.IsNullOrWhiteSpace(json))
        {
            return new StateFileModel();
        }

        try
        {
            var state = JsonSerializer.Deserialize<StateFileModel>(json, _jsonOptions) ?? new StateFileModel();
            state.Sessions ??= new Dictionary<string, SessionState>(StringComparer.Ordinal);
            foreach (var pair in state.Sessions)
            {
                if (string.IsNullOrWhiteSpace(pair.Value.Name))
                {
                    pair.Value.Name = pair.Key;
                }
            }

            return state;
        }
        catch (JsonException)
        {
            var empty = new StateFileModel();
            WriteStateUnlocked(empty);
            return empty;
        }
    }

    private void EnsureInitializedUnlocked()
    {
        Directory.CreateDirectory(_stateDir);

        if (!File.Exists(_stateFile))
        {
            WriteStateUnlocked(new StateFileModel());
            return;
        }

        try
        {
            using var stream = File.OpenRead(_stateFile);
            JsonSerializer.Deserialize<StateFileModel>(stream, _jsonOptions);
        }
        catch (JsonException)
        {
            WriteStateUnlocked(new StateFileModel());
        }
    }

    private void WriteStateUnlocked(StateFileModel state)
    {
        if (state is null)
        {
            throw new ArgumentNullException(nameof(state));
        }

        Directory.CreateDirectory(_stateDir);

        var json = JsonSerializer.Serialize(state, _jsonOptions);
        if (string.IsNullOrWhiteSpace(json))
        {
            throw new InvalidOperationException("Refusing to write empty state content.");
        }

        var tmpFile = _stateFile + ".tmp." + Environment.ProcessId + "." + Guid.NewGuid().ToString("N");
        File.WriteAllText(tmpFile, json, Encoding.UTF8);
        File.Move(tmpFile, _stateFile, true);
    }

    private static bool IsProcessAlive(int pid)
    {
        try
        {
            var process = Process.GetProcessById(pid);
            return !process.HasExited;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
    }

    private sealed class StateLock : IDisposable
    {
        private readonly FileStream? _lockStream;
        private readonly string? _lockDir;
        private bool _disposed;

        private StateLock(FileStream? lockStream, string? lockDir)
        {
            _lockStream = lockStream;
            _lockDir = lockDir;
        }

        public static bool TryAcquire(string stateDir, string lockFile, string lockDir, TimeSpan timeout, out StateLock? stateLock)
        {
            stateLock = null;
            Directory.CreateDirectory(stateDir);

            if (OperatingSystem.IsMacOS())
            {
                return TryAcquireDirectoryLock(lockDir, timeout, out stateLock);
            }

            var stopwatch = Stopwatch.StartNew();
            var lockUnavailable = false;

            while (stopwatch.Elapsed < timeout)
            {
                try
                {
                    var stream = new FileStream(lockFile, FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);
                    try
                    {
                        stream.Lock(0, 0);
                        stateLock = new StateLock(stream, null);
                        return true;
                    }
                    catch (IOException)
                    {
                        stream.Dispose();
                        lockUnavailable = true;
                    }
                }
                catch (PlatformNotSupportedException)
                {
                    lockUnavailable = true;
                    break;
                }
                catch (NotSupportedException)
                {
                    lockUnavailable = true;
                    break;
                }
                catch (UnauthorizedAccessException)
                {
                    lockUnavailable = true;
                }
                catch (IOException)
                {
                    lockUnavailable = true;
                }

                Thread.Sleep(100);
            }

            if (!lockUnavailable)
            {
                return false;
            }

            return TryAcquireDirectoryLock(lockDir, timeout, out stateLock);
        }

        private static bool TryAcquireDirectoryLock(string lockDir, TimeSpan timeout, out StateLock? stateLock)
        {
            stateLock = null;
            var stopwatch = Stopwatch.StartNew();
            var pidPath = Path.Combine(lockDir, "pid");

            while (stopwatch.Elapsed < timeout)
            {
                if (TryCreatePidFile(lockDir, pidPath))
                {
                    stateLock = new StateLock(null, lockDir);
                    return true;
                }

                if (File.Exists(pidPath))
                {
                    var pidText = File.ReadAllText(pidPath).Trim();
                    if (!int.TryParse(pidText, out var pid) || !IsProcessAlive(pid))
                    {
                        TryRemoveDirectory(lockDir);
                    }
                }

                Thread.Sleep(100);
            }

            return false;
        }

        private static bool TryCreatePidFile(string lockDir, string pidPath)
        {
            try
            {
                if (!Directory.Exists(lockDir))
                {
                    Directory.CreateDirectory(lockDir);
                }

                using var stream = new FileStream(pidPath, FileMode.CreateNew, FileAccess.Write, FileShare.None);
                var pidBytes = Encoding.ASCII.GetBytes(Environment.ProcessId.ToString());
                stream.Write(pidBytes, 0, pidBytes.Length);
                return true;
            }
            catch (IOException)
            {
                return false;
            }
            catch (UnauthorizedAccessException)
            {
                return false;
            }
        }

        private static void TryRemoveDirectory(string path)
        {
            try
            {
                if (Directory.Exists(path))
                {
                    Directory.Delete(path, true);
                }
            }
            catch (IOException)
            {
            }
            catch (UnauthorizedAccessException)
            {
            }
        }

        public void Dispose()
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
            if (_lockStream is not null)
            {
                try
                {
                    if (!OperatingSystem.IsMacOS())
                    {
                        _lockStream.Unlock(0, 0);
                    }
                }
                catch (IOException)
                {
                }

                _lockStream.Dispose();
            }

            if (_lockDir is not null)
            {
                TryRemoveDirectory(_lockDir);
            }
        }
    }
}
