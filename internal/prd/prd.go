package prd

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

// StackSummary captures detected stack signals from a project directory.
type StackSummary struct {
	Root            string
	IDs             []string
	Languages       []string
	Frameworks      []string
	Tools           []string
	Runtimes        []string
	PackageManagers []string
	Evidence        []string
	SelectedIDs     []string
}

// DetectStack inspects a directory and returns detected stack metadata.
func DetectStack(targetDir string) (StackSummary, error) {
	summary := StackSummary{}
	if strings.TrimSpace(targetDir) == "" {
		return summary, nil
	}

	info, err := os.Stat(targetDir)
	if err != nil || !info.IsDir() {
		return summary, nil
	}

	summary.Root = targetDir

	record := func(path string) {
		if path == "" {
			return
		}
		if summary.Root != "" {
			if rel, err := filepath.Rel(summary.Root, path); err == nil && !strings.HasPrefix(rel, "..") {
				path = rel
			}
		}
		addUnique(&summary.Evidence, path)
	}

	packageJSON := filepath.Join(targetDir, "package.json")
	if fileExists(packageJSON) {
		addUnique(&summary.IDs, "Node.js")
		addUnique(&summary.Runtimes, "Node.js")
		addUnique(&summary.Languages, "JavaScript")
		record(packageJSON)

		tsconfig := filepath.Join(targetDir, "tsconfig.json")
		if fileExists(tsconfig) {
			addUnique(&summary.Languages, "TypeScript")
			record(tsconfig)
		}

		pnpmLock := filepath.Join(targetDir, "pnpm-lock.yaml")
		if fileExists(pnpmLock) {
			addUnique(&summary.PackageManagers, "pnpm")
			record(pnpmLock)
		}

		yarnLock := filepath.Join(targetDir, "yarn.lock")
		if fileExists(yarnLock) {
			addUnique(&summary.PackageManagers, "yarn")
			record(yarnLock)
		}

		npmLock := filepath.Join(targetDir, "package-lock.json")
		if fileExists(npmLock) {
			addUnique(&summary.PackageManagers, "npm")
			record(npmLock)
		}

		bunLock := filepath.Join(targetDir, "bun.lockb")
		if fileExists(bunLock) {
			addUnique(&summary.Runtimes, "Bun")
			addUnique(&summary.PackageManagers, "bun")
			record(bunLock)
		}

		bunfig := filepath.Join(targetDir, "bunfig.toml")
		if fileExists(bunfig) {
			addUnique(&summary.Runtimes, "Bun")
			addUnique(&summary.PackageManagers, "bun")
			record(bunfig)
		}

		for _, name := range []string{"next.config.js", "next.config.mjs", "next.config.cjs"} {
			path := filepath.Join(targetDir, name)
			if fileExists(path) {
				addUnique(&summary.Frameworks, "Next.js")
				record(path)
			}
		}

		for _, name := range []string{"nuxt.config.js", "nuxt.config.ts"} {
			path := filepath.Join(targetDir, name)
			if fileExists(path) {
				addUnique(&summary.Frameworks, "Nuxt")
				record(path)
			}
		}

		for _, name := range []string{"svelte.config.js", "svelte.config.ts"} {
			path := filepath.Join(targetDir, name)
			if fileExists(path) {
				addUnique(&summary.Frameworks, "Svelte")
				record(path)
			}
		}

		for _, name := range []string{"vite.config.js", "vite.config.ts", "vite.config.mjs"} {
			path := filepath.Join(targetDir, name)
			if fileExists(path) {
				addUnique(&summary.Tools, "Vite")
				record(path)
			}
		}

		angularJSON := filepath.Join(targetDir, "angular.json")
		if fileExists(angularJSON) {
			addUnique(&summary.Frameworks, "Angular")
			record(angularJSON)
		}

		vueConfig := filepath.Join(targetDir, "vue.config.js")
		if fileExists(vueConfig) {
			addUnique(&summary.Frameworks, "Vue")
			record(vueConfig)
		}

		if jsonHasDependency(packageJSON, "react") {
			addUnique(&summary.Frameworks, "React")
		}
		if jsonHasDependency(packageJSON, "next") {
			addUnique(&summary.Frameworks, "Next.js")
		}
		if jsonHasDependency(packageJSON, "vue") {
			addUnique(&summary.Frameworks, "Vue")
		}
		if jsonHasDependency(packageJSON, "@angular/core") {
			addUnique(&summary.Frameworks, "Angular")
		}
		if jsonHasDependency(packageJSON, "svelte") {
			addUnique(&summary.Frameworks, "Svelte")
		}
		if jsonHasDependency(packageJSON, "nuxt") {
			addUnique(&summary.Frameworks, "Nuxt")
		}
		if jsonHasDependency(packageJSON, "express") {
			addUnique(&summary.Frameworks, "Express")
		}
		if jsonHasDependency(packageJSON, "fastify") {
			addUnique(&summary.Frameworks, "Fastify")
		}
		if jsonHasDependency(packageJSON, "@nestjs/core") {
			addUnique(&summary.Frameworks, "NestJS")
		}
	}

	goMod := filepath.Join(targetDir, "go.mod")
	if fileExists(goMod) {
		addUnique(&summary.IDs, "Go")
		addUnique(&summary.Languages, "Go")
		addUnique(&summary.Tools, "Go modules")
		record(goMod)
	}

	cargo := filepath.Join(targetDir, "Cargo.toml")
	if fileExists(cargo) {
		addUnique(&summary.IDs, "Rust")
		addUnique(&summary.Languages, "Rust")
		addUnique(&summary.Tools, "Cargo")
		record(cargo)
	}

	pyproject := filepath.Join(targetDir, "pyproject.toml")
	requirements := filepath.Join(targetDir, "requirements.txt")
	poetryLock := filepath.Join(targetDir, "poetry.lock")
	pipfile := filepath.Join(targetDir, "Pipfile")
	pipfileLock := filepath.Join(targetDir, "Pipfile.lock")
	if fileExists(pyproject) || fileExists(requirements) || fileExists(poetryLock) || fileExists(pipfile) || fileExists(pipfileLock) {
		addUnique(&summary.IDs, "Python")
		addUnique(&summary.Languages, "Python")
		if fileExists(pyproject) {
			record(pyproject)
			if fileMatchesRegex(pyproject, regexp.MustCompile(`(?i)\[tool\.poetry\]`)) {
				addUnique(&summary.Tools, "Poetry")
			}
		}
		if fileExists(requirements) {
			record(requirements)
		}
		if fileExists(poetryLock) {
			record(poetryLock)
		}
		if fileExists(pipfile) {
			record(pipfile)
		}
		if fileExists(pipfileLock) {
			record(pipfileLock)
		}

		if fileExists(requirements) {
			if fileMatchesRegex(requirements, regexp.MustCompile(`(?i)(^|\s)django([<>=]|$)`)) {
				addUnique(&summary.Frameworks, "Django")
			}
			if fileMatchesRegex(requirements, regexp.MustCompile(`(?i)(^|\s)flask([<>=]|$)`)) {
				addUnique(&summary.Frameworks, "Flask")
			}
			if fileMatchesRegex(requirements, regexp.MustCompile(`(?i)(^|\s)fastapi([<>=]|$)`)) {
				addUnique(&summary.Frameworks, "FastAPI")
			}
		}

		if fileExists(pyproject) && fileMatchesRegex(pyproject, regexp.MustCompile(`(?i)django|flask|fastapi`)) {
			if fileMatchesRegex(pyproject, regexp.MustCompile(`(?i)django`)) {
				addUnique(&summary.Frameworks, "Django")
			}
			if fileMatchesRegex(pyproject, regexp.MustCompile(`(?i)flask`)) {
				addUnique(&summary.Frameworks, "Flask")
			}
			if fileMatchesRegex(pyproject, regexp.MustCompile(`(?i)fastapi`)) {
				addUnique(&summary.Frameworks, "FastAPI")
			}
		}
	}

	gemfile := filepath.Join(targetDir, "Gemfile")
	if fileExists(gemfile) {
		addUnique(&summary.IDs, "Ruby")
		addUnique(&summary.Languages, "Ruby")
		record(gemfile)
		if fileMatchesRegex(gemfile, regexp.MustCompile(`(?i)rails`)) {
			addUnique(&summary.Frameworks, "Rails")
		}
		if fileMatchesRegex(gemfile, regexp.MustCompile(`(?i)sinatra`)) {
			addUnique(&summary.Frameworks, "Sinatra")
		}
	}

	mix := filepath.Join(targetDir, "mix.exs")
	if fileExists(mix) {
		addUnique(&summary.IDs, "Elixir")
		addUnique(&summary.Languages, "Elixir")
		record(mix)
		if fileMatchesRegex(mix, regexp.MustCompile(`(?i)phoenix`)) {
			addUnique(&summary.Frameworks, "Phoenix")
		}
	}

	composer := filepath.Join(targetDir, "composer.json")
	if fileExists(composer) {
		addUnique(&summary.IDs, "PHP")
		addUnique(&summary.Languages, "PHP")
		record(composer)
		if fileMatchesRegex(composer, regexp.MustCompile(`(?i)laravel`)) {
			addUnique(&summary.Frameworks, "Laravel")
		}
	}

	pom := filepath.Join(targetDir, "pom.xml")
	if fileExists(pom) {
		addUnique(&summary.IDs, "Java")
		addUnique(&summary.Languages, "Java")
		addUnique(&summary.Tools, "Maven")
		record(pom)
		if fileMatchesRegex(pom, regexp.MustCompile(`(?i)spring-boot`)) {
			addUnique(&summary.Frameworks, "Spring Boot")
		}
	}

	gradle := filepath.Join(targetDir, "build.gradle")
	if fileExists(gradle) {
		addUnique(&summary.IDs, "Java")
		addUnique(&summary.Languages, "Java")
		addUnique(&summary.Tools, "Gradle")
		record(gradle)
		if fileMatchesRegex(gradle, regexp.MustCompile(`(?i)spring-boot`)) {
			addUnique(&summary.Frameworks, "Spring Boot")
		}
	}

	gradleKts := filepath.Join(targetDir, "build.gradle.kts")
	if fileExists(gradleKts) {
		addUnique(&summary.IDs, "Java")
		addUnique(&summary.Languages, "Java")
		addUnique(&summary.Tools, "Gradle")
		record(gradleKts)
		if fileMatchesRegex(gradleKts, regexp.MustCompile(`(?i)spring-boot`)) {
			addUnique(&summary.Frameworks, "Spring Boot")
		}
	}

	csprojFiles, _ := filepath.Glob(filepath.Join(targetDir, "*.csproj"))
	slnFiles, _ := filepath.Glob(filepath.Join(targetDir, "*.sln"))
	if len(csprojFiles) > 0 || len(slnFiles) > 0 {
		addUnique(&summary.IDs, ".NET")
		addUnique(&summary.Languages, "C#")
		for _, file := range csprojFiles {
			record(file)
		}
	}
	for _, file := range slnFiles {
		record(file)
	}

	dockerfile := filepath.Join(targetDir, "Dockerfile")
	if fileExists(dockerfile) {
		addUnique(&summary.Tools, "Docker")
		record(dockerfile)
	}

	dockerCompose := filepath.Join(targetDir, "docker-compose.yml")
	if fileExists(dockerCompose) {
		addUnique(&summary.Tools, "Docker Compose")
		record(dockerCompose)
	}

	dockerComposeAlt := filepath.Join(targetDir, "docker-compose.yaml")
	if fileExists(dockerComposeAlt) {
		addUnique(&summary.Tools, "Docker Compose")
		record(dockerComposeAlt)
	}

	makefile := filepath.Join(targetDir, "Makefile")
	if fileExists(makefile) {
		addUnique(&summary.Tools, "Make")
		record(makefile)
	}

	terraformFiles, _ := filepath.Glob(filepath.Join(targetDir, "*.tf"))
	if len(terraformFiles) > 0 {
		addUnique(&summary.Tools, "Terraform")
		for _, file := range terraformFiles {
			record(file)
		}
	}

	summary.SelectedIDs = append([]string(nil), summary.IDs...)
	return summary, nil
}

