package cmd

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/goosewin/gralph/internal/backend"
	"github.com/goosewin/gralph/internal/config"
	"github.com/goosewin/gralph/internal/core"
	"github.com/goosewin/gralph/internal/state"
	"github.com/spf13/cobra"
)

var (
	startName             string
	startMaxIterations    int
	startTaskFile         string
	startCompletionMarker string
	startBackend          string
	startModel            string
	startVariant          string
	startPromptTemplate   string
	startWebhook          string
	startNoTmux           bool
	startStrictPRD        bool
	startTmuxChild        bool
)

var startCmd = &cobra.Command{
	Use:   "start <dir>",
	Short: "Start a new gralph loop",
	Args:  cobra.ExactArgs(1),
	RunE:  runStart,
}

func init() {
	startCmd.Flags().StringVarP(&startName, "name", "n", "", "Session name (default: directory name)")
	startCmd.Flags().IntVar(&startMaxIterations, "max-iterations", 0, "Max iterations before giving up")
	startCmd.Flags().StringVarP(&startTaskFile, "task-file", "f", "", "Task file path (relative to project)")
	startCmd.Flags().StringVar(&startCompletionMarker, "completion-marker", "", "Completion promise text")
	startCmd.Flags().StringVarP(&startBackend, "backend", "b", "", "AI backend (claude, opencode, gemini, codex)")
	startCmd.Flags().StringVarP(&startModel, "model", "m", "", "Model override (backend-specific)")
	startCmd.Flags().StringVar(&startVariant, "variant", "", "Model variant override (backend-specific)")
	startCmd.Flags().StringVar(&startPromptTemplate, "prompt-template", "", "Path to custom prompt template file")
	startCmd.Flags().StringVar(&startWebhook, "webhook", "", "Notification webhook URL")
	startCmd.Flags().BoolVar(&startNoTmux, "no-tmux", false, "Run in foreground (blocking)")
	startCmd.Flags().BoolVar(&startStrictPRD, "strict-prd", false, "Validate PRD before starting the loop")
	startCmd.Flags().BoolVar(&startTmuxChild, "tmux-child", false, "Run loop as tmux child")
	_ = startCmd.Flags().MarkHidden("tmux-child")

	rootCmd.AddCommand(startCmd)
}

