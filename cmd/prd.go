package cmd

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/goosewin/gralph/internal/backend"
	"github.com/goosewin/gralph/internal/config"
	"github.com/goosewin/gralph/internal/prd"
	"github.com/spf13/cobra"
)

var (
	prdAllowMissingContext bool

	prdCreateDir           string
	prdCreateOutput        string
	prdCreateGoal          string
	prdCreateConstraints   string
	prdCreateContext       string
	prdCreateSources       string
	prdCreateBackend       string
	prdCreateModel         string
	prdCreateAllowMissing  bool
	prdCreateMultiline     bool
	prdCreateInteractive   bool
	prdCreateNoInteractive bool
	prdCreateForce         bool
)

var prdCmd = &cobra.Command{
	Use:   "prd",
	Short: "Generate or validate PRDs",
}

var prdCheckCmd = &cobra.Command{
	Use:   "check <file>",
	Short: "Validate a PRD file",
	Args:  cobra.ExactArgs(1),
	RunE:  runPrdCheck,
}

var prdCreateCmd = &cobra.Command{
	Use:     "create [dir]",
	Short:   "Generate a spec-compliant PRD",
	Args:    cobra.MaximumNArgs(1),
	RunE:    runPrdCreate,
	Aliases: []string{"init", "new"},
}

func init() {
	prdCheckCmd.Flags().BoolVar(&prdAllowMissingContext, "allow-missing-context", false, "Allow missing Context Bundle paths")

	prdCreateCmd.Flags().StringVar(&prdCreateDir, "dir", "", "Project directory")
	prdCreateCmd.Flags().StringVarP(&prdCreateOutput, "output", "o", "", "Output PRD file path")
	prdCreateCmd.Flags().StringVar(&prdCreateGoal, "goal", "", "Short description of what to build")
	prdCreateCmd.Flags().StringVar(&prdCreateConstraints, "constraints", "", "Constraints or non-functional requirements")
	prdCreateCmd.Flags().StringVar(&prdCreateContext, "context", "", "Extra context files (comma-separated)")
	prdCreateCmd.Flags().StringVar(&prdCreateSources, "sources", "", "External URLs or references (comma-separated)")
	prdCreateCmd.Flags().StringVarP(&prdCreateBackend, "backend", "b", "", "Backend for PRD generation")
	prdCreateCmd.Flags().StringVarP(&prdCreateModel, "model", "m", "", "Model override for PRD generation")
	prdCreateCmd.Flags().BoolVar(&prdCreateAllowMissing, "allow-missing-context", false, "Allow missing Context Bundle paths")
	prdCreateCmd.Flags().BoolVar(&prdCreateMultiline, "multiline", false, "Enable multiline prompts (interactive)")
	prdCreateCmd.Flags().BoolVar(&prdCreateInteractive, "interactive", false, "Force interactive prompts")
	prdCreateCmd.Flags().BoolVar(&prdCreateNoInteractive, "no-interactive", false, "Disable interactive prompts")
	prdCreateCmd.Flags().BoolVar(&prdCreateForce, "force", false, "Overwrite existing output file")

	prdCmd.AddCommand(prdCheckCmd)
	prdCmd.AddCommand(prdCreateCmd)
	rootCmd.AddCommand(prdCmd)
}

func runPrdCheck(cmd *cobra.Command, args []string) error {
	taskFile := strings.TrimSpace(args[0])
	if taskFile == "" {
		return errors.New("task file is required")
	}

	err := prd.ValidateFile(taskFile, &prd.ValidateOptions{AllowMissingContext: prdAllowMissingContext})
	if err != nil {
		return err
	}

	fmt.Printf("PRD validation passed: %s\n", taskFile)
	return nil
}

