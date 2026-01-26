package state

import (
	"encoding/json"
	"os"
	"path/filepath"
	"syscall"
	"testing"
)

func TestInitStateCreatesAndRepairsFile(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("GRALPH_STATE_DIR", tempDir)
	t.Setenv("GRALPH_STATE_FILE", filepath.Join(tempDir, "state.json"))

	if err := InitState(); err != nil {
		t.Fatalf("init state: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(tempDir, "state.json"))
	if err != nil {
		t.Fatalf("read state file: %v", err)
	}

	var state stateFile
	if err := json.Unmarshal(data, &state); err != nil {
		t.Fatalf("unmarshal state: %v", err)
	}

	if len(state.Sessions) != 0 {
		t.Fatalf("expected empty sessions, got %d", len(state.Sessions))
	}

	if err := os.WriteFile(filepath.Join(tempDir, "state.json"), []byte("{invalid"), 0o644); err != nil {
		t.Fatalf("write invalid state: %v", err)
	}

	if err := InitState(); err != nil {
		t.Fatalf("reinit state: %v", err)
	}

	data, err = os.ReadFile(filepath.Join(tempDir, "state.json"))
	if err != nil {
		t.Fatalf("read state file after repair: %v", err)
	}

	if err := json.Unmarshal(data, &state); err != nil {
		t.Fatalf("unmarshal repaired state: %v", err)
	}

	if len(state.Sessions) != 0 {
		t.Fatalf("expected empty sessions after repair, got %d", len(state.Sessions))
	}
}

func TestSetGetListDeleteSession(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("GRALPH_STATE_DIR", tempDir)
	t.Setenv("GRALPH_STATE_FILE", filepath.Join(tempDir, "state.json"))

	fields := map[string]interface{}{
		"status": "running",
		"pid":    123,
	}

	if err := SetSession("demo", fields); err != nil {
		t.Fatalf("set session: %v", err)
	}

	session, found, err := GetSession("demo")
	if err != nil {
		t.Fatalf("get session: %v", err)
	}
	if !found {
		t.Fatalf("expected session to exist")
	}
	if session.Fields["status"] != "running" {
		t.Fatalf("expected status running, got %v", session.Fields["status"])
	}

	sessions, err := ListSessions()
	if err != nil {
		t.Fatalf("list sessions: %v", err)
	}
	if len(sessions) != 1 {
		t.Fatalf("expected 1 session, got %d", len(sessions))
	}

	if err := DeleteSession("demo"); err != nil {
		t.Fatalf("delete session: %v", err)
	}

	_, found, err = GetSession("demo")
	if err != nil {
		t.Fatalf("get session after delete: %v", err)
	}
	if found {
		t.Fatalf("expected session to be deleted")
	}
}

func TestCleanupStaleSessions(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("GRALPH_STATE_DIR", tempDir)
	t.Setenv("GRALPH_STATE_FILE", filepath.Join(tempDir, "state.json"))

	stalePID := findUnusedPID(t)
	if stalePID == 0 {
		t.Skip("unable to find unused PID")
	}

	if err := SetSession("alive", map[string]interface{}{"status": "running", "pid": os.Getpid()}); err != nil {
		t.Fatalf("set alive session: %v", err)
	}
	if err := SetSession("stale", map[string]interface{}{"status": "running", "pid": stalePID}); err != nil {
		t.Fatalf("set stale session: %v", err)
	}

	cleaned, err := CleanupStale("")
	if err != nil {
		t.Fatalf("cleanup stale: %v", err)
	}
	if len(cleaned) != 1 || cleaned[0] != "stale" {
		t.Fatalf("expected stale cleaned, got %v", cleaned)
	}

	session, found, err := GetSession("stale")
	if err != nil {
		t.Fatalf("get stale session: %v", err)
	}
	if !found {
		t.Fatalf("expected stale session to remain")
	}
	if session.Fields["status"] != "stale" {
		t.Fatalf("expected status stale, got %v", session.Fields["status"])
	}

	if err := SetSession("stale-remove", map[string]interface{}{"status": "running", "pid": stalePID}); err != nil {
		t.Fatalf("set stale-remove session: %v", err)
	}

	cleaned, err = CleanupStale(CleanupRemove)
	if err != nil {
		t.Fatalf("cleanup remove: %v", err)
	}
	if len(cleaned) != 1 || cleaned[0] != "stale-remove" {
		t.Fatalf("expected stale-remove cleaned, got %v", cleaned)
	}

	_, found, err = GetSession("stale-remove")
	if err != nil {
		t.Fatalf("get stale-remove session: %v", err)
	}
	if found {
		t.Fatalf("expected stale-remove session to be deleted")
	}
}

func findUnusedPID(t *testing.T) int {
	t.Helper()
	for pid := 50000; pid < 60000; pid++ {
		err := syscall.Kill(pid, 0)
		if err == syscall.ESRCH {
			return pid
		}
	}
	return 0
}
