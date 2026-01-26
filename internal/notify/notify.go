package notify

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
	"time"
)

type WebhookType string

const (
	WebhookDiscord WebhookType = "discord"
	WebhookSlack   WebhookType = "slack"
	WebhookGeneric WebhookType = "generic"
)

type CompleteOptions struct {
	SessionName string
	WebhookURL  string
	ProjectDir  string
	Iterations  int
	Duration    time.Duration
	Timeout     time.Duration
}

type FailedOptions struct {
	SessionName    string
	WebhookURL     string
	FailureReason  string
	ProjectDir     string
	Iterations     int
	MaxIterations  int
	RemainingTasks int
	Duration       time.Duration
	Timeout        time.Duration
}

func DetectWebhookType(url string) WebhookType {
	lower := strings.ToLower(url)
	if strings.Contains(lower, "discord.com/api/webhooks") || strings.Contains(lower, "discordapp.com/api/webhooks") {
		return WebhookDiscord
	}
	if strings.Contains(lower, "hooks.slack.com") {
		return WebhookSlack
	}
	return WebhookGeneric
}

func NotifyComplete(ctx context.Context, opts CompleteOptions) error {
	if strings.TrimSpace(opts.SessionName) == "" {
		return errors.New("session name is required")
	}
	if strings.TrimSpace(opts.WebhookURL) == "" {
		return errors.New("webhook URL is required")
	}
	payload, err := buildCompletePayload(opts, time.Now())
	if err != nil {
		return err
	}
	return SendWebhook(ctx, opts.WebhookURL, payload, opts.Timeout)
}

func NotifyFailed(ctx context.Context, opts FailedOptions) error {
	if strings.TrimSpace(opts.SessionName) == "" {
		return errors.New("session name is required")
	}
	if strings.TrimSpace(opts.WebhookURL) == "" {
		return errors.New("webhook URL is required")
	}
	payload, err := buildFailedPayload(opts, time.Now())
	if err != nil {
		return err
	}
	return SendWebhook(ctx, opts.WebhookURL, payload, opts.Timeout)
}

func SendWebhook(ctx context.Context, url string, payload []byte, timeout time.Duration) error {
	if strings.TrimSpace(url) == "" {
		return errors.New("webhook URL is required")
	}
	if len(payload) == 0 {
		return errors.New("payload is required")
	}
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	if ctx == nil {
		ctx = context.Background()
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(payload))
	if err != nil {
		return fmt.Errorf("create webhook request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: timeout}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("send webhook: %w", err)
	}
	defer resp.Body.Close()
	_, _ = io.Copy(io.Discard, resp.Body)

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("webhook returned HTTP %d", resp.StatusCode)
	}
	return nil
}

func buildCompletePayload(opts CompleteOptions, now time.Time) ([]byte, error) {
	project := defaultString(opts.ProjectDir, "unknown")
	iterations := numberString(opts.Iterations)
	duration := formatDuration(opts.Duration)
	timestamp := now.Format(time.RFC3339)

	payloadType := DetectWebhookType(opts.WebhookURL)
	switch payloadType {
	case WebhookDiscord:
		payload := map[string]interface{}{
			"embeds": []map[string]interface{}{
				{
					"title":       "\u2705 Gralph Complete",
					"description": fmt.Sprintf("Session **%s** has finished all tasks successfully.", opts.SessionName),
					"color":       5763719,
					"fields": []map[string]interface{}{
						{
							"name":   "Project",
							"value":  fmt.Sprintf("`%s`", project),
							"inline": false,
						},
						{
							"name":   "Iterations",
							"value":  iterations,
							"inline": true,
						},
						{
							"name":   "Duration",
							"value":  duration,
							"inline": true,
						},
					},
					"footer": map[string]interface{}{
						"text": "Gralph CLI",
					},
					"timestamp": timestamp,
				},
			},
		}
		return json.Marshal(payload)
	case WebhookSlack:
		payload := map[string]interface{}{
			"attachments": []map[string]interface{}{
				{
					"color": "#57F287",
					"blocks": []map[string]interface{}{
						{
							"type": "header",
							"text": map[string]interface{}{
								"type":  "plain_text",
								"text":  "\u2705 Gralph Complete",
								"emoji": true,
							},
						},
						{
							"type": "section",
							"text": map[string]interface{}{
								"type": "mrkdwn",
								"text": fmt.Sprintf("Session *%s* has finished all tasks successfully.", opts.SessionName),
							},
						},
						{
							"type": "section",
							"fields": []map[string]interface{}{
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Project:*\n`%s`", project),
								},
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Iterations:*\n%s", iterations),
								},
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Duration:*\n%s", duration),
								},
							},
						},
						{
							"type": "context",
							"elements": []map[string]interface{}{
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("Gralph CLI \u2022 %s", timestamp),
								},
							},
						},
					},
				},
			},
		}
		return json.Marshal(payload)
	default:
		payload := map[string]interface{}{
			"event":      "complete",
			"status":     "success",
			"session":    opts.SessionName,
			"project":    project,
			"iterations": iterations,
			"duration":   duration,
			"timestamp":  timestamp,
			"message":    fmt.Sprintf("Gralph loop '%s' completed successfully after %s iterations (%s)", opts.SessionName, iterations, duration),
		}
		return json.Marshal(payload)
	}
}

