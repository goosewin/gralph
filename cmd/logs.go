package cmd

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/goosewin/gralph/internal/state"
	"github.com/spf13/cobra"
)

var (
	logsFollow bool
)

var logsCmd = &cobra.Command{
	Use:   "logs <name>",
	Short: "Show logs for a gralph session",
	Args:  cobra.ExactArgs(1),
	RunE:  runLogs,
}

func init() {
	logsCmd.Flags().BoolVar(&logsFollow, "follow", false, "Follow log output")
	rootCmd.AddCommand(logsCmd)
}

func runLogs(cmd *cobra.Command, args []string) error {
	if err := state.InitState(); err != nil {
		return err
	}

	sessionName := args[0]
	session, found, err := state.GetSession(sessionName)
	if err != nil {
		return err
	}
	if !found {
		return fmt.Errorf("session not found: %s", sessionName)
	}

	logFile := stringField(session.Fields, "log_file")
	if logFile == "" {
		dir := stringField(session.Fields, "dir")
		if dir != "" {
			logFile = dir + "/.gralph/" + sessionName + ".log"
		}
	}
	if logFile == "" {
		return errors.New("cannot determine log file path")
	}

	if _, err := os.Stat(logFile); err != nil {
		return fmt.Errorf("log file does not exist: %s", logFile)
	}

	status := stringField(session.Fields, "status")
	fmt.Printf("Session: %s (status: %s)\n", sessionName, status)
	fmt.Printf("Log file: %s\n\n", logFile)

	if logsFollow {
		return followLogFile(logFile, 100)
	}

	lines, err := tailLines(logFile, 100)
	if err != nil {
		return err
	}
	for _, line := range lines {
		fmt.Println(line)
	}
	return nil
}

func tailLines(path string, limit int) ([]string, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	if limit <= 0 {
		return []string{}, nil
	}

	buffer := make([]string, 0, limit)
	scanner := bufioScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		if len(buffer) == limit {
			copy(buffer, buffer[1:])
			buffer[limit-1] = line
		} else {
			buffer = append(buffer, line)
		}
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	return buffer, nil
}

func followLogFile(path string, limit int) error {
	lines, err := tailLines(path, limit)
	if err != nil {
		return err
	}
	for _, line := range lines {
		fmt.Println(line)
	}

	file, err := os.Open(path)
	if err != nil {
		return err
	}
	defer file.Close()

	offset, err := file.Seek(0, io.SeekEnd)
	if err != nil {
		return err
	}

	reader := bufio.NewReader(file)
	for {
		line, readErr := reader.ReadString('\n')
		if readErr == nil {
			fmt.Print(strings.TrimRight(line, "\n"))
			fmt.Print("\n")
			offset += int64(len(line))
			continue
		}

		if readErr != io.EOF {
			return readErr
		}

		time.Sleep(500 * time.Millisecond)
		if _, err := file.Seek(offset, io.SeekStart); err != nil {
			return err
		}
	}
}
