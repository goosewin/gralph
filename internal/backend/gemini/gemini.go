package gemini

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/goosewin/gralph/internal/backend"
)

type Backend struct {
	execPath string
}

func New() *Backend {
	return &Backend{execPath: "gemini"}
}

var _ backend.Backend = (*Backend)(nil)

func init() {
	if err := backend.Register("gemini", New()); err != nil {
		panic(err)
	}
}

func (b *Backend) CheckInstalled() error {
	if strings.TrimSpace(b.execPath) == "" {
		return errors.New("gemini executable path is empty")
	}
	if _, err := exec.LookPath(b.execPath); err != nil {
		return fmt.Errorf("gemini not installed: %w", err)
	}
	return nil
}

func (b *Backend) GetModels() []string {
	return []string{"gemini-1.5-pro"}
}

func (b *Backend) RunIteration(ctx context.Context, opts backend.IterationOptions) error {
	if strings.TrimSpace(opts.Prompt) == "" {
		return errors.New("prompt is required")
	}
	if strings.TrimSpace(opts.OutputFile) == "" {
		return errors.New("output file is required")
	}
	if ctx == nil {
		ctx = context.Background()
	}

	args := []string{"--headless"}
	if strings.TrimSpace(opts.Model) != "" {
		args = append(args, "--model", opts.Model)
	}
	args = append(args, opts.Prompt)

	cmd := exec.CommandContext(ctx, b.execPath, args...)

	if err := os.MkdirAll(filepath.Dir(opts.OutputFile), 0o755); err != nil {
		return fmt.Errorf("create output dir: %w", err)
	}
	outputFile, err := os.Create(opts.OutputFile)
	if err != nil {
		return fmt.Errorf("create output file: %w", err)
	}
	defer outputFile.Close()

	var rawFile *os.File
	if strings.TrimSpace(opts.RawOutputFile) != "" {
		if err := os.MkdirAll(filepath.Dir(opts.RawOutputFile), 0o755); err != nil {
			return fmt.Errorf("create raw output dir: %w", err)
		}
		rawFile, err = os.Create(opts.RawOutputFile)
		if err != nil {
			return fmt.Errorf("create raw output file: %w", err)
		}
		defer rawFile.Close()
	}

	writers := []io.Writer{outputFile, os.Stdout}
	if rawFile != nil {
		writers = append(writers, rawFile)
	}
	stdoutWriter := io.MultiWriter(writers...)
	var stderr bytes.Buffer
	stderrWriter := io.MultiWriter(append(writers, &stderr)...)

	cmd.Stdout = stdoutWriter
	cmd.Stderr = stderrWriter

	if err := cmd.Run(); err != nil {
		if stderr.Len() > 0 {
			return fmt.Errorf("gemini failed: %w: %s", err, strings.TrimSpace(stderr.String()))
		}
		return fmt.Errorf("gemini failed: %w", err)
	}

	return nil
}

func (b *Backend) ParseText(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return string(data), nil
}