func runPrdCreate(cmd *cobra.Command, args []string) error {
	targetDir := strings.TrimSpace(prdCreateDir)
	if targetDir == "" && len(args) > 0 {
		targetDir = strings.TrimSpace(args[0])
	}
	if targetDir == "" {
		targetDir = "."
	}

	absDir, err := filepath.Abs(targetDir)
	if err != nil {
		return fmt.Errorf("resolve project directory: %w", err)
	}
	info, err := os.Stat(absDir)
	if err != nil || !info.IsDir() {
		return fmt.Errorf("directory does not exist: %s", absDir)
	}
	targetDir = absDir

	if _, err := config.LoadConfig(targetDir); err != nil {
		return err
	}

	interactive, err := resolvePrdInteractive(cmd)
	if err != nil {
		return err
	}
	if interactive {
		fmt.Fprintln(os.Stderr, "Interactive mode: follow the numbered steps. Press Enter to skip optional prompts.")
	}

	goal := strings.TrimSpace(prdCreateGoal)
	if goal == "" && interactive {
		fmt.Fprintln(os.Stderr, "Step 1/6: Project goal (required). Press Enter to skip if already provided.")
		if prdCreateMultiline {
			goal = strings.TrimSpace(promptMultiline("Goal (required)"))
		} else {
			goal = strings.TrimSpace(promptInput("Goal (required)", ""))
		}
	}
	if goal == "" {
		return errors.New("goal is required. Use --goal or run interactively")
	}

	constraints := strings.TrimSpace(prdCreateConstraints)
	if constraints == "" && interactive {
		fmt.Fprintln(os.Stderr, "Step 2/6: Constraints or requirements (optional). Press Enter to skip.")
		if prdCreateMultiline {
			constraints = strings.TrimSpace(promptMultiline("Constraints (optional)"))
		} else {
			constraints = strings.TrimSpace(promptInput("Constraints (optional)", ""))
		}
	}
	if constraints == "" {
		constraints = "None."
	}

	sourcesInput := strings.TrimSpace(prdCreateSources)
	if sourcesInput == "" && interactive {
		fmt.Fprintln(os.Stderr, "Step 3/6: External sources (comma-separated URLs). Press Enter to skip.")
		sourcesInput = strings.TrimSpace(promptInput("Sources (optional)", ""))
	}

	outputPath := strings.TrimSpace(prdCreateOutput)
	if outputPath == "" && interactive {
		fmt.Fprintln(os.Stderr, "Step 4/6: Output file (press Enter for PRD.generated.md).")
		outputPath = strings.TrimSpace(promptInput("PRD output file", "PRD.generated.md"))
	}
	if outputPath == "" {
		outputPath = "PRD.generated.md"
	}
	if !filepath.IsAbs(outputPath) {
		outputPath = filepath.Join(targetDir, outputPath)
	}

	if _, err := os.Stat(outputPath); err == nil && !prdCreateForce {
		if interactive {
			overwrite := strings.TrimSpace(promptInput("File exists. Overwrite? (y/N)", "N"))
			if !isYes(overwrite) {
				return fmt.Errorf("output file exists: %s (use --force to overwrite)", outputPath)
			}
		} else {
			return fmt.Errorf("output file exists: %s (use --force to overwrite)", outputPath)
		}
	}

	backendName := strings.TrimSpace(prdCreateBackend)
	if backendName == "" {
		backendName = configValue("defaults.backend")
	}
	if backendName == "" {
		backendName = backend.DefaultName()
	}

	model := strings.TrimSpace(prdCreateModel)
	if model == "" {
		model = configValue("defaults.model")
	}
	if model == "" {
		switch strings.ToLower(backendName) {
		case "opencode":
			model = configValue("opencode.default_model")
		case "gemini":
			model = configValue("gemini.default_model")
		case "codex":
			model = configValue("codex.default_model")
		}
	}

	backendInstance, ok := backend.Get(backendName)
	if !ok {
		return fmt.Errorf("backend not found: %s", backendName)
	}
	if err := backendInstance.CheckInstalled(); err != nil {
		return err
	}

	fmt.Fprintf(os.Stderr, "Generating PRD in %s\n", targetDir)
	fmt.Fprintf(os.Stderr, "Output file: %s\n", outputPath)
	fmt.Fprintf(os.Stderr, "Using backend: %s\n", backendName)
	if model != "" {
		fmt.Fprintf(os.Stderr, "Using model: %s\n", model)
	}

	stackSummary, err := prd.DetectStack(targetDir)
	if err != nil {
		return err
	}
	detectedStackList := stackListDisplay(stackSummary.IDs, "None detected")

	if interactive {
		fmt.Fprintln(os.Stderr, "Step 5/6: Stack detection")
		if len(stackSummary.IDs) > 0 {
			fmt.Fprintln(os.Stderr, "Detected stacks:")
			for idx, name := range stackSummary.IDs {
				fmt.Fprintf(os.Stderr, "  %d) %s\n", idx+1, name)
			}
		} else {
			fmt.Fprintln(os.Stderr, "No stack files detected.")
		}
	}

	if interactive && len(stackSummary.IDs) > 1 {
		confirm := strings.TrimSpace(promptInput("Use all detected stacks? (Y/n)", "Y"))
		if strings.HasPrefix(strings.ToLower(confirm), "n") {
			selection := strings.TrimSpace(promptInput("Select stacks by number (comma-separated, press Enter for all)", ""))
			if selection != "" {
				selected, ok := selectStackIDs(stackSummary.IDs, selection)
				if ok {
					stackSummary.SelectedIDs = selected
				} else {
					stackSummary.SelectedIDs = append([]string(nil), stackSummary.IDs...)
					fmt.Fprintln(os.Stderr, "Warning: no valid stack selection provided; using all detected stacks.")
				}
			}
		}
	}

	stackSummaryText := formatStackSummary(stackSummary, 2)
	stackSummaryPrompt := "None detected."
	if strings.TrimSpace(stackSummaryText) != "" {
		stackSummaryPrompt = stackSummaryText
	}

	configContextFiles := configValue("defaults.context_files")
	contextFiles := buildContextFileList(targetDir, prdCreateContext, configContextFiles)
	contextSection := "None."
	if len(contextFiles) > 0 {
		contextSection = strings.Join(contextFiles, "\n")
	}

	sourcesList, sourcesOrigin := resolveSourcesList(sourcesInput, stackSummary, goal)
	sourcesSection := "None."
	warningsSection := ""
	if len(sourcesList) > 0 {
		sourcesSection = strings.Join(sourcesList, "\n")
	} else {
		sourcesOrigin = "none"
		warningsSection = "No reliable external sources were provided or discovered. Verify requirements and stack assumptions before implementation."
	}

	if interactive {
		fmt.Fprintln(os.Stderr, "Step 6/6: Review summary")
	} else {
		fmt.Fprintln(os.Stderr, "Summary")
	}
	fmt.Println("Summary:")
	fmt.Printf("  Goal: %s\n", goal)
	fmt.Printf("  Constraints: %s\n", constraints)
	fmt.Printf("  Output: %s\n", outputPath)
	fmt.Printf("  Detected stacks: %s\n", detectedStackList)
	fmt.Printf("  Sources: %s\n", sourcesOrigin)
	fmt.Printf("  Allow missing context: %t\n", prdCreateAllowMissing)
	fmt.Printf("  Backend: %s\n", backendName)
	if model != "" {
		fmt.Printf("  Model: %s\n", model)
	}
	fmt.Println("  Context files:")
	if len(contextFiles) > 0 {
		for _, entry := range contextFiles {
			fmt.Printf("    - %s\n", entry)
		}
	} else {
		fmt.Println("    - None")
	}

	if interactive {
		proceed := strings.TrimSpace(promptInput("Proceed to generate PRD? (y/N)", "N"))
		if !isYes(proceed) {
			return errors.New("PRD generation cancelled")
		}
	} else {
		fmt.Fprintln(os.Stderr, "Non-interactive mode: skipping confirmation.")
	}

	templateText, err := getPRDTemplateText(targetDir)
	if err != nil {
		return err
	}

	warningPrompt := "None."
	if warningsSection != "" {
		warningPrompt = warningsSection
	}

	prompt := buildPRDPrompt(prdPromptOptions{
		TargetDir:    targetDir,
		Goal:         goal,
		Constraints:  constraints,
		StackSummary: stackSummaryPrompt,
		Sources:      sourcesSection,
		Warnings:     warningPrompt,
		ContextFiles: contextSection,
		TemplateText: templateText,
	})

	tmpOutput, err := os.CreateTemp("", "gralph-prd-output-*")
	if err != nil {
		return err
	}
	tmpOutputPath := tmpOutput.Name()
	_ = tmpOutput.Close()

	tmpPRD, err := os.CreateTemp("", "gralph-prd-*")
	if err != nil {
		return err
	}
	tmpPRDPath := tmpPRD.Name()
	_ = tmpPRD.Close()

	tmpErr, err := os.CreateTemp("", "gralph-prd-err-*")
	if err != nil {
		return err
	}
	tmpErrPath := tmpErr.Name()
	_ = tmpErr.Close()

	rawOutput, err := os.CreateTemp("", "gralph-prd-raw-*")
	if err != nil {
		return err
	}
	rawOutputPath := rawOutput.Name()
	_ = rawOutput.Close()

	defer func() {
		_ = os.Remove(tmpOutputPath)
		_ = os.Remove(tmpErrPath)
	}()

	if err := backendInstance.RunIteration(context.Background(), backend.IterationOptions{
		Prompt:        prompt,
		Model:         model,
		OutputFile:    tmpOutputPath,
		RawOutputFile: rawOutputPath,
	}); err != nil {
		if fileHasContent(rawOutputPath) {
			fmt.Fprintf(os.Stderr, "Warning: raw backend output saved to: %s\n", rawOutputPath)
		} else {
			_ = os.Remove(rawOutputPath)
		}
		return fmt.Errorf("PRD generation failed: %w", err)
	}

	result, err := backendInstance.ParseText(tmpOutputPath)
	if err != nil {
		if fileHasContent(rawOutputPath) {
			fmt.Fprintf(os.Stderr, "Warning: raw backend output saved to: %s\n", rawOutputPath)
		} else {
			_ = os.Remove(rawOutputPath)
		}
		return fmt.Errorf("PRD generation failed: %w", err)
	}
	if strings.TrimSpace(result) == "" {
		if fileHasContent(rawOutputPath) {
			fmt.Fprintf(os.Stderr, "Warning: raw backend output saved to: %s\n", rawOutputPath)
		} else {
			_ = os.Remove(rawOutputPath)
		}
		return errors.New("PRD generation returned empty output")
	}

	if err := os.WriteFile(tmpPRDPath, []byte(result+"\n"), 0o644); err != nil {
		return fmt.Errorf("write PRD output: %w", err)
	}

	allowedContextFile := ""
	if len(contextFiles) > 0 {
		path, err := writeTempList(contextFiles)
		if err != nil {
			return err
		}
		allowedContextFile = path
		defer os.Remove(path)
	}

	if err := prd.SanitizeGeneratedFile(tmpPRDPath, targetDir, allowedContextFile); err != nil {
		return err
	}

	fmt.Fprintln(os.Stderr, "Validating generated PRD")
	validateErr := prd.ValidateFile(tmpPRDPath, &prd.ValidateOptions{AllowMissingContext: prdCreateAllowMissing, BaseDir: targetDir})
	if validateErr != nil {
		_ = os.WriteFile(tmpErrPath, []byte(validateErr.Error()), 0o644)
		fmt.Fprintln(os.Stderr, "Warning: generated PRD failed validation.")
		fmt.Fprintln(os.Stderr, validateErr.Error())

		invalidPath := outputPath
		if !prdCreateForce {
			if strings.HasSuffix(strings.ToLower(outputPath), ".md") {
				invalidPath = strings.TrimSuffix(outputPath, ".md") + ".invalid.md"
			} else {
				invalidPath = outputPath + ".invalid"
			}
		}

		if err := os.MkdirAll(filepath.Dir(invalidPath), 0o755); err != nil {
			return err
		}
		if err := os.Rename(tmpPRDPath, invalidPath); err != nil {
			return err
		}
		fmt.Fprintf(os.Stderr, "Warning: saved invalid PRD to: %s\n", invalidPath)
		if fileHasContent(rawOutputPath) {
			fmt.Fprintf(os.Stderr, "Warning: raw backend output saved to: %s\n", rawOutputPath)
		} else {
			_ = os.Remove(rawOutputPath)
		}
		return errors.New("generated PRD failed validation")
	}

	if err := os.MkdirAll(filepath.Dir(outputPath), 0o755); err != nil {
		return err
	}
	if err := os.Rename(tmpPRDPath, outputPath); err != nil {
		return err
	}
	_ = os.Remove(rawOutputPath)

	fmt.Printf("PRD created: %s\n", outputPath)
	relativeOutput := outputPath
	if rel, err := filepath.Rel(targetDir, outputPath); err == nil && !strings.HasPrefix(rel, "..") {
		relativeOutput = rel
	}
	fmt.Println("Next step:")
	fmt.Printf("  gralph start %s --task-file %s --no-tmux --backend %s", targetDir, relativeOutput, backendName)
	if model != "" {
		fmt.Printf(" --model %s", model)
	}
	fmt.Println(" --strict-prd")

	return nil
}