// ValidateOptions controls PRD validation behavior.
type ValidateOptions struct {
	AllowMissingContext bool
	BaseDir             string
}

// ValidationError captures one or more PRD validation failures.
type ValidationError struct {
	Issues []string
}

func (v ValidationError) Error() string {
	return strings.Join(v.Issues, "\n")
}

// ValidateFile validates all task blocks and top-level rules in a PRD file.
func ValidateFile(taskFile string, opts *ValidateOptions) error {
	if strings.TrimSpace(taskFile) == "" {
		return errors.New("task file is required")
	}
	if _, err := os.Stat(taskFile); err != nil {
		return fmt.Errorf("task file does not exist: %w", err)
	}

	options := ValidateOptions{}
	if opts != nil {
		options = *opts
	}

	baseDir := options.BaseDir
	if baseDir == "" {
		baseDir = filepath.Dir(taskFile)
	}
	if abs, err := filepath.Abs(baseDir); err == nil {
		baseDir = abs
	}

	issues := []string{}

	if hasOpenQuestions(taskFile) {
		issues = append(issues, fmt.Sprintf("PRD validation error: %s: Open Questions section is not allowed", taskFile))
	}

	if stray := validateStrayUnchecked(taskFile); len(stray) > 0 {
		issues = append(issues, stray...)
	}

	blocks, err := GetTaskBlocks(taskFile)
	if err != nil {
		return err
	}
	for _, block := range blocks {
		issues = append(issues, validateTaskBlock(block, taskFile, options, baseDir)...)
	}

	if len(issues) > 0 {
		return ValidationError{Issues: issues}
	}
	return nil
}

