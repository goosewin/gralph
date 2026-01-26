package claude

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
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

type streamEvent struct {
	Type    string `json:"type"`
	Result  string `json:"result"`
	Message struct {
		Content []struct {
			Type string `json:"type"`
			Text string `json:"text"`
		} `json:"content"`
	} `json:"message"`
}

func New() *Backend {
	return &Backend{execPath: "claude"}
}

var _ backend.Backend = (*Backend)(nil)

func init() {
	if err := backend.Register("claude", New()); err != nil {
		panic(err)
	}
}

func (b *Backend) CheckInstalled() error {
	if strings.TrimSpace(b.execPath) == "" {
		return errors.New("claude executable path is empty")
	}
	if _, err := exec.LookPath(b.execPath); err != nil {
		return fmt.Errorf("claude not installed: %w", err)
	}
	return nil
}

func (b *Backend) GetModels() []string {
	return []string{"claude-opus-4-5"}
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

	args := []string{
		"--dangerously-skip-permissions",
		"--verbose",
		"--print",
		"--output-format",
		"stream-json",
	}
	if strings.TrimSpace(opts.Model) != "" {
		args = append(args, "--model", opts.Model)
	}
	args = append(args, "-p", opts.Prompt)

	cmd := exec.CommandContext(ctx, b.execPath, args...)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("claude stdout: %w", err)
	}

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

	var stderr bytes.Buffer
	if rawFile != nil {
		cmd.Stderr = io.MultiWriter(&stderr, rawFile)
	} else {
		cmd.Stderr = &stderr
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start claude: %w", err)
	}

	scanner := bufio.NewScanner(stdout)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		line := scanner.Text()
		if rawFile != nil {
			_, _ = rawFile.WriteString(line + "\n")
		}
		if _, err := outputFile.WriteString(line + "\n"); err != nil {
			return fmt.Errorf("write output file: %w", err)
		}

		if text := parseStreamText(line); text != "" {
			_, _ = io.WriteString(os.Stdout, text)
			if !strings.HasSuffix(text, "\n") {
				_, _ = io.WriteString(os.Stdout, "\n")
			}
			_, _ = io.WriteString(os.Stdout, "\n")
		}
	}
	if err := scanner.Err(); err != nil {
		return fmt.Errorf("read claude output: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		if stderr.Len() > 0 {
			return fmt.Errorf("claude failed: %w: %s", err, strings.TrimSpace(stderr.String()))
		}
		return fmt.Errorf("claude failed: %w", err)
	}

	return nil
}

func (b *Backend) ParseText(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}

	result := parseStreamResult(bytes.NewReader(data))
	if result != "" {
		return result, nil
	}

	return string(data), nil
}

func parseStreamResult(reader io.Reader) string {
	scanner := bufio.NewScanner(reader)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	result := ""
	for scanner.Scan() {
		var event streamEvent
		if err := json.Unmarshal(scanner.Bytes(), &event); err != nil {
			continue
		}
		if event.Type == "result" && event.Result != "" {
			result = event.Result
		}
	}
	return result
}

func parseStreamText(line string) string {
	var event streamEvent
	if err := json.Unmarshal([]byte(line), &event); err != nil {
		return ""
	}
	if event.Type != "assistant" {
		return ""
	}
	var builder strings.Builder
	for _, part := range event.Message.Content {
		if part.Type != "text" || part.Text == "" {
			continue
		}
		builder.WriteString(part.Text)
	}
	return builder.String()
}