func resolvePrdInteractive(cmd *cobra.Command) (bool, error) {
	flags := cmd.Flags()
	if flags.Changed("interactive") && flags.Changed("no-interactive") {
		return false, errors.New("cannot use --interactive and --no-interactive together")
	}

	if flags.Changed("interactive") {
		return prdCreateInteractive, nil
	}
	if flags.Changed("no-interactive") {
		return false, nil
	}

	if !isTerminal(os.Stdin) || !isTerminal(os.Stdout) {
		return false, nil
	}
	return true, nil
}

func isTerminal(file *os.File) bool {
	if file == nil {
		return false
	}
	info, err := file.Stat()
	if err != nil {
		return false
	}
	return info.Mode()&os.ModeCharDevice != 0
}

func promptInput(prompt, defaultValue string) string {
	if defaultValue != "" {
		fmt.Fprintf(os.Stderr, "%s [%s]: ", prompt, defaultValue)
	} else {
		fmt.Fprintf(os.Stderr, "%s: ", prompt)
	}

	reader := bufio.NewReader(os.Stdin)
	input, _ := reader.ReadString('\n')
	input = strings.TrimRight(input, "\r\n")
	if input == "" {
		input = defaultValue
	}
	return input
}

func promptMultiline(prompt string) string {
	fmt.Fprintf(os.Stderr, "%s (finish with empty line):\n", prompt)
	reader := bufio.NewReader(os.Stdin)
	lines := []string{}
	for {
		line, err := reader.ReadString('\n')
		if err != nil && !errors.Is(err, io.EOF) {
			break
		}
		line = strings.TrimRight(line, "\r\n")
		if line == "" {
			break
		}
		lines = append(lines, line)
		if errors.Is(err, io.EOF) {
			break
		}
	}
	return strings.Join(lines, "\n")
}

