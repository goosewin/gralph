package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var completionCmd = &cobra.Command{
	Use:       "completion [bash|zsh|fish]",
	Short:     "Generate shell completion scripts",
	Args:      cobra.ExactValidArgs(1),
	ValidArgs: []string{"bash", "zsh", "fish"},
	RunE:      runCompletion,
}

func init() {
	rootCmd.AddCommand(completionCmd)
}

func runCompletion(cmd *cobra.Command, args []string) error {
	shell := args[0]
	out := cmd.OutOrStdout()
	switch shell {
	case "bash":
		return rootCmd.GenBashCompletion(out)
	case "zsh":
		return rootCmd.GenZshCompletion(out)
	case "fish":
		return rootCmd.GenFishCompletion(out, true)
	default:
		return fmt.Errorf("unsupported shell: %s", shell)
	}
}
