package core

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

const DefaultPromptTemplate = "Read {task_file} carefully. Find any task marked '- [ ]' (unchecked).\n\nIf unchecked tasks exist:\n- Complete ONE task fully\n- Mark it '- [x]' in {task_file}\n- Commit changes\n- Exit normally (do NOT output completion promise)\n\nIf ZERO '- [ ]' remain (all complete):\n- Verify by searching the file\n- Output ONLY: <promise>{completion_marker}</promise>\n\nCRITICAL: Never mention the promise unless outputting it as the completion signal.\n\n{context_files_section}Task Block:\n{task_block}\n\nIteration: {iteration}/{max_iterations}"

// RenderPromptTemplate substitutes template variables for a prompt.
func RenderPromptTemplate(template, taskFile, completionMarker string, iteration, maxIterations int, taskBlock string, contextFiles []string) string {
	if strings.TrimSpace(taskBlock) == "" {
		taskBlock = "No task block available."
	}

	contextSection := ""
	if len(contextFiles) > 0 {
		contextSection = "Context Files (read these first):\n" + strings.Join(contextFiles, "\n") + "\n"
	}

	rendered := template
	rendered = strings.ReplaceAll(rendered, "{task_file}", taskFile)
	rendered = strings.ReplaceAll(rendered, "{completion_marker}", completionMarker)
	rendered = strings.ReplaceAll(rendered, "{iteration}", strconv.Itoa(iteration))
	rendered = strings.ReplaceAll(rendered, "{max_iterations}", strconv.Itoa(maxIterations))
	rendered = strings.ReplaceAll(rendered, "{task_block}", taskBlock)
	rendered = strings.ReplaceAll(rendered, "{context_files}", strings.Join(contextFiles, "\n"))
	rendered = strings.ReplaceAll(rendered, "{context_files_section}", contextSection)
	return rendered
}

// NormalizeContextFiles converts a comma-separated list into a trimmed slice.
func NormalizeContextFiles(raw string) []string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return nil
	}
	parts := strings.Split(raw, ",")
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		trimmed := strings.TrimSpace(part)
		if trimmed != "" {
			result = append(result, trimmed)
		}
	}
	return result
}

func resolvePromptTemplate(projectDir, promptOverride string) (string, error) {
	if strings.TrimSpace(promptOverride) != "" {
		return promptOverride, nil
	}

	if path := os.Getenv("GRALPH_PROMPT_TEMPLATE_FILE"); strings.TrimSpace(path) != "" {
		if data, err := os.ReadFile(path); err == nil {
			return string(data), nil
		}
	}

	if strings.TrimSpace(projectDir) != "" {
		candidate := filepath.Join(projectDir, ".gralph", "prompt-template.txt")
		if data, err := os.ReadFile(candidate); err == nil {
			return string(data), nil
		}
	}

	return DefaultPromptTemplate, nil
}
