package backend_test

import (
	"testing"

	"github.com/goosewin/gralph/internal/backend"
	_ "github.com/goosewin/gralph/internal/backend/claude"
	_ "github.com/goosewin/gralph/internal/backend/codex"
	_ "github.com/goosewin/gralph/internal/backend/gemini"
	_ "github.com/goosewin/gralph/internal/backend/opencode"
)

func TestRegistryLoadsBackends(t *testing.T) {
	backends := []string{"claude", "opencode", "gemini", "codex"}
	for _, name := range backends {
		registered, ok := backend.Get(name)
		if !ok {
			t.Fatalf("expected %s backend to be registered", name)
		}
		if registered == nil {
			t.Fatalf("expected backend instance for %s", name)
		}
		if len(registered.GetModels()) == 0 {
			t.Fatalf("expected %s models", name)
		}
	}
}
