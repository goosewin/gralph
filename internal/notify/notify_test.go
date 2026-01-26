package notify

import (
	"encoding/json"
	"testing"
	"time"
)

func TestDetectWebhookType(t *testing.T) {
	cases := []struct {
		name string
		url  string
		want WebhookType
	}{
		{name: "discord", url: "https://discord.com/api/webhooks/123", want: WebhookDiscord},
		{name: "discordapp", url: "https://discordapp.com/api/webhooks/123", want: WebhookDiscord},
		{name: "slack", url: "https://hooks.slack.com/services/abc", want: WebhookSlack},
		{name: "generic", url: "https://example.com/webhook", want: WebhookGeneric},
	}

	for _, tc := range cases {
		if got := DetectWebhookType(tc.url); got != tc.want {
			t.Fatalf("%s: expected %s got %s", tc.name, tc.want, got)
		}
	}
}

func TestBuildCompletePayloadDiscord(t *testing.T) {
	opts := CompleteOptions{
		SessionName: "alpha",
		WebhookURL:  "https://discord.com/api/webhooks/123",
		ProjectDir:  "/tmp/project",
		Iterations:  3,
		Duration:    3661 * time.Second,
	}
	payload, err := buildCompletePayload(opts, time.Date(2026, 1, 26, 12, 0, 0, 0, time.UTC))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	var decoded map[string]interface{}
	if err := json.Unmarshal(payload, &decoded); err != nil {
		t.Fatalf("unmarshal payload: %v", err)
	}

	embeds := decoded["embeds"].([]interface{})
	embed := embeds[0].(map[string]interface{})
	if embed["title"].(string) != "\u2705 Gralph Complete" {
		t.Fatalf("unexpected title: %v", embed["title"])
	}
	fields := embed["fields"].([]interface{})
	if len(fields) != 3 {
		t.Fatalf("expected 3 fields, got %d", len(fields))
	}
	iterationsField := fields[1].(map[string]interface{})
	if iterationsField["value"].(string) != "3" {
		t.Fatalf("unexpected iterations: %v", iterationsField["value"])
	}
	durationField := fields[2].(map[string]interface{})
	if durationField["value"].(string) != "1h 1m 1s" {
		t.Fatalf("unexpected duration: %v", durationField["value"])
	}
}

func TestBuildFailedPayloadSlackMaxIterations(t *testing.T) {
	opts := FailedOptions{
		SessionName:    "beta",
		WebhookURL:     "https://hooks.slack.com/services/abc",
		FailureReason:  "max_iterations",
		ProjectDir:     "/tmp/project",
		Iterations:     5,
		MaxIterations:  5,
		RemainingTasks: 2,
		Duration:       70 * time.Second,
	}
	payload, err := buildFailedPayload(opts, time.Date(2026, 1, 26, 12, 0, 0, 0, time.UTC))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	var decoded map[string]interface{}
	if err := json.Unmarshal(payload, &decoded); err != nil {
		t.Fatalf("unmarshal payload: %v", err)
	}

	attachments := decoded["attachments"].([]interface{})
	attachment := attachments[0].(map[string]interface{})
	blocks := attachment["blocks"].([]interface{})
	section := blocks[1].(map[string]interface{})
	text := section["text"].(map[string]interface{})
	if text["text"].(string) != "Session *beta* hit maximum iterations limit." {
		t.Fatalf("unexpected slack description: %v", text["text"])
	}
	fields := blocks[2].(map[string]interface{})["fields"].([]interface{})
	if len(fields) != 5 {
		t.Fatalf("expected 5 slack fields, got %d", len(fields))
	}
}

func TestBuildFailedPayloadGeneric(t *testing.T) {
	opts := FailedOptions{
		SessionName:    "gamma",
		WebhookURL:     "https://example.com/webhook",
		FailureReason:  "manual_stop",
		ProjectDir:     "/tmp/project",
		Iterations:     4,
		MaxIterations:  10,
		RemainingTasks: 1,
		Duration:       65 * time.Second,
	}
	payload, err := buildFailedPayload(opts, time.Date(2026, 1, 26, 12, 0, 0, 0, time.UTC))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	var decoded map[string]interface{}
	if err := json.Unmarshal(payload, &decoded); err != nil {
		t.Fatalf("unmarshal payload: %v", err)
	}

	if decoded["event"].(string) != "failed" {
		t.Fatalf("unexpected event: %v", decoded["event"])
	}
	if decoded["message"].(string) != "Gralph loop 'gamma' was manually stopped after 4 iterations with 1 tasks remaining" {
		t.Fatalf("unexpected message: %v", decoded["message"])
	}
}
