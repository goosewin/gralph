using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Gralph.State;
using Xunit;

namespace Gralph.Tests;

public sealed class StateStoreTests
{
    [Fact]
    public void InitCreatesDirectoryAndStateFile()
    {
        using var temp = new TempDirectory();
        var stateDir = System.IO.Path.Combine(temp.Path, "state");
        var stateFile = System.IO.Path.Combine(stateDir, "state.json");
        var lockFile = System.IO.Path.Combine(stateDir, "state.lock");
        var lockDir = lockFile + ".dir";

        var store = new StateStore(stateDir, stateFile, lockFile, lockDir, TimeSpan.FromSeconds(2));
        _ = store.ListSessions();

        Assert.True(Directory.Exists(stateDir));
        Assert.True(File.Exists(stateFile));
    }

    [Fact]
    public void InitCreatesValidJsonWithEmptySessions()
    {
        using var temp = new TempDirectory();
        var stateDir = System.IO.Path.Combine(temp.Path, "state");
        var stateFile = System.IO.Path.Combine(stateDir, "state.json");
        var lockFile = System.IO.Path.Combine(stateDir, "state.lock");
        var lockDir = lockFile + ".dir";

        var store = new StateStore(stateDir, stateFile, lockFile, lockDir, TimeSpan.FromSeconds(2));
        _ = store.ListSessions();

        var json = File.ReadAllText(stateFile);
        using var document = JsonDocument.Parse(json);
        var sessions = document.RootElement.GetProperty("sessions");
        Assert.Equal(JsonValueKind.Object, sessions.ValueKind);
        Assert.Empty(sessions.EnumerateObject());
    }

    [Fact]
    public void SetSessionCreatesNewSession()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        store.SetSession("test-session", session =>
        {
            session.Dir = "/tmp/test";
            session.Status = "running";
        });

        var sessionState = store.GetSession("test-session");
        Assert.NotNull(sessionState);
        Assert.Equal("running", sessionState!.Status);
    }

    [Fact]
    public void SetSessionUpdatesExistingSession()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        store.SetSession("update-test", session =>
        {
            session.Status = "running";
            session.Iteration = 1;
        });
        store.SetSession("update-test", session =>
        {
            session.Iteration = 5;
        });

        var sessionState = store.GetSession("update-test");
        Assert.NotNull(sessionState);
        Assert.Equal(5, sessionState!.Iteration);
        Assert.Equal("running", sessionState.Status);
    }

    [Fact]
    public void GetSessionReturnsNullForMissingSession()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        var sessionState = store.GetSession("nonexistent");
        Assert.Null(sessionState);
    }

    [Fact]
    public void ListSessionsReturnsAllSessions()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        store.SetSession("session-a", session => session.Status = "running");
        store.SetSession("session-b", session => session.Status = "complete");

        var sessions = store.ListSessions();
        Assert.Equal(2, sessions.Count);
        Assert.Contains(sessions, session => session.Name == "session-a");
        Assert.Contains(sessions, session => session.Name == "session-b");
    }

    [Fact]
    public void ListSessionsReturnsEmptyWhenNoSessions()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        var sessions = store.ListSessions();
        Assert.Empty(sessions);
    }

    [Fact]
    public void DeleteSessionRemovesSession()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        store.SetSession("delete-test", session => session.Status = "running");

        Assert.True(store.DeleteSession("delete-test"));
        Assert.Null(store.GetSession("delete-test"));
    }

    [Fact]
    public void DeleteSessionReturnsFalseForMissingSession()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out _);

        Assert.False(store.DeleteSession("nonexistent"));
    }

    [Fact]
    public void StateFileStoresNumericValuesAsNumbers()
    {
        using var temp = new TempDirectory();
        var store = CreateStore(temp, out var stateFile);

        store.SetSession("int-test", session =>
        {
            session.Iteration = 42;
            session.MaxIterations = 100;
        });

        var json = File.ReadAllText(stateFile);
        using var document = JsonDocument.Parse(json);
        var session = document.RootElement.GetProperty("sessions").GetProperty("int-test");
        Assert.Equal(JsonValueKind.Number, session.GetProperty("iteration").ValueKind);
        Assert.Equal(42, session.GetProperty("iteration").GetInt32());
        Assert.Equal(JsonValueKind.Number, session.GetProperty("max_iterations").ValueKind);
        Assert.Equal(100, session.GetProperty("max_iterations").GetInt32());
    }

    private static StateStore CreateStore(TempDirectory temp, out string stateFile)
    {
        var stateDir = System.IO.Path.Combine(temp.Path, "state");
        stateFile = System.IO.Path.Combine(stateDir, "state.json");
        var lockFile = System.IO.Path.Combine(stateDir, "state.lock");
        var lockDir = lockFile + ".dir";
        return new StateStore(stateDir, stateFile, lockFile, lockDir, TimeSpan.FromSeconds(2));
    }
}
