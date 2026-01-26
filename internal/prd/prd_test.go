package prd

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestGetTaskBlocks(t *testing.T) {
	tempDir := t.TempDir()
	content := strings.Join([]string{
		"# Project Requirements Document",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
		"### Task GO-12",
		"- **ID** GO-12",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-11",
		"- [ ] GO-12 Implement PRD check and create commands",
		"---",
	}, "\n")

	path := filepath.Join(tempDir, "PRD.md")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	blocks, err := GetTaskBlocks(path)
	if err != nil {
		t.Fatalf("get task blocks: %v", err)
	}
	if len(blocks) != 2 {
		t.Fatalf("expected 2 blocks, got %d", len(blocks))
	}
	if !strings.Contains(blocks[0], "Task GO-11") {
		t.Fatalf("expected GO-11 block")
	}
	if !strings.Contains(blocks[1], "Task GO-12") {
		t.Fatalf("expected GO-12 block")
	}
}

func TestValidateFileSuccess(t *testing.T) {
	tempDir := t.TempDir()
	if err := os.WriteFile(filepath.Join(tempDir, "README.md"), []byte("hi"), 0o644); err != nil {
		t.Fatalf("write README: %v", err)
	}
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	if err := ValidateFile(path, nil); err != nil {
		t.Fatalf("expected valid PRD, got %v", err)
	}
}

func TestValidateFileMissingField(t *testing.T) {
	tempDir := t.TempDir()
	if err := os.WriteFile(filepath.Join(tempDir, "README.md"), []byte("hi"), 0o644); err != nil {
		t.Fatalf("write README: %v", err)
	}
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	err := ValidateFile(path, nil)
	if err == nil || !strings.Contains(err.Error(), "Missing required field: Dependencies") {
		t.Fatalf("expected missing dependencies error, got %v", err)
	}
}

func TestValidateFileMultipleUnchecked(t *testing.T) {
	tempDir := t.TempDir()
	if err := os.WriteFile(filepath.Join(tempDir, "README.md"), []byte("hi"), 0o644); err != nil {
		t.Fatalf("write README: %v", err)
	}
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] first",
		"- [ ] second",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	err := ValidateFile(path, nil)
	if err == nil || !strings.Contains(err.Error(), "Multiple unchecked task lines") {
		t.Fatalf("expected multiple unchecked error, got %v", err)
	}
}

func TestValidateFileStrayUnchecked(t *testing.T) {
	tempDir := t.TempDir()
	if err := os.WriteFile(filepath.Join(tempDir, "README.md"), []byte("hi"), 0o644); err != nil {
		t.Fatalf("write README: %v", err)
	}
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"- [ ] stray",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	err := ValidateFile(path, nil)
	if err == nil || !strings.Contains(err.Error(), "Unchecked task line outside task block") {
		t.Fatalf("expected stray unchecked error, got %v", err)
	}
}

func TestValidateFileOpenQuestions(t *testing.T) {
	tempDir := t.TempDir()
	if err := os.WriteFile(filepath.Join(tempDir, "README.md"), []byte("hi"), 0o644); err != nil {
		t.Fatalf("write README: %v", err)
	}
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"",
		"## Open Questions",
		"- Something",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `README.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	err := ValidateFile(path, nil)
	if err == nil || !strings.Contains(err.Error(), "Open Questions section is not allowed") {
		t.Fatalf("expected open questions error, got %v", err)
	}
}

func TestValidateFileContextMissing(t *testing.T) {
	tempDir := t.TempDir()
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `missing.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	err := ValidateFile(path, nil)
	if err == nil || !strings.Contains(err.Error(), "Context Bundle path not found: missing.md") {
		t.Fatalf("expected missing context error, got %v", err)
	}
}

func TestValidateFileAllowMissingContext(t *testing.T) {
	tempDir := t.TempDir()
	path := filepath.Join(tempDir, "PRD.md")
	content := strings.Join([]string{
		"# PRD",
		"",
		"### Task GO-11",
		"- **ID** GO-11",
		"- **Context Bundle** `missing.md`",
		"- **DoD** Done",
		"- **Checklist**",
		"  * one",
		"- **Dependencies** GO-1",
		"- [ ] GO-11 Implement PRD utilities package",
		"",
	}, "\n")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write PRD: %v", err)
	}

	err := ValidateFile(path, &ValidateOptions{AllowMissingContext: true})
	if err != nil {
		t.Fatalf("expected allow missing context to pass, got %v", err)
	}
}
