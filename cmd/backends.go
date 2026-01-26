package cmd

import (
	"fmt"
	"os"
	"text/tabwriter"

	"github.com/goosewin/gralph/internal/backend"
	_ "github.com/goosewin/gralph/internal/backend/claude"
	_ "github.com/goosewin/gralph/internal/backend/codex"
	_ "github.com/goosewin/gralph/internal/backend/gemini"
	_ "github.com/goosewin/gralph/internal/backend/opencode"
	"github.com/spf13/cobra"
)

var backendsCmd = &cobra.Command{
	Use:   "backends",
	Short: "List available AI backends",
	RunE:  runBackends,
}

func init() {
	rootCmd.AddCommand(backendsCmd)
}

func runBackends(cmd *cobra.Command, args []string) error {
	names := backend.Names()
	if len(names) == 0 {
		fmt.Println("No backends registered")
		return nil
	}

	writer := tabwriter.NewWriter(os.Stdout, 0, 0, 2, ' ', 0)
	fmt.Fprintln(writer, "NAME\tINSTALLED")
	fmt.Fprintln(writer, "----\t---------")

	for _, name := range names {
		installed := "no"
		instance, ok := backend.Get(name)
		if ok {
			if err := instance.CheckInstalled(); err == nil {
				installed = "yes"
			}
		}
		fmt.Fprintf(writer, "%s\t%s\n", name, installed)
	}

	if err := writer.Flush(); err != nil {
		return err
	}

	fmt.Println("")
	fmt.Println("Usage: gralph start <dir> --backend <name>")
	return nil
}
