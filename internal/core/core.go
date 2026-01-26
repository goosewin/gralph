package core

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/goosewin/gralph/internal/backend"
	"github.com/goosewin/gralph/internal/config"
)

var (
	ErrMaxIterations = errors.New("max iterations reached")
)

type StateCallback func(update StateUpdate)

type StateUpdate struct {
	Session   string
	Iteration int
	Status    string
	Remaining int
}

type LoopOptions struct {
	ProjectDir       string
	TaskFile         string
	MaxIterations    int
	CompletionMarker string
	Model            string
	SessionName      string
	PromptTemplate   string
	BackendName      string
	Backend          backend.Backend
	LogFile          string
	StateCallback    StateCallback
	SleepDelay       time.Duration
}

type LoopResult struct {
	Completed  bool
	Iterations int
	Remaining  int
	Duration   time.Duration
}

// RunLoop executes the core iteration loop.
func RunLoop(ctx context.Context, opts LoopOptions) (LoopResult, error) {
	result := LoopResult{}
	if strings.TrimSpace(opts.ProjectDir) == "" {
		return result, errors.New("project directory is required")
	}

	projectDir, err := filepath.Abs(opts.ProjectDir)
	if err != nil {
		return result, fmt.Errorf("resolve project dir: %w", err)
	}
	info, err := os.Stat(projectDir)
	if err != nil || !info.IsDir() {
		return result, fmt.Errorf("project directory does not exist: %s", projectDir)
	}

	if opts.MaxIterations <= 0 {
		if value, ok := config.GetConfig("defaults.max_iterations"); ok {
			if parsed, err := strconv.Atoi(value); err == nil && parsed > 0 {
				opts.MaxIterations = parsed
			}
		}
		if opts.MaxIterations <= 0 {
			opts.MaxIterations = 30
		}
	}

	if strings.TrimSpace(opts.TaskFile) == "" {
		if value, ok := config.GetConfig("defaults.task_file"); ok && strings.TrimSpace(value) != "" {
			opts.TaskFile = value
		} else {
			opts.TaskFile = "PRD.md"
		}
	}

	if strings.TrimSpace(opts.CompletionMarker) == "" {
		if value, ok := config.GetConfig("defaults.completion_marker"); ok && strings.TrimSpace(value) != "" {
			opts.CompletionMarker = value
		} else {
			opts.CompletionMarker = "COMPLETE"
		}
	}

	fullTaskPath := filepath.Join(projectDir, opts.TaskFile)
	if _, err := os.Stat(fullTaskPath); err != nil {
		return result, fmt.Errorf("task file does not exist: %s", fullTaskPath)
	}

	logFile := opts.LogFile
	if strings.TrimSpace(logFile) == "" {
		logDir := filepath.Join(projectDir, ".gralph")
		if err := os.MkdirAll(logDir, 0o755); err != nil {
			return result, fmt.Errorf("create log dir: %w", err)
		}
		cleanupOldLogs(logDir)
		name := opts.SessionName
		if strings.TrimSpace(name) == "" {
			name = "gralph"
		}
		logFile = filepath.Join(logDir, name+".log")
	}

	logWriter, err := openLogWriter(logFile)
	if err != nil {
		return result, err
	}
	if logWriter != nil {
		defer logWriter.Close()
	}
	logger := newLogger(logWriter)

	iteration := 1
	start := time.Now()

	logger.Line(fmt.Sprintf("Starting gralph loop in %s", projectDir))
	logger.Line(fmt.Sprintf("Task file: %s", opts.TaskFile))
	logger.Line(fmt.Sprintf("Max iterations: %d", opts.MaxIterations))
	logger.Line(fmt.Sprintf("Completion marker: %s", opts.CompletionMarker))
	if strings.TrimSpace(opts.Model) != "" {
		logger.Line(fmt.Sprintf("Model: %s", opts.Model))
	}
	logger.Line("Started at: " + time.Now().Format(time.RFC3339))

	initialRemaining, err := CountRemainingTasks(fullTaskPath)
	if err != nil {
		return result, err
	}
	logger.Line(fmt.Sprintf("Initial remaining tasks: %d", initialRemaining))

	backendInstance, err := resolveBackend(opts)
	if err != nil {
		return result, err
	}
	if err := backendInstance.CheckInstalled(); err != nil {
		return result, err
	}

	delay := opts.SleepDelay
	if delay == 0 {
		delay = 2 * time.Second
	}

	for iteration <= opts.MaxIterations {
		remainingBefore, err := CountRemainingTasks(fullTaskPath)
		if err != nil {
			return result, err
		}

		logger.Line("")
		logger.Line(fmt.Sprintf("=== Iteration %d/%d (Remaining: %d) ===", iteration, opts.MaxIterations, remainingBefore))

		if opts.StateCallback != nil {
			opts.StateCallback(StateUpdate{Session: opts.SessionName, Iteration: iteration, Status: "running", Remaining: remainingBefore})
		}

		iterationResult, err := RunIteration(ctx, IterationOptions{
			ProjectDir:       projectDir,
			TaskFile:         opts.TaskFile,
			Iteration:        iteration,
			MaxIterations:    opts.MaxIterations,
			CompletionMarker: opts.CompletionMarker,
			Model:            opts.Model,
			PromptTemplate:   opts.PromptTemplate,
			Backend:          backendInstance,
			LogFile:          logFile,
		})
		if err != nil {
			if opts.StateCallback != nil {
				opts.StateCallback(StateUpdate{Session: opts.SessionName, Iteration: iteration, Status: "failed", Remaining: remainingBefore})
			}
			return result, err
		}

		completed, err := CheckCompletion(fullTaskPath, iterationResult, opts.CompletionMarker)
		if err != nil {
			return result, err
		}
		if completed {
			duration := time.Since(start)
			result.Completed = true
			result.Iterations = iteration
			result.Duration = duration
			result.Remaining = 0

			logger.Line("")
			logger.Line(fmt.Sprintf("Gralph complete after %d iterations.", iteration))
			logger.Line(fmt.Sprintf("Duration: %ds", int(duration.Seconds())))
			logger.Line("FINISHED: " + time.Now().Format(time.RFC3339))

			if opts.StateCallback != nil {
				opts.StateCallback(StateUpdate{Session: opts.SessionName, Iteration: iteration, Status: "complete", Remaining: 0})
			}
			return result, nil
		}

		remainingAfter, err := CountRemainingTasks(fullTaskPath)
		if err != nil {
			return result, err
		}
		logger.Line(fmt.Sprintf("Tasks remaining after iteration: %d", remainingAfter))
		if opts.StateCallback != nil {
			opts.StateCallback(StateUpdate{Session: opts.SessionName, Iteration: iteration, Status: "running", Remaining: remainingAfter})
		}

		iteration++
		if iteration <= opts.MaxIterations && delay > 0 {
			time.Sleep(delay)
		}
	}

	duration := time.Since(start)
	remainingFinal, err := CountRemainingTasks(fullTaskPath)
	if err != nil {
		return result, err
	}
	result.Completed = false
	result.Iterations = opts.MaxIterations
	result.Remaining = remainingFinal
	result.Duration = duration

	logger.Line("")
	logger.Line(fmt.Sprintf("Hit max iterations (%d)", opts.MaxIterations))
	logger.Line(fmt.Sprintf("Remaining tasks: %d", remainingFinal))
	logger.Line(fmt.Sprintf("Duration: %ds", int(duration.Seconds())))
	logger.Line("FINISHED: " + time.Now().Format(time.RFC3339))

	if opts.StateCallback != nil {
		opts.StateCallback(StateUpdate{Session: opts.SessionName, Iteration: opts.MaxIterations, Status: "max_iterations", Remaining: remainingFinal})
	}

	return result, ErrMaxIterations
}

