package state

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"syscall"
	"time"
)

// Session represents a stored gralph session with free-form fields.
type Session struct {
	Name   string
	Fields map[string]interface{}
}

// CleanupMode controls how stale sessions are handled.
type CleanupMode string

const (
	CleanupMark   CleanupMode = "mark"
	CleanupRemove CleanupMode = "remove"
)

var ErrLockTimeout = errors.New("state lock timeout")

type stateFile struct {
	Sessions map[string]map[string]interface{} `json:"sessions"`
}

type lockHandle struct {
	method string
	file   *os.File
	dir    string
}

// InitState initializes the state file and directory.
func InitState() error {
	return withLock(func() error {
		return initStateUnlocked()
	})
}

// GetSession returns a session by name.
func GetSession(name string) (Session, bool, error) {
	if name == "" {
		return Session{}, false, errors.New("session name is required")
	}

	var session Session
	var found bool
	err := withLock(func() error {
		if err := initStateUnlocked(); err != nil {
			return err
		}

		state, err := readStateUnlocked()
		if err != nil {
			return err
		}

		fields, ok := state.Sessions[name]
		if !ok {
			found = false
			return nil
		}

		session = Session{
			Name:   name,
			Fields: ensureNameField(name, copyFields(fields)),
		}
		found = true
		return nil
	})

	return session, found, err
}

// SetSession upserts a session with the provided fields.
func SetSession(name string, fields map[string]interface{}) error {
	if name == "" {
		return errors.New("session name is required")
	}

	return withLock(func() error {
		if err := initStateUnlocked(); err != nil {
			return err
		}

		state, err := readStateUnlocked()
		if err != nil {
			return err
		}

		existing := state.Sessions[name]
		if existing == nil {
			existing = map[string]interface{}{}
		}

		existing["name"] = name
		for key, value := range fields {
			existing[key] = value
		}

		state.Sessions[name] = existing
		return writeStateFile(state)
	})
}

// DeleteSession removes a session by name.
func DeleteSession(name string) error {
	if name == "" {
		return errors.New("session name is required")
	}

	return withLock(func() error {
		if err := initStateUnlocked(); err != nil {
			return err
		}

		state, err := readStateUnlocked()
		if err != nil {
			return err
		}

		if _, ok := state.Sessions[name]; !ok {
			return fmt.Errorf("session %q not found", name)
		}

		delete(state.Sessions, name)
		return writeStateFile(state)
	})
}

// ListSessions returns all sessions from state.
func ListSessions() ([]Session, error) {
	var sessions []Session
	err := withLock(func() error {
		if err := initStateUnlocked(); err != nil {
			return err
		}

		state, err := readStateUnlocked()
		if err != nil {
			return err
		}

		sessions = make([]Session, 0, len(state.Sessions))
		for name, fields := range state.Sessions {
			sessions = append(sessions, Session{
				Name:   name,
				Fields: ensureNameField(name, copyFields(fields)),
			})
		}

		return nil
	})

	return sessions, err
}

// CleanupStale marks or removes sessions with dead PIDs.
func CleanupStale(mode CleanupMode) ([]string, error) {
	if mode == "" {
		mode = CleanupMark
	}
	if mode != CleanupMark && mode != CleanupRemove {
		return nil, fmt.Errorf("invalid cleanup mode %q", mode)
	}

	cleaned := []string{}
	err := withLock(func() error {
		if err := initStateUnlocked(); err != nil {
			return err
		}

		state, err := readStateUnlocked()
		if err != nil {
			return err
		}

		changed := false
		for name, fields := range state.Sessions {
			status := stringField(fields, "status")
			if status != "running" {
				continue
			}

			pidValue, ok := intField(fields, "pid")
			if !ok || pidValue <= 0 {
				continue
			}

			if processAlive(pidValue) {
				continue
			}

			cleaned = append(cleaned, name)
			changed = true

			if mode == CleanupRemove {
				delete(state.Sessions, name)
				continue
			}

			fields = ensureNameField(name, fields)
			fields["status"] = "stale"
			state.Sessions[name] = fields
		}

		if !changed {
			return nil
		}

		return writeStateFile(state)
	})

	return cleaned, err
}

func withLock(fn func() error) error {
	handle, err := acquireLock()
	if err != nil {
		return err
	}
	defer handle.release()
	return fn()
}

