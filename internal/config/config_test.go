package config

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/spf13/viper"
)

func TestLoadConfigMergeAndOverrides(t *testing.T) {
	tempDir := t.TempDir()
	defaultPath := filepath.Join(tempDir, "default.yaml")
	globalPath := filepath.Join(tempDir, "global.yaml")
	projectDir := filepath.Join(tempDir, "project")
	projectPath := filepath.Join(projectDir, ".gralph.yaml")

	if err := os.MkdirAll(projectDir, 0o755); err != nil {
		t.Fatalf("mkdir project: %v", err)
	}

	writeFile(t, defaultPath, "defaults:\n  max_iterations: 30\n  backend: claude\nlogging:\n  level: info\n")
	writeFile(t, globalPath, "defaults:\n  max_iterations: 40\nlogging:\n  level: warn\n")
	writeFile(t, projectPath, "defaults:\n  max_iterations: 50\n")

	t.Setenv("GRALPH_DEFAULT_CONFIG", defaultPath)
	t.Setenv("GRALPH_GLOBAL_CONFIG", globalPath)
	t.Setenv("GRALPH_PROJECT_CONFIG_NAME", ".gralph.yaml")

	if _, err := LoadConfig(projectDir); err != nil {
		t.Fatalf("load config: %v", err)
	}

	if value, ok := GetConfig("defaults.max_iterations"); !ok || value != "50" {
		t.Fatalf("expected max_iterations 50, got %q", value)
	}

	if value, ok := GetConfig("defaults.backend"); !ok || value != "claude" {
		t.Fatalf("expected backend claude, got %q", value)
	}

	if value, ok := GetConfig("logging.level"); !ok || value != "warn" {
		t.Fatalf("expected logging.level warn, got %q", value)
	}

	t.Setenv("GRALPH_DEFAULTS_MAX_ITERATIONS", "77")
	if value, ok := GetConfig("defaults.max_iterations"); !ok || value != "77" {
		t.Fatalf("expected env override 77, got %q", value)
	}

	t.Setenv("GRALPH_MAX_ITERATIONS", "99")
	if value, ok := GetConfig("defaults.max_iterations"); !ok || value != "99" {
		t.Fatalf("expected legacy override 99, got %q", value)
	}
}

func TestSetConfigWritesGlobal(t *testing.T) {
	tempDir := t.TempDir()
	globalPath := filepath.Join(tempDir, "config.yaml")

	t.Setenv("GRALPH_CONFIG_DIR", tempDir)
	t.Setenv("GRALPH_GLOBAL_CONFIG", globalPath)

	if err := SetConfig("defaults.backend", "codex"); err != nil {
		t.Fatalf("set config: %v", err)
	}

	v := viper.New()
	v.SetConfigFile(globalPath)
	v.SetConfigType("yaml")
	if err := v.ReadInConfig(); err != nil {
		t.Fatalf("read global config: %v", err)
	}

	if value := v.GetString("defaults.backend"); value != "codex" {
		t.Fatalf("expected defaults.backend codex, got %q", value)
	}
}

func writeFile(t *testing.T, path, contents string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}