// SanitizeGeneratedFile rewrites a generated PRD to enforce required structure.
func SanitizeGeneratedFile(taskFile, baseDir, allowedContextFile string) error {
	if strings.TrimSpace(taskFile) == "" {
		return nil
	}
	if _, err := os.Stat(taskFile); err != nil {
		return nil
	}

	if baseDir == "" {
		baseDir = filepath.Dir(taskFile)
	}
	if abs, err := filepath.Abs(baseDir); err == nil {
		baseDir = abs
	}

	info, err := os.Stat(taskFile)
	if err != nil {
		return err
	}

	temp, err := os.CreateTemp(filepath.Dir(taskFile), "prd-sanitize-*")
	if err != nil {
		return err
	}
	defer func() {
		_ = temp.Close()
	}()

	if err := os.Chmod(temp.Name(), info.Mode()); err != nil {
		return err
	}

	file, err := os.Open(taskFile)
	if err != nil {
		return err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)

	var blockBuilder strings.Builder
	inBlock := false
	inOpenQuestions := false
	started := false

	for scanner.Scan() {
		line := scanner.Text()
		lower := strings.ToLower(line)

		if openQuestionsHeaderRe.MatchString(lower) {
			inOpenQuestions = true
			continue
		}
		if inOpenQuestions {
			if headingRe.MatchString(line) {
				inOpenQuestions = false
			} else {
				continue
			}
		}

		if !started {
			if headingRe.MatchString(line) {
				started = true
			} else {
				continue
			}
		}

		if taskHeaderRe.MatchString(line) {
			if inBlock {
				if _, err := io.WriteString(temp, sanitizeTaskBlock(blockBuilder.String(), baseDir, allowedContextFile)); err != nil {
					return err
				}
				blockBuilder.Reset()
			}
			inBlock = true
			blockBuilder.WriteString(line)
			continue
		}

		if inBlock && taskEndRe.MatchString(line) {
			if _, err := io.WriteString(temp, sanitizeTaskBlock(blockBuilder.String(), baseDir, allowedContextFile)); err != nil {
				return err
			}
			blockBuilder.Reset()
			inBlock = false
		}

		if inBlock {
			blockBuilder.WriteString("\n")
			blockBuilder.WriteString(line)
			continue
		}

		if uncheckedLineRe.MatchString(line) {
			line = uncheckedLineRe.ReplaceAllString(line, "$1- $2")
		}

		if _, err := io.WriteString(temp, line+"\n"); err != nil {
			return err
		}
	}

	if err := scanner.Err(); err != nil {
		return err
	}

	if inBlock {
		if _, err := io.WriteString(temp, sanitizeTaskBlock(blockBuilder.String(), baseDir, allowedContextFile)); err != nil {
			return err
		}
	}

	if err := temp.Close(); err != nil {
		return err
	}

	return os.Rename(temp.Name(), taskFile)
}

