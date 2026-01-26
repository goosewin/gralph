package core

import (
	"bufio"
	"errors"
	"os"
	"regexp"
	"strings"
)

var (
	taskHeaderRe   = regexp.MustCompile(`^\s*###\s+Task\s+`)
	taskEndRe      = regexp.MustCompile(`^\s*(---|##\s+)`)
	uncheckedRe    = regexp.MustCompile(`^\s*- \[ \]`)
	promiseNegated = regexp.MustCompile(`(?i)(cannot|can't|won't|will not|do not|don't|should not|shouldn't|must not|mustn't)[^<]*<promise>`)
)

// GetTaskBlocks extracts task blocks grouped by task headers.
func GetTaskBlocks(taskFile string) ([]string, error) {
	if strings.TrimSpace(taskFile) == "" {
		return nil, nil
	}

	file, err := os.Open(taskFile)
	if err != nil {
		return nil, nil
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

	if inBlock {
		blocks = append(blocks, builder.String())
	}

	return blocks, nil
}

// GetNextUncheckedTaskBlock returns the first block with an unchecked task.
func GetNextUncheckedTaskBlock(taskFile string) (string, error) {
	blocks, err := GetTaskBlocks(taskFile)
	if err != nil {
		return "", err
	}
	for _, block := range blocks {
		if containsUncheckedLine(block) {
			return block, nil
		}
	}
	return "", nil
}

// CountRemainingTasks counts unchecked tasks in a file.
func CountRemainingTasks(taskFile string) (int, error) {
	if strings.TrimSpace(taskFile) == "" {
		return 0, nil
	}

	blocks, err := GetTaskBlocks(taskFile)
	if err != nil {
		return 0, err
	}
	if len(blocks) > 0 {
		count := 0
		for _, block := range blocks {
			count += countUnchecked(block)
		}
		return count, nil
	}

	file, err := os.Open(taskFile)
	if err != nil {
		return 0, nil
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	count := 0
	for scanner.Scan() {
		if uncheckedRe.MatchString(scanner.Text()) {
			count++
		}
	}
	return count, nil
}

// CheckCompletion verifies zero tasks and a valid completion promise.
func CheckCompletion(taskFile, result, completionMarker string) (bool, error) {
	if strings.TrimSpace(taskFile) == "" {
		return false, errors.New("task file is required")
	}
	if strings.TrimSpace(result) == "" {
		return false, nil
	}
	if _, err := os.Stat(taskFile); err != nil {
		return false, err
	}
	remaining, err := CountRemainingTasks(taskFile)
	if err != nil {
		return false, err
	}
	if remaining > 0 {
		return false, nil
	}

	promiseLine := lastNonEmptyLine(result)
	if promiseLine == "" {
		return false, nil
	}

	promise := regexp.MustCompile(`^\s*<promise>` + regexp.QuoteMeta(completionMarker) + `</promise>\s*$`)
	if !promise.MatchString(promiseLine) {
		return false, nil
	}
	if promiseNegated.MatchString(promiseLine) {
		return false, nil
	}

	return true, nil
}

func countUnchecked(block string) int {
	count := 0
	scanner := bufio.NewScanner(strings.NewReader(block))
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		if uncheckedRe.MatchString(scanner.Text()) {
			count++
		}
	}
	return count
}

func containsUncheckedLine(block string) bool {
	scanner := bufio.NewScanner(strings.NewReader(block))
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		if uncheckedRe.MatchString(scanner.Text()) {
			return true
		}
	}
	return false
}

func lastNonEmptyLine(text string) string {
	lines := strings.Split(text, "\n")
	for i := len(lines) - 1; i >= 0; i-- {
		if strings.TrimSpace(lines[i]) != "" {
			return lines[i]
		}
	}
	return ""
}

func firstUncheckedLine(taskFile string) (string, error) {
	file, err := os.Open(taskFile)
	if err != nil {
		return "", err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		line := scanner.Text()
		if uncheckedRe.MatchString(line) {
			return line, nil
		}
	}
	return "", nil
}
