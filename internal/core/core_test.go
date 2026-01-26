package core

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/goosewin/gralph/internal/backend"
)

type fakeBackend struct {
	projectDir string
	taskFile   string
	outputText string
	markDone   bool
	calls      int
}

func (f *fakeBackend) CheckInstalled() error {
	return nil
}

func (f *fakeBackend) GetModels() []string {
	return []string{"fake"}
}

func (f *fakeBackend) RunIteration(_ context.Context, opts backend.IterationOptions) error {
	f.calls++
	if f.markDone && f.calls == 1 {
		path := filepath.Join(f.projectDir, f.taskFile)
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		updated := strings.Replace(string(data), "- [ ]", "- [x]", 1)
		if err := os.WriteFile(path, []byte(updated), 0o644); err != nil {
			return err
		}
	}
	return os.WriteFile(opts.OutputFile, []byte(f.outputText), 0o644)
}

func (f *fakeBackend) ParseText(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

func TestRunLoopCompletes(t *testing.T) {
	tempDir := t.TempDir()
	taskFile := "PRD.md"
	contents := "### Task T-1\n- [ ] Do thing\n"
	if err := os.WriteFile(filepath.Join(tempDir, taskFile), []byte(contents), 0o644); err != nil {
		t.Fatalf("write task file: %v", err)
	}

	backend := &fakeBackend{
		projectDir: tempDir,
		taskFile:   taskFile,
		outputText: "done\n<promise>COMPLETE</promise>\n",
		markDone:   true,
	}

	result, err := RunLoop(context.Background(), LoopOptions{
		ProjectDir:       tempDir,
		TaskFile:         taskFile,
		MaxIterations:    2,
		CompletionMarker: "COMPLETE",
		Backend:          backend,
		SleepDelay:       -1,
	})
	if err != nil {
		t.Fatalf("run loop: %v", err)
	}
	if !result.Completed {
		t.Fatalf("expected completed loop")
	}
	if result.Iterations != 1 {
		t.Fatalf("expected 1 iteration, got %d", result.Iterations)
	}
}

func TestRunLoopMaxIterations(t *testing.T) {
	tempDir := t.TempDir()
	taskFile := "PRD.md"
	contents := "- [ ] Do thing\n"
	if err := os.WriteFile(filepath.Join(tempDir, taskFile), []byte(contents), 0o644); err != nil {
		t.Fatalf("write task file: %v", err)
	}

	backend := &fakeBackend{
		projectDir: tempDir,
		taskFile:   taskFile,
		outputText: "still working\n",
		markDone:   false,
	}

	_, err := RunLoop(context.Background(), LoopOptions{
		ProjectDir:       tempDir,
		TaskFile:         taskFile,
		MaxIterations:    1,
		CompletionMarker: "COMPLETE",
		Backend:          backend,
		SleepDelay:       -1,
	})
	if !errors.Is(err, ErrMaxIterations) {
		t.Fatalf("expected ErrMaxIterations, got %v", err)
	}
}

func TestGetNextUncheckedTaskBlock(t *testing.T) {
	tempDir := t.TempDir()
	taskFile := filepath.Join(tempDir, "PRD.md")
	contents := "### Task A\n- [x] Done\n\n### Task B\n- [ ] Pending\n---\n"
	if err := os.WriteFile(taskFile, []byte(contents), 0o644); err != nil {
		t.Fatalf("write task file: %v", err)
	}

	block, err := GetNextUncheckedTaskBlock(taskFile)
	if err != nil {
		t.Fatalf("get next task block: %v", err)
	}
	if !strings.Contains(block, "### Task B") {
		t.Fatalf("expected Task B block, got %q", block)
	}
}
