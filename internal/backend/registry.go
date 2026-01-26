package backend

import (
	"errors"
	"sort"
	"strings"
	"sync"
)

var (
	ErrBackendNotFound   = errors.New("backend not found")
	ErrBackendRegistered = errors.New("backend already registered")
	ErrBackendInvalid    = errors.New("backend name is required")
)

var (
	registryMu sync.RWMutex
	registry   = map[string]Backend{}
)

// Register adds a backend to the registry by name.
func Register(name string, backend Backend) error {
	if strings.TrimSpace(name) == "" {
		return ErrBackendInvalid
	}
	if backend == nil {
		return errors.New("backend is nil")
	}

	key := strings.ToLower(strings.TrimSpace(name))
	registryMu.Lock()
	defer registryMu.Unlock()

	if _, exists := registry[key]; exists {
		return ErrBackendRegistered
	}

	registry[key] = backend
	return nil
}

// Get returns a backend by name.
func Get(name string) (Backend, bool) {
	key := strings.ToLower(strings.TrimSpace(name))
	if key == "" {
		return nil, false
	}

	registryMu.RLock()
	defer registryMu.RUnlock()

	backend, ok := registry[key]
	return backend, ok
}

// Names returns all registered backend names.
func Names() []string {
	registryMu.RLock()
	defer registryMu.RUnlock()

	names := make([]string, 0, len(registry))
	for name := range registry {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

// DefaultName returns the default backend name.
func DefaultName() string {
	return "claude"
}