func isYes(value string) bool {
	value = strings.TrimSpace(strings.ToLower(value))
	return value == "y" || value == "yes"
}

func stackListDisplay(values []string, empty string) string {
	if len(values) == 0 {
		return empty
	}
	return strings.Join(values, ", ")
}

func selectStackIDs(ids []string, selection string) ([]string, bool) {
	selection = strings.TrimSpace(selection)
	if selection == "" {
		return append([]string(nil), ids...), true
	}
	parts := strings.Split(selection, ",")
	selected := []string{}
	seen := map[int]struct{}{}
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}
		index, err := strconv.Atoi(part)
		if err != nil || index < 1 || index > len(ids) {
			continue
		}
		if _, ok := seen[index]; ok {
			continue
		}
		seen[index] = struct{}{}
		selected = append(selected, ids[index-1])
	}
	if len(selected) == 0 {
		return nil, false
	}
	return selected, true
}

func formatStackSummary(summary prd.StackSummary, headingLevel int) string {
	if headingLevel <= 0 {
		headingLevel = 2
	}
	headerPrefix := strings.Repeat("#", headingLevel)

	stacksLine := stackListDisplay(summary.IDs, "Unknown")
	languagesLine := stackListDisplay(summary.Languages, "Unknown")
	frameworksLine := stackListDisplay(summary.Frameworks, "None detected")
	toolsLine := stackListDisplay(summary.Tools, "None detected")
	runtimesLine := stackListDisplay(summary.Runtimes, "Unknown")
	packageManagersLine := stackListDisplay(summary.PackageManagers, "None detected")

	var builder strings.Builder
	builder.WriteString(headerPrefix + " Stack Summary\n\n")
	builder.WriteString("- Stacks: " + stacksLine + "\n")
	builder.WriteString("- Languages: " + languagesLine + "\n")
	builder.WriteString("- Runtimes: " + runtimesLine + "\n")
	builder.WriteString("- Frameworks: " + frameworksLine + "\n")
	builder.WriteString("- Tools: " + toolsLine + "\n")
	builder.WriteString("- Package managers: " + packageManagersLine + "\n")

	if len(summary.SelectedIDs) > 0 && len(summary.SelectedIDs) < len(summary.IDs) {
		builder.WriteString("- Stack focus: " + strings.Join(summary.SelectedIDs, ", ") + "\n")
	}

	builder.WriteString("\nEvidence:\n")
	if len(summary.Evidence) > 0 {
		for _, item := range summary.Evidence {
			builder.WriteString("- " + item + "\n")
		}
	} else {
		builder.WriteString("- None found\n")
	}

	return builder.String()
}

