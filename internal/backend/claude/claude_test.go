package claude

import (
	"os"
	"path/filepath"
	"testing"
)

func TestParseTextExtractsResult(t *testing.T) {
	tempDir := t.TempDir()
	path := filepath.Join(tempDir, "stream.jsonl")

	contents := "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Hello\"}]}}\n" +
		"{\"type\":\"result\",\"result\":\"final result\"}\n"

	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
		t.Fatalf("write stream file: %v", err)
	}

	backend := New()
	text, err := backend.ParseText(path)
	if err != nil {
		t.Fatalf("parse text: %v", err)
	}
	if text != "final result" {
		t.Fatalf("expected result %q, got %q", "final result", text)
	}
}
