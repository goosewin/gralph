package opencode

import (
	"os"
	"path/filepath"
	"testing"
)

func TestParseTextReturnsContents(t *testing.T) {
	tempDir := t.TempDir()
	path := filepath.Join(tempDir, "output.txt")
	contents := "hello from opencode\n"

	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
		t.Fatalf("write output file: %v", err)
	}

	backend := New()
	text, err := backend.ParseText(path)
	if err != nil {
		t.Fatalf("parse text: %v", err)
	}
	if text != contents {
		t.Fatalf("expected %q, got %q", contents, text)
	}
}