type IterationOptions struct {
	ProjectDir       string
	TaskFile         string
	Iteration        int
	MaxIterations    int
	CompletionMarker string
	Model            string
	PromptTemplate   string
	BackendName      string
	Backend          backend.Backend
	LogFile          string
}

// RunIteration executes a single backend iteration and returns its parsed result.
func RunIteration(ctx context.Context, opts IterationOptions) (string, error) {
	if strings.TrimSpace(opts.ProjectDir) == "" {
		return "", errors.New("project directory is required")
	}
	if opts.Iteration <= 0 {
		return "", errors.New("iteration number is required")
	}
	if opts.MaxIterations <= 0 {
		return "", errors.New("max iterations is required")
	}

	projectDir := opts.ProjectDir
	fullTaskPath := filepath.Join(projectDir, opts.TaskFile)
	if strings.TrimSpace(opts.TaskFile) == "" {
		fullTaskPath = filepath.Join(projectDir, "PRD.md")
		opts.TaskFile = "PRD.md"
	}
	if _, err := os.Stat(fullTaskPath); err != nil {
		return "", fmt.Errorf("task file does not exist: %s", fullTaskPath)
	}

	backendInstance, err := resolveBackend(LoopOptions{Backend: opts.Backend, BackendName: opts.BackendName})
	if err != nil {
		return "", err
	}

	promptTemplate, err := resolvePromptTemplate(projectDir, opts.PromptTemplate)
	if err != nil {
		return "", err
	}

	taskBlock, err := GetNextUncheckedTaskBlock(fullTaskPath)
	if err != nil {
		return "", err
	}
	if strings.TrimSpace(taskBlock) == "" {
		remaining, err := CountRemainingTasks(fullTaskPath)
		if err != nil {
			return "", err
		}
		if remaining > 0 {
			if line, err := firstUncheckedLine(fullTaskPath); err == nil && strings.TrimSpace(line) != "" {
				taskBlock = line
			}
		}
	}

	contextList := []string{}
	if value, ok := config.GetConfig("defaults.context_files"); ok {
		contextList = NormalizeContextFiles(value)
	}

	prompt := RenderPromptTemplate(promptTemplate, opts.TaskFile, opts.CompletionMarker, opts.Iteration, opts.MaxIterations, taskBlock, contextList)

	outputFile, err := os.CreateTemp("", "gralph-iteration-*.jsonl")
	if err != nil {
		return "", fmt.Errorf("create temp output file: %w", err)
	}
	outputPath := outputFile.Name()
	_ = outputFile.Close()
	defer os.Remove(outputPath)

	rawOutputPath := ""
	if strings.TrimSpace(opts.LogFile) != "" {
		if strings.HasSuffix(opts.LogFile, ".log") {
			rawOutputPath = strings.TrimSuffix(opts.LogFile, ".log") + ".raw.log"
		} else {
			rawOutputPath = opts.LogFile + ".raw.log"
		}
	}

	currentDir, _ := os.Getwd()
	if err := os.Chdir(projectDir); err != nil {
		return "", err
	}
	defer func() {
		_ = os.Chdir(currentDir)
	}()

	if err := backendInstance.RunIteration(ctx, backend.IterationOptions{
		Prompt:        prompt,
		Model:         opts.Model,
		OutputFile:    outputPath,
		RawOutputFile: rawOutputPath,
	}); err != nil {
		return "", err
	}

	if info, err := os.Stat(outputPath); err != nil || info.Size() == 0 {
		if rawOutputPath != "" {
			return "", fmt.Errorf("backend produced no output, raw output: %s", rawOutputPath)
		}
		return "", errors.New("backend produced no output")
	}

	parsed, err := backendInstance.ParseText(outputPath)
	if err != nil {
		return "", err
	}
	if strings.TrimSpace(parsed) == "" {
		if rawOutputPath != "" {
			return "", fmt.Errorf("backend returned no parsed result, raw output: %s", rawOutputPath)
		}
		return "", errors.New("backend returned no parsed result")
	}

	return parsed, nil
}

