package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/goosewin/gralph/internal/core"
	"github.com/goosewin/gralph/internal/state"
	"github.com/spf13/cobra"
)

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show status of all gralph sessions",
	RunE:  runStatus,
}

func init() {
	rootCmd.AddCommand(statusCmd)
}

func runStatus(cmd *cobra.Command, args []string) error {
	if err := state.InitState(); err != nil {
		return err
	}

	_, _ = state.CleanupStale(state.CleanupMark)

	sessions, err := state.ListSessions()
	if err != nil {
		return err
	}
	if len(sessions) == 0 {
		fmt.Println("No sessions found")
		fmt.Println("Start a new loop with: gralph start <directory>")
		return nil
	}

	nameWidth := len("NAME")
	dirWidth := len("DIR")
	iterWidth := len("ITERATION")
	statusWidth := len("STATUS")
	remainingWidth := len("REMAINING")

	displayDirs := make([]string, len(sessions))
	for i, session := range sessions {
		name := session.Name
		dir := stringField(session.Fields, "dir")
		displayDirs[i] = truncateDir(dir, 40)
		if len(name) > nameWidth {
			nameWidth = len(name)
		}
		if len(displayDirs[i]) > dirWidth {
			dirWidth = len(displayDirs[i])
		}
	}

	fmt.Printf("%-*s  %-*s  %-*s  %-*s  %-*s\n", nameWidth, "NAME", dirWidth, "DIR", iterWidth, "ITERATION", statusWidth, "STATUS", remainingWidth, "REMAINING")
	fmt.Printf("%-*s  %-*s  %-*s  %-*s  %-*s\n", nameWidth, strings.Repeat("-", nameWidth), dirWidth, strings.Repeat("-", dirWidth), iterWidth, strings.Repeat("-", iterWidth), statusWidth, strings.Repeat("-", statusWidth), remainingWidth, strings.Repeat("-", remainingWidth))

	for i, session := range sessions {
		iteration, _ := intField(session.Fields, "iteration")
		maxIterations, _ := intField(session.Fields, "max_iterations")
		status := stringField(session.Fields, "status")
		lastCount, ok := intField(session.Fields, "last_task_count")
		if !ok {
			lastCount = -1
		}

		taskFile := stringField(session.Fields, "task_file")
		if taskFile == "" {
			taskFile = "PRD.md"
		}
		dir := stringField(session.Fields, "dir")

		remaining := lastCount
		if dir != "" && taskFile != "" {
			taskPath := filepath.Join(dir, taskFile)
			if _, err := os.Stat(taskPath); err == nil {
				if count, err := core.CountRemainingTasks(taskPath); err == nil {
					remaining = count
				}
			}
		}

		iterDisplay := fmt.Sprintf("%d/%d", iteration, maxIterations)
		remainingDisplay := formatRemaining(remaining)

		fmt.Printf("%-*s  %-*s  %-*s  %-*s  %-*s\n", nameWidth, session.Name, dirWidth, displayDirs[i], iterWidth, iterDisplay, statusWidth, status, remainingWidth, remainingDisplay)
	}

	fmt.Println("")
	fmt.Println("Commands: gralph logs <name>, gralph stop <name>, gralph resume")
	return nil
}

func truncateDir(dir string, max int) string {
	if max <= 0 {
		return dir
	}
	if len(dir) <= max {
		return dir
	}
	if max <= 3 {
		return dir[:max]
	}
	return "..." + dir[len(dir)-(max-3):]
}

func formatRemaining(remaining int) string {
	if remaining < 0 {
		return "?"
	}
	if remaining == 1 {
		return "1 task"
	}
	return fmt.Sprintf("%d tasks", remaining)
}
