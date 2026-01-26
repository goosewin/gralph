package cmd

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"syscall"

	"github.com/goosewin/gralph/internal/state"
	"github.com/spf13/cobra"
)

var (
	stopAll bool
)

var stopCmd = &cobra.Command{
	Use:   "stop <name>",
	Short: "Stop a running gralph loop",
	Args:  cobra.MaximumNArgs(1),
	RunE:  runStop,
}

func init() {
	stopCmd.Flags().BoolVarP(&stopAll, "all", "a", false, "Stop all running sessions")
	rootCmd.AddCommand(stopCmd)
}

func runStop(cmd *cobra.Command, args []string) error {
	if err := state.InitState(); err != nil {
		return err
	}

	if stopAll {
		return stopAllSessions()
	}

	if len(args) == 0 || args[0] == "" {
		return errors.New("session name is required (use --all to stop all sessions)")
	}

	sessionName := args[0]
	session, found, err := state.GetSession(sessionName)
	if err != nil {
		return err
	}
	if !found {
		return fmt.Errorf("session not found: %s", sessionName)
	}

	if err := stopSession(session); err != nil {
		return err
	}

	fmt.Printf("Stopped session: %s\n", sessionName)
	return nil
}

func stopAllSessions() error {
	sessions, err := state.ListSessions()
	if err != nil {
		return err
	}
	if len(sessions) == 0 {
		fmt.Println("No sessions found")
		return nil
	}

	stopped := 0
	for _, session := range sessions {
		status := stringField(session.Fields, "status")
		if status != "running" {
			continue
		}
		if err := stopSession(session); err != nil {
			return err
		}
		fmt.Printf("Stopped session: %s\n", session.Name)
		stopped++
	}

	if stopped == 0 {
		fmt.Println("No running sessions to stop")
		return nil
	}

	fmt.Printf("Stopped %d session(s)\n", stopped)
	return nil
}

func stopSession(session state.Session) error {
	tmuxSession := stringField(session.Fields, "tmux_session")
	pid, _ := intField(session.Fields, "pid")

	if tmuxSession != "" {
		if err := exec.Command("tmux", "kill-session", "-t", tmuxSession).Run(); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: tmux session %q not found or already stopped.\n", tmuxSession)
		}
	} else if pid > 0 && processAlive(pid) {
		proc, err := os.FindProcess(pid)
		if err == nil {
			_ = proc.Signal(syscall.SIGTERM)
		}
	}

	return state.SetSession(session.Name, map[string]interface{}{
		"status":       "stopped",
		"pid":          0,
		"tmux_session": "",
	})
}