func acquireLock() (*lockHandle, error) {
	dir := stateDir()
	if dir == "" {
		return nil, errors.New("state directory unavailable")
	}

	if err := os.MkdirAll(dir, 0o755); err != nil {
		return nil, fmt.Errorf("create state dir: %w", err)
	}

	timeout := lockTimeout()
	lockFile := lockFilePath()
	file, err := os.OpenFile(lockFile, os.O_CREATE|os.O_RDWR, 0o644)
	if err == nil {
		err = tryFlock(file, timeout)
		if err == nil {
			return &lockHandle{method: "flock", file: file}, nil
		}

		if !isFlockUnsupported(err) {
			file.Close()
			return nil, err
		}

		file.Close()
	}

	return acquireDirLock(timeout)
}

func (handle *lockHandle) release() {
	if handle == nil {
		return
	}

	if handle.method == "flock" {
		if handle.file != nil {
			_ = syscall.Flock(int(handle.file.Fd()), syscall.LOCK_UN)
			_ = handle.file.Close()
		}
		return
	}

	if handle.method == "mkdir" {
		if handle.dir != "" {
			_ = os.RemoveAll(handle.dir)
		}
	}
}

func tryFlock(file *os.File, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for {
		err := syscall.Flock(int(file.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
		if err == nil {
			return nil
		}

		if errors.Is(err, syscall.EAGAIN) || errors.Is(err, syscall.EWOULDBLOCK) {
			if time.Now().After(deadline) {
				return ErrLockTimeout
			}
			time.Sleep(100 * time.Millisecond)
			continue
		}

		return err
	}
}

func acquireDirLock(timeout time.Duration) (*lockHandle, error) {
	lockDir := lockDirPath()
	if lockDir == "" {
		return nil, errors.New("lock directory unavailable")
	}

	deadline := time.Now().Add(timeout)
	for {
		if err := os.Mkdir(lockDir, 0o755); err == nil {
			_ = os.WriteFile(filepath.Join(lockDir, "pid"), []byte(strconv.Itoa(os.Getpid())), 0o644)
			return &lockHandle{method: "mkdir", dir: lockDir}, nil
		}

		if info, err := os.Stat(lockDir); err == nil && info.IsDir() {
			pid := readPid(filepath.Join(lockDir, "pid"))
			if pid == 0 || !processAlive(pid) {
				_ = os.RemoveAll(lockDir)
			}
		}

		if time.Now().After(deadline) {
			return nil, ErrLockTimeout
		}

		time.Sleep(100 * time.Millisecond)
	}
}

func initStateUnlocked() error {
	dir := stateDir()
	if dir == "" {
		return errors.New("state directory unavailable")
	}

	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("create state dir: %w", err)
	}

	path := stateFilePath()
	if path == "" {
		return errors.New("state file path unavailable")
	}

	if _, err := os.Stat(path); err != nil {
		if os.IsNotExist(err) {
			return writeStateFile(stateFile{Sessions: map[string]map[string]interface{}{}})
		}
		return fmt.Errorf("stat state file: %w", err)
	}

	if _, err := readStateUnlocked(); err != nil {
		return writeStateFile(stateFile{Sessions: map[string]map[string]interface{}{}})
	}

	return nil
}

func readStateUnlocked() (stateFile, error) {
	path := stateFilePath()
	data, err := os.ReadFile(path)
	if err != nil {
		return stateFile{}, fmt.Errorf("read state file: %w", err)
	}

	var state stateFile
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err := decoder.Decode(&state); err != nil {
		return stateFile{}, fmt.Errorf("decode state file: %w", err)
	}

	if state.Sessions == nil {
		state.Sessions = map[string]map[string]interface{}{}
	}

	return state, nil
}

func writeStateFile(state stateFile) error {
	if state.Sessions == nil {
		state.Sessions = map[string]map[string]interface{}{}
	}

	data, err := json.Marshal(state)
	if err != nil {
		return fmt.Errorf("marshal state: %w", err)
	}

	if len(data) == 0 {
		return errors.New("refusing to write empty state")
	}

	path := stateFilePath()
	if path == "" {
		return errors.New("state file path unavailable")
	}

	return writeFileAtomic(path, data)
}

func writeFileAtomic(path string, data []byte) error {
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("create state dir: %w", err)
	}

	tmpFile, err := os.CreateTemp(dir, filepath.Base(path)+".tmp.*")
	if err != nil {
		return fmt.Errorf("create temp file: %w", err)
	}

	tmpName := tmpFile.Name()
	defer func() {
		_ = os.Remove(tmpName)
	}()

	if _, err := tmpFile.Write(data); err != nil {
		_ = tmpFile.Close()
		return fmt.Errorf("write temp file: %w", err)
	}

	if err := tmpFile.Sync(); err != nil {
		_ = tmpFile.Close()
		return fmt.Errorf("sync temp file: %w", err)
	}

	if err := tmpFile.Chmod(0o644); err != nil {
		_ = tmpFile.Close()
		return fmt.Errorf("chmod temp file: %w", err)
	}

	if err := tmpFile.Close(); err != nil {
		return fmt.Errorf("close temp file: %w", err)
	}

	if err := os.Rename(tmpName, path); err != nil {
		return fmt.Errorf("replace state file: %w", err)
	}

	return nil
}

