package config

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/viper"
)

// Paths captures the config files used during LoadConfig.
type Paths struct {
	Default string
	Global  string
	Project string
}

var (
	currentConfig *viper.Viper
	currentPaths  Paths
)

// LoadConfig loads and merges configuration in priority order:
// default -> global -> project (highest).
func LoadConfig(projectDir string) (Paths, error) {
	v := viper.New()
	v.SetConfigType("yaml")
	v.SetEnvPrefix("GRALPH")
	v.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))
	v.AutomaticEnv()

	paths := Paths{
		Default: defaultConfigPath(),
		Global:  globalConfigPath(),
		Project: projectConfigPath(projectDir),
	}

	if err := readConfigFile(v, paths.Default); err != nil {
		return paths, err
	}
	if err := mergeConfigFile(v, paths.Global); err != nil {
		return paths, err
	}
	if err := mergeConfigFile(v, paths.Project); err != nil {
		return paths, err
	}

	currentConfig = v
	currentPaths = paths

	return paths, nil
}

// GetConfig returns a config value as a string with env overrides applied.
func GetConfig(key string) (string, bool) {
	if key == "" {
		return "", false
	}

	if legacyKey, ok := legacyEnvOverrides()[key]; ok {
		if value, found := os.LookupEnv(legacyKey); found {
			return value, true
		}
	}

	if currentConfig == nil {
		return "", false
	}

	if !currentConfig.IsSet(key) {
		return "", false
	}

	return valueToString(currentConfig.Get(key)), true
}

// SetConfig writes a configuration value to the global config file.
func SetConfig(key, value string) error {
	if key == "" {
		return errors.New("config key is required")
	}

	globalPath := globalConfigPath()
	if globalPath == "" {
		return errors.New("global config path is not available")
	}

	if err := os.MkdirAll(filepath.Dir(globalPath), 0o755); err != nil {
		return fmt.Errorf("create config dir: %w", err)
	}

	v := viper.New()
	v.SetConfigType("yaml")
	v.SetConfigFile(globalPath)
	if fileExists(globalPath) {
		if err := v.ReadInConfig(); err != nil {
			return fmt.Errorf("read global config: %w", err)
		}
	}

	v.Set(key, value)
	if err := v.WriteConfigAs(globalPath); err != nil {
		return fmt.Errorf("write global config: %w", err)
	}

	if currentConfig != nil {
		currentConfig.Set(key, value)
	}

	return nil
}

// ListConfig returns a flattened view of the current configuration.
func ListConfig() (map[string]string, error) {
	if currentConfig == nil {
		return nil, errors.New("config not loaded")
	}

	settings := currentConfig.AllSettings()
	flattened := map[string]string{}
	flattenSettings("", settings, flattened)
	return flattened, nil
}

func defaultConfigPath() string {
	if path, ok := os.LookupEnv("GRALPH_DEFAULT_CONFIG"); ok && path != "" {
		return path
	}

	var candidates []string
	if exe, err := os.Executable(); err == nil {
		exeDir := filepath.Dir(exe)
		candidates = append(candidates,
			filepath.Join(exeDir, "config", "default.yaml"),
			filepath.Join(exeDir, "..", "config", "default.yaml"),
		)
	}

	if cwd, err := os.Getwd(); err == nil {
		candidates = append(candidates, filepath.Join(cwd, "config", "default.yaml"))
	}

	if home, err := os.UserHomeDir(); err == nil {
		candidates = append(candidates, filepath.Join(home, ".config", "gralph", "config", "default.yaml"))
	}

	for _, candidate := range candidates {
		if fileExists(candidate) {
			return candidate
		}
	}

	return ""
}

func globalConfigPath() string {
	if path, ok := os.LookupEnv("GRALPH_GLOBAL_CONFIG"); ok && path != "" {
		return path
	}

	configDir := configDir()
	if configDir == "" {
		return ""
	}

	return filepath.Join(configDir, "config.yaml")
}

func projectConfigPath(projectDir string) string {
	if projectDir == "" {
		return ""
	}

	info, err := os.Stat(projectDir)
	if err != nil || !info.IsDir() {
		return ""
	}

	name := os.Getenv("GRALPH_PROJECT_CONFIG_NAME")
	if name == "" {
		name = ".gralph.yaml"
	}

	return filepath.Join(projectDir, name)
}

func configDir() string {
	if path, ok := os.LookupEnv("GRALPH_CONFIG_DIR"); ok && path != "" {
		return path
	}

	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}

	return filepath.Join(home, ".config", "gralph")
}

func readConfigFile(v *viper.Viper, path string) error {
	if !fileExists(path) {
		return nil
	}

	v.SetConfigFile(path)
	if err := v.ReadInConfig(); err != nil {
		return fmt.Errorf("read config %s: %w", path, err)
	}

	return nil
}

func mergeConfigFile(v *viper.Viper, path string) error {
	if !fileExists(path) {
		return nil
	}

	v.SetConfigFile(path)
	if err := v.MergeInConfig(); err != nil {
		return fmt.Errorf("merge config %s: %w", path, err)
	}

	return nil
}

func fileExists(path string) bool {
	if path == "" {
		return false
	}

	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

func legacyEnvOverrides() map[string]string {
	return map[string]string{
		"defaults.max_iterations":    "GRALPH_MAX_ITERATIONS",
		"defaults.task_file":         "GRALPH_TASK_FILE",
		"defaults.completion_marker": "GRALPH_COMPLETION_MARKER",
		"defaults.backend":           "GRALPH_BACKEND",
		"defaults.model":             "GRALPH_MODEL",
	}
}

func valueToString(value interface{}) string {
	switch typed := value.(type) {
	case []string:
		return strings.Join(typed, ",")
	case []interface{}:
		parts := make([]string, 0, len(typed))
		for _, item := range typed {
			parts = append(parts, fmt.Sprint(item))
		}
		return strings.Join(parts, ",")
	default:
		return fmt.Sprint(value)
	}
}

func flattenSettings(prefix string, value interface{}, out map[string]string) {
	if value == nil {
		return
	}

	switch typed := value.(type) {
	case map[string]interface{}:
		for key, item := range typed {
			nextKey := key
			if prefix != "" {
				nextKey = prefix + "." + key
			}
			flattenSettings(nextKey, item, out)
		}
	case map[interface{}]interface{}:
		for key, item := range typed {
			keyText := fmt.Sprint(key)
			nextKey := keyText
			if prefix != "" {
				nextKey = prefix + "." + keyText
			}
			flattenSettings(nextKey, item, out)
		}
	default:
		if prefix == "" {
			return
		}
		out[prefix] = valueToString(value)
	}
}
