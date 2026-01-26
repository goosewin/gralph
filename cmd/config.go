package cmd

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"

	"github.com/goosewin/gralph/internal/config"
	"github.com/spf13/cobra"
)

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Manage configuration",
	RunE:  runConfigList,
}

var configGetCmd = &cobra.Command{
	Use:   "get <key>",
	Short: "Get a configuration value",
	Args:  cobra.ExactArgs(1),
	RunE:  runConfigGet,
}

var configSetCmd = &cobra.Command{
	Use:   "set <key> <value>",
	Short: "Set a configuration value",
	Args:  cobra.ExactArgs(2),
	RunE:  runConfigSet,
}

var configListCmd = &cobra.Command{
	Use:   "list",
	Short: "List configuration values",
	Args:  cobra.NoArgs,
	RunE:  runConfigList,
}

func init() {
	configCmd.AddCommand(configGetCmd)
	configCmd.AddCommand(configSetCmd)
	configCmd.AddCommand(configListCmd)
	rootCmd.AddCommand(configCmd)
}

func runConfigGet(cmd *cobra.Command, args []string) error {
	if err := loadConfigForCwd(); err != nil {
		return err
	}

	key := strings.TrimSpace(args[0])
	if key == "" {
		return errors.New("config key is required")
	}

	value, ok := config.GetConfig(key)
	if !ok {
		return fmt.Errorf("config key not found: %s", key)
	}

	fmt.Println(value)
	return nil
}

func runConfigSet(cmd *cobra.Command, args []string) error {
	key := strings.TrimSpace(args[0])
	if key == "" {
		return errors.New("config key is required")
	}

	value := strings.TrimSpace(args[1])
	if value == "" {
		return errors.New("config value is required")
	}

	if err := config.SetConfig(key, value); err != nil {
		return err
	}

	fmt.Printf("Updated config: %s\n", key)
	return nil
}

func runConfigList(cmd *cobra.Command, args []string) error {
	if err := loadConfigForCwd(); err != nil {
		return err
	}

	items, err := config.ListConfig()
	if err != nil {
		return err
	}
	if len(items) == 0 {
		return nil
	}

	keys := make([]string, 0, len(items))
	for key := range items {
		keys = append(keys, key)
	}
	sort.Strings(keys)

	for _, key := range keys {
		fmt.Printf("%s=%s\n", key, items[key])
	}

	return nil
}

func loadConfigForCwd() error {
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("resolve current directory: %w", err)
	}
	_, err = config.LoadConfig(cwd)
	return err
}