func buildContextFileList(targetDir, userList, configList string) []string {
	entries := []string{}
	seen := map[string]struct{}{}

	addEntry := func(item string) {
		item = strings.TrimSpace(item)
		if item == "" {
			return
		}

		resolved := item
		if !filepath.IsAbs(item) {
			resolved = filepath.Join(targetDir, item)
		}

		info, err := os.Stat(resolved)
		if err != nil || info.IsDir() {
			return
		}

		display := resolved
		if rel, err := filepath.Rel(targetDir, resolved); err == nil && !strings.HasPrefix(rel, "..") {
			display = rel
		}

		if _, ok := seen[display]; ok {
			return
		}
		seen[display] = struct{}{}
		entries = append(entries, display)
	}

	for _, item := range splitCSV(configList) {
		addEntry(item)
	}
	for _, item := range splitCSV(userList) {
		addEntry(item)
	}

	for _, item := range []string{
		"README.md",
		"ARCHITECTURE.md",
		"DECISIONS.md",
		"CHANGELOG.md",
		"RISK_REGISTER.md",
		"PROCESS.md",
		"PRD.template.md",
		"bin/gralph",
		"config/default.yaml",
		"opencode.json",
		"completions/gralph.bash",
		"completions/gralph.zsh",
	} {
		addEntry(item)
	}

	addGlobEntries := func(pattern string) {
		matches, _ := filepath.Glob(filepath.Join(targetDir, pattern))
		for _, match := range matches {
			if rel, err := filepath.Rel(targetDir, match); err == nil {
				addEntry(rel)
			} else {
				addEntry(match)
			}
		}
	}

	addGlobEntries(filepath.Join("lib", "*.sh"))
	addGlobEntries(filepath.Join("lib", "backends", "*.sh"))
	addGlobEntries(filepath.Join("tests", "*.sh"))

	return entries
}

