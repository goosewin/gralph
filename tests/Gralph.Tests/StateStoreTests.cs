using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using Gralph.State;
using Xunit;

namespace Gralph.Tests;

public sealed class StateStoreTests
{
    [Fact]
    public void Init_CreatesStateFileWithSessions()
    {
        using var temp = new TempDirectory();
        var stateDir = temp.CreateSubdirectory("state");
        var stateFile = Path.Combine(stateDir, "state.json");
        var lockFile = Path.Combine(stateDir, "state.lock");

        using var env = new EnvironmentScope(
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT");
        env.Set("GRALPH_STATE_DIR", stateDir);
        env.Set("GRALPH_STATE_FILE", stateFile);
        env.Set("GRALPH_LOCK_FILE", lockFile);
        env.Set("GRALPH_LOCK_TIMEOUT", "2");

        var store = new StateStore(StatePaths.FromEnvironment(), new StubProcessInspector());
        store.Init();

        var node = JsonNode.Parse(File.ReadAllText(stateFile)) as JsonObject;
        Assert.NotNull(node);
        Assert.True(node!.ContainsKey("sessions"));
    }

    [Fact]
    public void SetSession_StoresValues()
    {
        using var temp = new TempDirectory();
        var stateDir = temp.CreateSubdirectory("state");
        var stateFile = Path.Combine(stateDir, "state.json");
        var lockFile = Path.Combine(stateDir, "state.lock");

        using var env = new EnvironmentScope(
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT");
        env.Set("GRALPH_STATE_DIR", stateDir);
        env.Set("GRALPH_STATE_FILE", stateFile);
        env.Set("GRALPH_LOCK_FILE", lockFile);
        env.Set("GRALPH_LOCK_TIMEOUT", "2");

        var store = new StateStore(StatePaths.FromEnvironment(), new StubProcessInspector());
        store.Init();

        store.SetSession("alpha", new Dictionary<string, object?>
        {
            ["status"] = "running",
            ["pid"] = 1234
        });

        var session = store.GetSession("alpha");
        Assert.NotNull(session);
        Assert.Equal("running", session!["status"]!.GetValue<string>());
    }

    [Fact]
    public void CleanupStale_MarksSession()
    {
        using var temp = new TempDirectory();
        var stateDir = temp.CreateSubdirectory("state");
        var stateFile = Path.Combine(stateDir, "state.json");
        var lockFile = Path.Combine(stateDir, "state.lock");

        using var env = new EnvironmentScope(
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT");
        env.Set("GRALPH_STATE_DIR", stateDir);
        env.Set("GRALPH_STATE_FILE", stateFile);
        env.Set("GRALPH_LOCK_FILE", lockFile);
        env.Set("GRALPH_LOCK_TIMEOUT", "2");

        var store = new StateStore(StatePaths.FromEnvironment(), new StubProcessInspector(alive: false));
        store.Init();

        store.SetSession("stale", new Dictionary<string, object?>
        {
            ["status"] = "running",
            ["pid"] = 999
        });

        var cleaned = store.CleanupStale();
        Assert.Single(cleaned);
        Assert.Equal("stale", cleaned[0]);

        var session = store.GetSession("stale");
        Assert.Equal("stale", session!["status"]!.GetValue<string>());
    }

    [Fact]
    public void CleanupStale_RemovesSession()
    {
        using var temp = new TempDirectory();
        var stateDir = temp.CreateSubdirectory("state");
        var stateFile = Path.Combine(stateDir, "state.json");
        var lockFile = Path.Combine(stateDir, "state.lock");

        using var env = new EnvironmentScope(
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT");
        env.Set("GRALPH_STATE_DIR", stateDir);
        env.Set("GRALPH_STATE_FILE", stateFile);
        env.Set("GRALPH_LOCK_FILE", lockFile);
        env.Set("GRALPH_LOCK_TIMEOUT", "2");

        var store = new StateStore(StatePaths.FromEnvironment(), new StubProcessInspector(alive: false));
        store.Init();

        store.SetSession("remove", new Dictionary<string, object?>
        {
            ["status"] = "running",
            ["pid"] = 555
        });

        var cleaned = store.CleanupStale(StaleCleanupMode.Remove);
        Assert.Single(cleaned);
        Assert.Equal("remove", cleaned[0]);
        Assert.Null(store.GetSession("remove"));
    }

    [Fact]
    public void Init_ThrowsWhenLockIsHeld()
    {
        using var temp = new TempDirectory();
        var stateDir = temp.CreateSubdirectory("state");
        var stateFile = Path.Combine(stateDir, "state.json");
        var lockFile = Path.Combine(stateDir, "state.lock");

        using var env = new EnvironmentScope(
            "GRALPH_STATE_DIR",
            "GRALPH_STATE_FILE",
            "GRALPH_LOCK_FILE",
            "GRALPH_LOCK_TIMEOUT");
        env.Set("GRALPH_STATE_DIR", stateDir);
        env.Set("GRALPH_STATE_FILE", stateFile);
        env.Set("GRALPH_LOCK_FILE", lockFile);
        env.Set("GRALPH_LOCK_TIMEOUT", "0.1");

        Directory.CreateDirectory(stateDir);
        using var handle = new FileStream(lockFile, FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);

        var store = new StateStore(StatePaths.FromEnvironment(), new StubProcessInspector());
        Assert.Throws<TimeoutException>(() => store.Init());
    }

    private sealed class StubProcessInspector : IProcessInspector
    {
        private readonly bool _alive;

        public StubProcessInspector(bool alive = true)
        {
            _alive = alive;
        }

        public bool IsAlive(int pid)
        {
            return _alive;
        }
    }
}