func ensureNameField(name string, fields map[string]interface{}) map[string]interface{} {
	if fields == nil {
		fields = map[string]interface{}{}
	}
	fields["name"] = name
	return fields
}

func copyFields(fields map[string]interface{}) map[string]interface{} {
	if fields == nil {
		return map[string]interface{}{}
	}
	copied := make(map[string]interface{}, len(fields))
	for key, value := range fields {
		copied[key] = value
	}
	return copied
}

func stringField(fields map[string]interface{}, key string) string {
	if fields == nil {
		return ""
	}
	value, ok := fields[key]
	if !ok || value == nil {
		return ""
	}
	switch typed := value.(type) {
	case string:
		return typed
	case json.Number:
		return typed.String()
	default:
		return fmt.Sprint(value)
	}
}

func intField(fields map[string]interface{}, key string) (int, bool) {
	if fields == nil {
		return 0, false
	}
	value, ok := fields[key]
	if !ok || value == nil {
		return 0, false
	}
	switch typed := value.(type) {
	case int:
		return typed, true
	case int64:
		return int(typed), true
	case float64:
		return int(typed), true
	case json.Number:
		parsed, err := typed.Int64()
		if err != nil {
			return 0, false
		}
		return int(parsed), true
	case string:
		parsed, err := strconv.Atoi(typed)
		if err != nil {
			return 0, false
		}
		return parsed, true
	default:
		return 0, false
	}
}

func processAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	err := syscall.Kill(pid, 0)
	return err == nil
}

func readPid(path string) int {
	data, err := os.ReadFile(path)
	if err != nil {
		return 0
	}
	parsed, err := strconv.Atoi(string(bytes.TrimSpace(data)))
	if err != nil {
		return 0
	}
	return parsed
}

func lockTimeout() time.Duration {
	if value := os.Getenv("GRALPH_LOCK_TIMEOUT"); value != "" {
		if parsed, err := strconv.Atoi(value); err == nil && parsed > 0 {
			return time.Duration(parsed) * time.Second
		}
	}
	return 10 * time.Second
}

func stateDir() string {
	if value := os.Getenv("GRALPH_STATE_DIR"); value != "" {
		return value
	}

	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return ""
	}

	return filepath.Join(home, ".config", "gralph")
}

func stateFilePath() string {
	if value := os.Getenv("GRALPH_STATE_FILE"); value != "" {
		return value
	}

	dir := stateDir()
	if dir == "" {
		return ""
	}

	return filepath.Join(dir, "state.json")
}

func lockFilePath() string {
	if value := os.Getenv("GRALPH_LOCK_FILE"); value != "" {
		return value
	}

	dir := stateDir()
	if dir == "" {
		return ""
	}

	return filepath.Join(dir, "state.lock")
}

func lockDirPath() string {
	if value := os.Getenv("GRALPH_LOCK_DIR"); value != "" {
		return value
	}

	lockFile := lockFilePath()
	if lockFile == "" {
		return ""
	}

	return lockFile + ".dir"
}

func isFlockUnsupported(err error) bool {
	return errors.Is(err, syscall.ENOSYS) || errors.Is(err, syscall.EOPNOTSUPP) || errors.Is(err, syscall.ENOTSUP)
}
