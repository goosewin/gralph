package cmd

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/goosewin/gralph/internal/backend"
	"github.com/goosewin/gralph/internal/core"
	"github.com/goosewin/gralph/internal/state"
	"github.com/spf13/cobra"
)

var resumeCmd = &cobra.Command{
	Use:   "resume [name]",
	Short: "Resume stale or stopped gralph sessions",
	Args:  cobra.MaximumNArgs(1),
	RunE:  runResume,
}

func init() {
	rootCmd.AddCommand(resumeCmd)
}

func runResume(cmd *cobra.Command, args []string) error {
	if err := state.InitState(); err != nil {
		return err
	}

	sessions, err := state.ListSessions()
	if err != nil {
		return err
	}
	if len(sessions) == 0 {
		fmt.Println("No sessions found")
		return nil
	}

	target := ""
	if len(args) > 0 {
		target = args[0]
	}

	resumed := 0
	for _, session := range sessions {
		if target != "" && session.Name != target {
			continue
		}

		shouldResume, reason := shouldResumeSession(session)
		if !shouldResume {
			if target != "" {
				fmt.Fprintf(os.Stderr, "Warning: session %q is not resumable (%s).\n", session.Name, reason)
			}
			continue
		}

		projectDir := stringField(session.Fields, "dir")
		if projectDir == "" {
			fmt.Fprintf(os.Stderr, "Warning: skipping %q (missing project dir).\n", session.Name)
			continue
		}
		if info, err := os.Stat(projectDir); err != nil || !info.IsDir() {
			fmt.Fprintf(os.Stderr, "Warning: skipping %q (directory missing: %s).\n", session.Name, projectDir)
			continue
		}

		taskFile := stringField(session.Fields, "task_file")
		if taskFile == "" {
			taskFile = "PRD.md"
		}
		taskPath := filepath.Join(projectDir, taskFile)
		if _, err := os.Stat(taskPath); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: skipping %q (task file missing: %s).\n", session.Name, taskPath)
			continue
		}

		maxIterations, ok := intField(session.Fields, "max_iterations")
		if !ok || maxIterations <= 0 {
			maxIterations = 30
		}
		completionMarker := stringField(session.Fields, "completion_marker")
		if completionMarker == "" {
			completionMarker = "COMPLETE"
		}
		backendName := stringField(session.Fields, "backend")
		if backendName == "" {
			backendName = backend.DefaultName()
		}
		model := stringField(session.Fields, "model")
		variant := stringField(session.Fields, "variant")
		webhook := stringField(session.Fields, "webhook")

		tmuxSession := "gralph-" + session.Name
		tmuxPid, err := startTmuxLoop(tmuxSession, projectDir, session.Name, taskFile, completionMarker, backendName, model, variant, webhook, maxIterations, "")
		if err != nil {
			return err
		}

		remaining := 0
		if count, err := core.CountRemainingTasks(taskPath); err == nil {
			remaining = count
		}

		if err := state.SetSession(session.Name, map[string]interface{}{
			"pid":             tmuxPid,
			"tmux_session":    tmuxSession,
			"status":          "running",
			"last_task_count": remaining,
		}); err != nil {
			return err
		}

		fmt.Printf("Resumed session: %s (tmux: %s)\n", session.Name, tmuxSession)
		resumed++
	}

	if target != "" && resumed == 0 {
		return errors.New("no matching session resumed")
	}

	if resumed == 0 {
		fmt.Println("No sessions to resume")
		return nil
	}

	fmt.Printf("Resumed %d session(s)\n", resumed)
	return nil
}

func shouldResumeSession(session state.Session) (bool, string) {
	status := stringField(session.Fields, "status")
	pid, _ := intField(session.Fields, "pid")

	switch status {
	case "running":
		if pid <= 0 {
			return true, "missing pid"
		}
		if !processAlive(pid) {
			return true, "stale pid"
		}
		return false, "already running"
	case "stale", "stopped":
		return true, status
	case "":
		return true, "unknown status"
	default:
		return false, status
	}
}