func runStart(cmd *cobra.Command, args []string) error {
	projectDir, err := filepath.Abs(args[0])
	if err != nil {
		return fmt.Errorf("resolve project directory: %w", err)
	}
	info, err := os.Stat(projectDir)
	if err != nil || !info.IsDir() {
		return fmt.Errorf("project directory does not exist: %s", projectDir)
	}

	if _, err := config.LoadConfig(projectDir); err != nil {
		return err
	}

	flags := cmd.Flags()
	maxIterations := startMaxIterations
	if !flags.Changed("max-iterations") {
		if value, ok := config.GetConfig("defaults.max_iterations"); ok {
			if parsed, err := strconv.Atoi(value); err == nil {
				maxIterations = parsed
			}
		}
	}
	if flags.Changed("max-iterations") && maxIterations <= 0 {
		return errors.New("max-iterations must be a positive integer")
	}
	if maxIterations <= 0 {
		maxIterations = 30
	}

	taskFile := strings.TrimSpace(startTaskFile)
	if !flags.Changed("task-file") {
		if value, ok := config.GetConfig("defaults.task_file"); ok {
			taskFile = strings.TrimSpace(value)
		}
	}
	if taskFile == "" {
		taskFile = "PRD.md"
	}

	completionMarker := strings.TrimSpace(startCompletionMarker)
	if !flags.Changed("completion-marker") {
		if value, ok := config.GetConfig("defaults.completion_marker"); ok {
			completionMarker = strings.TrimSpace(value)
		}
	}
	if completionMarker == "" {
		completionMarker = "COMPLETE"
	}

	backendName := strings.TrimSpace(startBackend)
	if !flags.Changed("backend") {
		if value, ok := config.GetConfig("defaults.backend"); ok {
			backendName = strings.TrimSpace(value)
		}
	}
	if backendName == "" {
		backendName = backend.DefaultName()
	}

	model := strings.TrimSpace(startModel)
	if !flags.Changed("model") {
		if value, ok := config.GetConfig("defaults.model"); ok {
			model = strings.TrimSpace(value)
		}
	}
	if model == "" {
		switch strings.ToLower(backendName) {
		case "opencode":
			model = configValue("opencode.default_model")
		case "gemini":
			model = configValue("gemini.default_model")
		case "codex":
			model = configValue("codex.default_model")
		}
	}

	backendInstance, ok := backend.Get(backendName)
	if !ok {
		return fmt.Errorf("backend not found: %s", backendName)
	}
	if err := backendInstance.CheckInstalled(); err != nil {
		return err
	}

	variant := strings.TrimSpace(startVariant)
	webhook := strings.TrimSpace(startWebhook)

	taskFilePath := filepath.Join(projectDir, taskFile)
	if _, err := os.Stat(taskFilePath); err != nil {
		return fmt.Errorf("task file does not exist: %s", taskFilePath)
	}

	if startStrictPRD {
		if err := validateStrictPRD(taskFilePath); err != nil {
			return fmt.Errorf("PRD validation failed: %w", err)
		}
	}

	sessionName := strings.TrimSpace(startName)
	if sessionName == "" {
		sessionName = filepath.Base(projectDir)
	}
	sessionName = sanitizeSessionName(sessionName)
	if sessionName == "" {
		return errors.New("session name is required")
	}

	gralphDir := filepath.Join(projectDir, ".gralph")
	if err := os.MkdirAll(gralphDir, 0o755); err != nil {
		return fmt.Errorf("create log dir: %w", err)
	}

	promptTemplatePath := ""
	if strings.TrimSpace(startPromptTemplate) != "" {
		path, err := copyPromptTemplate(startPromptTemplate, gralphDir)
		if err != nil {
			return err
		}
		promptTemplatePath = path
		if promptTemplatePath != "" {
			_ = os.Setenv("GRALPH_PROMPT_TEMPLATE_FILE", promptTemplatePath)
		}
	}

	if err := state.InitState(); err != nil {
		return err
	}

	if !startTmuxChild {
		if err := ensureSessionAvailable(sessionName); err != nil {
			return err
		}
	}

	logFile := filepath.Join(gralphDir, sessionName+".log")
	initialRemaining, err := core.CountRemainingTasks(taskFilePath)
	if err != nil {
		return err
	}

	if startTmuxChild {
		startNoTmux = true
	}

	if !startNoTmux && !startTmuxChild {
		tmuxSession := "gralph-" + sessionName
		tmuxPid, err := startTmuxLoop(tmuxSession, projectDir, sessionName, taskFile, completionMarker, backendName, model, variant, webhook, maxIterations, promptTemplatePath)
		if err != nil {
			return err
		}

		if err := state.SetSession(sessionName, sessionFields(projectDir, taskFile, tmuxPid, tmuxSession, maxIterations, completionMarker, logFile, backendName, model, variant, webhook, initialRemaining)); err != nil {
			return err
		}

		fmt.Printf("Gralph loop started in tmux session: %s\n", tmuxSession)
		return nil
	}

	if !startTmuxChild {
		if err := state.SetSession(sessionName, sessionFields(projectDir, taskFile, os.Getpid(), "", maxIterations, completionMarker, logFile, backendName, model, variant, webhook, initialRemaining)); err != nil {
			return err
		}
	}

	_ = os.Setenv("OPENCODE_CONFIG", filepath.Join(projectDir, "opencode.json"))

	stateCallback := func(update core.StateUpdate) {
		fields := map[string]interface{}{
			"iteration":       update.Iteration,
			"status":          update.Status,
			"last_task_count": update.Remaining,
		}
		_ = state.SetSession(sessionName, fields)
	}

	_, err = core.RunLoop(context.Background(), core.LoopOptions{
		ProjectDir:       projectDir,
		TaskFile:         taskFile,
		MaxIterations:    maxIterations,
		CompletionMarker: completionMarker,
		Model:            model,
		SessionName:      sessionName,
		PromptTemplate:   "",
		BackendName:      backendName,
		LogFile:          logFile,
		StateCallback:    stateCallback,
	})

	if err != nil {
		finalStatus := "failed"
		if errors.Is(err, core.ErrMaxIterations) {
			finalStatus = "max_iterations"
		}
		_ = state.SetSession(sessionName, map[string]interface{}{"status": finalStatus})
		return err
	}

	_ = state.SetSession(sessionName, map[string]interface{}{"status": "complete"})
	return nil
}

func configValue(key string) string {
	if value, ok := config.GetConfig(key); ok {
		return strings.TrimSpace(value)
	}
	return ""
}