func buildFailedPayload(opts FailedOptions, now time.Time) ([]byte, error) {
	project := defaultString(opts.ProjectDir, "unknown")
	reason := defaultString(opts.FailureReason, "unknown")
	iterations := numberString(opts.Iterations)
	maxIterations := numberString(opts.MaxIterations)
	remaining := numberString(opts.RemainingTasks)
	duration := formatDuration(opts.Duration)
	timestamp := now.Format(time.RFC3339)

	slackDescription := failedDescription(reason, opts.SessionName, true)
	discordDescription := failedDescription(reason, opts.SessionName, false)
	message := failedMessage(reason, opts.SessionName, iterations, maxIterations, remaining)

	payloadType := DetectWebhookType(opts.WebhookURL)
	switch payloadType {
	case WebhookDiscord:
		payload := map[string]interface{}{
			"embeds": []map[string]interface{}{
				{
					"title":       "\u274c Gralph Failed",
					"description": discordDescription,
					"color":       15548997,
					"fields": []map[string]interface{}{
						{
							"name":   "Project",
							"value":  fmt.Sprintf("`%s`", project),
							"inline": false,
						},
						{
							"name":   "Reason",
							"value":  reason,
							"inline": true,
						},
						{
							"name":   "Iterations",
							"value":  fmt.Sprintf("%s/%s", iterations, maxIterations),
							"inline": true,
						},
						{
							"name":   "Remaining Tasks",
							"value":  remaining,
							"inline": true,
						},
						{
							"name":   "Duration",
							"value":  duration,
							"inline": true,
						},
					},
					"footer": map[string]interface{}{
						"text": "Gralph CLI",
					},
					"timestamp": timestamp,
				},
			},
		}
		return json.Marshal(payload)
	case WebhookSlack:
		payload := map[string]interface{}{
			"attachments": []map[string]interface{}{
				{
					"color": "#ED4245",
					"blocks": []map[string]interface{}{
						{
							"type": "header",
							"text": map[string]interface{}{
								"type":  "plain_text",
								"text":  "\u274c Gralph Failed",
								"emoji": true,
							},
						},
						{
							"type": "section",
							"text": map[string]interface{}{
								"type": "mrkdwn",
								"text": slackDescription,
							},
						},
						{
							"type": "section",
							"fields": []map[string]interface{}{
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Project:*\n`%s`", project),
								},
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Reason:*\n%s", reason),
								},
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Iterations:*\n%s/%s", iterations, maxIterations),
								},
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Remaining Tasks:*\n%s", remaining),
								},
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("*Duration:*\n%s", duration),
								},
							},
						},
						{
							"type": "context",
							"elements": []map[string]interface{}{
								{
									"type": "mrkdwn",
									"text": fmt.Sprintf("Gralph CLI \u2022 %s", timestamp),
								},
							},
						},
					},
				},
			},
		}
		return json.Marshal(payload)
	default:
		payload := map[string]interface{}{
			"event":           "failed",
			"status":          "failure",
			"session":         opts.SessionName,
			"project":         project,
			"reason":          reason,
			"iterations":      iterations,
			"max_iterations":  maxIterations,
			"remaining_tasks": remaining,
			"duration":        duration,
			"timestamp":       timestamp,
			"message":         message,
		}
		return json.Marshal(payload)
	}
}

func formatDuration(duration time.Duration) string {
	if duration <= 0 {
		return "unknown"
	}
	total := int(duration.Seconds())
	if total <= 0 {
		return "unknown"
	}
	hours := total / 3600
	mins := (total % 3600) / 60
	secs := total % 60
	if hours > 0 {
		return fmt.Sprintf("%dh %dm %ds", hours, mins, secs)
	}
	if mins > 0 {
		return fmt.Sprintf("%dm %ds", mins, secs)
	}
	return fmt.Sprintf("%ds", secs)
}

func numberString(value int) string {
	if value <= 0 {
		return "unknown"
	}
	return strconv.Itoa(value)
}

func defaultString(value, fallback string) string {
	trimmed := strings.TrimSpace(value)
	if trimmed == "" {
		return fallback
	}
	return trimmed
}

func failedDescription(reason, sessionName string, slack bool) string {
	prefix := "**"
	suffix := "**"
	if slack {
		prefix = "*"
		suffix = "*"
	}
	label := fmt.Sprintf("%s%s%s", prefix, sessionName, suffix)
	switch reason {
	case "max_iterations":
		return fmt.Sprintf("Session %s hit maximum iterations limit.", label)
	case "error":
		return fmt.Sprintf("Session %s encountered an error.", label)
	case "manual_stop":
		return fmt.Sprintf("Session %s was manually stopped.", label)
	default:
		return fmt.Sprintf("Session %s failed: %s", label, reason)
	}
}

func failedMessage(reason, sessionName, iterations, maxIterations, remaining string) string {
	switch reason {
	case "max_iterations":
		return fmt.Sprintf("Gralph loop '%s' failed: hit max iterations (%s/%s) with %s tasks remaining", sessionName, iterations, maxIterations, remaining)
	case "error":
		return fmt.Sprintf("Gralph loop '%s' failed due to an error after %s iterations", sessionName, iterations)
	case "manual_stop":
		return fmt.Sprintf("Gralph loop '%s' was manually stopped after %s iterations with %s tasks remaining", sessionName, iterations, remaining)
	default:
		return fmt.Sprintf("Gralph loop '%s' failed: %s after %s iterations", sessionName, reason, iterations)
	}
}