func splitCSV(raw string) []string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return nil
	}
	parts := strings.Split(raw, ",")
	output := make([]string, 0, len(parts))
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if part != "" {
			output = append(output, part)
		}
	}
	return output
}

func resolveSourcesList(input string, summary prd.StackSummary, goal string) ([]string, string) {
	sources := normalizeList(splitCSV(input))
	origin := "user"

	if len(sources) == 0 {
		origin = "official"
		sources = normalizeList(collectOfficialSources(summary))
	}

	if len(sources) == 0 {
		origin = "search"
		query := strings.TrimSpace(goal)
		if len(summary.SelectedIDs) > 0 {
			query = strings.TrimSpace(fmt.Sprintf("%s %s documentation", query, strings.Join(summary.SelectedIDs, " ")))
		}
		sources = normalizeList(searchWebSources(query, 5))
	}

	sources = dedupeList(sources)
	return sources, origin
}

func normalizeList(values []string) []string {
	output := make([]string, 0, len(values))
	for _, item := range values {
		item = strings.TrimSpace(item)
		if item != "" {
			output = append(output, item)
		}
	}
	return output
}

func dedupeList(values []string) []string {
	seen := map[string]struct{}{}
	output := make([]string, 0, len(values))
	for _, item := range values {
		if item == "" {
			continue
		}
		if _, ok := seen[item]; ok {
			continue
		}
		seen[item] = struct{}{}
		output = append(output, item)
	}
	return output
}