func sessionFields(projectDir, taskFile string, pid int, tmuxSession string, maxIterations int, completionMarker, logFile, backendName, model, variant, webhook string, remaining int) map[string]interface{} {
	return map[string]interface{}{
		"dir":               projectDir,
		"task_file":         taskFile,
		"pid":               pid,
		"tmux_session":      tmuxSession,
		"started_at":        time.Now().Format(time.RFC3339),
		"iteration":         1,
		"max_iterations":    maxIterations,
		"status":            "running",
		"last_task_count":   remaining,
		"completion_marker": completionMarker,
		"log_file":          logFile,
		"backend":           backendName,
		"model":             model,
		"variant":           variant,
		"webhook":           webhook,
	}
}

func ensureSessionAvailable(name string) error {
	session, found, err := state.GetSession(name)
	if err != nil {
		return err
	}
	if !found {
		return nil
	}

	status := stringField(session.Fields, "status")
	if status != "running" {
		return nil
	}

	pid, ok := intField(session.Fields, "pid")
	if ok && pid > 0 && processAlive(pid) {
		return fmt.Errorf("session %q is already running (pid: %d)", name, pid)
	}

	fmt.Fprintf(os.Stderr, "Warning: session %q appears stale and will be restarted.\n", name)
	return nil
}

func startTmuxLoop(tmuxSession, projectDir, sessionName, taskFile, completionMarker, backendName, model, variant, webhook string, maxIterations int, promptTemplatePath string) (int, error) {
	if _, err := exec.LookPath("tmux"); err != nil {
		return 0, errors.New("tmux is required for background mode (install tmux or use --no-tmux)")
	}

	_ = exec.Command("tmux", "kill-session", "-t", tmuxSession).Run()

	exePath, err := os.Executable()
	if err != nil {
		return 0, fmt.Errorf("resolve executable: %w", err)
	}

	shellPath, err := exec.LookPath("bash")
	if err != nil {
		shellPath = "sh"
	}

	command := buildTmuxCommand(exePath, projectDir, sessionName, taskFile, completionMarker, backendName, model, variant, webhook, maxIterations, promptTemplatePath)
	if err := exec.Command("tmux", "new-session", "-d", "-s", tmuxSession, shellPath, "-lc", command).Run(); err != nil {
		return 0, fmt.Errorf("start tmux session: %w", err)
	}

	output, err := exec.Command("tmux", "list-panes", "-t", tmuxSession, "-F", "#{pane_pid}").Output()
	if err != nil {
		return 0, fmt.Errorf("read tmux pid: %w", err)
	}

	lines := strings.Split(strings.TrimSpace(string(output)), "\n")
	if len(lines) == 0 || strings.TrimSpace(lines[0]) == "" {
		return 0, errors.New("unable to read tmux pane pid")
	}

	pid, err := strconv.Atoi(strings.TrimSpace(lines[0]))
	if err != nil {
		return 0, fmt.Errorf("parse tmux pid: %w", err)
	}

	return pid, nil
}

func buildTmuxCommand(exePath, projectDir, sessionName, taskFile, completionMarker, backendName, model, variant, webhook string, maxIterations int, promptTemplatePath string) string {
	parts := []string{
		fmt.Sprintf("export PATH=%s", shellQuote(os.Getenv("PATH"))),
		fmt.Sprintf("export OPENCODE_CONFIG=%s", shellQuote(filepath.Join(projectDir, "opencode.json"))),
	}
	if promptTemplatePath != "" {
		parts = append(parts, fmt.Sprintf("export GRALPH_PROMPT_TEMPLATE_FILE=%s", shellQuote(promptTemplatePath)))
	}

	args := []string{
		shellQuote(exePath),
		"start",
		shellQuote(projectDir),
		"--no-tmux",
		"--tmux-child",
		"--name",
		shellQuote(sessionName),
		"--max-iterations",
		strconv.Itoa(maxIterations),
		"--task-file",
		shellQuote(taskFile),
		"--completion-marker",
		shellQuote(completionMarker),
		"--backend",
		shellQuote(backendName),
	}

	if model != "" {
		args = append(args, "--model", shellQuote(model))
	}
	if variant != "" {
		args = append(args, "--variant", shellQuote(variant))
	}
	if webhook != "" {
		args = append(args, "--webhook", shellQuote(webhook))
	}

	parts = append(parts, strings.Join(args, " "))
	return strings.Join(parts, " && ")
}

func shellQuote(value string) string {
	if value == "" {
		return "''"
	}
	return "'" + strings.ReplaceAll(value, "'", "'\"'\"'") + "'"
}

func sanitizeSessionName(value string) string {
	re := regexp.MustCompile(`[^a-zA-Z0-9_-]`)
	return re.ReplaceAllString(value, "-")
}

func processAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	return syscall.Kill(pid, 0) == nil
}