// GetTaskBlocks extracts task blocks grouped by task headers.
func GetTaskBlocks(taskFile string) ([]string, error) {
	if strings.TrimSpace(taskFile) == "" {
		return nil, nil
	}

	file, err := os.Open(taskFile)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)

	var blocks []string
	var builder strings.Builder
	inBlock := false

	for scanner.Scan() {
		line := scanner.Text()
		if taskHeaderRe.MatchString(line) {
			if inBlock {
				blocks = append(blocks, builder.String())
				builder.Reset()
			}
			inBlock = true
			builder.WriteString(line)
			continue
		}

		if inBlock && taskEndRe.MatchString(line) {
			blocks = append(blocks, builder.String())
			builder.Reset()
			inBlock = false
			continue
		}

		if inBlock {
			builder.WriteString("\n")
			builder.WriteString(line)
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	if inBlock {
		blocks = append(blocks, builder.String())
	}

	return blocks, nil
}

var (
	taskHeaderRe          = regexp.MustCompile(`^\s*###\s+Task\s+`)
	taskEndRe             = regexp.MustCompile(`^\s*(---|##\s+)`)
	uncheckedLineRe       = regexp.MustCompile(`^(\s*)-\s*\[\s\]\s*(.*)$`)
	headingRe             = regexp.MustCompile(`^\s*##\s+`)
	anyHeadingRe          = regexp.MustCompile(`^\s*#+\s+`)
	openQuestionsHeaderRe = regexp.MustCompile(`^\s*##\s+open\s+questions\b`)
	contextHeaderRe       = regexp.MustCompile(`^(\s*)-\s*\*\*Context Bundle\*\*`)
	contextFieldRe        = regexp.MustCompile(`^\s*-\s*\*\*[^*]+\*\*`)
	fieldLineRe           = regexp.MustCompile(`^\s*-\s*\*\*%s\*\*`)
	headerIDRe            = regexp.MustCompile(`^\s*###\s+Task\s+(.+)$`)
	idFieldRe             = regexp.MustCompile(`^\s*-\s*\*\*ID\*\*\s*(.*)$`)
	contextEntryRe        = regexp.MustCompile("`([^`]*)`")
)

func addUnique(items *[]string, value string) {
	if strings.TrimSpace(value) == "" {
		return
	}
	for _, item := range *items {
		if item == value {
			return
		}
	}
	*items = append(*items, value)
}

func fileExists(path string) bool {
	if path == "" {
		return false
	}
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

func fileMatchesRegex(path string, pattern *regexp.Regexp) bool {
	data, err := os.ReadFile(path)
	if err != nil {
		return false
	}
	return pattern.Match(data)
}

func jsonHasDependency(path, dep string) bool {
	data, err := os.ReadFile(path)
	if err != nil {
		return false
	}

	var payload map[string]map[string]interface{}
	if err := json.Unmarshal(data, &payload); err == nil {
		for _, key := range []string{"dependencies", "devDependencies", "peerDependencies"} {
			if deps, ok := payload[key]; ok {
				if _, ok := deps[dep]; ok {
					return true
				}
			}
		}
	}

	return strings.Contains(string(data), fmt.Sprintf("\"%s\"", dep))
}

func validateTaskBlock(block, taskFile string, options ValidateOptions, baseDir string) []string {
	issues := []string{}
	taskLabel := taskLabel(block)

	for _, field := range []string{"ID", "Context Bundle", "DoD", "Checklist", "Dependencies"} {
		if !blockHasField(block, field) {
			issues = append(issues, fmt.Sprintf("PRD validation error: %s: %s: Missing required field: %s", taskFile, taskLabel, field))
		}
	}

	uncheckedCount := countUncheckedLines(block)
	if uncheckedCount == 0 {
		issues = append(issues, fmt.Sprintf("PRD validation error: %s: %s: Missing unchecked task line", taskFile, taskLabel))
	} else if uncheckedCount > 1 {
		issues = append(issues, fmt.Sprintf("PRD validation error: %s: %s: Multiple unchecked task lines (%d)", taskFile, taskLabel, uncheckedCount))
	}

	if !options.AllowMissingContext {
		entries := extractContextEntries(block)
		if len(entries) == 0 {
			issues = append(issues, fmt.Sprintf("PRD validation error: %s: %s: Context Bundle must include at least one file path", taskFile, taskLabel))
		} else {
			for _, entry := range entries {
				entry = strings.TrimSpace(entry)
				if entry == "" {
					continue
				}
				resolved := entry
				if filepath.IsAbs(entry) {
					if baseDir != "" {
						if rel, err := filepath.Rel(baseDir, entry); err != nil || strings.HasPrefix(rel, "..") {
							issues = append(issues, fmt.Sprintf("PRD validation error: %s: %s: Context Bundle path outside repo: %s", taskFile, taskLabel, entry))
							continue
						}
					}
				} else if baseDir != "" {
					resolved = filepath.Join(baseDir, entry)
				}

				if _, err := os.Stat(resolved); err != nil {
					issues = append(issues, fmt.Sprintf("PRD validation error: %s: %s: Context Bundle path not found: %s", taskFile, taskLabel, entry))
				}
			}
		}
	}

	return issues
}

func hasOpenQuestions(taskFile string) bool {
	file, err := os.Open(taskFile)
	if err != nil {
		return false
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		if anyHeadingRe.MatchString(scanner.Text()) {
			line := scanner.Text()
			if regexp.MustCompile(`(?i)^\s*#+\s+Open Questions\b`).MatchString(line) {
				return true
			}
		}
	}
	return false
}

func validateStrayUnchecked(taskFile string) []string {
	file, err := os.Open(taskFile)
	if err != nil {
		return nil
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	inBlock := false
	lineNumber := 0
	issues := []string{}

	for scanner.Scan() {
		lineNumber++
		line := scanner.Text()
		if taskHeaderRe.MatchString(line) {
			inBlock = true
		} else if inBlock && taskEndRe.MatchString(line) {
			inBlock = false
		}

		if !inBlock && regexp.MustCompile(`^\s*-\s*\[\s\]`).MatchString(line) {
			issues = append(issues, fmt.Sprintf("PRD validation error: %s: line %d: Unchecked task line outside task block", taskFile, lineNumber))
		}
	}

	return issues
}

func taskLabel(block string) string {
	idField := extractTaskIDField(block)
	if idField != "" {
		return idField
	}
	header := extractTaskHeaderID(block)
	if header != "" {
		return header
	}
	return "unknown"
}

func extractTaskHeaderID(block string) string {
	scanner := bufio.NewScanner(strings.NewReader(block))
	for scanner.Scan() {
		line := scanner.Text()
		match := headerIDRe.FindStringSubmatch(line)
		if len(match) > 1 {
			return strings.TrimSpace(match[1])
		}
	}
	return ""
}

func extractTaskIDField(block string) string {
	scanner := bufio.NewScanner(strings.NewReader(block))
	for scanner.Scan() {
		match := idFieldRe.FindStringSubmatch(scanner.Text())
		if len(match) > 1 {
			return strings.TrimSpace(match[1])
		}
	}
	return ""
}

func blockHasField(block, field string) bool {
	pattern := regexp.MustCompile(fmt.Sprintf(fieldLineRe.String(), regexp.QuoteMeta(field)))
	return pattern.MatchString(block)
}

func countUncheckedLines(block string) int {
	scanner := bufio.NewScanner(strings.NewReader(block))
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	count := 0
	for scanner.Scan() {
		if regexp.MustCompile(`^\s*-\s*\[\s\]`).MatchString(scanner.Text()) {
			count++
		}
	}
	return count
}

func extractContextEntries(block string) []string {
	lines := strings.Split(block, "\n")
	entries := []string{}
	inContext := false
	for _, line := range lines {
		if contextHeaderRe.MatchString(line) {
			inContext = true
		} else if inContext && contextFieldRe.MatchString(line) {
			break
		}

		if inContext {
			matches := contextEntryRe.FindAllStringSubmatch(line, -1)
			for _, match := range matches {
				if len(match) > 1 {
					entries = append(entries, match[1])
				}
			}
		}
	}

	return entries
}

func sanitizeTaskBlock(block, baseDir, allowedContextFile string) string {
	allowed := loadAllowedContext(allowedContextFile)
	entries := extractContextEntries(block)

	valid := []string{}
	for _, entry := range entries {
		display := contextDisplayPath(entry, baseDir)
		if !contextEntryExists(display, baseDir) {
			continue
		}
		if len(allowed) > 0 {
			if _, ok := allowed[display]; !ok {
				continue
			}
		}
		addUnique(&valid, display)
	}

	if len(valid) == 0 {
		if fallback := pickFallbackContext(baseDir, allowedContextFile); fallback != "" {
			valid = []string{fallback}
		}
	}

	contextLine := "- **Context Bundle**"
	if len(valid) > 0 {
		formatted := ""
		for _, entry := range valid {
			if formatted == "" {
				formatted = fmt.Sprintf("`%s`", entry)
			} else {
				formatted += fmt.Sprintf(", `%s`", entry)
			}
		}
		contextLine = fmt.Sprintf("- **Context Bundle** %s", formatted)
	}

	lines := strings.Split(block, "\n")
	var output strings.Builder
	inContextBlock := false
	uncheckedSeen := false

	for _, line := range lines {
		if match := contextHeaderRe.FindStringSubmatch(line); len(match) > 0 {
			indent := match[1]
			output.WriteString(indent + contextLine + "\n")
			inContextBlock = true
			continue
		}

		if inContextBlock {
			if contextFieldRe.MatchString(line) {
				inContextBlock = false
			} else {
				continue
			}
		}

		if uncheckedLineRe.MatchString(line) {
			if uncheckedSeen {
				line = uncheckedLineRe.ReplaceAllString(line, "$1- $2")
			} else {
				uncheckedSeen = true
			}
		}

		output.WriteString(line + "\n")
	}

	return output.String()
}

func loadAllowedContext(filePath string) map[string]struct{} {
	allowed := map[string]struct{}{}
	if strings.TrimSpace(filePath) == "" {
		return allowed
	}
	data, err := os.ReadFile(filePath)
	if err != nil {
		return allowed
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		allowed[line] = struct{}{}
	}
	return allowed
}

func pickFallbackContext(baseDir, allowedFile string) string {
	if allowedFile != "" {
		data, err := os.ReadFile(allowedFile)
		if err == nil {
			for _, line := range strings.Split(string(data), "\n") {
				line = strings.TrimSpace(line)
				if line == "" {
					continue
				}
				if baseDir != "" {
					if _, err := os.Stat(filepath.Join(baseDir, line)); err == nil {
						return line
					}
				}
			}
		}
	}

	if baseDir != "" {
		if _, err := os.Stat(filepath.Join(baseDir, "README.md")); err == nil {
			return "README.md"
		}
	}

	return ""
}

func contextEntryExists(entry, baseDir string) bool {
	if strings.TrimSpace(entry) == "" {
		return false
	}
	if filepath.IsAbs(entry) {
		if baseDir != "" {
			if rel, err := filepath.Rel(baseDir, entry); err != nil || strings.HasPrefix(rel, "..") {
				return false
			}
		}
		_, err := os.Stat(entry)
		return err == nil
	}

	if baseDir == "" {
		return false
	}
	_, err := os.Stat(filepath.Join(baseDir, entry))
	return err == nil
}

func contextDisplayPath(entry, baseDir string) string {
	if strings.TrimSpace(entry) == "" {
		return ""
	}
	if filepath.IsAbs(entry) && baseDir != "" {
		if rel, err := filepath.Rel(baseDir, entry); err == nil && !strings.HasPrefix(rel, "..") {
			return rel
		}
	}
	return entry
}