type logWriter struct {
	writer io.Writer
	closer io.Closer
}

func openLogWriter(path string) (*logWriter, error) {
	if strings.TrimSpace(path) == "" {
		return nil, nil
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return nil, fmt.Errorf("create log dir: %w", err)
	}
	file, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		return nil, fmt.Errorf("open log file: %w", err)
	}
	return &logWriter{writer: file, closer: file}, nil
}

func (l *logWriter) Write(p []byte) (int, error) {
	if l == nil || l.writer == nil {
		return 0, nil
	}
	return l.writer.Write(p)
}

func (l *logWriter) Close() error {
	if l == nil || l.closer == nil {
		return nil
	}
	return l.closer.Close()
}

type logger struct {
	logWriter io.Writer
}

func newLogger(writer io.Writer) *logger {
	return &logger{logWriter: writer}
}

func (l *logger) Line(message string) {
	if message == "" {
		message = ""
	}
	line := message + "\n"
	_, _ = io.WriteString(os.Stdout, line)
	if l.logWriter != nil {
		_, _ = l.logWriter.Write([]byte(line))
	}
}

func resolveBackend(opts LoopOptions) (backend.Backend, error) {
	if opts.Backend != nil {
		return opts.Backend, nil
	}
	name := strings.TrimSpace(opts.BackendName)
	if name == "" {
		if value, ok := config.GetConfig("defaults.backend"); ok {
			name = value
		}
	}
	if name == "" {
		name = backend.DefaultName()
	}
	instance, ok := backend.Get(name)
	if !ok {
		return nil, fmt.Errorf("backend not found: %s", name)
	}
	return instance, nil
}

func cleanupOldLogs(logDir string) {
	retainDays := 7
	if value, ok := config.GetConfig("logging.retain_days"); ok {
		if parsed, err := strconv.Atoi(value); err == nil && parsed > 0 {
			retainDays = parsed
		}
	}
	if retainDays <= 0 {
		retainDays = 7
	}

	entries, err := os.ReadDir(logDir)
	if err != nil {
		return
	}
	cutoff := time.Now().Add(-time.Duration(retainDays) * 24 * time.Hour)
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		if !strings.HasSuffix(entry.Name(), ".log") {
			continue
		}
		info, err := entry.Info()
		if err != nil {
			continue
		}
		if info.ModTime().Before(cutoff) {
			_ = os.Remove(filepath.Join(logDir, entry.Name()))
		}
	}
}