func collectOfficialSources(summary prd.StackSummary) []string {
	sources := []string{}
	add := func(value string) {
		if value != "" {
			sources = append(sources, value)
		}
	}

	for _, item := range summary.SelectedIDs {
		switch item {
		case "Node.js":
			add("https://nodejs.org/docs/latest/api/")
		case "Go":
			add("https://go.dev/doc/")
		case "Rust":
			add("https://doc.rust-lang.org/")
		case "Python":
			add("https://docs.python.org/3/")
		case "Ruby":
			add("https://www.ruby-lang.org/en/documentation/")
		case "Java":
			add("https://docs.oracle.com/en/java/")
		case ".NET":
			add("https://learn.microsoft.com/dotnet/")
		case "PHP":
			add("https://www.php.net/manual/en/")
		case "Elixir":
			add("https://elixir-lang.org/docs.html")
		}
	}

	for _, item := range summary.Frameworks {
		switch item {
		case "React":
			add("https://react.dev/")
		case "Next.js":
			add("https://nextjs.org/docs")
		case "Vue":
			add("https://vuejs.org/guide/")
		case "Angular":
			add("https://angular.dev/guide")
		case "Svelte":
			add("https://svelte.dev/docs")
		case "Nuxt":
			add("https://nuxt.com/docs")
		case "Express":
			add("https://expressjs.com/")
		case "Fastify":
			add("https://www.fastify.io/docs/latest/")
		case "NestJS":
			add("https://docs.nestjs.com/")
		case "Django":
			add("https://docs.djangoproject.com/en/stable/")
		case "Flask":
			add("https://flask.palletsprojects.com/")
		case "FastAPI":
			add("https://fastapi.tiangolo.com/")
		case "Rails":
			add("https://guides.rubyonrails.org/")
		case "Sinatra":
			add("https://sinatrarb.com/documentation.html")
		case "Phoenix":
			add("https://hexdocs.pm/phoenix/")
		case "Laravel":
			add("https://laravel.com/docs")
		case "Spring Boot":
			add("https://docs.spring.io/spring-boot/docs/current/reference/html/")
		}
	}

	for _, item := range summary.Tools {
		switch item {
		case "Vite":
			add("https://vitejs.dev/guide/")
		case "Docker":
			add("https://docs.docker.com/")
		case "Docker Compose":
			add("https://docs.docker.com/compose/")
		case "Make":
			add("https://www.gnu.org/software/make/manual/make.html")
		case "Terraform":
			add("https://developer.hashicorp.com/terraform/docs")
		case "Go modules":
			add("https://go.dev/ref/mod")
		case "Cargo":
			add("https://doc.rust-lang.org/cargo/")
		case "Maven":
			add("https://maven.apache.org/guides/")
		case "Gradle":
			add("https://docs.gradle.org/current/userguide/userguide.html")
		case "Poetry":
			add("https://python-poetry.org/docs/")
		}
	}

	for _, item := range summary.PackageManagers {
		switch item {
		case "pnpm":
			add("https://pnpm.io/")
		case "yarn":
			add("https://yarnpkg.com/")
		case "npm":
			add("https://docs.npmjs.com/")
		case "bun":
			add("https://bun.sh/docs")
		}
	}

	return sources
}

func searchWebSources(query string, maxResults int) []string {
	query = strings.TrimSpace(query)
	if query == "" || maxResults <= 0 {
		return nil
	}

	encoded := strings.ReplaceAll(query, " ", "+")
	url := fmt.Sprintf("https://lite.duckduckgo.com/lite/?q=%s", encoded)
	client := &http.Client{Timeout: 8 * time.Second}
	resp, err := client.Get(url)
	if err != nil {
		return nil
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil
	}

	re := regexp.MustCompile(`https?://[^"\s]+`)
	matches := re.FindAllString(string(data), -1)
	results := []string{}
	for _, match := range matches {
		if strings.Contains(match, "duckduckgo.com") {
			continue
		}
		results = append(results, match)
		if len(results) >= maxResults {
			break
		}
	}
	return results
}

func getPRDTemplateText(targetDir string) (string, error) {
	candidates := []string{}
	if targetDir != "" {
		candidates = append(candidates, filepath.Join(targetDir, "PRD.template.md"))
	}
	if exe, err := os.Executable(); err == nil {
		exeDir := filepath.Dir(exe)
		candidates = append(candidates, filepath.Join(exeDir, "..", "PRD.template.md"))
	}

	for _, candidate := range candidates {
		if data, err := os.ReadFile(candidate); err == nil {
			return string(data), nil
		}
	}

	return defaultPRDTemplate(), nil
}

