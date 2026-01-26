package backend_test

import (
	"testing"

	"github.com/goosewin/gralph/internal/backend"
	_ "github.com/goosewin/gralph/internal/backend/claude"
)

func TestRegistryLoadsClaude(t *testing.T) {
	registered, ok := backend.Get("claude")
	if !ok {
		t.Fatalf("expected claude backend to be registered")
	}
	if registered == nil {
		t.Fatalf("expected backend instance")
	}
	if len(registered.GetModels()) == 0 {
		t.Fatalf("expected claude models")
	}
}