func stringField(fields map[string]interface{}, key string) string {
	value, ok := fields[key]
	if !ok || value == nil {
		return ""
	}
	switch typed := value.(type) {
	case string:
		return typed
	case fmt.Stringer:
		return typed.String()
	default:
		return fmt.Sprint(value)
	}
}

func intField(fields map[string]interface{}, key string) (int, bool) {
	value, ok := fields[key]
	if !ok || value == nil {
		return 0, false
	}
	switch typed := value.(type) {
	case int:
		return typed, true
	case int64:
		return int(typed), true
	case float64:
		return int(typed), true
	case string:
		parsed, err := strconv.Atoi(typed)
		if err != nil {
			return 0, false
		}
		return parsed, true
	case json.Number:
		parsed, err := typed.Int64()
		if err != nil {
			return 0, false
		}
		return int(parsed), true
	default:
		return 0, false
	}
}

func copyPromptTemplate(sourcePath, gralphDir string) (string, error) {
	path := strings.TrimSpace(sourcePath)
	if path == "" {
		return "", nil
	}

	if !filepath.IsAbs(path) {
		cwd, err := os.Getwd()
		if err != nil {
			return "", err
		}
		path = filepath.Join(cwd, path)
	}

	info, err := os.Stat(path)
	if err != nil {
		return "", fmt.Errorf("prompt template file does not exist: %s", path)
	}
	if info.IsDir() {
		return "", fmt.Errorf("prompt template is a directory: %s", path)
	}

	destination := filepath.Join(gralphDir, "prompt-template.txt")
	if path == destination {
		return destination, nil
	}

	source, err := os.Open(path)
	if err != nil {
		return "", fmt.Errorf("read prompt template: %w", err)
	}
	defer source.Close()

	dest, err := os.Create(destination)
	if err != nil {
		return "", fmt.Errorf("write prompt template: %w", err)
	}
	defer dest.Close()

	if _, err := io.Copy(dest, source); err != nil {
		return "", fmt.Errorf("copy prompt template: %w", err)
	}

	return destination, nil
}

func validateStrictPRD(taskFile string) error {
	blocks, err := core.GetTaskBlocks(taskFile)
	if err != nil {
		return err
	}
	if len(blocks) == 0 {
		return errors.New("no task blocks found")
	}

	for _, block := range blocks {
		if err := validateBlock(block); err != nil {
			return err
		}
	}

	if err := ensureNoUncheckedOutsideBlocks(taskFile); err != nil {
		return err
	}

	return nil
}

func validateBlock(block string) error {
	header := firstLine(block)
	required := []string{
		"- **ID**",
		"- **Context Bundle**",
		"- **DoD**",
		"- **Checklist**",
		"- **Dependencies**",
	}
	for _, field := range required {
		if !strings.Contains(block, field) {
			return fmt.Errorf("%s missing %s", header, field)
		}
	}

	if countUncheckedLines(block) != 1 {
		return fmt.Errorf("%s must contain exactly one unchecked task line", header)
	}

	return nil
}

func ensureNoUncheckedOutsideBlocks(taskFile string) error {
	file, err := os.Open(taskFile)
	if err != nil {
		return err
	}
	defer file.Close()

	scanner := bufioScanner(file)
	inBlock := false
	for scanner.Scan() {
		line := scanner.Text()
		if taskHeaderRe.MatchString(line) {
			inBlock = true
		} else if inBlock && taskEndRe.MatchString(line) {
			inBlock = false
		}

		if !inBlock && uncheckedLineRe.MatchString(line) {
			return fmt.Errorf("unchecked task line found outside task block: %s", strings.TrimSpace(line))
		}
	}
	return scanner.Err()
}

func countUncheckedLines(block string) int {
	scanner := bufioScanner(strings.NewReader(block))
	count := 0
	for scanner.Scan() {
		if uncheckedLineRe.MatchString(scanner.Text()) {
			count++
		}
	}
	return count
}

func firstLine(block string) string {
	parts := strings.Split(block, "\n")
	if len(parts) == 0 {
		return "task block"
	}
	line := strings.TrimSpace(parts[0])
	if line == "" {
		return "task block"
	}
	return line
}

var (
	taskHeaderRe    = regexp.MustCompile(`^\s*###\s+Task\s+`)
	taskEndRe       = regexp.MustCompile(`^\s*(---|##\s+)`)
	uncheckedLineRe = regexp.MustCompile(`^\s*- \[ \]`)
)

func bufioScanner(r io.Reader) *bufio.Scanner {
	scanner := bufio.NewScanner(r)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	return scanner
}
