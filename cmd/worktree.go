package cmd

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/spf13/cobra"
)

var worktreeCmd = &cobra.Command{
	Use:   "worktree",
	Short: "Manage task worktrees",
}

var worktreeCreateCmd = &cobra.Command{
	Use:   "create <ID>",
	Short: "Create a task worktree",
	Args:  cobra.ExactArgs(1),
	RunE:  runWorktreeCreate,
}

var worktreeFinishCmd = &cobra.Command{
	Use:   "finish <ID>",
	Short: "Finish a task worktree",
	Args:  cobra.ExactArgs(1),
	RunE:  runWorktreeFinish,
}

func init() {
	worktreeCmd.AddCommand(worktreeCreateCmd)
	worktreeCmd.AddCommand(worktreeFinishCmd)
	rootCmd.AddCommand(worktreeCmd)
}

func runWorktreeCreate(cmd *cobra.Command, args []string) error {
	taskID, err := validateTaskID(args[0])
	if err != nil {
		return err
	}

	repoRoot, err := gitRepoRoot()
	if err != nil {
		return err
	}

	if err := ensureGitClean(repoRoot); err != nil {
		return err
	}

	worktreesDir := filepath.Join(repoRoot, ".worktrees")
	if err := os.MkdirAll(worktreesDir, 0o755); err != nil {
		return fmt.Errorf("create worktrees dir: %w", err)
	}

	branchName := "task-" + taskID
	worktreePath := filepath.Join(worktreesDir, branchName)

	exists, err := gitRefExists(repoRoot, "refs/heads/"+branchName)
	if err != nil {
		return err
	}
	if exists {
		return fmt.Errorf("branch already exists: %s", branchName)
	}

	if info, err := os.Stat(worktreePath); err == nil {
		if info.IsDir() {
			return fmt.Errorf("worktree path already exists: %s", worktreePath)
		}
		return fmt.Errorf("worktree path already exists: %s", worktreePath)
	} else if !errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("check worktree path: %w", err)
	}

	if _, err := runGit(repoRoot, "worktree", "add", "-b", branchName, worktreePath); err != nil {
		return fmt.Errorf("failed to create worktree at %s: %w", worktreePath, err)
	}

	fmt.Printf("Created worktree %s on branch %s\n", worktreePath, branchName)
	return nil
}

func runWorktreeFinish(cmd *cobra.Command, args []string) error {
	taskID, err := validateTaskID(args[0])
	if err != nil {
		return err
	}

	repoRoot, err := gitRepoRoot()
	if err != nil {
		return err
	}

	if err := ensureGitClean(repoRoot); err != nil {
		return err
	}

	branchName := "task-" + taskID
	worktreesDir := filepath.Join(repoRoot, ".worktrees")
	worktreePath := filepath.Join(worktreesDir, branchName)

	exists, err := gitRefExists(repoRoot, "refs/heads/"+branchName)
	if err != nil {
		return err
	}
	if !exists {
		return fmt.Errorf("branch does not exist: %s", branchName)
	}

	info, err := os.Stat(worktreePath)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return fmt.Errorf("worktree path is missing: %s (run 'gralph worktree create %s' first)", worktreePath, taskID)
		}
		return fmt.Errorf("check worktree path: %w", err)
	}
	if !info.IsDir() {
		return fmt.Errorf("worktree path is missing: %s (run 'gralph worktree create %s' first)", worktreePath, taskID)
	}

	currentBranch, err := runGit(repoRoot, "rev-parse", "--abbrev-ref", "HEAD")
	if err != nil {
		return err
	}
	if currentBranch == branchName {
		return fmt.Errorf("cannot finish while on branch %s", branchName)
	}

	if _, err := runGit(repoRoot, "merge", "--no-ff", branchName); err != nil {
		return fmt.Errorf("failed to merge branch: %s", branchName)
	}

	if _, err := runGit(repoRoot, "worktree", "remove", worktreePath); err != nil {
		return fmt.Errorf("failed to remove worktree at %s: %w", worktreePath, err)
	}

	fmt.Printf("Finished worktree %s and merged %s\n", worktreePath, branchName)
	return nil
}

var taskIDRe = regexp.MustCompile(`^[A-Za-z]+-[0-9]+$`)

func validateTaskID(taskID string) (string, error) {
	taskID = strings.TrimSpace(taskID)
	if taskID == "" {
		return "", errors.New("task ID is required")
	}
	if !taskIDRe.MatchString(taskID) {
		return "", fmt.Errorf("invalid task ID format: %s (expected like A-1)", taskID)
	}
	return taskID, nil
}

func gitRepoRoot() (string, error) {
	root, err := runGit("", "rev-parse", "--show-toplevel")
	if err != nil {
		return "", errors.New("not a git repository (or any of the parent directories)")
	}
	return root, nil
}

func ensureGitClean(repoRoot string) error {
	output, err := runGit(repoRoot, "status", "--porcelain")
	if err != nil {
		return fmt.Errorf("unable to check git status in %s: %w", repoRoot, err)
	}
	if strings.TrimSpace(output) != "" {
		return errors.New("git working tree is dirty. Commit or stash changes before running worktree commands")
	}
	return nil
}

func gitRefExists(repoRoot, ref string) (bool, error) {
	args := []string{"show-ref", "--verify", "--quiet", ref}
	cmd := exec.Command("git", append([]string{"-C", repoRoot}, args...)...)
	err := cmd.Run()
	if err == nil {
		return true, nil
	}
	var exitErr *exec.ExitError
	if errors.As(err, &exitErr) {
		if exitErr.ExitCode() == 1 {
			return false, nil
		}
	}
	return false, fmt.Errorf("git show-ref failed: %w", err)
}

func runGit(repoRoot string, args ...string) (string, error) {
	fullArgs := args
	if repoRoot != "" {
		fullArgs = append([]string{"-C", repoRoot}, args...)
	}
	cmd := exec.Command("git", fullArgs...)
	output, err := cmd.CombinedOutput()
	trimmed := strings.TrimSpace(string(output))
	if err != nil {
		if trimmed == "" {
			return "", fmt.Errorf("git %s failed: %w", strings.Join(args, " "), err)
		}
		return "", fmt.Errorf("git %s failed: %s", strings.Join(args, " "), trimmed)
	}
	return trimmed, nil
}
