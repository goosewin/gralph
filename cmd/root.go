package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

// Version is overridden at build time via -ldflags.
var Version = "dev"

var rootCmd = &cobra.Command{
	Use:     "gralph",
	Short:   "Autonomous AI coding loops",
	Long:    "Gralph runs autonomous AI coding loops until PRD tasks complete.",
	Version: Version,
}

// Execute runs the root command.
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