func defaultPRDTemplate() string {
	return strings.Join([]string{
		"## Overview",
		"",
		"Briefly describe the project, goals, and intended users.",
		"",
		"## Problem Statement",
		"",
		"- What problem does this solve?",
		"- What pain points exist today?",
		"",
		"## Solution",
		"",
		"High-level solution summary.",
		"",
		"---",
		"",
		"## Functional Requirements",
		"",
		"### FR-1: Core Feature",
		"",
		"Describe the primary user-facing behavior.",
		"",
		"### FR-2: Secondary Feature",
		"",
		"Describe supporting behavior.",
		"",
		"---",
		"",
		"## Non-Functional Requirements",
		"",
		"### NFR-1: Performance",
		"",
		"- Example: Response times under 200ms for key operations.",
		"",
		"### NFR-2: Reliability",
		"",
		"- Example: Crash recovery or retries where appropriate.",
		"",
		"---",
		"",
		"## Implementation Tasks",
		"",
		"Each task must use a `### Task <ID>` block header and include the required fields.",
		"Each task block must contain exactly one unchecked task line.",
		"",
		"### Task EX-1",
		"",
		"- **ID** EX-1",
		"- **Context Bundle** `path/to/file`, `path/to/other`",
		"- **DoD** Define the done criteria for this task.",
		"- **Checklist**",
		"  * First verification item.",
		"  * Second verification item.",
		"- **Dependencies** None",
		"- [ ] EX-1 Short task summary",
		"",
		"---",
		"",
		"## Success Criteria",
		"",
		"- Define measurable outcomes that indicate completion.",
		"",
		"---",
		"",
		"## Sources",
		"",
		"- List authoritative URLs used as source of truth.",
		"",
		"---",
		"",
		"## Warnings",
		"",
		"- Only include this section if no reliable sources were found.",
		"- State what is missing and what must be verified.",
		"",
	}, "\n")
}

type prdPromptOptions struct {
	TargetDir    string
	Goal         string
	Constraints  string
	StackSummary string
	Sources      string
	Warnings     string
	ContextFiles string
	TemplateText string
}

func buildPRDPrompt(opts prdPromptOptions) string {
	return strings.Join([]string{
		"You are generating a gralph PRD in markdown. The output must be spec-compliant and grounded in the repository.",
		"",
		fmt.Sprintf("Project directory: %s", opts.TargetDir),
		"",
		"Goal:",
		opts.Goal,
		"",
		"Constraints:",
		opts.Constraints,
		"",
		"Detected stack summary (from repository files):",
		opts.StackSummary,
		"",
		"Sources (authoritative URLs or references):",
		opts.Sources,
		"",
		"Warnings (only include in the PRD if Sources is empty):",
		opts.Warnings,
		"",
		"Context files (read these first if present):",
		opts.ContextFiles,
		"",
		"Requirements:",
		"- Output only the PRD markdown with no commentary or code fences.",
		"- Use ASCII only.",
		"- Do not include an \"Open Questions\" section.",
		"- Do not use any checkboxes outside task blocks.",
		"- Context Bundle entries must be real files in the repo and must be selected from the Context files list above.",
		"- If a task creates new files, do not list the new files in Context Bundle; cite the closest existing files instead.",
		"- Use atomic, granular tasks grounded in the repo and context files.",
		"- Each task block must use a '### Task <ID>' header and include **ID**, **Context Bundle**, **DoD**, **Checklist**, **Dependencies**.",
		"- Each task block must contain exactly one unchecked task line like '- [ ] <ID> <summary>'.",
		"- If Sources is empty, include a 'Warnings' section with the warning text above and no checkboxes.",
		"- Do not invent stack, frameworks, or files not supported by the context files and stack summary.",
		"",
		"Template:",
		opts.TemplateText,
		"",
	}, "\n")
}

func writeTempList(values []string) (string, error) {
	temp, err := os.CreateTemp("", "gralph-prd-context-*")
	if err != nil {
		return "", err
	}
	defer temp.Close()

	for _, item := range values {
		if _, err := temp.WriteString(item + "\n"); err != nil {
			return "", err
		}
	}

	return temp.Name(), nil
}

func fileHasContent(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.Size() > 0
}
